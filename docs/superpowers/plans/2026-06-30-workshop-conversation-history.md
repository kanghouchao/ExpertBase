# 工坊の対話履歴 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完了した工坊の対話をナレッジベース単位で永続化し、工坊ナビゲーションの直下にページング可能な履歴を表示する。履歴項目から対話を復元でき、工坊本体をクリックするたびに新規対話を開始できるようにする。

**Architecture:** 永続的な対話データは専用の `.expertbase/workshop.sqlite` に保存し、再構築可能な知識インデックスには混在させない。Rust が対話ドメイン、リポジトリ、薄い Tauri コマンドを所有する。フロントエンドは生成中の対話をローカル状態に保持し、助手の応答が成功した後だけ保存する。履歴は `/workshop?conversation=<id>` で指定し、すでに選択中の工坊を再クリックした場合も小さなブラウザイベントで画面を初期化する。

**Tech Stack:** Rust 2021, rusqlite, serde/serde_json, chrono, Tauri 2 IPC, Next.js 16 App Router static export, React 19, TypeScript, Bun test, Tailwind CSS.

---

## スコープと確定事項

- 履歴はアクティブなナレッジベースに属する。KB を切り替えたら、その KB の履歴だけを読み込む。
- 最初の助手応答が成功した時点で対話を作成し、以後の成功応答ごとに更新する。失敗または中断した実行中ターンは保存しない。
- 素材パスと表示に必要な完全なメッセージ状態（`text`、任意の思考本文、ツールカード）を保存し、Ollama へ渡す発話だけでなく表示も復元する。
- 正規化した最初のユーザー発話を 40 Unicode 文字で切ってタイトルにする。タイトル生成用の AI 呼び出しは追加しない。
- バックエンドのページサイズは 20 件固定とする。「さらに読み込む」は次の 20 件を追加し、「折りたたむ」はサイドバー状態を先頭ページだけへ戻す。
- `updated_at DESC, id DESC` で並べ、継続した対話を先頭へ移動する。
- 選択中の Ollama モデルは保存しない。復元後は現在利用可能なモデル選択で継続する。
- 現在の左上「新しい対話」ボタンは削除する。工坊本体のナビゲーション項目を唯一の新規対話操作とし、すでに `/workshop` 上にいる場合の再クリックも対象にする。
- 削除、改名、検索、ピン留め、KB 横断履歴は対象外とする。

## ファイル構成

### 新規作成

- `src-tauri/src/workshop/domain.rs` — 永続対話値と決定的なタイトル生成。
- `src-tauri/src/workshop/infrastructure/history.rs` — 専用 SQLite schema と save/get/page list リポジトリ操作。
- `frontend/src/features/workshop/model/history.ts` — 履歴 URL 解析、先頭ページへの折りたたみ、工坊 browser event。
- `frontend/src/features/workshop/model/history.test.ts` — URL 解析と折りたたみ挙動の純粋フロントエンドテスト。
- `frontend/src/features/workshop/ui/workshop-history-nav.tsx` — 工坊ナビ直下の履歴一覧と追加/折りたたみ操作。

### 変更

- `src-tauri/src/workshop/mod.rs` — 実体のある domain layer を登録。
- `src-tauri/src/workshop/infrastructure/mod.rs` — history repository を application layer へ公開。
- `src-tauri/src/workshop/application.rs` — agent chat の挙動を変えず save/get/list use case を追加。
- `src-tauri/src/workshop/interface.rs` — 履歴用の薄い active-KB Tauri adapter を追加。
- `src-tauri/src/kb/mod.rs` — active-KB root resolver だけを crate 内へ再公開。
- `src-tauri/src/lib.rs` — 3 つの新規 Tauri command を登録。
- `frontend/src/shared/api/tauri/client.ts` — 型付き履歴 IPC contract を一元化。
- `frontend/src/features/workshop/model/process-state.ts` — 永続履歴と同じ message/tool transport 型を再利用。
- `frontend/src/features/workshop/index.ts` — widget layer が必要とする sidebar component と新規対話 event だけを公開。
- `frontend/src/widgets/app-shell/nav-item.tsx` — URL が変わらない場合も工坊 link のクリックを active page へ通知。
- `frontend/src/widgets/app-shell/sidebar.tsx` — 工坊直下へ履歴を表示し、本体項目から新規対話操作を発火。
- `frontend/src/features/workshop/ui/workshop-view.tsx` — query ID による復元、成功 turn の保存、navigation 初期化、旧 button 削除。
- `frontend/src/app/(app)/workshop/page.tsx` — static export で `useSearchParams` に必要な Suspense 境界を追加。
- `frontend/src/shared/i18n/dictionaries.ts` — 中国語、英語、日本語の履歴文言を追加。

## Task 1: 工坊対話の定義と永続化

**Files:**
- Create: `src-tauri/src/workshop/domain.rs`
- Create: `src-tauri/src/workshop/infrastructure/history.rs`
- Modify: `src-tauri/src/workshop/mod.rs`
- Modify: `src-tauri/src/workshop/infrastructure/mod.rs`

- [ ] **Step 1: 失敗するドメイン・リポジトリテストを書く**

実装より先に次のテストを追加する。リポジトリテストはテーブルの存在だけでなく、往復復元、更新順、作成日時の維持、ページ境界を証明する。

テストファイルが実際にコンパイル対象へ入るよう、先に `src-tauri/src/workshop/mod.rs` の既存宣言へ `mod domain;` を追加し、`src-tauri/src/workshop/infrastructure/mod.rs` へ `pub(crate) mod history;` を追加する。この時点ではテストが未定義シンボルを参照するため、次の red 確認で確実に失敗する。

```rust
// src-tauri/src/workshop/domain.rs
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn title_uses_first_user_message_and_truncates_by_characters() {
    let messages = vec![
      WorkshopMessage::assistant("先行応答"),
      WorkshopMessage::user("  这是   一段\n包含空白的标题abcdefghijklmnopqrstuvwxyz  "),
    ];

    assert_eq!(conversation_title(&messages).chars().count(), 40);
    assert!(conversation_title(&messages).starts_with("这是 一段 包含空白的标题"));
  }
}
```

```rust
// src-tauri/src/workshop/infrastructure/history.rs
#[cfg(test)]
mod tests {
  use super::*;
  use crate::workshop::domain::WorkshopMessage;

  #[test]
  fn save_get_update_and_paginate_conversations() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = open(tmp.path()).unwrap();
    let first = save(
      &conn,
      None,
      &["/tmp/a.pdf".into()],
      &[WorkshopMessage::user("第一条"), WorkshopMessage::assistant("回答")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap();
    let second = save(
      &conn,
      None,
      &[],
      &[WorkshopMessage::user("第二条"), WorkshopMessage::assistant("回答")],
      "2026-06-30T02:00:00.000Z",
    )
    .unwrap();

    let loaded = get(&conn, first.id).unwrap();
    assert_eq!(loaded.source_ids, vec!["/tmp/a.pdf"]);
    assert_eq!(loaded.messages[1].text, "回答");

    let updated = save(
      &conn,
      Some(first.id),
      &["/tmp/a.pdf".into()],
      &[
        WorkshopMessage::user("第一条"),
        WorkshopMessage::assistant("回答"),
        WorkshopMessage::user("继续"),
        WorkshopMessage::assistant("新回答"),
      ],
      "2026-06-30T03:00:00.000Z",
    )
    .unwrap();
    assert_eq!(updated.created_at, "2026-06-30T01:00:00.000Z");

    let page = list(&conn, 0, 1).unwrap();
    assert_eq!(page.items[0].id, first.id);
    assert!(page.has_more);
    let page = list(&conn, 1, 1).unwrap();
    assert_eq!(page.items[0].id, second.id);
    assert!(!page.has_more);
  }

  #[test]
  fn updating_unknown_conversation_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let conn = open(tmp.path()).unwrap();
    let error = save(
      &conn,
      Some(999),
      &[],
      &[WorkshopMessage::user("不存在")],
      "2026-06-30T01:00:00.000Z",
    )
    .unwrap_err();
    assert!(error.contains("conversation not found"));
  }
}
```

- [ ] **Step 2: 対象 Rust テストを実行して失敗を確認する**

実行:

```bash
bun run test workshop::
```

期待結果: 新しいドメイン値とリポジトリ関数がまだ無いためコンパイルが失敗する。

- [ ] **Step 3: ドメイン値とタイトル規則を実装する**

`src-tauri/src/workshop/domain.rs` に次の公開ワイヤー/ドメイン値を作る。損失のある変換を避けるため、`WorkshopMessage` は既存フロントエンドの `ProcessMessage` 形状と一致させる。

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopToolEvent {
  pub name: String,
  pub args: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub summary: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopMessage {
  pub role: WorkshopMessageRole,
  pub text: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub thinking: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tools: Option<Vec<WorkshopToolEvent>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkshopMessageRole {
  User,
  Ai,
}

impl WorkshopMessage {
  #[cfg(test)]
  pub(crate) fn user(text: &str) -> Self {
    Self { role: WorkshopMessageRole::User, text: text.into(), thinking: None, tools: None }
  }

  #[cfg(test)]
  pub(crate) fn assistant(text: &str) -> Self {
    Self { role: WorkshopMessageRole::Ai, text: text.into(), thinking: None, tools: None }
  }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopConversationSummary {
  pub id: i64,
  pub title: String,
  pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopConversationPage {
  pub items: Vec<WorkshopConversationSummary>,
  pub has_more: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopConversation {
  pub id: i64,
  pub title: String,
  pub source_ids: Vec<String>,
  pub messages: Vec<WorkshopMessage>,
  pub created_at: String,
  pub updated_at: String,
}

pub fn conversation_title(messages: &[WorkshopMessage]) -> String {
  messages
    .iter()
    .find(|message| message.role == WorkshopMessageRole::User)
    .map(|message| message.text.split_whitespace().collect::<Vec<_>>().join(" "))
    .filter(|title| !title.is_empty())
    .map(|title| title.chars().take(40).collect())
    .unwrap_or_else(|| "New conversation".into())
}
```

Step 1 のテストはファイル末尾に残す。

- [ ] **Step 4: 専用 SQLite リポジトリを実装する**

`src-tauri/src/workshop/infrastructure/history.rs` を作る。この DB は永続的なユーザーワークフロー状態なので、`index.sqlite` から意図的に分離し、`kb_rebuild_index` で消えないようにする。

```rust
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::workshop::domain::{
  conversation_title, WorkshopConversation, WorkshopConversationPage, WorkshopConversationSummary,
  WorkshopMessage,
};

pub fn open(root: &Path) -> Result<Connection, String> {
  let dir = root.join(".expertbase");
  std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
  let conn = Connection::open(dir.join("workshop.sqlite")).map_err(|error| error.to_string())?;
  conn.execute_batch(
    "CREATE TABLE IF NOT EXISTS conversations(
       id INTEGER PRIMARY KEY AUTOINCREMENT,
       title TEXT NOT NULL,
       source_ids TEXT NOT NULL,
       messages TEXT NOT NULL,
       created_at TEXT NOT NULL,
       updated_at TEXT NOT NULL
     );
     CREATE INDEX IF NOT EXISTS idx_conversations_updated
       ON conversations(updated_at DESC, id DESC);",
  )
  .map_err(|error| error.to_string())?;
  Ok(conn)
}

pub fn save(
  conn: &Connection,
  id: Option<i64>,
  source_ids: &[String],
  messages: &[WorkshopMessage],
  now: &str,
) -> Result<WorkshopConversation, String> {
  let title = conversation_title(messages);
  let source_ids_json = serde_json::to_string(source_ids).map_err(|error| error.to_string())?;
  let messages_json = serde_json::to_string(messages).map_err(|error| error.to_string())?;
  let id = match id {
    Some(id) => {
      let changed = conn
        .execute(
          "UPDATE conversations
             SET title=?1, source_ids=?2, messages=?3, updated_at=?4
             WHERE id=?5",
          rusqlite::params![title, source_ids_json, messages_json, now, id],
        )
        .map_err(|error| error.to_string())?;
      if changed == 0 {
        return Err(format!("conversation not found: {id}"));
      }
      id
    }
    None => {
      conn
        .execute(
          "INSERT INTO conversations(title,source_ids,messages,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?4)",
          rusqlite::params![title, source_ids_json, messages_json, now],
        )
        .map_err(|error| error.to_string())?;
      conn.last_insert_rowid()
    }
  };
  get(conn, id)
}

pub fn get(conn: &Connection, id: i64) -> Result<WorkshopConversation, String> {
  conn
    .query_row(
      "SELECT id,title,source_ids,messages,created_at,updated_at
         FROM conversations WHERE id=?1",
      [id],
      |row| {
        let source_ids: String = row.get(2)?;
        let messages: String = row.get(3)?;
        Ok((
          row.get(0)?,
          row.get(1)?,
          source_ids,
          messages,
          row.get(4)?,
          row.get(5)?,
        ))
      },
    )
    .optional()
    .map_err(|error| error.to_string())?
    .ok_or_else(|| format!("conversation not found: {id}"))
    .and_then(|(id, title, source_ids, messages, created_at, updated_at)| {
      Ok(WorkshopConversation {
        id,
        title,
        source_ids: serde_json::from_str(&source_ids).map_err(|error| error.to_string())?,
        messages: serde_json::from_str(&messages).map_err(|error| error.to_string())?,
        created_at,
        updated_at,
      })
    })
}

pub fn list(conn: &Connection, offset: usize, limit: usize) -> Result<WorkshopConversationPage, String> {
  let mut stmt = conn
    .prepare(
      "SELECT id,title,updated_at FROM conversations
         ORDER BY updated_at DESC, id DESC LIMIT ?1 OFFSET ?2",
    )
    .map_err(|error| error.to_string())?;
  let mut items = stmt
    .query_map(rusqlite::params![(limit + 1) as i64, offset as i64], |row| {
      Ok(WorkshopConversationSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        updated_at: row.get(2)?,
      })
    })
    .map_err(|error| error.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())?;
  let has_more = items.len() > limit;
  items.truncate(limit);
  Ok(WorkshopConversationPage { items, has_more })
}
```

この実装の下に Step 1 のリポジトリテストを残す。

- [ ] **Step 5: 対象テストを実行して成功を確認する**

実行:

```bash
bun run test workshop::
```

期待結果: 新しい工坊ドメイン/履歴テストがすべて通る。

- [ ] **Step 6: 保存層をコミットする**

```bash
git add src-tauri/src/workshop/domain.rs src-tauri/src/workshop/infrastructure/history.rs src-tauri/src/workshop/mod.rs src-tauri/src/workshop/infrastructure/mod.rs
git commit -m "feat(workshop): persist conversation history"
```

## Task 2: application と Tauri IPC から履歴を公開する

**Files:**
- Modify: `src-tauri/src/kb/mod.rs`
- Modify: `src-tauri/src/workshop/application.rs`
- Modify: `src-tauri/src/workshop/interface.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `frontend/src/shared/api/tauri/client.ts`
- Modify: `frontend/src/features/workshop/model/process-state.ts`

- [ ] **Step 1: ラッパー実装前に application テストを追加する**

`src-tauri/src/workshop/application.rs` の既存テストモジュールへ次を追加する:

```rust
#[test]
fn conversation_use_cases_round_trip_in_active_root() {
  let tmp = tempfile::tempdir().unwrap();
  let messages = vec![
    WorkshopMessage {
      role: WorkshopMessageRole::User,
      text: "整理这份资料".into(),
      thinking: None,
      tools: None,
    },
    WorkshopMessage {
      role: WorkshopMessageRole::Ai,
      text: "完成".into(),
      thinking: None,
      tools: None,
    },
  ];

  let saved = save_conversation(tmp.path(), None, vec![], messages).unwrap();
  assert_eq!(get_conversation(tmp.path(), saved.id).unwrap().title, "整理这份资料");
  assert_eq!(list_conversations(tmp.path(), 0).unwrap().items[0].id, saved.id);
}
```

テストモジュールでは `WorkshopMessage` と `WorkshopMessageRole` も import する。

- [ ] **Step 2: application テストを実行して失敗を確認する**

実行:

```bash
bun run test workshop::application::tests::conversation_use_cases_round_trip_in_active_root
```

期待結果: 3 つのユースケース関数が無いためコンパイルが失敗する。

- [ ] **Step 3: 固定ページサイズの application ユースケースを追加する**

`src-tauri/src/workshop/application.rs` に次の import と関数を追加する:

```rust
use chrono::SecondsFormat;

use super::domain::{
  WorkshopConversation, WorkshopConversationPage, WorkshopMessage,
};
use super::infrastructure::history;

const HISTORY_PAGE_SIZE: usize = 20;

pub fn save_conversation(
  root: &Path,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, String> {
  let conn = history::open(root)?;
  let now = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
  history::save(&conn, id, &source_ids, &messages, &now)
}

pub fn get_conversation(root: &Path, id: i64) -> Result<WorkshopConversation, String> {
  history::get(&history::open(root)?, id)
}

pub fn list_conversations(root: &Path, offset: usize) -> Result<WorkshopConversationPage, String> {
  history::list(&history::open(root)?, offset, HISTORY_PAGE_SIZE)
}
```

- [ ] **Step 4: 3 つの薄い Tauri コマンドを追加する**

`src-tauri/src/workshop/interface.rs` で次を import する:

```rust
use crate::workshop::domain::{WorkshopConversation, WorkshopConversationPage, WorkshopMessage};
```

次のアダプタを追加する。ブロッキングなファイルシステム/SQLite 処理はすべて async runtime スレッドの外で実行する:

```rust
#[tauri::command]
pub async fn workshop_save_conversation(
  app: tauri::AppHandle,
  id: Option<i64>,
  source_ids: Vec<String>,
  messages: Vec<WorkshopMessage>,
) -> Result<WorkshopConversation, String> {
  let home = app.path().home_dir().map_err(|error| error.to_string())?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::save_conversation(&root, id, source_ids, messages)
  })
  .await
  .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn workshop_get_conversation(
  app: tauri::AppHandle,
  id: i64,
) -> Result<WorkshopConversation, String> {
  let home = app.path().home_dir().map_err(|error| error.to_string())?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::get_conversation(&root, id)
  })
  .await
  .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn workshop_list_conversations(
  app: tauri::AppHandle,
  offset: usize,
) -> Result<WorkshopConversationPage, String> {
  let home = app.path().home_dir().map_err(|error| error.to_string())?;
  tauri::async_runtime::spawn_blocking(move || {
    let root = crate::kb::active_kb_root(&home)?;
    application::list_conversations(&root, offset)
  })
  .await
  .map_err(|error| error.to_string())?
}
```

`active_kb_root` は現在 `kb::application` 内部にあるため、`src-tauri/src/kb/mod.rs` から `pub(crate) use application::active_kb_root;` として限定公開し、上記では `crate::kb::active_kb_root` を呼ぶ。内部モジュール自体は public にしない。

- [ ] **Step 5: `src-tauri/src/lib.rs` にコマンドを登録する**

`workshop_chat` の後、`workshop_cancel` の前へ次を追加する:

```rust
workshop::interface::workshop_get_conversation,
workshop::interface::workshop_list_conversations,
workshop::interface::workshop_save_conversation,
```

- [ ] **Step 6: 正確な TypeScript IPC 型とクライアントを追加する**

`frontend/src/shared/api/tauri/client.ts` に次を追加する:

```ts
export type WorkshopToolEvent = { name: string; args: string; summary?: string };

export type WorkshopMessage =
  | { role: "user"; text: string; thinking?: never; tools?: never }
  | { role: "ai"; text: string; thinking?: string; tools?: WorkshopToolEvent[] };

export type WorkshopConversationSummary = {
  id: number;
  title: string;
  updatedAt: string;
};

export type WorkshopConversationPage = {
  items: WorkshopConversationSummary[];
  hasMore: boolean;
};

export type WorkshopConversation = {
  id: number;
  title: string;
  sourceIds: string[];
  messages: WorkshopMessage[];
  createdAt: string;
  updatedAt: string;
};

export async function listWorkshopConversations(
  offset: number
): Promise<WorkshopConversationPage> {
  if (!isTauri()) return { items: [], hasMore: false };
  return invoke<WorkshopConversationPage>("workshop_list_conversations", { offset });
}

export async function getWorkshopConversation(id: number): Promise<WorkshopConversation> {
  return invoke<WorkshopConversation>("workshop_get_conversation", { id });
}

export async function saveWorkshopConversation(input: {
  id: number | null;
  sourceIds: string[];
  messages: WorkshopMessage[];
}): Promise<WorkshopConversation> {
  return invoke<WorkshopConversation>("workshop_save_conversation", input);
}
```

続いて `process-state.ts` の重複型を削除し、transport 型を再利用する:

```ts
import type {
  ChatTurn,
  WorkshopMessage,
  WorkshopToolEvent,
} from "@/shared/api/tauri/client";

export type ToolEvent = WorkshopToolEvent;
export type ProcessMessage = WorkshopMessage;
```

- [ ] **Step 7: Rust テストと TypeScript lint を実行する**

実行:

```bash
bun run test workshop::application::tests::conversation_use_cases_round_trip_in_active_root
bun run lint
```

期待結果: 対象 Rust テストが通り、ESLint エラーがない。

- [ ] **Step 8: IPC 層をコミットする**

```bash
git add src-tauri/src/kb/mod.rs src-tauri/src/workshop/application.rs src-tauri/src/workshop/interface.rs src-tauri/src/lib.rs frontend/src/shared/api/tauri/client.ts frontend/src/features/workshop/model/process-state.ts
git commit -m "feat(workshop): expose conversation history IPC"
```

## Task 3: テスト済みのフロントエンド履歴状態とナビゲーションイベントを追加する

**Files:**
- Create: `frontend/src/features/workshop/model/history.ts`
- Create: `frontend/src/features/workshop/model/history.test.ts`

- [ ] **Step 1: 失敗する純粋フロントエンドテストを書く**

```ts
// frontend/src/features/workshop/model/history.test.ts
import { describe, expect, test } from "bun:test";

import { collapseHistory, parseConversationId } from "./history";

describe("parseConversationId", () => {
  test("accepts positive integer ids only", () => {
    expect(parseConversationId("42")).toBe(42);
    expect(parseConversationId(null)).toBeNull();
    expect(parseConversationId("0")).toBeNull();
    expect(parseConversationId("1.5")).toBeNull();
    expect(parseConversationId("abc")).toBeNull();
  });
});

test("collapseHistory keeps exactly the first backend page", () => {
  const items = Array.from({ length: 27 }, (_, index) => ({
    id: index + 1,
    title: `conversation ${index + 1}`,
    updatedAt: "2026-06-30T00:00:00.000Z",
  }));
  expect(collapseHistory(items).map((item) => item.id)).toEqual(
    Array.from({ length: 20 }, (_, index) => index + 1)
  );
});
```

- [ ] **Step 2: テストを実行して失敗を確認する**

実行:

```bash
bun test --cwd frontend src/features/workshop/model/history.test.ts
```

期待結果: `history.ts` が無いため FAIL する。

- [ ] **Step 3: 共有する履歴機構だけを実装する**

```ts
// frontend/src/features/workshop/model/history.ts
import type { WorkshopConversationSummary } from "@/shared/api/tauri/client";

export const HISTORY_PAGE_SIZE = 20;

const NEW_CONVERSATION_EVENT = "expertbase:workshop:new-conversation";
const HISTORY_CHANGED_EVENT = "expertbase:workshop:history-changed";

export function parseConversationId(value: string | null): number | null {
  if (!value || !/^\d+$/.test(value)) return null;
  const id = Number(value);
  return Number.isSafeInteger(id) && id > 0 ? id : null;
}

export function collapseHistory(
  items: WorkshopConversationSummary[]
): WorkshopConversationSummary[] {
  return items.slice(0, HISTORY_PAGE_SIZE);
}

export function requestNewWorkshopConversation(): void {
  window.dispatchEvent(new Event(NEW_CONVERSATION_EVENT));
}

export function onNewWorkshopConversation(listener: () => void): () => void {
  window.addEventListener(NEW_CONVERSATION_EVENT, listener);
  return () => window.removeEventListener(NEW_CONVERSATION_EVENT, listener);
}

export function notifyWorkshopHistoryChanged(): void {
  window.dispatchEvent(new Event(HISTORY_CHANGED_EVENT));
}

export function onWorkshopHistoryChanged(listener: () => void): () => void {
  window.addEventListener(HISTORY_CHANGED_EVENT, listener);
  return () => window.removeEventListener(HISTORY_CHANGED_EVENT, listener);
}
```

- [ ] **Step 4: テストを実行して成功を確認する**

実行:

```bash
bun test --cwd frontend src/features/workshop/model/history.test.ts
```

期待結果: 2 テストが通る。

- [ ] **Step 5: フロントエンド状態層をコミットする**

```bash
git add frontend/src/features/workshop/model/history.ts frontend/src/features/workshop/model/history.test.ts
git commit -m "test(workshop): define history navigation behavior"
```

## Task 4: サイドバーの工坊直下にページング履歴を表示する

**Files:**
- Create: `frontend/src/features/workshop/ui/workshop-history-nav.tsx`
- Modify: `frontend/src/features/workshop/index.ts`
- Modify: `frontend/src/widgets/app-shell/nav-item.tsx`
- Modify: `frontend/src/widgets/app-shell/sidebar.tsx`
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

- [ ] **Step 1: ページング履歴コンポーネントを追加する**

`frontend/src/features/workshop/ui/workshop-history-nav.tsx` を作る。mount 時、アクティブ KB 変更時、成功保存による `history-changed` 発火時に先頭ページを読み込む。追加取得は現在件数を offset とし、古い非同期応答は request 番号で無視する。

```tsx
"use client";

import Link from "next/link";
import { usePathname, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";

import { useKbStore } from "@/entities/knowledge-base";
import {
  listWorkshopConversations,
  type WorkshopConversationSummary,
} from "@/shared/api/tauri/client";
import { cn } from "@/shared/lib/utils";
import { useI18n } from "@/shared/providers/providers";
import { Icon } from "@/shared/ui/icon";
import {
  collapseHistory,
  HISTORY_PAGE_SIZE,
  onWorkshopHistoryChanged,
  parseConversationId,
} from "../model/history";

export function WorkshopHistoryNav() {
  const { t } = useI18n();
  const { active } = useKbStore();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const activeId =
    pathname === "/workshop" ? parseConversationId(searchParams.get("conversation")) : null;
  const [items, setItems] = useState<WorkshopConversationSummary[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const requestRef = useRef(0);

  const loadFirstPage = useCallback(async () => {
    const request = ++requestRef.current;
    setLoading(true);
    setError(false);
    try {
      const page = await listWorkshopConversations(0);
      if (request !== requestRef.current) return;
      setItems(page.items);
      setHasMore(page.hasMore);
    } catch {
      if (request !== requestRef.current) return;
      setItems([]);
      setHasMore(false);
      setError(true);
    } finally {
      if (request === requestRef.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    setItems([]);
    setHasMore(false);
    void loadFirstPage();
    return onWorkshopHistoryChanged(() => void loadFirstPage());
  }, [active?.path, loadFirstPage]);

  async function loadMore() {
    if (loading || !hasMore) return;
    const request = ++requestRef.current;
    setLoading(true);
    setError(false);
    try {
      const page = await listWorkshopConversations(items.length);
      if (request !== requestRef.current) return;
      setItems((current) => [...current, ...page.items]);
      setHasMore(page.hasMore);
    } catch {
      if (request !== requestRef.current) return;
      setError(true);
    } finally {
      if (request === requestRef.current) setLoading(false);
    }
  }

  function collapse() {
    setItems((current) => collapseHistory(current));
    setHasMore(true);
  }

  if (items.length === 0 && !error) return null;

  return (
    <div className="ml-7 border-l border-line pl-2.5">
      <div className="mb-1 px-2 font-mono text-[10px] font-bold tracking-wider text-ink-faint uppercase">
        {t("workshop.history")}
      </div>
      <div className="flex flex-col gap-0.5">
        {items.map((item) => (
          <Link
            key={item.id}
            href={`/workshop?conversation=${item.id}`}
            aria-current={activeId === item.id ? "page" : undefined}
            className={cn(
              "flex min-w-0 items-center gap-1.5 rounded-md px-2 py-1.5 text-[12px] transition-colors",
              activeId === item.id
                ? "bg-surface text-ink"
                : "text-ink-muted hover:bg-surface-2 hover:text-ink"
            )}
          >
            <Icon name="chat" size={12} />
            <span className="truncate">{item.title}</span>
          </Link>
        ))}
      </div>
      {error && <div className="px-2 py-1 text-[11px] text-brand">{t("workshop.historyError")}</div>}
      <div className="mt-1 flex gap-2 px-2">
        {hasMore && (
          <button
            type="button"
            disabled={loading}
            onClick={() => void loadMore()}
            className="text-[11px] font-semibold text-ink-muted hover:text-ink disabled:opacity-40"
          >
            {t("workshop.historyMore")}
          </button>
        )}
        {items.length > HISTORY_PAGE_SIZE && (
          <button
            type="button"
            onClick={collapse}
            className="text-[11px] font-semibold text-ink-muted hover:text-ink"
          >
            {t("workshop.historyCollapse")}
          </button>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: widget 向け feature 公開 API を export する**

`frontend/src/features/workshop/index.ts` を次に変更する:

```ts
export { requestNewWorkshopConversation } from "./model/history";
export { WorkshopHistoryNav } from "./ui/workshop-history-nav";
export { WorkshopView } from "./ui/workshop-view";
```

- [ ] **Step 3: `NavItem` から同一 URL のクリックも通知する**

`NavItem` の props に `onClick?: () => void` を追加し、`<Link onClick={onClick}>` へ直接渡す。default は抑止せず、履歴 URL は通常どおり Next のナビゲーションを使う。

```tsx
export function NavItem({
  item,
  active,
  label,
  sublabel,
  badge,
  onClick,
}: {
  item: NavItemData;
  active: boolean;
  label: string;
  sublabel: string;
  badge?: number;
  onClick?: () => void;
}) {
```

既存の `<Link>` 開始タグへ `onClick` だけを追加する:

```tsx
<Link
  href={item.href}
  onClick={onClick}
  aria-current={active ? "page" : undefined}
  className={cn(
    "group relative flex items-center gap-3 rounded-[11px] px-3.25 py-2.5 transition-colors",
    active ? "bg-surface text-ink shadow-(--shadow-sm)" : "text-ink-soft hover:bg-surface-2"
  )}
>
```

- [ ] **Step 4: 工坊項目の直下へ履歴を置く**

`frontend/src/widgets/app-shell/sidebar.tsx` で次を import する:

```ts
import { Suspense } from "react";

import {
  requestNewWorkshopConversation,
  WorkshopHistoryNav,
} from "@/features/workshop";
```

単一項目 renderer を次のグループ renderer に置き換える:

```tsx
const renderItem = (item: (typeof NAV)[number]) => (
  <div key={item.id} className="flex flex-col gap-1">
    <NavItem
      item={item}
      active={item.id === activeId}
      label={t(`nav.${item.id}`)}
      sublabel={t(`nav.${item.id}.sub`)}
      onClick={item.id === "workshop" ? requestNewWorkshopConversation : undefined}
    />
    {item.id === "workshop" && (
      <Suspense fallback={null}>
        <WorkshopHistoryNav />
      </Suspense>
    )}
  </div>
);
```

KB switcher を表示したままナビゲーション列だけをスクロール可能にする:

```tsx
<nav className="flex min-h-0 flex-1 flex-col gap-0.75 overflow-y-auto">
  {NAV.map(renderItem)}
</nav>
```

- [ ] **Step 5: 全 locale の文言を追加する**

`frontend/src/shared/i18n/dictionaries.ts` の各言語ブロックへ次のキーを追加する:

```ts
// zh
"workshop.history": "历史对话",
"workshop.historyMore": "加载更多",
"workshop.historyCollapse": "收起来",
"workshop.historyError": "历史记录加载失败",

// en
"workshop.history": "History",
"workshop.historyMore": "Load more",
"workshop.historyCollapse": "Collapse",
"workshop.historyError": "Failed to load history",

// ja
"workshop.history": "対話履歴",
"workshop.historyMore": "さらに読み込む",
"workshop.historyCollapse": "折りたたむ",
"workshop.historyError": "履歴を読み込めませんでした",
```

- [ ] **Step 6: フロントエンドテストと lint を実行する**

実行:

```bash
bun test --cwd frontend src/features/workshop/model/history.test.ts
bun run lint
```

期待結果: Bun の 2 テストが通り、ESLint エラーがない。

- [ ] **Step 7: サイドバー層をコミットする**

```bash
git add frontend/src/features/workshop/ui/workshop-history-nav.tsx frontend/src/features/workshop/index.ts frontend/src/widgets/app-shell/nav-item.tsx frontend/src/widgets/app-shell/sidebar.tsx frontend/src/shared/i18n/dictionaries.ts
git commit -m "feat(workshop): show paginated history in sidebar"
```

## Task 5: 工坊で対話を復元・継続・保存する

**Files:**
- Modify: `frontend/src/features/workshop/ui/workshop-view.tsx`
- Modify: `frontend/src/app/(app)/workshop/page.tsx`
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

- [ ] **Step 1: query 駆動の対話 ID と import を追加する**

`workshop-view.tsx` に次を追加する:

```ts
import { useRouter, useSearchParams } from "next/navigation";
import {
  getWorkshopConversation,
  saveWorkshopConversation,
  type WorkshopMessage,
} from "@/shared/api/tauri/client";
import {
  notifyWorkshopHistoryChanged,
  onNewWorkshopConversation,
  parseConversationId,
} from "../model/history";
```

`WorkshopView` の先頭に安定した URL/ID 状態を追加する:

```ts
const router = useRouter();
const searchParams = useSearchParams();
const requestedConversationId = parseConversationId(searchParams.get("conversation"));
const conversationIdRef = useRef<number | null>(requestedConversationId);
```

- [ ] **Step 2: reset をナビゲーション駆動の唯一の新規対話操作にする**

既存の `reset` 実装を次に置き換える:

```ts
function reset() {
  resetRef.current = true;
  conversationIdRef.current = null;
  void workshopCancel();
  setMessages([]);
  setSources([]);
  setInstruction("");
  setThinkingBuf("");
  setNarrationBuf("");
  setToolLog([]);
  setPhase("idle");
  setError(null);
}
```

すでに `/workshop` 上にいる場合も工坊本体のクリックで初期化されるよう、一度だけ subscribe する:

```ts
useEffect(() =>
  onNewWorkshopConversation(() => {
    reset();
    router.replace("/workshop");
  }),
[router]
);
```

exhaustive-deps が要求する場合は reset 本体を `useCallback` に抽出し、lint 規則は抑止しない。

- [ ] **Step 3: 選択された履歴対話を復元する**

`requestedConversationId` とアクティブ KB パスを key にする effect を追加する。状態を置き換える前に実行中生成を中断し、ID または KB 変更後に届いた古い読み込み結果は無視する。

```ts
useEffect(() => {
  if (requestedConversationId === null) {
    if (conversationIdRef.current !== null) reset();
    return;
  }

  let current = true;
  resetRef.current = true;
  void workshopCancel();
  setPhase("idle");
  setError(null);
  void getWorkshopConversation(requestedConversationId)
    .then((conversation) => {
      if (!current) return;
      conversationIdRef.current = conversation.id;
      setMessages(conversation.messages);
      setSources(
        conversation.sourceIds.map((path) => materialFromFile(path, t("workshop.addLocalFile")))
      );
      resetRef.current = false;
    })
    .catch((loadError) => {
      if (!current) return;
      reset();
      setError(loadError instanceof Error ? loadError.message : String(loadError));
    });

  return () => {
    current = false;
  };
}, [active?.path, requestedConversationId, t]);
```

`const { available, active } = useKbStore();` を使う。lint を満たし、関数 ID だけによる再実行を避けるため callback を安定化する。

- [ ] **Step 4: 完了した助手ターンだけを永続化する**

`runTurn` 内の成功時 `setMessages([...history, assistant])` ブロックを次に置き換える:

```ts
const completed: WorkshopMessage[] = [
  ...history,
  {
    role: "ai",
    text: reply || narration,
    thinking: thinking || undefined,
    tools: tools.length ? tools : undefined,
  },
];
setMessages(completed);

try {
  const saved = await saveWorkshopConversation({
    id: conversationIdRef.current,
    sourceIds: sources.map((source) => source.id),
    messages: completed,
  });
  if (!resetRef.current) {
    conversationIdRef.current = saved.id;
    router.replace(`/workshop?conversation=${saved.id}`);
    notifyWorkshopHistoryChanged();
  }
} catch (saveError) {
  if (!resetRef.current) {
    setError(saveError instanceof Error ? saveError.message : String(saveError));
  }
}
setPhase("idle");
```

既存の agent 失敗/中断分岐は変更しない。未完了のユーザーターンを除去するため、履歴レコードは作成されない。永続化だけが失敗した場合も、受信済み AI 応答は消さない。

- [ ] **Step 5: 不要になった左上の新規対話ボタンを削除する**

条件付き header 呼び出しを次に変更する:

```tsx
<ProcessTopBar t={t} />
```

`ProcessTopBar` は文脈タイトルと Ollama badge を残し、`onReset`、outline button、左矢印 icon を削除する:

```tsx
function ProcessTopBar({ t }: { t: (key: string) => string }) {
  return (
    <div className="flex flex-none items-center gap-3.5 border-b border-line pb-4">
      <div className="min-w-0 flex-1">
        <div className="font-mono text-[11px] font-semibold tracking-[0.14em] text-ink-muted uppercase">
          {t("workshop.processCrumb")}
        </div>
        <h1 className="mt-0.75 truncate font-serif text-[21px] font-medium text-ink">
          {t("workshop.processTitle")}
        </h1>
      </div>
      <Tag tone="ai" className="flex-none">
        <Icon name="spark" size={12} /> AI {t("workshop.assist")} · Ollama
      </Tag>
    </div>
  );
}
```

不要になった `workshop.newChat` キーを 3 言語の dictionary ブロックから同じコミットで削除する。

- [ ] **Step 6: static export 用 Suspense 境界を追加する**

`frontend/src/app/(app)/workshop/page.tsx` を次に置き換える:

```tsx
import { Suspense } from "react";

import { WorkshopView } from "@/features/workshop";

export default function WorkshopPage() {
  return (
    <Suspense fallback={null}>
      <WorkshopView />
    </Suspense>
  );
}
```

- [ ] **Step 7: フロントエンドの全検証を実行する**

実行:

```bash
bun test --cwd frontend src/features/workshop/model/history.test.ts
bun run lint
bun run --cwd frontend build
```

期待結果: Bun テストが通り、lint エラーがなく、Next static export が完了する。Turbopack が sandbox の helper process または local port 制限だけで失敗した場合は、コード不具合と判断する前に同じ build を必要な権限で再実行する。

- [ ] **Step 8: 対話ライフサイクル層をコミットする**

```bash
git add frontend/src/features/workshop/ui/workshop-view.tsx frontend/src/app/'(app)'/workshop/page.tsx frontend/src/shared/i18n/dictionaries.ts
git commit -m "feat(workshop): restore and continue saved conversations"
```

## Task 6: 全回帰検証とデスクトップ受入確認

**Files:**
- 原則は検証のみとし、この機能が直接起こした不具合が判明した場合だけコードを変更する。

- [ ] **Step 1: すべての自動品質ゲートを実行する**

リポジトリ root から実行:

```bash
bun test --cwd frontend src/features/workshop/model/history.test.ts
bun run test
bun run lint
bun run --cwd frontend build
git diff --check
```

期待結果:

- フロントエンド履歴テストが通る。
- 既存 agent/tool/KB 回帰を含む全 Rust テストが通る。
- ESLint エラーがない。
- Next static export が完了する。
- `git diff --check` が何も出力しない。

- [ ] **Step 2: 成功条件に沿ってデスクトップフローを確認する**

実行:

```bash
bun run dev
```

Tauri ウィンドウで手動確認する:

1. 工坊で prompt を送信し、AI 応答成功後に再読み込みなしで工坊直下へ履歴が 1 件現れる。
2. 同じ対話を継続し、重複作成されず 1 件のまま先頭へ移動する。
3. 別の履歴項目を開き、メッセージ、思考 panel、tool card、添付素材 chip が復元される。
4. 復元した対話を継続し、復元済み chat turn と新規 turn が Ollama へ渡る。
5. 履歴表示中に工坊本体をクリックすると空の新規対話が開く。
6. 新規対話で発話した後、`/workshop` のまま工坊本体を再クリックしても初期化される。
7. アプリから 21 件の対話を作成し、「さらに読み込む」で 21 件目が先頭ページの下へ追加され、「折りたたむ」で正確に 20 件へ戻る。
8. KB を切り替えても別 KB の履歴が混ざらない。
9. 生成中断と生成失敗のどちらでも履歴が作成・更新されない。
10. 旧左上の新規対話ボタンが存在しない。

- [ ] **Step 3: 最終スコープを確認する**

実行:

```bash
git status --short
git diff --stat HEAD~5..HEAD
```

期待結果: この計画で列挙したファイルだけが変更され、生成 frontend output、SQLite ファイル、無関係な整形差分が追跡されていない。

- [ ] **Step 4: 検証で必要になった修正だけを記録する**

Step 1 または Step 2 で狭い修正が必要になった場合、その修正だけをコミットする:

```bash
git add -p
git commit -m "fix(workshop): correct conversation history regression"
```

修正が不要なら空コミットは作らない。

## セルフレビュー

- 要件網羅: 履歴を工坊直下へ配置し、追加式ページング、先頭ページへの折りたたみ、クリック復元に対応する。工坊本体は新規対話操作のまま維持し、旧画面内ボタンは削除する。
- 境界網羅: 永続履歴は KB 単位で派生インデックスから分離する。Tauri コマンドは薄く保ち、widget は Workshop feature の公開 API を利用し、transport 型を一元化する。
- 失敗網羅: 中断/失敗ターンは保存せず、永続化失敗で完了済み応答を消さない。古い KB/ID 読み込みを無視し、同一 URL の工坊クリックでも初期化する。
- placeholder 確認: 実装手順に TBD/TODO、未指定の validation、「同様に」の参照を残していない。
- 型整合: Rust `WorkshopMessage` は TypeScript `WorkshopMessage` と同じ `user | ai` と camelCase 形状へ serialize する。対話 ID は SQLite signed integer と安全な JavaScript number で統一し、ページングは全経路で offset + 固定 20 件を使う。
