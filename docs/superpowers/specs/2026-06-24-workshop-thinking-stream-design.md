# 工作坊·思考流（可折叠）设计

**日期**: 2026-06-24
**分支**: feat/workshop-redesign-kb-delete
**前置**: 异步/流式修复 + 阶段反馈（已实现，见 `2026-06-24-workshop-ai-streaming-feedback.md`）

## 目标

对**支持思考模式的 Ollama 模型**，在工作坊加工页实时流式展示模型的思考过程（reasoning），思考结束后自动折叠为可重新展开的摘要。不支持思考的模型保持现状。

## 核心洞察

Ollama 思考模型把推理放在 `message.thinking`（**先于答案流出**），最终答案放 `message.content`（后流），两者是分开的字段（已用 context7 核准 Ollama `/api/chat` 文档）。

我们的 `content` 是结构化 JSON（`format` 固定 schema），逐字渲染半截 JSON 不可读；但 **`thinking` 是自然语言、先到**——它正是要展示的"处理过程的流"。所以：**流式展示 thinking；草稿(content)仍在 done 时整体落成。**

`think:true` 与 `format`(结构化输出) 可共存。

## 架构

### 1. 能力检测（自动）

- `ai_list_ollama_models` 对每个模型调 `/api/show`，读 `capabilities` 是否含 `"thinking"`，返回 `OllamaModel { name, thinking: bool }`。
- 模型下拉对 `thinking==true` 的模型显示「思考」徽章。
- `workshop_draft` 收到 `think: bool`（前端按所选模型的 `thinking` 传入），仅在 true 时给请求体加 `think: true`。
- **代价**: 打开工作坊时每个模型一次本地 `/api/show`（快、可接受）。

### 2. 后端·流解析（`consume_chat_stream`）

每个 NDJSON chunk 现在可能有 `message.thinking` 和/或 `message.content`：
- `thinking` 非空 → 累积到 `thinking_acc`，并 `on_progress(StreamProgress::Thinking { delta })`（增量）。
- `content` 非空 → 累积到 `content_acc`，`on_progress(StreamProgress::Generating { chars })`（同现状）。
- `done` → 用 `content_acc` 解析 JSON 成 `StructureResult`（不变）。
- thinking→content 的切换是隐式的（前端收到 Generating 即知思考结束）。

`StructureResult` 契约**不变**（thinking 文本不进后端结果，只走 Channel）。

### 3. 事件 / 阶段

- 新增 `StreamProgress::Thinking { delta: String }` → `DraftEvent::Thinking { delta }`（serde `tag="phase"`, camelCase → `{"phase":"thinking","delta":"…"}`）。
- 前端 `DraftUiPhase` 加 `"thinking"`；`isGeneratingPhase` 含之；`phaseLabelKey("thinking") = "workshop.phase.thinking"`。
- 阶段序列：`connecting → retrieving → loadingModel → thinking(流) → generating(chars) → done`。
- 非思考模型：无 thinking chunk → 自动跳过 thinking 阶段（loadingModel→generating），现状不变。

### 4. 前端·UI（可折叠思考面板）

- 在 AI 对话行内渲染「思考过程」面板（disclosure）：标题 + chevron + 字数。
- `phase==="thinking"`：面板**自动展开**，实时追加 thinking 文本（柔和/等宽样式），底部跟随滚动，带"思考中…"指示。
- 切到 `generating`（content 开始）：面板**自动折叠**为摘要「思考过程 · N 字」，可点开重看。
- thinking 文本挂到该 AI 消息对象：`ProcessMessage` 的 ai 分支加 `thinking?: string`，done 时写入。历史轮默认折叠、可展开。
- 当前进行中轮的 thinking 累积在组件 state（`thinkingBuf`），done 时随 result 一起落入消息。

## 契约变更汇总

```
// Rust
StreamProgress::Thinking { delta: String }              // domain
DraftEvent::Thinking { delta: String }                  // interface, From 映射
OllamaModel { name: String, thinking: bool }            // +thinking
workshop_draft(.., model: String, think: bool, on_event)// +think
list_ollama_models() -> Vec<OllamaModel{name,thinking}> // +capability via /api/show

// TS
type DraftPhase = … | { phase: "thinking"; delta: string }
type OllamaModel = { name: string; thinking: boolean }
type DraftUiPhase = … | "thinking"
ProcessMessage ai 分支: { role:"ai"; result; thinking?: string }
workshopDraft(inboxPaths, messages, model, think, onPhase)
```

## 数据流

```
用户发送
  └ 前端: think = 所选模型.thinking
retrieving → loadingModel
  └ thinking chunks → Thinking{delta} → 面板展开+流式累积(thinkingBuf)
  └ content chunks  → Generating{chars} → 面板自动折叠
done
  └ 草稿落成 + thinking 文本挂到该 AI 消息（折叠存档）
错误/非思考模型 → 无 thinking 面板，同现状
```

## 边界（YAGNI）

- 只对思考模型加这套；非思考模型不强行流式渲染半截 JSON content。
- 不动后端 `StructureResult`；thinking 仅经 Channel + 存前端消息。
- 不做质量环（仍无真实评分，见 workshop-no-fake-metrics 记忆）。
- 不做"逐行流式渲染最终草稿正文"（content 是 JSON，需增量 JSON 解析，过度）。

## 测试

- Rust `consume_chat_stream`: 喂含 `thinking`+`content` 的 NDJSON，断言先报 Thinking(delta) 后报 Generating，最终解析出草稿。
- Rust `DraftEvent::Thinking` 序列化 `{"phase":"thinking","delta":"…"}`。
- Rust 能力检测: `/api/show` 响应解析出 `thinking` 能力（解析函数纯单测，喂 canned JSON）。
- 前端 `phaseLabelKey("thinking")` / `isGeneratingPhase("thinking")`（node test）。
- 前端 thinking 累积/折叠的纯逻辑若可抽出则单测；UI 由 lint+build+实机验证。

## 假设 / 待验证

- `think:true` + `format` 共存对该 12B 模型行为正常（实机验证）。
- 该模型若不支持 thinking，徽章不显示、不传 think，行为同现状（主路径）。
