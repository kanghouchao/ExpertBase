// バックエンドの条目参照 DTO を Wiki カード用の表示型へ変換する薄い層。
// 本文・品質などは別途取得するまで中立値。

import type { EntryRef } from "@/shared/api/tauri/client";
import type { WikiEntry } from "./types";

/** 条目参照を Wiki カード用の表示型へ変換する。 */
export function entryRefToWiki(ref: EntryRef): WikiEntry {
  return {
    id: ref.path,
    title: ref.title,
    en: "",
    cat: ref.cat || "uncategorized",
    updated: "",
    words: 0,
    links: 0,
    backlinks: 0,
    quality: 0,
    excerpt: "",
    related: [],
    orphan: false,
  };
}
