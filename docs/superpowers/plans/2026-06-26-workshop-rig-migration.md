# ワークショップ・Rig フレームワーク移行 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ワークショップの手書き agent ループを Rig（rig-core 0.39）の Agent/Tool/streaming へ置き換え、その上で素材を AI ツール読み取り（`read_source`）へ改め、外部素材（ローカルファイル）に対応する。

**Architecture:** Rig の `ollama::Client.agent(model).preamble().tools().additional_params(num_ctx/think).build()` → `stream_chat().multi_turn().await` の `MultiTurnStreamItem` を `StreamProgress` へ写像し、mpsc 経由で interface→Channel へ流す。KB ツールは Rig `Tool` impl（`workshop/infrastructure`）で async→spawn_blocking→自前接続。停止は共有 `AtomicBool`。

**Tech Stack:** Rust / Tauri 2、rig-core 0.39（ollama provider）、tokio（rt-multi-thread・macros・sync）、futures、reqwest、rusqlite、serde、Next.js/React/Bun。

設計根拠：`docs/superpowers/specs/2026-06-26-workshop-rig-migration-design.md`。

## Global Constraints

- 注釈 / コード文書は日本語（AGENTS.md）。Tauri コマンドは `Result<T, String>`。IPC struct/enum は `Serialize` + `#[serde(rename_all = "camelCase")]`。
- DDD：Rig フレームワーク依存は `workshop/infrastructure` に閉じる。application は `StreamProgress`/`ChatTurn`/`AiError` のみ扱う。`ollama.rs` はモデル発見だけ残す。
- TDD：ツールは `#[tokio::test]` で `.call()` を直テスト。Rust は 2 スペースインデント、`cargo fmt` を全ファイルにかけない。
- 検証：`bun run test`（= `cargo test`）、`bun run lint` + `bun run build`（フロントエンド変更時）。
- `workshop_chat` の IPC 契約は Phase 1 では不変（フロントエンド変更なし）。

---

### Phase 0: Spike 検証（先にツマミを確かめる） — ✅ 完了

**Files:** `Cargo.toml`（rig-core 追加）

ソースコードレベル + 実機で Rig×Ollama を de-risk。使い捨ての spike バイナリは作らず、ソース精査で確定（より確実）。

- [x] `cargo add rig-core`（0.39）→ 既存依存と共存コンパイル（`cargo check` 31s）
- [x] `additional_params` が num_ctx→options・think→トップレベルへ仕分けられること（rig ollama provider のソースで確認）
- [x] ストリーム item（Text/Reasoning/ToolCall/ToolResult/FinalResponse）が `StreamProgress` 五相へ写像可能
- [x] ローカル gemma4:26b-a4b-it-qat が `tools` + `thinking` 能力を持つ（`/api/show`）

**結果**：rustc 1.96（宣言 1.77.2 でローカルは OK）。リスク要点はすべてクリア。

### Phase 1: Rig 移行（挙動不変） — ✅ 完了（コミット `ed5f9b7`）

**Files:**
- New: `src-tauri/src/workshop/infrastructure/{mod.rs, tools.rs, rig_agent.rs}`
- Modify: `ai/{agent.rs, domain.rs, mod.rs, interface.rs}`、`ai/infrastructure/ollama.rs`、`workshop/{application.rs, interface.rs, mod.rs}`、`Cargo.toml`

- [x] tokio / futures 依存追加
- [x] `tools.rs`：`SearchKb` / `WriteEntry` を Rig `Tool` で実装（緩い Args、`call` 内 spawn_blocking + 自前接続、`confirm` 再利用）
- [x] `rig_agent.rs`：Ollama エージェント駆動 + `MultiTurnStreamItem`→`StreamProgress` 写像 + mpsc + 停止
- [x] `ollama.rs` 分割：モデル発見（`available`/`list_models`/`show_*`）を自由関数で残し、agent 側は削除
- [x] `domain.rs` 痩身：`ChatTurn`/`StreamProgress`/`AiError` のみ（`AiProvider`/`AgentMsg`/`ToolCall`/`ToolDef`/`TurnOutcome`/`FakeProvider` 削除）
- [x] `agent.rs`：preamble を英語化 + 言語方針 + 素材の性格づけ。`agent_tools` 削除
- [x] `application::chat` を async 痩身版（→ `rig_agent::run`）、`interface.rs` を mpsc + spawn + 排出へ
- [x] 旧テスト（ScriptedAgent / consume_agent_stream / render_tools / agent_tools 等）削除、ツール直テスト追加

**検証（実行記録）**：
- `bun run test` → **65 passed / 0 failed**
- 実機 gemma4：① ストリーム narration + LoadingModel ✓；② think=true → Thinking ストリーム ✓；③ Rig が `write_entry` を派遣 → `entries/…md` を実際に書き出す（entries_written=1）+ ToolCall/ToolResult 報告 ✓
- コミット `ed5f9b7`（以前のセッションの草稿削除 + 本移行を一括、同一バッチのファイルで纏わるため分割不可）

### Phase 2: 素材を `read_source` ツールへ + 外部ファイル — ✅ 完了

**Files:** `workshop/infrastructure/{tools.rs, rig_agent.rs}`、`ai/agent.rs`、`workshop/{application.rs, interface.rs}`、`capture/mod.rs`（ファイル抽出関数を公開面へ）

- [x] `ReadSource{root, sources}` Rig `Tool`：id の接頭辞で振り分け——`inbox/...`→`parse_material`；絶対パス→text/`extract_pdf`/`extract_docx`。`sources` は許可集合＝モデルが任意パスを読むのを防ぐ。外部ファイルは読み取りのみ・KB へ落とさない
- [x] `agent.rs`：preamble を「`read_source(id)` で素材を読む」へ。system は id の **目録**（本文なし、タイトル＝ファイル名の id 自体）、素材なしは節ごと省略
- [x] `rig_agent.rs`：`read_source` 登録、system に目録を組む（プリインジェクト撤去）
- [x] `with_tools=false` 純チャット分岐を撤去。tools は常に登録（`tools` IPC 引数は前端互換のため受けるが分岐しない＝Phase 3 で前端が必須化）
- [x] `capture/mod.rs`：`extract_pdf`/`extract_docx` を `pub use` で workshop へ公開
- [x] TDD：`read_source` の inbox / 外部ファイル / 不正 id の直テスト 3 本

**検証（実行記録）**：
- 微调：`application::chat` は `sources: Vec<String>` 1 本受け、inbox_rels（write 時に processed にする集合）を `starts_with("inbox/")` で内部から導出＝外部ファイルは KB に落とさない。
- `bun run test` → **69 passed / 0 failed**（read_source 3 本 + agent 目録 2 本を追加、`cargo check --tests` 警告なし）

### Phase 3: フロントエンド +ボタンにファイルを添付 + i18n + 既定モデル — ✅ 完了

**Files:** `workshop-process-view.tsx`、`shared/api/tauri/client.ts`、`shared/i18n/dictionaries.ts`、`workshop/interface.rs`（IPC 引数リネーム + 混在 id 検証）

- [x] +ボタンメニュー：常に開けるようにし「ローカルファイル追加」を先頭に追加（`pickLocalFile`→絶対パス）+ 既存の inbox プール。`sources` の id を inbox 相対 | ファイル絶対 に一般化
- [x] `client.ts`：`workshopChat` の `inboxPaths`→`sourceIds`、ダイアログを `pickLocalFile()`（typed client の裏）に閉じる。後端 `workshop_chat` も `inbox_paths`→`source_ids` + 絶対パスは外部ファイルとして許可・それ以外は inbox 検証
- [x] `dictionaries.ts`：`workshop.addLocalFile` / `workshop.toolsRequired` を zh/en/ja 3 言語へ追加（キー一致）
- [x] tools 必須化：`canGenerate` に `selectedTools` を加え、非 tools モデルでは送信不可 + `toolsRequired` 提示
- [x] 既定モデルの nudge：Qwen3 → 任意の tools 対応 → 先頭、の順で既定選択
- [x] `bun run lint` + `bun run build`

**検証（実行記録）**：
- `bun run lint` → 通過（エラーなし）
- `bun run build` → 通過（前端静的書き出し + Tauri release 同梱、`.app` / `.dmg` 生成）
- 旧シンボル残留なし（grep：`inbox_paths` / `with_tools` / `source_text` / `inboxPaths` ＝ 0 件）

## エンドツーエンド検証（全フェーズ通過後）

1. `bun run test`：Rust 全緑——ツール直テスト、read_source の二系統（inbox / ローカルファイル）、write_entry 書き込み + inbox processed、ChatEvent シリアライズ、モデル発見。
2. `bun run lint` + `bun run build`：通過、旧シンボルの残留なし（grep）。
3. 実機 `bun run dev`（tools モデル、推奨 Qwen3 8B）：通常の質問→思考 + ストリーム；翻訳 / 改稿→`read_source` で読んでから返信（要約に縮まない）；「条目として保存」→`write_entry` が active KB へ落とす；+ボタンでローカル PDF→`read_source` 抽出成功・外部ファイルは KB へ落とさない；非 tools モデル→送信禁止 + 提示；停止即中断。
