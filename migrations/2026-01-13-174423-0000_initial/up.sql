CREATE TABLE users (
	id SERIAL PRIMARY KEY,
	username text UNIQUE NOT NULL,
	passhash text NOT NULL
);
