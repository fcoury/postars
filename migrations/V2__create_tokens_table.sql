CREATE TABLE tokens (
  token_id serial PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users (id),
  access_token varchar(5000) NOT NULL,
  refresh_token varchar(5000) NOT NULL,
  created_at timestamp NOT NULL DEFAULT NOW(),
  updated_at timestamp NOT NULL DEFAULT NOW()
);

