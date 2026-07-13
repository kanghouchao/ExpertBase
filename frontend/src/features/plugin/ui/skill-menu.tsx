"use client";

import { useMemo, useState } from "react";

import type { Skill } from "@/shared/api";
import { useI18n } from "@/shared/providers/providers";
import { Icon } from "@/shared/ui/icon";
import { Input } from "@/shared/ui/input";
import {
  Popover,
  PopoverContent,
  PopoverHeader,
  PopoverTitle,
  PopoverTrigger,
} from "@/shared/ui/popover";
import { SkillPanel } from "./skill-panel";

// 検索窓を出す下限。数件だけなら検索は雑音、多いときだけ意味を持つ。
const SEARCH_THRESHOLD = 6;

// 発見済み技能への入口。ツールバーのアイコン一つに畳み、常駐する一覧はポップオーバーの中に
// 閉じ込める（技能が増えても入力欄を押し出さない・使わない大半のやり取りでは目に入らない）。
// 発動済み件数はアイコンのバッジで先に見せる＝開かなくても状態がわかる。
// 技能が 0 件のときはボタンごと出さない（誰も設定していない機能への案内は雑音）。
export function SkillMenu({
  skills,
  activatedNames,
  onActivate,
}: {
  skills: Skill[];
  activatedNames: string[];
  onActivate: (name: string) => void;
}) {
  const { t } = useI18n();
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return skills;
    return skills.filter(
      (skill) =>
        skill.name.toLowerCase().includes(q) || skill.description.toLowerCase().includes(q)
    );
  }, [skills, query]);

  if (skills.length === 0) return null;

  return (
    <Popover>
      <PopoverTrigger
        title={t("plugin.skills.trigger")}
        className="relative grid size-9 flex-none place-items-center rounded-[10px] border border-line-strong bg-surface text-ink-soft transition-colors hover:bg-surface-2 data-[popup-open]:border-ai data-[popup-open]:text-ai"
      >
        <Icon name="plug" size={17} />
        {activatedNames.length > 0 && (
          <span className="absolute -top-1.5 -right-1.5 grid min-w-4.5 place-items-center rounded-full bg-ai px-1 font-mono text-[10px] leading-4 font-bold text-white">
            {activatedNames.length}
          </span>
        )}
      </PopoverTrigger>
      <PopoverContent side="top" align="start" sideOffset={10} className="w-80 gap-0 p-0">
        <PopoverHeader className="gap-0 border-b border-line px-3.5 pt-3 pb-2.5">
          <div className="flex items-center gap-1.5">
            <PopoverTitle className="text-[13.5px] font-bold text-ink">
              {t("plugin.skills.trigger")}
            </PopoverTitle>
            <span className="font-mono text-[11px] text-ink-faint">{skills.length}</span>
          </div>
          {skills.length > SEARCH_THRESHOLD && (
            <div className="relative mt-2">
              <Icon
                name="search"
                size={13}
                className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-faint"
              />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t("plugin.skills.searchPlaceholder")}
                className="h-8 pl-7.5 text-[12.5px]"
              />
            </div>
          )}
        </PopoverHeader>
        <div className="max-h-[min(60vh,22rem)] overflow-y-auto p-2.5">
          {filtered.length === 0 ? (
            <p className="px-1.5 py-4 text-center text-[12.5px] text-ink-faint">
              {t("plugin.skills.noMatch")}
            </p>
          ) : (
            <SkillPanel skills={filtered} activatedNames={activatedNames} onActivate={onActivate} />
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
