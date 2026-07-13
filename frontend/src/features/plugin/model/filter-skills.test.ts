import { describe, expect, test } from "bun:test";

import type { Skill } from "@/shared/api";

import { filterSkills } from "./filter-skills";

function skill(name: string, description: string): Skill {
  return {
    name,
    description,
    body: "",
    location: `/skills/${name}/SKILL.md`,
    source: "kb",
    hasScripts: false,
  };
}

const SKILLS = [
  skill("tea-brewing", "緑茶の淹れ方を案内する"),
  skill("meeting-notes", "会議録音を整理する"),
];

describe("filterSkills", () => {
  test("空クエリは全件を返す", () => {
    expect(filterSkills(SKILLS, "")).toEqual(SKILLS);
    expect(filterSkills(SKILLS, "   ")).toEqual(SKILLS);
  });

  test("name の部分一致で絞り込む", () => {
    expect(filterSkills(SKILLS, "tea")).toEqual([SKILLS[0]]);
  });

  test("description の部分一致でも絞り込む", () => {
    expect(filterSkills(SKILLS, "会議")).toEqual([SKILLS[1]]);
  });

  test("大小写を無視する", () => {
    expect(filterSkills(SKILLS, "TEA")).toEqual([SKILLS[0]]);
  });

  test("一致が無ければ空配列", () => {
    expect(filterSkills(SKILLS, "coffee")).toEqual([]);
  });
});
