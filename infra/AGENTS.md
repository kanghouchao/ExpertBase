# Infrastructure Agent Guidelines

## Scope

This directory contains development and production infrastructure definitions for Expert Base.

The infrastructure baseline supports backend and frontend development. It should stay small, explicit, and easy to run locally.

## Technical Baseline

- Container runtime: Docker-compatible runtime.
- Local orchestration: Docker Compose.
- Database: PostgreSQL.
- Cache and broker: Redis.
- Object storage: MinIO.
- Gateway: Traefik.
- Optional app profile: frontend and backend development services.
- Environment split: `development` and `production`.

## Command Policy

- Prefer Taskfile commands for infrastructure work.
- Use `ENV=development task ...` or `ENV=production task ...`.
- The default environment is `development`, so `task up` is equivalent to `ENV=development task up`.
- Use `task init-env`, `task config`, `task config-app`, `task up`, `task up-app`, `task down`, `task down-app`, `task ps`, and `task logs` instead of calling `docker compose` directly.
- Run direct `docker compose` commands only when the Taskfile does not expose the needed operation. If doing so, state why.

## File Rules

- `compose.development.yml`: development infrastructure services.
- `compose.production.yml`: production deployment baseline.
- `env/development.env.example`: development environment template.
- `env/production.env.example`: production environment template.
- `env/*.env`: active local environment files. Do not commit these files.
- `data/`: local persistent service data. Do not commit this directory.
- `logs/`: local logs. Do not commit this directory.

## Infrastructure Practices

- Keep local infrastructure boring and explicit.
- Pin major image versions where practical. Do not use complex production-only services for local development.
- Prefer named volumes over bind-mounted database directories.
- Use Traefik as the gateway layer for HTTP routing.
- Keep direct service ports available during early development unless there is a reason to remove them.
- Keep development and production Compose files separate when their services or runtime assumptions differ.
- Production Compose is a deployment baseline only. Do not add claims of complete production readiness without HTTPS, backups, monitoring, logging, and secret management.
- Do not add Kubernetes, Terraform, Pulumi, or cloud-specific infrastructure until the project has a real deployment target.
- Do not add destructive cleanup tasks unless the user explicitly asks for them.
- Do not store secrets in committed files.

## Quality Bar

- After editing Compose files, run `task config` to validate configuration.
- Do not run `task up` or `task up-app` unless the user asks to start services.
- Use `task up-app` only when the frontend and backend Compose services are intentionally needed.
- If validation fails because Docker is unavailable, report that directly.
