#[macro_use] extern crate rocket;
#[cfg(test)] mod tests;
use rocket::http::{Cookie, CookieJar, Status};
use std::net::IpAddr;
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rocket::response::{self, status, Responder, Response};
use rocket::request::Request;
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
#[derive(Responder)]
struct Unauth(status::Unauthorized<String>);

impl From<rocket_db_pools::sqlx::Error> for Unauth {
	fn from(inval: rocket_db_pools::sqlx::Error) -> Self{
	Unauth(status::Unauthorized(Some(inval.to_string())))	
	}
}

impl From<&str> for Unauth {
	fn from(inval: &str) -> Self{
		Unauth(status::Unauthorized(Some(inval.to_string())))
	}
}

// for now this just serves a version string, will become landing page once webservice is moved internally.
#[get("/")]
fn index() -> &'static str {
    "oSNAP API server v0.1.2"
}

// Profile request mechanism
#[get("/api/profile/<user>")]
async fn profile(mut db: Connection<Postgres>, user: &str, cookies: &CookieJar<'_>) -> Result<status::Accepted<Json<Profile>>, Unauth>{
	// Check Client Authorization
	// Get auth token cookie
	let auth: String;
	match cookies.get_private("osnap-authtoken") {
		Some(tkn) => auth = tkn.value().to_string(),
		None => return Err(Unauth::from("Client did not send authtoken cookie!")),
	}
	// verify auth token cookie
	let record = sqlx::query("SELECT * FROM users WHERE usern = ?").bind(sanitizer(user, FieldType::Alpha)).fetch_one(&mut *db).await?;
	let dbauth: &str = record.try_get("auth")?;
	if dbauth != auth {
		return Err(Unauth::from("Auth token does not match! Try logging back in."));
	}
	Ok(status::Accepted(Some(Json::from(Profile{user: user.to_string(), name: record.try_get("name")?, age: record.try_get("age")?, gender: record.try_get("gender")?, phone: record.try_get("phone")?, contacts_names: record.try_get("contacts_names")?, contacts_phones: record.try_get("contacts_phones")?, ratings: record.try_get("ratings")?}))))
	
}

// profile data update mechanism.
#[post("/api/profile/<user>", format="json", data = "<request>")]
async fn profileup(mut db: Connection<Postgres>, request: Json<ProfileUpdate<'_>>, user: &str, cookies: &CookieJar<'_>) -> Result<status::Accepted<String>, Unauth>{
	// Check Client Authorization
	// Get auth token cookie
	let auth: String;
	match cookies.get_private("osnap-authtoken") {
		Some(c) => auth = c.value().to_string(),
		None => return Err(Unauth::from("Client did not send authtoken cookie!")),
	}
	// verify auth token cookie
	let expected = sqlx::query("SELECT auth FROM users WHERE usern = ?").bind(sanitizer(user, FieldType::Alpha)).fetch_one(&mut *db).await?;
	let dbauth: &str = expected.try_get("auth")?;
	if dbauth != auth {
		return Err(Unauth::from("Auth token does not match! Try logging back in."));
	}
	// if we get here, auth is good
	if request.name != "none"{
		sqlx::query("UPDATE users SET name = ? WHERE usern = ?").bind(sanitizer(request.name, FieldType::AlphaNum)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.gender != "none"{
		sqlx::query("UPDATE users SET gender = ? WHERE usern = ?").bind(sanitizer(request.gender, FieldType::AlphaNum)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.phone != "none"{
		sqlx::query("UPDATE users SET phone = ? WHERE usern = ?").bind(sanitizer(request.phone, FieldType::Phone)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.contacts_names != "none"{
		sqlx::query("UPDATE users SET contacts_names = ? WHERE usern = ?").bind(sanitizer(request.contacts_names, FieldType::Alpha)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.contacts_phones != "none"{
		sqlx::query("UPDATE users SET contacts_phones = ? WHERE usern = ?").bind(sanitizer(request.contacts_phones, FieldType::Phone)).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}
	if request.age != defaultint() {
		sqlx::query("UPDATE users SET age = ? WHERE usern = ?").bind(request.age).bind(sanitizer(user, FieldType::Alpha)).execute(&mut *db).await?;
	}

Ok(status::Accepted(Some("Updated successfully.".to_string())))
}

//TODO: re-add signing functions with oAuth support

// function for backend walk request handling

// TODO: switch to in-memory data structure
#[post("/api/request", format="json", data = "<request>")]
async fn walk_request_handler(mut db: Connection<Postgres>, request: Json<WalkRequest>) -> Result<status::Created<String>, Unauth>{
	// make sure client is authorized
	let auth: String;
	match cookies.get_private("osnap-authtoken") {
		Some(c) => auth = c.value().to_string(),
		None => return Err(Unauth::from("Client did not send authtoken cookie!")),
	}
	// verify auth token 
	let expected = sqlx::query("SELECT usern FROM users WHERE auth = ?").bind(auth)).fetch_one(&mut *db).await?;
	let user: &str = expected.try_get("usern")?; // we should auto-return unauth if the credential doesn't exist
	if !user.len() {
		return Err(Unauth::from("Auth token does not match! Try logging back in."));
	}
	// generate a random request id 
	// TODO: (maybe) use UUid crate for request IDs
	let request_id = OsRng.next_u32().to_string();
	// create an entry in the database's Requests table for the backend to match requests
	sqlx::query("INSERT INTO Requests (id,User,Dest,Lat,Long) VALUES(?, ?, ?, ?, ?)")
	.bind(&request_id)
	.bind(user)
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
async fn walk_wizard(mut db: Connection<Postgres>, id: &str) -> String{
"Not implimented".to_string()
}

// Manage the walk confirmation stage. Both users must accept the walk to continue
#[post("/api/trip/<id>", format="json", data = "<request>")]
async fn walkman(mut db: Connection<Postgres>, request: Json<WalkResponce<'_>>, id: &str) -> Value{
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
#[database("Postgres")]
struct Postgres(sqlx::PgPool);

#[launch]
fn rocket() -> _ {
	let (mut asend, arecv) = unbounded();
	let mut hndl = launch_alert_thread(arecv);
    rocket::build().mount("/", routes![index, profile, profileup, signup_handler, signin_handler, walk_request_handler, walk_wizard, walkman])
	.attach(Postgres::init()).manage(Persist{alertsnd: asend, thrdhndl: hndl})
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
