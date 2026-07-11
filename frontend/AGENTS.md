# Frontend Agent Guidelines

## Scope

This directory contains the Expert Base UI. It is a Next.js App Router project using TypeScript, React, Tailwind CSS, and shadcn/ui, statically exported and loaded by the Tauri 2 desktop shell in `src-tauri/`.

When working under `frontend/`, follow this file in addition to the repository-level instructions.

## Technical Baseline

- Runtime and package manager: Bun.
- Framework: Next.js App Router.
- Language: TypeScript.
- UI runtime: React Server Components by default.
- Styling: Tailwind CSS v4 through `src/app/globals.css`.
- Component system: shadcn/ui source components under `src/shared/ui`.
- Import alias: `@/*`.
- Build target: static export (`output: "export"`) loaded by the Tauri 2 shell; no server runtime.

## Command Policy

- From the repository root, prefer the root `package.json` scripts for setup, dev, build, lint, and test.
- Use frontend-local scripts only for operations the root scripts do not expose, such as `bun run format` from `frontend/`.
- Run other frontend-local `bun` or `bunx` commands only when the scripts do not expose the needed operation. If doing so, state why.

## Directory Rules

- `src/app/`: routes, layouts, pages, route groups, and app-level loading/error files. Pages stay thin and compose feature screens through their public API.
- `src/widgets/`: app-shell composition that binds features, entities, and shared.
- `src/features/`: complete user scenarios (UI under `ui/`), exposed through a slice `index.ts`.
- `src/entities/`: domain-facing client models, types, and adapters under `model/` (no UI), exposed through a slice `index.ts`.
- `src/shared/`: reusable primitives and framework-neutral helpers — `shared/ui` (shadcn + custom primitives), `shared/api/tauri` (typed IPC client), `shared/config`, `shared/i18n`, `shared/lib`, `shared/providers`.
- `src/shared/ui/`: shadcn/ui generated primitives and shared custom primitives. Do not edit shadcn primitives casually; treat changes as local forks of upstream components.

Create new folders only when the first real file needs them. Do not add empty architecture folders.

## Feature-Sliced Design

Use Feature-Sliced Design (FSD) for frontend code organization, adapted to Next.js App Router.

- `src/app/` remains the routing and composition layer. Route files read routing context, compose screens, and stay thin.
- `src/features/` contains complete user scenarios, including scenario-specific UI, state, API calls, and logic.
- `src/entities/` contains domain-facing client models, types, validation, and pure functions. It must not contain UI or transport/client invocation logic.
- `src/shared/` contains reusable UI primitives, framework-neutral utilities, configuration, and typed clients that can be used by any layer.
- The codebase has migrated to FSD layers; `src/lib/` and `src/components/` no longer exist. Shared code lives under `src/shared/`, and shadcn primitives under `src/shared/ui/`.
- Add new FSD folders only when the first real file needs them. Do not create empty architecture folders.

Dependencies must flow downward:

```text
src/app -> src/widgets -> src/features -> src/entities -> src/shared
```

`src/widgets/` composes feature and entity slices into reusable app-shell sections, but must not own business rules. If another FSD layer becomes necessary, document the boundary before introducing it. Cross-slice imports must go through a slice public API such as `index.ts`; do not import another slice's internals.

## Next.js Practices

- Prefer Server Components. Add `"use client"` only when a component needs browser APIs, state, effects, event handlers, or client-only libraries.
- Keep data fetching close to the route or server component that uses it.
- Avoid request waterfalls. Start independent async work early and await it together.
- Do not pass large server objects into Client Components. Pass minimal serialized props.
- Use route-level `loading.tsx`, `error.tsx`, and `not-found.tsx` only when the route needs them.
- Keep Tauri IPC calls behind the typed client in `src/shared/api/tauri` instead of scattering `invoke` calls through UI components.

## shadcn/ui Practices

- Use shadcn/ui components before creating custom primitives.
- Add components with `bunx shadcn add <component>`. Note: `components.json` aliases still point to `@/components` and `@/lib`, so generated files land in `src/components/ui` — move them to `src/shared/ui` and fix imports (`@/lib/utils` → `@/shared/lib/utils`).
- Check installed components before importing them.
- Use semantic tokens such as `bg-background`, `text-foreground`, `text-muted-foreground`, `border-border`, and `bg-card`.
- Use `cn()` from `src/shared/lib/utils.ts` for conditional classes.
- Use `gap-*` for spacing. Do not use `space-x-*` or `space-y-*`.
- Use `size-*` when width and height are equal.
- Use lucide icons in buttons when an icon exists. Put the icon inside the `Button` instead of hand-writing SVG.

## Styling Rules

- Keep global design tokens in `src/app/globals.css`.
- Do not create additional global CSS files unless the need is project-wide.
- Prefer layout utilities and design tokens over raw color classes.
- Avoid one-off visual systems inside feature components. If a pattern repeats, promote it to `src/shared/ui`.
- Do not introduce CSS-in-JS unless explicitly required.

## State and Data

- Keep server state on the server first.
- Use Client Components for local interaction state only.
- Do not introduce global state libraries without a concrete cross-route state requirement.
- When backend contracts exist, generate or centralize types instead of duplicating response shapes manually.

## Quality Bar

- For behavior changes, write or update the relevant test before implementation. If a meaningful frontend test cannot be written first, state why before coding.
- Frontend tests use Bun's built-in runner (`bun:test`), colocated as `*.test.ts` (or `*.test.tsx` when rendering components). Run them with `bun test` from `frontend/`.
- Before finishing frontend changes, run `bun run lint`.
- Run `bun run build` when changing routing, layout, config, imports, or anything that can affect production compilation.
- If build fails due to local sandbox or network restrictions, report the exact reason instead of hiding it.
- Do not modify generated shadcn/ui components unless the change is intentional and documented.
