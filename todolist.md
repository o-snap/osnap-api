# Jake's massive todo list

This is not all encompassing, mainly fixes to things already partially implimented

## Move Requests from a database table to a persistant in-memory collection of structs

There's no reason to use something as cumbersome as a relational database to store ephemeral request records. In-memory storage would be much better suited to the task

## In-memory component for trips

It's a good idea to have both an SQL-database component for the trip and a in-memory object for the trip. This in-memory object could more easily keep track of current locations and so on whereas the postgres component will be for logging purposes

## Migrate database system from SQLite to postgres

In progress - uses the same database driver and langueage so should be trivial

## Write test cases

Self explanatory - we currently have 0% test coverage

## Impliment microsoft oauth authentication

we need a login mechanism and I'd really rather not bother with handling passwords if I can help it

## overhaul trips structure

Overhaul trips structure to comply with that in `database.md`. For one, we need to support more that 2 users
