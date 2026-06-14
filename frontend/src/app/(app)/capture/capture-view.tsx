"use client";

import { useEffect, useState } from "react";
import Link from "next/link";

import { Icon, type IconName } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { EmptyState } from "@/components/eb/empty-state";
import { cn } from "@/lib/utils";
import { captureText, captureWeb, listInbox } from "@/lib/tauri/client";
import { inboxToMaterial } from "@/lib/data/adapt";
import { useKbStore } from "@/lib/kb/store";
import type { RawMaterial } from "@/lib/data/types";
import { MaterialRow } from "../_components/material-row";
import { SegTabs } from "../_components/seg-tabs";

const TABS = ["upload", "record", "manual", "web"] as const;
type Tab = (typeof TABS)[number];

// 入力欄に値を入れて UX を示すための中立的なプレースホルダー例。
const WEB_EXAMPLES = ["example.com/article", "blog.example.com/post", "docs.example.com/guide"];

export function CaptureView() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [tab, setTab] = useState<Tab>("manual");
  const [text, setText] = useState("");
  const [webUrl, setWebUrl] = useState("");
  const [depth, setDepth] = useState(1);
  const [readability, setReadability] = useState(true);
  const [items, setItems] = useState<RawMaterial[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      const inbox = await listInbox();
      setItems(inbox.map(inboxToMaterial));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const inbox = await listInbox();
      setItems(inbox.map(inboxToMaterial));
    })();
  }, [available]);

  const pending = items.filter((item) => item.status !== "processed");

  async function handleManualSave() {
    if (!text.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await captureText(text, "manual");
      setText("");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleWebCrawl() {
    if (!webUrl.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await captureWeb(webUrl.trim());
      setWebUrl("");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

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
              disabled
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
                disabled
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
                <Button size="sm" disabled={!available || !text.trim() || busy} onClick={handleManualSave}>
                  <Icon name="check" size={15} />
                  {t("capture.manual.save")}
                </Button>
              </div>
            </div>
          )}

          {tab === "web" && (
            <div>
              {/* URL 入力行 */}
              <div className="flex items-center gap-2.5 rounded-xl border border-line-strong bg-surface-2 py-1 pr-1 pl-3.5 transition-colors focus-within:border-ai">
                <Icon name="globe" size={19} className="flex-none text-ai" />
                <input
                  value={webUrl}
                  onChange={(event) => setWebUrl(event.target.value)}
                  placeholder={t("capture.web.placeholder")}
                  className="min-w-0 flex-1 bg-transparent py-2.25 font-mono text-[14.5px] text-ink outline-none placeholder:text-ink-faint"
                />
                <Button size="sm" disabled={!available || !webUrl.trim() || busy} onClick={handleWebCrawl}>
                  <Icon name="arrowR" size={15} />
                  {t("capture.web.crawl")}
                </Button>
              </div>

              {/* 抓取オプション（MVP では深さ 1・本文抽出のみ。多階層クロールは未実装） */}
              <div className="mt-3.25 flex flex-wrap items-center gap-4 pl-0.5">
                <div className="flex items-center gap-2">
                  <span className="text-[12.5px] text-ink-muted">{t("capture.web.depth")}</span>
                  <div className="inline-flex overflow-hidden rounded-[9px] border border-line-strong">
                    {[1, 2, 3].map((d) => (
                      <button
                        key={d}
                        type="button"
                        onClick={() => setDepth(d)}
                        className={cn(
                          "h-7 w-7.5 font-mono text-[13px] font-semibold transition-colors",
                          d > 1 && "border-l border-line",
                          depth === d ? "bg-ai text-white" : "bg-surface text-ink-muted"
                        )}
                      >
                        {d}
                      </button>
                    ))}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setReadability((value) => !value)}
                  className={cn(
                    "flex items-center gap-1.75 text-[12.5px] font-semibold transition-colors",
                    readability ? "text-ai" : "text-ink-faint"
                  )}
                >
                  <span
                    className={cn(
                      "relative h-4.5 w-8 flex-none rounded-full transition-colors",
                      readability ? "bg-ai" : "bg-line-strong"
                    )}
                  >
                    <span
                      className={cn(
                        "absolute top-0.5 size-3.5 rounded-full bg-white transition-[left]",
                        readability ? "left-4" : "left-0.5"
                      )}
                    />
                  </span>
                  {t("capture.web.readability")}
                </button>
              </div>

              {/* 入力例 */}
              <div className="mt-4.5 flex flex-wrap items-center gap-2">
                <span className="font-mono text-xs text-ink-faint">{t("capture.web.examples")}</span>
                {WEB_EXAMPLES.map((example) => (
                  <button
                    key={example}
                    type="button"
                    onClick={() => setWebUrl(`https://${example}`)}
                    className="rounded-full border border-line bg-surface px-3 py-1.25 font-mono text-xs text-ink-soft transition-colors hover:bg-surface-2"
                  >
                    {example}
                  </button>
                ))}
              </div>

              <div className="mt-4 flex items-center gap-2.25">
                <Icon name="plug" size={15} className="flex-none text-ai" />
                <span className="text-[12.5px] text-ink-muted">{t("capture.web.hint")}</span>
              </div>
            </div>
          )}

          {error && <div className="mt-3 text-[12.5px] font-semibold text-brand">{error}</div>}
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
