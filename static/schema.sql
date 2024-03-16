CREATE TABLE IF NOT EXISTS playthroughs (
  owner NUMERIC(20, 0) PRIMARY KEY,
  stage SMALLINT NOT NULL CHECK (stage BETWEEN 0 AND 14),
  started TIMESTAMP
);

CREATE TABLE IF NOT EXISTS playthrough_players (
  user_id NUMERIC(20, 0) PRIMARY KEY,
  playthrough_owner NUMERIC(20, 0) NOT NULL REFERENCES playthroughs(owner) ON DELETE CASCADE,
  class SMALLINT NOT NULL
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

