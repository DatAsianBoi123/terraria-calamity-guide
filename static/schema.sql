DO $$ BEGIN
  CREATE TYPE player AS (
    id NUMERIC(20, 0),
    class SMALLINT
  );
  EXCEPTION WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS playthroughs (
  id SERIAL PRIMARY KEY,
  owner NUMERIC(20, 0) UNIQUE,
  players player[] NOT NULL,
  stage SMALLINT NOT NULL CHECK (stage BETWEEN 0 AND 14),
  started TIMESTAMP
);

