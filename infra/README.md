# Expert Base Infrastructure

このディレクトリには、Expert Base の Docker Compose インフラが含まれます。

development と production は、実行時の前提が異なるため、意図的に別々の Compose ファイルに分けています。development はローカルでの扱いやすさを優先します。production はデプロイ境界を優先します。

これはまだ初期インフラベースラインです。production Compose ファイルは、完全な本番運用セットアップではありません。HTTPS、証明書、バックアップ、監視、ログ、シークレット管理、アップグレード手順は今後の作業です。

## 環境

デフォルト環境は `development` です。

```bash
task config
task up
```

production baseline には `ENV=production` を使います。

```bash
ENV=production task config
ENV=production task up
```

Taskfile は環境を次のように対応付けます。

```txt
ENV=development -> compose.development.yml + env/development.env
ENV=production  -> compose.production.yml  + env/production.env
```

`env/<ENV>.env` が存在しない場合、`task init-env` は対応する example file から作成します。

## ファイル

```txt
infra/
  compose.development.yml       # local development services
  compose.production.yml        # production deployment baseline
  env/
    development.env.example     # committed development template
    development.env             # local development values, ignored by git
    production.env.example      # committed production template
    production.env              # production values, ignored by git
  Taskfile.yml                  # infrastructure commands
  AGENTS.md                     # agent instructions for this directory
```

`env/*.env` はコミットしません。

## Development Services

```txt
Traefik        # local HTTP gateway and dashboard
PostgreSQL 17  # primary database, direct port exposed
Redis 8.2.0    # cache and Celery broker, direct port exposed
MinIO          # S3-compatible object storage, direct ports exposed
Backend        # optional app profile, bind-mounted source
Frontend       # optional app profile, bind-mounted source
```

development では次を使います。

- `compose.development.yml`
- `env/development.env`
- `*.localhost` hostnames
- debugging 用の direct ports
- bind-mounted frontend and backend source
- `bun run dev` などの development commands
- insecure local access を許可した Traefik dashboard

デフォルトの development routes:

```txt
http://expertbase.localhost:8080
http://api.expertbase.localhost:8080
http://minio.expertbase.localhost:8080
http://traefik.expertbase.localhost:8080
```

デフォルトの development direct ports:

```txt
Traefik HTTP:      8080
Traefik dashboard: 8081
PostgreSQL:        5432
Redis:             6379
MinIO API:         9000
MinIO UI:          9001
Backend direct:    8000
Frontend direct:   3000
```

## Production Baseline

```txt
Traefik        # public HTTP gateway
PostgreSQL 17  # internal database
Redis 8.2.0    # internal cache and broker
MinIO          # internal object storage
Backend        # image-based service
Frontend       # image-based service
```

production では次を使います。

- `compose.production.yml`
- `env/production.env`
- real hostnames
- image-based frontend and backend services
- bind-mounted source code なし
- PostgreSQL、Redis、MinIO の direct ports なし
- insecure Traefik dashboard なし

production は現在 HTTP のみを公開します。HTTPS と証明書自動化は、deployment target が明確になってから追加します。

## コマンド

コマンドは `infra/` から実行します。

```bash
task init-env
task config
task config-app
task up
task up-app
task ps
task logs
task down-app
task down
```

明示的な環境:

```bash
ENV=development task config
ENV=production task config
```

`task config` は Compose configuration を検証し、レンダリングします。

`task config-app` は frontend と backend service を含めた Compose configuration を検証し、レンダリングします。production はデフォルトで app services を含むため、主に development で有用です。

`task up` は選択された環境の services を起動します。

`task up-app` は `app` profile 付きで services を起動します。development で frontend と backend services も Compose に含めたい場合に使います。

`task ps` は service status を表示します。

`task logs` は全 services の logs を follow します。特定 service のみを follow するには `task logs -- postgres` を使います。

`task down-app` は `app` profile を含む services を停止します。

`task down` は named volumes を削除せずに services を停止します。

AI エージェントとコントリビューターは、直接 `docker compose` を呼ぶのではなく、これらの `task` コマンドを優先します。

## データ

Compose は named Docker volumes を使います。

```txt
postgres_data
redis_data
minio_data
```

`task down` で services を停止しても、これらの volumes は削除されません。

明示的に依頼されない限り、破壊的な cleanup tasks は追加しません。
