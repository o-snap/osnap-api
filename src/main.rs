#[macro_use] extern crate rocket;
use argon2::{password_hash::PasswordHasher,Argon2};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use rand_core::{RngCore, OsRng};
// use rocket::serde::{Deserialize, json::Json, json::Map};
use serde::{Deserialize, Serialize};
use serde_json::{Result};
#[derive(Deserialize)]
struct ProfRequest<'r> {
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
	contacts: Vec<Contact<'r>>,
	#[serde(default = "defaultprefs")] 
	prefs: Prefs<'r>
}

#[derive(Serialize)]
struct Profile<'r> {
	user: &'r str,
	auth: &'r str,
	name: &'r str,
	age: u16,
	gender: &'r str,
	contacts: Vec<Contact<'r>>,
	prefs: Prefs<'r>
}

#[derive(Deserialize, Serialize)]
struct Contact<'r> {
	name: &'r str,
	method: &'r str,
	addr: &'r str
}

#[derive(Deserialize, Serialize)]
struct Prefs<'r> {
	age: u16,
	gender: &'r str,
	minrating: f32
}

#[get("/")]
fn index() -> &'static str {
    "O-Snap API server v0.1.0-goathack"
}



#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}

fn none() -> &'static str {
	"none"
}
fn defaultint() -> u16 {
	65535
}

fn defaultprefs<'r>() -> Prefs<'r>{
	Prefs{age: 65535, gender: "none", minrating: -1.0}
}

fn emptyvec<'r>() -> Vec<Contact<'r>>{
	vec!()
}
