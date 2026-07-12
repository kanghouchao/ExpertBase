"use client";

// workshop-session コントローラと React を接ぐ薄い hook。編排はここに書かない。
// 路由・KB ストアの変化を syncRoute へ転発し、快照を useSyncExternalStore で描くだけ。

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState, useSyncExternalStore } from "react";

import { useKbStore } from "@/entities/knowledge-base";

import { parseConversationId } from "./history";
import { createWorkshopSession } from "./workshop-session";

export function useWorkshopSession() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const { available, active } = useKbStore();
  const kbPath = active?.path ?? null;
  const conversationId = parseConversationId(searchParams.get("conversation"));

  // App Router の router は掛載間で安定参照 = 初回の閉包捕獲で足りる。
  const [session] = useState(() =>
    createWorkshopSession({ navigate: (url) => router.replace(url) })
  );

  // StrictMode の掛載→解除→再掛載でも実例は作り直さず、外部購読だけ付け外しする。
  useEffect(() => session.attach(), [session]);

  useEffect(() => {
    session.syncRoute({ kbPath, conversationId, available });
  }, [session, kbPath, conversationId, available]);

  const snapshot = useSyncExternalStore(
    session.subscribe,
    session.getSnapshot,
    session.getSnapshot
  );

  return {
    ...snapshot,
    setInstruction: session.setInstruction,
    selectModel: session.selectModel,
    toggleSource: session.toggleSource,
    addLocalFile: session.addLocalFile,
    send: session.send,
    stop: session.stop,
    confirm: session.confirm,
  };
}
