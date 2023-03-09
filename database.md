# Backend Database Schema

The database uses an on-server postgresql instance. Earlier versions used SQLite but this was discarded due to its reliance on unsafe C code. 

## users table

This table stores user records

field | type | notes | example
---------|-----|---------|----------
usern | text (primary key) | the person's WPI username (user is restricted keyword) | jappleseed
name | text | the user's full name | Johnny Appleseed
age | integer | user age | 19
gender | text | custom text field (optional) | male
picture | text | server filesystem path of user picture | /srv/osnap/pictures/jappleseed.png
phone | text | phone number | 123-456-7890
contacts_names | text | comma-sepated list of names | John Doe,Jane Doe
contacts_phones | text | comma-separated list of numbers 234-567-8901,345-678-9012
ratings | text | comma-separated list of ratings from 1 to 5 | 5,4,4,3,4,5,5,4,2
auth | text | randomly generated authentication code | 1a422bfe9554704d37a

## trips table

field | type | notes | example
---------|-----|---------|----------
users | text | comma-separated list of usernames participating in trip | jappleseed,jellington
failsafe | integer | binary encoding of weather each user has endabled failsafe$^*$ | 3
id | text | randomly generated trip identifier
time | integer | UNIX timestamp of when the trip initiated
origin | text | comma-separated coordinates | 0.00000,0.000000
destination | text | comma-separated coordinates | 1.00000,-1.000000
status | integer | status of the trip$_†$ | 1

*The position of each bit of the integer corresponds to a user, with the LSB corresponding to the 1st user in the comma-separated list. A 1 signifies that that user enabled the failsafe. For example, if the trip has 3 users and the first and third have enabled failsafe, the failsafe encoding will be 101b = 5.

† Trip statuses: 0 - suggested, 1 - in progress, 2 - complete, 3 - aborted
