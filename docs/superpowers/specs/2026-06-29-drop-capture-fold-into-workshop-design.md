# 収集箱の廃止・工坊への畳み込み 設計

**日付**: 2026-06-29
**ブランチ**: 実装時に `feat/drop-capture` を切る（現状 main、spec はレビュー用に未コミット）
**前提**: 工坊の Rig agent（read_source / search_kb / write_entry、外部ファイル「+」添付）実装済み（#8 までにマージ）

## 目標

**収集箱（capture / inbox）を廃止**し、その生きた能力を工坊へ畳み込む。フローを「素材を会話に持ち込んで**即処理**」へ一本化する（先に inbox へ溜めてから処理する GTD 式 → 即時処理）。重点は採集→整理→公開の流れであり、収集箱という前置の仕分け場ではない。

ユーザー要求（提出順）：
1. インポートは工坊の対話框「+」で行う。
2. 网页抓取は AI が呼べる `fetch_web` ツールにする。
3. 手書き / 粘贴は対話框へ直接入力する。
4. 録音・动画転写は今回の重点ではない → **缓**（後で工坊へ搬入）。
5. 収集箱を削除し、AI の能力で置き換える。

## 核心的な洞察（実現可能性の裏付け、コード確認済み）

- **「文書/PDF を追加→解析→AI へ」は既に通っている。** `read_source`（`tools.rs:181-191`）は外部ファイルを拡張子で分流し、pdf→`extract_pdf`・docx→`extract_docx`・その他→テキスト読みで返す。**コピーせず・KB へ落とさず**読む。
- 工坊の「+」（`workshop-view.tsx:213 addLocalFile`）は既に外部ファイルを**絶対パスの source** として追加し、AI が `read_source` で読む。つまり収集箱は AI の前で「先に inbox へ一部複製しておく」前置の中間人。
- **文書解析に臨時目录は不要。** `doc.rs` の `pdf_extract` / `dotext` はパスを直接読み内存で抽出、中間ファイルを書かない。臨時目录は媒体転码（音视频→WAV の ASR）でのみ要る → 缓と一緒に後送り。
- **网页抓取スタックは既存。** `capture/infrastructure/web.rs` の `fetch_html` + `extract_readable`（Readability）を `fetch_web` ツールで再利用するだけ。新規依存なし。

## 設計判断

### 1. 解析は `read_source` に折叠したまま（別 tool 化しない）

`read_source(id) → 正文` が正しい粒度。AI は id を渡せば本文を得て、ファイル型を自分で判断する必要がない。`parse_file` を別出しすると手数が増えるだけで利得なし（YAGNI）。

### 2. 出処（provenance）：コピーしない・`attachments/` 目录は作らない、ただし引用文字列は残す

- 被引用ファイルを KB へ複製しない（ユーザー決定）。よって `attachments/`・`sources/` 目录は不要。
- ただし `entry.sources` には**引用文字列（外部パス / URL）を残す**（無料、文字列 1 行）。リンクは原ファイルの移動/削除で烂れ得るが、知識庫は「これは `/…/report.pdf` 由来」を知っておくべき。
- 帰結：`confirm` から inbox→processed の簿記を撤去し、添付された source id をそのまま `sources` に書く形へ**簡素化**する。

### 3. 臨時目录は今回作らない（YAGNI）

文書解析は内存完結。媒体転码が要る ASR を缓するので、臨時目录もその時に設計する。

## アーキテクチャ：削 / 加 / 缓 / 保留

### 削（前端）

- `features/capture/` 全片（`ui/capture-view.tsx`・`ui/material-row.tsx`・`lib/recorder.ts`・`index.ts`）
- `app/(app)/capture/` 路由
- `shared/config/nav.ts` の `"capture"` 项（id / href / 型ユニオン）
- `shared/api/tauri/client.ts`：`captureText` / `captureFile` / `captureWeb` / `captureAudio` / `transcribeMaterial` / `listInbox` / `readInboxMaterial` / `deleteInboxMaterial` / `InboxItem` / `DownloadProgress`
- capture 系 i18n キー（`capture.*` / `tabs.*` のうち capture 専用 / `empty.materials*`）
- **工坊から inbox 依存を撤去**：`inbox` / `pending` / `pool` の選択池ドロワー、`listInbox` / `deleteInboxMaterial` 利用。「+」外部ファイル添付 + 対話框入力だけ残す（素材チップ・selection 状態自体は残す）。

### 削（後端）

- `capture` の application/interface/domain：`ingest_text` / `ingest_file` / `ingest_web` / `ingest_audio_bytes` / `write_material` / `write_audio_material` / `copy_attachment` と、`capture_text` / `capture_file` / `capture_web` / `capture_audio` コマンド
- `kb::infrastructure::index`：`InboxItem` / `upsert_inbox` / `list_inbox` / `set_inbox_status` / `delete_inbox`
- `kb::interface`：`kb_read_inbox_material` / `kb_delete_inbox_material` / `kb_list_inbox`
- `kb::application`：`read_inbox_material` / `delete_inbox_material`（+ 対応テスト）
- `kb::infrastructure::store` の rebuild：inbox スキャン枝
- `lib.rs`：上記コマンドの `generate_handler!` 登録
- `read_source`（`tools.rs:169-180`）の `inbox/` 内部素材枝、`workshop::interface`（`interface.rs:79-86`）の inbox 検証枝 → source は外部絶対パスのみに

### 加

- `workshop/infrastructure/tools.rs` に `FetchWeb` ツール：URL を受け `web::fetch_html` + `web::extract_readable` で本文 Markdown を返す（~40 行）。`rig_agent` の工具集へ登録。`capture` mod から `fetch_html` / `extract_readable` を re-export。
- system プロンプトに一文：ユーザーが渡した URL は `fetch_web` で本文を読める旨。

### 缓（削らず残す、後で工坊へ搬入）

- `asr` モジュール（whisper）。録音 UI を消すと呼び出し方が消える → `transcribe_material` は孤立。**asr モジュールのコードは残置**し、`lib.rs` の `transcribe_material` 登録だけ外す（inbox パス前提の契約が死ぬため、誤解を招く dead コマンドを公開しない）。録音 UI / 臨時目录 / 工坊内録音は別 spec で。

### 保留（再利用）

- `capture/infrastructure/doc.rs`（`extract_pdf` / `extract_docx`）→ `read_source` が利用。
- `capture/infrastructure/web.rs`（`fetch_html` / `extract_readable`）→ `fetch_web` が利用。
- → `capture` モジュールは「抽出器の置き場（doc + web）」へ縮退して残る。名前と実体がズレるが、改名（例 `extract`）は import 波及があるため**今回はしない**（後日検討、コメントで明示）。

## 契約 / IPC 変更

- `workshop_chat` の `source_ids`：現在「inbox 相対 | 絶対パス」混在 → **絶対パス（外部ファイル）のみ**。`interface.rs:80` の分岐から inbox 検証枝を削除。`ChatEvent` 五相は不変。
- `confirm(root, conn, title, cat, body, source_refs)`：`inbox_rels` → `source_refs`（記録のみ、processed マークなし）。
- 削除コマンド：capture 系 + inbox 系（上記）。

## 境界（YAGNI）

- 録音 / 动画 / ASR / 臨時目录：今回やらない（缓）。
- web 抓取の多階層クロール（depth 2/3）・readability トグル：`fetch_web` は**単一 URL の本文抽出のみ**。元 UI のオプションは持ち込まない。
- `fetch_web` のドメイン許可リスト / SSRF 防御：local-first・単一ユーザーのデスクトップ前提で**今回は入れない**。ユーザーが会話に明示的に渡した URL を取る建付け。多テナント化や自動巡回を入れるときに再評価（既知の限界として明記）。
- 出処リンクの健全性チェック（原ファイル存在確認）はしない。
- 跨ワークスペース・URL 以外の遠隔取り込みはしない。

## テスト / 検証

- **fetch_web 単体**：空 URL / 取得失敗がモデル向け文字列で返ること、ローカル HTML 文字列 → `extract_readable` で本文化されること（実ネットは叩かない）。
- **confirm**：`source_refs` が `entry.sources` に文字列として記録されること（既存テストを inbox→外部参照へ更新、processed アサート削除）。
- **read_source**：外部ファイル読みが残ること、`inbox/` id が `unknown source id`（or 廃止）になること。
- inbox 撤去で壊れる `kb` テスト（index / application / store）の削除・更新。
- `bun run test`（cargo test）/ `bun run lint` / `bun run build`。

## 仮定 / 要検証

- dashboard / 他画面が inbox の pending 件数を表示していないか（`features/dashboard` を確認、表示してたら撤去対象に追加）。
- `capture.*` i18n キーが capture 以外から参照されていないか。
- `recorder.ts` / 録音削除で `asr` が dead code 警告を出す可能性 → 残置方針（`#[allow(dead_code)]` or 登録だけ外して module 全体は temporarily 未使用許容）。実装時に最小の対処を選ぶ。
- `read_source` の `sources` 許可集合が外部絶対パスのみになることで、既存の inbox 関連テスト（`read_source_reads_inbox_material_body`）は削除/書換。
