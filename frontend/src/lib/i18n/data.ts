import type { Translate } from "@/lib/i18n/translate";

// カテゴリはユーザーデータ由来のため翻訳しない。「全部」センチネルだけ UI 言語に追従する。
export function wikiCategoryLabel(category: string, t: Translate): string {
  return category === "全部" ? t("cat.all") : category;
}

export function qualityLabel(value: number, t: Translate): string {
  if (value >= 85) return t("quality.excellent");
  if (value >= 70) return t("quality.good");
  return t("quality.needsWork");
}
