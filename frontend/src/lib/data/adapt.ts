// バックエンド（Rust 派生インデックス）の型を、既存ビューが使う表示用型へ変換する薄い層。
// ビューの JSX を保ったまま実データを流すためのアダプタ。

import type { InboxItem, EntryRef } from "@/shared/api/tauri/client";
import type { RawMaterial, RawType, RawStatus, WikiEntry } from "@/lib/data/types";

// 受信箱の type（text/web/pdf/doc/audio/video/image）→ 表示用アイコン種別。
const TYPE_MAP: Record<string, RawType> = {
  text: "note",
  note: "note",
  web: "note",
  image: "note",
  pdf: "pdf",
  doc: "doc",
  audio: "audio",
  video: "video",
};

/** 受信箱素材を MaterialRow 用の表示型へ変換する。 */
export function inboxToMaterial(item: InboxItem): RawMaterial {
  const name = item.path.split("/").pop() ?? item.path;
  const status: RawStatus = item.status === "processed" ? "processed" : "pending";
  return {
    id: item.path,
    type: TYPE_MAP[item.type] ?? "note",
    title: name,
    source: item.source || item.type,
    date: item.capturedAt.slice(0, 10),
    status,
    size: "",
    preview: "",
    words: 0,
    tags: [],
  };
}

/** 条目参照を Wiki カード用の表示型へ変換する。本文・品質などは別途取得するまで中立値。 */
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
