# Expert Base Backend

このディレクトリには、Expert Base のバックエンドプロジェクトが含まれます。

バックエンドは現在、Python アプリケーションプロジェクトとして初期化されています。依存関係管理、ツール、環境規約、task コマンドは用意されていますが、FastAPI アプリケーションコードはまだ作成されていません。

## 技術スタック

- Python 3.13
- uv
- FastAPI
- Pydantic Settings
- SQLAlchemy
- Alembic
- Celery
- Redis
- HTTPX
- Ruff
- Pytest

## 現在のベースライン

バックエンドは次の内容で初期化されています。

- `pyproject.toml` の runtime dependencies
- `pyproject.toml` の development dependencies
- `.env.example` のローカル環境テンプレート
- `Taskfile.yml` のバックエンド task コマンド
- `AGENTS.md` のバックエンド向けエージェント指示

API app、routes、models、migrations、workers、business modules はまだ実装されていません。

## コマンド

コマンドは `backend/` から実行します。

```bash
task install
task lint
task format
task format:check
task test
task lock
```

`task install` は uv で依存関係をインストールします。

`task lint` は Ruff lint checks を実行します。

`task format` は Ruff で Python ファイルを整形します。

`task format:check` は Ruff の整形状態を検証します。

`task test` は Pytest を実行します。

`task lock` は依存関係を upgrade せずに `uv.lock` を更新します。`uv.lock` はローカル生成物として扱い、Git にはコミットしません。

AI エージェントとコントリビューターは、直接 `uv` を呼ぶのではなく `task` コマンドを優先します。

## 予定しているディレクトリ構成

アプリケーションコードはまだ作成されていません。実装を開始したら、次の構成を使います。

```txt
backend/
  src/
    expertbase/
      api/              # FastAPI app, routers, dependencies
      core/             # settings, logging, security, shared infrastructure
      modules/          # domain modules
      workers/          # Celery app and tasks
  tests/                # backend tests
  alembic/              # database migrations
  pyproject.toml
```

空のアーキテクチャフォルダは作りません。最初の実ファイルが必要になった時点でディレクトリを追加します。

## 環境

バックエンドコードが環境設定を使い始める段階で、ローカル開発用に `.env.example` を `.env` にコピーします。

```bash
cp .env.example .env
```

現在の環境テンプレートには次が含まれます。

- `APP_ENV`
- `APP_NAME`
- `DATABASE_URL`
- `REDIS_URL`
- `OBJECT_STORAGE_ENDPOINT`
- `OBJECT_STORAGE_BUCKET`

## 開発メモ

このプロジェクトは uv application project として設定されています。

```toml
[tool.uv]
package = false
```

これは、依存関係管理のために import 可能な Python package を先に用意する必要がない、という意味です。バックエンド実装を開始するときに、アプリケーション package structure を追加します。
