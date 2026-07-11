# Agent への MCP / Agent Skills 対応 — 調査メモ

**日付**: 2026-07-11
**種別**: 調査(設計稿ではない。設計は grilling → spec で別途起こす)
**対象**: `src-tauri/src/agent/`(rig-core 0.39.0 駆動、Ollama / llama.app)に MCP クライアント対応と Agent Skills 対応を足すには何が必要か。

---

## 1. 背景(現状の再確認)

- エージェントループは `agent::infrastructure::runner::run` が rig の `stream_chat().multi_turn()` を回す。ツールは呼び出し側が `Vec<Box<dyn ToolDyn>>` で**注入**する(`src-tauri/src/agent/infrastructure/runner.rs:34-44`)。
- 業務ツールは workshop が組む:`build_toolset` が read_source / search_kb / write_entry / … を `Vec<Box<dyn rig_core::tool::ToolDyn>>` として返し、破壊的ツールは `Arc<ConfirmGate>` でユーザー確認を挟む(`src-tauri/src/workshop/infrastructure/tools.rs:37-55`)。
- 確認フローは `StreamProgress::ConfirmRequest` でフロントに尋ね、`workshop_confirm` で回填する既存機構がある(`src-tauri/src/agent/domain.rs:32`)。
- 独自 Provider trait は作らない方針。rig の `Tool` / `ToolDyn` が抽象の口(`docs/superpowers/specs/2026-07-01-agent-provider-decoupling-design.md` の確定決定 2)。
- モデル能力検出:Ollama `/api/show` の capabilities から `tools` / `thinking` を判定済み(`src-tauri/src/agent/infrastructure/ollama.rs:33-64`)。

つまり **「ツールを外から差す縫い目」は既にある**。MCP はこの縫い目に第三者のツールを差す話、Skills はシステムプロンプトと(必要なら)ツールの話であり、どちらもアーキテクチャの作り直しは要らない。

---

## 2. MCP(Model Context Protocol)調査

### 2.1 仕様の現状

- 現行仕様バージョンは **2025-11-25**(日付ベース版号、後方互換の変更では上げない)。
  出典: <https://modelcontextprotocol.io/specification/versioning>
- メッセージは JSON-RPC 2.0。Host(LLM アプリ)/ Client(Host 内のコネクタ)/ Server(能力提供者)の三者構成。
  出典: <https://modelcontextprotocol.io/specification/2025-11-25/index>

### 2.2 クライアントのライフサイクル

出典: <https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle>

1. **initialize**(必ず最初):client が `protocolVersion` / `capabilities` / `clientInfo` を送り、server が自分の `capabilities`(例 `tools: { listChanged: true }`)と `serverInfo` を返す。
2. client が `notifications/initialized` を送って運用フェーズへ。
3. バージョン交渉:client は対応する最新版を送り、server が非対応なら別版を返す。合意できなければ client は切断 SHOULD。
4. 終了(stdio):client が **stdin を閉じ → 待って SIGTERM → それでも駄目なら SIGKILL**。専用の shutdown メッセージは無い。
5. 全リクエストにタイムアウトを設ける SHOULD(超過時は cancellation 通知)。

### 2.3 tools/list と tools/call の形

出典: <https://modelcontextprotocol.io/specification/2025-11-25/server/tools>

- `tools/list` → `{ tools: [{ name, title?, description, inputSchema, outputSchema?, annotations?, … }], nextCursor? }`。ページネーションあり。`inputSchema` は JSON Schema(既定 2020-12)。
- `tools/call` → params `{ name, arguments }`、result `{ content: [{type:"text",…} | image | audio | resource_link | resource], isError?, structuredContent? }`。
- ツール実行エラーはプロトコルエラーではなく `isError: true` の結果として返り、**client はそれを LLM に渡して自己修正させる SHOULD**。
- server が `listChanged` を宣言していれば `notifications/tools/list_changed` が飛ぶ(client は再 list)。

### 2.4 トランスポート

出典: <https://modelcontextprotocol.io/specification/2025-11-25/basic/transports>

| | stdio | Streamable HTTP |
|---|---|---|
| 起動 | client が server を**子プロセスとして起動** | server は独立プロセス、単一 MCP エンドポイント(POST/GET) |
| 枠組 | 改行区切り JSON-RPC(埋め込み改行禁止)。stdout は MCP メッセージ専用、stderr はログ用 | POST ごとに 1 メッセージ。応答は `application/json` か `text/event-stream`(SSE)。client は両対応 MUST |
| セッション | プロセス生存期間 | `MCP-Session-Id` ヘッダ(server が発行したら以後全リクエストに付与 MUST)。`MCP-Protocol-Version` ヘッダも以後必須 |
| 安全 | — | server は `Origin` 検証 MUST(DNS rebinding 対策)、localhost バインド SHOULD |

「Clients **SHOULD** support stdio whenever possible.」— stdio が第一級。

### 2.5 設定ファイルの業界慣例

- **Claude Desktop** `claude_desktop_config.json`(macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`):

  ```json
  { "mcpServers": { "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/username/Desktop"],
      "env": { "BRAVE_API_KEY": "..." } } } }
  ```

  出典: <https://modelcontextprotocol.io/docs/develop/connect-local-servers>
- **Claude Code** `.mcp.json`(project scope、VCS にコミットする想定)も同じ `mcpServers` キー。stdio 型は `command`/`args`/`env`、リモート型は `{ "type": "http", "url": "…", "headers": {…} }`。`${VAR}` / `${VAR:-default}` の環境変数展開に対応。**プロジェクト由来のサーバは使用前にユーザー承認を求める**。
  出典: <https://code.claude.com/docs/en/mcp>

→ 独自形式を発明せず `mcpServers` 互換の JSON(または既存慣行どおり `~/.expertBase/` の TOML に写像)を採るのが妥当。互換形式ならユーザーが他クライアントの設定を流用できる。

### 2.6 安全・ユーザー同意の要求

- 仕様トップの Key Principles:「**Hosts must obtain explicit user consent before invoking any tool**」「descriptions of tool behavior such as annotations should be considered untrusted」。
  出典: <https://modelcontextprotocol.io/specification/2025-11-25/index>(Security and Trust & Safety)
- tools ページ:「there **SHOULD** always be a human in the loop with the ability to deny tool invocations」、client は「Prompt for user confirmation on sensitive operations」「Show tool inputs to the user before calling the server」「Implement timeouts for tool calls」「Log tool usage for audit purposes」SHOULD。
  出典: <https://modelcontextprotocol.io/specification/2025-11-25/server/tools>(Security Considerations)

→ 既存 `ConfirmGate`(破壊的ツールの実行前確認)と同型の要求。MCP ツールは第三者コードなので、**確認の既定を「全ツール確認」側に倒し、ユーザーが許可を緩める**方向が仕様の趣旨に合う。

### 2.7 Rust 実装の選択肢

#### rmcp(公式 Rust SDK)

- リポジトリ: <https://github.com/modelcontextprotocol/rust-sdk>(Model Context Protocol org 公式)
- 最新版 **2.2.0**(2026-07-08)。1.7.0 は 2026-05-13、2.0.0 は 2026-06-29(crates.io API `https://crates.io/api/v1/crates/rmcp/versions` で確認)。
- クライアント作成(README の例):

  ```rust
  let client = ().serve(TokioChildProcess::new(Command::new("npx").configure(|cmd| {
      cmd.arg("-y").arg("@modelcontextprotocol/server-everything");
  }))?).await?;
  ```

  `serve()` が initialize 交渉まで済ませる。`client.list_all_tools().await?` / `client.call_tool(…)`。
  出典: <https://github.com/modelcontextprotocol/rust-sdk>(README)
- トランスポート:stdio は `TokioChildProcess`、HTTP は `StreamableHttpClientTransport`。cargo feature は `client` + `transport-child-process` + `transport-streamable-http-client-reqwest`。
  出典: <https://docs.rs/rmcp/latest/rmcp/>
- 1.7.0 の型名:ツール呼び出しパラメータは `CallToolRequestParams`(`CallToolRequestParam` は deprecated alias)、ツール定義は `rmcp::model::Tool`。
  出典: <https://docs.rs/rmcp/1.7.0/rmcp/model/index.html>

#### rig-core 0.39.0 の公式 MCP 統合(重要)

docs.rs 上の rig-core 0.39.0 の Cargo.toml で確認:

```toml
[dependencies.rmcp]
version = "1.7.0"          # caret 指定 = 1.x 系のみ(2.x は不可)
features = ["client"]
optional = true

[features]
rmcp = ["dep:rmcp"]
```

出典: <https://docs.rs/crate/rig-core/0.39.0/source/Cargo.toml>

`rmcp` feature を点けると `rig_core::tool::rmcp` モジュールが生え、そこに:

- **`McpTool::from_mcp_server(definition: rmcp::model::Tool, client: ServerSink) -> Self`** — MCP ツール定義 + サーバ送信ハンドルから rig ツールを作る。既定タイムアウト付き、`with_timeout` で調整可。
- **`McpTool` は `ToolDyn` を直接実装**(`name` / `definition` / `call`)。つまり **既存の `build_toolset` が返す `Vec<Box<dyn ToolDyn>>` にそのまま混ぜられる**。
- `McpClientHandler` — `notifications/tools/list_changed` を受けてツール一覧を自動更新するハンドラ(`McpClientHandler::new(client_info, tool_server_handle)` → `handler.connect(transport)`)。詳細 API は**未検証**(docs.rs の当該ページは要精読)。

出典: <https://docs.rs/rig-core/0.39.0/rig_core/tool/rmcp/index.html>, <https://docs.rs/rig-core/0.39.0/rig_core/tool/rmcp/struct.McpTool.html>

**含意**: 自前アダプタ層は不要。制約は rig が rmcp を **1.x に固定**していること(2.x の新機能・修正は rig の追従待ち)。

---

## 3. Agent Skills 調査

### 3.1 仕様(オープン標準)

出典: <https://agentskills.io/specification>

- スキル = `SKILL.md` を含むディレクトリ。任意で `scripts/` `references/` `assets/`。
- frontmatter(YAML):

  | フィールド | 必須 | 制約 |
  |---|---|---|
  | `name` | ✔ | ≤64 字、小文字英数とハイフンのみ、**親ディレクトリ名と一致** |
  | `description` | ✔ | ≤1024 字。「何をするか+いつ使うか」を書く |
  | `license` / `compatibility` / `metadata` / `allowed-tools` | — | `allowed-tools` は実験的 |

- **渐進開示(progressive disclosure)3 層**:
  1. メタデータ(name+description、スキルあたり約 100 トークン)— 起動時に全スキル分読み込む
  2. `SKILL.md` 本文(推奨 <5000 トークン、500 行未満)— スキル発動時に読み込む
  3. `scripts/` `references/` `assets/` — 本文が参照した時だけ読み込む
- スクリプトの対応言語は「**agent 実装に依存**」— スクリプト実行は仕様上の必須要件ではない。

### 3.2 クライアント(agent 側)実装ガイド

出典: <https://agentskills.io/client-implementation/adding-skills-support.md>(公式の統合ガイド。以下すべて同ページ)

1. **発見**: 起動時にスキルディレクトリを走査。慣例は project スコープ(`<project>/.agents/skills/` + クライアント固有ディレクトリ)と user スコープ(`~/.agents/skills/` 等)。`.agents/skills/` が**クロスクライアント互換の事実標準**。name 衝突は project > user。
2. **信頼境界**: 「project レベルのスキルは信頼できないリポジトリ由来かもしれない。**ユーザーがフォルダを信頼済みと印を付けた場合のみ読み込む**ことを検討せよ」(プロンプト注入対策)。
3. **解析**: frontmatter から `name` / `description` を抜く。壊れた YAML は寛容に(description 欠落だけはスキップ)。保存するのは `name` / `description` / `location`(SKILL.md への絶対パス)の 3 点で十分。
4. **開示(tier 1)**: カタログをシステムプロンプトに注入(XML/JSON/箇条書き何でも可)+ 使い方の短い指示文。スキルが 0 個なら**カタログ自体を出さない**。

   ```xml
   <available_skills>
     <skill><name>pdf-processing</name><description>…</description><location>…</location></skill>
   </available_skills>
   ```

5. **発動(tier 2)**: 2 方式。
   - **file-read 発動**: モデルが自前の file-read ツールで SKILL.md を読む(汎用 file-read ツールが要る)。
   - **専用ツール発動**: `activate_skill(name)` ツールを登録し本文を返す。**`name` 引数をスキル名の enum に制約すると幻覚を防げる**。frontmatter は剥がして本文だけ返す実装が多数派。構造化タグ(`<skill_content name=…>`)で包み、同梱リソースは一覧だけ返して先読みしない。
   - **ユーザー明示発動**: スラッシュコマンド等でハーネスが直接注入する経路も推奨(「モデルの判断を待たない」)。
6. **文脈管理**: スキル本文は会話圧縮(compaction)から**保護する**。同一スキルの再注入は去重。

### 3.3 スクリプト実行と安全

出典: <https://agentskills.io/skill-creation/using-scripts.md>

- スクリプトは非対話シェルで走る前提(対話プロンプト禁止)。`uv run`(PEP 723)/ `npx` 等で自己完結。
- 公式ガイドに**サンドボックス必須の規定は無い**。安全側の言及は (a) project スキルの信頼ゲート(3.2)、(b) 破壊的操作に `--confirm` / `--dry-run` を推奨、の 2 点。実行環境の隔離は実装者の責務。
- **含意**: ExpertBase には現在シェル実行ツールが無い。スクリプト対応=「任意コマンド実行ツールを新設する」ことと同義であり、安全面の影響が最も大きい部分。仕様上は**スクリプト無しでも skills 対応と言える**(tier 1+2 のみ)。

### 3.4 ローカル小型モデルへの適合(当方の評価、出典なし)

- 発動はモデルの判断頼み(catalog を読んで自分で activate する)。Ollama の小型モデルでは取りこぼしが予想される。
- 緩和策はガイド内に既にある: 専用 `activate_skill` ツール + enum 制約、およびユーザー明示発動(UI からスキルを選んで注入)。後者はモデル能力に依存しないので、小型モデル前提の本製品では**ユーザー明示発動を第一級にする**のが理にかなう。
- ツール無しモデル(`tools` capability 無し)でも、ユーザー明示発動なら本文をプロンプトに直接注入できる。

---

## 4. 統合オプションと取捨(推奨込み、最終設計は別途)

### 4.1 MCP

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **A(推奨)** | rig の `rmcp` feature + `McpTool::from_mcp_server` | アダプタ 0 行。`ToolDyn` として既存 `build_toolset` の列に混ざる。ループ・ストリーミング不変 | rmcp 1.x 固定(rig の追従待ち)。確認ゲートは別途ラッパが要る |
| B | rmcp 2.x を直接依存し、自前で `ToolDyn` アダプタを書く | 最新 rmcp、タイムアウト・通知・同意を完全制御 | rig が同梱するものの再実装。依存二重化の恐れ |
| C | JSON-RPC を手書き | 依存なし | 仕様の再実装(lifecycle/SSE/session)。論外 |

- A の補完として、**MCP ツールを `ConfirmGate` 装飾でくるむ**(`ToolDyn` を実装する薄い wrapper で `call` 前に確認を挟む)ことで仕様の「invoke 前の明示同意」を既存機構で満たせる。wrapper は自前 `Tool` 実装が既に 9 個ある `tools.rs` のパターンの延長。
- 接続ライフサイクル(子プロセスの起動/stdin クローズ/SIGTERM、HTTP セッション)は agent の infrastructure に置く。会話ごとに接続するか常駐させるかは設計論点(→ §5)。
- まず **stdio のみ**で始めるのが仕様の推奨(「SHOULD support stdio whenever possible」)とも整合。Streamable HTTP は第 2 段。

### 4.2 Skills

| 案 | 内容 | 利点 | 欠点 |
|---|---|---|---|
| **A(推奨)** | tier 1+2 のみ:起動時走査 → カタログをシステムプロンプトへ → 専用 `activate_skill` ツール(enum 制約)+ UI からの明示発動 | 小型モデルでも機能。シェル実行を導入しない=安全面の新リスクなし。仕様準拠の「skills 対応」を名乗れる | `scripts/` を使うスキルは動かない(本文指示のみ) |
| B | A + 汎用 file-read + スクリプト実行(ConfirmGate 必須) | フル互換 | 任意コマンド実行の導入。サンドボックス設計が丸ごと必要 |

- 置き場所: user スコープは `~/.agents/skills/`(相互運用の事実標準)+ アプリ固有ディレクトリ、の 2 本走査が慣例。ExpertBase は「プロジェクト」概念が KB なので、KB 内 `skills/` を project 相当にするかは論点(→ §5)。
- プロンプト注入位置: `workshop/prompt.rs` が既にシステムプロンプト(preamble)の所有者なので、カタログ組み立てはそこに合流させるのが自然。

### 4.3 DDD レイヤ写像(叩き台)

| レイヤ | MCP | Skills |
|---|---|---|
| agent/domain | サーバ設定の値(name, transport, command/args/env or url)、同意ポリシー値 | スキルメタ(name/description/location)と frontmatter 検証規則 |
| agent/infrastructure | rmcp 接続管理(spawn/serve/shutdown)、`McpTool` 列の取得、設定永続化(`settings_store` の隣) | ディレクトリ走査・SKILL.md 解析 |
| workshop(application) | `build_toolset` に MCP ツール列を合流、ConfirmGate 装飾 | カタログをプロンプトへ、`activate_skill` ツール登録 |
| interface | サーバ CRUD・接続テストの Tauri コマンド | スキル一覧・有効/無効の Tauri コマンド |

- 「pluginization」はトップレベル原則(AGENTS.md)。**MCP + Skills がそのままプラグイン機構の実体**になり得る — 独自プラグイン形式を発明する前にこの 2 標準で足りるか、を設計段階で判断すべき。

---

## 5. 未決問題(grilling 用)

1. **スコープ**: MCP と Skills を同時にやるか、どちらか先行か。ユーザー価値が先に立つのはどちらか。
2. **MCP トランスポート**: stdio のみで v1 とするか。Streamable HTTP(リモートサーバ、認証ヘッダ)まで含めるか。
3. **接続ライフサイクル**: MCP サーバは(a)アプリ起動時に常駐接続、(b)会話開始時に接続、(c)ツール呼び出し時に lazy 接続のどれか。子プロセスの寿命管理と UI 表示に直結。
4. **同意モデル**: 仕様は「invoke 前の明示同意」。全 MCP ツール毎回確認は UX が重い。サーバ単位の信頼付与+破壊的そうなツールだけ毎回確認、のような段階を作るか。`annotations`(readOnlyHint 等)は**信頼できない**とされる点に注意。
5. **設定形式**: `mcpServers` 互換 JSON を直接読むか、既存の `~/.expertBase/ai.toml` 方式に写像するか。設定 UI をどこまで作るか(手書き JSON で v1 は許容か)。
6. **Skills の置き場所**: `~/.agents/skills/` 互換を採るか、KB ディレクトリ内 `skills/` を第一級にするか、両方か。KB 同期(将来の有償機能)にスキルを含めるか。
7. **スクリプト実行**: v1 で切り捨てで良いか(推奨は切り捨て)。切り捨てる場合、`scripts/` を参照するスキルをどう扱うか(警告表示?)。
8. **小型モデルの発動信頼性**: モデル自動発動をどこまで信じるか。UI 明示発動(スキル選択)を主経路にするか。
9. **モデル能力ゲート**: `tools` capability の無いモデルでは MCP ツールも `activate_skill` も載せられない。その場合のフォールバック(Skills はユーザー明示注入のみ、MCP は無効)で良いか。
10. **rmcp 1.x 固定**: rig 経由(1.x)で始めて問題が出たら直接依存に切り替える、で良いか。切り替えコストは `McpTool` 相当の wrapper 1 枚。

---

## 6. 出典一覧

### MCP(一次資料)

- 仕様バージョニング: <https://modelcontextprotocol.io/specification/versioning>
- 仕様トップ(Security and Trust & Safety): <https://modelcontextprotocol.io/specification/2025-11-25/index>
- ライフサイクル: <https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle>
- ツール: <https://modelcontextprotocol.io/specification/2025-11-25/server/tools>
- トランスポート: <https://modelcontextprotocol.io/specification/2025-11-25/basic/transports>
- Claude Desktop 設定: <https://modelcontextprotocol.io/docs/develop/connect-local-servers>
- Claude Code `.mcp.json`: <https://code.claude.com/docs/en/mcp>

### Rust SDK / rig

- rmcp リポジトリ(公式 SDK): <https://github.com/modelcontextprotocol/rust-sdk>
- rmcp docs: <https://docs.rs/rmcp/latest/rmcp/>(バージョン履歴は crates.io API で確認、最新 2.2.0 / 2026-07-08)
- rmcp 1.7.0 model モジュール: <https://docs.rs/rmcp/1.7.0/rmcp/model/index.html>
- rig-core 0.39.0 Cargo.toml(rmcp 1.7.0 pin): <https://docs.rs/crate/rig-core/0.39.0/source/Cargo.toml>
- rig-core 0.39.0 rmcp 統合: <https://docs.rs/rig-core/0.39.0/rig_core/tool/rmcp/index.html>
- `McpTool`: <https://docs.rs/rig-core/0.39.0/rig_core/tool/rmcp/struct.McpTool.html>

### Agent Skills(一次資料)

- 仕様: <https://agentskills.io/specification>
- クライアント実装ガイド: <https://agentskills.io/client-implementation/adding-skills-support.md>
- スクリプト利用: <https://agentskills.io/skill-creation/using-scripts.md>

### 本リポジトリ

- `src-tauri/src/agent/infrastructure/runner.rs:34-44`(ツール注入の縫い目)
- `src-tauri/src/workshop/infrastructure/tools.rs:37-55`(`build_toolset` + ConfirmGate)
- `src-tauri/src/agent/domain.rs:32`(ConfirmRequest 進捗)
- `docs/superpowers/specs/2026-07-01-agent-provider-decoupling-design.md`(rig を抽象の口とする確定決定)

### 未検証事項

- `McpClientHandler`(rig 0.39.0)の正確な API と、`list_changed` 自動更新をどう組み込むか(docs.rs 要精読)。
- rig の `McpTool` 引数 `ServerSink` の正確な型経路(rmcp 側の対サーバ送信ハンドル。`Peer<RoleClient>` 系のはずだが未確認)。
- rmcp 1.x → 2.x の破壊的変更の内訳(移行ガイドの精読は rig を 2.x 対応させる必要が出た時で良い)。
