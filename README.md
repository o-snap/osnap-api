# O-Snap API
This is the backend API for the goathacks O-Snap project.

[![Build](https://github.com/o-snap/osnap-api/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/o-snap/osnap-api/actions/workflows/rust.yml)

# API Reference
All interactions with the API are via JSON HTTPS POSTs and GETs. 

**Important:** All requests other than login and signup must also include a cookie named `osnap-authtoken` in the request header. This will contain the encrypted authentication token issued by the API during the signin process. 

## Get profile
The app or web frontend can request the entire user profile and cache the data to use in the interface and reduce API load. An example request would be in the form of a GET to `/api/profile/{username}` where `username` is the user's username (without the curly braces). 

If the authentication token matches the username of the request, the API server would then respond in the following format:
```JSON
{
	"user": "jappleseed",
	"name": "Jane Appleseed",
	"age": 19,
	"gender": "female",
	"picture": "/pictures/eca7fd8979e2408d",
	"phone": "123-456-7890",
	"Contacts": "XXX-XXX-XXXX,YYY-YYY-YYYY,ZZZ-ZZZ-ZZZZ",
	"ratings": "5,4,5,2,5,4,3,5"

}
```
Note that the `gender` field can contain user-input text. The API will try to sanitize all user input but plan accordingly in handling of the field.

If the auth token is invalid, the server will respond with a `401` unauthorized status code. 

## Update Profile
Updating a profile is very similar to getting profile data, however it is a POST to `/api/profile/{username}` instead of a GET. The request body only needs to send the field(s) that have changed. For example:

```JSON
{
	"age": 20,
	"gender": ""
}
```

Note that only some fields can be updated via this method. Ratings must be updated via the rating functionality discussed later.

As this process does not require any data in responce, this is one of the few endpoints that does *not* return JSON data, only HTTP status codes. If the operation is successful, the server will return `200`, if unsuccessful it will respond `400`, if the auth token is invalid, it will respond `401`.

## Initiate Walk

The user can request a partner by POSTING to `/api/request` in the following format
```JSON
{
	"destination": 	{
		"latitude": "0.000000",
		"longitude": "0.000000",
	},
	"curlocation":
	{
		"latitude": "0.000000",
		"longitude": "0.000000",
	},
	"minbuddies": 1,
	"maxbuddies": 2,
	"time": 1681978800
}
```

The server will then try to match the user with other user(s) based on their preferences, destination, and current location. The `time` field is the desired time of departure expressed in UNIX timestamp. As this process is not instant, the server will issue the user a session ID which will be used for any further requests pertaining to the trip. The response will look like:

```JSON
{
	"trip": "9a95"
}
```

## Starting a Trip
Subsequent requests concerning a trip will be directed to the `/api/trip/{tripID}` endpoint where `{tripID}` is replace with the ID issued by the server in the request phase. While waiting for a partner to be found, clients can GET the endpoint which will return JSON containing the status of the trip. The three possible values are 

* `pending` - the server is still trying to find a walking buddy
* `confirmed` - A partner was found and we're ready to go
* `failed` - A partner could not be found
* `inprog` - both users accepted the walk and it is surrently in process
* `end` - the walk finished successfully 


If a buddy was found, the server will also provide the buddy(s)' information:

```JSON
{
	"status": "confirmed",
	"buddy": [
			{
			"name": "Johnny Appleseed",
			"approxdist": "32 feet",
			"picture": "/pictures/468136b6f3",
			"phone": "XXX-XXX-XXXX",
			"avgrating": 4.1,
			"numratings": 23
			},
			{
			"name": "Jimmy McGill",
			"approxdist": "71 feet",
			"picture": "/pictures/29ac2b6f8",
			"phone": "XXX-XXX-XXXX",
			"avgrating": 4.6,
			"numratings": 41
			}
			]
}
```

### Trip Actions

Both users are presented with the option to `accept`, `failsafe` or `decline` the trip which will be done by POSTing JSON to `/api/trip/{tripID}`. Responding `failsafe` at this point implicitly accepts the walk and enables the failsafe system. The failsafe service automatically generates an emergency alert if the destination is not reached within a set time. The failsafe can only be disarmed if the user's geolocation generally matches their destination. 

Example:
```JSON
{
	"operation": "accept"
}
```

The server will respond with an HTTP status code, `200` if the request was recieved and processed succesfully, regardless of the requested operation. It will respond `400` if the recieved data was not `accept`, `failsafe`, or `decline`, and as always will respond `401` if the authtoken cookie is invalid. 

## During Trip
During the trip, the user may keep the UI open (using built-in google maps integration for directions) or may lock their phone. After both users accept the walk and it is officially initiated, the frontend should periodically send the user's current location to the API via JSON POST to `/api/trip/{tripID}/inflight`. the frontend should also query the buddy's location by GETing to the same endpoint. This is for 2 reasons: to help the buddies locate eachother and to provide the failsafe mechanism with as up-to-date location data as possible if something were to go wrong. The interface should also present the user with a button to manually activate a distress signal, this will set the `distress` field to `true`. The `distress` field does not have to be specified and defaults to `false`. Example POSTs:

```JSON
{
	"curlocation":
	{
		"latitude": "0.000000",
		"longitude": "0.000000",
	}
}
```
or
```JSON
{
	"curlocation":
	{
		"latitude": "0.000000",
		"longitude": "0.000000",
	},
	"distress": true
}
```

When a valid client performs a GET request against the `inflight` endpoint, the server will respond with:

```JSON
{
	"status": "inprog",
	"buddy": [
			{
			"name": "Johnny Appleseed",
			"curlocation":
				{
					"latitude": "0.000000",
					"longitude": "0.000000",
				}
			},
			{
			"name": "Jimmy McGill",
			"curlocation":
				{
					"latitude": "0.000000",
					"longitude": "0.000000",
				}
			}
		]
}
```

In this context, curlocation is the *buddy's* current location and `status` is the same as the listed statuses in the "Starting a Trip" section.

## End of Trip

When the user terminates their trip, the API server *must* be notified. The frontend should send the user's location at the time of walk termination for use in the failsafe system and the rating of the buddy. The client will POST to `/api/trip/{tripID}/end` the following:
```JSON
{
	"curlocation":
		{
			"latitude": "0.000000",
			"longitude": "0.000000",
		},
	"rating": [4]
}
```

Ratings should be a list of integers, 1 for each buddy. The order of the list should be by last name alphabetical order. 

This endpoint **must** be called whenever a trip is ended by the user. The terminating user's buddy(s) will be notified by an update to the `status` field when the client queries `/api/trip/{tripID}` via a GET request. 

## Authentication
In future releases, we hope to integrate with SSO via microsoft accounts but for now the API's server stores hashed user credentials. To add a user, POST to `/api/signup`:
```JSON
{
	"user": "jappleseed",
	"name": "Johnny Appleseed",
	"phone": "XXX-XXX-XXXX",
	"password": "{USER PASSWORD}"
	
}
```

The server needs to verify the user's status as a member of the WPI community. The easiest (and cheapest) way to impliment this is by checking that, during account creation, the user's IP is within WPI's public IPv4 block. Sending an email would be a more reliable method, but could not be implimented in time. The server will return an HTTP status code responce.

If the server returns `200` (Ok), direct users to the signin page or, better yet, cache their credentials client-side and automatically POST them to `/api/signin` to continue.

To log in an existing user, POST to `/api/signin`:
```JSON
{
	"user": "jappleseed",
	"password": "{USER PASSWORD}"
	
}
```
to which the server will respond with an hp status. If the status is `200` (Ok), the reply will also contain a cookie named `osnap-authtoken`. This cookie **MUST** be stored client side as it will be used in all future interactions wil ther server. This should be done by including it in the HTTP headers. 
