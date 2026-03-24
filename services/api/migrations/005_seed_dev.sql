-- Migration 005: Development seed data
-- Only runs in development — guarded by a check on the DB name.
-- Passwords are bcrypt hashes of 'password123' (cost 12).

DO $$
BEGIN
  -- Safety guard: never seed production
  IF current_database() NOT IN ('swara_dev', 'swara_test') THEN
    RAISE EXCEPTION 'Seed data must not be applied to database: %', current_database();
  END IF;
END;
$$;

-- ── Users ──────────────────────────────────────────────────────────────────
INSERT INTO users (id, username, password_hash, is_pro, location_mode)
VALUES
  (
    '00000000-0000-0000-0000-000000000001',
    'raka',
    -- bcrypt hash of 'password123', cost 12
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4J/gWq8W6i',
    true,
    'gps'
  ),
  (
    '00000000-0000-0000-0000-000000000002',
    'dina',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4J/gWq8W6i',
    false,
    'gps'
  ),
  (
    '00000000-0000-0000-0000-000000000003',
    'bagas',
    '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewdBPj4J/gWq8W6i',
    true,
    'manual'
  )
ON CONFLICT (username) DO NOTHING;

-- Set Bagas's Singgah location to Yogyakarta city centre
UPDATE users
SET selected_location = ST_MakePoint(110.3695, -7.7956)::geography
WHERE id = '00000000-0000-0000-0000-000000000003';

-- ── Channels ───────────────────────────────────────────────────────────────
-- Kopi Pagi FM — Jakarta Selatan (Raka), currently live
INSERT INTO channels (id, owner_id, frequency, name, location, radius_m, status, livekit_room_name)
VALUES (
  '10000000-0000-0000-0000-000000000001',
  '00000000-0000-0000-0000-000000000001',
  98.7,
  'Kopi Pagi FM',
  ST_MakePoint(106.8272, -6.2088)::geography,  -- Jakarta Selatan
  15000,
  'live',
  'channel-10000000-0000-0000-0000-000000000001'
) ON CONFLICT DO NOTHING;

-- Indie Sore Radio — Bandung (Bagas), offline
INSERT INTO channels (id, owner_id, frequency, name, location, radius_m, status)
VALUES (
  '10000000-0000-0000-0000-000000000002',
  '00000000-0000-0000-0000-000000000003',
  103.4,
  'Indie Sore Radio',
  ST_MakePoint(107.6191, -6.9175)::geography,  -- Bandung
  20000,
  'offline'
) ON CONFLICT DO NOTHING;

-- Jogja Malam FM — Yogyakarta (Bagas), offline
INSERT INTO channels (id, owner_id, frequency, name, location, radius_m, status)
VALUES (
  '10000000-0000-0000-0000-000000000003',
  '00000000-0000-0000-0000-000000000003',
  91.1,
  'Jogja Malam FM',
  ST_MakePoint(110.3695, -7.7956)::geography,  -- Yogyakarta
  12000,
  'offline'
) ON CONFLICT DO NOTHING;

-- ── Schedules ──────────────────────────────────────────────────────────────
INSERT INTO schedules (channel_id, show_name, host_name, start_time, day_of_week, duration_minutes)
VALUES
  -- Kopi Pagi FM — daily morning show
  (
    '10000000-0000-0000-0000-000000000001',
    'Morning Talk',
    'Raka & Dina',
    '06:00:00+07',
    NULL,   -- daily
    120
  ),
  -- Kopi Pagi FM — weekend evening special
  (
    '10000000-0000-0000-0000-000000000001',
    'Weekend Vibes',
    'Raka',
    '20:00:00+07',
    6,      -- Saturday
    180
  ),
  -- Indie Sore Radio — weekday afternoon
  (
    '10000000-0000-0000-0000-000000000002',
    'Sore Santai',
    'Bagas',
    '16:00:00+07',
    NULL,
    90
  ),
  -- Jogja Malam FM — late night
  (
    '10000000-0000-0000-0000-000000000003',
    'Malam Indie',
    'Bagas',
    '22:00:00+07',
    NULL,
    120
  );
