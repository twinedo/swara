-- Migration 004: Schedules table
-- Programme schedule entries for each channel.
-- day_of_week NULL means the show runs daily (e.g. a permanent slot).

CREATE TABLE schedules (
  id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
  channel_id        UUID        NOT NULL REFERENCES channels(id) ON DELETE CASCADE,

  show_name         TEXT        NOT NULL CHECK (char_length(show_name) BETWEEN 1 AND 200),
  host_name         TEXT        DEFAULT NULL CHECK (host_name IS NULL OR char_length(host_name) <= 200),

  -- Time-of-day the show starts, with timezone offset stored
  start_time        TIMETZ      NOT NULL,

  -- 0=Sunday, 1=Monday … 6=Saturday. NULL = every day.
  day_of_week       INTEGER     DEFAULT NULL
                    CHECK (day_of_week IS NULL OR day_of_week BETWEEN 0 AND 6),

  duration_minutes  INTEGER     DEFAULT NULL CHECK (duration_minutes IS NULL OR duration_minutes > 0),

  created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lookups are always by channel_id + optionally day
CREATE INDEX schedules_channel_id_idx     ON schedules (channel_id);
CREATE INDEX schedules_channel_day_idx    ON schedules (channel_id, day_of_week);

COMMENT ON TABLE  schedules                IS 'Programme schedule entries for Swara channels';
COMMENT ON COLUMN schedules.day_of_week   IS '0=Sun … 6=Sat. NULL = daily recurring show.';
COMMENT ON COLUMN schedules.start_time    IS 'TIMETZ preserves the offset from when the show was entered';
