#[macro_use] extern crate rocket;
use std::net::IpAddr;
use geoutils::{Location as Loc, Distance};
use rocket_db_pools::{sqlx::{self,Row}, Database, Connection};
use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, Value, json};
use rand_core::{RngCore, OsRng};
use std::{thread, time, time::Duration};
use crossbeam::channel::{self, unbounded, Receiver};

// TODO: Impliment request guards for data integrity & security
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
pub struct PubProfile {
	name: String,
	approxdist: String,
	avgrating: f32,
	numratings: u32,
	picture: String
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
pub struct AuthOnly<'r>{
	user: &'r str,
	auth: &'r str,
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

#[derive(Deserialize)]
struct Signin<'r>{
	user: &'r str,
	password: &'r str,
}
#[derive(Clone)]
struct AlertComm{
	armed: bool,
	start: time::Instant,
	name: String,
	dest: String,
	contacts: String
}

struct Persist {
    alertsnd: channel::Sender<AlertComm>,
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

	match sqlx::query("SELECT * from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await{
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
	sqlx::query("INSERT INTO Users (user,name,phone,password) VALUES(?, ?, ?, ?)")
	.bind(request.user)
	.bind(request.name)
	.bind(request.phone)
	.bind(request.password)
	.execute(&mut *db).await.unwrap();
	json!({"status": "ok"})

}

#[post("/signin", format="json", data = "<request>")]
async fn signin_handler(mut db: Connection<Users>, request: Json<Signin<'_>>) -> Value{
	let auth = OsRng.next_u32().to_string();
	match sqlx::query("SELECT password from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await {
		Ok(row) => {if row.get::<&str, &str>("password") == request.password {
			sqlx::query("UPDATE Users SET auth = ? WHERE user = ?").bind(&auth).bind(request.user).execute(&mut *db).await.unwrap();
			return json!({"login": auth})
		}}
		Err(_) => return json!({"login": "noaccount"})
	}
	json!({"login": "bad"})
}


#[post("/request", format="json", data = "<request>")]
async fn walk_request_handler(mut db: Connection<Users>, request: Json<WalkRequest<'_>>) -> Value{
	// make sure client is authorized
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		return json!({"request":"failed"});
	}
	let request_id = OsRng.next_u32().to_string();
	sqlx::query("INSERT INTO Requests (ID,User,Dest,Lat,Long) VALUES(?, ?, ?, ?, ?)")
	.bind(&request_id)
	.bind(request.user)
	.bind(request.dest)
	.bind(request.loc.latitude)
	.bind(request.loc.longitude)
	.execute(&mut *db).await.unwrap();
	json!({"request":request_id})
}


#[get("/trip/<ID>")]
async fn walk_wizard(mut db: Connection<Users>, ID: &str) -> String{
	for i in vec!(';','\\','*'){
		if ID.contains(i) {
			panic!("Illegal input in trip ID!");
		}
	}
	let trip = sqlx::query("SELECT Dest FROM Requests WHERE ID = ?").bind(&ID).fetch_one(&mut *db).await.unwrap();
	if trip.get::<&str, &str>("Trip").len() > 1 { // a suitable match was already found
		return String::from("Confirmed");
	}
	let dest: String = trip.get("Dest");
	let curpos = Loc::new(trip.get::<&str, &str>("Lat").parse::<f32>().unwrap(), trip.get::<&str, &str>("Long").parse::<f32>().unwrap());
	println!("Running matching wizard for trip {} bound for {}",&ID,&dest);
	let peers = sqlx::query("SELECT * FROM Requests WHERE Dest = ?").bind(&dest).fetch_all(&mut *db).await.unwrap();
	let mut leastdist = 1000000.0; //meters 
	let mut bestmatch_ID:String = "0000".to_string();
	let mut bestmatch_user:String = "nobody".to_string();
	for peer in peers{
		if peer.get::<&str, &str>("Dest") != dest {continue;}
		let pos = Loc::new(peer.get::<&str, &str>("Lat").parse::<f32>().unwrap(), peer.get::<&str, &str>("Long").parse::<f32>().unwrap());
		if pos.haversine_distance_to(&curpos).meters() < leastdist {
			leastdist = pos.haversine_distance_to(&curpos).meters();
			bestmatch_ID = peer.get("ID");
			bestmatch_user = peer.get("User");
		}
	}
	if leastdist < 1000000.0 {
		let trip_id = OsRng.next_u32().to_string();
		// Possible status codes: 0-cancelled, 2-pending, 4-inprogress, 6-complete
		sqlx::query("INSERT INTO Trips (ID, Dest, user1, user2, status) VALUES(?, ?, ?, ?, 2)")
		.bind(&trip_id)
		.bind(dest)
		.bind(trip.get::<&str, &str>("user"))
		.bind(bestmatch_user)
		.execute(&mut *db).await.unwrap();
		sqlx::query("UPDATE Requests SET Trip = ? WHERE ID = ?").bind(&trip_id).bind(ID).execute(&mut *db).await.unwrap();
		sqlx::query("UPDATE Requests SET Trip = ? WHERE ID = ?").bind(&trip_id).bind(bestmatch_ID).execute(&mut *db).await.unwrap();
		return String::from("Confirmed");
	}
	String::from("pending")
}

#[post("/trip/<ID>", format="json", data = "<request>")]
async fn walkman(mut db: Connection<Users>, request: Json<WalkResponce<'_>>, ID: &str) -> Value{
	// make sure client is authorized
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		return json!({"request":"error: unauthorized"});
	}
	// make sure trip exists 
	let trip = sqlx::query("SELECT * from Trips WHERE ID = ?").bind(ID).fetch_one(&mut *db).await.unwrap();
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
	sqlx::query("UPDATE Trips SET status = ?, ? = ? WHERE ID = ?")
	.bind(stat)
	.bind(user)
	.bind(fs)
	.bind(ID)
	.execute(&mut *db).await.unwrap();
	json!({"request":"ok"})
}

#[post("/trip/<ID>/buddy", format="json", data = "<request>")]
async fn peerinfo(mut db: Connection<Users>, request: Json<AuthOnly<'_>>, ID: &str) -> Json<PubProfile>{
	// make sure client is authorized
	if sqlx::query("SELECT auth from Users WHERE user = ?").bind(request.user).fetch_one(&mut *db).await.unwrap().get::<&str, &str>("auth") != request.auth{
		panic!("Unauthorized user!");
	}
	let trip = sqlx::query("SELECT * from Trips WHERE ID = ?").bind(ID).fetch_one(&mut *db).await.unwrap();
	if trip.is_empty(){
		panic!("Bad trip!");
	}
	let mut buddy = "user2";
	if trip.get::<&str, &str>("user1") != request.user{
		buddy = "user1";
	}
	let buddyname = trip.get::<&str, &str>(buddy);
	let buddyprof = sqlx::query("SELECT * FROM Users WHERE user = ?").bind(buddyname).fetch_one(&mut *db).await.unwrap();
	let ratings = buddyprof.get::<&str, &str>("ratings").split(",");
	let mut numrate = 0;
	let mut avg = 0.0;
	for i in ratings{
		numrate += 1;
		avg += i.parse::<f32>().unwrap();
	}
	let vname = buddyprof.get::<&str, &str>("name").to_string();

	Json(PubProfile { name: vname, approxdist: "Not implimented".to_string(), avgrating: avg, numratings: numrate, picture: "Not implimented!".to_string() })
}

#[derive(Database)]
#[database("Users")]
struct Users(sqlx::SqlitePool);

#[launch]
fn rocket() -> _ {
	let (asend, arecv) = unbounded();
	launch_alert_thread(arecv);
    rocket::build().mount("/", routes![index, profile, signup_handler, signin_handler, walk_request_handler, walk_wizard, walkman, peerinfo])
	.attach(Users::init()).manage(Persist{alertsnd: asend})
}

fn launch_alert_thread(reciever: Receiver<AlertComm>){
	thread::spawn(move || {
		loop {
			let mut store:Vec<AlertComm> = vec!();
			match reciever.try_recv(){
				Ok(msg) => {
					if msg.armed.clone(){
						let name = msg.name.clone();
						for i in store.as_slice(){
						if i.name == name{break;}
						}
						store.push(msg);
					}
				}
				Err(_) => thread::sleep(Duration::from_millis(15))
			}

		}
	});
}