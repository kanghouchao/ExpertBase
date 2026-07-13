"use client";

import { useMemo, useState } from "react";

import type { Skill } from "@/shared/api";
import { filterSkills } from "@/features/plugin";

// 行頭の `/` を検出し、直後の空白を含まないトークンを検索クエリとして切り出す（純関数、DOM 非依存）。
// 非行頭の `/` や、`/` の後に空白を挟んで続けて打った場合は null（issue #44 要求6）。
export function detectSlashTrigger(
  value: string,
  cursor: number
): { lineStart: number; query: string } | null {
  const lineStart = value.lastIndexOf("\n", cursor - 1) + 1;
  if (value[lineStart] !== "/") return null;
  const token = value.slice(lineStart + 1, cursor);
  if (/\s/.test(token)) return null;
  return { lineStart, query: token };
}

export type SlashState =
  | { open: false }
  | { open: true; emptyReason: "no-skills"; lineStart: number }
  | { open: true; matches: Skill[]; activeIndex: number; lineStart: number };

/** 入力欄の `/` 技能発動メニュー（issue #44）の派生状態。value/cursor/skills から毎レンダー
 * 再計算し、方向キー用の activeIndex と Esc の一時的な非表示だけを内部に持つ。テキストの
 * 書き換え・技能の発動は呼び出し側（workshop-view.tsx、s.setInstruction / s.activateSkill を
 * 持つ）の責務＝このフックは「今どう見せるべきか」だけを答える。 */
export function useSkillSlash(params: { value: string; cursor: number; skills: Skill[] }): {
  state: SlashState;
  moveActive: (delta: 1 | -1) => void;
  activeSkill: Skill | null;
  close: () => void;
} {
  const { value, cursor, skills } = params;
  const trigger = useMemo(() => detectSlashTrigger(value, cursor), [value, cursor]);
  const matches = useMemo(
    () => (trigger && skills.length > 0 ? filterSkills(skills, trigger.query) : []),
    [trigger, skills]
  );

  const [activeIndex, setActiveIndex] = useState(0);
  // クエリ（＝候補集合）が変わったら選択位置を先頭へ戻す。effect ではなくレンダー中の
  // 「前回キーと比較して違えば即座に補正する」React 標準パターンで行う
  // （setState-in-effect による無駄な二度目のレンダーを避ける）。
  const [lastQueryKey, setLastQueryKey] = useState(trigger?.query ?? null);
  const queryKey = trigger?.query ?? null;
  if (queryKey !== lastQueryKey) {
    setLastQueryKey(queryKey);
    setActiveIndex(0);
  }

  // Esc で「今回だけ」閉じる。同じ value/cursor のままなら再度開かない。文字を打つ・カーソルが
  // 動くなど value/cursor が変われば自動的に無効化される（比較で一致しなくなるだけ、掃除不要）。
  const [dismissedAt, setDismissedAt] = useState<{ value: string; cursor: number } | null>(null);
  const dismissed =
    dismissedAt !== null && dismissedAt.value === value && dismissedAt.cursor === cursor;

  const state: SlashState = useMemo(() => {
    if (!trigger || dismissed) return { open: false };
    if (skills.length === 0) {
      return { open: true, emptyReason: "no-skills", lineStart: trigger.lineStart };
    }
    if (matches.length === 0) return { open: false };
    const clampedIndex = Math.min(activeIndex, matches.length - 1);
    return { open: true, matches, activeIndex: clampedIndex, lineStart: trigger.lineStart };
  }, [trigger, dismissed, skills.length, matches, activeIndex]);

  function moveActive(delta: 1 | -1): void {
    if (!(state.open && "matches" in state)) return;
    const len = state.matches.length;
    setActiveIndex((prev) => (prev + delta + len) % len);
  }

  const activeSkill =
    state.open && "matches" in state ? (state.matches[state.activeIndex] ?? null) : null;

  function close(): void {
    setDismissedAt({ value, cursor });
  }

  return { state, moveActive, activeSkill, close };
}
