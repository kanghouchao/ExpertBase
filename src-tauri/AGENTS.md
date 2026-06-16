# Rust Core Agent Guidelines

## Scope

This directory contains the Expert Base Rust core: the Tauri 2 desktop shell and the local data layer. It loads the static export produced by `frontend/` and exposes functionality to the UI through Tauri IPC commands.

When working under `src-tauri/`, follow this file in addition to the repository-level instructions.

## Technical Baseline

- Framework: Tauri 2.
- Language: Rust, edition 2021, minimum version 1.77.2.
- Library crate: `expert_base_lib` (`src/lib.rs`); `src/main.rs` is a thin binary entry point.
- Serialization: serde / serde_json.
- Logging: `log` macros, emitted through `tauri-plugin-log` (registered in debug builds only).

## Command Policy

- From the repository root, prefer `bun run dev`, `bun run build`, and `bun run test` (which runs `cargo test --manifest-path src-tauri/Cargo.toml`).
- Use direct `cargo` commands inside `src-tauri/` only when the root scripts do not expose the needed operation.
- There is no committed rustfmt configuration and the existing code uses 2-space indentation. Do not run `cargo fmt` over whole files; match the surrounding style instead.

## Directory Rules

- `src/lib.rs`: Tauri builder setup and command registration. Register new commands in `tauri::generate_handler!` (commands live in each feature's `interface` module, e.g. `kb::interface::kb_list`).
- `src/<feature>/`: one module per feature domain (e.g. `kb/`), split into `domain`, `application`, `infrastructure`, and `interface` layers — only the layers a feature actually needs. A feature re-exports its public surface from `mod.rs`; other features depend on that surface, not on internal layers.
- `capabilities/`: Tauri capability definitions. Grant the minimum permissions a feature needs.
- `tauri.conf.json`: app configuration.
- `gen/`: generated files. Do not edit by hand.

## Domain-Driven Design

Use Domain-Driven Design (DDD) for Rust backend code in `src-tauri/`.

- Domain code owns business rules, invariants, entities, value objects, domain errors, and domain events.
- Application code represents use cases. It orchestrates domain behavior and depends on domain abstractions, not concrete infrastructure.
- Infrastructure code implements persistence, filesystem access, indexing, plugin loading, OS integration, and other concrete adapters.
- Interface code adapts the outside world to the application layer. Tauri commands are interface adapters only.
- Domain code must not depend on Tauri, storage, filesystem, frontend DTOs, or other infrastructure details.

All current features (`kb`, `ai`, `capture`, `workshop`) are split into the DDD layers they actually use. New feature modules may start as `src/<feature>.rs`, but split into feature-local `domain`, `application`, `infrastructure`, and `interface` modules as they accumulate real business rules or multiple use cases. Create only the layers a feature actually needs — do not add empty layers (e.g. `ai` has no `application` layer; `workshop` has no `infrastructure` layer). Use top-level shared modules only when a cross-feature abstraction is justified by real reuse.

## IPC Command Practices

- Define commands with `#[tauri::command]` returning `Result<T, String>`, mapping internal errors with `map_err(|e| e.to_string())`.
- Structs crossing the IPC boundary derive `Serialize` with `#[serde(rename_all = "camelCase")]` to match the TypeScript client in `frontend/src/shared/api/tauri`.
- Keep commands as thin wrappers: parse IPC inputs, call application services or plain functions, and format IPC outputs. Do not put business rules in command handlers.

## Quality Bar

- Follow TDD for behavior changes: write or update the failing unit test first, then implement the smallest change that makes it pass.
- New domain and application logic gets unit tests in a `#[cfg(test)] mod tests` block in the same file.
- Use `tempfile` for filesystem and infrastructure tests.
- Before finishing Rust changes, run `bun run test` from the repository root and report the actual result.
