# Local API Setup

This project currently runs as a Rust API in `services/api`.

Local dependencies:

- PostgreSQL with PostGIS
- Redis
- LiveKit Server

Quick start on macOS:

```bash
./scripts/setup-local-macos.sh
```

Then run LiveKit:

```bash
source .env.local
./scripts/start-livekit-dev.sh
```

Then run the API:

```bash
source .env.local
./scripts/run-api-local.sh
```

Useful test calls:

```bash
curl -X POST http://127.0.0.1:3100/api/auth/register \
  -H 'content-type: application/json' \
  -d '{"username":"demo","password":"password123"}'
```

```bash
curl "http://127.0.0.1:3100/api/channels/nearby?lat=-6.2&lng=106.8&radius=15000"
```
