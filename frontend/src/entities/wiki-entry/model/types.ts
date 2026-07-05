// Wiki 条目のドメイン向けクライアントモデル。

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
