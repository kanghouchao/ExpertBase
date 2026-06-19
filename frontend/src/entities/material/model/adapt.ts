// バックエンド（Rust 派生インデックス）の受信箱 DTO を表示用の素材モデルへ変換する薄い層。

import type { InboxItem } from "@/shared/api/tauri/client";
import type { RawMaterial, RawType, RawStatus } from "./types";

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
  const status: RawStatus =
    item.status === "processed"
      ? "processed"
      : item.status === "transcribed"
        ? "transcribed"
        : "pending";
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
