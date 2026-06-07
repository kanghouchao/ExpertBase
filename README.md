# Expert Base

Expert Base is a private, extensible knowledge base system for professional knowledge workers.

It is not designed to merely store documents. Its goal is to help individuals and organizations turn scattered documents, web pages, spreadsheets, notes, experience records, and business data into a long-lived private knowledge asset that can be read, reviewed, searched, linked, reused, published, and used by AI.

## Project Goals

Expert Base helps users continuously grow a private knowledge base from raw source material into structured, maintainable knowledge.

The core workflow is knowledge compilation rather than traditional RAG-first retrieval. Raw sources remain immutable as the factual base. AI helps compile those sources into a Markdown-compatible Wiki workspace with pages, citations, indexes, entities, topics, and bidirectional links. Human review keeps the compiled knowledge accurate, trustworthy, and valuable for the target domain.

The system is designed for both human users and AI tools:

- Human users can read, edit, review, search, filter, link, and manage knowledge.
- AI tools can use reviewed and authorized knowledge for question answering, analysis, writing, automation, and assistant workflows.

Knowledge bases are private by default. Users control which knowledge remains private, which metadata can be public, and which content can be exposed as paid text libraries, consulting services, APIs, public pages, web assistants, or IM bots.

Long term, Expert Base aims to support a WordPress-like ecosystem: an easy-to-use SaaS product, a self-hostable open-source version, and a plugin model for extending ingestion, processing, publishing, assistant, storage, and integration capabilities.

## Architecture Direction

The current architecture baseline is documented in [docs/ARCHITECTURE.md](/Users/kanghouchao/CodeProjects/ExpertBase/docs/ARCHITECTURE.md).

The project follows these architectural decisions:

- Modular monolith first, not microservices.
- Wiki-first knowledge architecture, not RAG-first.
- Raw sources, compiled Wiki, and Wiki schema as separate layers.
- Bidirectional links as a first-class knowledge structure.
- Human-in-the-loop AI processing and review.
- API-first backend.
- Externalized plugin protocol instead of running arbitrary plugin code in the core system.
- Channel gateway for IM and bot integrations.
- Local development and early self-hosting with Docker Compose.

## Technology Baseline

Backend:

- Python 3.13
- FastAPI
- Pydantic Settings
- SQLAlchemy
- Alembic
- Celery
- Redis
- HTTPX
- uv
- Ruff
- Pytest

Frontend:

- Next.js App Router
- React
- TypeScript
- Bun
- Tailwind CSS v4
- shadcn/ui

Infrastructure:

- PostgreSQL 17
- Redis 8.2.0
- MinIO
- Traefik
- Docker Compose
- Taskfile

pgvector is considered optional for future large-scale semantic search. It is not a first-stage core dependency because the primary knowledge layer is the compiled Wiki workspace.

## Repository Structure

```txt
.
  backend/       # Python backend project baseline
  frontend/      # Next.js frontend project baseline
  infra/         # Local Docker Compose infrastructure
  docs/          # Architecture and project documentation
  README.md      # Project overview
  AGENTS.md      # Root agent and contributor rules
  CLAUDE.md      # Claude entrypoint for local rules
  Taskfile.yml   # Root command shortcuts
```

Directory-specific documentation:

- [backend/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/backend/README.md)
- [frontend/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/frontend/README.md)
- [infra/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/infra/README.md)

## Current Status

The repository is in framework setup stage.

Already initialized:

- Frontend project scaffold with Next.js, Bun, Tailwind CSS, and shadcn/ui.
- Backend dependency baseline with uv, FastAPI-related dependencies, linting, and testing tools.
- Infrastructure baseline with PostgreSQL, Redis, MinIO, and optional app services.
- Architecture documentation for the Wiki-first system direction.

Not implemented yet:

- Backend API application code.
- Database schema and migrations.
- Worker tasks.
- Product UI screens beyond the initial frontend scaffold.
- Knowledge compilation pipeline.
- Plugin runtime.
- IM bot integrations.

## Commands

Use Taskfile commands as the main project entrypoint. The root Taskfile focuses on starting, stopping, inspecting, and cleaning the whole project environment.

The default environment is `development`.

From the repository root:

```bash
task start
task start:app
task stop
task stop:app
task status
task logs
task clean:cache
```

Run against another environment by setting `ENV` before the command:

```bash
ENV=production task config
ENV=production task start
```

For directory-specific workflows, run commands from the corresponding directory:

```bash
cd frontend
task dev
task lint
task build

cd backend
task lint
task test

cd infra
task config
ENV=production task config
task up
```

Do not start long-running services unless that is the intended task.

## Development Principles

- Keep changes small and directly tied to the goal.
- Prefer simple, understandable architecture over speculative flexibility.
- Keep business logic out of framework glue code.
- Use directory-specific `AGENTS.md` files before changing that area.
- Prefer `task` commands over direct `bun`, `uv`, or `docker compose` commands.
- Do not create empty architecture folders before real files need them.
- Keep generated artifacts and environment secrets out of git.

## Documentation

Primary documents:

- [docs/ARCHITECTURE.md](/Users/kanghouchao/CodeProjects/ExpertBase/docs/ARCHITECTURE.md): system architecture and technical decisions.
- [frontend/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/frontend/README.md): frontend stack, commands, and conventions.
- [backend/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/backend/README.md): backend baseline, dependencies, and planned structure.
- [infra/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/infra/README.md): local infrastructure, services, and commands.
