#[macro_use] extern crate rocket;
#[cfg(test)] mod tests;
use rocket::http::{Cookie, CookieJar};
use std::net::IpAddr;
use geoutils::{Location as Loc, Distance};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rand_core::{RngCore, OsRng};
use std::{thread, time, time::Duration};
use crossbeam::channel::{self, unbounded, Receiver};
mod parsers;
pub use crate::parsers::*;
use chrono::{Utc, DateTime};

// struct to store information about a failsafe session
#[derive(Clone)]
struct AlertComm{
	armed: bool,
	start: DateTime<Utc>,
	name: String,
	dest: String,
	contacts: String,
	trip_id: String
}
// stores crossbeam channel to communicate with failsafe thread
struct Persist {
    alertsnd: channel::Sender<AlertComm>,
    thrdhndl: thread::JoinHandle<()>,
}


// for now this just serves a version string, will become landing page once webservice is moved internally.
#[get("/")]
fn index() -> &'static str {
    "oSNAP API server v0.1.0-goathack"
}
// profile data request or update mechanism.
// TODO: move user data update mechanism to separate function (maybe profile data can become a GET?)
// should significantly reduce complexity of profile request handler. Could also consolidate with the pubprofile handler
// TODO: look into using results and HTTP status codes so we don't need emptyprof
#[post("/api/profile/<user>", format="json", data = "<request>")]
async fn profile(mut db: Connection<Users>, request: Json<ProfileUpdate<'_>>, user: &str) -> Json<Profile>{

}

// profile sign up handler
#[post("/api/signup", format="json", data = "<request>")]
async fn signup_handler(mut db: Connection<Users>, request: Json<Signup<'_>>, addr: IpAddr) -> Value{
	// grab ip address 
	// TODO: remove IP address collection when oAUTH is implimended
	let mut ip = addr.to_string();
	ip = ip.get(0..7).unwrap().to_string();
	if ip != "130.215" && ip != "207.174"{
		// assume they're at WPI if the're within these address ranges (I know it's a really stupid way to do this but i'm on a short timetable)
		return json!({
			"status": "unauthorized"
		});
	}
	// add user to database
	// TODO: move password hashing to API side of things
	sqlx::query("INSERT INTO Users (user,name,phone,password) VALUES(?, ?, ?, ?)")
	.bind(request.user)
	.bind(request.name)
	.bind(request.phone)
	.bind(request.password)
	.execute(&mut *db).await.unwrap();
	json!({"status": "ok"})

}
// sign in request
#[post("/api/signin", format="json", data = "<request>")]
async fn signin_handler(mut db: Connection<Users>, request: Json<Signin<'_>>) -> Value{
	// generate a random authentication token
	// TODO: only generate auth token on successful login
	// its an unnecessary waste of time and server resources
	// TODO: I don't think OsRNng is considered secure. Better RNG? Change to u64?
	// since this token will be served in a Rocket encrypted cookie, I don't think it should be too bad to just use u64
	let auth = OsRng.next_u32().to_string();
	// grab user data from SQL db
	match sqlx::query("SELECT password from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await {
		//TODO: add password hash check with argon2
		Ok(row) => {if row.get::<&str, &str>("password") == request.password {
			// if password matches, issue token. 
			// TODO: add auth token expiration
			sqlx::query("UPDATE Users SET auth = ? WHERE user = ?").bind(&auth).bind(request.user).execute(&mut *db).await.unwrap();
			return json!({"login": auth})
		}}
		Err(_) => return json!({"login": "noaccount"})
	}
	json!({"login": "bad"})
}

// function for backend walk request handling
#[post("/api/request", format="json", data = "<request>")]
async fn walk_request_handler(mut db: Connection<Users>, request: Json<WalkRequest<'_>>) -> Value{
	// make sure client is authorized
	// TODO: Update request authentication to match new API spec
	// the API no longer requires the username to be sent at all and the auth token was moved to a cookie from JSON body.
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		return json!({"request":"failed"});
	}
	// generate a random request id 
	// TODO: (maybe) use UUid crate for request IDs
	let request_id = OsRng.next_u32().to_string();
	// create an entry in the database's Requests table for the backend to match requests
	sqlx::query("INSERT INTO Requests (id,User,Dest,Lat,Long) VALUES(?, ?, ?, ?, ?)")
	.bind(&request_id)
	.bind(request.user)
	.bind(request.dest)
	.bind(request.loc.latitude)
	.bind(request.loc.longitude)
	.execute(&mut *db).await.unwrap();
	json!({"request":request_id})
}

// tries to find a walking buddy for a user and returns a status code
// TODO: refactor all functions to return JSON not strings
// even though this endpoint should only return a single word, it's best to keep things consistant.
#[get("/api/trip/<id>")]
async fn walk_wizard(mut db: Connection<Users>, id: &str) -> String{
	// since this function does not use JSON inputs, we can't use request guards to validate data
	// TODO: replace tripID sanitization with request guards
	// walk_wizard's sanitization is crazy inefficient. 
	for i in [';','\\','*']{
		if id.contains(i) {
			panic!("Illegal input in trip id!");
		}
	}
	// search requests table for requests matching the provided request id
	let trip = sqlx::query("SELECT * FROM Requests WHERE id = ?").bind(id).fetch_one(&mut *db).await.unwrap();
	if trip.get::<&str, &str>("Trip").len() > 1 { // a suitable match was already found
		// if there is any data in the trip field, wizard already found trip
		return String::from("Confirmed");
	}
	let dest: String = trip.get("Dest");
	let curpos = Loc::new(trip.get::<&str, &str>("Lat").parse::<f32>().unwrap(), trip.get::<&str, &str>("Long").parse::<f32>().unwrap());
	println!("Running matching wizard for trip {} bound for {}",id,&dest);
	let peers = sqlx::query("SELECT * FROM Requests WHERE Dest = ?").bind(&dest).fetch_all(&mut *db).await.unwrap();
	let mut leastdist = 1000000.0; //meters 
	let mut bestmatch_id:String = "0000".to_string();
	let mut bestmatch_user:String = "nobody".to_string();
	for peer in peers{
		if peer.get::<&str, &str>("Dest") != dest {continue;}
		let pos = Loc::new(peer.get::<&str, &str>("Lat").parse::<f32>().unwrap(), peer.get::<&str, &str>("Long").parse::<f32>().unwrap());
		if pos.haversine_distance_to(&curpos).meters() < leastdist {
			leastdist = pos.haversine_distance_to(&curpos).meters();
			bestmatch_id = peer.get("id");
			bestmatch_user = peer.get("User");
		}
	}
	if leastdist < 1000000.0 {
		let trip_id = OsRng.next_u32().to_string();
		// Possible status codes: 0-cancelled, 2-pending, 4-inprogress, 6-complete
		sqlx::query("INSERT INTO Trips (id, Dest, user1, user2, status) VALUES(?, ?, ?, ?, 2)")
		.bind(&trip_id)
		.bind(dest)
		.bind(trip.get::<&str, &str>("user"))
		.bind(bestmatch_user)
		.execute(&mut *db).await.unwrap();
		sqlx::query("UPDATE Requests SET Trip = ? WHERE id = ?").bind(&trip_id).bind(id).execute(&mut *db).await.unwrap();
		sqlx::query("UPDATE Requests SET Trip = ? WHERE id = ?").bind(&trip_id).bind(bestmatch_id).execute(&mut *db).await.unwrap();
		return String::from("Confirmed");
	}
	String::from("pending")
}

// Manage the walk confirmation stage. Both users must accept the walk to continue
#[post("/api/trip/<id>", format="json", data = "<request>")]
async fn walkman(mut db: Connection<Users>, request: Json<WalkResponce<'_>>, id: &str) -> Value{
	// make sure client is authorized
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		return json!({"request":"error: unauthorized"});
	}
	// make sure trip exists 
	let trip = sqlx::query("SELECT * from Trips WHERE id = ?").bind(id).fetch_one(&mut *db).await.unwrap();
	if trip.is_empty(){
		return json!({"request":"nonexistant"});
	}
	let mut stat = trip.get::<u16, &str>("status");
	let mut fs = 0;
	if stat == 0{
		return json!({"request":"cancelled by peer"});
	}
	match request.operation {
		"accept" => stat += 1,
		"decline" => stat = 0,
		_ => return json!({"request":"invalid"})
	}
	if request.failsafe {
		fs = 1;
	}
	let mut user = "u1fs";
	if trip.get::<&str, &str>("user2") == request.user{
		user = "u2fs"
	}
	sqlx::query("UPDATE Trips SET status = ?, ? = ? WHERE id = ?")
	.bind(stat)
	.bind(user)
	.bind(fs)
	.bind(id)
	.execute(&mut *db).await.unwrap();
	json!({"request":"ok"})
}
// Show a user the public profile of their assigned buddy
#[post("/trip/<id>/buddy", format="json", data = "<request>")]
async fn peerinfo(mut db: Connection<Users>, request: Json<AuthOnly<'_>>, id: &str) -> Json<PubProfile>{
	// make sure client is authorized
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		panic!("Unauthorized user!");
	}
	let trip = sqlx::query("SELECT * from Trips WHERE id = ?").bind(id).fetch_one(&mut *db).await.unwrap();
	if trip.is_empty(){
		panic!("Bad trip!");
	}
	let mut buddy = "user2";
	if trip.get::<&str, &str>("user1") != request.user{
		buddy = "user1";
	}
	let buddyname = trip.get::<&str, &str>(buddy);
	let buddyprof = sqlx::query("SELECT * FROM Users WHERE user = ?").bind(buddyname).fetch_one(&mut *db).await.unwrap();
	let ratings = buddyprof.get::<&str, &str>("ratings").split(",");
	let mut numrate = 0;
	let mut avg = 0.0;
	for i in ratings{
		numrate += 1;
		avg += i.parse::<f32>().unwrap();
	}
	let vname = buddyprof.get::<&str, &str>("name").to_string();

	Json(PubProfile { name: vname, approxdist: "Not implimented".to_string(), avgrating: avg, numratings: numrate, picture: "Not implimented!".to_string() })
}

// initialize SQLite database connection defined in Rocket.toml
// TODO: Migrate from sqlite to postresql
// sqlite driver is written in C making it unsafe
#[derive(Database)]
#[database("Users")]
struct Users(sqlx::SqlitePool);

#[launch]
fn rocket() -> _ {
	let (mut asend, arecv) = unbounded();
	let mut hndl = launch_alert_thread(arecv);
    rocket::build().mount("/", routes![index, profile, signup_handler, signin_handler, walk_request_handler, walk_wizard, walkman, peerinfo])
	.attach(Users::init()).manage(Persist{alertsnd: asend, thrdhndl: hndl})
}
/* Launches a dedicated thread to manage the failsafe system. Communication with this thread is done via the AlertComm struct and is 1-way.
* launch function returns a join handle which can be periodically checked to make sure it's still alive and respawn if necessary. 
*/
fn launch_alert_thread(reciever: Receiver<AlertComm>) -> thread::JoinHandle<()> {
	// TODO: switch the alread thread to thread builder
	// might be nice to be able to name the thread
	// TODO: add alert thread respawn functionality in walk request handler
	thread::spawn(move || {
		loop {
			//initialize a storage area for alert data
			let mut store:Vec<AlertComm> = vec!();
			// check if there are any new messages
			match reciever.try_recv(){
				Ok(msg) => {
					if msg.armed{
						let name = msg.name.clone();
						let trip_id = msg.trip_id.clone();
						for i in store.as_slice(){
						// discard messages telling us to arm a user's trip that's already armed
						if i.name == name && i.trip_id == trip_id {break;}
						}
						store.push(msg);
					}
				}
				Err(_) => thread::sleep(Duration::from_millis(15))
			}

		}
	})
}
