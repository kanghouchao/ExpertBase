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

- `src/lib.rs`: Tauri builder setup and command registration. Register new commands in `tauri::generate_handler!`.
- `src/<feature>.rs`: one module per feature domain (e.g. `kb.rs` for the knowledge base).
- `capabilities/`: Tauri capability definitions. Grant the minimum permissions a feature needs.
- `tauri.conf.json`: app configuration.
- `gen/`: generated files. Do not edit by hand.

## IPC Command Practices

- Define commands with `#[tauri::command]` returning `Result<T, String>`, mapping internal errors with `map_err(|e| e.to_string())`.
- Structs crossing the IPC boundary derive `Serialize` with `#[serde(rename_all = "camelCase")]` to match the TypeScript client in `frontend/src/lib/tauri`.
- Keep commands as thin wrappers: put the real logic in plain functions that take explicit inputs (paths, values) instead of `AppHandle`, so they are unit-testable.

## Quality Bar

- New logic gets unit tests in a `#[cfg(test)] mod tests` block in the same file; use `tempfile` for filesystem tests.
- Before finishing Rust changes, run `bun run test` from the repository root and report the actual result.
