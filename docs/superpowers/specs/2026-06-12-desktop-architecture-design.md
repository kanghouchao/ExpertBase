# Expert Base デスクトップアーキテクチャ設計

日付: 2026-06-12
ステータス: 承認済み（調査セッションでユーザーが方向性を確定）

## 背景と決定

Expert Base は「コンテンツは成長する資産・絶対的な安全・AI フレンドリー」を価値の中心に置く。
従来の Web 前後端分離アーキテクチャ（Next.js + FastAPI + Docker/Traefik）はこの価値と矛盾する
（データがサーバーに置かれる、個人開発者にホスティングコストと運用責任が乗る）。

調査の結論として、**ローカルファーストのデスクトップアプリケーション**に全面移行する。
将来的に「同期・Web 公開・Bot ホスティング」など、サーバーが必須の機能だけをオプションの
クラウドサービスとして追加する（Obsidian モデル）。

## 技術選定

| 領域 | 選定 | 理由 |
|------|------|------|
| デスクトップシェル | Tauri 2 | バンドル ~3MB、低メモリ、deny-by-default の権限モデルが将来のプラグイン実行に適する |
| UI | 既存の Next.js App Router を静的エクスポート（`output: "export"`） | 現フロントエンドは純クライアントコード。Tauri 公式がサポートする統合方式で、既存コードと規約を全部保持できる |
| ランタイム/PM | Bun（現状維持） | 既存の規約 |
| ローカルデータ | ナレッジベース = Markdown ファイルのフォルダ + SQLite インデックス | オープンフォーマット（AI フレンドリー・ロックインなし）。SQLite は検索・グラフ用の派生インデックスであり、真実のソースは常に Markdown |
| バックエンド | 削除。コアロジックは Rust（Tauri コマンド） | FastAPI は hello のみで失うものがない。AI パイプラインが必要になった時点で Python sidecar を検討（Tauri が対応） |
| インフラ | Docker/Traefik 全削除 | デスクトップアプリに不要 |

## リポジトリ構成（変更後）

```
ExpertBase/
├── frontend/            # UI（Next.js 静的エクスポート）— 既存のまま
│   └── ...
├── src-tauri/           # Rust コア（デスクトップシェル + ローカルデータ層）
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs       # Tauri ビルダー、コマンド登録
│   │   └── kb.rs        # ナレッジベース層（データディレクトリ初期化、IPC コマンド）
│   ├── capabilities/
│   ├── tauri.conf.json  # frontendDist = ../frontend/out, devUrl = localhost:3000
│   └── Cargo.toml
├── docs/
└── Taskfile.yml         # dev / build / lint をデスクトップアプリ向けに書き換え
```

`backend/` と `infra/` は削除。

## データフロー

- UI（WebView 内の React）→ `@tauri-apps/api` の `invoke` → Rust の Tauri コマンド → ローカルファイル / SQLite。
- HTTP は使わない。`src/lib` に typed なクライアント（`invoke` ラッパー）を置き、UI コンポーネントに `invoke` を散らさない（既存の「API クライアントは lib に集約」規約を継承）。

## この移行マイルストーンのスコープ

アーキテクチャの置き換えのみ。機能追加はしない。

1. Next.js を静的エクスポートに切り替え、ビルドが通る
2. Tauri 2 シェルが既存 UI を表示して起動する
3. Rust 側に最小のナレッジベース層: アプリデータディレクトリの初期化 + UI から呼べる IPC コマンド（旧 hello API の置き換えに相当）
4. `backend/`・`infra/` の削除、Taskfile・README・AGENTS.md の更新

スコープ外: 同期、プラグインシステム、発布機能、AI パイプライン、MCP サーバー（いずれも本設計の方向性とは整合済み、別マイルストーン）。

## エラーハンドリング

- Tauri コマンドは `Result<T, String>` を返し、UI 側のクライアントで型付きエラーに変換する。
- データディレクトリが初期化できない場合はアプリ起動時にエラーを表示する。

## テスト / 検証

- `task lint` と `bun run build`（静的エクスポート）が通る
- `cargo check`（src-tauri）が通る
- `tauri dev` でウィンドウが起動し、UI が表示され、IPC コマンドが往復する
