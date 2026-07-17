"use client";

import type { Skill } from "@/shared/api";
import { useI18n } from "@/shared/providers/providers";
import { SkillSourceTag } from "@/features/plugin";
import { Icon } from "@/shared/ui/icon";
import { cn } from "@/shared/lib/utils";

// 入力欄 `/` の技能候補ポップオーバー（issue #44）。入力欄自体に張り付く、
// キーボード操作前提の候補列。
export function SkillSlashMenu({
  matches,
  activeIndex,
  onSelect,
  onHover,
}: {
  /** null は「技能が 0 件」の案内表示、配列は候補一覧（0 件はありえない＝呼び出し側が閉じる）。 */
  matches: Skill[] | null;
  activeIndex: number;
  onSelect: (skill: Skill) => void;
  onHover?: (index: number) => void;
}) {
  const { t } = useI18n();

  return (
    <div className="absolute bottom-full left-0 z-10 mb-2 max-h-72 w-full overflow-y-auto rounded-2xl border border-line bg-surface p-1.5 shadow-(--shadow-lg)">
      {matches === null ? (
        <p className="px-3 py-3 text-center text-[12.5px] text-ink-faint">
          {t("plugin.skills.empty")}
        </p>
      ) : (
        matches.map((skill, index) => (
          <button
            key={skill.name}
            type="button"
            onClick={() => onSelect(skill)}
            onMouseEnter={() => onHover?.(index)}
            className={cn(
              "flex w-full items-center gap-2 rounded-xl px-2.5 py-2 text-left transition-colors",
              index === activeIndex ? "bg-ai-wash" : "hover:bg-surface-2"
            )}
          >
            <Icon
              name="plug"
              size={13}
              className={cn("flex-none", index === activeIndex ? "text-ai" : "text-ink-faint")}
            />
            <span
              className={cn(
                "flex-none truncate text-[12.5px] font-semibold",
                index === activeIndex ? "text-ai" : "text-ink"
              )}
            >
              {skill.name}
            </span>
            <SkillSourceTag source={skill.source} className="flex-none" />
            <span className="min-w-0 flex-1 truncate text-[12px] text-ink-muted">
              {skill.description}
            </span>
          </button>
        ))
      )}
    </div>
  );
}
