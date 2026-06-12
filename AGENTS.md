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
