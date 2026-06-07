# Expert Base Frontend

This directory contains the Expert Base web application.

The frontend is responsible for the product UI: knowledge workspace management, source import flows, Wiki review workflows, assistant configuration, publishing controls, and future administration screens.

## Technology Stack

- Next.js App Router
- React
- TypeScript
- Bun
- Tailwind CSS v4
- shadcn/ui
- lucide-react

The frontend is a server-first Next.js application. Server Components should be the default. Client Components are used only for browser-only behavior, local interaction state, forms, and rich UI controls.

## Current Baseline

The project was initialized with `create-next-app` and Bun.

shadcn/ui has been initialized and the following base components are available:

- `button`
- `card`
- `input`
- `label`
- `select`
- `textarea`

These components live in `src/components/ui` as source files. They are part of the local codebase, not opaque dependencies.

## Directory Structure

```txt
frontend/
  src/
    app/                 # App Router routes, layouts, and global CSS
    components/
      ui/                # shadcn/ui primitives
    lib/                 # Shared utilities and future API clients
  public/                # Static assets
  components.json        # shadcn/ui configuration
  next.config.ts         # Next.js configuration
  eslint.config.mjs      # ESLint configuration
  tsconfig.json          # TypeScript configuration
  package.json           # Bun scripts and dependencies
  bun.lock               # Bun lockfile
```

As the product grows, use these folders:

```txt
src/components/          # Shared product components
src/features/            # Feature-level UI, actions, and client interactions
src/hooks/               # Reusable client hooks
src/types/               # Frontend-only TypeScript types
```

Do not create empty architecture folders. Add a folder when the first real file needs it.

## Commands

Run commands from `frontend/`.

```bash
task install
task dev
task lint
task build
task start
```

`task dev` starts the local Next.js development server.

`task lint` must pass before frontend changes are considered complete.

`task build` should be used when changing routing, layout, config, imports, or production-sensitive code.

The Taskfile wraps the underlying Bun commands. AI agents and contributors should prefer the `task` commands so local workflows stay consistent.

## Architecture Principles

### Server-first UI

Use React Server Components by default. Add `"use client"` only when a component needs:

- browser APIs
- React state
- effects
- event handlers
- client-only libraries

Keep Client Components small and pass them minimal serialized props.

### App Router Boundaries

Use `src/app` for route-level composition:

- `layout.tsx`
- `page.tsx`
- `loading.tsx`
- `error.tsx`
- `not-found.tsx`

Do not put reusable product components directly inside route files once they become non-trivial. Move them to `src/components` or `src/features`.

### API Access

Backend API calls should be centralized behind typed helpers in `src/lib` instead of being scattered through UI components.

When backend OpenAPI contracts are available, prefer generated or centralized types over handwritten duplicate response shapes.

### Styling

Use Tailwind CSS and shadcn/ui design tokens.

Prefer semantic tokens:

```txt
bg-background
text-foreground
text-muted-foreground
bg-card
border-border
```

Avoid raw color utilities for product UI unless the color is part of an explicit design decision.

Use `gap-*` for spacing instead of `space-x-*` or `space-y-*`.

Use `cn()` from `src/lib/utils.ts` for conditional class composition.

### Components

Use shadcn/ui primitives before creating custom primitives.

Add shadcn/ui components with:

```bash
task shadcn:add -- <component>
```

Treat files under `src/components/ui` as local copies of upstream shadcn/ui components. Do not edit them casually. If a component needs product-specific behavior, compose it from `src/components` instead.

### State

Keep server data on the server when possible.

Use Client Component state for local UI interactions only.

Do not introduce a global state library until there is a real cross-route state requirement.

## Development Notes

This project uses the Next.js version installed in `package.json`. Some APIs and conventions may differ from older Next.js examples. When in doubt, read the local docs under:

```txt
node_modules/next/dist/docs/
```

The local agent instructions are in:

```txt
AGENTS.md
```
