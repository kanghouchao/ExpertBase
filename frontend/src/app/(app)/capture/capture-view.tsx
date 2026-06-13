"use client";

import { useState } from "react";
import Link from "next/link";

import { Icon, type IconName } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { EmptyState } from "@/components/eb/empty-state";
import { RAW_MATERIALS } from "@/lib/data/store";
import { MaterialRow } from "../_components/material-row";
import { SegTabs } from "../_components/seg-tabs";

const TABS = ["upload", "record", "manual"] as const;
type Tab = (typeof TABS)[number];

export function CaptureView() {
  const { t } = useI18n();
  const [tab, setTab] = useState<Tab>("upload");
  const [text, setText] = useState("");
  const items = RAW_MATERIALS;
  const pending = items.filter((item) => item.status !== "processed");
  const captureDisabled = true;

  return (
    <div className="view-enter mx-auto max-w-190">
      <PageHead eyebrow={t("capture.eyebrow")} title={t("capture.title")} sub={t("capture.sub")} />

      <Panel pad={0} className="mb-5 overflow-hidden">
        <div className="border-b border-line p-4">
          <SegTabs tabs={TABS} value={tab} onChange={setTab} label={(item) => t(`tabs.${item}`)} />
        </div>
        <div className="p-5.5">
          {tab === "upload" && (
            <button
              disabled={captureDisabled}
              className="w-full cursor-not-allowed rounded-2xl border-2 border-dashed border-line-strong bg-surface-2 px-6 py-12 text-center opacity-70 transition"
            >
              <span className="mx-auto mb-4 grid size-14 place-items-center rounded-[15px] bg-surface text-brand shadow-(--shadow-sm)">
                <Icon name="upload" size={26} />
              </span>
              <span className="block text-base font-semibold text-ink">
                {t("capture.upload.title")}
              </span>
              <span className="mt-1.5 block text-[13px] text-ink-muted">
                {t("capture.upload.sub")}
              </span>
              <span className="mt-2 block text-[12.5px] font-semibold text-brand">
                {t("capture.disabled")}
              </span>
              <span className="mt-4 flex justify-center gap-2 text-ink-muted">
                {(["pdf", "doc", "audio", "video", "scan", "note"] as IconName[]).map((icon) => (
                  <span
                    key={icon}
                    className="grid size-8.5 place-items-center rounded-lg border border-line bg-surface"
                  >
                    <Icon name={icon} size={17} />
                  </span>
                ))}
              </span>
            </button>
          )}

          {tab === "record" && (
            <div className="py-8 text-center">
              <button
                disabled={captureDisabled}
                className="mx-auto grid size-24 cursor-not-allowed place-items-center rounded-full bg-ink text-paper opacity-70 shadow-(--shadow-md)"
              >
                <Icon name="mic" size={34} />
              </button>
              <div className="mt-5 font-mono text-[28px] font-semibold tracking-[0.04em] text-ink">
                00:00
              </div>
              <div className="mt-2.5 text-[13.5px] text-ink-muted">{t("capture.record.tip")}</div>
              <div className="mt-2 text-[12.5px] font-semibold text-brand">
                {t("capture.disabled")}
              </div>
            </div>
          )}

          {tab === "manual" && (
            <div>
              <textarea
                value={text}
                onChange={(event) => setText(event.target.value)}
                placeholder={t("capture.manual.placeholder")}
                className="min-h-50 w-full resize-y rounded-xl border border-line-strong bg-surface-2 p-3.5 text-[14.5px] leading-relaxed text-ink outline-none"
              />
              <div className="mt-3 flex items-center justify-between">
                <span className="font-mono text-xs text-ink-faint">
                  {t("capture.manual.count", { count: text.length })}
                </span>
                <Button size="sm" disabled>
                  <Icon name="check" size={15} />
                  {t("capture.manual.save")}
                </Button>
              </div>
              <div className="mt-2 text-[12.5px] font-semibold text-brand">
                {t("capture.disabled")}
              </div>
            </div>
          )}
        </div>
      </Panel>

      <div className="mb-2.5 flex items-center justify-between px-0.5">
        <div className="flex items-center gap-2 font-mono text-[12.5px] font-semibold tracking-[0.04em] text-ink-muted">
          <Icon name="clock" size={14} />
          {t("capture.recent")}
          <span className="text-ink-faint">
            {t("capture.summary", { total: items.length, pending: pending.length })}
          </span>
        </div>
        <Link
          href="/workshop"
          className="flex items-center gap-1 text-[13px] font-semibold text-brand"
        >
          {t("capture.toWorkshop")} <Icon name="arrowR" size={14} />
        </Link>
      </div>
      <Panel pad={0}>
        {items.length === 0 && (
          <EmptyState icon="inbox" title={t("empty.materials")} sub={t("empty.materials.sub")} />
        )}
        {items.slice(0, 5).map((item) => (
          <MaterialRow key={item.id} material={item} />
        ))}
      </Panel>
    </div>
  );
}
