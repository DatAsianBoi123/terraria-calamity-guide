DO $$ BEGIN
  CREATE TYPE powerup AS ENUM (
    'LifeCrystal',
    'LifeFruit',
    'BloodOrange',
    'MiracleFruit',
    'Elderberry',
    'Dragonfruit',
    'ManaCrystal',
    'CometShard',
    'EtherealCore',
    'PhantomHeart',
    'MushroomPlasmaRoot',
    'InfernalBlood',
    'RedLightningContainer',
    'ElectrolyteGelPack',
    'StarlightFuelCell',
    'Ectoheart',
    'HermitBox',
    'DemonHeart',
    'CelestialOnion'
  );
EXCEPTION
  WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
  CREATE TYPE health_potion AS ENUM ('Lesser', 'Normal', 'Greater', 'Super', 'Supreme', 'Omega');
EXCEPTION
  WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS loadouts (
  id SERIAL PRIMARY KEY,
  class SMALLINT NOT NULL CHECK (class BETWEEN 0 AND 4),
  stage SMALLINT NOT NULL CHECK (stage BETWEEN 0 AND 14),
  armor TEXT NOT NULL,
  weapons TEXT[] NOT NULL CHECK (array_ndims(weapons) = 1 AND array_length(weapons, 1) = 4),
  equipment TEXT[] NOT NULL
);

CREATE TABLE IF NOT EXISTS extra_loadout_data (
  id SERIAL PRIMARY KEY,
  loadout_id INT REFERENCES loadouts(id),
  label VARCHAR(255) NOT NULL,
  data TEXT[] NOT NULL
);

CREATE TABLE IF NOT EXISTS stage_data (
  stage SMALLINT PRIMARY KEY CHECK (stage BETWEEN 0 AND 14),
  health_potion health_potion,
  powerups powerup[]
);

CREATE TABLE IF NOT EXISTS playthroughs (
  owner NUMERIC(20, 0) PRIMARY KEY,
  stage SMALLINT NOT NULL CHECK (stage BETWEEN 0 AND 14),
  started TIMESTAMP
);

CREATE TABLE IF NOT EXISTS playthrough_players (
  user_id NUMERIC(20, 0) PRIMARY KEY,
  playthrough_owner NUMERIC(20, 0) NOT NULL REFERENCES playthroughs(owner) ON DELETE CASCADE,
  class SMALLINT NOT NULL CHECK (class BETWEEN 0 AND 4)
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

