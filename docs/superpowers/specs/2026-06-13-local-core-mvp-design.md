# Expert Base ローカルコア MVP 設計

日付: 2026-06-13
ステータス: ドラフト（ユーザーレビュー待ち）

## 背景と位置づけ

デスクトップアーキテクチャ（[2026-06-12 の設計](2026-06-12-desktop-architecture-design.md)）の上に、最初の機能マイルストーンを載せる。

機能全体は 4 層に分かれる（L1 ローカルコア閉ループ / L2 知能・品質 / L3 対外発布 / L4 プラグイン）。
本 MVP は **L1 のみ**を対象とする。L1 は Obsidian モデルにおける「無料デスクトップ層」であり、
サーバー不要・単独で完結した価値を出せる。L3（発布・Bot）と L4（プラグイン市場）は別マイルストーン。

### 確定した AI 方針

調査セッションでユーザーが以下を確定した。

- **取り込み（Capture）は AI なし・完全ローカル。** 文字を抽出できる素材は抽出し、できないメディアは
  添付として保存するだけ。**ユーザーによる説明文も任意**（何も書かなくてよい）。
- **AI はワークショップ（Workshop）にのみ存在する。** 素材を加工する時、AI は
  「**既存のナレッジベース + 今回の新素材**」を踏まえて、構造化された条目と `[[リンク]]` 候補の作成を支援する。
  AI プロバイダは BYO-key（ユーザーが自分の API キーを入力）で、Rust の trait の裏に隠す。
- **ワークショップのメディア方針 = アプローチ A（テキスト LLM のみ）。** AI が構造化するのは
  「すでに文字を持つ素材」だけ。文字のない不透明メディア（音声・動画・画像で説明文も無いもの）は、
  ユーザーが手動で条目を書き、当該メディアを出典として参照する。原始メディアをクラウドへ送らない。

## スコープ

### 含む

- **KB データ層（Rust）**: Markdown ファイルを唯一の真実のソースとし、SQLite を派生インデックス
  （FTS5 全文検索 + リンクグラフ + メタデータ）として持つ。インデックスはいつでもファイルから再構築できる。
- **Capture（AI なし）**: テキスト/Markdown の貼り付け、Web ページ取り込み（readability で本文抽出）、
  ファイル取り込み（デジタル PDF/Word はテキスト抽出、音声/動画/画像は添付として保存 + 任意の説明文）。
- **Workshop**: 受信箱の素材一覧 → エディタ（左=ソース、右=結果）。
  - AI アクション: 関連する既存条目を検索（RAG）し、新素材と合わせて構造化草稿 + `[[リンク]]` 候補を
    ストリーミング生成。ユーザーは指示文で再生成・手動編集できる。
  - 手動パス: AI を使わず直接条目を書く/直すこともできる。
  - 承認すると Markdown 条目を `entries/` に書き出し、インデックスを更新する。
- **Wiki**: 条目の閲覧・カテゴリ・被リンク（バックリンク）・関連。Markdown エディタで編集。`[[ ]]` 双方向リンク。
- **Graph**: 既存条目とリンクを力学グラフライブラリで描画。ノードクリックで条目へ。
- **検索**: SQLite FTS5 による全文検索。
- **Dashboard**: インデックスから算出した実データ統計（条目数・リンク数・孤立数・最近の更新）と孤立条目の検出。

### 含まない（延期）

文字起こし（Whisper）、OCR、マルチモーダル取り込み、意味検索/ベクトル、複数素材のマージ、
完全な Lint（薄い/重複/陳腐化の自動検出。孤立検出のみ MVP に含む）、Publish、Bots、プラグイン市場。

## アーキテクチャ

### ディスク構成（真実のソース）

```
<kb-root>/
  .expertbase/
    kb.toml          # 既存。ナレッジベース設定
    index.sqlite     # 派生インデックス（削除・再構築可能）
  inbox/             # 取り込んだ原始素材（未加工）
  entries/           # Wiki 条目（.md, frontmatter + [[双方向リンク]]）
  attachments/       # 取り込んだメディア/大容量ファイル
```

- **条目（entries/*.md）** の frontmatter: `type`, `title`, `description`, `cat`, `tags`, `created`, `updated`。
  `type` は OKF 互換の必須フィールドとして扱い、通常条目の既定値は `Entry` とする。
  本文中の `[[タイトル]]` がリンク。バックリンクは派生（保存しない）。
  MVP ではリンク解決を曖昧にしないため、`title` は KB 内で一意とする。
- **受信箱の素材（inbox/*.md）** も Markdown で表現する。frontmatter: `type`(text/web/pdf/doc/audio/video/image),
  `source`, `status`(pending/processed), `attachment`(任意, 添付への相対パス), `captured_at`。
  本文は抽出テキスト（文字を持つ素材）またはユーザー説明文（任意, メディア）。
  これにより「すべてが Markdown、インデックスはそこから派生」という規約を保つ。

### OKF との関係

Google Cloud の Open Knowledge Format（OKF）は、知識を「ディレクトリ + Markdown + YAML frontmatter」で
表現する最小限の交換形式であり、Expert Base のローカルコア方針と相性がよい。
本 MVP では OKF をアプリケーションアーキテクチャではなく、**将来のインポート/エクスポート互換を意識した
ディスク上の知識表現の参考仕様**として採用する。

取り込む考え方:

- 知識単位は Markdown ファイルで表現する。
- YAML frontmatter に `type`, `title`, `description`, `tags` などの機械可読メタデータを置く。
- ファイルとディレクトリは Git や通常のファイルシステムで読める形に保つ。
- ファイル間の関係は本文中のリンクから派生させる。
- `index.md` や `log.md` は将来の OKF エクスポート時に生成できる補助ファイルとして扱う。

採用しないもの:

- OKF をそのまま UI/検索/同期/権限/プラグインの設計に拡張しない。
- MVP の内部リンクは既存方針どおり `[[タイトル]]` を使う。OKF の Markdown link 形式への変換は
  インポート/エクスポート層で扱う。
- SQLite は OKF の一部ではなく、Expert Base の派生インデックスとしてのみ使う。

このため、MVP の保存形式は「OKF 風のローカル Markdown ナレッジベース」であり、完全な OKF バンドルとしての
入出力は別マイルストーンで扱う。

### 派生インデックス（SQLite, rusqlite）

| テーブル | 役割 |
|------|------|
| `entries(path, type, title, description, cat, tags, updated, words)` | 条目メタデータ。`title` は一意 |
| `links(src_path, dst_title)` | リンク辺（バックリンク・孤立・グラフの元） |
| `entries_fts`（FTS5 trigram: title + body） | 全文検索。日本語/中国語の 3 文字以上の部分一致を MVP の基準にする |
| `inbox(path, type, source, status, captured_at)` | 受信箱の状態 |

真実のソースは常に Markdown。インデックスが壊れたらファイルから再構築する。

### データフロー

UI（WebView 内 React）→ 既存の typed `invoke` クライアント（`frontend/src/lib/tauri`）→ Rust の Tauri コマンド
→ Markdown ファイル / SQLite。HTTP は使わない。AI 呼び出しも Rust 内の `AiProvider` を経由し、
UI は API キーに直接触れない（キーは Rust 側の設定に保存）。

### AI 接合面（AiProvider trait）

```rust
// 構造化リクエスト（ワークショップが組み立てる）
pub struct StructureRequest {
  pub source_text: String,            // 新素材の本文（文字を持つもののみ）
  pub related: Vec<EntrySummary>,     // FTS で引いた関連既存条目（title + excerpt）
  pub instruction: String,           // ユーザーの指示文
}
pub struct StructureResult {
  pub title: String,
  pub cat: String,
  pub body_markdown: String,
  pub suggested_links: Vec<String>,  // 既存条目タイトル
}
pub trait AiProvider {
  fn structure(&self, req: StructureRequest, on_token: Channel<String>) -> Result<StructureResult, AiError>;
}
```

- MVP は `ClaudeProvider`（`reqwest` で Anthropic API 直叩き、BYO-key）のみ実装する。
- 関連条目の検索は MVP では **FTS5 のキーワード一致のみ**（ベクトル不要）。上位 N 件の title + excerpt を
  プロンプトに渡し、リンク候補は LLM に既存タイトルから選ばせる。
- ストリーミングは Tauri 2 の `Channel` でトークンを UI へ流す。非ストリーミング（完了後に一括表示）でも可。
- 将来のローカル LLM やマルチモーダルは、この trait の別実装として後から差し込む（下流は変更不要）。

## コンポーネント（責務・インターフェース・依存）

各ユニットは単一責務で、Tauri コマンド境界で疎結合にする。

1. **kb データ層** (`src-tauri/src/kb/`): ファイル I/O、frontmatter 解析、`[[リンク]]` 抽出、
   インデックスの構築・更新・再構築。純関数（パスと値を受け取る）として実装し、コマンドは薄いラッパー。
   MVP では `serde_yaml` で frontmatter を扱い、`regex` で `[[リンク]]` を抽出する。完全な Markdown AST は後続段階で検討する。
   依存: `serde_yaml`, `regex`, `rusqlite`。
2. **capture** (`src-tauri/src/capture/`): 各取り込み元 → `inbox/` の Markdown 素材へ正規化。
   依存: `dom_smoothie`+`htmd`（Web）, `pdf-extract`/`docx-rs`（文書）, ファイルコピー（メディア）。
3. **workshop** (`src-tauri/src/workshop/`): 素材 + 関連条目 → `AiProvider` を呼ぶ編成、結果を `entries/` へ確定。
   依存: kb データ層, `AiProvider`。
4. **ai** (`src-tauri/src/ai/`): `AiProvider` trait と `ClaudeProvider`。依存: `reqwest`。
5. **Wiki / Graph / 検索 / Dashboard ビュー**（フロント）: 既存の空状態ビューに、typed クライアント経由で実データを接続。
   Graph 描画とエディタは外部ライブラリを利用。

## 再利用するもの（車輪の再発明をしない）

実装時に最終確認するが、現時点の候補:

| 用途 | 候補 |
|------|------|
| Markdown 解析 / frontmatter | MVP は `serde_yaml` + `regex`。Markdown AST が必要になった段階で `comrak`/`pulldown-cmark` を検討 |
| 全文検索 | SQLite **FTS5 trigram**（CJK 部分一致を優先） |
| Web 本文抽出 | `dom_smoothie`（Readability 移植）+ `htmd`（HTML→MD） |
| PDF/Word テキスト抽出 | `pdf-extract` / `docx-rs` |
| Markdown エディタ（フロント） | CodeMirror 6（Obsidian と同系）または Milkdown |
| グラフ描画（フロント） | `react-force-graph` または Cytoscape.js |
| LLM 呼び出し | Anthropic API（`reqwest` 直叩き、または `genai` で抽象化） |

自前で書くのは: リンク/バックリンクのインデックス、ワークショップの RAG 編成 + プロンプト、受信箱の状態機械のみ。

## エラーハンドリング

- Tauri コマンドは `Result<T, String>` を返し、UI 側の typed クライアントで型付きエラーに変換する。
- UI から渡される KB 内パスは `entries/*.md` / `inbox/*.md` などの許可された相対 Markdown パスに限定し、
  絶対パス・`..`・ネストした未知パスを拒否する。
- AI エラー（API キー未設定・ネットワーク・レート制限）は UI で区別して表示し、手動パスへ退避できる。
- インデックス破損時はファイルから再構築する。ファイル I/O 失敗はそのまま表面化させる。

## テスト / 検証

- **Rust 単体テスト**（`#[cfg(test)]` + `tempfile`）: frontmatter 解析、`[[リンク]]` 抽出、
  バックリンク/孤立の算出、FTS の往復、受信箱の状態遷移、各 capture 正規化。
- `AiProvider` は trait なので、ワークショップ編成はフェイク実装でテストできる（ネットワーク不要）。
- **フロント**: `bun run lint` と `bun run build`（静的エクスポート）が通る。
- **結合**: `tauri dev` で取り込み → ワークショップ → 条目確定 → Wiki/Graph/検索 に反映、を一周確認。
