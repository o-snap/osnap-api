#[macro_use] extern crate rocket;
#[cfg(test)] mod tests;
use rocket::http::{Cookie, CookieJar};
use std::net::IpAddr;
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rocket::response::status;
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
    "oSNAP API server v0.1.1"
}

// Profile request mechanism
#[post("/api/profile/<user>")]
async fn profile(mut db: Connection<postgres>, user: &str, cookies: &CookieJar<'_>) -> Result<status::Accepted<Json<Profile>>, status::Unauthorized<String>>{
	// Check Client Authorization
	// Get auth token cookie
	let mut auth: &str;
	match cookies.get_private("osnap-authtoken") {
		Some(c) => auth = c.value(),
		None => return Err("Client did not send authtoken cookie!"),
	}
	// verify auth token cookie
	let record = sqlx::query("SELECT * FROM Users WHERE usern = ?").bind(sanitizer(user, FieldType::Alpha)).fetch_one(&mut *db).await?;
	if record.try_get("auth")? != auth {
		return Err("Auth token does not match! Try logging back in.");
	}
	return Ok(Profile{user: user, name: &record.try_get("name")?, name: &record.try_get("age")?, gender: &record.try_get("gender")?, phone: &record.try_get("phone")?, contacts: &record.try_get("contacts")?});
	
}

// profile data update mechanism.
#[post("/api/profile/<user>", format="json", data = "<request>")]
async fn profileup(mut db: Connection<postgres>, request: Json<ProfileUpdate<'_>>, user: &str, cookies: &CookieJar<'_>) -> Result<status::Accepted<String>, status::Unauthorized<String>>{
	// Check Client Authorization
	// Get auth token cookie
	let mut auth: &str;
	match cookies.get_private("osnap-authtoken") {
		Some(c) => auth = c.value(),
		None => return Err("Client did not send authtoken cookie!"),
	}
	// verify auth token cookie
	let expected = sqlx::query("SELECT auth FROM Users WHERE usern = ?").bind(sanitizer(user, FieldType::Alpha)).fetch_one(&mut *db).await?;
	if expected.try_get("auth")? != auth {
		return Err("Auth token does not match! Try logging back in.");
	}
	// if we get here, auth is good
	if request.name != "none"{
		sqlx::query("UPDATE Users SET name = ? WHERE usern = ?").bind(sanitizer(request.name, FieldType::AlphaNum)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.gender != "none"{
		sqlx::query("UPDATE Users SET gender = ? WHERE usern = ?").bind(sanitizer(request.gender, FieldType::AlphaNum)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.phone != "none"{
		sqlx::query("UPDATE Users SET phone = ? WHERE usern = ?").bind(sanitizer(request.phone, FieldType::Phone)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.contacts != "none"{
		sqlx::query("UPDATE Users SET contacts = ? WHERE usern = ?").bind(sanitizer(request.contacts, FieldType::Phone)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.age != defaultint() {
		sqlx::query("UPDATE Users SET age = ? WHERE usern = ?").bind(request.age).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}

Ok("Updated successfully.")
}

// profile sign up handler
#[post("/api/signup", format="json", data = "<request>")]
async fn signup_handler(mut db: Connection<postgres>, request: Json<Signup<'_>>, addr: IpAddr) -> Value{
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
async fn signin_handler(mut db: Connection<postgres>, request: Json<Signin<'_>>) -> Value{
	// generate a random authentication token
	// TODO: only generate auth token on successful login
	// its an unnecessary waste of time and server resources
	// TODO: I don't think OsRNng is considered secure. Better RNG? Change to u64?
	// since this token will be served in a Rocket encrypted cookie, I don't think it should be too bad to just use u64
	let auth = OsRng.next_u32().to_string();
	// grab user data from SQL db
	match sqlx::query("SELECT password from Users WHERE usern = ?").bind(request.user).fetch_one(&mut *db).await {
		//TODO: add password hash check with argon2
		Ok(row) => {if row.get::<&str, &str>("password") == request.password {
			// if password matches, issue token. 
			// TODO: add auth token expiration
			sqlx::query("UPDATE Users SET auth = ? WHERE usern = ?").bind(&auth).bind(request.user).execute(&mut *db).await.unwrap();
			return json!({"login": auth})
		}}
		Err(_) => return json!({"login": "noaccount"})
	}
	json!({"login": "bad"})
}

// function for backend walk request handling
#[post("/api/request", format="json", data = "<request>")]
async fn walk_request_handler(mut db: Connection<postgres>, request: Json<WalkRequest>) -> Value{
	// make sure client is authorized
	// TODO: Update request authentication to match new API spec
	// the API no longer requires the userame to be sent at all and the auth token was moved to a cookie from JSON body.
	if sqlx::query("SELECT auth from Users WHERE usern = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
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
async fn walk_wizard(mut db: Connection<postgres>, id: &str) -> String{
"Not implimented".to_string()
}

// Manage the walk confirmation stage. Both users must accept the walk to continue
#[post("/api/trip/<id>", format="json", data = "<request>")]
async fn walkman(mut db: Connection<postgres>, request: Json<WalkResponce<'_>>, id: &str) -> Value{
	// make sure client is authorized
	if sqlx::query("SELECT auth from Users WHERE usern = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
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

// initialize SQLite database connection defined in Rocket.toml
// TODO: Migrate from sqlite to postresql
// sqlite driver is written in C making it unsafe
#[derive(Database)]
#[database("postgres")]
struct postgres(sqlx::PgPool);

#[launch]
fn rocket() -> _ {
	let (mut asend, arecv) = unbounded();
	let mut hndl = launch_alert_thread(arecv);
    rocket::build().mount("/", routes![index, profile, profileup, signup_handler, signin_handler, walk_request_handler, walk_wizard, walkman])
	.attach(postgres::init()).manage(Persist{alertsnd: asend, thrdhndl: hndl})
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
