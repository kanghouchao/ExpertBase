# Expert Base Frontend

このディレクトリには、Expert Base の Web アプリケーションが含まれます。

フロントエンドは、プロダクト UI を担当します。対象には、ナレッジワークスペース管理、ソース取り込みフロー、Wiki レビューワークフロー、アシスタント設定、公開制御、将来の管理画面が含まれます。

## 技術スタック

- Next.js App Router
- React
- TypeScript
- Bun
- Tailwind CSS v4
- shadcn/ui
- lucide-react

## ディレクトリ構成

```txt
frontend/
  src/
    app/                 # App Router routes, layouts, and global CSS
    components/
      ui/                # shadcn/ui primitives
    lib/                 # Shared utilities and future API clients
  public/                # Static assets
  components.json        # shadcn/ui configuration
  next.config.ts         # Next.js configuration
  eslint.config.mjs      # ESLint configuration
  tsconfig.json          # TypeScript configuration
  package.json           # Bun scripts and dependencies
```

プロダクトが成長したら、次のフォルダを使います。

```txt
src/components/          # Shared product components
src/features/            # Feature-level UI, actions, and client interactions
src/hooks/               # Reusable client hooks
src/types/               # Frontend-only TypeScript types
```

空のアーキテクチャフォルダは作りません。最初の実ファイルが必要になった時点でフォルダを追加します。

## コマンド

コマンドは `frontend/` から実行します。

`task dev` はローカルの Next.js 開発サーバーを起動します。

`task lint` は、フロントエンド変更を完了扱いにする前に通す必要があります。

`task format` は Prettier でフロントエンドファイルを整形します。

`task format:check` は Prettier の整形状態を検証します。

`task build` は、routing、layout、config、import、本番ビルドに影響するコードを変更したときに使います。
