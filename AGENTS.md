# Expert Base Agent Guidelines

Expert Base is a private, extensible, local-first knowledge base system for professional knowledge workers, shipped as a Tauri 2 desktop application.

## Repository Baseline

- `frontend/`: UI (Next.js static export). See [package.json](frontend/package.json).
- `src-tauri/`: Rust core (desktop shell, local data layer). See [Cargo.toml](src-tauri/Cargo.toml).
- `docs/`: User stories and feature specifications.

## Command Policy

Use Taskfile commands as the primary entrypoint.

At the repository root, prefer:

- `task install`  // Installs all dependencies.
- `task dev`      // Runs the desktop app in development mode.
- `task build`    // Builds the desktop app bundles.
- `task lint`     // Lints the frontend.
- `task test`     // Runs Rust tests.
- `task clean:cache` // Cleans up build caches.

Inside subdirectories, read the local `AGENTS.md` first and use that directory's Taskfile.
