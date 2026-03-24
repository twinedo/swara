-- Migration 003: Channels table
-- Each channel is a live-audio room anchored to a geographic point with a broadcast radius.
-- The spatial index on `location` is MANDATORY — all /nearby queries use ST_DWithin() on it.

CREATE TABLE channels (
  id               UUID           PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_id         UUID           NOT NULL REFERENCES users(id) ON DELETE CASCADE,

  -- FM-style frequency display (e.g. 98.7). Unique per owner, not globally.
  frequency        NUMERIC(5,1)   NOT NULL CHECK (frequency > 0),

  name             TEXT           NOT NULL CHECK (char_length(name) BETWEEN 1 AND 100),

  -- Broadcast origin point (WGS84). Indexed via GIST below.
  location         GEOGRAPHY(POINT, 4326) NOT NULL,

  -- Broadcast reach radius in metres. Default 15 km.
  radius_m         INTEGER        NOT NULL DEFAULT 15000 CHECK (radius_m > 0),

  -- 'live'    → broadcaster is active, LiveKit room is open
  -- 'offline' → no active broadcast
  status           TEXT           NOT NULL DEFAULT 'offline'
                   CHECK (status IN ('live', 'offline')),

  -- LiveKit room identifier — set on broadcast/start, cleared on broadcast/stop
  livekit_room_name TEXT          DEFAULT NULL,

  created_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
  updated_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW(),

  -- A single user cannot register the same frequency twice
  UNIQUE (frequency, owner_id)
);

-- =====================================================================
-- SPATIAL INDEX — critical for ST_DWithin performance on /nearby queries
-- Without this, PostGIS falls back to a sequential scan over all channels.
-- =====================================================================
CREATE INDEX channels_location_gist_idx ON channels USING GIST (location);

-- Supporting indexes
CREATE INDEX channels_owner_id_idx  ON channels (owner_id);
CREATE INDEX channels_status_idx    ON channels (status);   -- dead-air scanner uses this

-- Auto-update updated_at
CREATE TRIGGER channels_updated_at
  BEFORE UPDATE ON channels
  FOR EACH ROW EXECUTE FUNCTION update_updated_at();  -- function defined in migration 002

COMMENT ON TABLE  channels                 IS 'Swara broadcast channels — each is a LiveKit room scoped to a geographic area';
COMMENT ON COLUMN channels.location        IS 'WGS84 point — origin of the broadcast. GIST indexed.';
COMMENT ON COLUMN channels.radius_m        IS 'Coverage radius in metres. Used by /nearby ST_DWithin filter.';
COMMENT ON COLUMN channels.status         IS 'live | offline — source of truth in Postgres; Redis heartbeat keeps it honest';
COMMENT ON COLUMN channels.livekit_room_name IS 'Matches the LiveKit room name used for token generation';
