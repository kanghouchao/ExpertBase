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
- バック `src-tauri/src/` の全機能（`kb / ai / capture / workshop`）を DDD レイヤ（domain/application/infrastructure/interface のうち実在するもの）へ分割。
- スライス公開 API（`index.ts`）の導入と、内部実装への直接 import の解消。
- import パスの一括更新、`AGENTS.md` の該当記述の追従更新（必要時）。

### 含まない

- 機能追加・挙動変更・新規テスト観点の追加。
- shadcn 生成物（`components/ui`）の中身改変。
- ルーティング構成（URL）の変更。

## 採用方針（ベストプラクティス・妥協なし）

ユーザー決定: **「やるなら最善で、妥協しない」**。よって全機能を DDD の実在レイヤへ分割し、
フロントは全スライスを実ファイルで充填する。

「空殻禁止」との両立: `ai/capture/workshop` も含め、各機能を分割しても**生成される全レイヤに実コードが入る**
（ai = domain trait + infrastructure アダプタ + interface、capture = application + infrastructure 抽出器 + interface、
workshop = domain + application + interface）。実体のないレイヤは作らない。これにより
「ベストプラクティス（明示的な層境界）」と AGENTS.md の「空フォルダ禁止」を同時に満たす。

| 検討案 | 採否 |
|--------|------|
| 全機能を実在レイヤへ DDD 分割 + フロント全 FSD | **採用** |
| `kb` のみ分割し他は軽量据え置き | 不採用（妥協のため） |
| フロントのみ先行 | 不採用（目的の半分） |

レイヤ規約: レイヤは**複数ファイルなら同名ディレクトリ**（`domain/`）、**単一ファイルならレイヤ名 `.rs`**
（`application.rs`）。実コードのないレイヤファイル/ディレクトリは作らない。

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
  app-shell は `features/onboarding` を合成する（`widget -> feature` は下方向で合法）。

→ `widgets` 境界 = 「feature / entity / shared を合成する、ルート非依存のアプリ外殻 UI」。現状の唯一の widget は `app-shell`。

注: `material-row` は当初 widget 候補だったが、実測では **capture 機能のみが利用**するため
`features/capture/ui/material-row` に置く（単一機能専用 UI を widget 化すると feature→widget の逆向き依存を生むため）。

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
  features/
    dashboard/      index.ts, ui/{dashboard-view, recent-materials, wiki-health}
    capture/        index.ts, ui/{capture-view, material-row}
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
| `app/(app)/_components/material-row.tsx` | `features/capture/ui/material-row.tsx` | feature（capture のみ利用） |
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
- `material-row`（features/capture/ui）は `entities/material` + `shared/ui` に依存 → `feature -> entity/shared` で合法。
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

### `ai / capture / workshop`（各機能を実在レイヤへ分割）

空殻を作らず、各機能が**実際に持つ**レイヤへ分割する。

**`ai`**（domain ポート + infra アダプタ + interface）:
```
src/ai/
  mod.rs            # レイヤ宣言 + pub(crate) 再公開
  domain.rs         # AiProvider trait / StructureRequest / StructureResult / EntrySummary / AiError / FakeProvider(cfg test)
  infrastructure/
    mod.rs          # pub mod ollama
    ollama.rs       # 旧 ai/ollama.rs（OllamaProvider）
  interface.rs      # ai_has_key / ai_list_ollama_models コマンド
```

**`capture`**（domain 分類 + application ユースケース + infra 抽出 + interface）:
```
src/capture/
  mod.rs
  domain.rs         # kind_for_ext / split_name（素材タイプ判定の純ロジック）
  application.rs    # write_material / copy_attachment + capture_text/file/web のユースケース本体
  infrastructure/
    mod.rs          # pub mod doc; pub mod web
    doc.rs          # 旧 capture/doc.rs（PDF/Word 抽出）
    web.rs          # 旧 capture/web.rs（readability 抽出）
  interface.rs      # capture_text / capture_file / capture_web コマンド
```

**`workshop`**（domain 検索語抽出 + application RAG 編成 + interface）:
```
src/workshop/
  mod.rs
  domain.rs         # candidate_terms（FTS 検索候補語の抽出規則）
  application.rs    # related_entries / draft / confirm（kb・ai を編成するユースケース）
  interface.rs      # workshop_draft / workshop_confirm コマンド
```

| 旧 | 新 | レイヤ |
|----|----|--------|
| `ai.rs`（trait/DTO/AiError/FakeProvider） | `ai/domain.rs` | domain |
| `ai/ollama.rs` | `ai/infrastructure/ollama.rs` | infra |
| `ai.rs`（2 コマンド） | `ai/interface.rs` | interface |
| `capture.rs`（kind_for_ext/split_name） | `capture/domain.rs` | domain |
| `capture.rs`（write_material/copy_attachment + 取込ロジック） | `capture/application.rs` | application |
| `capture/doc.rs`・`capture/web.rs` | `capture/infrastructure/{doc,web}.rs` | infra |
| `capture.rs`（3 コマンド） | `capture/interface.rs` | interface |
| `workshop.rs`（candidate_terms） | `workshop/domain.rs` | domain |
| `workshop.rs`（related_entries/draft/confirm） | `workshop/application.rs` | application |
| `workshop.rs`（2 コマンド） | `workshop/interface.rs` | interface |

注: `capture/application.rs` は infra（doc/web/FS）と domain（kind_for_ext）に依存し、
コマンドからは呼ばれる側。`workshop/application.rs` は `crate::ai`・`crate::kb` を編成する。
domain（candidate_terms 等）は外部依存を持たない純関数のみ。

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
2. **バック: `ai/capture/workshop` を DDD 分割**（実在レイヤへ分割、テスト移送）。 → 検証: `bun run test` 緑、IPC 名差分ゼロ
3. **フロント: `shared/` 確立**（ui/eb/seg-tabs, api/tauri, config/nav, i18n, lib/utils, providers の移送 + import 更新）。 → 検証: `bun run lint` + `bun run build` 緑
4. **フロント: `entities/` 確立**（knowledge-base / material / wiki-entry の model + adapt + 公開 API）。 → 検証: build 緑
5. **フロント: `features/` 確立**（全 `*-view` + dashboard 子 + onboarding を移送、公開 API 化）。 → 検証: build 緑
6. **フロント: `widgets/` 確立**（app-shell, material-row）と `app/` を薄い page に整理。 → 検証: build 緑、URL 不変
7. **ドキュメント追従**（必要なら `AGENTS.md` の現行配置記述を実態へ更新）。 → 検証: 記述と実態一致

## 決定事項（ユーザー確定: 妥協なし・ベストプラクティス）

1. `ai/capture/workshop` も **完全に DDD レイヤ分割**する（実在レイヤのみ、空殻なし）。
2. フロントの IPC DTO 型は **`shared/api/tauri` に契約として集約**（バック契約の単一ソース、AGENTS.md「型を集約」に整合）。
3. `widgets/` 層を **導入**する（境界は本設計で文書化済み）。
4. `lib/data/store.ts` の空プレースホルダは **使用点に応じ entities へ集約し、未使用分は削除**する。

## リスクと対策

- **大量 import 書き換えによる解決漏れ** → 各段階で `bun run build` / `cargo test` を必ず通し、段階間で緑を維持。
- **可視性（pub/pub(crate)）変更で他機能が壊れる** → `kb/mod.rs` で旧シンボルを再公開し外部参照を温存。
- **「空殻フォルダ」化** → 実コードを持つレイヤのみ作成。各機能で実体のない層（例: ai に application は無い）は作らない。
- **IPC 契約の意図しない変化** → コマンド名・serde 属性・登録順を grep 差分で監視。
