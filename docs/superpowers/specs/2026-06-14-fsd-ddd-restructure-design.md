# Expert Base 前後端アーキテクチャ再編設計（FSD / DDD）

日付: 2026-06-14
ステータス: ドラフト（ユーザーレビュー待ち）

## 背景と目的

`AGENTS.md`（ルート / `frontend/` / `src-tauri/`）が定める方針に、既存コードの
**物理的なディレクトリ構造**を一致させる。

- フロントエンド: Feature-Sliced Design（FSD）。`app -> features -> entities -> shared` の一方向依存。
- バックエンド: Domain-Driven Design（DDD）。`interface -> application -> domain` で、`domain` は
  Tauri / 永続化 / FS / DTO に依存しない。

### 設計上の重要な制約（文書由来）

両 `AGENTS.md` は次を明記しており、本再編はこれを破ってはならない。

- 「**空の構造フォルダを作らない**。最初の実ファイルが必要になった時だけ作る。」
- フロント: 「`src/lib/` と `src/components/` は、**意図的な移行が要求されない限り**、現行の共有置き場として有効。」
  → 本タスクは「意図的な移行の要求」に該当するため移行を行うが、空殻は作らない。
- バック: 「**実コードが深い構造を要求するまで現行のモジュール配置を軽量に保つ**。機能が実際の
  ビジネスルールや複数ユースケースを抱えた時に `domain / application / infrastructure / interface` へ分割する。」
  → 「実際にルールを抱えている機能」だけを分割し、薄い機能は薄いまま整理する。

### 不変条件（最重要）

これは**純粋な再配置リファクタリング**である。**振る舞いは一切変えない**。

- IPC コマンド名・引数・戻り値の JSON 形（camelCase）は不変。
- Markdown / SQLite のディスク契約、frontmatter 規約は不変。
- 既存テストは移動後もそのまま緑。新規ロジックは足さない。
- UI の見た目・文言・遷移は不変。

## スコープ

### 含む

- フロント `src/` を FSD レイヤ（`app / widgets / features / entities / shared`）へ再配置。
- バック `src-tauri/src/` の `kb` 機能を DDD レイヤへ分割し、`ai / capture / workshop` の内部レイヤ境界を明確化。
- スライス公開 API（`index.ts`）の導入と、内部実装への直接 import の解消。
- import パスの一括更新、`AGENTS.md` の該当記述の追従更新（必要時）。

### 含まない

- 機能追加・挙動変更・新規テスト観点の追加。
- shadcn 生成物（`components/ui`）の中身改変。
- ルーティング構成（URL）の変更。

## 採用案と代替案

| 案 | 内容 | トレードオフ | 採否 |
|----|------|--------------|------|
| A. 全量強制再排 | 4 つのバック機能すべてを `domain/application/infrastructure/interface` に割る | 形式的に最も「教科書 DDD」。ただし `ai/capture/workshop` は薄く、空殻に近い層を生み文書の「軽量に保つ」に反する | 不採用（文書違反リスク） |
| **B. 文書忠実移行（推奨）** | フロントは全スライス実ファイルで充填。バックは実ルールを持つ `kb` のみ完全分割、`ai/capture/workshop` は機能内のレイヤを明確化（実分離のある所だけ submodule 化） | 文書の「空殻禁止 / 軽量維持」と「DDD/FSD」を両立。差分は機械的で回帰安全 | **採用** |
| C. フロントのみ先行 | フロント FSD だけ実施しバックは据え置き | 範囲は小さいが「前後端の架構を揃える」目的を半分しか満たさない | 不採用 |

以降は **案 B** を前提に詳細を定義する。`ai/capture/workshop` を完全 4 層へ昇格するか（案 A 寄り）は
レビュー時の決定ポイントとして残す（「決定ポイント」節）。

---

## フロントエンド（FSD）

### レイヤ定義と依存方向

```
src/app      ルーティング/合成。page は薄く、screen を公開 API 経由で呼ぶ
   │
src/widgets  複数 feature/entity を束ねる合成 UI（アプリシェル、再利用 UI ブロック）
   │
src/features 単一ユーザーシナリオ（画面 + 状態 + 呼び出し + ロジック）
   │
src/entities ドメイン向けクライアントモデル・型・純関数・アダプタ（UI/HTTP を含まない）
   │
src/shared   再利用プリミティブ・framework 中立ユーティリティ・設定・型付き IPC クライアント
```

import は下方向のみ。スライス間 import は必ず公開 API（`index.ts`）経由。

### `widgets/` 層の導入根拠（文書要件: 境界を文書化してから導入）

`AGENTS.md` は「`widgets/` 等を導入する場合は事前に境界を文書化せよ」と求める。本設計で導入する根拠:

- `components/shell/*`（app-shell, sidebar, title-bar, nav-item, kb-switcher, settings-dialog）は
  **複数 feature と entity と shared を合成するアプリ外殻**であり、単一シナリオ（feature）でも純粋プリミティブ（shared）でもない。
- `material-row` は capture / workshop / dashboard の **3 つの feature から再利用**される、`material` エンティティに
  束ねられた UI ブロック。`AGENTS.md` は「entities は UI を含まない」と明記するため entities には置けない。
  feature 横断の再利用 UI は widgets が正しい置き場。

→ `widgets` 境界 = 「entities/shared（および必要なら feature）を合成する、ルート非依存の再利用合成 UI」。

### 目標ツリー

```
src/
  app/
    layout.tsx, globals.css, favicon.ico
    (app)/
      layout.tsx              # widgets/app-shell を合成（薄い）
      page.tsx                # features/dashboard を描画（薄い）
      capture/page.tsx        # features/capture
      workshop/page.tsx       # features/workshop
      workshop/process/page.tsx
      wiki/page.tsx           # features/wiki
      graph/page.tsx          # features/graph
      publish/page.tsx        # features/publish
      bots/page.tsx           # features/bots
      plugins/page.tsx        # features/plugins
  widgets/
    app-shell/      index.ts, app-shell, sidebar, title-bar, nav-item, kb-switcher, settings-dialog
    material-row/   index.ts, material-row
  features/
    dashboard/      index.ts, ui/{dashboard-view, recent-materials, wiki-health}
    capture/        index.ts, ui/capture-view
    workshop/       index.ts, ui/{workshop-view, workshop-process-view}
    wiki/           index.ts, ui/wiki-view
    graph/          index.ts, ui/graph-view
    publish/        index.ts, ui/publish-view
    bots/           index.ts, ui/bots-view
    plugins/        index.ts, ui/plugins-view
    onboarding/     index.ts, ui/onboarding
  entities/
    knowledge-base/ index.ts, model/store      # 旧 lib/kb/store
    material/       index.ts, model/{types, adapt}  # RawMaterial / RAW_TYPE / STATUS / inboxToMaterial
    wiki-entry/     index.ts, model/{types, adapt}  # WikiEntry / GraphNode / entryRefToWiki
  shared/
    api/tauri/      index.ts, client          # 旧 lib/tauri/client（IPC 関数 + 契約 DTO 型）
    ui/             button, card, dialog, input, label, select, switch, textarea,  # 旧 components/ui
                    icon, logo, panel, tag, page-head, empty-state, ring, seg-tabs # 旧 components/eb + _components/seg-tabs
    config/         nav                        # 旧 lib/nav
    i18n/           translate, dictionaries, data
    lib/            utils                       # cn()
    providers/      providers                   # 旧 components/providers（theme/i18n context）
```

### 逐ファイル移行マッピング（old -> new）

| 旧パス | 新パス | レイヤ |
|--------|--------|--------|
| `app/(app)/capture/capture-view.tsx` | `features/capture/ui/capture-view.tsx` | feature |
| `app/(app)/workshop/workshop-view.tsx` | `features/workshop/ui/workshop-view.tsx` | feature |
| `app/(app)/workshop/process/workshop-process-view.tsx` | `features/workshop/ui/workshop-process-view.tsx` | feature |
| `app/(app)/wiki/wiki-view.tsx` | `features/wiki/ui/wiki-view.tsx` | feature |
| `app/(app)/graph/graph-view.tsx` | `features/graph/ui/graph-view.tsx` | feature |
| `app/(app)/publish/publish-view.tsx` | `features/publish/ui/publish-view.tsx` | feature |
| `app/(app)/bots/bots-view.tsx` | `features/bots/ui/bots-view.tsx` | feature |
| `app/(app)/plugins/plugins-view.tsx` | `features/plugins/ui/plugins-view.tsx` | feature |
| `components/dashboard/dashboard-view.tsx` | `features/dashboard/ui/dashboard-view.tsx` | feature |
| `components/dashboard/recent-materials.tsx` | `features/dashboard/ui/recent-materials.tsx` | feature |
| `components/dashboard/wiki-health.tsx` | `features/dashboard/ui/wiki-health.tsx` | feature |
| `components/onboarding/onboarding.tsx` | `features/onboarding/ui/onboarding.tsx` | feature |
| `components/shell/*` | `widgets/app-shell/*` | widget |
| `app/(app)/_components/material-row.tsx` | `widgets/material-row/material-row.tsx` | widget |
| `app/(app)/_components/seg-tabs.tsx` | `shared/ui/seg-tabs.tsx` | shared |
| `lib/kb/store.ts` | `entities/knowledge-base/model/store.ts` | entity |
| `lib/data/types.ts` | `entities/material` + `entities/wiki-entry` の `model/types.ts` に分割 | entity |
| `lib/data/adapt.ts` | `entities/material/model/adapt.ts`（inboxToMaterial） + `entities/wiki-entry/model/adapt.ts`（entryRefToWiki） | entity |
| `lib/data/store.ts` | 空プレースホルダ。利用箇所（publish の `WIKI`）に応じ `entities/wiki-entry/model` へ集約、未使用分は削除 | entity |
| `lib/tauri/client.ts` | `shared/api/tauri/client.ts`（IPC 関数 + `Kb/EntryRef/SearchHit/Stats/GraphData/InboxItem/StructureResult/OllamaModel` DTO） | shared |
| `lib/nav.ts` | `shared/config/nav.ts` | shared |
| `lib/i18n/*` | `shared/i18n/*` | shared |
| `lib/utils.ts` | `shared/lib/utils.ts` | shared |
| `components/ui/*` | `shared/ui/*`（中身不変） | shared |
| `components/eb/*` | `shared/ui/*` | shared |
| `components/providers.tsx` | `shared/providers/providers.tsx` | shared |

### 依存方向の検証（現状 import グラフより）

検証は全 `@/` import を新レイヤに射影し、上向き/横断 import が無いことを確認する。判明済みの注意点:

- `lib/data/types.ts` と `lib/nav.ts` は `@/components/eb/icon`（`IconName` 型）を import。
  → `icon` は `shared/ui`、`types`→entity、`nav`→`shared/config`。`entity -> shared`、`shared/config -> shared/ui` は
  どちらも合法（型のみ依存）。
- `material-row` は `entities/material` + `shared/ui` に依存 → widget なので合法。
- `entities/*/model/adapt.ts` は `shared/api/tauri` の DTO 型を import → `entity -> shared` で合法。
- `entities/knowledge-base/model/store.ts` は `shared/api/tauri` の関数を呼ぶ → `entity -> shared` で合法
  （状態コンテナであり HTTP クライアント実装そのものではない）。
- `widgets/app-shell` は `features/onboarding` を合成 → `widget -> feature` で合法。
- 公開 API: 各 `features/*`・`entities/*`・`widgets/*`・`shared/api/tauri` は `index.ts` を持ち、
  外部はそれ経由でのみ import。スライス内部ファイルへの直接 import を禁止。

### `app/` を薄く保つ

各 `app/(app)/<route>/page.tsx` は対応する feature の公開 API から screen を import して描画するだけにする。
ルーティング文脈（params 等）が要るものだけ読む。`page` にロジックを置かない。

---

## バックエンド（DDD）

### レイヤ定義

```
interface       Tauri コマンド（#[tauri::command]）。入出力の変換のみ。薄い。
application      ユースケース。domain を編成し、infrastructure 抽象に依存。
domain          実体・値オブジェクト・不変条件・ドメインエラー・純関数。外部依存なし。
infrastructure  永続化(SQLite/TOML)・FS・HTTP 取得・文書抽出など具体アダプタ。
```

`domain` は Tauri / rusqlite / reqwest / FS / serde DTO に依存しない。

### `kb` 機能の完全分割（実ビジネスルール + 13 ユースケースを持つため）

現 `kb.rs`（408 行）は「レジストリ規則 + パス安全不変条件 + TOML 永続化 + 13 コマンド」を混在させている。
これを次へ分割する。**関数本体・テストは原則そのまま移送**（純粋な移動）。

```
src/kb/
  mod.rs                      # サブモジュール宣言 + 既存の pub(crate) 再公開
  domain/
    entry.rs                  # 旧 kb/entry.rs（EntryMeta/Entry/parse/serialize/extract_links/word_count/split_frontmatter）
    material.rs               # 旧 kb/store.rs のうち純粋部分（MaterialMeta/Material/parse_material/serialize_material）
    registry.rs               # KbEntry/Registry/KbConfig/create_kb 検証/set_active 規則/expand_home/checked_kb_markdown_path
  application/
    service.rs                # open_active/active_kb_root + 各コマンドのユースケース本体
  infrastructure/
    config_store.rs           # load_registry/save_registry（TOML I/O）
    index.rs                  # 旧 kb/index.rs（SQLite FTS5）
    store.rs                  # 旧 kb/store.rs のうち FS 部分（slug/write_entry/read_entry/save_entry）
  interface.rs                # kb_* の #[tauri::command] 群（薄いラッパ）
```

| 旧 | 新 | レイヤ |
|----|----|--------|
| `kb/entry.rs` | `kb/domain/entry.rs` | domain |
| `kb/store.rs`（型/parse/serialize） | `kb/domain/material.rs` | domain |
| `kb/store.rs`（slug/write_entry/read_entry/save_entry） | `kb/infrastructure/store.rs` | infra |
| `kb/index.rs` | `kb/infrastructure/index.rs` | infra |
| `kb.rs`（KbEntry/Registry/KbConfig/create_kb/set_active/expand_home/checked_kb_markdown_path） | `kb/domain/registry.rs` | domain |
| `kb.rs`（load_registry/save_registry） | `kb/infrastructure/config_store.rs` | infra |
| `kb.rs`（open_active/active_kb_root） | `kb/application/service.rs` | application |
| `kb.rs`（kb_* コマンド） | `kb/interface.rs` | interface |

注: `create_kb` は `fs::create_dir_all` / `fs::write` を直接呼ぶ。厳密 DDD では domain は FS 非依存のため、
**検証規則（domain/registry）と書き込み（infra/config_store）を分離**する。分離は移送と同時に行う最小限の調整に留める。

### `ai / capture / workshop`（軽量整理。文書の「軽量維持」に従う）

これらは薄く、空殻層を生むため**ファイル分割は行わず**、各モジュール内のレイヤ責務を
コメント区画と関数配置で明確化するに留める（決定ポイントで完全分割へ昇格可）。

- `ai.rs`: 既に「domain（`AiProvider` trait, DTO, `AiError`）+ interface（2 コマンド）」、`ai/ollama.rs` は infra アダプタ。
  → 現状維持。`ai/ollama.rs` が infrastructure であることを doc コメントで明示。
- `capture.rs`: application（`write_material`/`copy_attachment`/`kind_for_ext` + 3 コマンド）、`capture/{doc,web}.rs` は
  infra（PDF/Word 抽出・readability）。→ 現状維持、責務を明示。
- `workshop.rs`: application（`related_entries`/`draft`/`confirm` + 2 コマンド）。kb・ai を編成するユースケース層。→ 現状維持。

### `lib.rs`（合成ルート / interface 登録）

`mod kb; mod ai; mod capture; mod workshop;` と `generate_handler!` の登録を維持。
`kb` 分割に伴い、コマンドの参照パスを `kb::interface::kb_*`（または `mod.rs` で再公開した `kb::kb_*`）へ更新。
**コマンド名・登録順は不変**（IPC 契約維持）。

---

## 命名・公開 API 規則

- フロント: スライスのフォルダは kebab-case。各スライスは `index.ts` で公開シンボルのみ re-export。
  内部ファイルへの cross-slice import 禁止（`features/*/ui/...` を外から直接読まない）。
- バック: `kb/mod.rs` で従来 `pub(crate)` だった関数（`open_active` 等）の可視性を維持し、
  他機能（capture/workshop）からの参照を壊さない。

## テストと回帰門禁

純粋移動のため、**移動前後で全テスト緑**が合格条件。

1. ベースライン記録: `bun run test`（cargo）/ `bun run lint` / `bun run build` を再編前に実行し結果を控える。 → 検証: 緑を確認
2. 各段階後に同 3 コマンドを再実行。 → 検証: ベースラインと同一の緑
3. Rust 単体テストは `#[cfg(test)] mod tests` ごと移送し、`use super::*` / `crate::kb::...` パスを更新。 → 検証: `cargo test` 同数の test が緑
4. IPC 契約の不変を確認: コマンド名・`#[serde(rename_all = "camelCase")]`・登録順を grep で差分ゼロ確認。 → 検証: 名前/順序の差分なし
5. フロント: `bun run build`（静的エクスポート）成功で import 解決を担保。 → 検証: ビルド成功

## 段階的コミット計画

各段階は独立してビルド/テスト緑を保ち、レビュー可能な単位でコミットする。

1. **バック: `kb` を DDD 分割**（ファイル移送 + 可視性調整 + import 更新）。 → 検証: `bun run test` 緑、IPC 名差分ゼロ
2. **バック: `ai/capture/workshop` 責務明示**（doc コメント区画のみ、構造変更なし）。 → 検証: `bun run test` 緑
3. **フロント: `shared/` 確立**（ui/eb/seg-tabs, api/tauri, config/nav, i18n, lib/utils, providers の移送 + import 更新）。 → 検証: `bun run lint` + `bun run build` 緑
4. **フロント: `entities/` 確立**（knowledge-base / material / wiki-entry の model + adapt + 公開 API）。 → 検証: build 緑
5. **フロント: `features/` 確立**（全 `*-view` + dashboard 子 + onboarding を移送、公開 API 化）。 → 検証: build 緑
6. **フロント: `widgets/` 確立**（app-shell, material-row）と `app/` を薄い page に整理。 → 検証: build 緑、URL 不変
7. **ドキュメント追従**（必要なら `AGENTS.md` の現行配置記述を実態へ更新）。 → 検証: 記述と実態一致

## 決定ポイント（レビューで確認したい点）

1. `ai/capture/workshop` を **案 A（完全 4 層分割）へ昇格**するか、**案 B（軽量整理）**で止めるか。推奨は B。
2. フロントの IPC DTO 型を **`shared/api/tauri` に契約として集約**（推奨）するか、entities 側へ移すか。
3. `widgets/` 層の導入可否（本設計は導入前提で境界を文書化済み）。
4. `lib/data/store.ts` の空プレースホルダを entities へ集約するか、未使用分を削除するか。

## リスクと対策

- **大量 import 書き換えによる解決漏れ** → 各段階で `bun run build` / `cargo test` を必ず通し、段階間で緑を維持。
- **可視性（pub/pub(crate)）変更で他機能が壊れる** → `kb/mod.rs` で旧シンボルを再公開し外部参照を温存。
- **「空殻フォルダ」化** → 実ファイルを持つレイヤのみ作成。`ai/capture/workshop` は薄いまま据え置き。
- **IPC 契約の意図しない変化** → コマンド名・serde 属性・登録順を grep 差分で監視。
