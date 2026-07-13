"use client";

import { useI18n } from "@/shared/providers/providers";
import type { Skill } from "@/shared/api";
import { Button } from "@/shared/ui/button";
import { Icon } from "@/shared/ui/icon";
import { Tag } from "@/shared/ui/tag";

// 発見済み Agent Skill の行の並び。判定・状態は持たない提示コンポーネント
// （source-chip.tsx / tool-call-card.tsx と同じ流儀）。押すと onActivate(name) を呼ぶだけで、
// activatedNames への反映は呼び出し側（workshop-session.ts）が担う。外枠（カード・検索・空状態）は
// 呼び出し側の SkillMenu が持つ＝ここは行そのものの見た目にだけ責任を持つ。
// 入力欄スラッシュコマンド経由の発動（#44 の範囲）はこのコンポーネントの管轄外。
export function SkillPanel({
  skills,
  activatedNames,
  onActivate,
}: {
  skills: Skill[];
  activatedNames: string[];
  onActivate: (name: string) => void;
}) {
  const { t } = useI18n();

  return (
    <div className="flex flex-col gap-1.5">
      {skills.map((skill) => {
        const activated = activatedNames.includes(skill.name);
        return (
          <div key={skill.name} className="rounded-lg border border-line bg-surface-2 px-3 py-2">
            <div className="flex flex-wrap items-center gap-1.5">
              <Icon name="plug" size={13} className="flex-none text-ink-faint" />
              <span className="truncate text-[12.5px] font-semibold text-ink">{skill.name}</span>
              <Tag tone={skill.source === "kb" ? "accent" : "muted"}>
                {skill.source === "kb" ? t("plugin.skills.source.kb") : t("plugin.skills.source.user")}
              </Tag>
              {skill.hasScripts && <Tag tone="line">{t("plugin.skills.noScripts")}</Tag>}
            </div>
            <p className="mt-1 text-[12px] leading-relaxed text-ink-muted">{skill.description}</p>
            <div className="mt-1.5 flex justify-end">
              <Button
                type="button"
                size="xs"
                variant={activated ? "secondary" : "outline"}
                disabled={activated}
                onClick={() => onActivate(skill.name)}
              >
                {activated && <Icon name="check" size={12} />}
                {activated ? t("plugin.skills.activated") : t("plugin.skills.activate")}
              </Button>
            </div>
          </div>
        );
      })}
    </div>
  );
}
