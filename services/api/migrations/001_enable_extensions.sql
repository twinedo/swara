-- Migration 001: Enable required PostgreSQL extensions
-- Run this first — PostGIS and uuid-ossp are required by all subsequent migrations

CREATE EXTENSION IF NOT EXISTS postgis;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";  -- Fallback for gen_random_uuid() on older PG versions

-- Verify PostGIS is working (will error out early if extension failed)
DO $$
BEGIN
  PERFORM PostGIS_Version();
END;
$$;
