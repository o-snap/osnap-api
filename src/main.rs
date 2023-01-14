#[macro_use] extern crate rocket;
use argon2::{password_hash::PasswordHasher,Argon2};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use rand_core::{RngCore, OsRng};
use serde::{Deserialize, Serialize};
use rocket::serde::{json::Json};


#[derive(Deserialize)]
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

#[derive(Serialize)]
pub struct Profile<'r> {
	user: &'r str,
	auth: &'r str,
	name: &'r str,
	age: u16,
	gender: &'r str,
	phone: &'r str,
	contacts: Vec<&'r str>,
//	ratings: Vec<u8>
}

#[derive(Deserialize)]
pub struct WalkRequest<'r> {
	user: &'r str,
	auth: &'r str,
	name: &'r str,
	dest: &'r str,
	loc: Location<'r>
}

#[derive(Deserialize)]
pub struct Location<'r> {
	latitude: &'r str,
	longitude: &'r str
}
#[derive(Serialize)]
pub struct PubProfile<'r> {
	name: &'r str,
	approxdist: &'r str,
	avgrating: f32,
	numratings: u32,
	picture: &'r str
}
#[derive(Deserialize)]
pub struct WalkResponce<'r>{
	user: &'r str,
	auth: &'r str,
	operation: &'r str,
	#[serde(default = "falsebool")]
	failsafe: bool
}

#[derive(Deserialize)]
pub struct DisarmRequest<'r>{
	user: &'r str,
	auth: &'r str,
	operation: &'r str,
	curlocation: Location<'r>
}

pub struct Status<'r>{
	status: &'r str
}

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


#[get("/")]
fn index() -> &'static str {
    "oSNAP API server v0.1.0-goathack"
}

#[post("/profile", format="json", data = "<request>")]
async fn profile(mut db: Connection<Users>, request: Json<ProfRequest<'_>>) -> Json<Profile>{

	match sqlx::query("SELECT * from Users WHERE name = ?").bind(request.user).fetch_one(&mut *db).await{
		Ok(entry) => {
			if entry.get("auth") != request.auth{
				return badprof();
				}
			if request.operation == "update"
			}
		}
		Err(e) => {
			return badprof;
		}
	}
}

#[derive(Database)]
#[database("Users")]
struct Users(sqlx::SqlitePool);

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index]).attach(Users::init())
}

fn badprof() -> Json<Profile>{
	Json(Profile{user:"none", auth:"none", name:"none", age:0, phone:"none", gender:"none",contacts:vec!()})
}

fn updateprof()
