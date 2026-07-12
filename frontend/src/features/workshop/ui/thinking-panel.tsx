"use client";

import { useEffect, useRef, useState } from "react";

import { Icon } from "@/shared/ui/icon";
import { useI18n } from "@/shared/providers/providers";

// 折りたたみパネル。思考トレース(推論・mono)を表示する。
// streaming 中は自動展開＋底部追従、終わったら自動的に「ラベル · N 字」へ折りたたむ(再展開可)。
export function ThinkingPanel({ text, streaming }: { text: string; streaming: boolean }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(streaming);
  const [wasStreaming, setWasStreaming] = useState(streaming);
  const bodyRef = useRef<HTMLDivElement>(null);
  // streaming の切替に追従して自動開閉(レンダー中の状態調整＝React 推奨パターン)。
  if (wasStreaming !== streaming) {
    setWasStreaming(streaming);
    setOpen(streaming);
  }
  // 流式中は本文を最下部に追従させる。
  useEffect(() => {
    if (streaming && bodyRef.current) bodyRef.current.scrollTop = bodyRef.current.scrollHeight;
  }, [text, streaming]);
  return (
    <div className="mb-2.5 overflow-hidden rounded-lg border border-ai-soft bg-ai-wash/40">
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left"
      >
        <Icon name={open ? "chevD" : "chevR"} size={13} className="flex-none text-ai" />
        <span className="text-[12px] font-bold text-ai">{t("workshop.think.label")}</span>
        {streaming ? (
          <span className="size-3 animate-spin rounded-full border-2 border-ai-soft border-t-ai" />
        ) : (
          <span className="font-mono text-[11px] text-ink-faint">· {text.length}</span>
        )}
      </button>
      {open && (
        <div
          ref={bodyRef}
          className="max-h-48 overflow-auto border-t border-ai-soft px-3 py-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap text-ink-soft"
        >
          {text}
        </div>
      )}
    </div>
  );
}
