"use client";

import { useState } from "react";

import { Icon } from "@/shared/ui/icon";
import { cn } from "@/shared/lib/utils";

import type { ToolEvent } from "../model/process-state";

// エージェントのツール呼び出し 1 件のカード(検索・読み取り・書き込み)。args は JSON 文字列なので値だけ抜いて表示。
// 結果(read_source なら素材本文)は長くなるので折りたたみ、ヘッダは浅色。クリックで展開する。
export function ToolCallCard({ tool }: { tool: ToolEvent }) {
  const [open, setOpen] = useState(false);
  let argText = tool.args;
  try {
    const parsed = JSON.parse(tool.args);
    argText = Object.values(parsed)
      .map((v) => String(v))
      .join(", ");
  } catch {
    /* JSON でなければ生文字列のまま表示 */
  }
  const icon = tool.name === "search_kb" || tool.name === "search_web" ? "search" : "doc";
  const hasResult = Boolean(tool.summary);
  return (
    <div className="overflow-hidden rounded-lg border border-line bg-surface-2 text-[12.5px]">
      <button
        type="button"
        onClick={() => hasResult && setOpen((prev) => !prev)}
        className={cn(
          "flex w-full items-center gap-2 px-3 py-1.5 text-left text-ink-faint",
          hasResult ? "cursor-pointer" : "cursor-default"
        )}
      >
        {hasResult && <Icon name={open ? "chevD" : "chevR"} size={12} className="flex-none" />}
        <Icon name={icon} size={13} className="flex-none text-ai" />
        <span className="font-mono font-semibold text-ink-soft">{tool.name}</span>
        {argText && <span className="truncate">{argText}</span>}
      </button>
      {open && hasResult && (
        <div className="max-h-48 overflow-auto border-t border-line px-3 py-2 font-mono text-[12px] leading-relaxed whitespace-pre-wrap wrap-break-word text-ink-soft">
          {tool.summary}
        </div>
      )}
    </div>
  );
}

// 完成したターンに残すツール呼び出しログ。
export function ToolCallLog({ tools }: { tools: ToolEvent[] }) {
  return (
    <div className="mb-2.5 flex flex-col gap-1.5">
      {tools.map((tool, idx) => (
        <ToolCallCard key={idx} tool={tool} />
      ))}
    </div>
  );
}
