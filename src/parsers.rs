use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};
use std::string::String;
// Define structs for automatic JSON Parsing 

// struct used to serve user data in JSON
#[derive(Serialize, Deserialize, Clone)]
pub struct Profile {
	pub user: String,
	pub name: String,
	pub age: i16,
	pub gender: String,
	pub phone: String,
	pub contacts_names: Vec<String>,
	pub contacts_phones: Vec<String>,
	pub ratings: String
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProfileUpdate<'r> {
	#[serde(default = "none")]
	pub name: &'r str,
	#[serde(default = "defaultint")]
	pub age: i16,
	#[serde(default = "none")]
	pub gender: &'r str,
	#[serde(default = "none")]
	pub phone: &'r str,
	#[serde(default = "none")]
	pub contacts_names: &'r str,
	#[serde(default = "none")]
	pub contacts_phones: &'r str
}



pub struct Location {
	pub raw_coords: String,
	pub lat: String,
	pub long: String,
	pub street: String, //google API provided address
	pub place_id: String
}

impl From<String> for Location{
	fn from(inval: String) -> Self{
		let mut coords = inval.split(',');
		// TODO: add google maps API integration
		Location{raw_coords: inval.clone(), lat: coords.next().unwrap_or("INVALID").to_string(), long: coords.next().unwrap_or("INVALID").to_string(), street: "Not Implimented".to_string(), place_id: "Not Implimented".to_string()}
	}
}

// struct to serve profile data of other users, does not include sensitive info
#[derive(Serialize, Deserialize)]
pub struct PubProfile {
	pub name: String,
	pub approxdist: String,
	pub avgrating: f32,
	pub numratings: u32,
	pub phone: String
}

// enum to provide walk status during trip start
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum WalkStat {
	Ready {status: String, buddy: Vec<PubProfile>},
	StatOnly {status: String}
}

// used to deserialized user responses to walk buddies suggested by server
#[derive(Deserialize)]
pub struct WalkResponce<'r>{
	pub operation: &'r str
}

// communications from client while walk in in progress
#[derive(Serialize, Deserialize)]
pub struct InFlightSet<> {
	pub curlocation: String,
	#[serde(default = "falsebool")]
	pub distress: bool
}

// struct to contain buddys' locations for inflight
#[derive(Serialize, Deserialize)]
struct BuddyLoc {
	name: String,
	curlocation: String
}

//data to send to client about walk and buddy locations
#[derive(Serialize, Deserialize)]
pub struct InFlightGet {
	status: String,
	buddy: Vec<BuddyLoc>
}

// data from client when trip is over
#[derive(Serialize, Deserialize)]
pub struct TripEnd {
	curlocation: String,
	rating: i16
}

#[derive(Deserialize)]
pub struct Signup<'r>{
	pub user: &'r str,
	pub name: &'r str,
	pub phone: &'r str,
	pub password: &'r str,
}
// different from authonly as this is a password, not an auth token 
#[derive(Deserialize)]
pub struct Signin<'r>{
	pub user: &'r str,
	pub password: &'r str,
}


fn none() -> &'static str {
	"none"
}
pub fn defaultint() -> i16 {
	32767
}

fn falsebool()->bool{
	false
}

// rule sets for input sanitization
pub enum FieldType {
	Alpha,
	Num,
	AlphaNum,
	Phone
}
// Santiizes data to be sent to SQL database. Does not return errors because this should be last line of defence. It's the frontend's job to give the user nice errors about their input, it's this functions job to prevent injection.
pub fn sanitizer(inval: &str, kind: FieldType) -> String {
	let mut outval = String::new();
	for ch in inval.chars(){
		match kind{
			FieldType::Alpha => {
			if ch.is_alphabetic() {outval.push(ch)}	
			}
			FieldType::Num => {
			if ch.is_digit(0) {outval.push(ch)}	
			}
			FieldType::AlphaNum => {
			if ch.is_alphanumeric() {outval.push(ch)}	
			}
			FieldType::Phone => {
			if ch.is_digit(0) || ch == '-' || ch ==',' {outval.push(ch)}
			if outval.len() > 12 {outval.clear()}
			}
		}
	}
	outval	
}
