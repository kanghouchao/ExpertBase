# Desktop Architecture Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the web frontend/backend architecture with a Tauri 2 local-first desktop application, reusing the existing Next.js UI via static export.

**Architecture:** The existing Next.js App Router UI (pure client code, mock data) is switched to static export and loaded by a Tauri 2 shell. A minimal Rust knowledge-base layer initializes the local app data directory and exposes it over IPC. `backend/` (FastAPI hello) and `infra/` (Docker/Traefik) are deleted.

**Tech Stack:** Tauri 2 (Rust), Next.js 16 static export, React 19, Bun, root `package.json` scripts.

**Spec:** `docs/superpowers/specs/2026-06-12-desktop-architecture-design.md`

---

### Task 1: Install Rust toolchain

**Files:** none (system prerequisite)

- [ ] **Step 1: Check whether Rust is already installed**

Run: `command -v cargo && rustc --version`
Expected on this machine: no output (not installed). Skip to Step 3 if installed.

- [ ] **Step 2: Install rustup with the stable toolchain**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
```

- [ ] **Step 3: Verify**

Run: `source "$HOME/.cargo/env" && rustc --version && cargo --version`
Expected: `rustc 1.8x.x` and `cargo 1.8x.x` version lines.
Note: every later `cargo` invocation in this plan assumes `$HOME/.cargo/bin` is on PATH (`source "$HOME/.cargo/env"`).

---

### Task 2: Switch Next.js to static export

**Files:**
- Modify: `frontend/next.config.ts`
- Modify: `frontend/.gitignore` (add `/out`)

- [ ] **Step 1: Replace `frontend/next.config.ts`**

```ts
import type { NextConfig } from "next";

const isProd = process.env.NODE_ENV === "production";
const internalHost = process.env.TAURI_DEV_HOST || "localhost";

const nextConfig: NextConfig = {
  // Tauri loads the UI from static files; SSR is not available.
  output: "export",
  images: {
    unoptimized: true,
  },
  // In dev the Tauri webview loads assets from the Next.js dev server.
  assetPrefix: isProd ? undefined : `http://${internalHost}:3000`,
};

export default nextConfig;
```

- [ ] **Step 2: Ignore the export output**

Add `/out` on its own line to `frontend/.gitignore`.

- [ ] **Step 3: Verify the static export builds**

Run: `cd frontend && bun run build && ls out/index.html`
Expected: build succeeds, `out/index.html` exists.

- [ ] **Step 4: Verify lint still passes**

Run: `cd frontend && bun run lint`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add frontend/next.config.ts frontend/.gitignore
git commit -m "feat(frontend): switch Next.js to static export for Tauri"
```

---

### Task 3: Scaffold the Tauri 2 shell

**Files:**
- Create: `package.json` (repo root)
- Create: `src-tauri/` (via `tauri init`, then overwrite config)
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Add the Tauri CLI at the repo root**

Run from repo root: `bun add -d @tauri-apps/cli@^2`
Expected: creates root `package.json` + `bun.lock` with the CLI as devDependency.

- [ ] **Step 2: Scaffold src-tauri non-interactively**

Run from repo root:

```bash
bunx tauri init --ci \
  --app-name expert-base \
  --window-title "Expert Base" \
  --frontend-dist ../frontend/out \
  --dev-url http://localhost:3000 \
  --before-dev-command "" \
  --before-build-command ""
```

Expected: `src-tauri/` created with `Cargo.toml`, `src/`, `icons/`, `capabilities/`, `tauri.conf.json`, `.gitignore`.

- [ ] **Step 3: Replace `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Expert Base",
  "version": "0.1.0",
  "identifier": "app.expertbase.desktop",
  "build": {
    "beforeDevCommand": {
      "script": "bun run dev",
      "cwd": "../frontend",
      "wait": false
    },
    "beforeBuildCommand": {
      "script": "bun run build",
      "cwd": "../frontend"
    },
    "devUrl": "http://localhost:3000",
    "frontendDist": "../frontend/out"
  },
  "app": {
    "windows": [
      {
        "title": "Expert Base",
        "width": 1280,
        "height": 800,
        "minWidth": 960,
        "minHeight": 640
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: asset: http://asset.localhost; font-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 4: Verify the Rust project compiles**

Run: `cd src-tauri && cargo check`
Expected: `Finished` line, no errors. (First run downloads crates; takes a few minutes.)

- [ ] **Step 5: Commit**

```bash
git add package.json bun.lock src-tauri
git commit -m "feat(desktop): scaffold Tauri 2 shell loading the Next.js UI"
```

---

### Task 4: Rust knowledge-base layer (TDD)

**Files:**
- Create: `src-tauri/src/kb.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml` (add `tempfile` dev-dependency)

- [ ] **Step 1: Add the dev-dependency**

In `src-tauri/Cargo.toml` add:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Write the failing test**

Create `src-tauri/src/kb.rs`:

```rust
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KbStatus {
    pub root: String,
    pub initialized: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_kb_root_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let root = ensure_kb_root(tmp.path()).unwrap();
        assert!(root.is_dir());
        assert!(root.ends_with("knowledge-base"));
    }

    #[test]
    fn ensure_kb_root_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let first = ensure_kb_root(tmp.path()).unwrap();
        let second = ensure_kb_root(tmp.path()).unwrap();
        assert_eq!(first, second);
    }
}
```

And register the module in `src-tauri/src/lib.rs` (add `mod kb;` at the top).

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd src-tauri && cargo test`
Expected: compile error — `ensure_kb_root` not found.

- [ ] **Step 4: Implement**

Insert into `src-tauri/src/kb.rs` between the struct and the tests:

```rust
/// Ensure the knowledge base root directory exists under the app data dir.
pub fn ensure_kb_root(base_dir: &Path) -> std::io::Result<PathBuf> {
    let root = base_dir.join("knowledge-base");
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

#[tauri::command]
pub fn kb_status(app: tauri::AppHandle) -> Result<KbStatus, String> {
    let base = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let root = ensure_kb_root(&base).map_err(|e| e.to_string())?;
    Ok(KbStatus {
        root: root.to_string_lossy().into_owned(),
        initialized: true,
    })
}
```

- [ ] **Step 5: Register the command**

In `src-tauri/src/lib.rs`, the builder must register the handler:

```rust
mod kb;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![kb::kb_status])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

(Keep whatever `tauri init` generated that is still needed — e.g. plugin registrations — and remove the generated `greet` example command if present.)

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd src-tauri && cargo test`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 7: Commit**

```bash
git add src-tauri
git commit -m "feat(desktop): add knowledge base root initialization and kb_status IPC command"
```

---

### Task 5: Frontend IPC client + sidebar status line

**Files:**
- Create: `frontend/src/lib/tauri/client.ts`
- Create: `frontend/src/components/shell/kb-status.tsx`
- Modify: `frontend/src/components/shell/sidebar.tsx`
- Modify: `frontend/package.json` (add `@tauri-apps/api`)

- [ ] **Step 1: Install the API package**

Run: `cd frontend && bun add @tauri-apps/api@^2`

- [ ] **Step 2: Create the typed IPC client**

Create `frontend/src/lib/tauri/client.ts`:

```ts
import { invoke, isTauri } from "@tauri-apps/api/core";

export type KbStatus = {
  root: string;
  initialized: boolean;
};

/** Local knowledge base status, or null when running in a plain browser. */
export async function getKbStatus(): Promise<KbStatus | null> {
  if (!isTauri()) return null;
  return invoke<KbStatus>("kb_status");
}
```

- [ ] **Step 3: Create the status component**

Create `frontend/src/components/shell/kb-status.tsx`:

```tsx
"use client";

import { useEffect, useState } from "react";

import { getKbStatus } from "@/lib/tauri/client";

export function KbStatus() {
  const [root, setRoot] = useState<string | null>(null);

  useEffect(() => {
    getKbStatus().then(
      (status) => setRoot(status?.root ?? null),
      () => setRoot(null),
    );
  }, []);

  if (!root) return null;
  return (
    <div
      className="truncate px-3.25 pt-3 font-mono text-[10px] text-ink-faint"
      title={root}
    >
      {root}
    </div>
  );
}
```

- [ ] **Step 4: Mount it in the sidebar**

In `frontend/src/components/shell/sidebar.tsx`, import it and render it after `<KbSwitcher … />`:

```tsx
import { KbStatus } from "@/components/shell/kb-status";
// …
      <KbSwitcher activeId={activeKb} setActiveId={setActiveKb} />
      <KbStatus />
```

- [ ] **Step 5: Verify lint and build**

Run: `cd frontend && bun run lint && bun run build`
Expected: both pass.

- [ ] **Step 6: Commit**

```bash
git add frontend
git commit -m "feat(frontend): add typed Tauri IPC client and KB status line"
```

---

### Task 6: Tear down backend/infra and update root scripts + docs

**Files:**
- Delete: `backend/`, `infra/`
- Modify: `package.json` (root), `README.md` (root), `AGENTS.md` (root)
- Modify: `frontend/AGENTS.md` (backend-client rule → IPC-client rule)

- [ ] **Step 1: Delete the web-era directories**

```bash
git rm -r backend infra
```

- [ ] **Step 2: Add root `package.json` scripts**

```json
{
  "name": "expertbase",
  "private": true,
  "scripts": {
    "setup": "bun install && bun install --cwd frontend",
    "dev": "tauri dev",
    "build": "tauri build",
    "lint": "bun run --cwd frontend lint",
    "test": "cargo test --manifest-path src-tauri/Cargo.toml",
    "clean": "rm -rf frontend/.next frontend/out src-tauri/target",
    "tauri": "tauri"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2"
  }
}
```

- [ ] **Step 3: Update root `AGENTS.md`**

Replace the Repository Baseline and Command Policy sections:

```markdown
# Expert Base Agent Guidelines

Expert Base is a private, extensible, local-first knowledge base system for professional knowledge workers, shipped as a Tauri 2 desktop application.

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
```

- [ ] **Step 4: Update root `README.md`**

Rewrite the アーキテクチャ方針 section to state: local-first Tauri 2 desktop app; UI is Next.js static export in `frontend/`; Rust core in `src-tauri/`; knowledge base data lives locally (Markdown files + derived index); server-dependent capabilities (sync, web publish, bot hosting) are future optional cloud services. Update the command list to match the root `package.json` scripts (`setup`, `dev`, `build`, `lint`, `test`, `clean`) and drop the `ENV=production` compose instructions.

- [ ] **Step 5: Update `frontend/AGENTS.md`**

Replace the sentence about backend API clients in "Next.js Practices":

- Old: "Keep API calls to the backend behind a small typed client in `src/lib` instead of scattering `fetch` calls through UI components."
- New: "Keep Tauri IPC calls behind the typed client in `src/lib/tauri` instead of scattering `invoke` calls through UI components."

Also add one line to Technical Baseline: "Build target: static export (`output: \"export\"`) loaded by the Tauri 2 shell; no server runtime."

- [ ] **Step 6: Verify the root scripts work**

Run from repo root: `bun run lint && bun run test`
Expected: both pass.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat!: remove web backend/infra; repo is now a Tauri desktop app"
```

---

### Task 7: End-to-end smoke test

**Files:** none

- [ ] **Step 1: Full check suite**

Run from repo root:

```bash
bun run lint && bun run test && bun run build
```

Expected: all pass.

- [ ] **Step 2: Launch the app in dev mode**

Run from repo root in the background: `bun run dev`
Wait for the Rust build to finish and the window process to appear.

Verify: `pgrep -fl "expert-base"` shows the app process (and the UI loaded without panics in the tauri dev output). Then terminate the dev process.

- [ ] **Step 3: Final commit (if any fixes were needed)**

```bash
git add -A
git commit -m "fix(desktop): smoke test fixes"
```
