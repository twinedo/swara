-- verify_schema.sql
-- Run after migrations to confirm the database is correctly set up.
-- All queries should return the expected values; any deviation indicates a problem.
-- Usage: psql $DATABASE_URL -f services/api/scripts/verify_schema.sql

\echo '=== Swara Schema Verification ==='
\echo ''

-- 1. PostGIS
\echo '--- PostGIS version ---'
SELECT PostGIS_Version();

-- 2. Tables present
\echo ''
\echo '--- Tables ---'
SELECT table_name
FROM information_schema.tables
WHERE table_schema = 'public'
  AND table_name IN ('users', 'channels', 'schedules', '_migrations')
ORDER BY table_name;

-- 3. Spatial index on channels.location (MANDATORY)
\echo ''
\echo '--- Spatial index on channels.location ---'
SELECT
    indexname,
    indexdef
FROM pg_indexes
WHERE tablename = 'channels'
  AND indexname = 'channels_location_gist_idx';

-- 4. PostGIS column types
\echo ''
\echo '--- Geography column types ---'
SELECT
    f_table_name AS table_name,
    f_geography_column AS column_name,
    type,
    srid
FROM geography_columns
WHERE f_table_name IN ('users', 'channels')
ORDER BY f_table_name, f_geography_column;

-- 5. Applied migrations
\echo ''
\echo '--- Applied migrations ---'
SELECT filename, applied_at
FROM _migrations
ORDER BY applied_at;

-- 6. Quick ST_DWithin sanity check — Jakarta to Bandung (~170 km)
\echo ''
\echo '--- ST_DWithin sanity check (Jakarta → Bandung, expect false at 50km) ---'
SELECT ST_DWithin(
    ST_MakePoint(106.8272, -6.2088)::geography,
    ST_MakePoint(107.6191, -6.9175)::geography,
    50000
) AS within_50km;

\echo ''
\echo '--- ST_DWithin sanity check (Jakarta, expect true at 200km) ---'
SELECT ST_DWithin(
    ST_MakePoint(106.8272, -6.2088)::geography,
    ST_MakePoint(107.6191, -6.9175)::geography,
    200000
) AS within_200km;

-- 7. Row counts (seed verification)
\echo ''
\echo '--- Row counts (seed data, dev only) ---'
SELECT 'users' AS tbl, COUNT(*) AS rows FROM users
UNION ALL
SELECT 'channels' AS tbl, COUNT(*) AS rows FROM channels
UNION ALL
SELECT 'schedules' AS tbl, COUNT(*) AS rows FROM schedules;

\echo ''
\echo '=== Verification complete ==='
