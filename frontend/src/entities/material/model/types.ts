// 工作坊に添付する素材（外部ファイル）のクライアントモデルと表示用メタデータ。
// UI コンポーネントは含まない（表示トークンのマッピングのみ）。

import type { IconName } from "@/shared/ui/icon";

export type RawType = "pdf" | "doc" | "note";

export type RawMaterial = {
  id: string;
  type: RawType;
  title: string;
  source: string;
};

// `color` は型アイコンへインラインで適用する CSS カラー（テーマ変数を含む）。
export const RAW_TYPE: Record<RawType, { icon: IconName; color: string }> = {
  pdf: { icon: "pdf", color: "#b5572f" },
  doc: { icon: "doc", color: "#5566b0" },
  note: { icon: "note", color: "var(--ai)" },
};
