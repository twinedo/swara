-- Migration 002: Users table
-- Stores auth credentials, Pro status, and optional Singgah (region hop) location

CREATE TABLE users (
  id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
  username         TEXT         UNIQUE NOT NULL,
  password_hash    TEXT         NOT NULL,
  is_pro           BOOLEAN      NOT NULL DEFAULT false,

  -- 'gps'    → frontend uses device location
  -- 'manual' → Pro user has set a Singgah override location
  location_mode    TEXT         NOT NULL DEFAULT 'gps'
                   CHECK (location_mode IN ('gps', 'manual')),

  -- Only populated when is_pro = true AND location_mode = 'manual'
  -- Stored as WGS84 geographic point (lon, lat)
  selected_location GEOGRAPHY(POINT, 4326) DEFAULT NULL,

  created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
  updated_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Index for username lookups (login)
CREATE INDEX users_username_idx ON users (username);

-- Partial index — only pro users can have a selected_location, so only index those rows
CREATE INDEX users_selected_location_idx ON users USING GIST (selected_location)
  WHERE selected_location IS NOT NULL;

-- Auto-update updated_at on row change
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at
  BEFORE UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();

COMMENT ON TABLE  users                    IS 'Swara user accounts';
COMMENT ON COLUMN users.location_mode      IS 'gps = device GPS, manual = Pro Singgah override';
COMMENT ON COLUMN users.selected_location  IS 'Pro-only: manually chosen region point for Singgah feature';
