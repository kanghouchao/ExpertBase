import type { Skill } from "@/shared/api";

// name / description の部分一致（大文字・小文字を無視）で技能を絞り込む純関数。
// workshop の `/` スラッシュ入力（use-skill-slash）が使う。
export function filterSkills(skills: Skill[], query: string): Skill[] {
  const q = query.trim().toLowerCase();
  if (!q) return skills;
  return skills.filter(
    (skill) => skill.name.toLowerCase().includes(q) || skill.description.toLowerCase().includes(q)
  );
}
