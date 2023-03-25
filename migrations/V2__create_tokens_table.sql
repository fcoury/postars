CREATE TABLE tokens (
  id serial PRIMARY KEY,
  user_id integer NOT NULL REFERENCES users (id),
  access_token varchar(5000) NOT NULL,
  refresh_token varchar(5000) NOT NULL,
  created_at timestamp NOT NULL DEFAULT NOW(),
  updated_at timestamp NOT NULL DEFAULT NOW()
);

CREATE OR REPLACE FUNCTION update_tokens_modified_at ()
  RETURNS TRIGGER
  AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER tokens_modified_at_trigger
  BEFORE UPDATE ON tokens
  FOR EACH ROW
  EXECUTE FUNCTION update_tokens_modified_at ();

