"use client";

import type { Skill } from "@/shared/api";
import { Tag } from "@/shared/ui/tag";
import { useI18n } from "@/shared/providers/providers";

// 技能の由来（KB / user）を示すタグ。skill-panel と skill-slash-menu が同じ
// 「由来 → ラベル + 色調」対応を別々に持っていたのでここへ一本化する。
export function SkillSourceTag({
  source,
  className,
}: {
  source: Skill["source"];
  className?: string;
}) {
  const { t } = useI18n();
  return (
    <Tag tone={source === "kb" ? "accent" : "muted"} className={className}>
      {source === "kb" ? t("plugin.skills.source.kb") : t("plugin.skills.source.user")}
    </Tag>
  );
}
