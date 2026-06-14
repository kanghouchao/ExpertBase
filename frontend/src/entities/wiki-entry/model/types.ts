// Wiki 条目とグラフ描画のドメイン向けクライアントモデル。

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
