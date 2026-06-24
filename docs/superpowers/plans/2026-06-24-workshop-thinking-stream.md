# 工作坊·思考流（可折叠）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对支持思考模式的 Ollama 模型，在工作坊加工页实时流式展示思考过程，思考结束自动折叠为可重看的摘要；不支持的模型行为不变。

**Architecture:** Ollama `think:true` 让 `message.thinking`（先流）与 `message.content`（后流·JSON）分离；后端逐块把 thinking 增量经 Tauri `Channel` 推到前端，content 仍报字符数；草稿在 done 落成。能力由 `/api/show` 的 `capabilities` 自动检测。

**Tech Stack:** Tauri 2（`tauri::ipc::Channel`）、reqwest blocking 流式、serde、Next.js/React、Bun、node --test。

设计依据：`docs/superpowers/specs/2026-06-24-workshop-thinking-stream-design.md`。

## Global Constraints

- 注释/文档日文（AGENTS.md）。Tauri 命令返回 `Result<T, String>`。跨 IPC struct/enum 派生 `Serialize` + `#[serde(rename_all = "camelCase")]`。
- DDD：`Channel` 只在 interface 层；application/domain 用纯回调 `&mut dyn FnMut(StreamProgress)`。
- FSD：IPC 只在 `shared/api/tauri/client.ts`；纯逻辑在 `features/workshop/model/process-state.ts`。
- TDD：先写失败测试。Rust 2 空格缩进，不 `cargo fmt` 全文件。
- 验证：`cargo test --manifest-path src-tauri/Cargo.toml`、`node --test scripts/*.test.mjs`、`bun run --cwd frontend lint`、`bun run --cwd frontend build`。
- 后端 `StructureResult` 契约不变；thinking 只走 Channel + 存前端消息。不做质量环。

---

### Task 1: 模型能力检测（/api/show → OllamaModel.thinking）

**Files:**
- Modify: `src-tauri/src/ai/infrastructure/ollama.rs`

**Interfaces:**
- Produces:
  - `OllamaModel { name: String, thinking: bool }`
  - `fn show_supports_thinking(body: &str) -> bool`（私有・纯・可单测）
  - `OllamaProvider::list_models() -> Result<Vec<OllamaModel>, AiError>`（每模型补 thinking 能力）

- [ ] **Step 1: 写失败测试**

在 `ollama.rs` 的 `mod tests` 追加：

```rust
  #[test]
  fn show_supports_thinking_reads_capabilities() {
    let yes = r#"{"capabilities":["completion","thinking"]}"#;
    let no = r#"{"capabilities":["completion"]}"#;
    assert!(show_supports_thinking(yes));
    assert!(!show_supports_thinking(no));
    assert!(!show_supports_thinking("{}"));
  }
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml ollama`
Expected: FAIL（`show_supports_thinking` 未定义；`OllamaModel` 字段不全）

- [ ] **Step 3: 实现**

`OllamaModel` 加字段：

```rust
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OllamaModel {
  pub name: String,
  pub thinking: bool,
}
```

`parse_models_response` 把 thinking 先置 false（/api/tags 无能力信息）：

```rust
    .map(|model| OllamaModel { name: model.name, thinking: false })
```

新增能力解析 + 探测（放在 `parse_models_response` 之后）：

```rust
#[derive(Deserialize)]
struct ShowResponse {
  #[serde(default)]
  capabilities: Vec<String>,
}

/// /api/show のレスポンスから thinking 能力の有無を読む。
fn show_supports_thinking(body: &str) -> bool {
  serde_json::from_str::<ShowResponse>(body)
    .map(|s| s.capabilities.iter().any(|c| c == "thinking"))
    .unwrap_or(false)
}
```

把 `list_models` 改为复用一个 client、并对每个模型补 thinking：

```rust
  pub fn list_models() -> Result<Vec<OllamaModel>, AiError> {
    let client = reqwest::blocking::Client::builder()
      .timeout(Duration::from_secs(3))
      .build()
      .map_err(|e| AiError::Network(e.to_string()))?;
    let resp = client
      .get(format!("{API_BASE}/api/tags"))
      .send()
      .map_err(|e| AiError::Network(e.to_string()))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| AiError::Network(e.to_string()))?;
    if status.as_u16() != 200 {
      return Err(AiError::Other(format!("Ollama 模型列表读取失败({status}): {text}")));
    }
    let mut models = parse_models_response(&text)?;
    // 各モデルの thinking 能力を /api/show で補う（ローカル・高速）。
    for model in &mut models {
      if let Ok(show) = client
        .post(format!("{API_BASE}/api/show"))
        .json(&json!({ "model": model.name }))
        .send()
        .and_then(|r| r.text())
      {
        model.thinking = show_supports_thinking(&show);
      }
    }
    Ok(models)
  }
```

更新既有测试 `parse_models_response_returns_downloaded_model_names`，补 `thinking: false`：

```rust
        OllamaModel { name: "qwen3:8b".into(), thinking: false },
        OllamaModel { name: "llama3.1:8b".into(), thinking: false },
```

- [ ] **Step 4: 跑测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ai/infrastructure/ollama.rs
git commit -m "feat(ai): detect model thinking capability via /api/show"
```

---

### Task 2: 后端流式思考（think 参数 + thinking delta 上报）

**Files:**
- Modify: `src-tauri/src/ai/domain.rs`
- Modify: `src-tauri/src/ai/infrastructure/ollama.rs`
- Modify: `src-tauri/src/workshop/interface.rs`

**Interfaces:**
- Consumes: `StreamProgress`、`OllamaModel.thinking`（Task 1）
- Produces:
  - `StreamProgress::Thinking { delta: String }`
  - `DraftEvent::Thinking { delta: String }` + `From`
  - `OllamaProvider::with_model_think(model: String, think: bool)`
  - `workshop_draft(app, inbox_paths, messages, model, think: bool, on_event)`

- [ ] **Step 1: 写失败测试**

在 `ollama.rs` 的 `mod tests` 追加：

```rust
  #[test]
  fn consume_chat_stream_reports_thinking_then_content() {
    let lines = vec![
      Ok(r#"{"message":{"thinking":"考え中"},"done":false}"#.to_string()),
      Ok(r#"{"message":{"content":"{\"kind\":\"chat\",\"title\":\"\",\"cat\":\"\",\"body_markdown\":\"hi\",\"suggested_links\":[]}"},"done":true}"#.to_string()),
    ];
    let mut events = Vec::new();
    let result = consume_chat_stream(lines.into_iter(), &mut |p| events.push(p)).unwrap();
    assert_eq!(result.body_markdown, "hi");
    assert_eq!(events.first(), Some(&StreamProgress::Thinking { delta: "考え中".into() }));
    assert!(events.iter().any(|e| matches!(e, StreamProgress::Generating { .. })));
  }

  #[test]
  fn build_body_includes_think_when_enabled() {
    assert_eq!(build_body("m", &req(), true)["think"], true);
    assert_eq!(build_body("m", &req(), false).get("think"), None);
  }
```

在 `interface.rs` 的 `mod tests` 追加：

```rust
    let think = serde_json::to_value(DraftEvent::from(StreamProgress::Thinking { delta: "x".into() })).unwrap();
    assert_eq!(think["phase"], "thinking");
    assert_eq!(think["delta"], "x");
```

- [ ] **Step 2: 跑测试确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: FAIL（`StreamProgress::Thinking` / `build_body` 签名 / `DraftEvent::Thinking` 未定义）

- [ ] **Step 3: 实现 domain**

`src-tauri/src/ai/domain.rs` 给 `StreamProgress` 加变体：

```rust
pub enum StreamProgress {
  /// 関連既存条目を FTS で検索中（モデル呼び出し前の確定的な段）。
  Retrieving,
  /// 推論トレース（thinking）の増分。思考モデルのみ。
  Thinking { delta: String },
  /// リクエスト送信済み・最初のトークン待ち（モデルのロード中を含む）。
  LoadingModel,
  /// 本文（content）受信中。chars は累積文字数。
  Generating { chars: usize },
}
```

- [ ] **Step 4: 实现 ollama（think + thinking 解析）**

`OllamaProvider` 加 `think` 字段并更新构造：

```rust
pub struct OllamaProvider {
  model: Option<String>,
  base_url: String,
  think: bool,
}
```

`new()` 与 `with_model()` 的 `Self { .. }` 字面量都加 `think: false`。新增构造：

```rust
  pub fn with_model_think(model: String, think: bool) -> Self {
    let mut provider = Self::with_model(model);
    provider.think = think;
    provider
  }
```

`build_body` 加 `think` 参数：

```rust
fn build_body(model: &str, req: &StructureRequest, think: bool) -> Value {
  let mut body = json!({
    "model": model,
    "messages": chat_messages(req),
    "stream": true,
    "format": output_schema(),
    "options": { "temperature": 0.2 }
  });
  if think {
    body["think"] = json!(true);
  }
  body
}
```

`ChatMessage` 加 thinking：

```rust
#[derive(Deserialize)]
struct ChatMessage {
  #[serde(default)]
  content: String,
  #[serde(default)]
  thinking: Option<String>,
}
```

`consume_chat_stream` 分离 thinking/content：

```rust
fn consume_chat_stream(
  lines: impl Iterator<Item = std::io::Result<String>>,
  on_progress: &mut dyn FnMut(StreamProgress),
) -> Result<StructureResult, AiError> {
  let mut content = String::new();
  for line in lines {
    let line = line.map_err(|e| {
      AiError::Network(format!(
        "读取 Ollama 响应失败（模型加载或生成可能超时，可先在终端 `ollama run` 预热）: {e}"
      ))
    })?;
    if line.trim().is_empty() {
      continue;
    }
    let chunk: StreamChunk =
      serde_json::from_str(&line).map_err(|e| AiError::Other(e.to_string()))?;
    if let Some(thinking) = chunk.message.thinking {
      if !thinking.is_empty() {
        on_progress(StreamProgress::Thinking { delta: thinking });
      }
    }
    if !chunk.message.content.is_empty() {
      content.push_str(&chunk.message.content);
      on_progress(StreamProgress::Generating { chars: content.chars().count() });
    }
    if chunk.done {
      break;
    }
  }
  let raw: RawResult =
    serde_json::from_str(&content).map_err(|e| AiError::Other(e.to_string()))?;
  Ok(StructureResult {
    kind: raw.kind,
    title: raw.title,
    cat: raw.cat,
    body_markdown: raw.body_markdown,
    suggested_links: raw.suggested_links,
  })
}
```

`structure` 里把 `build_body(&model, &req)` 改为 `build_body(&model, &req, self.think)`。

- [ ] **Step 5: 实现 interface（DraftEvent + workshop_draft think 参数）**

`src-tauri/src/workshop/interface.rs` 给 `DraftEvent` 加变体与映射：

```rust
pub enum DraftEvent {
  Retrieving,
  /// 推論トレースの増分。
  Thinking { delta: String },
  LoadingModel,
  Generating { chars: usize },
}

impl From<StreamProgress> for DraftEvent {
  fn from(p: StreamProgress) -> Self {
    match p {
      StreamProgress::Retrieving => DraftEvent::Retrieving,
      StreamProgress::Thinking { delta } => DraftEvent::Thinking { delta },
      StreamProgress::LoadingModel => DraftEvent::LoadingModel,
      StreamProgress::Generating { chars } => DraftEvent::Generating { chars },
    }
  }
}
```

`workshop_draft` 加 `think: bool` 参数，并用 `with_model_think` 构造（其余不变）：

```rust
pub async fn workshop_draft(
  app: tauri::AppHandle,
  inbox_paths: Vec<String>,
  messages: Vec<ChatTurn>,
  model: String,
  think: bool,
  on_event: Channel<DraftEvent>,
) -> Result<StructureResult, String> {
```

闭包内：

```rust
    let provider = crate::ai::ollama::OllamaProvider::with_model_think(model, think);
```

- [ ] **Step 6: 跑测试确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS（含新 3 断言；原测试不破）

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/ai/domain.rs src-tauri/src/ai/infrastructure/ollama.rs src-tauri/src/workshop/interface.rs
git commit -m "feat(workshop): stream model thinking trace when enabled"
```

---

### Task 3: 前端契约 + 状态机（thinking 阶段 / 消息 thinking 字段）

**Files:**
- Modify: `frontend/src/shared/api/tauri/client.ts`
- Modify: `frontend/src/features/workshop/model/process-state.ts`
- Test: `scripts/workshop-process-state.test.mjs`

**Interfaces:**
- Produces:
  - `DraftPhase = … | { phase: "thinking"; delta: string }`
  - `OllamaModel = { name: string; thinking: boolean }`
  - `workshopDraft(inboxPaths, messages, model, think, onPhase?)`
  - `DraftUiPhase = … | "thinking"`；`phaseLabelKey("thinking")="workshop.phase.thinking"`
  - `ProcessMessage` ai 分支：`{ role:"ai"; result; thinking?: string }`

- [ ] **Step 1: 写失败测试**

在 `scripts/workshop-process-state.test.mjs` 的 phase 测试里补 thinking：

```js
test("thinking phase is generating and has its own label", () => {
  assert.equal(isGeneratingPhase("thinking"), true);
  assert.equal(phaseLabelKey("thinking"), "workshop.phase.thinking");
});
```

- [ ] **Step 2: 跑测试确认失败**

Run: `node --test scripts/workshop-process-state.test.mjs`
Expected: FAIL（`phaseLabelKey("thinking")` 返回 idle 兜底，断言不符）

- [ ] **Step 3: 实现 process-state**

`DraftUiPhase` 加 `"thinking"`；`isGeneratingPhase` 加之；`phaseLabelKey` 加分支：

```ts
export type DraftUiPhase =
  | "idle"
  | "connecting"
  | "retrieving"
  | "thinking"
  | "loadingModel"
  | "generating"
  | "done";

export function isGeneratingPhase(phase: DraftUiPhase): boolean {
  return (
    phase === "connecting" ||
    phase === "retrieving" ||
    phase === "thinking" ||
    phase === "loadingModel" ||
    phase === "generating"
  );
}
```

`phaseLabelKey` 的 switch 加：

```ts
    case "thinking":
      return "workshop.phase.thinking";
```

`ProcessMessage` ai 分支加 `thinking?`：

```ts
export type ProcessMessage<Source = unknown> =
  | { role: "user"; text: string; sources?: Source[] }
  | { role: "ai"; result: StructureResult; thinking?: string };
```

`replaceLatestEntryResult` 保留 thinking（把 `{ role: "ai", result }` 改为带 thinking）：

```ts
  return messages.map((message, current) =>
    current === index && message.role === "ai"
      ? { role: "ai", result, thinking: message.thinking }
      : message
  );
```

- [ ] **Step 4: 实现 client.ts**

`DraftPhase` 加 thinking；`OllamaModel` 加 thinking；`workshopDraft` 加 think 参数：

```ts
export type DraftPhase =
  | { phase: "retrieving" }
  | { phase: "thinking"; delta: string }
  | { phase: "loadingModel" }
  | { phase: "generating"; chars: number };

export type OllamaModel = {
  name: string;
  thinking: boolean;
};

export async function workshopDraft(
  inboxPaths: string[],
  messages: ChatTurn[],
  model: string,
  think: boolean,
  onPhase?: (phase: DraftPhase) => void
): Promise<StructureResult> {
  const channel = new Channel<DraftPhase>();
  if (onPhase) channel.onmessage = onPhase;
  return invoke<StructureResult>("workshop_draft", {
    inboxPaths,
    messages,
    model,
    think,
    onEvent: channel,
  });
}
```

- [ ] **Step 5: 跑测试 + lint**

Run: `node --test scripts/workshop-process-state.test.mjs && bun run --cwd frontend lint`
Expected: 测试 PASS；lint 无错误。

- [ ] **Step 6: Commit**

```bash
git add frontend/src/shared/api/tauri/client.ts frontend/src/features/workshop/model/process-state.ts scripts/workshop-process-state.test.mjs
git commit -m "feat(workshop): add thinking phase, think flag and message thinking field"
```

---

### Task 4: 前端 UI（可折叠思考面板 + 模型徽章 + i18n）

**Files:**
- Modify: `frontend/src/features/workshop/ui/workshop-process-view.tsx`
- Modify: `frontend/src/shared/i18n/dictionaries.ts`

**Interfaces:**
- Consumes: Task 3 全部前端契约。

- [ ] **Step 1: 加 i18n（三语）**

`dictionaries.ts` 在每个语言块的 `"workshop.phase.retrieving"` 旁加：

中文：
```ts
  "workshop.phase.thinking": "思考中…",
  "workshop.think.label": "思考过程",
  "workshop.think.badge": "思考",
```
English（在英文块 `"workshop.phase.retrieving": "Retrieving links…",` 旁）：
```ts
  "workshop.phase.thinking": "Thinking…",
  "workshop.think.label": "Reasoning",
  "workshop.think.badge": "think",
```
日本語（在日文块 `"workshop.phase.retrieving": "関連項目を検索中…",` 旁）：
```ts
  "workshop.phase.thinking": "思考中…",
  "workshop.think.label": "思考プロセス",
  "workshop.think.badge": "思考",
```

- [ ] **Step 2: view — 状态与回调**

`import` 从 process-state 增加（已有 isGeneratingPhase/phaseLabelKey/DraftUiPhase 等，无需重复）；从 client 增加 `type DraftPhase`（已导入）。

在 `const [genChars, setGenChars] = useState(0);` 旁加 thinking 缓冲：

```ts
  const [thinkingBuf, setThinkingBuf] = useState("");
```

`runTurn` 起始把 `setGenChars(0);` 之后加 `setThinkingBuf("");`。把 onPhase 回调改为：

```ts
        (p: DraftPhase) => {
          if (p.phase === "generating") {
            setPhase("generating");
            setGenChars(p.chars);
          } else if (p.phase === "thinking") {
            setPhase("thinking");
            setThinkingBuf((prev) => prev + p.delta);
          } else if (p.phase === "retrieving") {
            setPhase("retrieving");
          } else {
            setPhase("loadingModel");
          }
        }
```

`workshopDraft` 调用加 think 实参（按所选模型能力）。先在组件内算出能力：

```ts
  const selectedThinking =
    visibleModels.find((m) => m.name === visibleSelectedModel)?.thinking ?? false;
```

调用处：

```ts
      const result = await workshopDraft(
        sources.map((s) => s.id),
        history.map(toChatTurn),
        visibleSelectedModel,
        selectedThinking,
        (p: DraftPhase) => { /* 上面的回调 */ }
      );
```

成功分支把 thinking 挂到 AI 消息（把 `{ role: "ai", result }` 改为）：

```ts
      setMessages([...history, { role: "ai", result, thinking: thinkingBuf || undefined }]);
```

- [ ] **Step 3: view — ThinkingPanel 组件**

在文件内（如 `DraftCard` 之后）新增：

```tsx
// 折りたたみ可能な思考トレース。streaming 中は自動展開、終了で自動折りたたみ（再展開可）。
function ThinkingPanel({ text, streaming }: { text: string; streaming: boolean }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(streaming);
  const endRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!streaming) setOpen(false);
  }, [streaming]);
  useEffect(() => {
    if (streaming && open) endRef.current?.scrollIntoView({ block: "end" });
  }, [text, streaming, open]);
  return (
    <div className="mb-2.5 overflow-hidden rounded-xl border border-line bg-surface-2">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="flex w-full items-center gap-2 px-3.5 py-2 text-left"
      >
        <Icon name="spark" size={13} className="text-ai" />
        <span className="text-[12.5px] font-semibold text-ink-soft">{t("workshop.think.label")}</span>
        {streaming && (
          <span className="size-3 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
        )}
        <span className="font-mono text-[10.5px] text-ink-faint">{text.length}</span>
        <div className="flex-1" />
        <Icon
          name="chevD"
          size={14}
          className={`text-ink-muted transition-transform ${open ? "" : "-rotate-90"}`}
        />
      </button>
      {open && (
        <div className="max-h-64 overflow-auto border-t border-line px-3.5 py-2.5 text-[12.5px] leading-relaxed whitespace-pre-wrap text-ink-soft">
          {text}
          <div ref={endRef} />
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: view — 渲染思考面板**

① 历史 AI 消息：在 entry/chat 渲染前加思考面板。把 ai 分支的渲染改为外层包裹（在 `m.result.kind === "chat" ? (...) : (...)` 外层先渲染 thinking）。具体把这段：

```tsx
                ) : m.result.kind === "chat" ? (
                  <ChatRow key={i} ai>
```

改为先在 ChatRow 内顶部插入面板——最小改法：把两个 ai 分支的 `<ChatRow key={i} ai>` 内容首行加：

```tsx
                    {m.thinking && <ThinkingPanel text={m.thinking} streaming={false} />}
```

（chat 分支与 entry 分支各加一次，紧跟 `<ChatRow key={i} ai>` 之后、原内容之前。）

② 进行中轮：在 `{generating && (...)}` spinner 块内，spinner 行之前插入实时面板：

```tsx
              {generating && (
                <ChatRow ai>
                  {thinkingBuf && (
                    <ThinkingPanel text={thinkingBuf} streaming={phase === "thinking"} />
                  )}
                  <div className="flex items-center gap-2.5 text-[13.5px] text-ink-soft">
                    <span className="size-4 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
                    {phase === "generating"
                      ? `${t("workshop.phase.generating")} · ${genChars}`
                      : t(phaseLabelKey(phase))}
                  </div>
                </ChatRow>
              )}
```

- [ ] **Step 5: view — 模型下拉「思考」徽章**

把模型下拉 `<option>` 文案对思考模型追加徽章（`<select>` 的 option 内不能放组件，用文字标记）：

```tsx
                      visibleModels.map((model) => (
                        <option key={model.name} value={model.name}>
                          {model.thinking ? `${model.name} · ${t("workshop.think.badge")}` : model.name}
                        </option>
                      ))
```

并修正预览模型常量 `PREVIEW_MODELS` 加 `thinking`：

```ts
const PREVIEW_MODELS: OllamaModel[] = [
  { name: "qwen3:8b", thinking: true },
  { name: "llama3.1:8b", thinking: false },
];
```

- [ ] **Step 6: lint + build**

Run: `bun run --cwd frontend lint && bun run --cwd frontend build`
Expected: lint 无错误；build 成功（TypeScript 通过）。

- [ ] **Step 7: Commit**

```bash
git add frontend/src/features/workshop/ui/workshop-process-view.tsx frontend/src/shared/i18n/dictionaries.ts
git commit -m "feat(workshop): collapsible thinking panel and model think badge"
```

---

### Task 5: 实机验证

**Files:** 无。

- [ ] **Step 1: 全套自动化测试**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml
node --test scripts/*.test.mjs
bun run --cwd frontend lint
```
Expected: 全绿。

- [ ] **Step 2: 准备思考模型**

确认本地有思考模型：`ollama list`。若无，`ollama pull qwen3`（或其它带 thinking 能力的模型）。`ollama show qwen3` 的 Capabilities 应含 `thinking`。

- [ ] **Step 3: 思考模型路径**

`bun run dev` 启动；进加工页，模型下拉选带「思考」徽章的模型，发送一句指令。观察：
- 阶段 spinner 出现「思考中…」，**思考面板实时流式追加推理文本**；
- content 开始时（「生成中·N字」）思考面板**自动折叠**为「思考过程 · N」；
- 点击可重新展开；草稿正常落成、确认可用；
- 全程 UI 不冻结。

- [ ] **Step 4: 非思考模型路径（回归）**

选无徽章的模型（如 gemma）发送：**无思考面板**，行为同现状（检索→加载模型→生成中→完成），不报错。

- [ ] **Step 5: 通过则结束；失败回 systematic-debugging**

---

## Self-Review

- **Spec 覆盖**：能力检测→T1；think 参数+thinking 流→T2；前端阶段+消息字段→T3；折叠面板+徽章+i18n→T4；共存/回归验证→T5。全覆盖。
- **类型一致**：`StreamProgress::Thinking{delta:String}`→`DraftEvent::Thinking{delta}`(camelCase `{"phase":"thinking","delta"}`)→TS `DraftPhase {phase:"thinking";delta:string}`→`DraftUiPhase "thinking"` 一致；`OllamaModel.thinking` Rust/TS 一致；`workshop_draft(..,think,..)` 与 `workshopDraft(..,think,..)` 参数顺序一致（model 后、onEvent 前）。
- **No Placeholders**：每步含完整可粘贴代码 + 三语 i18n 值。
- **边界**：非思考模型零行为变化（无 thinking chunk → 无 Thinking 事件 → 无面板）；后端 `StructureResult` 不变。

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-24-workshop-thinking-stream.md`.

1. **Subagent-Driven (recommended)** — 每 task 派新 subagent，task 间 review。
2. **Inline Execution** — 本会话 executing-plans 批量执行，带检查点。

Which approach?
