// ナレッジベースのドメイン型と表示用マッピング。
// 実データ層（store.ts）が実装されるまで、ビューはこの型に沿って空データを描画する。

import type { IconName } from "@/components/eb/icon";

export type RawType = "audio" | "video" | "pdf" | "doc" | "note";
export type RawStatus = "pending" | "transcribed" | "processed";

export type RawMaterial = {
  id: string;
  type: RawType;
  title: string;
  source: string;
  date: string;
  status: RawStatus;
  size: string;
  preview: string;
  words: number;
  tags: string[];
};

// `color` は型アイコンへインラインで適用する CSS カラー（テーマ変数を含む）。
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

export type GraphNode = {
  id: string;
  label: string;
  cat: string;
  x: number;
  y: number;
};

export type GraphData = {
  nodes: GraphNode[];
  edges: [string, string][];
};
