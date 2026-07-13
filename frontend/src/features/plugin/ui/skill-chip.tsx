"use client";

import { Icon } from "@/shared/ui/icon";
import type { Skill } from "@/shared/api";

// コンポーザー内の発動済み技能チップ。source-chip.tsx と同じ形だが、素材（中立色）と見分ける
// ため AI 強調色を使う＝一目で「これは技能」とわかる。onRemove があれば × で取り消せる。
export function SkillChip({ skill, onRemove }: { skill: Skill; onRemove?: () => void }) {
  return (
    <span className="inline-flex max-w-65 items-center gap-2 rounded-[9px] border border-ai-soft bg-ai-wash py-1.5 pr-2 pl-2.5">
      <Icon name="plug" size={13} className="flex-none text-ai" />
      <span className="truncate text-[12.5px] font-semibold text-ai">{skill.name}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          title={skill.name}
          className="grid flex-none place-items-center text-ai/70 transition-colors hover:text-ai"
        >
          <Icon name="x" size={14} />
        </button>
      )}
    </span>
  );
}
