use serde::{Deserialize, Serialize};
use serde_json::{Result};

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
	contacts: Vec<Contact<'r>>,
	prefs: Prefs<'r>
}

#[derive(Deserialize, Serialize)]
pub struct Contact<'r> {
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
pub struct TripResp<'r> {
	user: &'r str,
	tripID: &'r str
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
