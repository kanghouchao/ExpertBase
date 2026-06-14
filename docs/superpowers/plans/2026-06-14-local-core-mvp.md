# ローカルコア MVP 実装計画

日付: 2026-06-14
対象仕様: `docs/superpowers/specs/2026-06-13-local-core-mvp-design.md`

## 目的

L1 ローカルコア MVP を、Tauri デスクトップアプリ内で完結する機能として実装する。

- Markdown ファイルをナレッジベースの真実のソースにする。
- SQLite/FTS5 は削除・再構築可能な派生インデックスとして扱う。
- Capture は AI を使わず、ローカル素材を `inbox/*.md` に正規化する。
- Workshop だけが AI を使い、`AiProvider` trait の背後にプロバイダ実装を隠す。
- フロントエンドは `frontend/src/lib/tauri/client.ts` の typed client だけを通じて Rust コマンドへアクセスする。

## アーキテクチャ境界

- UI はファイルシステム、SQLite、API キーへ直接触れない。
- Rust 側が KB ルート、Markdown I/O、SQLite、Capture、AI 呼び出しを所有する。
- `entries/*.md` と `inbox/*.md` は許可された相対 Markdown パスだけを受け付ける。絶対パス、`..`、未知のネストは拒否する。
- `[[タイトル]]` リンクの曖昧さを避けるため、MVP では entry title を KB 内で一意にする。
- 長時間処理（Web 取り込み、文書抽出、AI draft）は UI 側で pending/error 状態を持つ。Rust 側では Tauri の async command または blocking 処理の隔離を使い、UI の応答性を前提にする。

## 実装済み範囲

### Phase 1: KB データ層

- `src-tauri/src/kb/entry.rs`
  - frontmatter 付き Markdown の parse/serialize
  - `[[リンク]]` 抽出
  - 語数カウント
- `src-tauri/src/kb/store.rs`
  - `entries/*.md` の作成・読み込み
  - `inbox/*.md` 素材の parse/serialize
- `src-tauri/src/kb/index.rs`
  - SQLite schema
  - `entries`, `links`, `inbox`, `entries_fts`
  - FTS5 trigram tokenizer
  - title 一意性
  - backlinks, orphans, search, stats, graph
- `src-tauri/src/kb.rs`
  - KB registry/config
  - active KB
  - index rebuild
  - list/search/backlinks/stats/graph/orphans/read/save/list-inbox commands
  - KB 内相対パス検証

検証:

- `cargo test --manifest-path src-tauri/Cargo.toml kb`
- CJK trigram 検索、英語検索、空クエリ、重複 title 拒否、パス逃逸拒否をテストする。

### Phase 2: Capture

- `src-tauri/src/capture.rs`
  - text/file/web capture command
  - media attachment copy
  - `inbox/*.md` への正規化
- `src-tauri/src/capture/web.rs`
  - readability 抽出
  - Markdown 化
- `src-tauri/src/capture/doc.rs`
  - PDF/Word 抽出境界
  - 不正入力で安全に失敗するテスト

検証:

- `cargo test --manifest-path src-tauri/Cargo.toml capture`
- Capture は AI を呼ばない。

### Phase 3: AI Provider

- `src-tauri/src/ai.rs`
  - `AiProvider` trait
  - `StructureRequest`
  - `StructureResult`
  - `AiError`
  - `FakeProvider`
  - BYO key command
- `src-tauri/src/ai/claude.rs`
  - Anthropic Messages API request builder
  - structured JSON response parser
  - live network なしの unit test

実装時の確認事項:

- Anthropic の model id と Messages API 形式は、実装時点の Anthropic 公式ドキュメントで確認する。
- ローカル skill 名に依存しない。利用可能な OpenAI/Codex skill がある場合でも、最終的な API 形状は公式ドキュメントで確認する。

検証:

- `cargo test --manifest-path src-tauri/Cargo.toml ai`
- live API key を必要とするテストは unit test に含めない。

### Phase 4: Workshop

- `src-tauri/src/workshop.rs`
  - FTS による関連 entry 検索
  - `AiProvider` への draft request 組み立て
  - draft confirm
  - `entries/*.md` 書き出し
  - inbox status の `processed` 更新
  - `inbox/*.md` 相対パス検証

検証:

- `cargo test --manifest-path src-tauri/Cargo.toml workshop`
- fake provider で draft と confirm を検証する。

### Phase 5: フロントエンド接続

- `frontend/src/lib/tauri/client.ts`
  - 新規 Tauri command wrapper
  - 型付き response
- Capture / Dashboard / Wiki / Workshop / Graph
  - mock-only 状態から real data flow へ接続
  - データが無い場合は既存の empty state を維持

検証:

- `bun run lint`
- `bun run build`
- Tauri 手動確認: capture text -> workshop draft -> confirm -> Wiki/Graph/Search/Dashboard に反映

## 受け入れ条件

- `bun run test` が通る。
- `bun run lint` が通る。
- `bun run build` が通る。
- `entries/*.md` / `inbox/*.md` 以外の IPC 相対パスは拒否される。
- 同じ title の entry は別ファイルとして登録できない。
- 日本語 3 文字以上の検索語が FTS5 trigram で命中する。
- Capture は AI を呼ばない。
- Workshop の AI 処理は `AiProvider` から差し替え可能で、unit test は fake provider で完結する。

## MVP 外

- Whisper 文字起こし
- OCR
- マルチモーダル LLM
- ベクトル検索
- 複数素材の自動マージ
- Publish
- Bots
- プラグイン市場

## 既知の制約

- FTS5 trigram は 3 文字未満の CJK 検索に向かない。MVP では 3 文字以上を検索単位にする。
- AI draft は非ストリーミングで開始する。ストリーミング UI は後続タスクで扱う。
- native file dialog と AI key settings UI は、コマンド実装とは別に UI 接続を完了させる必要がある。
