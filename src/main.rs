#[macro_use] extern crate rocket;
#[cfg(test)] mod tests;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::State;
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rocket::response::{self, status, Responder, Response};
use rocket::request::Request;
use std::{thread, time, time::Duration};
use crossbeam::channel::{self, unbounded, Receiver};
use std::sync::{Arc, Mutex};
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

struct WalkState {
	active: Arc<Mutex<Vec<WalkRequest>>>
}

// Wrapper for Unauthorized responce that impliments SQLX error conversion
#[derive(Responder)]
struct Unauth(status::Custom<String>);

impl From<rocket_db_pools::sqlx::Error> for Unauth {
	fn from(inval: rocket_db_pools::sqlx::Error) -> Self{
	Unauth(status::Custom(Status::Unauthorized, inval.to_string()))
	}
}

impl From<&str> for Unauth {
	fn from(inval: &str) -> Self{
		Unauth(status::Custom(Status::Unauthorized, inval.to_string()))
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
	let auth = cookies.get_private("osnap-authtoken").ok_or("No auth token was provided!")?.value().to_string();
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
	let auth = cookies.get_private("osnap-authtoken").ok_or("No auth token was provided!")?.value().to_string();
	let record = sqlx::query("SELECT auth FROM users WHERE usern = ?").bind(sanitizer(user, FieldType::Alpha)).fetch_one(&mut *db).await?;
	let dbauth: &str = record.try_get("auth")?;
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

//This function recieves walk requests, validates them, and stores them.
#[post("/api/request", format="json", data = "<indata>")]
async fn walk_request_handler(mut db: Connection<Postgres>, mut indata: Json<WalkRequest>, requests: &State<WalkState>,  cookies: &CookieJar<'_>) -> Result<status::Created<String>, Unauth>{
	let auth = cookies.get_private("osnap-authtoken").ok_or("No auth token was provided!")?.value().to_string();
	let record = sqlx::query("SELECT usern FROM users WHERE auth = ?").bind(auth).fetch_one(&mut *db).await?;
	record.try_get("usern")?; // we should auto-return unauth if the credential doesn't exist
	// verify incoming request
	// TODO: add geofence verification when google maps API is implimented
	if indata.minbuddies > indata.maxbuddies || indata.minbuddies < 0 {
		return Err(Unauth(status::Custom(Status::BadRequest, "Invalid Buddy range supplied".to_string())));
	}
	let loc: Vec<&str> = indata.loc.split(',').collect();
	let dest: Vec<&str> = indata.dest.split(',').collect();
	if loc.len() != 2 || dest.len() != 2 {
		return Err(Unauth(status::Custom(Status::BadRequest, "Invalid Location Format supplied!".to_string())));
	}
	
	for vect in [loc, dest]{
		for i in vect{
			match i.parse::<f32>() {
				Ok(_) => continue,
				Err(_) => return Err(Unauth(status::Custom(Status::BadRequest, "Invalid Location data supplied!".to_string()))),
			}
		}
	}
	
	let newID = idGen();
	indata.id = newID;
	//let mut requests_store: Vec<WalkRequest> = *requests.active.lock().unwrap();
	requests.active.lock().unwrap().push(indata.into_inner());

	Ok(status::Created::new(newID.to_string()))
}


// initialize Postgres database connection defined in Rocket.toml
#[derive(Database)]
#[database("Postgres")]
struct Postgres(sqlx::PgPool);

#[launch]
fn rocket() -> _ {
	let (mut asend, arecv) = unbounded();
	let mut hndl = launch_alert_thread(arecv);
    rocket::build().mount("/", routes![index, profile, profileup, walk_request_handler])
	.attach(Postgres::init()).manage(Persist{alertsnd: asend, thrdhndl: hndl}).manage(WalkState{active: Arc::new(Mutex::new(vec!()))})
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
