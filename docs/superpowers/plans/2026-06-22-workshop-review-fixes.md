# Workshop Review Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Workshop の素材出典と草稿を一致させ、複数ターンの会話に完全な草稿状態を渡す。

**Architecture:** テスト可能な草稿・会話状態変換を純粋な TypeScript 関数へ分離する。最初の会話後は素材の追加だけを許可し、追加素材を含む entry 草稿が生成されるまで確定を無効化する。

**Tech Stack:** React 19、TypeScript、Tauri IPC、Node.js 24 built-in test runner

---

### Task 1: Workshop 状態規則を抽出してテストする

**Files:**
- Create: `frontend/src/features/workshop/model/process-state.ts`
- Create: `scripts/workshop-process-state.test.mjs`

- [ ] 失敗する純粋関数テストを追加する。
- [ ] `node --test scripts/workshop-process-state.test.mjs` で RED を確認する。
- [ ] 完全な assistant turn、最新 entry 更新、手動草稿結合、素材 ID 比較を最小実装する。
- [ ] 同じテストで GREEN を確認する。
- [ ] `test(workshop): define draft source consistency rules` としてコミットする。

### Task 2: 会話と素材状態へ接続する

**Files:**
- Modify: `frontend/src/features/workshop/ui/workshop-process-view.tsx`
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

- [ ] assistant 履歴へ完全な `StructureResult` JSON を渡す。
- [ ] 最後の entry の手動編集を会話履歴へ同期する。
- [ ] 草稿生成時の素材 ID を記録し、現在の素材と違う間は確定を無効化する。
- [ ] 最初の会話後は素材削除を無効化し、素材追加は許可する。
- [ ] 追加素材を含む entry が生成されるまで再生成を促す多言語メッセージを表示する。
- [ ] Node テスト、lint、frontend build、Rust テストを実行する。
- [ ] `fix(workshop): keep drafts aligned with source context` としてコミットする。

