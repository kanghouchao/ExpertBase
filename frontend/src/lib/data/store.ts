// ナレッジベースのデータ供給層。
// 実データの読み書き（ファイル / DB）は未実装のため、現段階ではすべて空を返す。
// ビュー側はこのモジュールだけを参照し、実装が入っても差し替えが機械的に済むようにする。

import type { GraphData, RawMaterial, WikiEntry } from "@/lib/data/types";

export const RAW_MATERIALS: RawMaterial[] = [];

export const WIKI: WikiEntry[] = [];

export const GRAPH_DATA: GraphData = { nodes: [], edges: [] };

// 未加工素材数（サイドバーのバッジなどに使用）
export const PENDING = RAW_MATERIALS.filter((r) => r.status !== "processed").length;

export const STATS = {
  rawCount: RAW_MATERIALS.length,
  wikiCount: WIKI.length,
  links: GRAPH_DATA.edges.length,
  botMsgs: 0,
  members: 0,
};
