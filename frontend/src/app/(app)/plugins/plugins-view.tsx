"use client";

import { useState } from "react";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import { L } from "@/lib/data/overrides";
import { PLUGIN_CATS, PLUGINS, type Plugin } from "@/lib/data/mock";
import { pluginCategoryLabel } from "@/lib/i18n/data";
import { cn } from "@/lib/utils";
import { SegTabs } from "../_components/seg-tabs";

function PluginCard({
  plugin,
  active,
  onSelect,
}: {
  plugin: Plugin;
  active: boolean;
  onSelect: () => void;
}) {
  const { t, lang } = useI18n();

  return (
    <button onClick={onSelect} className="text-left">
      <Panel hover className={cn("h-full", active && "border-brand-soft")}>
        <div className="mb-3 flex items-start gap-3">
          <span
            className="grid size-10 place-items-center rounded-[10px] text-white"
            style={{ background: plugin.accent }}
          >
            <Icon name={plugin.icon} size={19} />
          </span>
          <div className="min-w-0 flex-1">
            <div className="truncate font-semibold text-ink">
              {L("plugin", plugin, "name", lang)}
            </div>
            <div className="mt-0.5 font-mono text-[11px] text-ink-faint">{plugin.vendor}</div>
          </div>
          {plugin.installed && (
            <Tag tone={plugin.enabled ? "ai" : "muted"}>
              {t(plugin.enabled ? "plugins.enabled" : "plugins.installed")}
            </Tag>
          )}
        </div>
        <p className="line-clamp-2 text-[13px] leading-relaxed text-ink-muted">
          {L("plugin", plugin, "blurb", lang)}
        </p>
        <div className="mt-4 flex items-center gap-2 font-mono text-[11px] text-ink-faint">
          <span>★ {plugin.rating}</span>
          <span>·</span>
          <span>{t("plugins.installs", { count: plugin.installs })}</span>
          <span className="ml-auto">{t(plugin.cloud ? "plugins.cloud" : "plugins.local")}</span>
        </div>
      </Panel>
    </button>
  );
}

export function PluginsView() {
  const { t, lang } = useI18n();
  const [category, setCategory] = useState<(typeof PLUGIN_CATS)[number]>("全部");
  const [selectedId, setSelectedId] = useState(PLUGINS[0].id);
  const list = PLUGINS.filter((plugin) => category === "全部" || plugin.cat === category);
  const selected = list.find((plugin) => plugin.id === selectedId) ?? list[0] ?? PLUGINS[0];

  return (
    <div className="view-enter">
      <PageHead eyebrow={t("plugins.eyebrow")} title={t("plugins.title")} sub={t("plugins.sub")} />
      <Panel className="mb-5.5">
        <div className="flex items-center gap-4">
          <div>
            <div className="font-serif text-[30px] leading-none font-bold text-brand">
              {PLUGINS.filter((plugin) => plugin.installed).length}
            </div>
            <div className="mt-1 font-mono text-[11px] text-ink-faint">
              {t("plugins.installedCount")}
            </div>
          </div>
          <div className="h-11 w-px bg-line" />
          <div className="flex flex-wrap gap-2">
            {PLUGINS.filter((plugin) => plugin.installed).map((plugin) => (
              <Tag key={plugin.id} tone={plugin.enabled ? "ai" : "muted"}>
                <Icon name={plugin.icon} size={11} />
                {L("plugin", plugin, "name", lang)}
              </Tag>
            ))}
          </div>
        </div>
      </Panel>
      <div className="grid grid-cols-[1fr_360px] gap-5">
        <div>
          <div className="mb-4 flex items-center justify-between">
            <SegTabs
              tabs={PLUGIN_CATS}
              value={category}
              onChange={setCategory}
              label={(item) => pluginCategoryLabel(item, t)}
            />
            <span className="font-mono text-[12px] text-ink-faint">
              {t("plugins.count", { count: list.length })}
            </span>
          </div>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-3.5">
            {list.map((plugin) => (
              <PluginCard
                key={plugin.id}
                plugin={plugin}
                active={plugin.id === selected.id}
                onSelect={() => setSelectedId(plugin.id)}
              />
            ))}
          </div>
        </div>
        <Panel>
          <div className="mb-4 flex items-start gap-3">
            <span
              className="grid size-12 place-items-center rounded-xl text-white"
              style={{ background: selected.accent }}
            >
              <Icon name={selected.icon} size={22} />
            </span>
            <div className="min-w-0 flex-1">
              <h2 className="truncate text-lg font-bold text-ink">
                {L("plugin", selected, "name", lang)}
              </h2>
              <div className="mt-1 font-mono text-[11px] text-ink-faint">{selected.vendor}</div>
            </div>
          </div>
          <p className="text-[13.5px] leading-relaxed text-ink-muted">
            {L("plugin", selected, "blurb", lang)}
          </p>
          <div className="mt-5 grid grid-cols-2 gap-2">
            <div className="rounded-lg bg-surface-2 p-3">
              <div className="font-mono text-[11px] text-ink-faint">{t("plugins.capability")}</div>
              <div className="mt-1 font-semibold">{pluginCategoryLabel(selected.cat, t)}</div>
            </div>
            <div className="rounded-lg bg-surface-2 p-3">
              <div className="font-mono text-[11px] text-ink-faint">{t("plugins.location")}</div>
              <div className="mt-1 font-semibold">
                {t(selected.cloud ? "plugins.cloud" : "plugins.local")}
              </div>
            </div>
          </div>
          <div className="mt-5 rounded-xl border border-line bg-surface-2 p-4">
            <div className="mb-3 text-[13px] font-bold">{t("plugins.config")}</div>
            <label className="mb-3 block">
              <span className="mb-1 block font-mono text-[11px] text-ink-faint">API Key</span>
              <input
                className="w-full rounded-lg border border-line-strong bg-surface px-3 py-2 text-[13px] outline-none"
                placeholder="••••••••••••"
              />
            </label>
            <label className="block">
              <span className="mb-1 block font-mono text-[11px] text-ink-faint">
                {t("plugins.modelRegion")}
              </span>
              <input
                className="w-full rounded-lg border border-line-strong bg-surface px-3 py-2 text-[13px] outline-none"
                defaultValue={selected.cloud ? "ap-northeast-1" : "~/ExpertBase"}
              />
            </label>
          </div>
          <Button className="mt-5 w-full">
            <Icon name={selected.installed ? "check" : "plus"} size={15} />
            {t(selected.installed ? "plugins.save" : "plugins.install")}
          </Button>
        </Panel>
      </div>
    </div>
  );
}
