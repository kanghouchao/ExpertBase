# 後端エラー国際化 設計

> Issue #12。目的：Rust 後端のエラーメッセージが中/日/英混在で、前端が翻訳できない問題を解消する。

## 背景

`src-tauri/AGENTS.md` の既定パターンは全コマンドが `Result<T, String>` を返し、`.map_err(|e| e.to_string())` で内部エラーを素通しする。この結果:

- 手書きの業務エラー文案（約 18 箇所）が中国語または日本語で domain/application 層にハードコードされている。
- 底層ライブラリ（io/sqlite/reqwest 等）のエラーがそのまま英語で前端へ透過している（約 83 箇所の `.map_err(|e| e.to_string())`）。
- 前端はどちらも `e instanceof Error ? e.message : String(e)` で受け取りそのまま表示するだけで、翻訳の入口がない。

前端には既に軽量 i18n（`shared/i18n`）があり、`createT(lang)` が `{param}` 補間付きの `t(key, params)` を提供し、zh/en/ja 三言語のフラットな辞書（`dictionaries.ts`）を持つ。この設計はエラー側をこの既存機構に接続する。

## スコープ

**対象**：手書きの業務エラー文案のみ（下記カタログの 18 個）。加えて、底層ライブラリの素通しエラー（83 箇所）は個別にコード化せず、**1 個の汎用フォールバック**（`err.generic`）にまとめ、翻訳済み前置き＋原文詳細を両方見せる（ユーザーは専門知識ワーカーなので、原文詳細は隠さずトラブルシュート用に残す）。

**対象外**（探索中に判明、意図的に除外）：

- `extract/infrastructure/doc.rs` の PDF/Word 解析失敗文案、および `workshop/infrastructure/tools.rs` の `read_blocking`/`search_blocking`/`write_blocking` が返す `"(read error: ...)"` 等の文字列。これらは IPC の `Result<T, String>` ではなく、**エージェントのツールループがモデルへ返す文字列**（`read_blocking` のコメント: 「エラーは全てモデル向け文字列で返す」）。ユーザーの UI に直接表示されるものではないため、`AppError` 契約の対象にしない。触らない。
- `frontend/src/app/layout.tsx` の `catch(_){}` はテーマ初期化スクリプトで、エラー内容自体を扱っていない。対象外。
- `entities/knowledge-base/model/store.ts` の `refreshKbs()` が state に積む `error` フィールドは、現状どのコンポーネントからも参照されていない（表示されていない死んだフィールド）。今回は触らない — 将来表示する時に、下記の同じパターン（構造化エラーを保持し、描画時に翻訳）を適用すればよい。

## Rust 側：統一エラー型 `AppError`

新規 `src-tauri/src/error.rs`（`lib.rs` に `mod error;` を追加）。4 つの feature モジュール全てが同じ契約を共有するため、初めての top-level shared モジュールとして正当化される。

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
  /// 前端辞書の完全な key（例 "err.kb.nameRequired"）。前端側でのプレフィックス合成はしない。
  pub code: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub params: Option<BTreeMap<String, String>>,
}

impl AppError {
  pub fn code(code: &str) -> Self { ... }                                   // パラメータなし
  pub fn param(code: &str, key: &str, value: impl std::fmt::Display) -> Self { ... } // 単一パラメータ
  pub fn params(code: &str, pairs: impl IntoIterator<Item = (&'static str, String)>) -> Self { ... } // 複数
  pub fn generic(e: impl std::fmt::Display) -> Self {
    Self::param("err.generic", "detail", e)
  }
}
```

`Display` は実装しない（`.to_string()` で暗黙にユーザー向け文言化する経路を断つ——今回の問題の再発防止）。ログに出す場合は `{:?}` を使う。

### 移行方針

- `Result` を返す 22 個の `#[tauri::command]`（kb 13 / agent 5 / workshop 4。`workshop_cancel` は `Result` を返さないため対象外）の戻り値を `Result<T, String>` → `Result<T, AppError>` に変更。
- kb / extract(呼ばれる範囲のみ) / workshop の domain・application・infrastructure 層で、`Result<T, String>` を使っている箇所は `Result<T, AppError>` に変える。ハードコードされた 18 個の文案は `AppError::code(...)` / `AppError::param(...)` に、`.map_err(|e| e.to_string())` は `.map_err(AppError::generic)` に置き換える。
- `agent::AiError` 列挙体は削除する。3 バリアント（Network/Other/Cancelled）は各構築箇所で直接 `AppError` を組み立てる形に置き換える。理由：`Other` が「手書き文案」と「ライブラリ素通し」を同じ型に混在させていて、変換時に区別できなかった。`AppError` に統一すれば区別は構築時点で自明になる。

## エラーコードカタログ

前端 zh/en/ja 辞書に `err.*` として追加する完全な key 一覧。

| code | 元の文案 | params |
|---|---|---|
| `err.kb.noActiveKb` | 没有激活的知识库 | — |
| `err.kb.nameRequired` | 知识库名称不能为空 | — |
| `err.kb.pathRequired` | 存储位置不能为空 | — |
| `err.kb.pathAlreadyRegistered` | 该位置已注册为知识库 | — |
| `err.kb.pathAlreadyHasKb` | 该目录已经包含 ExpertBase 知识库，请选择其他位置 | — |
| `err.kb.notFound` | 未找到该知识库 | — |
| `err.kb.metaNotDirectory` | 知识库元数据不是目录 | — |
| `err.kb.entryFrontmatterMissing` | frontmatter が見つかりません | — |
| `err.kb.entryFrontmatterUnterminated` | frontmatter の終端が見つかりません | — |
| `err.kb.pathMustBeRelative` | 知识库路径必须是相对路径 | — |
| `err.kb.pathOutsideAllowedDir` | 知识库路径不在允许的 Markdown 目录内 | — |
| `err.kb.duplicateEntryName` | 同名の条目が既に存在します | `path` |
| `err.agent.emptyConversation` | 対話メッセージが空です | — |
| `err.agent.modelListFailed` | 模型列表读取失败 | `status`, `detail` |
| `err.agent.network` | 网络错误 | `detail` |
| `err.agent.cancelled` | 已取消 | — |
| `err.workshop.kbSwitchedDuringSave` | 知识库已切换，已取消保存对话 | — |
| `err.workshop.sourceMustBeAbsolute` | source must be an absolute path | `id` |
| `err.workshop.conversationNotFound` | conversation not found（`history.rs`、実装中に発見） | `id` |
| `err.workshop.conversationCorrupted` | conversation has no meta（`history.rs`、実装中に発見） | `id` |
| `err.generic` | （新設）汎用フォールバック — 翻訳済み前置き + 原文詳細 | `detail` |

各キーは zh/en/ja 三言語ぶん、`dictionaries.ts` に追記する（既存の `{param}` 補間をそのまま使う）。

## 前端側

`shared/api/tauri/client.ts` に型を追加:

```ts
export type AppError = { code: string; params?: Record<string, string> };
```

`shared/i18n/translate.ts`（または隣接ファイル）に変換ヘルパーを追加:

```ts
export function isAppError(e: unknown): e is AppError { ... }
export function translateError(t: Translate, e: unknown): string {
  if (isAppError(e)) return t(e.code, e.params);
  return String(e);
}
```

**呼び出し側の扱いは 2 パターン**（catch の場所が React コンポーネント内かどうかで分岐）:

1. **コンポーネント内 catch**（`t` がすぐ手に入る）: その場で `translateError(t, e)` を呼び、結果の文字列を `useState` に格納する。対象: `widgets/app-shell/kb-switcher.tsx`、`features/onboarding/ui/onboarding.tsx`、`features/workshop/ui/workshop-view.tsx`（3 箇所）。
2. **コンポーネント外の store**（`features/workshop/model/workshop-run.ts`）: `t` を呼べないので、生のエラー値をそのまま state に保持する（現状の `message: string` を `cause: unknown` に変更）。実際に描画する側（`workshop-view.tsx` の `{error ?? runError?.message}` 相当の箇所）で `translateError(t, runError?.cause)` を呼ぶ。

## ドキュメント更新

`src-tauri/AGENTS.md` の「IPC Command Practices」節を更新し、`Result<T, String>` / `map_err(|e| e.to_string())` の記述を `Result<T, AppError>` / `AppError` コンストラクタの記述に置き換える。

## テスト方針（TDD）

- `error.rs`: `AppError::code`/`param`/`params`/`generic` のシリアライズ形状（`code` + `params` の有無、camelCase）を検証する単体テストを先に書く。
- 各 feature の既存テストで、以前 `.contains("知识库已切换")` のように文言を assert していた箇所は `assert_eq!(err.code, "err.workshop.kbSwitchedDuringSave")` のようにコードを assert する形に書き換える（先に失敗させてから実装を直す）。
- 前端: `translateError` / `isAppError` の単体テスト（`bun:test`、`frontend/` 配下、既存の `*.test.ts` と同じ流儀）。

## 検証

- `bun run test`（cargo）green。
- `bun run lint` + `bun run --cwd frontend build` green。
