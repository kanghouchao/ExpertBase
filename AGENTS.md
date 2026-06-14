# Expert Base Agent Guidelines

Expert Base is a private, extensible, local-first knowledge base system for professional knowledge workers, shipped as a Tauri 2 desktop application.

## Language Policy

- Write code comments and documentation in Japanese.
- Converse with the user in Chinese.
- Write `AGENTS.md` and `CLAUDE.md` files in English.

## Repository Baseline

- `frontend/`: UI (Next.js static export). See [package.json](frontend/package.json).
- `src-tauri/`: Rust core (desktop shell, local data layer). See [Cargo.toml](src-tauri/Cargo.toml).
- `docs/`: User stories and feature specifications.

## Architecture and Development Consensus

- The top-level architecture principles are frontend/backend separation, asynchronous execution, and pluginization.
- Rust backend code in `src-tauri/` follows Domain-Driven Design (DDD).
  - Business rules live in domain modules.
  - Application services represent use cases and orchestrate domain behavior.
  - Infrastructure modules implement persistence, filesystem, indexing, plugin, OS, and Tauri boundary details.
  - Interface adapters such as Tauri commands, HTTP handlers, CLI handlers, or event handlers must stay thin.
- Frontend code follows Feature-Sliced Design (FSD).
  - Route files compose screens and read routing context.
  - User scenarios belong in features.
  - Domain-facing client models belong in entities.
  - Reusable primitives and framework-neutral helpers belong in shared.
  - Dependencies flow downward only; cross-slice imports must use public APIs.
- Development follows Test-Driven Development (TDD).
  - Write or update the failing test before implementation.
  - Implement the smallest code that makes the test pass.
  - Refactor only after tests pass.
  - If a meaningful test cannot be written first, state why before coding.
- Documentation evolves with implementation. Update docs when behavior, architecture, boundaries, runtime configuration, or quality gates change.

## Command Policy

Use the root `package.json` scripts as the primary entrypoint.

At the repository root, prefer:

- `bun run setup`  // Installs all dependencies.
- `bun run dev`    // Runs the desktop app in development mode.
- `bun run build`  // Builds the desktop app bundles.
- `bun run lint`   // Lints the frontend.
- `bun run test`   // Runs Rust tests.
- `bun run clean`  // Cleans up build caches.

Inside subdirectories, read the local `AGENTS.md` first.
