// Mock content for ExpertBase — a tea (茶) knowledge base the owner sells access
// to. Stands in for a future backend client; shapes are intentionally typed so
// the swap to real API responses is mechanical across the shell, dashboard,
// and route-level feature views.

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
  size: string;
  preview: string;
  words: number;
  tags: string[];
};

export const RAW_MATERIALS: RawMaterial[] = [
  {
    id: "r1",
    type: "audio",
    title: "与制茶师傅的访谈录音",
    sourceKey: "raw.r1.source",
    dateKey: "time.2h",
    status: "transcribed",
    size: "128 MB",
    preview:
      "……所以你看，杀青的温度其实没有一个死数字，要看茶青的含水量。手摸下去，第一遍要高温杀透，让它快速失水。",
    words: 18420,
    tags: ["制茶", "杀青"],
  },
  {
    id: "r2",
    type: "pdf",
    title: "2024 普洱仓储白皮书.pdf",
    sourceKey: "raw.r2.source",
    dateKey: "time.yesterday",
    status: "pending",
    size: "8.4 MB",
    preview: "本白皮书梳理了干仓与湿仓的温湿度区间、霉变风险阈值，以及不同年份饼茶的转化曲线对照。",
    words: 24100,
    tags: ["仓储", "普洱"],
  },
  {
    id: "r3",
    type: "note",
    title: "随手记：今早试的三款岩茶",
    sourceKey: "raw.r3.source",
    dateKey: "time.yesterday",
    status: "processed",
    size: "—",
    preview: "肉桂桂皮香明显，水偏霸；水仙醇厚、丛味足；大红袍拼配平衡但少了点记忆点。",
    words: 120,
    tags: ["岩茶", "品鉴"],
  },
  {
    id: "r4",
    type: "video",
    title: "盖碗冲泡手法教学.mov",
    sourceKey: "raw.r4.source",
    dateKey: "time.3d",
    status: "pending",
    size: "420 MB",
    preview: "关键帧：注水高度、出汤角度、留根与否的对比演示。",
    words: 0,
    tags: ["冲泡", "盖碗"],
  },
  {
    id: "r5",
    type: "doc",
    title: "会员常见问题汇总.docx",
    sourceKey: "raw.r5.source",
    dateKey: "time.lastWeek",
    status: "processed",
    size: "1.2 MB",
    preview: "Q：新茶可以马上喝吗？Q：醒茶要醒多久？Q：紫砂壶一壶一茶是必须的吗？",
    words: 3200,
    tags: ["FAQ", "会员"],
  },
  {
    id: "r6",
    type: "audio",
    title: "语音备忘：勐海茶山见闻",
    sourceKey: "raw.r6.source",
    dateKey: "time.lastWeek",
    status: "transcribed",
    size: "12 MB",
    preview: "老班章和老曼峨的距离其实很近，但口感差异巨大，苦底化得快不快是关键。",
    words: 1640,
    tags: ["茶山", "普洱"],
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

export type WikiEntry = {
  id: string;
  title: string;
  en: string;
  cat: string;
  updated: string;
  words: number;
  links: number;
  backlinks: number;
  quality: number;
  excerpt: string;
  related: string[];
  orphan: boolean;
};

export const WIKI: WikiEntry[] = [
  {
    id: "w1",
    title: "杀青",
    en: "Kill-green",
    cat: "工艺",
    updated: "今天",
    words: 1240,
    links: 5,
    backlinks: 3,
    quality: 92,
    excerpt:
      "杀青是通过高温钝化酶活性、终止氧化的关键工序。温度、时间与手法共同决定后续转化的基础。",
    related: ["萎凋", "揉捻", "乌龙茶", "岩茶"],
    orphan: false,
  },
  {
    id: "w2",
    title: "萎凋",
    en: "Withering",
    cat: "工艺",
    updated: "今天",
    words: 980,
    links: 4,
    backlinks: 4,
    quality: 88,
    excerpt: "萎凋使鲜叶适度失水、走青气、增柔韧，是乌龙与白茶风味形成的起点。",
    related: ["杀青", "白茶", "乌龙茶"],
    orphan: false,
  },
  {
    id: "w3",
    title: "普洱干仓仓储",
    en: "Dry storage",
    cat: "仓储",
    updated: "昨天",
    words: 2100,
    links: 6,
    backlinks: 2,
    quality: 95,
    excerpt: "干仓指在相对湿度 60-70%、温度稳定的环境中自然陈化。相较湿仓，转化慢但纯净、风险低。",
    related: ["普洱茶", "陈化曲线", "湿仓"],
    orphan: false,
  },
  {
    id: "w4",
    title: "盖碗冲泡",
    en: "Gaiwan brewing",
    cat: "冲泡",
    updated: "3 天前",
    words: 760,
    links: 3,
    backlinks: 5,
    quality: 80,
    excerpt: "盖碗是最通用的评茶与日常冲泡器具。注水、出汤、留根三个变量决定一泡茶的稳定与层次。",
    related: ["岩茶", "乌龙茶", "注水手法"],
    orphan: false,
  },
  {
    id: "w5",
    title: "岩茶",
    en: "Rock tea",
    cat: "茶类",
    updated: "昨天",
    words: 1520,
    links: 7,
    backlinks: 6,
    quality: 90,
    excerpt: "武夷岩茶以岩骨花香著称。山场、焙火与品种共同构成其复杂度，肉桂、水仙为当家品种。",
    related: ["杀青", "盖碗冲泡", "焙火", "肉桂"],
    orphan: false,
  },
  {
    id: "w6",
    title: "普洱茶",
    en: "Pu-erh",
    cat: "茶类",
    updated: "上周",
    words: 1890,
    links: 8,
    backlinks: 7,
    quality: 93,
    excerpt: "以云南大叶种晒青毛茶为原料，分生普与熟普。越陈越香的转化潜力使其兼具品饮与收藏价值。",
    related: ["普洱干仓仓储", "陈化曲线", "茶山"],
    orphan: false,
  },
  {
    id: "w7",
    title: "焙火",
    en: "Roasting",
    cat: "工艺",
    updated: "上周",
    words: 640,
    links: 2,
    backlinks: 2,
    quality: 72,
    excerpt: "焙火通过热作用调整茶叶含水量与风味走向，轻火清香、足火醇厚。",
    related: ["岩茶"],
    orphan: false,
  },
  {
    id: "w8",
    title: "紫砂壶养护",
    en: "Yixing care",
    cat: "器具",
    updated: "2 周前",
    words: 540,
    links: 0,
    backlinks: 0,
    quality: 58,
    excerpt: "一壶一茶、勤泡勤养。日常以热水内外冲淋，避免油污与异味，长期形成温润包浆。",
    related: [],
    orphan: true,
  },
  {
    id: "w9",
    title: "茶多酚",
    en: "Tea polyphenols",
    cat: "成分",
    updated: "3 周前",
    words: 420,
    links: 0,
    backlinks: 1,
    quality: 64,
    excerpt: "茶多酚是茶叶中主要的活性物质，影响涩感与抗氧化特性。加工中的氧化程度决定其最终含量。",
    related: ["杀青"],
    orphan: false,
  },
  {
    id: "w10",
    title: "冷萃",
    en: "Cold brew",
    cat: "冲泡",
    updated: "1 个月前",
    words: 360,
    links: 0,
    backlinks: 0,
    quality: 46,
    excerpt: "低温长时间萃取，苦涩物质析出少，口感清甜柔和，适合夏季与办公场景。",
    related: [],
    orphan: true,
  },
];

export const WIKI_CATS = ["全部", "工艺", "茶类", "仓储", "冲泡", "器具", "成分"];

export const GRAPH_DATA = {
  nodes: [
    { id: "yancha", label: "岩茶", cat: "茶类", x: 50, y: 34 },
    { id: "puer", label: "普洱茶", cat: "茶类", x: 72, y: 55 },
    { id: "shaqing", label: "杀青", cat: "工艺", x: 34, y: 20 },
    { id: "weidiao", label: "萎凋", cat: "工艺", x: 22, y: 40 },
    { id: "gaiwan", label: "盖碗冲泡", cat: "冲泡", x: 46, y: 62 },
    { id: "dry", label: "干仓仓储", cat: "仓储", x: 86, y: 40 },
    { id: "roast", label: "焙火", cat: "工艺", x: 62, y: 18 },
    { id: "poly", label: "茶多酚", cat: "成分", x: 16, y: 16 },
    { id: "zisha", label: "紫砂壶", cat: "器具", x: 14, y: 74 },
    { id: "cold", label: "冷萃", cat: "冲泡", x: 40, y: 86 },
  ],
  edges: [
    ["yancha", "shaqing"],
    ["yancha", "gaiwan"],
    ["yancha", "roast"],
    ["shaqing", "weidiao"],
    ["shaqing", "poly"],
    ["puer", "dry"],
    ["gaiwan", "puer"],
    ["yancha", "puer"],
    ["weidiao", "gaiwan"],
    ["zisha", "puer"],
    ["cold", "gaiwan"],
  ],
} as const;

export const GRAPH_CATS: Record<string, string> = {
  茶类: "var(--ai)",
  工艺: "var(--brand)",
  仓储: "#9b5a6b",
  冲泡: "var(--gold)",
  成分: "#5e7e8b",
  器具: "#7a5ae0",
};

export type Plugin = {
  id: string;
  name: string;
  cat: "处理" | "存储" | "发布" | "Bot";
  vendor: string;
  installed: boolean;
  enabled: boolean;
  icon: IconName;
  cloud: boolean;
  blurb: string;
  rating: number;
  installs: string;
  accent: string;
};

export const PLUGINS: Plugin[] = [
  {
    id: "p1",
    name: "Whisper 转写引擎",
    cat: "处理",
    vendor: "OpenAI · 官方",
    installed: true,
    enabled: true,
    icon: "audio",
    cloud: true,
    blurb: "把上传的音频/视频自动转写为带时间戳的文本，支持中英混合与方言识别。",
    rating: 4.8,
    installs: "12.4k",
    accent: "var(--brand)",
  },
  {
    id: "p2",
    name: "本地向量库",
    cat: "存储",
    vendor: "ExpertBase",
    installed: true,
    enabled: true,
    icon: "db",
    cloud: false,
    blurb: "将 Wiki 条目向量化并存于本地，离线可检索，数据不出本机。",
    rating: 4.9,
    installs: "9.1k",
    accent: "var(--ai)",
  },
  {
    id: "p3",
    name: "LINE 机器人",
    cat: "Bot",
    vendor: "ExpertBase",
    installed: true,
    enabled: true,
    icon: "chat",
    cloud: true,
    blurb: "把 Wiki 一键发布为 LINE 官方账号机器人，会员可用自然语言问答。",
    rating: 4.7,
    installs: "6.8k",
    accent: "#06c755",
  },
  {
    id: "p4",
    name: "OCR 文档识别",
    cat: "处理",
    vendor: "Community",
    installed: true,
    enabled: false,
    icon: "scan",
    cloud: false,
    blurb: "识别扫描件、图片中的文字，自动归类到收集箱。",
    rating: 4.5,
    installs: "5.2k",
    accent: "#9b5a6b",
  },
  {
    id: "p10",
    name: "Vercel 发布",
    cat: "发布",
    vendor: "Vercel",
    installed: true,
    enabled: true,
    icon: "globe",
    cloud: true,
    blurb: "把展示层站点一键部署到 Vercel 全球 CDN，自带 HTTPS 与预览链接。",
    rating: 4.8,
    installs: "7.7k",
    accent: "#111111",
  },
  {
    id: "p12",
    name: "Claude 加工引擎",
    cat: "处理",
    vendor: "Anthropic",
    installed: true,
    enabled: true,
    icon: "spark",
    cloud: true,
    blurb: "用 Claude 把原始素材结构化、纠错、生成双向链接建议，质量更高。",
    rating: 4.9,
    installs: "10.2k",
    accent: "var(--brand)",
  },
  {
    id: "p6",
    name: "Telegram 机器人",
    cat: "Bot",
    vendor: "Community",
    installed: false,
    enabled: false,
    icon: "send",
    cloud: true,
    blurb: "与 LINE 插件同源，面向 Telegram 群组与频道提供问答服务。",
    rating: 4.4,
    installs: "3.3k",
    accent: "#2aabee",
  },
  {
    id: "p11",
    name: "Cloudflare R2",
    cat: "存储",
    vendor: "Cloudflare",
    installed: false,
    enabled: false,
    icon: "cloud",
    cloud: true,
    blurb: "零出口费用的对象存储，适合长期归档大体量音视频原始素材。",
    rating: 4.7,
    installs: "3.6k",
    accent: "#f38020",
  },
];

export const PLUGIN_CATS = ["全部", "处理", "存储", "发布", "Bot"] as const;

export const DEPLOY_HISTORY = [
  {
    ver: "v48",
    status: "live",
    target: "Vercel",
    when: "2 分钟前",
    dur: "18s",
    commit: "新增「岩茶」「焙火」等 5 条目",
  },
  {
    ver: "v47",
    status: "ready",
    target: "Vercel",
    when: "昨天 21:14",
    dur: "16s",
    commit: "优化首页 AI 搜索框文案",
  },
  {
    ver: "v46",
    status: "ready",
    target: "Vercel",
    when: "3 天前",
    dur: "21s",
    commit: "修复深色模式下的对比度",
  },
  {
    ver: "v45",
    status: "error",
    target: "Cloudflare",
    when: "上周",
    dur: "—",
    commit: "迁移到 Cloudflare Pages（构建失败）",
  },
];

export const BOTS = [
  {
    id: "b1",
    name: "茶语 · 会员顾问",
    channel: "LINE",
    status: "online",
    members: 328,
    msgs: "1,204",
    accent: "#06c755",
    desc: "面向付费会员的问答机器人，基于完整 Wiki，可推荐茶品与冲泡方案。",
  },
  {
    id: "b2",
    name: "内部草稿助手",
    channel: "Web",
    status: "draft",
    members: 0,
    msgs: "—",
    accent: "var(--ai)",
    desc: "仅自己可见，用于测试新条目入库后的问答质量。",
  },
];

export const BOT_CHAT = [
  { who: "user", text: "我买的2019年生普，现在能喝吗？还是要再放放？" },
  {
    who: "bot",
    text: "2019 年的生普现在正处于转化早期，可以喝，但风味还在变化中。若是名山料建议再存放，日常口粮现在喝刚好。",
    cite: ["普洱茶", "普洱干仓仓储"],
  },
  { who: "user", text: "那存的话湿度要注意什么？" },
  {
    who: "bot",
    text: "核心是干仓：相对湿度控制在 60-70%，温度稳定、避免阳光直射与异味。",
    cite: ["普洱干仓仓储"],
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
