# Expert Base Infrastructure

This directory contains Docker Compose infrastructure for Expert Base.

Development and production are intentionally split into separate Compose files because their runtime assumptions are different. Development favors local ergonomics. Production favors deployment boundaries.

This is still an early infrastructure baseline. The production Compose file is not a complete production operations setup yet: HTTPS, certificates, backups, monitoring, logging, secret management, and upgrade procedures are still future work.

## Environments

The default environment is `development`.

```bash
task config
task up
```

Use `ENV=production` for the production baseline:

```bash
ENV=production task config
ENV=production task up
```

The Taskfile maps environments like this:

```txt
ENV=development -> compose.development.yml + env/development.env
ENV=production  -> compose.production.yml  + env/production.env
```

If `env/<ENV>.env` does not exist, `task init-env` creates it from the matching example file.

## Files

```txt
infra/
  compose.development.yml       # local development services
  compose.production.yml        # production deployment baseline
  env/
    development.env.example     # committed development template
    development.env             # local development values, ignored by git
    production.env.example      # committed production template
    production.env              # production values, ignored by git
  Taskfile.yml                  # infrastructure commands
  AGENTS.md                     # agent instructions for this directory
```

Do not commit `env/*.env`.

## Development Services

```txt
Traefik        # local HTTP gateway and dashboard
PostgreSQL 17  # primary database, direct port exposed
Redis 8.2.0    # cache and Celery broker, direct port exposed
MinIO          # S3-compatible object storage, direct ports exposed
Backend        # optional app profile, bind-mounted source
Frontend       # optional app profile, bind-mounted source
```

Development uses:

- `compose.development.yml`
- `env/development.env`
- `*.localhost` hostnames
- direct ports for debugging
- bind-mounted frontend and backend source
- development commands such as `bun run dev`
- Traefik dashboard with insecure local access

Default development routes:

```txt
http://expertbase.localhost:8080
http://api.expertbase.localhost:8080
http://minio.expertbase.localhost:8080
http://traefik.expertbase.localhost:8080
```

Default development direct ports:

```txt
Traefik HTTP:      8080
Traefik dashboard: 8081
PostgreSQL:        5432
Redis:             6379
MinIO API:         9000
MinIO UI:          9001
Backend direct:    8000
Frontend direct:   3000
```

## Production Baseline

```txt
Traefik        # public HTTP gateway
PostgreSQL 17  # internal database
Redis 8.2.0    # internal cache and broker
MinIO          # internal object storage
Backend        # image-based service
Frontend       # image-based service
```

Production uses:

- `compose.production.yml`
- `env/production.env`
- real hostnames
- image-based frontend and backend services
- no bind-mounted source code
- no direct PostgreSQL, Redis, or MinIO ports
- no insecure Traefik dashboard

Production currently exposes HTTP only. HTTPS and certificate automation should be added once the deployment target is clear.

## Commands

Run commands from `infra/`.

```bash
task init-env
task config
task config-app
task up
task up-app
task ps
task logs
task down-app
task down
```

With explicit environment:

```bash
ENV=development task config
ENV=production task config
```

`task config` validates and renders the Compose configuration.

`task config-app` validates and renders the Compose configuration including frontend and backend services. It is mainly useful for development because production includes app services by default.

`task up` starts services for the selected environment.

`task up-app` starts services with the `app` profile. Use this for development when you want Compose to include the frontend and backend services.

`task ps` shows service status.

`task logs` follows logs for all services. Use `task logs -- postgres` to follow one service.

`task down-app` stops services including the `app` profile.

`task down` stops services without deleting named volumes.

AI agents and contributors should prefer these `task` commands instead of calling `docker compose` directly.

## Data

Compose uses named Docker volumes:

```txt
postgres_data
redis_data
minio_data
```

Stopping services with `task down` does not delete these volumes.

Do not add destructive cleanup tasks unless explicitly requested.
