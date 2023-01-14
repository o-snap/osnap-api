# O-Snap API
This is the backend API for the goathacks O-Snap project

# API Reference
All interactions with the API are done by POSTing JSON to the `/endpoint` directory of the server.

Every request to the endpoint will have 3 common fields: `user`, `auth`, and `operation`. User is the username of the user for which data is requested, auth is an authorization token issued to the client, and operation varies by request type. 

## Get profile
The app or web frontend can request the entire user profile and cache the data to use in the interface and reduce API load. An example request would be in the form of a POST to `/endpoint` of the following JSON:
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
	"Contacts": [
		{
			"name": "roomate",
			"method": "phone",
			"number": "XXX-XXX-XXXX"
		}
	]
	"ratingavg": 4.3
	"ratingcount": 15
}
```
Note that the `gender` field can contain user-input text. The API will try to sanitize all user input but plan accordingly in handling of the field.

## Update Profile
Updating a profile is very similar to getting profile data. The client only needs to send the field(s) that have changed.
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
