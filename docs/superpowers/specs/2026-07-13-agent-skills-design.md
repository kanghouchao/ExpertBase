# プラグイン化 · Agent Skills 対応 設計

**日付**: 2026-07-13
**ブランチ（提案）**: `feat/41-agent-skills`
**関連 Issue**: #41（本体）。#42（MCP、姉妹 issue、今回スコープ外）。#44（入力欄スラッシュコマンド UI、今回スコープ外・接続点だけ用意）。

## 目標

AGENTS.md の頂上原則「プラグイン化」を Agent Skills（<https://agentskills.io/specification>）で兌現する。KB 内 `skills/` と `~/.agents/skills/` の二段スキャンでスキルを発見し、tools 能力を持つモデルには catalog をシステムプロンプトへ注入 + `activate_skill` ツールで自動発動できるようにし、tools 能力の有無に関わらずユーザーが技能一覧 UI から明示発動できるようにする。新設の頂上モジュール `plugin`（`src-tauri/src/plugin/`）にスキャン・解析・活動化ロジックを置き、workshop は消費方に徹する。スクリプト実行（`scripts/`）・技能編集・KB 同期は v1 の非目標（issue 明記）。

## 核心的な洞察（読源码確認した事実）

- **rig-core 0.39.0 の `Tool` トレイト**（`~/.cargo/registry/.../rig-core-0.39.0/src/tool/mod.rs`）: `definition(&self, prompt: String) -> ToolDefinition { name, description, parameters: serde_json::Value }` を手書きする。本リポジトリの全既存ツール（`workshop/infrastructure/tools/kb_read.rs` ほか）は `#[derive(rig_derive::rig_tool)]` などのマクロを一切使わず、素の `impl Tool` + `serde_json::json!()` で `parameters` を組んでいる。**enum 制約は `parameters` の JSON Schema に `"enum": [...]` を手で足すだけ**で、rig 側に専用 API は無い（`read_entry(id)` のような限定値パターンの先例は無いが、JSON Schema の `enum` キーは標準機能なのでモデル側には効く）。`activate_skill` の `name` 引数は、発見済みスキル名のリストを起動時に `json!({"enum": names})` へ動的に詰めれば良い。
- **KB ディレクトリ規約**（`kb/application.rs`, `kb/infrastructure/*`）: KB ルート直下に `entries/`（条目本体）と `.expertbase/`（`kb.toml` + `index.sqlite`、内部メタデータ専用）。ユーザー編集対象のコンテンツ用ディレクトリは `entries/` と同格に置く慣習 → KB 内技能ディレクトリは **`<kb ルート>/skills/`**（`entries/` と同格、`.expertbase/` の中ではない）。
- **`workshop_chat` の IPC には既に `tools: bool` パラメータがある**（`workshop/interface.rs:95`）が、現状 `let _ = tools;` で握りつぶされている。フロントは `workshop-session.ts` の `selectedTools`（`OllamaModel.tools`）から計算済みの値を渡している。→ tools 能力ゲートの配線点はここ。
- **`workshop_chat` 経由の生成は設計時点では tools 対応モデル必須だった**（`workshop-session.ts` の `canGenerate = ... && selectedTools && ...`）。tools 非対応モデルは当時の UI では生成ボタン自体が押せなかった。→ **2026-07-17 のレビュー修正で解消**: `canGenerate` は tools 能力を要求せず、tools 非対応モデルでは後端が toolset を一切組まない（`# Tools` 節ごと省略）。明示発動（`# Activated Skills` 注入）だけが効く経路として端到端で成立する（#41/#44 受け入れ条件）。
- **frontmatter 分離の手法**（`kb/domain/entry.rs` の `split_frontmatter` + `serde_yaml::from_str`）はコードとして一般的（`---` フェンス2つで YAML と本文を割る、BOM/CRLF 許容）。ただし `kb::entry` の関数は KB 条目専用モジュール内にあり、`plugin` から見ると「別 feature の internal 実装への越境」になる。`serde_yaml` は Cargo 依存に既にあるので**依存関係だけ共有し、フェンス分離の実装は `plugin` 内に小さく複製する**（後述の確定した決定 13）。
- **`agent::infrastructure::runner::run` は `tools: Vec<Box<dyn ToolDyn>>` を無条件に `.tools(tools)` へ渡す**（`agent/infrastructure/runner.rs:72,84`）。tools 非対応モデルに対するゲートは runner 層には無い＝ゲートは呼び出し側（`workshop::application::chat`）の責務。
- **`workshop/prompt.rs` が system 前文の唯一の持ち主**。`agent_system_with(tools_section, source_ids)` が `# Tools` と `# Sources` を組み立てる。`# Tools` の内容は `tools::render_tools_section` が `definition()` から生成し、prompt.rs は「どこに置くか」だけを知る。Skills もこの対称パターンに乗せる。
- **`used_sources: Arc<Mutex<Vec<String>>>`**（`workshop/infrastructure/tools/mod.rs`）は `build_toolset` 呼び出し（＝ 1 回の `chat()` 実行）ごとに新しく作られ、`ReadSource`/`FetchWeb` がそれを共有して「今回の生成内で読んだ素材」を重複排除する。`activate_skill` の単発生成内重複排除に同じ型をそのまま使える。
- **`Backend` 型は `{ kb, agent, workshop }` の3ドメインで、`shared/api/types.ts` が全ての IPC 契約型 + ドメイン API インタフェースを一元管理する**（機能がどの `features/*` に属すかとは無関係）。`kbApi`/`agentApi`/`workshopApi` は `shared/api/backend.ts` で `tauriBackend`/`fakeBackend` に委譲する薄いラッパ。

## アーキテクチャ

### レイヤの落とし所

| レイヤ | 置くもの |
|---|---|
| `plugin/domain.rs` | `Skill { name, description, body, location, source, has_scripts }` 値構造体、`SkillSource { Kb, User }`、frontmatter 解析の純関数（`parse_skill_frontmatter(raw, dir_name) -> Result<(name, description, body), SkipReason>`、`SkipReason` 4 種）、catalog / activated セクションのレンダリング純関数 |
| `plugin/infrastructure/scan.rs` | 二段ディレクトリ走査 `discover_skills(kb_root: &Path, home: &Path) -> Vec<Skill>`（`domain::parse_skill_frontmatter` を呼び、失敗時は `log::warn!` して該当スキルだけ捨てる） |
| `plugin/infrastructure/activate_skill.rs` | `ActivateSkill`（`rig_core::tool::Tool` 実装、`ToolDyn` として `build_toolset` に混ぜる）。単発生成内重複排除は `Arc<Mutex<Vec<String>>>`（`used_sources` と同型） |
| `plugin/interface.rs` | `plugin_list_skills` Tauri コマンド（KB ルート + home で `discover_skills` を呼び、`Vec<Skill>` をそのまま返す。name/description/location/source/hasScripts/body を一括で返す） |
| `plugin/mod.rs` | 公開面: `pub use domain::{Skill, SkillSource, render_catalog, render_activated}; pub use infrastructure::scan::discover_skills; pub use infrastructure::activate_skill::ActivateSkill; pub mod interface;` |
| `workshop/prompt.rs` | `agent_system_with` に `skills_catalog: &str` / `activated_skills_section: &str` を追加。空文字列なら該当節を省略（`# Sources` と同じ語義） |
| `workshop/infrastructure/tools/mod.rs` | `build_toolset` に `skills: &[plugin::Skill]` を渡し、`tools_capable && !skills.is_empty()` のときだけ `ActivateSkill` を push |
| `workshop/application.rs` | `chat()` が `plugin::discover_skills` の結果（interface から渡される）+ `tools_capable: bool` + `activated_skill_names: Vec<String>` を受け、catalog（tools_capable のときだけ）と activated セクション（無条件）を組んで `agent_system_with` へ渡す |
| `workshop/interface.rs` | `workshop_chat` が既存の `tools: bool` を実際に使う（`let _ = tools;` を廃止）。新規 IPC 引数 `activated_skill_names: Vec<String>`。ブロッキングブロック内で `plugin::discover_skills(&root, &home)` を settings 読み込みと一緒に実行 |
| `lib.rs` | `mod plugin;` 追加、`plugin::interface::plugin_list_skills` を `generate_handler!` に登録 |
| フロント `shared/api/types.ts` | `Skill` / `SkillSource` 型、`PluginApi { listSkills(): Promise<Skill[]> }`、`Backend` に `plugin: PluginApi` 追加。`WorkshopApi.chat` の引数に `activatedSkillNames: string[]` 追加 |
| フロント `shared/api/backend.ts` / `tauri.ts` / `fake.ts` | `pluginApi` 委譲 + `invoke("plugin_list_skills")` 実装 + フェイク実装 |
| フロント `features/plugin/` | 新設 feature。`ui/skill-panel.tsx`（一覧描画 + 発動チェック/ボタン。source バッジ・`hasScripts` 注記を含む、props 駆動: `skills`, `activatedNames`, `onActivate`）、`index.ts`（公開面） |
| フロント `features/workshop/model/workshop-session.ts` | `activatedSkillNames: string[]` を会話状態へ追加（`sourceIds` と同格）。`plugin_list_skills` を呼んで一覧を保持し、`onActivate` で名前を追加。`chat()` 呼び出しへ `activatedSkillNames` を渡す |
| フロント `features/workshop/ui/workshop-view.tsx` | `features/plugin` の `SkillPanel` を composition で組み込む |

### system prompt の構造（`agent_system_with` の新シグネチャ）

issue 要求3（catalog）と要求4（明示発動）は別物なので、system prompt には **独立した2節** を作る（互いに排他ではなく両方同時に出うる）。

```rust
// workshop/prompt.rs
pub fn agent_system_with(
  tools_section: &str,
  source_ids: &[String],
  skills_catalog: &str,           // 空文字列なら # Skills 節を省略
  activated_skills_section: &str, // 空文字列なら # Activated Skills 節を省略
) -> String
```

- `# Skills`（catalog）: 発見済み**全**スキルの `name: description` 箇条書き。**tools 能力が無いモデルには渡さない**（呼び出し側が空文字列を渡す）。0 件のときも空文字列（catalog 自体を出さない、issue 要求3明記）。
- `# Activated Skills`: `activated_skill_names` に対応するスキルの本文（frontmatter 剥離済み）を `## <name>\n<body>` で連結。**tools 能力に関わらず常に評価**（issue 要求4「tools 能力に依存しない」）。空なら省略。

### `plugin::domain` の主要シグネチャ

```rust
pub enum SkillSource { Kb, User }              // Serialize, rename_all = "camelCase"

pub struct Skill {
  pub name: String,
  pub description: String,
  pub body: String,        // frontmatter 剥離済み本文
  pub location: String,    // SKILL.md への絶対パス（表示用）
  pub source: SkillSource,
  pub has_scripts: bool,   // scripts/ サブディレクトリの有無
}

pub enum SkipReason { MalformedFrontmatter, NameMissing, DescriptionMissing, NameMismatch }

/// 純関数。IO なし。呼び出し側（infrastructure）が生テキストと dir_name を渡す。
pub fn parse_skill_frontmatter(raw: &str, dir_name: &str)
  -> Result<(String /*name*/, String /*description*/, String /*body*/), SkipReason>;

pub fn render_catalog(skills: &[Skill]) -> String;               // "- name: description" 箇条書き、0件は ""
pub fn render_activated(skills: &[Skill], activated: &[String]) -> String; // "## name\nbody" 連結、0件は ""
```

### `plugin::infrastructure::scan` の走査ロジック

```rust
pub fn discover_skills(kb_root: &Path, home: &Path) -> Vec<Skill> {
  let mut by_name: HashMap<String, Skill> = HashMap::new();
  for skill in scan_dir(&home.join(".agents/skills"), SkillSource::User) {
    by_name.insert(skill.name.clone(), skill);
  }
  for skill in scan_dir(&kb_root.join("skills"), SkillSource::Kb) {
    by_name.insert(skill.name.clone(), skill); // 同名は KB が勝つ（後勝ち）
  }
  let mut skills: Vec<Skill> = by_name.into_values().collect();
  skills.sort_by(|a, b| a.name.cmp(&b.name)); // 決定的順序
  skills
}

fn scan_dir(dir: &Path, source: SkillSource) -> Vec<Skill> {
  // read_dir が無ければ空 Vec（ディレクトリ自体が存在しなくてもエラーにしない）。
  // 各サブディレクトリの SKILL.md を読み、domain::parse_skill_frontmatter を通す。
  // Err(reason) は log::warn!("skill skipped: {dir_name} ({reason:?})") してそのスキルだけ捨てる。
}
```

### `ActivateSkill` ツール

```rust
pub struct ActivateSkill {
  pub skills: Vec<Skill>,                          // build_toolset が discover_skills の結果を渡す
  pub activated_this_call: Arc<Mutex<Vec<String>>>, // 1 回の chat() 実行内の重複排除（used_sources と同型）
}
// definition(): parameters の name に `"enum": skills.iter().map(|s| &s.name)` を動的注入
// call(): name 未発見 → "(no skill found: ...)"。今回既発動済み → "(skill already activated this turn: ...)"。
//         それ以外 → activated_this_call に積んで skill.body をそのまま返す（<skill_content> ラップなし）。
```

### `workshop::application::chat` のフロー（擬似コード）

```rust
pub async fn chat(
  settings: AiSettings, model: String, think: bool,
  tools_capable: bool,                 // workshop_chat の既存 `tools` パラメータをそのまま配線
  root: PathBuf, sources: Vec<String>,
  skills: Vec<plugin::Skill>,          // interface 側の spawn_blocking で discover_skills 済み
  activated_skill_names: Vec<String>,  // フロントの activatedSkillNames
  messages: Vec<ChatTurn>, cancel: Arc<AtomicBool>,
  tx: UnboundedSender<StreamProgress>, pending: confirm::PendingConfirms,
) -> Result<String, AppError> {
  let gate = ...; // 既存のまま
  let mut toolset = tools::build_toolset(&root, &sources, settings.brave_api_key, gate);
  if tools_capable && !skills.is_empty() {
    toolset.push(Box::new(plugin::ActivateSkill {
      skills: skills.clone(),
      activated_this_call: Arc::new(Mutex::new(Vec::new())),
    }));
  }
  let catalog = if tools_capable { plugin::render_catalog(&skills) } else { String::new() };
  let activated_section = plugin::render_activated(&skills, &activated_skill_names);
  let system = agent_system_with(
    &tools::render_tools_section(&toolset).await, &sources, &catalog, &activated_section,
  );
  crate::agent::run(provider, &base_url, &model, think, &system, toolset, messages, cancel, &tx).await
}
```

### フロントの発動フロー

- `features/plugin` の `SkillPanel` は自前で `pluginApi.listSkills()` を呼んで一覧を描画する（`source` バッジ・`hasScripts` の「本バージョンはスクリプトを実行しません」注記込み）。各行にチェック/ボタンがあり、押すと親から渡された `onActivate(name)` を呼ぶだけ（判定・状態は持たない、`source-chip.tsx` 等と同じ props 駆動の提示コンポーネント）。
- `activatedSkillNames: string[]` の状態自体は `features/workshop/model/workshop-session.ts` が持つ（`sourceIds` と同格の「この会話に付随する状態」）。`onActivate` はここへ追加するだけ（重複排除は配列に既存なら追加しない、単純な `Set` 相当）。
- ストリーム中に `ChatPhase { phase: "toolResult", name: "activate_skill", ... }` を観測したら、その技能名を同じ `activatedSkillNames` へ追加する（モデル自動発動とボタン発動を同じ記帳先に一本化）。
- 次回 `workshopApi.chat(...)` 呼び出し時に `activatedSkillNames` をそのまま渡す。斜め線（#44）が担うのは「入力欄 `/skill-name` を解析して同じ `onActivate` 相当の更新関数を呼ぶ」だけで、Tauri コマンド層は変更不要。

## 確定した決定（覆し可能）

1. **スキャン時機**: 常駐状態・キャッシュ・ファイル監視は無し。`workshop_chat` 呼び出し毎、および `plugin_list_skills` 呼び出し毎に毎回スキャンする。ローカルディレクトリ走査+YAML 解析はミリ秒級（スキル数は現実的に数十件止まり）なので、キャッシュ整合性の複雑さを避ける方が単純。
2. **system prompt は2節**: `# Skills`（catalog、tools 能力ゲート、0 件なら省略）と `# Activated Skills`（発動済み本文、tools 能力に依存しない、空なら省略）。互いに独立、両方同時に出うる。
3. **#41 のスコープ**: 技能一覧 UI + 最小限で使える発動チェック/ボタン（`activatedSkillNames` を直接更新）まで実装する。入力欄スラッシュコマンドの構文解析は #44 に残す。`plugin_list_skills` は 1 回の呼び出しで `name`/`description`/`location`/`source`/`hasScripts`/`body` を全て返す（往復を増やさない）。
4. **フロント配置**: 新設 `features/plugin`（後端の頂上 `plugin` モジュールに対称）。`entities/skill` は新設しない。workshop は `features/plugin` の公開面（`index.ts`）経由で `SkillPanel` を組み込む。
5. **KB 内 `skills/` の位置**: `<KB ルート>/skills/`（`entries/` と同格。`.expertbase/` の中ではない、`kb.toml` の一部でもない）。
6. **スキップ判定4条件**（寛容な解析、単一スキルの失敗が走査全体を止めない）: (a) frontmatter の YAML 解析失敗、(b) `name` 欠落/空、(c) `description` 欠落/空、(d) `name` とディレクトリ名の不一致。4条件とも同じ扱い＝スキップ + `log::warn!`。
7. **信頼境界（trust gate）は v1 で作らない**: 公式 client 実装ガイド（調研文書 §3.2 第2点）は「project レベルのスキルはユーザーが信頼済みと印を付けた場合のみ読み込め」と推奨するが、本製品は local-first 単一ユーザー向けで、KB `skills/` はユーザー自身が管理する KB のサブディレクトリであり「他人のリポジトリを開く」という同ガイドの脅威モデルとは前提が異なる。issue の受け入れ条件にも信頼確認ステップの要求は無い。KB 内スキルは既定で信頼済み扱いとする。公式ガイドからの明示的な逸脱として記録する。
8. **ログ機構**: 既存の `log::warn!` + `tauri-plugin-log`（debug ビルドのみ有効、`lib.rs:14-20`）をそのまま使う。ログ出力先の変更、リリースビルドでの可視化、スキップ内容の UI 表示は行わない。
9. **spec 上限（name ≤64字+文字種、description ≤1024字）は強制しない**: issue が要求するのは決定6の4条件のみ。文字数超過は catalog のトークン消費が増えるだけで正誤の問題ではなく、実害が出てから制限すれば良い（YAGNI）。
10. **`activate_skill` の重複排除は二層**: (a) 単発生成内（同一 `chat()` 呼び出し内、モデルが `multi_turn` の中で同じスキルを複数回呼ぶ場合）は `used_sources` と同型の `Arc<Mutex<Vec<String>>>` を `ActivateSkill` が持ち、二度目は「既に発動済み」通知を返す。(b) 会話をまたぐ重複排除は、モデル発動・ボタン発動の両方を同じフロント状態 `activatedSkillNames` に記帳することで実現する（バックエンドに会話跨ぎの新規状態は作らない）。
11. **`activate_skill` の戻り値は素の本文文字列**。`<skill_content name="...">` のような構造化タグは付けない（本リポジトリの全ツールが素の文字列を返す既存慣習に合わせる。公式ガイドの推奨だが、ローカル小型モデルに構造化タグが必要という根拠は無い）。
12. **tools 能力ゲートの配線**: `workshop_chat` に既に存在する（今は無視されている）`tools: bool` を実際に使う。新規の能力検出機構は作らない。
13. **`plugin` は `kb::entry::split_frontmatter` を直接呼ばない**: 技術的には `kb::entry` は `pub use domain::entry;` で公開されており呼べるが、`plugin` は将来 MCP（#42）も同居する「業務非依存の外部標準アダプタ」という位置づけであり、`kb`（業務固有機能）への依存を持ち込むと筋が悪い。フェンス分離ロジック（約20行、BOM/CRLF 許容）は `plugin::domain` 内に複製する。共有するのは `serde_yaml` という**依存関係**のみで、既存の `kb::entry` 実装コードそのものではない。`kb::open_active`（`pub(crate)`、KB ルート取得）は workshop も使っている既存の橋渡しであり、そちらは通常通り利用する。
14. **catalog / activated セクションのレンダリングは `plugin::domain` の純関数**（`render_catalog` / `render_activated`）。`workshop/prompt.rs` は「どこに節を置くか」だけを持ち、内容の生成には関与しない。`# Tools` 節（`tools::render_tools_section` が生成、`prompt.rs` が拼接するだけ）と対称なパターン。

## 境界（YAGNI）

- **スクリプト実行（`scripts/`）は実装しない**（issue 非目標）。`has_scripts` フラグだけ持ち、UI で「本バージョンはスクリプトを実行しません」と注記する。
- **技能作成/編集 UI は作らない**（issue 非目標）。
- **KB 同期へのスキル包含は考慮しない**（issue 非目標、将来の商業化課題）。
- **信頼境界（trust gate）は作らない**（決定7）。
- **ファイル監視は作らない**（決定1）。
- **spec の文字数/文字種上限チェックは作らない**（決定9）。
- **`<skill_content>` 構造化タグは作らない**（決定11）。
- **リリースビルドでのログ可視化・スキップの UI 通知は作らない**（決定8）。
- **`~/.agents/skills/` 以外の user スコープ候補ディレクトリ（クライアント固有パス）は探索しない**: issue が明示するのは KB 内 `skills/` と `~/.agents/skills/` の2つだけ。

## テスト/検証

- **単体テスト（廉価・有意）**:
  - `plugin::domain::parse_skill_frontmatter`: 正常系（name/description/body 抽出）、4条件それぞれの `SkipReason`（YAML 壊れ、name 欠落、description 欠落、name 不一致）。
  - `plugin::domain::render_catalog` / `render_activated`: 0件で空文字列、複数件の整形、`activated` に無い名前は無視。
  - `plugin::infrastructure::scan::discover_skills`: KB のみ・user のみ・両方（同名は KB 優先）・両方空（ディレクトリ不在含む）、`tempfile` で実ディレクトリを作る。
  - `plugin::infrastructure::ActivateSkill`: 存在するスキルの本文を返す、存在しないスキル名の通知、同一インスタンス内の二度目呼び出しの重複排除通知、`definition()` の `parameters.enum` がスキル名と一致。
  - `workshop::prompt::agent_system_with`: `# Skills` / `# Activated Skills` の有無切り替え（既存の `# Sources` 省略テストと同型）。
  - `workshop::infrastructure::tools::build_toolset`: `tools_capable=false` または `skills` が空のとき `activate_skill` が toolset に現れないこと。
- **フロント**: `features/plugin` の `SkillPanel`（一覧描画、`hasScripts` バッジ、`onActivate` 呼び出し）。`workshop-session.ts` の `activatedSkillNames` 追加・重複排除・`chat()` への配線（`workshop-run.test.ts` 系に倣う）。
- **搬移・配線でカバーできない**: 実モデルによる `activate_skill` 自動発動の信頼性（小型モデルが catalog を見て正しく呼ぶか）は自動テスト化しない。手動確認（issue の受け入れ条件通り、tools 対応モデルで実機確認）に留める。
- `bun run test`（= cargo test。現在 129 passed）+ `bun run lint` + `bun run --cwd frontend build`。

## 仮定/要検証

- ~~**前端の `canGenerate` ゲートと「tools 非対応モデルでも明示発動は使える」という受け入れ条件の間に実践上のギャップがある**~~ → **解消済み（2026-07-17 レビュー修正）**: `canGenerate` から `selectedTools` 要求を外し、tools 非対応モデルでも送信できる（KB ツール無し・発動済み技能だけの対話を許す判断）。合わせて `build_toolset` は `tools_capable=false` で空 toolset を返し（呼べないツールをリクエストに載せない、rig は空なら tools 欄ごと省略）、`# Tools` 節も省略する。`workshop.toolsRequired` の一行ヒントは不要になり削除。
- `agentskills.io` の frontmatter 仕様（`name` ≤64字+文字種、`description` ≤1024字、`license`/`compatibility`/`metadata`/`allowed-tools`）は調研文書 §3.1 で既に一次資料確認済み、本設計ではこれ以上の再検証はしない（決定9で意図的に強制しないと決めた）。
- rig-core の enum 制約が実際に Ollama 側でモデルの幻覚を防ぐ効果があるかは、tools 対応ローカルモデルでの実地確認が必要（`parameters.enum` を渡すこと自体は JSON Schema として正しいが、小型モデルがどこまで従うかは未検証）。
