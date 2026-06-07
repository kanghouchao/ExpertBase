# Backend Agent Guidelines

## Scope

This directory contains the Expert Base backend service.

The backend is responsible for API boundaries, authentication, knowledge workspace orchestration, source ingestion, Wiki compilation workflows, background jobs, plugin integration, and channel gateway integration.

## Technical Baseline

- Language: Python 3.13.
- Dependency manager: uv.
- API framework: FastAPI.
- Data validation and settings: Pydantic and Pydantic Settings.
- Database access: SQLAlchemy.
- Database migrations: Alembic.
- Background jobs: Celery.
- Cache and broker: Redis.
- HTTP client: HTTPX.
- Lint and format: Ruff.
- Tests: Pytest.

## Command Policy

- Prefer Taskfile commands for backend work.
- Use `task install`, `task lint`, `task format`, `task test`, and `task lock` instead of calling `uv` directly.
- Run direct `uv` commands only when the Taskfile does not expose the needed operation. If doing so, state why.

## Initialization Boundary

The backend may have dependencies, configuration, and documentation before application code exists.

Do not create FastAPI app code, routes, database models, migrations, workers, or business modules unless the user explicitly asks for implementation.

## Planned Directory Rules

When application code starts, use this structure:

```txt
src/
  expertbase/
    api/              # FastAPI app, routers, dependencies
    core/             # settings, logging, security, shared infrastructure
    modules/          # domain modules
    workers/          # Celery app and tasks
tests/                # backend tests
alembic/              # database migrations
```

Create directories only when the first real file needs them. Do not add empty architecture folders.

## Backend Practices

- Keep framework wiring separate from domain logic.
- Keep API schemas, domain models, and persistence models distinct when behavior diverges.
- Do not call external AI providers, plugins, or IM APIs directly from route handlers.
- Put integration boundaries behind small service/adaptor modules.
- Use Pydantic Settings for environment configuration.
- Use Alembic for all database schema changes once persistence starts.
- Keep background work idempotent where possible.

## Quality Bar

- Before finishing backend changes, run `task lint`.
- Run `task test` when tests exist or when changing behavior.
- Run `task lock` after dependency changes if the lock file is not already updated.
- If a command fails due to missing implementation scaffolding, report that clearly instead of masking it.
