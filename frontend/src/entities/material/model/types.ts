// 受信箱素材のドメイン向けクライアントモデルと表示用メタデータ。
// UI コンポーネントは含まない（表示トークンのマッピングのみ）。

import type { IconName } from "@/shared/ui/icon";

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
