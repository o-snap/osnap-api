#[macro_use] extern crate rocket;
use argon2::{password_hash::PasswordHasher,Argon2};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use rand_core::{RngCore, OsRng};

#[get("/")]
fn index() -> &'static str {
    "O-Snap API server v0.1.0-goathack"
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index])
}
