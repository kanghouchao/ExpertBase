# ワークショップ・Rig フレームワーク移行 設計

**日付**: 2026-06-26
**ブランチ**: feat/workshop-redesign-kb-delete
**前提**: 草稿 / 審査の削除 + KB 書き込みを agent ツール化（実装済み、同ブランチ）；ストリーム叙述 + 停止ボタン（実装済み）

## 目標

ワークショップの**手書き agent ループ**（`AiProvider::agent_turn` + `application::chat` の反復 + `ollama.rs` の wire 変換 / ストリーム / 停止）を **Rig フレームワーク**（rig-core 0.39）の Agent / Tool / streaming へ置き換える。その後、Rig の上で「素材を AI ツール読み取りへ」+「外部素材」を実装する。

ユーザー要求（提出順）：
1. Rig を導入し、その Agent/Tool/streaming で手書きループを置き換える。
2. 素材を **AI が自分でツールで読む**（`read_source`）形に改め、「AI が読んだ内容」と「我々のプロンプト」を構造的に分離する。
3. **+ボタンで外部素材を添付できる**——ローカル任意ファイル、inbox だけではない（URL は対象外）。
4. プロンプトは全て英語、ただし**ユーザーの言語で返信する**。

## 核心的な洞察（2026 調査、実現可能性の裏付け）

- rig-core 0.39 は Ollama provider をネイティブにサポート；`Tool` trait（`NAME`/`Args`/`Output`/`Error` + `definition` + `call`）は既存の `ToolDef` + `exec_kb_tool` とほぼ一対一に対応。
- **Ollama のツマミは無損失**：rig の ollama provider（`OllamaCompletionRequest::try_from`）が `additional_params` 内の `think` をリクエストのトップレベルへ、その他（`num_ctx` 含む）を `options` へ merge する——まさに Ollama が期待する形式。ソースレベルで確認、実行時の推測に頼らない。
- **ストリームの区別がきれい**：`stream_chat().multi_turn().await` が `MultiTurnStreamItem` を産出。`StreamedAssistantContent`（Text / Reasoning / ToolCall）+ `StreamedUserContent::ToolResult` + `FinalResponse` を含み、既存の `StreamProgress` 五相と一対一に写像できる。
- Ollama の「ストリーム + ツール」は 2026 で修正済み（モデルテンプレートに沿って、流しながらツール呼び出しと本文を分離）；ツールを付けてもストリームを失わない。
- 抽出スタックとファイル選択ダイアログは**既に依存**：`capture/infrastructure/doc.rs`（`extract_pdf` / `extract_docx`）、`tauri-plugin-dialog`。Phase 2 の外部ファイルは基本的に再利用であり、新規依存ではない。

## アーキテクチャ

### 1. Rig が置き換えるもの・残すもの

**置き換え（削除）**：`OllamaProvider` の agent パス、`build_agent_body` / `render_tools` / `render_agent_msg` / `consume_agent_stream` / `post_agent` / `send_chat`、`agent_turn` impl、`application::chat` の手書きループ + `exec_kb_tool`、`AtomicBool` ポーリング、domain の `AiProvider` / `AgentMsg` / `ToolCall` / `ToolDef` / `TurnOutcome` / `FakeProvider`。

**残す**：`ollama.rs` のモデル発見（`available` / `list_models` + `/api/show` 能力探索）——Rig は `/api/tags`・`/api/show` を提供しない；`StreamProgress` / `ChatEvent`（UI 契約、Rig ストリーム → フロントエンドの橋）；`ChatTurn`（IPC DTO）；`confirm()`（条目書き込みの実体、`write_entry` が再利用）；kb の `index` / `store`。

### 2. DDD の落とし所（フレームワーク汚染をユースケース feature に閉じる）

KB ツール（search_kb / write_entry）は本質的に workshop の KB ユースケースなので、Rig の `Tool` impl + agent runner を **`workshop/infrastructure/`** に置く（計画では当初 `ai/infrastructure` と書いたが、実行時にここへ移す方が DDD として素直）。`ai` は純粋な「Ollama モデル発見 + 将来の Client ファクトリ」へ退く。

- `workshop/infrastructure/tools.rs`：`SearchKb{root}` / `WriteEntry{root, inbox_rels}` impl Rig `Tool`。
- `workshop/infrastructure/rig_agent.rs`：Ollama エージェントの組み立て + ストリーム駆動 + `StreamProgress` 写像。

### 3. async → ブロッキング sqlite の橋

Rig の `Tool::call` は async だが sqlite / FS はブロッキング。各ツールの `call` 内で `tokio::task::spawn_blocking` し、その中で **root から索引を開き直す**（`rusqlite::Connection` は `Sync` ではないので共有せず、毎回開く）。索引はファイルなので別接続でも一貫する。

### 4. スレッドモデル + 進捗 + 中断

- 手書き経路は `spawn_blocking(全てブロッキング)`。Rig は async（reqwest async + tokio）。
- 進捗コールバック `&mut dyn FnMut` は非 Send で spawn できないので、**mpsc**（`tokio::sync::mpsc`）に置換：runner が `StreamProgress` を tx へ送り、interface が rx を Channel へ排出する。runner future は Send なので `tauri::async_runtime::spawn` 可能。
- **中断**：共有 `AtomicBool`（停止ボタン）を各チャンク前に確認、立っていれば return → stream を drop → reqwest 接続が切れて Ollama 側も停止する。`workshop_cancel` は従来どおりフラグを立てるだけ。

### 5. system プロンプト（英語化）

`AGENT_SYSTEM` を英語に：役割 + ツール説明 + `Always reply in the same language as the user's latest message.` + 素材を「参考資料」と性格づける一文（勝手に概括・改稿しない）。Phase 1 では素材は従来どおり `agent_system_with` で pin（プリインジェクト）。

## 確定した決定（覆し可能）

1. **DDD のポートは残さず**、application が infra の `rig_agent::run` を直接呼ぶ（Rig 型は infra に閉じ、application は `StreamProgress`/`ChatTurn`/`AiError` のみ扱う）。テスト容易性はツール直テスト（`#[tokio::test]` で `.call()` を直接叩く）で担保する。
2. **素材は `read_source` ツールで読む（Phase 2）**。帰結：ワークショップは **tools 対応モデル必須**、`with_tools=false` 純チャット分岐は撤去。
3. **ワークスペース = 保存境界、入力境界ではない**：`read_source` は inbox（内部）/ ローカルファイル（外部）を読めるが、外部ファイルは**読み取りのみ・KB へ落とさない**；`write_entry` は常に active KB へ。URL は対象外、**跨ワークスペースも行わない**（YAGNI）。
4. 既定モデル推奨 **Qwen3 8B**（UI の既定 / 推奨のみ、実行時はユーザー選択）。

## 契約

`workshop_chat` の IPC 契約は**不変**（`inbox_paths` / `messages` / `model` / `think` / `tools` / 返り値 String / `ChatEvent` 五相）→ **フロントエンド変更なし**（Phase 1）。Phase 2 で素材を id + タイトルの目録に変える際、`inbox_paths` を `source_ids`（inbox 相対パス | ファイル絶対パス の混在）へ一般化する。

## 境界（YAGNI）

- Rig の embedding / vector-store RAG・複数クラウド provider・multi-agent は**今回入れない**（FTS5 検索のまま）。実需が出たとき再評価。
- 跨ワークスペースの素材取り込みはしない。外部ローカルファイルは読み取りのみで KB へ落とさない（一過性）。URL は今回対象外。
- LoadingModel フェーズは Rig が個別に出さない可能性 → ストリーム開始前に 1 回 emit して代替（小さな許容回退）。

## テスト / 検証

- **単体テスト**：ツール直テスト（search/write の `.call()`）、`confirm`、`ChatEvent` シリアライズ、モデル発見。`bun run test`（= `cargo test`）。
- **実機**（手動、`#[ignore]` 想定 or 一時テストで確認後に撤去）：実 Ollama に対し runner を回し、num_ctx/think の透過・ストリーム・ツール派遣・条目の書き込みを確認。
- フロントエンド：`bun run lint` + `bun run build`。

## 仮定 / 要検証

- additional_params の num_ctx/think 透過：**ソースレベルで確認済み**（runtime も Phase 1 で確認）。
- 中断 = stream drop で Ollama が止まる：標準的な async cancel、Phase 1 の実接続で確認。
- 弱いモデルがツール引数を別型で渡す問題：Args を緩い型（String + `#[serde(default)]`）で受けて中身を検証する。
- MSRV：rig-core 0.39 はローカル rustc 1.96 で問題なし（宣言 `rust-version` は必要なら引き上げ）。
