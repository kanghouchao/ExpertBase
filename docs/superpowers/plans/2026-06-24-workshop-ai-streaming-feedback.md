# Workshop AI 阶段流式反馈 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 workshop 的 AI 草稿调用不再冻结 UI，并在等待期间向右侧状态栏实时上报「加载模型中 → 生成中(N 字) → 完成/错误」的阶段反馈。

**Architecture:** 把同步阻塞的 `workshop_draft` 命令改为 `async fn` + `spawn_blocking`（移出主线程，解冻 UI），Ollama 调用改 `stream:true` 在 Rust 侧逐块消费 NDJSON，通过 Tauri `Channel<DraftEvent>` 把阶段事件推给前端（复用 `asr::interface::transcribe_material` 已确立的「Tauri 非依赖回调 → Channel」模式）。前端用一个确定性状态机（idle/connecting/loadingModel/generating/done）驱动 spinner 与 inspector 状态栏。

**Tech Stack:** Tauri 2（`tauri::ipc::Channel`、`async_runtime::spawn_blocking`）、reqwest blocking 流式读取、serde、Next.js/React、Bun、node --test。

## 设计依据（来自知识库 `技术文档/AI/Agent/`）

| 原则 | 出处 | 本方案落点 |
|---|---|---|
| 心跳/Query Loop 以**事件流**消费模型输出（drive-schedule-feedback），不是等最终文本块 | [[Harness Engineering 设计原则]] Ch3 | Ollama `stream:true` + `consume_chat_stream` 逐块上报 |
| **错误路径是主路径**，失败要「看起来像系统行为」 | [[Harness Engineering 设计原则]] Ch6 | 结构化错误信息带可执行 suggestion；阶段状态机显式化 |
| **优雅降级，禁止静默失败**；错误带 suggestion，永不返回 None | [[AI Agent 工程实践]] 生产清单 | 连接/超时/404 各自带「如何修」的提示 |
| **能确定就确定**：确定层(FS/FTS/状态机)与模糊层(LLM)分离 | [[Fuzzy Engineering 范式]] | NDJSON 解析与前端状态机抽成纯函数并单测 |

## Global Constraints

- 代码注释与文档用**日文**（AGENTS.md 语言政策）。
- Tauri 命令签名返回 `Result<T, String>`，内部错误 `map_err(|e| e.to_string())`（src-tauri/AGENTS.md）。
- 跨 IPC 边界的 struct/enum 派生 `Serialize` + `#[serde(rename_all = "camelCase")]`，与 TS 客户端一致。
- 后端遵守 DDD 分层：domain 不依赖 Tauri/HTTP；`Channel` 只出现在 interface 层；application 通过纯回调 `&mut dyn FnMut(StreamProgress)` 传递进度。
- 前端遵守 FSD：IPC 调用只在 `shared/api/tauri/client.ts`；纯逻辑放 `features/workshop/model/process-state.ts`。
- TDD：每个行为改动先写失败测试再实现。
- 缩进：Rust 2 空格，不要 `cargo fmt` 全文件。
- 验证命令：`bun run test`（cargo test）、`node --test scripts/*.test.mjs`、`bun run --cwd frontend lint`。
- `workshop_confirm` 保持同步（本地 FS+SQLite，很快），不在本计划范围。

---

### Task 1: domain/application — 给 AiProvider 接入进度回调（纯插管，不改行为）

**Files:**
- Modify: `src-tauri/src/ai/domain.rs`
- Modify: `src-tauri/src/ai/mod.rs`
- Modify: `src-tauri/src/ai/infrastructure/ollama.rs`（仅签名 + 一次 LoadingModel 上报）
- Modify: `src-tauri/src/workshop/application.rs`

**Interfaces:**
- Produces:
  - `enum StreamProgress { LoadingModel, Generating { chars: usize } }`（domain，`Clone + Debug + PartialEq`，无 Tauri 依赖）
  - `AiProvider::structure(&self, req: StructureRequest, on_progress: &mut dyn FnMut(StreamProgress)) -> Result<StructureResult, AiError>`
  - `application::draft(provider, conn, source_text, messages, on_progress: &mut dyn FnMut(StreamProgress)) -> Result<StructureResult, AiError>`

- [ ] **Step 1: 写失败测试（domain：FakeProvider 上报进度）**

在 `src-tauri/src/ai/domain.rs` 的 `mod tests` 内追加：

```rust
  #[test]
  fn fake_provider_reports_loading_then_generating() {
    let req = StructureRequest {
      source_text: "abc".into(),
      related: vec![],
      messages: vec![],
    };
    let mut events = Vec::new();
    FakeProvider.structure(req, &mut |p| events.push(p)).unwrap();
    assert_eq!(events.first(), Some(&StreamProgress::LoadingModel));
    assert!(matches!(events.last(), Some(StreamProgress::Generating { chars: 3 })));
  }
```

- [ ] **Step 2: 跑测试确认失败（编译错误：签名不匹配 / 类型未定义）**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ai::domain`
Expected: FAIL（`StreamProgress` 未定义、`structure` 参数数量不符）

- [ ] **Step 3: 实现 domain 改动**

在 `src-tauri/src/ai/domain.rs` 中 `StructureResult` 定义之后、`AiError` 之前插入：

```rust
/// ストリーミング進捗。Tauri 非依存のドメイン値（interface 層が Channel へ橋渡しする）。
#[derive(Clone, Debug, PartialEq)]
pub enum StreamProgress {
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// トークン受信中。chars は累積文字数。
  Generating { chars: usize },
}
```

把 trait 改为：

```rust
pub trait AiProvider {
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError>;
}
```

把 `FakeProvider` 的 impl 改为（上报 LoadingModel → Generating）：

```rust
#[cfg(test)]
impl AiProvider for FakeProvider {
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    on_progress(StreamProgress::LoadingModel);
    let title = req.source_text.lines().next().unwrap_or("").trim().to_string();
    on_progress(StreamProgress::Generating { chars: req.source_text.chars().count() });
    let suggested_links = req.related.iter().take(3).map(|e| e.title.clone()).collect();
    Ok(StructureResult {
      kind: "entry".into(),
      title: if title.is_empty() { "無題".into() } else { title },
      cat: "uncategorized".into(),
      body_markdown: req.source_text.clone(),
      suggested_links,
    })
  }
}
```

更新既有测试 `fake_provider_echoes_and_suggests_related_links`，把调用改成：

```rust
    let res = FakeProvider.structure(req, &mut |_| {}).unwrap();
```

- [ ] **Step 4: 导出 StreamProgress**

在 `src-tauri/src/ai/mod.rs` 把 re-export 行改为：

```rust
pub use domain::{
  AiError, AiProvider, ChatTurn, EntrySummary, StreamProgress, StructureRequest, StructureResult,
};
```

- [ ] **Step 5: 让 ollama 与 application 跟上新签名（ollama 暂不流式）**

`src-tauri/src/ai/infrastructure/ollama.rs`：把 `impl AiProvider for OllamaProvider` 的方法签名改为带 `on_progress`，并在 `.send()` 之前插一行 `on_progress(StreamProgress::LoadingModel);`（其余逻辑保持 `stream:false` 不动）：

```rust
impl AiProvider for OllamaProvider {
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    let model = match &self.model {
      Some(model) => model.clone(),
      None => Self::first_local_model()?
        .ok_or_else(|| AiError::Other("Ollama 没有可用模型，请先下载模型".into()))?,
    };
    let body = build_body(&model, &req);
    let client = reqwest::blocking::Client::builder()
      .timeout(Duration::from_secs(120))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))?;
    on_progress(StreamProgress::LoadingModel);
    let resp = client
      .post(format!("{}/api/chat", self.base_url))
      .header("content-type", "application/json")
      .json(&body)
      .send()
      .map_err(|e| AiError::Network(e.to_string()))?;

    let status = resp.status();
    let text = resp.text().map_err(|e| AiError::Network(e.to_string()))?;
    match status.as_u16() {
      200 => parse_response(&text),
      404 => Err(AiError::Other(format!("Ollama 模型未找到: {model}"))),
      _ => Err(AiError::Other(format!("Ollama API 错误({status}): {text}"))),
    }
  }
}
```

在文件顶部 `use` 里把 `StreamProgress` 加上：

```rust
use crate::ai::domain::{AiError, AiProvider, StreamProgress, StructureRequest, StructureResult};
```

`src-tauri/src/workshop/application.rs`：把 `draft` 改为透传回调：

```rust
pub fn draft<P: AiProvider>(
  provider: &P,
  conn: &Connection,
  source_text: &str,
  messages: Vec<ChatTurn>,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<StructureResult, AiError> {
  let related = related_entries(conn, source_text, 5).map_err(AiError::Other)?;
  provider.structure(
    StructureRequest { source_text: source_text.to_string(), related, messages },
    on_progress,
  )
}
```

把文件顶部 `use crate::ai::{...}` 里加入 `StreamProgress`：

```rust
use crate::ai::{AiError, AiProvider, ChatTurn, EntrySummary, StreamProgress, StructureRequest, StructureResult};
```

更新既有测试 `draft_with_fake_provider_returns_result`，把调用改为：

```rust
    let result = draft(&FakeProvider, &conn, "新しい 淹れ方 の本文", messages, &mut |_| {}).unwrap();
```

并在 `application.rs` 的 `mod tests` 追加透传测试：

```rust
  #[test]
  fn draft_forwards_streaming_progress() {
    let conn = Connection::open_in_memory().unwrap();
    index::ensure_schema(&conn).unwrap();
    let mut events = Vec::new();
    draft(&FakeProvider, &conn, "本文", vec![], &mut |p| events.push(p)).unwrap();
    assert!(events.contains(&StreamProgress::LoadingModel));
  }
```

- [ ] **Step 6: 跑全部 Rust 测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS（原 64 个 + 新增 2 个，全绿）

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/ai/domain.rs src-tauri/src/ai/mod.rs src-tauri/src/ai/infrastructure/ollama.rs src-tauri/src/workshop/application.rs
git commit -m "feat(ai): thread StreamProgress callback through AiProvider"
```

---

### Task 2: ollama — 真·流式消费 NDJSON + 结构化错误带 suggestion

**Files:**
- Modify: `src-tauri/src/ai/infrastructure/ollama.rs`

**Interfaces:**
- Consumes: `StreamProgress`（Task 1）
- Produces:
  - `fn consume_chat_stream(lines: impl Iterator<Item = std::io::Result<String>>, on_progress: &mut dyn FnMut(StreamProgress)) -> Result<StructureResult, AiError>`（私有，可单测）
  - `build_body` 输出 `"stream": true`

- [ ] **Step 1: 写失败测试（流式解析器累积并上报进度）**

在 `src-tauri/src/ai/infrastructure/ollama.rs` 的 `mod tests` 追加：

```rust
  #[test]
  fn consume_chat_stream_accumulates_content_and_reports_progress() {
    // Ollama /api/chat stream=true は NDJSON。各行の message.content を連結すると JSON 本体になる。
    let lines = vec![
      Ok(r#"{"message":{"content":"{\"kind\":\"entry\",\"title\":\"緑"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"茶\",\"cat\":\"tea\",\"body_markdown\":\"本文\",\"suggested_links\":[]}"},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let result = consume_chat_stream(lines.into_iter(), &mut |p| events.push(p)).unwrap();
    assert_eq!(result.kind, "entry");
    assert_eq!(result.title, "緑茶");
    assert_eq!(result.cat, "tea");
    // チャンク毎に Generating（累積文字数）が上報される
    assert!(matches!(events.first(), Some(StreamProgress::Generating { .. })));
    assert_eq!(events.len(), 2);
  }

  #[test]
  fn build_body_streams() {
    let body = build_body("llama3.2", &req());
    assert_eq!(body["stream"], true);
  }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ollama`
Expected: FAIL（`consume_chat_stream` 未定义；`build_body_streams` 断言 false≠true）

- [ ] **Step 3: 实现流式**

在 `ollama.rs` 把 `build_body` 的 `"stream": false` 改为 `"stream": true`。

删除 `ChatResponse`、`parse_response`，以及测试 `parse_response_extracts_entry_result` / `parse_response_extracts_chat_reply`（流式替代了一次性解析；这是本改动产生的孤儿，按 surgical 原则一并移除）。新增 `StreamChunk` 与 `consume_chat_stream`（放在 `RawResult` 定义之后）：

```rust
#[derive(Deserialize)]
struct StreamChunk {
  message: ChatMessage,
  #[serde(default)]
  done: bool,
}

/// NDJSON ストリーム（Ollama /api/chat stream=true）を消費し、本文を連結して構造化結果へ。
/// チャンク毎に累積文字数を Generating として上報する。format スキーマ固定なので、
/// 連結後の本文は 1 つの JSON オブジェクトになる。
fn consume_chat_stream(
  lines: impl Iterator<Item = std::io::Result<String>>,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<StructureResult, AiError> {
  let mut acc = String::new();
  for line in lines {
    let line = line.map_err(|e| {
      // 本文読み取り中のタイムアウトはモデルのロード/生成が長いケース。提案つきで返す。
      AiError::Network(format!(
        "读取 Ollama 响应失败（模型加载或生成可能超时，可先在终端 `ollama run` 预热）: {e}"
      ))
    })?;
    if line.trim().is_empty() {
      continue;
    }
    let chunk: StreamChunk =
      serde_json::from_str(&line).map_err(|e| AiError::Other(e.to_string()))?;
    acc.push_str(&chunk.message.content);
    on_progress(StreamProgress::Generating { chars: acc.chars().count() });
    if chunk.done {
      break;
    }
  }
  let raw: RawResult =
    serde_json::from_str(&acc).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(StructureResult {
    kind: raw.kind,
    title: raw.title,
    cat: raw.cat,
    body_markdown: raw.body_markdown,
    suggested_links: raw.suggested_links,
  })
}
```

把 `impl AiProvider for OllamaProvider` 的方法体改为流式 + 分层超时 + 提案つきエラー：

```rust
impl AiProvider for OllamaProvider {
  fn structure(
    &self,
    req: StructureRequest,
    on_progress: &mut dyn FnMut(StreamProgress),
  ) -> Result<StructureResult, AiError> {
    let model = match &self.model {
      Some(model) => model.clone(),
      None => Self::first_local_model()?
        .ok_or_else(|| AiError::Other("Ollama 没有可用模型，请先下载模型".into()))?,
    };
    let body = build_body(&model, &req);
    // 接続は短く（未起動を即検知）、全体は長く（モデルのロード + 生成を許容）。
    let client = reqwest::blocking::Client::builder()
      .connect_timeout(Duration::from_secs(3))
      .timeout(Duration::from_secs(180))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))?;

    on_progress(StreamProgress::LoadingModel);
    let resp = client
      .post(format!("{}/api/chat", self.base_url))
      .header("content-type", "application/json")
      .json(&body)
      .send()
      .map_err(|e| {
        if e.is_timeout() {
          AiError::Network(format!(
            "等待 Ollama 响应超时（模型可能正在加载，请先在终端执行 `ollama run {model}` 预热，或选择更小的模型）"
          ))
        } else {
          AiError::Network(format!("无法连接 Ollama（请确认 `ollama serve` 正在运行）: {e}"))
        }
      })?;

    let status = resp.status();
    if status.as_u16() == 404 {
      return Err(AiError::Other(format!(
        "Ollama 模型未找到: {model}（请先执行 `ollama pull {model}`）"
      )));
    }
    if !status.is_success() {
      let text = resp.text().unwrap_or_default();
      return Err(AiError::Other(format!("Ollama API 错误({status}): {text}")));
    }

    use std::io::BufRead;
    let reader = std::io::BufReader::new(resp);
    consume_chat_stream(reader.lines(), on_progress)
  }
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS（含新 `consume_chat_stream_*` / `build_body_streams`；已删除的 `parse_response_*` 不再存在）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ai/infrastructure/ollama.rs
git commit -m "feat(ai): stream Ollama chat and report token progress"
```

---

### Task 3: interface — workshop_draft 改 async + Channel<DraftEvent>

**Files:**
- Modify: `src-tauri/src/workshop/interface.rs`

**Interfaces:**
- Consumes: `StreamProgress`（domain）、`application::draft`（Task 1 新签名）
- Produces:
  - `enum DraftEvent { LoadingModel, Generating { chars: usize } }`（`Serialize`，`#[serde(tag = "phase", rename_all = "camelCase")]`）
  - `From<StreamProgress> for DraftEvent`
  - `async fn workshop_draft(app, inbox_paths: Vec<String>, messages: Vec<ChatTurn>, model: String, on_event: Channel<DraftEvent>) -> Result<StructureResult, String>`

- [ ] **Step 1: 写失败测试（DraftEvent 序列化契约）**

在 `src-tauri/src/workshop/interface.rs` 末尾新增测试模块：

```rust
#[cfg(test)]
mod tests {
  use super::*;
  use crate::ai::StreamProgress;

  #[test]
  fn draft_event_serializes_with_phase_tag() {
    let gen = serde_json::to_value(DraftEvent::from(StreamProgress::Generating { chars: 7 })).unwrap();
    assert_eq!(gen["phase"], "generating");
    assert_eq!(gen["chars"], 7);
    let load = serde_json::to_value(DraftEvent::from(StreamProgress::LoadingModel)).unwrap();
    assert_eq!(load["phase"], "loadingModel");
  }
}
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml workshop::interface`
Expected: FAIL（`DraftEvent` 未定义）

- [ ] **Step 3: 实现 interface（照搬 asr::interface 的 spawn_blocking + Channel 模式）**

把 `src-tauri/src/workshop/interface.rs` 顶部改为：

```rust
//! workshop インターフェイス層。Tauri コマンド（IPC アダプタ）。

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::Manager;

use crate::ai::{ChatTurn, StreamProgress, StructureResult};
use crate::kb::material;
use crate::workshop::application;

/// 草稿生成の進捗イベント。フロントの Channel へ送る（右側 status バーのフェーズ表示）。
#[derive(Clone, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum DraftEvent {
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// トークン受信中。chars は累積文字数。
  Generating { chars: usize },
}

impl From<StreamProgress> for DraftEvent {
  fn from(p: StreamProgress) -> Self {
    match p {
      StreamProgress::LoadingModel => DraftEvent::LoadingModel,
      StreamProgress::Generating { chars } => DraftEvent::Generating { chars },
    }
  }
}
```

把 `workshop_draft` 整个函数替换为 async 版（`StructureResult` 现从 `crate::ai` 引入，不再需要 `crate::ai::StructureResult` 全路径；旧的 `use crate::ai::{ChatTurn, StructureResult};` 已被上面的 use 覆盖）：

```rust
/// 複数の受信箱素材 + 会話履歴から AI 構造化（草稿 or 会話返信）を生成する。
/// KB 読み込み・FTS・Ollama 呼び出しはブロッキングなので別スレッドへ。進捗は on_event で上報。
#[tauri::command]
pub async fn workshop_draft(
  app: tauri::AppHandle,
  inbox_paths: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
  on_event: Channel<DraftEvent>,
) -> Result<StructureResult, String> {
  let home = app.path().home_dir().map_err(|e| e.to_string())?;
  let joined = tauri::async_runtime::spawn_blocking(move || -> Result<StructureResult, String> {
    let (root, conn) = crate::kb::open_active(&home)?;
    let mut bodies = Vec::with_capacity(inbox_paths.len());
    for inbox_path in &inbox_paths {
      let inbox_rel = crate::kb::checked_kb_markdown_path(inbox_path, "inbox")?;
      let raw = std::fs::read_to_string(root.join(inbox_rel)).map_err(|e| e.to_string())?;
      bodies.push(material::parse_material(&raw)?.body);
    }
    let source_text = bodies.join("\n\n---\n\n");
    let provider = crate::ai::ollama::OllamaProvider::with_model(model);
    let mut on_progress = |p: StreamProgress| {
      let _ = on_event.send(DraftEvent::from(p));
    };
    application::draft(&provider, &conn, &source_text, messages, &mut on_progress)
      .map_err(|e| e.to_string())
  })
  .await;

  match joined {
    Ok(inner) => inner,
    Err(e) => Err(e.to_string()),
  }
}
```

`workshop_confirm` 保持不变。

- [ ] **Step 4: 跑测试 + 确认命令注册无需改动**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS。`src-tauri/src/lib.rs` 的 `generate_handler!` 里 `workshop::interface::workshop_draft` 无需改（仅函数体改 async，路径不变）。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/workshop/interface.rs
git commit -m "feat(workshop): run draft off main thread and stream phase events"
```

---

### Task 4: 前端 client — workshopDraft 接 Channel<DraftPhase>

**Files:**
- Modify: `frontend/src/shared/api/tauri/client.ts`

**Interfaces:**
- Produces:
  - `type DraftPhase = { phase: "loadingModel" } | { phase: "generating"; chars: number }`
  - `workshopDraft(inboxPaths, messages, model, onPhase?: (phase: DraftPhase) => void): Promise<StructureResult>`

- [ ] **Step 1: 改造 client（薄传输层，无单测；由 lint + build 类型检查把关，照搬 transcribeMaterial 的 Channel 写法）**

`Channel` 已在文件顶部 import。在 `ChatTurn` 类型定义之后追加：

```ts
/** 草稿生成のフェーズイベント（Rust DraftEvent と一致）。 */
export type DraftPhase =
  | { phase: "loadingModel" }
  | { phase: "generating"; chars: number };
```

把现有 `workshopDraft` 替换为带可选 `onPhase` 的版本：

```ts
/** 複数の受信箱素材 + 会話履歴から AI 応答を生成する。onPhase で進捗フェーズを受け取る。 */
export async function workshopDraft(
  inboxPaths: string[],
  messages: ChatTurn[],
  model: string,
  onPhase?: (phase: DraftPhase) => void
): Promise<StructureResult> {
  const channel = new Channel<DraftPhase>();
  if (onPhase) channel.onmessage = onPhase;
  return invoke<StructureResult>("workshop_draft", { inboxPaths, messages, model, onEvent: channel });
}
```

- [ ] **Step 2: lint 确认通过**

Run: `bun run --cwd frontend lint`
Expected: 无错误。

- [ ] **Step 3: Commit**

```bash
git add frontend/src/shared/api/tauri/client.ts
git commit -m "feat(workshop): pass draft phase channel from client"
```

---

### Task 5: 前端 state — 阶段状态机纯函数 + 单测

**Files:**
- Modify: `frontend/src/features/workshop/model/process-state.ts`
- Test: `scripts/workshop-process-state.test.mjs`

**Interfaces:**
- Produces:
  - `type DraftUiPhase = "idle" | "connecting" | "loadingModel" | "generating" | "done"`
  - `isGeneratingPhase(phase: DraftUiPhase): boolean`
  - `phaseLabelKey(phase: DraftUiPhase): string`

- [ ] **Step 1: 写失败测试**

在 `scripts/workshop-process-state.test.mjs` 顶部的 import 里加入 `isGeneratingPhase, phaseLabelKey`（与现有 import 同一来源 `../frontend/src/features/workshop/model/process-state.ts`），并追加：

```js
test("isGeneratingPhase is true only while a turn is in flight", () => {
  assert.equal(isGeneratingPhase("idle"), false);
  assert.equal(isGeneratingPhase("connecting"), true);
  assert.equal(isGeneratingPhase("loadingModel"), true);
  assert.equal(isGeneratingPhase("generating"), true);
  assert.equal(isGeneratingPhase("done"), false);
});

test("phaseLabelKey maps loadingModel to its own i18n key", () => {
  assert.equal(phaseLabelKey("loadingModel"), "workshop.phase.loadingModel");
  assert.equal(phaseLabelKey("connecting"), "workshop.phase.connecting");
  assert.equal(phaseLabelKey("generating"), "workshop.phase.generating");
  assert.equal(phaseLabelKey("idle"), "workshop.st.idle");
  assert.equal(phaseLabelKey("done"), "workshop.st.done");
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `node --test scripts/workshop-process-state.test.mjs`
Expected: FAIL（`isGeneratingPhase` / `phaseLabelKey` 未导出）

- [ ] **Step 3: 实现纯函数**

在 `frontend/src/features/workshop/model/process-state.ts` 末尾追加：

```ts
// AI 草稿生成のフェーズ（確定的状態機。UI の反馈はここから駆動する）。
export type DraftUiPhase = "idle" | "connecting" | "loadingModel" | "generating" | "done";

export function isGeneratingPhase(phase: DraftUiPhase): boolean {
  return phase === "connecting" || phase === "loadingModel" || phase === "generating";
}

/** フェーズ → i18n キー（spinner ラベル / inspector status）。 */
export function phaseLabelKey(phase: DraftUiPhase): string {
  switch (phase) {
    case "connecting":
      return "workshop.phase.connecting";
    case "loadingModel":
      return "workshop.phase.loadingModel";
    case "generating":
      return "workshop.phase.generating";
    case "done":
      return "workshop.st.done";
    default:
      return "workshop.st.idle";
  }
}
```

- [ ] **Step 4: 跑测试确认通过**

Run: `node --test scripts/workshop-process-state.test.mjs`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add frontend/src/features/workshop/model/process-state.ts scripts/workshop-process-state.test.mjs
git commit -m "feat(workshop): add draft phase state machine helpers"
```

---

### Task 6: 前端 view + i18n — 把阶段反馈接进 UI

**Files:**
- Modify: `frontend/src/features/workshop/ui/workshop-process-view.tsx`
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

**Interfaces:**
- Consumes: `workshopDraft` 的 `onPhase`（Task 4）、`DraftPhase`（Task 4）、`isGeneratingPhase` / `phaseLabelKey` / `DraftUiPhase`（Task 5）

- [ ] **Step 1: 加 i18n 键**

在 `frontend/src/shared/i18n/dictionaries.ts` 中，找到每种语言里已有的 `"workshop.thinking"` 条目，在其旁边新增三条（与现有 key 风格一致）：

- 中文字典：
```ts
  "workshop.phase.connecting": "连接中…",
  "workshop.phase.loadingModel": "加载模型中…",
  "workshop.phase.generating": "生成中",
```
- 日本語字典：
```ts
  "workshop.phase.connecting": "接続中…",
  "workshop.phase.loadingModel": "モデル読み込み中…",
  "workshop.phase.generating": "生成中",
```
- English 字典：
```ts
  "workshop.phase.connecting": "Connecting…",
  "workshop.phase.loadingModel": "Loading model…",
  "workshop.phase.generating": "Generating",
```

- [ ] **Step 2: view 改用阶段状态机**

在 `frontend/src/features/workshop/ui/workshop-process-view.tsx`：

① import 增补——把 `workshopDraft` 那组 import 里加上 `type DraftPhase`，并从 process-state 引入新成员：

```ts
import {
  buildManualDraft,
  canRemoveSource,
  isGeneratingPhase,
  phaseLabelKey,
  replaceLatestEntryResult,
  sameSourceIds,
  toChatTurn,
  type DraftUiPhase,
  type ProcessMessage,
} from "../model/process-state";
```

② 状态声明——把 `const [generating, setGenerating] = useState(false);` 替换为：

```ts
  const [phase, setPhase] = useState<DraftUiPhase>("idle");
  const [genChars, setGenChars] = useState(0);
  const generating = isGeneratingPhase(phase);
```

③ `runTurn`——替换为阶段驱动版：

```ts
  // 会話履歴つきで AI を呼ぶ。entry なら草稿を更新、chat なら会話気泡として表示。
  async function runTurn(history: Msg[]) {
    setMessages(history);
    setInstruction("");
    setShowPicker(false);
    setPhase("connecting");
    setGenChars(0);
    setError(null);
    try {
      const result = await workshopDraft(
        sources.map((s) => s.id),
        history.map(toChatTurn),
        visibleSelectedModel,
        (p: DraftPhase) => {
          if (p.phase === "generating") {
            setPhase("generating");
            setGenChars(p.chars);
          } else {
            setPhase("loadingModel");
          }
        }
      );
      setMessages([...history, { role: "ai", result }]);
      if (result.kind === "entry") {
        setDraft(result);
        setDraftSourceIds(sources.map((source) => source.id));
      }
      setPhase("idle");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      // 失敗したターンは履歴から外し、入力を戻して再試行できるようにする。
      const last = history[history.length - 1];
      setMessages(history.slice(0, -1));
      if (last?.role === "user") setInstruction(last.text);
      setPhase("idle");
    }
  }
```

④ 「生成中」spinner 行——把现有 `{generating && (...)}` 块里的文本 `{t("workshop.thinking")}` 替换为阶段标签 + 字数：

```tsx
              {generating && (
                <ChatRow ai>
                  <div className="flex items-center gap-2.5 text-[13.5px] text-ink-soft">
                    <span className="size-4 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
                    {phase === "generating"
                      ? `${t("workshop.phase.generating")} · ${genChars}`
                      : t(phaseLabelKey(phase))}
                  </div>
                </ChatRow>
              )}
```

⑤ Inspector 调用——把 `<Inspector ... generating={generating} ... />` 增加一个 `runningLabel` prop：

```tsx
        {visibleDraft && (
          <Inspector
            model={visibleSelectedModel}
            generating={generating}
            runningLabel={
              phase === "generating"
                ? `${t("workshop.phase.generating")} · ${genChars}`
                : t(phaseLabelKey(phase))
            }
            draft={visibleDraft}
            canConfirm={canConfirm}
            sourcesChanged={sourcesChanged}
            busy={busy}
            onConfirm={handleConfirm}
          />
        )}
```

⑥ `Inspector` 组件——签名加 `runningLabel: string;`，并把 status 的 running 分支标签从 `t("workshop.running")` 改为 `runningLabel`：

```ts
function Inspector({
  model,
  generating,
  runningLabel,
  draft,
  canConfirm,
  sourcesChanged,
  busy,
  onConfirm,
}: {
  model: string;
  generating: boolean;
  runningLabel: string;
  draft: StructureResult;
  canConfirm: boolean;
  sourcesChanged: boolean;
  busy: boolean;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  const status = generating
    ? { label: runningLabel, color: "var(--gold)" }
    : canConfirm
      ? { label: t("workshop.st.done"), color: "var(--ai)" }
      : { label: t("workshop.st.idle"), color: "var(--ink-muted)" };
```

（`Inspector` 其余对 `generating` 的引用——`linksReady`、`{!generating && !canConfirm && ...}`——保持不变，仍用传入的 `generating` 布尔。）

- [ ] **Step 3: lint + build 确认通过**

Run: `bun run --cwd frontend lint && bun run --cwd frontend build`
Expected: lint 无错误；build 成功（改了 feature UI/类型，按 frontend/AGENTS.md 跑 build）。

- [ ] **Step 4: Commit**

```bash
git add frontend/src/features/workshop/ui/workshop-process-view.tsx frontend/src/shared/i18n/dictionaries.ts
git commit -m "feat(workshop): show draft phase feedback in composer and inspector"
```

---

### Task 7: 端到端手动验证（应用已在 dev 运行，热重载）

**Files:** 无（验证任务）

- [ ] **Step 1: 全套自动化测试**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
node --test scripts/*.test.mjs
bun run --cwd frontend lint
```
Expected: 全绿。

- [ ] **Step 2: 复现原始 bug 的场景**

前置：先 `ollama stop <model>`（或确保该模型未常驻显存），让首次调用触发冷加载。然后在运行中的 app 里：进入某个 inbox 素材的 workshop process 页 → 选模型 → 输入指令 → 发送。

观察（对照修复前）：
- 窗口**不冻结**，鼠标光标正常、可点击其它区域。
- 右侧 inspector 状态栏依次显示：`接続中…/连接中…` → `加载模型中…` →（首个 token 后）`生成中 · N`（N 递增）→ 完成。
- 会话区 spinner 行同步显示相同阶段文案。

- [ ] **Step 3: 验证错误路径（优雅降级）**

`ollama serve` 停掉后再发送一次：应在数秒内（connect_timeout=3s）显示带提示的错误，例如「无法连接 Ollama（请确认 `ollama serve` 正在运行）」，且 UI 不冻结、输入被恢复可重试。

- [ ] **Step 4: 通过则结束；失败则回到 systematic-debugging**

---

## Self-Review

- **Spec 覆盖**：① 解冻 UI → Task 3（async + spawn_blocking）；② 状态栏实时反馈 → Task 2（流式上报）+ Task 6（UI 阶段机）；③ 优雅降级/可执行错误 → Task 2（提案つきエラー）+ Task 6（错误恢复输入）。全部有对应任务。
- **类型一致**：`StreamProgress`(domain) → `DraftEvent`(interface, serde tag=phase) → `DraftPhase`(TS) 三处字段名 `phase` / `chars` 一致；`DraftUiPhase` 的 `loadingModel`/`generating` 与 Rust `LoadingModel`/`Generating`（camelCase 序列化）对齐。
- **No Placeholders**：每个代码步骤含完整可粘贴代码；i18n 步骤给出三语具体值。
- **超时分层** 已落在 Task 2（connect_timeout 3s / total 180s + 提案つきエラー）。重试/断路器/tracing 属 Option C，本计划不含（YAGNI，按所选档位）。

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-24-workshop-ai-streaming-feedback.md`. Two execution options:

1. **Subagent-Driven (recommended)** — 每个 task 派一个新 subagent，task 间我来 review，迭代快。
2. **Inline Execution** — 本会话内按 executing-plans 批量执行，带检查点。

Which approach?
