# Markdown LaTeX Rendering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render inline and block LaTeX expressions in the existing shared Markdown component.

**Architecture:** Extend the current `react-markdown` pipeline with `remark-math` for parsing and `rehype-katex` for static rendering. Load KaTeX's packaged stylesheet once from the root layout so Workshop and Wiki callers gain support without changing their data flow.

**Tech Stack:** React 19, Next.js 16, Bun test, react-markdown, remark-math, rehype-katex, KaTeX

---

### Task 1: Add a failing LaTeX rendering test

**Files:**
- Create: `frontend/src/shared/ui/markdown.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import { Markdown } from "./markdown";

test("Markdown renders inline LaTeX with KaTeX", () => {
  const html = renderToStaticMarkup(<Markdown>{"$\\rightarrow$"}</Markdown>);

  expect(html).toContain('class="katex"');
});
```

- [ ] **Step 2: Run the test and verify the expected failure**

Run: `bun test frontend/src/shared/ui/markdown.test.tsx`

Expected: FAIL because the rendered HTML does not contain `class="katex"`.

### Task 2: Enable KaTeX in the shared renderer

**Files:**
- Modify: `frontend/package.json`
- Modify: `frontend/bun.lock`
- Modify: `frontend/src/shared/ui/markdown.tsx:3-5,74`
- Modify: `frontend/src/app/layout.tsx:1-3`

- [ ] **Step 1: Install the standard math rendering packages**

Run: `bun add --cwd frontend remark-math rehype-katex katex`

Expected: `frontend/package.json` and `frontend/bun.lock` include all three packages.

- [ ] **Step 2: Add the math plugins to the existing Markdown pipeline**

Update the imports and `ReactMarkdown` call in `frontend/src/shared/ui/markdown.tsx`:

```tsx
import ReactMarkdown, { type Components } from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

// existing components stay unchanged

<ReactMarkdown
  remarkPlugins={[remarkGfm, remarkMath]}
  rehypePlugins={[rehypeKatex]}
  components={components}
>
  {children}
</ReactMarkdown>
```

- [ ] **Step 3: Load KaTeX styles once from the root layout**

Update the imports in `frontend/src/app/layout.tsx`:

```tsx
import type { Metadata } from "next";
import "katex/dist/katex.min.css";
import "./globals.css";
```

- [ ] **Step 4: Run the focused test and verify it passes**

Run: `bun test frontend/src/shared/ui/markdown.test.tsx`

Expected: 1 pass, 0 failures.

- [ ] **Step 5: Commit the behavior change**

```bash
git add frontend/package.json frontend/bun.lock frontend/src/app/layout.tsx frontend/src/shared/ui/markdown.tsx frontend/src/shared/ui/markdown.test.tsx
git commit -m "feat(frontend): render LaTeX in Markdown"
```

### Task 3: Verify frontend quality gates

**Files:**
- No file changes expected

- [ ] **Step 1: Run all frontend unit tests**

Run: `bun test frontend/src`

Expected: all tests pass.

- [ ] **Step 2: Run the repository lint command**

Run: `bun run lint`

Expected: exit code 0 with no lint errors.

- [ ] **Step 3: Run the frontend production build**

Run: `bun run --cwd frontend build`

Expected: exit code 0 and a successful static export.

- [ ] **Step 4: Check the final diff**

Run: `git status --short && git diff --check HEAD~1..HEAD`

Expected: clean worktree and no whitespace errors.
