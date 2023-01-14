#[macro_use] extern crate rocket;
use argon2::{password_hash::PasswordHasher,Argon2};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use rand_core::{RngCore, OsRng};
use serde::{Deserialize, Serialize};
mod structures;
use rocket::serde::{json::Json};

#[get("/")]
fn index() -> &'static str {
    "O-Snap API server v0.1.0-goathack"
}

#[post("/profile", format="json", data = "<request>")]
fn profile(request: Json<structures::ProfRequest<'_>>) -> Json<structures::Profile>{
	
}


#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}

fn checkauth<'r>(key: String) -> Result<&'r str, &'r str>{
	
	Err("unauthorized")
}
