#[macro_use] extern crate rocket;
use std::net::IpAddr;

use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rand_core::{RngCore, OsRng};

#[derive(Deserialize, Clone)]
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

#[derive(Deserialize)]
struct Signup<'r>{
	user: &'r str,
	name: &'r str,
	phone: &'r str,
	password: &'r str,
}

//goofy ahh functions due to serde quirk
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
				if request.age != 0{
					sqlx::query("UPDATE Users SET age = ? WHERE user = ?").bind(request.age).bind(request.user).execute(&mut *db).await.unwrap();
				}
				if request.gender != "none"{
					sqlx::query("UPDATE Users SET gender = ? WHERE user = ?").bind(request.gender).bind(request.user).execute(&mut *db).await.unwrap();
				}
				if request.contacts.len() > 0{
					let mut tmp = String::new();
					for i in request.contacts.as_slice(){
						tmp.push_str(i);
						tmp.push(',');
					}
					sqlx::query("UPDATE Users SET contacts = ? WHERE user = ?").bind(tmp).bind(request.user).execute(&mut *db).await.unwrap();
				}
				if request.name != "none"{
					sqlx::query("UPDATE Users SET name = ? WHERE user = ?").bind(request.name).bind(request.user).execute(&mut *db).await.unwrap();
				}
			}
			let vuser = entry.get::<&str, &str>("user").to_string();
			let vname= entry.get::<&str, &str>("name").to_string();
			let vage = entry.get::<u16, &str>("age");
			let vphone = entry.get::<&str, &str>("name").clone().to_string();
			let vgender = entry.get::<&str, &str>("gender").clone().to_string();
			let vcontacts = entry.get::<&str, &str>("contacts").split(",").collect::<Vec<&str>>();
			let mut wcontacts: Vec<String> = vec!();
			for i in vcontacts{
				wcontacts.push(i.to_string());
			}
			let vratings = entry.get::<&str, &str>("ratings").clone().to_string();
			let newprof = Profile{
				user: vuser,
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

fn badprof() -> Json<Profile>{
	Json(Profile{user:"none".to_string(), name:"none".to_string(), age:0, phone:"none".to_string(), gender:"none".to_string(),contacts:vec!(),ratings:"".to_string()})
}

#[post("/signup", format="json", data = "<request>")]
async fn signup_handler(mut db: Connection<Users>, request: Json<Signup<'_>>, addr: IpAddr) -> Value{
	let mut ip = addr.to_string();
	ip = ip.get(0..7).unwrap().to_string();
	if ip != "130.215" && ip != "207.174"{
		// assume they're at WPI if the're within these address ranges (I know it's a really stupid way to do this but i'm on a short timetable)
		return json!({
			"status": "unauthorized"
		});
	}
	let auth = OsRng.next_u32().to_string();
	sqlx::query("INSERT INTO Users (user,auth,name,phone,password) VALUES(?, ?, ?, ?, ?)")
	.bind(request.user)
	.bind(auth)
	.bind(request.name)
	.bind(request.phone)
	.bind(request.password)
	.execute(&mut *db).await.unwrap();
	json!({"status": "ok"})

}
#[derive(Database)]
#[database("Users")]
struct Users(sqlx::SqlitePool);

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, profile, signup_handler]).attach(Users::init())
}

