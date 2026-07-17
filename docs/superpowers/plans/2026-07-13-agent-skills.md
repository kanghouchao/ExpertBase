# プラグイン化 · Agent Skills 対応 実装計画

> 設計は [specs/2026-07-13-agent-skills-design.md](../specs/2026-07-13-agent-skills-design.md)。ステップは `- [ ]` で追跡。各タスク後に `bun run test` / `bun run lint` を確認。

**Goal:** 新設の頂上モジュール `plugin`（`src-tauri/src/plugin/`）で KB 内 `skills/` と `~/.agents/skills/` の二段スキャン + SKILL.md 解析を実装し、tools 対応モデルへは catalog 注入 + `activate_skill` ツール、tools 能力に関わらずユーザー明示発動（技能一覧 UI + 本文の system prompt 注入）を提供する。workshop は消費側に徹し（`build_toolset` へ `activate_skill` を混ぜる、`prompt.rs` へ2節を足す）、フロントは新設 `features/plugin` が一覧 UI を持ち、workshop がそれを組み込む。

**Tech Stack:** Rust / Tauri 2 / rig-core 0.39（`Tool`/`ToolDyn` 手書き実装、既存パターン踏襲）、TypeScript / React / Next.js（FSD、`features/plugin` 新設）

**Baseline:** cargo test 129 passed。

---

### Task 1: `plugin::domain` — 値構造体 + frontmatter 解析 + レンダリング純関数

**Files:** new `src-tauri/src/plugin/domain.rs`, new `src-tauri/src/plugin/mod.rs`, `src-tauri/src/lib.rs`

- [ ] `mod plugin;` を `lib.rs` に追加（他 feature 同様、`domain`/`infrastructure`/`interface` の DDD 分割）。
- [ ] `plugin/domain.rs`: `SkillSource { Kb, User }`（`Serialize`, `rename_all = "camelCase"`）、`Skill { name, description, body, location, source, has_scripts }`（`Serialize`, `rename_all = "camelCase"`、IPC 境界を越えるので `Clone` も要る）。
- [ ] `SkipReason { MalformedFrontmatter, NameMissing, DescriptionMissing, NameMismatch }`（`Debug` のみ、ログ整形用）。
- [ ] `parse_skill_frontmatter(raw: &str, dir_name: &str) -> Result<(String, String, String), SkipReason>`: `---` フェンスで YAML/本文を分割する小さなヘルパを `plugin::domain` 内に複製（`kb::entry::split_frontmatter` は呼ばない、設計の確定した決定13）。`serde_yaml::from_str` で `{ name: Option<String>, description: Option<String> }`（両方 `#[serde(default)]`）へ解析し、空/欠落は `NameMissing`/`DescriptionMissing`、`name != dir_name` は `NameMismatch`、YAML 自体が壊れているか フェンスが無ければ `MalformedFrontmatter`。
- [ ] `render_catalog(skills: &[Skill]) -> String`: `- name: description` の箇条書き、空スライスは `""`。
- [ ] `render_activated(skills: &[Skill], activated: &[String]) -> String`: `activated` の順に対応する `Skill` を探し `## name\nbody` を `\n\n` 連結、見つからない名前は無視、空なら `""`。
- [ ] 単体テスト: `parse_skill_frontmatter` の正常系 + 4 つの `SkipReason`（フェンス欠落、YAML 壊れ、name 欠落、description 欠落、name 不一致）。`render_catalog`/`render_activated` の 0 件・複数件・存在しない名前の無視。
- [ ] `cargo test`: green（新規テストのみ追加）。

### Task 2: `plugin::infrastructure::scan` — 二段ディレクトリ走査

**Files:** new `src-tauri/src/plugin/infrastructure/mod.rs`, new `src-tauri/src/plugin/infrastructure/scan.rs`

- [ ] `discover_skills(kb_root: &Path, home: &Path) -> Vec<Skill>`: `home.join(".agents/skills")`（user、先）→ `kb_root.join("skills")`（kb、後）の順に `scan_dir` を呼び `HashMap<String, Skill>` へ upsert（後勝ち＝ KB が user を上書き）。結果は `name` 昇順ソートして `Vec` 化（決定的順序）。
- [ ] `scan_dir(dir: &Path, source: SkillSource) -> Vec<Skill>`: `std::fs::read_dir` が失敗（ディレクトリ不在含む）したら空 `Vec`。各サブディレクトリについて `<dir>/SKILL.md` を読み、`domain::parse_skill_frontmatter` を通す。`Err(reason)` は `log::warn!("skill skipped: {dir_name} ({reason:?})")` して当該スキルだけ捨てる（走査全体は継続）。成功したら `location`（`SKILL.md` の絶対パス文字列）、`has_scripts`（`<skill_dir>/scripts` が存在するか）を補って `Skill` を組み立てる。
- [ ] `plugin/infrastructure/mod.rs`: `pub(crate) mod scan;`（他 feature の infra mod.rs に倣う）。
- [ ] 単体テスト（`tempfile` で実ディレクトリ構築）: KB のみ・user のみ・両方（同名は KB 優先で本文が KB 側になること）・両方空（ディレクトリ未作成）・SKILL.md が壊れているスキル1つが混ざっても他のスキルは正常に返る（寛容な解析の実地確認）。
- [ ] `cargo test`: green。

### Task 3: `plugin::infrastructure::activate_skill` — `ActivateSkill` ツール

**Files:** new `src-tauri/src/plugin/infrastructure/activate_skill.rs`, `src-tauri/src/plugin/infrastructure/mod.rs`

- [ ] `ActivateSkill { skills: Vec<Skill>, activated_this_call: Arc<Mutex<Vec<String>>> }`。`workshop/infrastructure/tools/kb_read.rs` と同じ手書き `impl Tool`（`rig_derive` マクロは使わない、本リポジトリの既存流儀）。
- [ ] `const NAME: &'static str = "activate_skill";`、`type Args`（`#[derive(Deserialize)] struct ActivateSkillArgs { #[serde(default)] name: String }`、弱いモデルの欠落を緩く受ける既存流儀に合わせる）。
- [ ] `definition()`: `description` はスキル発動の使いどころを英語で説明。`parameters` の `name` プロパティに `"enum": self.skills.iter().map(|s| s.name.clone()).collect::<Vec<_>>()` を動的に詰める（`serde_json::json!` で手書き、rig 側のマクロ/専用 API は不要 — 設計の核心的洞察を実装で確認）。
- [ ] `call()`: `name` が空 → `"(activate_skill needs a non-empty name)"`。`self.skills` に無い → `"(no skill found: {name})"`。`activated_this_call` に既にあれば → `"(skill already activated this turn: {name})"`（ロックして確認 + 追加を1操作で）。それ以外は名前を `activated_this_call` に積んで `skill.body.clone()` をそのまま返す（`<skill_content>` ラップなし、設計の決定11）。
- [ ] `plugin/infrastructure/mod.rs`: `pub(crate) mod activate_skill;`。
- [ ] 単体テスト: 存在するスキルの本文を返す、存在しない名前の通知、空名の通知、同一インスタンスでの二度目呼び出しの重複排除通知、`definition()` の `parameters["properties"]["name"]["enum"]` がスキル名一覧と一致すること。
- [ ] `cargo test`: green。

### Task 4: `plugin::interface` + 公開面 + コマンド登録

**Files:** new `src-tauri/src/plugin/interface.rs`, `src-tauri/src/plugin/mod.rs`, `src-tauri/src/lib.rs`

- [ ] `plugin/interface.rs`: `plugin_list_skills(app: tauri::AppHandle) -> Result<Vec<Skill>, AppError>`。`app.path().home_dir()` + `crate::kb::open_active(&home)` で KB ルートを取り、`spawn_blocking` の中で `scan::discover_skills(&root, &home)` を呼んで返す（`workshop_chat` の既存ブロッキングパターンに合わせる）。
- [ ] `plugin/mod.rs`: `mod domain; mod infrastructure; pub mod interface;` + 公開面 `pub use domain::{Skill, SkillSource, render_catalog, render_activated}; pub use infrastructure::scan::discover_skills; pub use infrastructure::activate_skill::ActivateSkill;`。
- [ ] `lib.rs`: `mod plugin;` は Task 1 で追加済み。`generate_handler!` に `plugin::interface::plugin_list_skills` を追加。
- [ ] `cargo test`: green（`plugin_list_skills` 自体は Tauri コマンドなので統合テストは書かない、`discover_skills` の単体テストで実質カバー済み）。

### Task 5: `workshop/prompt.rs` — `# Skills` / `# Activated Skills` 節

**Files:** `src-tauri/src/workshop/prompt.rs`

- [ ] `agent_system_with` のシグネチャに `skills_catalog: &str`, `activated_skills_section: &str` を追加。`skills_catalog` が空でなければ `\n\n# Skills\n{skills_catalog}` を足す。`activated_skills_section` が空でなければ `\n\n# Activated Skills\n{activated_skills_section}` を足す（`# Sources` の省略パターンと同型）。
- [ ] 既存の `agent_system_with` 呼び出し（`workshop/application.rs`）を新シグネチャに合わせて更新（本タスクでは仮に空文字列を渡し、実データ配線は Task 7）。
- [ ] 単体テスト: 両方非空で両節が出る、両方空で両節が省略される（既存の `# Sources` 省略テストに倣う）、`# Skills` と `# Activated Skills` が独立に出し分けられる（catalog だけ・activated だけの組み合わせ）。
- [ ] `cargo test`: green。

### Task 6: `workshop::infrastructure::tools::build_toolset` — `activate_skill` の条件付き注入

**Files:** `src-tauri/src/workshop/infrastructure/tools/mod.rs`

- [ ] `build_toolset` に `skills: &[plugin::Skill]`, `tools_capable: bool` を追加。`tools_capable && !skills.is_empty()` のときだけ `Box::new(plugin::ActivateSkill { skills: skills.to_vec(), activated_this_call: Arc::new(Mutex::new(Vec::new())) })` を push。
- [ ] 既存の `tools_section_renders_each_definition_with_signature_and_params` 等のテストは `build_toolset` の呼び出しシグネチャ変更に合わせて更新（`skills: &[]`, `tools_capable: false` を渡す既存ケース + 新規に `tools_capable: true` + スキル1件で `activate_skill(name): ...` が `render_tools_section` の出力に現れることを確認するテストを追加）。
- [ ] `cargo test`: green。

### Task 7: `workshop::application::chat` + `workshop::interface::workshop_chat` — 配線

**Files:** `src-tauri/src/workshop/application.rs`, `src-tauri/src/workshop/interface.rs`

- [ ] `application::chat` のシグネチャに `tools_capable: bool`, `skills: Vec<plugin::Skill>`, `activated_skill_names: Vec<String>` を追加（`#[allow(clippy::too_many_arguments)]` は既に付いている）。
- [ ] `chat` 本体: `tools::build_toolset(&root, &sources, settings.brave_api_key, gate, &skills, tools_capable)` へ配線。`catalog = if tools_capable { plugin::render_catalog(&skills) } else { String::new() }`。`activated_section = plugin::render_activated(&skills, &activated_skill_names)`。`agent_system_with(&tools_section, &sources, &catalog, &activated_section)`。
- [ ] `interface::workshop_chat`: `let _ = tools;` を削除し、実際に `tools_capable` として下流へ渡す。新規 IPC 引数 `activated_skill_names: Vec<String>` を追加。既存の `spawn_blocking`（素材 id 検証 + 設定読み込み）ブロックに `let skills = crate::plugin::discover_skills(&root, &home);` を足し、戻り値タプルに `skills` を追加。`application::chat(...)` 呼び出しへ `tools`（IPC の bool）と `skills` と `activated_skill_names` を渡す。
- [ ] `workshop::application` の既存テスト（`conversation_use_cases_round_trip_in_active_root` 等、`chat` を直接呼ばないもの）は無影響のはず。`chat` 自体は実モデル依存で無テストのまま（既存方針を継承、設計の「テスト/検証」節参照）。
- [ ] `cargo test`: green（Ollama 挙動不変、`tools_capable=false` かつ `skills` 空のときの既存動作と完全一致することを確認）。

### Task 8: フロント `shared/api` — `Skill` 型 + `PluginApi` + アダプタ

**Files:** `frontend/src/shared/api/types.ts`, `frontend/src/shared/api/backend.ts`, `frontend/src/shared/api/tauri.ts`, `frontend/src/shared/api/fake.ts`

- [ ] `types.ts`: `SkillSource = "kb" | "user"`、`Skill = { name: string; description: string; body: string; location: string; source: SkillSource; hasScripts: boolean }`。`PluginApi = { listSkills(): Promise<Skill[]> }`。`Backend` に `plugin: PluginApi` を追加。`WorkshopApi.chat` の引数リストへ `activatedSkillNames: string[]` を追加（`tools` の後、`onPhase` の前）。
- [ ] `backend.ts`: `pluginApi = { listSkills: () => current.plugin.listSkills() }` を追加。`workshopApi.chat` の委譲へ `activatedSkillNames` を通す。
- [ ] `tauri.ts`: `plugin: { listSkills: () => invoke<Skill[]>("plugin_list_skills") }` を追加。`workshop.chat` の `invoke` 呼び出しに `activatedSkillNames` を足す。
- [ ] `fake.ts`: `plugin: { listSkills: async () => [] }` を追加（他の一覧系 fake と同じ「空状態」約定）。`workshop.chat` は既存どおり `desktopOnly()`（引数追加のみ、シグネチャ整合）。
- [ ] `bun run lint`: green（型整合の確認）。

### Task 9: フロント `features/plugin` — 技能一覧 UI

**Files:** new `frontend/src/features/plugin/index.ts`, new `frontend/src/features/plugin/ui/skill-panel.tsx`

- [ ] `ui/skill-panel.tsx`: props 駆動の提示コンポーネント（`features/workshop/ui/source-chip.tsx` 等に倣う）。props: `skills: Skill[]`, `activatedNames: string[]`, `onActivate: (name: string) => void`。各行に `source` バッジ（KB / user）、`hasScripts` のときは「本バージョンはスクリプトを実行しません」注記、既に `activatedNames` に含まれる名前はチェック済み/発動済み表示にして再クリックを抑止（重複排除は表示側でも親状態側でも保証、二重防御）。
- [ ] `index.ts`: `SkillPanel` を公開面としてエクスポート。
- [ ] コンポーネントテスト（`bun:test` + 描画確認、他 workshop ui コンポーネントのテスト有無に準拠 — 既存に UI コンポーネント単体テストの慣習が薄い場合はロジック部分のみ確認できる形に留める）。
- [ ] `bun run lint`: green。

### Task 10: フロント `features/workshop` — `activatedSkillNames` 状態 + 組み込み

**Files:** `frontend/src/features/workshop/model/workshop-session.ts`, `frontend/src/features/workshop/model/workshop-run.ts`, `frontend/src/features/workshop/ui/workshop-view.tsx`

- [ ] `workshop-session.ts`: 会話状態に `activatedSkillNames: string[]`（`sourceIds` と同格）を追加。`pluginApi.listSkills()` を読み込み、`skills: Skill[]` をスナップショットへ持たせる。`activateSkill(name: string)`（重複排除して追加）を公開。
- [ ] `workshop-run.ts`（または該当するストリーム処理箇所）: `ChatPhase { phase: "toolResult", name: "activate_skill", ... }` を観測したら `activateSkill(name)` 相当の更新を呼ぶ（`summary` に技能名が乗るか、`toolCall` 時点の `args` から `name` を拾うかは実装時に `ToolCall.args` の JSON を見て確定 — `activate_skill` の呼び出し引数 `{"name": "..."}` から取るのが確実）。
- [ ] `workshop-session.ts` の `chat()` 呼び出しへ `activatedSkillNames` を渡す配線を追加。
- [ ] `workshop-view.tsx`: `features/plugin` の `SkillPanel` を composition で組み込み（`skills`, `activatedSkillNames`, `activateSkill` を渡す）。
- [ ] 既存 `workshop-run.test.ts` / `workshop-session.test.ts` を新シグネチャ（`activatedSkillNames` 引数追加）に合わせて更新し、`activate_skill` のツール結果観測 → 状態反映のテストケースを追加。
- [ ] `bun run lint` + `bun test`（`frontend/` 配下）: green。

### Task 11: 検証・コミット・PR

- [ ] `bun run test`（cargo）+ `bun run lint` + `bun run --cwd frontend build` を全て green で確認。
- [ ] 受け入れ条件を手動確認（設計の「仮定/要検証」に記した `canGenerate` ギャップに注意 — tools 対応モデルでの catalog + `activate_skill` 動作、KB `skills/` と `~/.agents/skills/` の同名優先、description 欠落スキルのスキップを確認）。
- [ ] タスクごと、または論理単位でコミット。
- [ ] `gh pr create` で PR（Issue #41 を参照）。
