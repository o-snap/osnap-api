# O-Snap API
This is the backend API for the goathacks O-Snap project

# API Reference
All interactions with the API are done by POSTing JSON to the server.

Every request to the endpoint will have 2 common fields: `user`, `auth`. User is the username of the user for which data is requested and auth is an authorization token issued to the client.

## Get profile
The app or web frontend can request the entire user profile and cache the data to use in the interface and reduce API load. An example request would be in the form of a POST to `/profile` on the server of the following JSON:
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"operation": "fetch"
}
```
The API server would then respond with a responce of the following format:
```JSON
{
	"user": "jappleseed",
	"name": "Jane Appleseed",
	"age": 19,
	"gender": "female",
	"picture": "/pictures/eca7fd8979e2408d",
	"Contacts": "XXX-XXX-XXXX,YYY-YYY-YYYY,ZZZ-ZZZ-ZZZZ",
	"ratings": "5,4,5,2,5,4,3,5"

}
```
Note that the `gender` field can contain user-input text. The API will try to sanitize all user input but plan accordingly in handling of the field.

## Update Profile
Updating a profile is very similar to getting profile data (POST to `/profile`). The client only needs to send the field(s) that have changed.
Example:
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"operation": "update",
	"age": 20
}
```
Note that only some fields can be updated via this method. Ratings must be updated via the rating functionality discussed below.

## Initiate Walk
The user can request a partner by POSTING to `/request` in the following format
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"destination": "FaradayHall"
	"curlocation":
	[
		{
			"latitude": "0.000000",
			"longitude": "0.000000",
		}
	]
}
```

The server will then try to match the user with another user based on their preferences, destination, and current location. As this process is not instant, the server will issue the user a session ID which will be used for any further requests pertaining to the trip. The response will look like:

```JSON
{
	"trip": "9a95"
}
```

## Trip information
Subsequent requests concerning a trip will be directed to the `/trip/{tripID}` endpoint where `{tripID}` is replace with the ID issued by the server in the request phase. While waiting for a partner to be found, clients can GET the endpoint which will return a string containing the status of the trip. The three possible values are 
* `pending` - the server is still trying to find a walking buddy
* `confirmed` - A partner was found and we're ready to go
* `failed` - A partner could not be found


No further data is provided over the GET interface for privacy. Once the status changes to `confirmed`, the client should POST to the same `/trip/{tripID}` endpoint:
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"operation": "fetch"
}
```
The server will then respond with:
```JSON
{
	"name": "Johnny Appleseed",
	"approxdist": "32 feet",
	"picture": "/pictures/468136b6f3",
	"avgrating": 4.1,
	"ratings": 23
}
```
## Trip Actions
Both users are presented with the option to `accept` or `decline` the trip which will be done via the `operation` field. Users can also request the failsafe service which automatically generates an emergency alert if the destination is not reached within a set time. The failsafe can only be disarmed if the user's geolocation generally matches their destination. 

Example:
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"operation": "accept",
	"failsafe": true
}
```
or
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"operation": "decline"
}
```
The server will respond with JSON. If the user did not request failsafe, it will simply be either
```JSON
{
	"status": "ok"
}
```
or
```JSON
{
	"status": "error: {}"
}
```

## Failsafe
When the user gets to their destination and if failsafe is enabled, they must notify the server. The client will POST to `/trip/{tripID}` the following:
```JSON
{
	"user": "jappleseed",
	"auth": "3ae33e1cb43b75efdaf5eca7",
	"operation": "disarm",
	"curlocation":
		[
			{
				"latitude": "0.000000",
				"longitude": "0.000000",
			}
		]
}
```
To which the server will respond either 
```JSON
{
	"status": "ok"
}
```
or
```JSON
{
	"status": "error: {}"
}
```
where the error may be "not at destination", or "bad phrase". 5 unsuccessful disarm attempts will result in an alert condition. 

## Authentication
In future releases, we hope to integrate with SSO via microsoft accounts but for now the API's server stores hashed user credentials. To add a user, POST to `/signup`:
```JSON
{
	"email": "jappleseed@wpi.edu",
	"name": "Johnny Appleseed",
	"phone": "XXX-XXX-XXXX",
	"password": "ARGON2 HASHED PASSWORD"
	
}
```
**NEVER** send the user's password in plaintext to the API!!!
The server needs to verify the user's status as a member of the WPI community. The easiest (and cheapest) way to impliment this is by checking that, during account creation, the user's IP is within WPI's public IPv4 block. Sending an email would be a more reliable method, but could not be implimented in time. The server will return a JSON with a single field, `status`, which will be set to either `unauthorized` or `ok`.
If the server returns OK, direct users to the signin page or, better yet, cache their credentials and automatically POST them to `/signin` to continue.

To log in an existing user, POST to `/signin`:
```JSON
{
	"email": "jappleseed@wpi.edu",
	"password": "ARGON2 HASHED PASSWORD"
	
}
```
to which the server will respond with an auth token to be used in future transactions (should be stored as a secure cookie client-side) or the word "bad"

```JSON
{
	"login": "29ba3cf90733d6ae908fbf"	
}
```
or
```JSON
{
	"login": "bad"	
}
```
or 
```JSON
{
	"login": "noaccount"	
}
```