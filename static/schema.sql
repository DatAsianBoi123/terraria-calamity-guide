DO $$ BEGIN
  CREATE TYPE player AS (
    id NUMERIC(20, 0),
    class SMALLINT
  );
  EXCEPTION WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS playthroughs (
  id SERIAL PRIMARY KEY,
  owner NUMERIC(20, 0) NOT NULL UNIQUE,
  players player[] NOT NULL,
  stage SMALLINT NOT NULL CHECK (stage BETWEEN 0 AND 14),
  started TIMESTAMP
);

CREATE TABLE IF NOT EXISTS issues (
  id INT PRIMARY KEY,
  author NUMERIC(20, 0) NOT NULL,
  class SMALLINT NOT NULL CHECK (class BETWEEN 0 AND 4),
  stage SMALLINT NOT NULL CHECK (stage BETWEEN 0 AND 14),
  incorrect VARCHAR(255) NOT NULL,
  correct VARCHAR(255) NOT NULL,
  created_at TIMESTAMP DEFAULT now()
);

