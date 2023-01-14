#[macro_use] extern crate rocket;
use argon2::{password_hash::PasswordHasher,Argon2};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use rand_core::{RngCore, OsRng};
// use rocket::serde::{Deserialize, json::Json, json::Map};
use serde::{Deserialize, Serialize};
use serde_json::{Result};
#[derive(Deserialize)]
struct Prof_request<'r> {
	user: &'r str,
	auth: &'r str,
	operation: &'r str,
	name: &'r str,
	age: u16,
	gender: &'r str,
	contacts: Vec<Contact<'r>>,
	// TODO: Add prefs and default vals
	
}

#[derive(Deserialize, Serialize)]
struct Contact<'r> {
	name: &'r str,
	method: &'r str,
	addr: &'r str
}



#[get("/")]
fn index() -> &'static str {
    "O-Snap API server v0.1.0-goathack"
}



#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
