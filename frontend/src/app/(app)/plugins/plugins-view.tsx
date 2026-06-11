"use client";

import { useState } from "react";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Tag } from "@/components/eb/tag";
import { Button } from "@/components/ui/button";
import { PLUGIN_CATS, PLUGINS, type Plugin } from "@/lib/data/mock";
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
            <div className="truncate font-semibold text-ink">{plugin.name}</div>
            <div className="mt-0.5 font-mono text-[11px] text-ink-faint">{plugin.vendor}</div>
          </div>
          {plugin.installed && (
            <Tag tone={plugin.enabled ? "ai" : "muted"}>{plugin.enabled ? "已启用" : "已安装"}</Tag>
          )}
        </div>
        <p className="line-clamp-2 text-[13px] leading-relaxed text-ink-muted">{plugin.blurb}</p>
        <div className="mt-4 flex items-center gap-2 font-mono text-[11px] text-ink-faint">
          <span>★ {plugin.rating}</span>
          <span>·</span>
          <span>{plugin.installs} 安装</span>
          <span className="ml-auto">{plugin.cloud ? "云端" : "本地"}</span>
        </div>
      </Panel>
    </button>
  );
}

export function PluginsView() {
  const [category, setCategory] = useState<(typeof PLUGIN_CATS)[number]>("全部");
  const [selectedId, setSelectedId] = useState(PLUGINS[0].id);
  const list = PLUGINS.filter((plugin) => category === "全部" || plugin.cat === category);
  const selected = list.find((plugin) => plugin.id === selectedId) ?? list[0] ?? PLUGINS[0];

  return (
    <div className="view-enter">
      <PageHead
        eyebrow="插件市场 · Plugins"
        title="一切皆插件"
        sub="主系统只保留最小核心。数据处理、存储、Bot 接入全部以插件形式自由组合。"
      />
      <Panel className="mb-5.5">
        <div className="flex items-center gap-4">
          <div>
            <div className="font-serif text-[30px] leading-none font-bold text-brand">
              {PLUGINS.filter((plugin) => plugin.installed).length}
            </div>
            <div className="mt-1 font-mono text-[11px] text-ink-faint">个已安装</div>
          </div>
          <div className="h-11 w-px bg-line" />
          <div className="flex flex-wrap gap-2">
            {PLUGINS.filter((plugin) => plugin.installed).map((plugin) => (
              <Tag key={plugin.id} tone={plugin.enabled ? "ai" : "muted"}>
                <Icon name={plugin.icon} size={11} />
                {plugin.name}
              </Tag>
            ))}
          </div>
        </div>
      </Panel>
      <div className="grid grid-cols-[1fr_360px] gap-5">
        <div>
          <div className="mb-4 flex items-center justify-between">
            <SegTabs tabs={PLUGIN_CATS} value={category} onChange={setCategory} />
            <span className="font-mono text-[12px] text-ink-faint">{list.length} 个插件</span>
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
              <h2 className="truncate text-lg font-bold text-ink">{selected.name}</h2>
              <div className="mt-1 font-mono text-[11px] text-ink-faint">{selected.vendor}</div>
            </div>
          </div>
          <p className="text-[13.5px] leading-relaxed text-ink-muted">{selected.blurb}</p>
          <div className="mt-5 grid grid-cols-2 gap-2">
            <div className="rounded-lg bg-surface-2 p-3">
              <div className="font-mono text-[11px] text-ink-faint">能力</div>
              <div className="mt-1 font-semibold">{selected.cat}</div>
            </div>
            <div className="rounded-lg bg-surface-2 p-3">
              <div className="font-mono text-[11px] text-ink-faint">运行位置</div>
              <div className="mt-1 font-semibold">{selected.cloud ? "云端" : "本地"}</div>
            </div>
          </div>
          <div className="mt-5 rounded-xl border border-line bg-surface-2 p-4">
            <div className="mb-3 text-[13px] font-bold">配置参数</div>
            <label className="mb-3 block">
              <span className="mb-1 block font-mono text-[11px] text-ink-faint">API Key</span>
              <input
                className="w-full rounded-lg border border-line-strong bg-surface px-3 py-2 text-[13px] outline-none"
                placeholder="••••••••••••"
              />
            </label>
            <label className="block">
              <span className="mb-1 block font-mono text-[11px] text-ink-faint">模型 / 区域</span>
              <input
                className="w-full rounded-lg border border-line-strong bg-surface px-3 py-2 text-[13px] outline-none"
                defaultValue={selected.cloud ? "ap-northeast-1" : "~/ExpertBase"}
              />
            </label>
          </div>
          <Button className="mt-5 w-full">
            <Icon name={selected.installed ? "check" : "plus"} size={15} />
            {selected.installed ? "保存配置" : "安装插件"}
          </Button>
        </Panel>
      </div>
    </div>
  );
}
