"use client";

import type { ReactNode } from "react";

import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";
import { useI18n } from "@/shared/providers/providers";
import { RAW_TYPE, type RawMaterial } from "@/entities/material";

// 右側のステータス面板。草稿は無くなったので、実行状態・モデル・在席素材だけを映す。
export function Inspector({
  model,
  generating,
  runningLabel,
  thinking,
  tools,
  sources,
}: {
  model: string;
  generating: boolean;
  runningLabel: string;
  thinking: boolean;
  tools: boolean;
  sources: RawMaterial[];
}) {
  const { t } = useI18n();
  const status = generating
    ? { label: runningLabel, color: "var(--gold)" }
    : { label: t("workshop.st.idle"), color: "var(--ink-muted)" };
  // lg では flex 行の既定 stretch で列いっぱいに伸びてしまうので、self-start で
  // 内容ぶんの高さに畳む（内容が収まらないときだけ max-h + 内部スクロール）。
  return (
    <aside className="flex w-full flex-none flex-col overflow-hidden rounded-2xl border border-line bg-surface shadow-(--shadow-sm) lg:max-h-full lg:w-80 lg:self-start">
      <div className="flex items-center gap-2.5 border-b border-line px-4.5 py-3.5">
        <Icon name="layers" size={16} className="text-ai" />
        <div className="text-[13.5px] font-bold text-ink">{t("workshop.insp.title")}</div>
        <div className="flex-1" />
        <span
          className="inline-flex items-center gap-1.5 font-mono text-[12px] font-bold"
          style={{ color: status.color }}
        >
          <span
            className="size-2 rounded-full"
            style={{
              background: status.color,
              animation: generating ? "pulseDot 1s infinite" : "none",
            }}
          />
          {status.label}
        </span>
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-4.5 pb-4">
        <InspRow label={t("workshop.insp.model")} first>
          <div className="flex items-center gap-2.5">
            <span className="grid size-7 flex-none place-items-center rounded-lg bg-ai-wash text-ai">
              <Icon name="bot" size={15} />
            </span>
            <div className="min-w-0">
              <div className="truncate text-[13.5px] font-semibold text-ink">
                {model || "Ollama"}
              </div>
              {(thinking || tools) && (
                <div className="mt-1 flex flex-wrap gap-1">
                  {thinking && <Tag tone="ai">{t("workshop.think.badge")}</Tag>}
                  {tools && <Tag tone="ai">{t("workshop.tools.badge")}</Tag>}
                </div>
              )}
            </div>
          </div>
        </InspRow>

        {sources.length > 0 && (
          <InspRow label={t("workshop.insp.sources")}>
            <div className="flex flex-col gap-2">
              {sources.map((s) => {
                const type = RAW_TYPE[s.type];
                return (
                  <div key={s.id} className="flex items-center gap-2.5">
                    <span
                      className="grid size-7 flex-none place-items-center rounded-lg bg-surface-2"
                      style={{ color: type.color }}
                    >
                      <Icon name={type.icon} size={14} />
                    </span>
                    <div className="min-w-0">
                      <div className="truncate text-[13px] font-semibold text-ink">{s.title}</div>
                      <div className="truncate font-mono text-[10.5px] text-ink-faint">
                        {s.source}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </InspRow>
        )}
      </div>
    </aside>
  );
}

function InspRow({
  label,
  children,
  first = false,
}: {
  label: string;
  children: ReactNode;
  first?: boolean;
}) {
  return (
    <div className={first ? "py-3" : "border-t border-line py-3"}>
      <div className="mb-2 font-mono text-[10.5px] font-bold tracking-widest text-ink-muted uppercase">
        {label}
      </div>
      {children}
    </div>
  );
}
