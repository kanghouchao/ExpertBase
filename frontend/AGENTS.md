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
- Component system: shadcn/ui source components under `src/components/ui`.
- Import alias: `@/*`.
- Build target: static export (`output: "export"`) loaded by the Tauri 2 shell; no server runtime.

## Command Policy

- Prefer Taskfile commands for frontend work.
- Use `task install`, `task dev`, `task lint`, `task build`, and `task format` instead of calling `bun` directly.
- Run direct `bun` or `bunx` commands only when the Taskfile does not expose the needed operation. If doing so, state why.

## Directory Rules

- `src/app/`: routes, layouts, pages, route groups, and app-level loading/error files.
- `src/components/ui/`: shadcn/ui generated primitives. Do not edit casually; treat changes as local forks of upstream components.
- `src/lib/`: framework-neutral utilities, API clients, and configuration helpers.

Create new folders only when the first real file needs them. Do not add empty architecture folders.

## Next.js Practices

- Prefer Server Components. Add `"use client"` only when a component needs browser APIs, state, effects, event handlers, or client-only libraries.
- Keep data fetching close to the route or server component that uses it.
- Avoid request waterfalls. Start independent async work early and await it together.
- Do not pass large server objects into Client Components. Pass minimal serialized props.
- Use route-level `loading.tsx`, `error.tsx`, and `not-found.tsx` only when the route needs them.
- Keep Tauri IPC calls behind the typed client in `src/lib/tauri` instead of scattering `invoke` calls through UI components.

## shadcn/ui Practices

- Use shadcn/ui components before creating custom primitives.
- Add components with `task shadcn:add -- <component>`.
- Check installed components before importing them.
- Use semantic tokens such as `bg-background`, `text-foreground`, `text-muted-foreground`, `border-border`, and `bg-card`.
- Use `cn()` from `src/lib/utils.ts` for conditional classes.
- Use `gap-*` for spacing. Do not use `space-x-*` or `space-y-*`.
- Use `size-*` when width and height are equal.
- Use lucide icons in buttons when an icon exists. Put the icon inside the `Button` instead of hand-writing SVG.

## Styling Rules

- Keep global design tokens in `src/app/globals.css`.
- Do not create additional global CSS files unless the need is project-wide.
- Prefer layout utilities and design tokens over raw color classes.
- Avoid one-off visual systems inside feature components. If a pattern repeats, promote it to `src/components`.
- Do not introduce CSS-in-JS unless explicitly required.

## State and Data

- Keep server state on the server first.
- Use Client Components for local interaction state only.
- Do not introduce global state libraries without a concrete cross-route state requirement.
- When backend contracts exist, generate or centralize types instead of duplicating response shapes manually.

## Quality Bar

- Before finishing frontend changes, run `task lint`.
- Run `task build` when changing routing, layout, config, imports, or anything that can affect production compilation.
- If build fails due to local sandbox or network restrictions, report the exact reason instead of hiding it.
- Do not modify generated shadcn/ui components unless the change is intentional and documented.
