# Expert Base

Expert Base は、プロフェッショナルな知識労働者向けの、プライベートで拡張可能な、ローカルファーストのナレッジベースシステムです。

## プロジェクトの目的

Expert Base は、未整理のソース資料から、構造化され保守しやすいプライベートナレッジベースを継続的に育てるためのシステムです。それから、ユーザーはこのナレッジベースを検索し、双方向リンクをたどり、AI を活用して洞察を得ることができます。最終的な目標は、専門家が自分の知識を管理し、活用するための強力で柔軟なプラットフォームを提供することです。

## アーキテクチャ方針

現在のプロジェクトは、次のアーキテクチャ判断を前提にしています。

- ローカルファーストの Tauri 2 デスクトップアプリケーションとして提供する。
- UI は `frontend/` の Next.js（静的エクスポート）で、Tauri シェルが読み込む。
- コアロジックとローカルデータ層は `src-tauri/` の Rust で実装する。
- ナレッジベースのデータはローカルに保存する（Markdown ファイル + 派生インデックス）。真実のソースは常にオープンフォーマット。
- サーバーが必須の機能（同期・Web 公開・Bot ホスティング）は、将来のオプションのクラウドサービスとして扱う。
- プラグイン化できる構造にする。

設計の経緯は [docs/superpowers/specs/2026-06-12-desktop-architecture-design.md](docs/superpowers/specs/2026-06-12-desktop-architecture-design.md) を参照。

ディレクトリごとのドキュメント:

- [frontend/README.md](frontend/README.md)

## コマンド

Taskfile コマンドをプロジェクトの主なエントリーポイントとして使います。

リポジトリルートから:

```bash
task install      # 依存関係をインストール
task dev          # デスクトップアプリを開発モードで起動
task build        # デスクトップアプリのバンドルをビルド
task lint         # フロントエンドの Lint
task test         # Rust テストを実行
task clean:cache  # ビルドキャッシュを削除
```

前提ツール: [Bun](https://bun.sh/)、[Rust](https://rustup.rs/)、[Task](https://taskfile.dev/)。
