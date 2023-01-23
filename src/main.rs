#[macro_use] extern crate rocket;
use std::net::IpAddr;
use geoutils::{Location as Loc, Distance};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rand_core::{RngCore, OsRng};
use std::{thread, time, time::Duration};
use crossbeam::channel::{self, unbounded, Receiver};

// Define structs for automatic JSON Parsing 
// TODO: Impliment request guards for data integrity & security

// Struct for a profile data request. TODO: move update requests to separate struct and server dir
#[derive(Deserialize, Clone)]
pub struct ProfRequest<'r> {
	user: &'r str,
	auth: &'r str,
	operation: &'r str,
	#[serde(default = "none")]
	name: &'r str,
	#[serde(default = "defaultint")]
	age: u16,
	#[serde(default = "none")]
	gender: &'r str,
	#[serde(default = "emptyvec")]
	contacts: Vec<&'r str>
}

// struct used to serve user data in JSON
#[derive(Serialize, Clone)]
pub struct Profile {
	user: String,
	name: String,
	age: u16,
	gender: String,
	phone: String,
	contacts: Vec<String>,
	ratings: String
}

// request submitted by user to find walking buddy
// TODO: add desired time of departure
#[derive(Deserialize)]
pub struct WalkRequest<'r> {
	user: &'r str,
	auth: &'r str,
	name: &'r str,
	dest: &'r str,
	loc: Location<'r>
}
// Location struct. TODO: Might remove and just have comma separated string
#[derive(Deserialize)]
pub struct Location<'r> {
	latitude: &'r str,
	longitude: &'r str
}
// struct to serve profile data of other users, does not include sensitive info
#[derive(Serialize)]
pub struct PubProfile {
	name: String,
	approxdist: String,
	avgrating: f32,
	numratings: u32,
	picture: String
}
// used to deserialized user responses to walk buddies suggested by server
#[derive(Deserialize)]
pub struct WalkResponce<'r>{
	user: &'r str,
	auth: &'r str,
	operation: &'r str,
	#[serde(default = "falsebool")]
	failsafe: bool
}
// struct for authentication, might remove
#[derive(Deserialize)]
pub struct AuthOnly<'r>{
	user: &'r str,
	auth: &'r str,
}
// request to disarm the failsafe system 
#[derive(Deserialize)]
pub struct DisarmRequest<'r>{
	user: &'r str,
	auth: &'r str,
	operation: &'r str,
	curlocation: Location<'r>
}

#[derive(Deserialize)]
struct Signup<'r>{
	user: &'r str,
	name: &'r str,
	phone: &'r str,
	password: &'r str,
}
// different from authonly as this is a password, not an auth token 
// TODO: remove and just use AuthOnly (or msft oAUTH)
#[derive(Deserialize)]
struct Signin<'r>{
	user: &'r str,
	password: &'r str,
}
// struct to store information about a failsafe session
#[derive(Clone)]
struct AlertComm{
	armed: bool,
	start: time::Instant,
	name: String,
	dest: String,
	contacts: String,
	trip_id: String
}
// stores scrossbean channel to communicate with failsafe thread
struct Persist {
    alertsnd: channel::Sender<AlertComm>,
    thrdhndl: thread::JoinHandle<()>,
}


//goofy ahh functions due to serde quirk 
//TODO: can probably be removed once user data requests and update are moved to separate endpoints
fn none() -> &'static str {
	"none"
}
fn defaultint() -> u16 {
	65535
}

fn falsebool()->bool{
	false
}

fn emptyvec<'r>() -> Vec<&'r str>{
        vec!()
}

// for now this just serves a version string, will become landing page once webservice is moved internally.
#[get("/")]
fn index() -> &'static str {
    "oSNAP API server v0.1.0-goathack"
}
// profile data request or update mechanism.
// TODO: move update mechanism to separate function
// TODO: look into using results and status codes so we don't need emptyprof
#[post("/api/profile", format="json", data = "<request>")]
async fn profile(mut db: Connection<Users>, request: Json<ProfRequest<'_>>) -> Json<Profile>{

	match sqlx::query("SELECT * from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await{
		Ok(entry) => {
			if entry.get::<&str, &str>("auth") != request.auth{
				return badprof();
				}
			if request.operation == "update"{
				if request.age != 0{
					sqlx::query("UPDATE Users SET age = ? WHERE user = ?").bind(request.age).bind(request.user).execute(&mut *db).await.unwrap();
				}
				if request.gender != "none"{
					sqlx::query("UPDATE Users SET gender = ? WHERE user = ?").bind(request.gender).bind(request.user).execute(&mut *db).await.unwrap();
				}
				if !request.contacts.is_empty(){
					let mut tmp = String::new();
					for i in request.contacts.as_slice(){
						tmp.push_str(i);
						tmp.push(',');
					}
					sqlx::query("UPDATE Users SET contacts = ? WHERE user = ?").bind(tmp).bind(request.user).execute(&mut *db).await.unwrap();
				}
				if request.name != "none"{
					sqlx::query("UPDATE Users SET name = ? WHERE user = ?").bind(request.name).bind(request.user).execute(&mut *db).await.unwrap();
				}
			}
			let vuser = entry.get::<&str, &str>("user").to_string();
			let vname= entry.get::<&str, &str>("name").to_string();
			let vage = entry.get::<u16, &str>("age");
			let vphone = entry.get::<&str, &str>("name").to_string();
			let vgender = entry.get::<&str, &str>("gender").to_string();
			let vcontacts = entry.get::<&str, &str>("contacts").split(',').collect::<Vec<&str>>();
			let mut wcontacts: Vec<String> = vec!();
			for i in vcontacts{
				wcontacts.push(i.to_string());
			}
			let vratings = entry.get::<&str, &str>("ratings").to_string();
			let newprof = Profile{
				user: vuser,
				name: vname,
				age: vage,
				phone: vphone,
				gender: vgender,
				contacts: wcontacts,
				ratings: vratings
			};
			Json(newprof)
		}
		Err(_) => {
			badprof()
		}
	}
}
// return an empty profile
fn badprof() -> Json<Profile>{
	Json(Profile{user:"none".to_string(), name:"none".to_string(), age:0, phone:"none".to_string(), gender:"none".to_string(),contacts:vec!(),ratings:"".to_string()})
}
// profile sign up handler
#[post("/api/signup", format="json", data = "<request>")]
async fn signup_handler(mut db: Connection<Users>, request: Json<Signup<'_>>, addr: IpAddr) -> Value{
	// grab ip address TODO: remove when oAUTH is implimended
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
	// TODO: only generate on successful auth to prevent resource exhaustion
	// TODO: I don't think OsRNng is considered secure. Better RNG? Change to u64?
	let auth = OsRng.next_u32().to_string();
	// grab user data from SQL db
	match sqlx::query("SELECT password from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await {
		//TODO: add password hash check with argon2
		Ok(row) => {if row.get::<&str, &str>("password") == request.password {
			// if password matches, issue token. 
			// TODO: token expiration
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
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		return json!({"request":"failed"});
	}
	// generate a random request id 
	// TODO: (maybe) use UUid crate?
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
// TODO: return JSON not string
#[get("/api/trip/<id>")]
async fn walk_wizard(mut db: Connection<Users>, id: &str) -> String{
	// since this function does not use JSON inputs, we can't use request guards to validate data
	// TODO: check if we acutally can use requests guards for this
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
	//TODO: switch to thread builder
	//TODO: add respawn functionality in walk request handler
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
