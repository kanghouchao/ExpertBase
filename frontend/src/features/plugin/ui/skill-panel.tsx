"use client";

import { useI18n } from "@/shared/providers/providers";
import type { Skill } from "@/shared/api";
import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";

import { SkillSourceTag } from "./skill-source-tag";

// 発見済み Agent Skill の行の並び（設定ダイアログの「発見できている技能を眺めるだけ」用途、
// 読み取り専用）。判定・状態は持たない提示コンポーネント（source-chip.tsx と同じ流儀）。
// 発動そのものは入力欄の `/` スラッシュコマンド（#44、use-skill-slash.ts）の管轄。
export function SkillPanel({ skills }: { skills: Skill[] }) {
  const { t } = useI18n();

  return (
    <div className="flex flex-col gap-1.5">
      {skills.map((skill) => (
        <div key={skill.name} className="rounded-lg border border-line bg-surface-2 px-3 py-2">
          <div className="flex flex-wrap items-center gap-1.5">
            <Icon name="plug" size={13} className="flex-none text-ink-faint" />
            <span className="truncate text-[12.5px] font-semibold text-ink">{skill.name}</span>
            <SkillSourceTag source={skill.source} />
            {skill.hasScripts && <Tag tone="line">{t("plugin.skills.scriptsNotExecuted")}</Tag>}
          </div>
          <p className="mt-1 text-[12px] leading-relaxed text-ink-muted">{skill.description}</p>
        </div>
      ))}
    </div>
  );
}
