#[macro_use] extern crate rocket;
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

#[derive(Serialize, Clone)]
pub struct Profile {
	user: String,
	auth: String,
	name: String,
	age: u16,
	gender: String,
	phone: String,
	contacts: Vec<String>,
	ratings: String
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
			if entry.get::<&str, &str>("auth") != request.auth{
				return badprof();
				}
			if request.operation == "update"{
			updateprof();
			}
			let vuser = request.user.to_string();
			let vauth = request.auth.to_string();
			let vname= entry.get::<&str, &str>("name").to_string();
			let vage = entry.get::<u16, &str>("age");
			let vphone = entry.get::<&str, &str>("name").clone().to_string();
			let vgender = entry.get::<&str, &str>("gender").clone().to_string();
			let vcontacts = entry.get::<&str, &str>("contacts").split_whitespace().collect::<Vec<&str>>();
			let mut wcontacts: Vec<String> = vec!();
			for i in vcontacts{
				wcontacts.push(i.to_string());
			}
			let vratings = entry.get::<&str, &str>("ratings").clone().to_string();
			let newprof = Profile{
				user: vuser,
				auth: vauth,
				name: vname,
				age: vage,
				phone: vphone,
				gender: vgender,
				contacts: wcontacts,
				ratings: vratings
			};
			return Json(newprof);
		}
		Err(_) => {
			return badprof();
		}
	}
}

#[derive(Database)]
#[database("Users")]
struct Users(sqlx::SqlitePool);

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, profile]).attach(Users::init())
}

fn badprof() -> Json<Profile>{
	Json(Profile{user:"none".to_string(), auth:"none".to_string(), name:"none".to_string(), age:0, phone:"none".to_string(), gender:"none".to_string(),contacts:vec!(),ratings:"".to_string()})
}

fn updateprof(){
	
}
