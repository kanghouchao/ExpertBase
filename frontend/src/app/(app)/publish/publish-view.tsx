"use client";

import { useMemo, useState } from "react";
import Link from "next/link";

import { Icon, type IconName } from "@/shared/ui/icon";
import { PageHead } from "@/shared/ui/page-head";
import { Panel } from "@/shared/ui/panel";
import { EmptyState } from "@/shared/ui/empty-state";
import { Tag } from "@/shared/ui/tag";
import { Logo } from "@/shared/ui/logo";
import { cn } from "@/shared/lib/utils";
import { useI18n } from "@/shared/providers/providers";
import { useKbStore } from "@/entities/knowledge-base";
import { WIKI } from "@/entities/wiki-entry";
import { wikiCategoryLabel } from "@/shared/i18n/data";
import { SegTabs } from "@/shared/ui/seg-tabs";

// 発行コンソールの固定オプション（プロダクト構成であってモックデータではない）。
type CloudTarget = { id: string; name?: string; icon: IconName; accent: string; recommended?: boolean };
const CLOUD_TARGETS: CloudTarget[] = [
  { id: "vercel", name: "Vercel", icon: "globe", accent: "#111111", recommended: true },
  { id: "cf", name: "Cloudflare Pages", icon: "cloud", accent: "#f38020" },
  { id: "self", icon: "db", accent: "var(--ai)" },
];

type ExportFormat = { id: string; ext: string; icon: IconName };
const EXPORT_FORMATS: ExportFormat[] = [
  { id: "site", ext: "ZIP", icon: "code" },
  { id: "md", ext: "ZIP", icon: "doc" },
  { id: "pdf", ext: "PDF", icon: "pdf" },
  { id: "json", ext: "JSON", icon: "db" },
];

const AUTO_DELAYS = ["now", "5m", "30m", "day"] as const;
type AutoDelay = (typeof AUTO_DELAYS)[number];

type Mode = "web" | "export";

// ── ai / accent でトーン分けする小さなトグル（このビュー内で2回再利用） ──
function PubToggle({
  on,
  onClick,
  tone = "ai",
}: {
  on: boolean;
  onClick: () => void;
  tone?: "ai" | "accent";
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      onClick={onClick}
      className={cn(
        "relative h-6 w-10.5 flex-none rounded-full transition-colors",
        on ? (tone === "ai" ? "bg-ai" : "bg-brand") : "bg-line-strong"
      )}
    >
      <span
        className={cn(
          "absolute top-0.5 size-5 rounded-full bg-white shadow-(--shadow-sm) transition-[left]",
          on ? "left-5" : "left-0.5"
        )}
      />
    </button>
  );
}

export function PublishView() {
  const { t } = useI18n();
  const { active } = useKbStore();

  const [mode, setMode] = useState<Mode>("web");
  const [platform, setPlatform] = useState("vercel");
  const [format, setFormat] = useState("site");
  const [botOn, setBotOn] = useState(false);
  const [autoOn, setAutoOn] = useState(false);
  const [autoDelay, setAutoDelay] = useState<AutoDelay>("5m");

  // 発行範囲は Wiki のカテゴリから導出する。実データが無いうちは空。
  const categories = useMemo(() => {
    const counts = new Map<string, number>();
    for (const entry of WIKI) counts.set(entry.cat, (counts.get(entry.cat) ?? 0) + 1);
    return [...counts].map(([cat, n]) => ({ cat, n }));
  }, []);
  const [scope, setScope] = useState<Record<string, boolean>>({});
  const allSelected = categories.length > 0 && categories.every((c) => scope[c.cat]);
  const total = categories.reduce((sum, c) => sum + (scope[c.cat] ? c.n : 0), 0);

  const platformName = (tg: CloudTarget) =>
    tg.id === "self" ? t("publish.target.self.name") : (tg.name ?? tg.id);
  const targetName =
    mode === "web"
      ? platformName(CLOUD_TARGETS.find((c) => c.id === platform) ?? CLOUD_TARGETS[0])
      : t(`publish.fmt.${format}`);

  const summary: [string, string][] = [
    [String(total), t("publish.sum.entries")],
    ["0", t("publish.sum.words")],
    ["—", t("publish.sum.size")],
    [botOn ? t("publish.bot.embedded") : t("publish.bot.static"), t("publish.sum.bot")],
  ];

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("publish.eyebrow")}
        title={t("publish.title")}
        sub={t("publish.sub")}
        right={
          <div className="flex items-center gap-2.25 rounded-[10px] border border-line-strong bg-surface px-3.25 py-2">
            <span className={cn("size-2 rounded-full", autoOn ? "bg-brand" : "bg-ink-faint")} />
            <span className="text-[13px] font-semibold text-ink-soft">
              {autoOn ? t("publish.watching") : t("publish.autoOff")}
            </span>
          </div>
        }
      />

      {/* ── 発行コンソール ── */}
      <Panel pad={0} className="mb-5 overflow-hidden">
        <div className="flex items-center gap-3 border-b border-line px-5.5 py-4">
          <Icon name="broadcast" size={18} className="text-brand" />
          <span className="text-[15.5px] font-bold text-ink">{t("publish.console")}</span>
          <div className="flex-1" />
          <SegTabs<Mode>
            tabs={["web", "export"] as const}
            value={mode}
            onChange={setMode}
            label={(v) => t(`publish.tab.${v}`)}
          />
        </div>

        <div className="grid grid-cols-[1.15fr_1fr]">
          {/* 左：発行先 + 範囲 */}
          <div className="border-r border-line px-5.5 py-5">
            <div className="mb-3.5 flex items-center gap-2">
              <span className="font-mono text-xs font-bold tracking-[0.08em] text-ink-muted uppercase">
                {mode === "web" ? t("publish.web.pick") : t("publish.export.pick")}
              </span>
              <Tag tone="muted" className="ml-auto">
                {mode === "web" ? t("publish.web.by") : t("publish.export.by")}
              </Tag>
            </div>

            {mode === "web" ? (
              <div className="flex flex-col gap-2.25">
                {CLOUD_TARGETS.map((tg) => {
                  const on = platform === tg.id;
                  return (
                    <button
                      key={tg.id}
                      type="button"
                      onClick={() => setPlatform(tg.id)}
                      className={cn(
                        "flex items-center gap-3 rounded-[11px] border px-3.25 py-2.75 text-left transition-colors",
                        on ? "border-ai bg-ai-wash" : "border-line bg-surface hover:bg-surface-2"
                      )}
                    >
                      <span
                        className="grid size-8.5 flex-none place-items-center rounded-[9px] text-white"
                        style={{ background: tg.accent }}
                      >
                        <Icon name={tg.icon} size={18} />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="flex items-center gap-1.75 text-[13.5px] font-semibold text-ink">
                          {platformName(tg)}
                          {tg.recommended && (
                            <span className="font-mono text-[10px] font-bold text-ai">
                              {t("publish.recommended")}
                            </span>
                          )}
                        </span>
                        <span className="mt-0.5 block truncate text-[11.5px] text-ink-muted">
                          {t(`publish.target.${tg.id}.desc`)}
                        </span>
                      </span>
                      <span
                        className={cn(
                          "grid size-4.5 flex-none place-items-center rounded-full border-2",
                          on ? "border-ai" : "border-line-strong"
                        )}
                      >
                        {on && <span className="size-2 rounded-full bg-ai" />}
                      </span>
                    </button>
                  );
                })}
              </div>
            ) : (
              <div className="grid grid-cols-2 gap-2.25">
                {EXPORT_FORMATS.map((f) => {
                  const on = format === f.id;
                  return (
                    <button
                      key={f.id}
                      type="button"
                      onClick={() => setFormat(f.id)}
                      className={cn(
                        "flex items-start gap-2.75 rounded-xl border px-3.5 py-3.25 text-left transition-colors",
                        on ? "border-brand bg-brand-wash" : "border-line bg-surface hover:bg-surface-2"
                      )}
                    >
                      <span className="grid size-8.5 flex-none place-items-center rounded-[9px] border border-line bg-surface-2 text-ink-soft">
                        <Icon name={f.icon} size={17} />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="flex items-center gap-1.75">
                          <span className="text-[13.5px] font-semibold text-ink">
                            {t(`publish.fmt.${f.id}`)}
                          </span>
                          <span className="rounded-[5px] border border-line-strong px-1.25 font-mono text-[10px] font-bold text-ink-faint">
                            {f.ext}
                          </span>
                        </span>
                        <span className="mt-0.75 block text-[11.5px] leading-snug text-ink-muted">
                          {t(`publish.fmt.${f.id}.d`)}
                        </span>
                      </span>
                      {on && <Icon name="check" size={16} className="flex-none text-brand" />}
                    </button>
                  );
                })}
              </div>
            )}

            {/* 発行範囲 */}
            <div className="mt-5 mb-2.75 flex items-center gap-2">
              <span className="font-mono text-xs font-bold tracking-[0.08em] text-ink-muted uppercase">
                {t("publish.scope")}
              </span>
              <span className="text-[11.5px] text-ink-faint">{t("publish.scope.pick")}</span>
              {categories.length > 0 && (
                <button
                  type="button"
                  onClick={() =>
                    setScope(() => {
                      const next: Record<string, boolean> = {};
                      for (const c of categories) next[c.cat] = !allSelected;
                      return next;
                    })
                  }
                  className="ml-auto font-mono text-xs font-semibold text-brand"
                >
                  {t("publish.scope.all")}
                </button>
              )}
            </div>
            {categories.length === 0 ? (
              <p className="rounded-[11px] border border-dashed border-line-strong bg-surface-2 px-3.5 py-3 text-[12.5px] leading-relaxed text-ink-muted">
                {t("empty.publish.scope")}
              </p>
            ) : (
              <div className="flex flex-wrap gap-2">
                {categories.map((c) => {
                  const sel = scope[c.cat];
                  return (
                    <button
                      key={c.cat}
                      type="button"
                      onClick={() => setScope((s) => ({ ...s, [c.cat]: !s[c.cat] }))}
                      className={cn(
                        "flex items-center gap-1.75 rounded-[9px] border px-2.75 py-1.75 text-[12.5px] font-semibold transition-colors",
                        sel
                          ? "border-brand-soft bg-brand-wash text-brand"
                          : "border-line-strong bg-surface text-ink-muted"
                      )}
                    >
                      <span
                        className={cn(
                          "grid size-3.75 flex-none place-items-center rounded-[4px] text-white",
                          sel ? "bg-brand" : "border-2 border-line-strong"
                        )}
                      >
                        {sel && <Icon name="check" size={11} />}
                      </span>
                      {wikiCategoryLabel(c.cat, t)}
                      <span className="font-mono text-[11px] opacity-80">{c.n}</span>
                    </button>
                  );
                })}
              </div>
            )}
          </div>

          {/* 右：サマリーと発行アクション（バックエンド未接続のため無効化） */}
          <div className="flex flex-col bg-surface-2 px-5.5 py-5">
            <div className="mb-3.25 font-mono text-xs font-bold tracking-[0.08em] text-ink-muted uppercase">
              {t("publish.summary")}
            </div>
            <div className="mb-auto grid grid-cols-2 gap-2.5">
              {summary.map(([value, label], i) => (
                <div key={label} className="rounded-[10px] border border-line bg-surface px-3.5 py-3">
                  <div
                    className={cn(
                      "leading-tight font-bold",
                      i === 3 ? "text-[15px]" : "font-mono text-xl",
                      i === 3 && botOn ? "text-ai" : "text-ink"
                    )}
                  >
                    {value}
                  </div>
                  <div className="mt-1 text-[11px] text-ink-faint">{label}</div>
                </div>
              ))}
            </div>

            <button
              type="button"
              disabled
              className="mt-4.5 flex w-full cursor-not-allowed items-center justify-center gap-2.25 rounded-[11px] bg-line-strong px-4.5 py-3.25 text-[15px] font-bold text-white opacity-80"
            >
              <Icon name={mode === "web" ? "broadcast" : "download"} size={18} />
              {t(`publish.btn.${mode}`, { target: targetName })}
            </button>
            <div className="mt-2.25 text-center text-[11.5px] font-semibold text-brand">
              {t("publish.disabled")}
            </div>
          </div>
        </div>
      </Panel>

      {/* ── サイトプレビュー（Web発行モードのみ） ── */}
      {mode === "web" && (
        <div className="mb-5">
          <div className="mx-0.5 mb-3 flex items-center gap-2.5">
            <Icon name="eye" size={16} className="text-ink-muted" />
            <span className="text-sm font-bold text-ink">{t("publish.preview.title")}</span>
            <span className="text-xs text-ink-faint">{t("publish.preview.sub")}</span>
          </div>
          <SitePreview siteName={active?.name ?? "ExpertBase"} />
        </div>
      )}

      {/* ── ボット埋め込み + 自動公開 ── */}
      <div className="mb-5 grid grid-cols-2 items-start gap-5">
        <BotCard on={botOn} setOn={setBotOn} />
        <AutoCard
          on={autoOn}
          setOn={setAutoOn}
          delay={autoDelay}
          setDelay={setAutoDelay}
          mode={mode}
          targetName={targetName}
        />
      </div>

      {/* ── 発行履歴 ── */}
      <Panel pad={0} className="overflow-hidden">
        <div className="flex items-center gap-2.5 border-b border-line px-5.5 py-3.75">
          <Icon name="clock" size={16} className="text-ink-muted" />
          <span className="text-sm font-bold text-ink">{t("publish.history")}</span>
        </div>
        <EmptyState
          icon="clock"
          title={t("empty.publish.history")}
          sub={t("empty.publish.history.sub")}
        />
      </Panel>
    </div>
  );
}

// 公開後に訪問者が見るサイトの外観。条目が無いうちは中身を空状態で示す。
function SitePreview({ siteName }: { siteName: string }) {
  const { t } = useI18n();
  return (
    <div className="overflow-hidden rounded-2xl border border-line-strong shadow-(--shadow-lg)">
      <div className="flex items-center gap-2 border-b border-line bg-surface-2 px-4 py-2.75">
        <div className="flex gap-1.75">
          <span className="size-2.75 rounded-full bg-[#e0796b]" />
          <span className="size-2.75 rounded-full bg-[#e3b04b]" />
          <span className="size-2.75 rounded-full bg-[#7fb069]" />
        </div>
        <div className="mx-auto flex max-w-90 flex-1 items-center justify-center gap-1.5 rounded-[7px] border border-line bg-surface px-3 py-1.25 font-mono text-xs text-ink-muted">
          <Icon name="shield" size={12} />
          {siteName}
        </div>
      </div>

      <div className="bg-paper">
        <div className="border-b border-line bg-linear-to-br from-brand-wash to-surface px-12 pt-12 pb-10">
          <div className="mb-6 flex items-center gap-2.5">
            <Logo size={28} />
            <span className="font-serif text-[19px] font-semibold text-ink">{siteName}</span>
            <div className="flex-1" />
            <span className="rounded-lg bg-ink px-3 py-1.5 text-[13px] font-semibold text-paper">
              {t("publish.preview.member")}
            </span>
          </div>
          <h1 className="mb-3 max-w-140 font-serif text-[40px] leading-[1.1] font-semibold tracking-[-0.02em] text-ink">
            {siteName}
          </h1>
          <div className="flex max-w-110 items-center gap-2.5 rounded-xl border border-line-strong bg-surface px-4 py-3 shadow-(--shadow-sm)">
            <Icon name="search" size={18} className="text-brand" />
            <span className="text-[14.5px] text-ink-muted">{t("publish.preview.search")}</span>
            <div className="flex-1" />
            <Tag tone="ai">
              <Icon name="spark" size={12} /> AI
            </Tag>
          </div>
        </div>
        <EmptyState icon="eye" title={t("empty.wiki")} sub={t("empty.publish.preview")} />
      </div>
    </div>
  );
}

// ── ボット埋め込みカード（任意）。詳細設定は「ボット」側にある ──
function BotCard({ on, setOn }: { on: boolean; setOn: (v: boolean) => void }) {
  const { t } = useI18n();
  return (
    <Panel pad={0} className={cn("overflow-hidden", on ? "border-ai-soft" : "border-line")}>
      <div className={cn("flex items-center gap-3 px-5 py-4", on && "border-b border-line")}>
        <span
          className={cn(
            "grid size-9.5 flex-none place-items-center rounded-[10px]",
            on ? "bg-ai text-white" : "border border-line bg-surface-2 text-ink-muted"
          )}
        >
          <Icon name="bot" size={20} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-[15px] font-bold text-ink">{t("publish.botcard.title")}</span>
            <Tag tone="muted">{t("publish.botcard.tag")}</Tag>
          </div>
          <div className="mt-0.75 text-xs leading-snug text-ink-muted">
            {t("publish.botcard.sub")}
          </div>
        </div>
        <PubToggle on={on} onClick={() => setOn(!on)} tone="ai" />
      </div>

      <div className="px-5 pt-3.5 pb-4.5">
        <div
          className={cn(
            "flex gap-2.25 rounded-[10px] px-3.25 py-2.75",
            on ? "mb-3.5 bg-ai-wash" : "bg-surface-2"
          )}
        >
          <Icon
            name={on ? "spark" : "shield"}
            size={16}
            className={cn("mt-0.25 flex-none", on ? "text-ai" : "text-ink-muted")}
          />
          <div className="text-[12.5px] leading-relaxed text-ink-soft">
            {on ? t("publish.botcard.on") : t("publish.botcard.off")}
          </div>
        </div>

        {on && (
          <Link
            href="/bots"
            className="flex items-center gap-2.75 rounded-[10px] border border-ai-soft bg-surface px-3.25 py-2.75 transition-colors hover:bg-ai-wash"
          >
            <span className="grid size-7.5 flex-none place-items-center rounded-lg bg-ai-wash text-ai">
              <Icon name="bot" size={16} />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-[13px] font-semibold text-ink">
                {t("publish.botcard.using")}
              </span>
              <span className="mt-0.5 block text-[11.5px] text-ink-faint">
                {t("publish.botcard.config")}
              </span>
            </span>
            <Icon name="arrowR" size={16} className="flex-none text-ai" />
          </Link>
        )}
      </div>
    </Panel>
  );
}

// ── 自動公開カード ──
function AutoCard({
  on,
  setOn,
  delay,
  setDelay,
  mode,
  targetName,
}: {
  on: boolean;
  setOn: (v: boolean) => void;
  delay: AutoDelay;
  setDelay: (v: AutoDelay) => void;
  mode: Mode;
  targetName: string;
}) {
  const { t } = useI18n();
  return (
    <Panel pad={0} className={cn("overflow-hidden", on ? "border-brand-soft" : "border-line")}>
      <div className={cn("flex items-center gap-3 px-5 py-4", on && "border-b border-line")}>
        <span
          className={cn(
            "grid size-9.5 flex-none place-items-center rounded-[10px]",
            on ? "bg-brand text-white" : "border border-line bg-surface-2 text-ink-muted"
          )}
        >
          <Icon name="zap" size={20} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="text-[15px] font-bold text-ink">{t("publish.auto.title")}</div>
          <div className="mt-0.75 text-xs leading-snug text-ink-muted">
            {t("publish.auto.sub")}
          </div>
        </div>
        <PubToggle on={on} onClick={() => setOn(!on)} tone="accent" />
      </div>

      <div className="px-5 pt-3.5 pb-4.5">
        {on ? (
          <div>
            <div className="flex items-center justify-between py-2.25">
              <span className="text-[12.5px] text-ink-muted">{t("publish.auto.trigger")}</span>
              <span className="text-[12.5px] font-semibold text-ink">
                {t("publish.auto.trigger.val")}
              </span>
            </div>
            <div className="flex items-center justify-between border-t border-line py-2.25">
              <span className="text-[12.5px] text-ink-muted">{t("publish.auto.target")}</span>
              <span className="flex items-center gap-1.5 text-[12.5px] font-semibold text-ink">
                <Icon
                  name={mode === "web" ? "broadcast" : "download"}
                  size={13}
                  className="text-brand"
                />
                {targetName}
              </span>
            </div>
            <div className="border-t border-line pt-3 pb-1">
              <div className="mb-2.25 text-[12.5px] text-ink-muted">{t("publish.auto.delay")}</div>
              <div className="flex flex-wrap gap-1.5">
                {AUTO_DELAYS.map((v) => {
                  const sel = delay === v;
                  return (
                    <button
                      key={v}
                      type="button"
                      onClick={() => setDelay(v)}
                      className={cn(
                        "rounded-lg border px-2.75 py-1.5 text-xs font-semibold transition-colors",
                        sel
                          ? "border-brand bg-brand-wash text-brand"
                          : "border-line-strong bg-surface text-ink-soft"
                      )}
                    >
                      {t(`publish.auto.delay.${v}`)}
                    </button>
                  );
                })}
              </div>
            </div>
            <div className="mt-3.5 flex gap-1.75 text-[11.5px] leading-relaxed text-ink-faint">
              <Icon name="shield" size={13} className="mt-0.25 flex-none" />
              {t("publish.auto.note")}
            </div>
          </div>
        ) : (
          <div className="flex items-center gap-2.25 text-[12.5px] text-ink-muted">
            <Icon name="clock" size={15} className="text-ink-faint" />
            {t("publish.auto.idle")}
          </div>
        )}
      </div>
    </Panel>
  );
}
