# Expert Base Agent Guidelines

Expert Base is a private, extensible knowledge base system for professional knowledge workers.

## Repository Baseline

- `frontend/`: [package.json](frontend/package.json).
- `backend/`: [pyproject.toml](backend/pyproject.toml).
- `infra/`: [compose.development.yml](infra/compose.development.yml).
- `docs/`: User stories and feature specifications.

## Command Policy

Use Taskfile commands as the primary entrypoint.

At the repository root, prefer:

- `task start`  // Starts all services.
- `task start:app` // Starts only the application services (frontend and backend).
- `task stop` // Stops all services.
- `task stop:app` // Stops only the application services (frontend and backend).
- `task status` // Shows the status of all services.
- `task logs` // Shows logs for all services.
- `task clean:cache` // Cleans up cache and temporary files.

Inside subdirectories, read the local `AGENTS.md` first and use that directory's Taskfile.
