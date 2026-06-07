# Expert Base Agent Guidelines

## Project Context

Expert Base is a private, extensible knowledge base system for professional knowledge workers.

The current product direction is Wiki-first knowledge compilation, not RAG-first retrieval. Raw sources remain the factual base. AI compiles and maintains a structured, reviewable, bidirectionally linked Wiki workspace that can later power search, publishing, assistants, APIs, and IM bots.

## Repository Baseline

- `frontend/`: Next.js App Router, React, TypeScript, Bun, Tailwind CSS, shadcn/ui.
- `backend/`: Python 3.13, uv, FastAPI dependency baseline, SQLAlchemy, Alembic, Celery, Redis.
- `infra/`: Docker Compose development and production infrastructure with PostgreSQL, Redis, MinIO, and Traefik.
- `docs/`: architecture and project documentation.

The repository is still in framework setup stage. Do not add product implementation code unless explicitly asked.

## Architecture Rules

- Keep the first implementation as a modular monolith.
- Keep the knowledge model Wiki-first: Raw Sources, Compiled Wiki, Wiki Schema.
- Treat bidirectional links as core knowledge structure, not a UI-only feature.
- Keep pgvector optional. Do not make vector search the default architecture path.
- Keep AI output reviewable with citations, versions, and human confirmation.
- Keep plugins externalized through manifests and protocols. Do not run arbitrary user plugin code inside the core service.
- Keep IM and bot integrations behind a channel gateway.

## Command Policy

Use Taskfile commands as the primary entrypoint.

At the repository root, prefer:

- `task start`
- `task start:app`
- `task stop`
- `task stop:app`
- `task status`
- `task logs`
- `task clean:cache`

Inside subdirectories, read the local `AGENTS.md` first and use that directory's Taskfile.

## Current Boundaries

- Do not create backend API routes, database models, migrations, workers, or product modules unless requested.
- Do not create frontend product screens unless requested.
- Do not add empty architecture folders.
- Do not add production cloud infrastructure until the deployment target is clear.
- Do not start long-running services unless the task requires it.
