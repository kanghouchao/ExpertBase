# Workshop ESC Stop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 表示中の Workshop 対話を Escape で停止し、AI 出力前なら送信直前の入力と履歴へ戻す。

**Architecture:** 既存の `workshop-run` ストアを停止判定の唯一の場所にする。UI は現在表示中の KB と対話 ID を渡して停止し、ストアが返した出力前ロールバック情報だけを入力欄と保存済み履歴へ反映する。

**Tech Stack:** TypeScript, React 19, Next.js 16, Bun test

---

### Task 1: 停止結果と対話境界をモデルで保証する

**Files:**
- Modify: `frontend/src/features/workshop/model/workshop-run.test.ts`
- Modify: `frontend/src/features/workshop/model/workshop-run.ts`

- [ ] **Step 1: 出力前ロールバックと対話境界の失敗テストを書く**

`describe("stopActive", ...)` に次を追加する。各テストでは未完了の `chatImpl` を使い、後続テストへ実行を残さないよう最後に reject する。

```typescript
test("returns the prompt and previous history when stopped before AI output", async () => {
  let rejectChat: (reason: unknown) => void = () => {};
  chatImpl = () =>
    new Promise<string>((_resolve, reject) => {
      rejectChat = reject;
    });

  startRun({
    kbPath: "/home/user/ExpertBase/tea",
    conversationId: 8,
    sourceIds: [],
    baseHistory: [
      { role: "user", text: "前の問い" },
      { role: "ai", text: "前の答え" },
      { role: "user", text: "やり直す問い" },
    ],
    model: "qwen3:8b",
    think: true,
    tools: true,
  });
  await tick();

  expect(stopActive("/home/user/ExpertBase/tea", 8)).toEqual({
    prompt: "やり直す問い",
    history: [
      { role: "user", text: "前の問い" },
      { role: "ai", text: "前の答え" },
    ],
  });
  expect(getSnapshot().active).toBeNull();
  expect(cancelCalls).toBe(1);

  rejectChat(new Error("Cancelled"));
  await tick();
  expect(saveCalls).toHaveLength(0);
});

test("does not stop a background conversation", async () => {
  let rejectChat: (reason: unknown) => void = () => {};
  chatImpl = () =>
    new Promise<string>((_resolve, reject) => {
      rejectChat = reject;
    });

  startRun({
    kbPath: "/home/user/ExpertBase/tea",
    conversationId: 9,
    sourceIds: [],
    baseHistory: [{ role: "user", text: "問" }],
    model: "qwen3:8b",
    think: false,
    tools: true,
  });
  await tick();

  expect(stopActive("/home/user/ExpertBase/tea", 10)).toBeNull();
  expect(getSnapshot().active?.conversationId).toBe(9);
  expect(cancelCalls).toBe(0);

  discardActive();
  rejectChat(new Error("Cancelled"));
  await tick();
});
```

既存の部分本文テストは `stopActive("C:\\Users\\user\\ExpertBase\\tea", 3)` を呼び、戻り値が `null` であることも確認する。

- [ ] **Step 2: 関連テストを実行して RED を確認する**

Run:

```bash
bun test frontend/src/features/workshop/model/workshop-run.test.ts
```

Expected: `stopActive` が引数を受けず値を返さないため、新しい期待が失敗する。

- [ ] **Step 3: `stopActive` を最小限拡張する**

`frontend/src/features/workshop/model/workshop-run.ts` に戻り値型を追加する。

```typescript
export type StopResult = {
  prompt: string;
  history: WorkshopMessage[];
};
```

`stopActive` を次の挙動へ変更する。

```typescript
/** 表示中の対話だけを停止する。出力前なら送信直前へ戻す情報を返す。 */
export function stopActive(
  kbPath: string | null | undefined,
  conversationId: number | null
): StopResult | null {
  const active = state.active;
  if (!isRunForConversation(active, kbPath, conversationId)) return null;
  cancelled = true;

  const hasOutput = !!active.narration || !!active.thinking || active.tools.length > 0;
  if (hasOutput) {
    void workshopCancel();
    return null;
  }

  const last = active.baseHistory.at(-1);
  const rollback =
    last?.role === "user"
      ? { prompt: last.text, history: active.baseHistory.slice(0, -1) }
      : null;
  emit({ active: null, error: null });
  void workshopCancel();
  return rollback;
}
```

この形では、出力前停止時にストアを先に閉じるため、その後届いたストリーム断片や reject は既存の `isActive` ガードで無視される。部分出力がある場合は現在の catch/save 経路をそのまま使う。

- [ ] **Step 4: 関連テストを実行して GREEN を確認する**

Run:

```bash
bun test frontend/src/features/workshop/model/workshop-run.test.ts
```

Expected: 全テスト PASS、失敗 0。

### Task 2: Escape を表示中の対話へ配線し、出力前履歴を戻す

**Files:**
- Modify: `frontend/src/features/workshop/ui/workshop-view.tsx`

- [ ] **Step 1: 停止ハンドラーへロールバック保存を追加する**

現在の `handleStop` を `useCallback` にし、モデルへ表示中の KB と対話 ID を渡す。ロールバックが返った場合だけ、入力と表示履歴を即座に戻し、既存 API で送信前履歴を上書きする。

```typescript
const handleStop = useCallback(() => {
  const rollback = stopActive(active?.path, viewedId);
  if (!rollback || !active?.path || viewedId === null) return;

  setInstruction(rollback.prompt);
  setMessages(rollback.history);
  void saveWorkshopConversation({
    kbPath: active.path,
    id: viewedId,
    sourceIds: sources.map((source) => source.id),
    messages: rollback.history,
  })
    .then(() => notifyWorkshopHistoryChanged())
    .catch((saveError) =>
      setError(saveError instanceof Error ? saveError.message : String(saveError))
    );
}, [active?.path, sources, viewedId]);
```

保存失敗時は入力内容を失わず、既存のエラー領域へ具体的な保存エラーを表示する。

- [ ] **Step 2: 生成中だけ Escape リスナーを登録する**

`handleStop` の直後に effect を追加する。

```typescript
useEffect(() => {
  if (!generating) return;
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") handleStop();
  };
  window.addEventListener("keydown", onKeyDown);
  return () => window.removeEventListener("keydown", onKeyDown);
}, [generating, handleStop]);
```

停止ボタンは同じ `handleStop` を使い続ける。`generating` は `isRunForConversation` から算出済みなので、別対話のバックグラウンド生成中にはリスナーを登録しない。

- [ ] **Step 3: frontend lint を実行する**

Run:

```bash
bun run lint
```

Expected: exit 0、ESLint error 0。

- [ ] **Step 4: frontend build を実行する**

Run:

```bash
bun run --cwd frontend build
```

Expected: exit 0、Next.js static export が完了する。

- [ ] **Step 5: 全関連テストを再実行する**

Run:

```bash
bun test frontend/src/features/workshop/model/workshop-run.test.ts
```

Expected: 全テスト PASS、失敗 0。

- [ ] **Step 6: 実装をコミットする**

```bash
git add frontend/src/features/workshop/model/workshop-run.ts \
  frontend/src/features/workshop/model/workshop-run.test.ts \
  frontend/src/features/workshop/ui/workshop-view.tsx
git commit -m "feat(workshop): stop generation with Escape"
```
