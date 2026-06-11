# Expert Base

Expert Base は、プロフェッショナルな知識労働者向けの、プライベートで拡張可能なナレッジベースシステムです。

## プロジェクトの目的

Expert Base は、未整理のソース資料から、構造化され保守しやすいプライベートナレッジベースを継続的に育てるためのシステムです。それから、ユーザーはこのナレッジベースを検索し、双方向リンクをたどり、AI を活用して洞察を得ることができます。最終的な目標は、専門家が自分の知識を管理し、活用するための強力で柔軟なプラットフォームを提供することです。

## アーキテクチャ方針

現在のプロジェクトは、次のアーキテクチャ判断を前提にしています。

- フロントエンドとバックエンドを分離する。
- 非同期アーキテクチャを前提にする。
- プラグイン化できる構造にする。

ディレクトリごとのドキュメント:

- [backend/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/backend/README.md)
- [frontend/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/frontend/README.md)
- [infra/README.md](/Users/kanghouchao/CodeProjects/ExpertBase/infra/README.md)

## コマンド

Taskfile コマンドをプロジェクトの主なエントリーポイントとして使います。ルートの Taskfile は、プロジェクト全体の起動、停止、状態確認、クリーンアップに焦点を当てています。

リポジトリルートから:

```bash
task start
task start:app
task stop
task stop:app
task status
task logs
task clean:cache
```

デフォルト環境は `development` です。別の環境に対して実行する場合は、コマンドの前に `ENV` を指定します。

```bash
ENV=production task config
ENV=production task start
```

長時間実行されるサービスは、それ自体が目的のタスクでない限り起動しません。

