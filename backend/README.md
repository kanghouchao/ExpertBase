# Expert Base Backend

This directory contains the Expert Base backend project.

The backend is currently initialized as a Python application project. It has dependency management, tooling, environment conventions, and task commands in place, but no FastAPI application code has been created yet.

## Technology Stack

- Python 3.13
- uv
- FastAPI
- Pydantic Settings
- SQLAlchemy
- Alembic
- Celery
- Redis
- HTTPX
- Ruff
- Pytest

## Current Baseline

The backend has been initialized with:

- runtime dependencies in `pyproject.toml`
- development dependencies in `pyproject.toml`
- locked dependencies in `uv.lock`
- local environment template in `.env.example`
- backend task commands in `Taskfile.yml`
- backend agent instructions in `AGENTS.md`

No API app, routes, models, migrations, workers, or business modules have been implemented yet.

## Commands

Run commands from `backend/`.

```bash
task install
task lint
task format
task test
task lock
```

`task install` installs dependencies with uv.

`task lint` runs Ruff lint checks.

`task format` formats Python files with Ruff.

`task test` runs Pytest.

`task lock` refreshes `uv.lock` without upgrading dependencies.

AI agents and contributors should prefer the `task` commands instead of calling `uv` directly.

## Planned Directory Structure

Application code has not been created yet. When implementation starts, use this structure:

```txt
backend/
  src/
    expertbase/
      api/              # FastAPI app, routers, dependencies
      core/             # settings, logging, security, shared infrastructure
      modules/          # domain modules
      workers/          # Celery app and tasks
  tests/                # backend tests
  alembic/              # database migrations
  pyproject.toml
  uv.lock
```

Do not create empty architecture folders. Add a directory when the first real file needs it.

## Environment

Copy `.env.example` to `.env` for local development when backend code starts using environment configuration.

```bash
cp .env.example .env
```

The current environment template includes:

- `APP_ENV`
- `APP_NAME`
- `DATABASE_URL`
- `REDIS_URL`
- `OBJECT_STORAGE_ENDPOINT`
- `OBJECT_STORAGE_BUCKET`

## Development Notes

This project is configured as a uv application project:

```toml
[tool.uv]
package = false
```

That means uv does not require an importable Python package before dependencies can be managed. Application package structure will be added when backend implementation begins.
