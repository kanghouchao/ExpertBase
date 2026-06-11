// Mock content for ExpertBase — a tea (茶) knowledge base the owner sells access
// to. Stands in for a future backend client; shapes are intentionally typed so
// the swap to real API responses is mechanical. Phase 1 ports only the slices
// the shell + Dashboard consume.

import type { IconName } from "@/components/eb/icon";

export type RouteId =
  | "dash"
  | "capture"
  | "workshop"
  | "wiki"
  | "graph"
  | "showcase"
  | "bots"
  | "plugins";

export type NavItem = {
  id: RouteId;
  href: string;
  icon: IconName;
  tone?: "ai";
};

export const NAV: NavItem[] = [
  { id: "dash", href: "/", icon: "dash" },
  { id: "capture", href: "/capture", icon: "inbox" },
  { id: "workshop", href: "/workshop", icon: "merge" },
  { id: "wiki", href: "/wiki", icon: "book" },
  { id: "graph", href: "/graph", icon: "graph", tone: "ai" },
  { id: "showcase", href: "/showcase", icon: "eye" },
  { id: "bots", href: "/bots", icon: "bot" },
  { id: "plugins", href: "/plugins", icon: "plug" },
];

export type RawType = "audio" | "video" | "pdf" | "doc" | "note";
export type RawStatus = "pending" | "transcribed" | "processed";

export type RawMaterial = {
  id: string;
  type: RawType;
  title: string;
  sourceKey: string;
  dateKey: string;
  status: RawStatus;
};

export const RAW_MATERIALS: RawMaterial[] = [
  {
    id: "r1",
    type: "audio",
    title: "与制茶师傅的访谈录音",
    sourceKey: "raw.r1.source",
    dateKey: "time.2h",
    status: "transcribed",
  },
  {
    id: "r2",
    type: "pdf",
    title: "2024 普洱仓储白皮书.pdf",
    sourceKey: "raw.r2.source",
    dateKey: "time.yesterday",
    status: "pending",
  },
  {
    id: "r3",
    type: "note",
    title: "随手记：今早试的三款岩茶",
    sourceKey: "raw.r3.source",
    dateKey: "time.yesterday",
    status: "processed",
  },
  {
    id: "r4",
    type: "video",
    title: "盖碗冲泡手法教学.mov",
    sourceKey: "raw.r4.source",
    dateKey: "time.3d",
    status: "pending",
  },
  {
    id: "r5",
    type: "doc",
    title: "会员常见问题汇总.docx",
    sourceKey: "raw.r5.source",
    dateKey: "time.lastWeek",
    status: "processed",
  },
  {
    id: "r6",
    type: "audio",
    title: "语音备忘：勐海茶山见闻",
    sourceKey: "raw.r6.source",
    dateKey: "time.lastWeek",
    status: "transcribed",
  },
];

// `color` is a CSS color (often a theme var) applied inline on the type icon.
export const RAW_TYPE: Record<RawType, { icon: IconName; color: string }> = {
  audio: { icon: "audio", color: "var(--brand)" },
  video: { icon: "video", color: "#9b5a6b" },
  pdf: { icon: "pdf", color: "#b5572f" },
  doc: { icon: "doc", color: "#5566b0" },
  note: { icon: "note", color: "var(--ai)" },
};

export type TagTone = "line" | "accent" | "ai" | "muted" | "gold";

export const STATUS: Record<RawStatus, { tone: TagTone }> = {
  pending: { tone: "muted" },
  transcribed: { tone: "ai" },
  processed: { tone: "accent" },
};

export type Severity = "high" | "med" | "low";

export type LintFinding = {
  id: string;
  type: "orphan" | "quality" | "dup" | "stale";
  sev: Severity;
  title: string;
  detail: string;
};

export const LINT: LintFinding[] = [
  {
    id: "l1",
    type: "orphan",
    sev: "high",
    title: "孤立条目",
    detail: "「紫砂壶养护」「冷萃」没有任何双向链接，建议关联到相关器具/冲泡条目。",
  },
  {
    id: "l2",
    type: "quality",
    sev: "med",
    title: "内容偏薄",
    detail: "「焙火」仅 640 字且引用单一，建议补充退火周期与火功等级。",
  },
  {
    id: "l3",
    type: "dup",
    sev: "low",
    title: "疑似重复",
    detail: "「茶多酚」与「杀青」中关于氧化的描述高度重叠，可合并或互相引用。",
  },
  {
    id: "l4",
    type: "stale",
    sev: "low",
    title: "久未更新",
    detail: "「冷萃」一个月未更新，且会员近期咨询量上升。",
  },
];

export const STATS = {
  rawCount: 23,
  wikiCount: 48,
  links: 134,
  orphans: 2,
  botMsgs: 1204,
  members: 328,
  health: 78,
};

export type KnowledgeBase = {
  id: string;
  primary?: boolean;
  name?: string;
  icon: IconName;
  accent: string;
  entries: number;
};

// Primary KB localizes its name via t('app.kb'); secondary names are
// user-authored content and stay as written.
export const KNOWLEDGE_BASES: KnowledgeBase[] = [
  { id: "kb1", primary: true, icon: "book", accent: "var(--brand)", entries: 48 },
  { id: "kb2", name: "咖啡风味笔记", icon: "inbox", accent: "#6e8b5e", entries: 12 },
  { id: "kb3", name: "香道入门", icon: "spark", accent: "#9b5a6b", entries: 7 },
];

// Pending materials drive the Workshop nav badge.
export const PENDING = RAW_MATERIALS.filter((r) => r.status !== "processed").length;
