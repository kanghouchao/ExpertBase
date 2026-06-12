"use client";

import { useState } from "react";
import Link from "next/link";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Button, buttonVariants } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { L } from "@/lib/data/overrides";
import { BOT_CHAT, BOTS } from "@/lib/data/mock";
import { cn } from "@/lib/utils";

export function BotsView() {
  const { t, lang } = useI18n();
  const [botId, setBotId] = useState(BOTS[0].id);
  const bot = BOTS.find((item) => item.id === botId) ?? BOTS[0];

  return (
    <div className="view-enter">
      <PageHead
        eyebrow={t("bots.eyebrow")}
        title={t("bots.title")}
        sub={t("bots.sub")}
        right={
          <Button>
            <Icon name="plus" size={17} />
            {t("bots.new")}
          </Button>
        }
      />
      <div className="grid grid-cols-[360px_1fr] gap-5">
        <div className="grid gap-3">
          {BOTS.map((item) => (
            <button key={item.id} onClick={() => setBotId(item.id)} className="text-left">
              <Panel className={cn(item.id === botId && "border-brand-soft")}>
                <div className="flex items-start gap-3">
                  <span
                    className="grid size-10 place-items-center rounded-[10px] text-white"
                    style={{ background: item.accent }}
                  >
                    <Icon name="bot" size={19} />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate font-semibold text-ink">
                      {L("bot", item, "name", lang)}
                    </div>
                    <p className="mt-1 line-clamp-2 text-[12.5px] leading-relaxed text-ink-muted">
                      {L("bot", item, "desc", lang)}
                    </p>
                  </div>
                  <Tag tone={item.status === "online" ? "ai" : "muted"}>
                    {t(item.status === "online" ? "bots.online" : "bots.draft")}
                  </Tag>
                </div>
                <div className="mt-4 flex gap-4 font-mono text-[11px] text-ink-faint">
                  <span>{item.channel}</span>
                  <span>{t("bots.members", { count: item.members })}</span>
                  <span>{t("bots.messages", { count: item.msgs })}</span>
                </div>
              </Panel>
            </button>
          ))}
          <Link
            href="/plugins"
            className={cn(buttonVariants({ variant: "outline" }), "border-line-strong bg-surface")}
          >
            <Icon name="plug" size={15} />
            {t("bots.addChannel")}
          </Link>
        </div>
        <div className="grid grid-cols-[1fr_360px] gap-5">
          <Panel>
            <div className="mb-5 flex items-start justify-between">
              <div>
                <h2 className="font-serif text-[30px] font-semibold text-ink">
                  {L("bot", bot, "name", lang)}
                </h2>
                <p className="mt-1 text-[13.5px] text-ink-muted">{L("bot", bot, "desc", lang)}</p>
              </div>
              <Tag tone={bot.status === "online" ? "ai" : "muted"}>
                {t(bot.status === "online" ? "bots.online.dot" : "bots.draft")}
              </Tag>
            </div>
            <div className="grid grid-cols-2 gap-3">
              {[
                "bots.policy.cite",
                "bots.policy.scope",
                "bots.policy.log",
                "bots.policy.handoff",
              ].map((item) => (
                <label
                  key={item}
                  className="flex items-center gap-2 rounded-lg border border-line bg-surface-2 px-3 py-2.5 text-[13px] font-semibold"
                >
                  <input type="checkbox" defaultChecked className="accent-brand" />
                  {t(item)}
                </label>
              ))}
            </div>
            <div className="mt-5 rounded-xl border border-ai-soft bg-ai-wash p-4 text-[13px] text-ink-soft">
              <div className="mb-1 flex items-center gap-2 font-bold text-ai">
                <Icon name="db" size={15} />
                {t("bots.rag.title")}
              </div>
              {t("bots.rag.body")}
            </div>
            <div className="mt-5 flex gap-3">
              <Button variant="outline" className="border-line-strong bg-surface">
                <Icon name="check" size={15} />
                {t("bots.saveDraft")}
              </Button>
              <Button>
                <Icon name="upload" size={15} />
                {t("bots.publish")}
              </Button>
            </div>
          </Panel>
          <Panel pad={0} className="overflow-hidden">
            <div className="border-b border-line px-4 py-3">
              <div className="font-semibold">{t("bots.preview")}</div>
              <div className="mt-0.5 font-mono text-[11px] text-ai">{t("bots.replySpeed")}</div>
            </div>
            <div className="grid gap-3 p-4">
              {BOT_CHAT.map((message, index) => (
                <div
                  key={index}
                  className={cn(
                    "max-w-[86%] rounded-2xl px-3.5 py-2.5 text-[13px] leading-relaxed",
                    message.who === "user"
                      ? "ml-auto bg-brand text-white"
                      : "bg-surface-2 text-ink-soft"
                  )}
                >
                  {message.text}
                  {message.cite && (
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {message.cite.map((cite) => (
                        <Tag key={cite} tone="ai">
                          [[{cite}]]
                        </Tag>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
            <div className="border-t border-line p-3">
              <div className="flex items-center gap-2 rounded-xl border border-line-strong bg-surface-2 px-3 py-2 text-[13px] text-ink-faint">
                {t("bots.ask")}
                <Icon name="send" size={15} className="ml-auto text-brand" />
              </div>
            </div>
          </Panel>
        </div>
      </div>
    </div>
  );
}
