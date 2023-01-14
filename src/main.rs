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
	contacts: Vec<Contact<'r>>,
	#[serde(default = "defaultprefs")]
	prefs: Prefs<'r>
}

#[derive(Serialize)]
pub struct Profile<'r> {
	user: &'r str,
	auth: &'r str,
	name: &'r str,
	age: u16,
	gender: &'r str,
	phone: &'r str,
	contacts: Vec<Contact<'r>>,
	prefs: Prefs<'r>
}

#[derive(Deserialize, Serialize)]
pub struct Contact<'r> { // emergency contact
	name: &'r str,
	method: &'r str,
	addr: &'r str
}

#[derive(Deserialize, Serialize)]
pub struct Prefs<'r> {
	age: u16,
	gender: &'r str,
	minrating: f32
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

fn defaultprefs<'r>() -> Prefs<'r>{
	Prefs{age: 65535, gender: "none", minrating: -1.0}
}

fn emptyvec<'r>() -> Vec<Contact<'r>>{
	vec!()
}

fn falsebool()->bool{
	false
}


#[get("/")]
fn index() -> &'static str {
    "O-Snap API server v0.1.0-goathack"
}

#[post("/profile", format="json", data = "<request>")]
async fn profile(mut db: Connection<Users>, request: Json<ProfRequest<'_>>) -> Json<Profile>{

	match sqlx::query("SELECT * from Users WHERE name = ?").bind(request.name).fetch_one(&mut *db).await{
		Ok(entry) => {
			if entry.get("auth") != request.auth{
				let p = Prefs {age:0,gender:"none",minrating:0.0};
				return Json(Profile{user:"none", auth:"none", name:"none", age:0, phone:"none", gender:"none",contacts:vec!(),prefs:p});
			}
		}
		Err(e) => {
			let p = Prefs {age:0,gender:"none",minrating:0.0};
			return Json(Profile{user:"none", auth:"none", name:"none", age:0, phone:"none", gender:"none",contacts:vec!(),prefs:p})
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

fn checkauth<'r>(key: &'r str) -> Result<&'r str, &'r str>{
	
	Err("unauthorized")
}
