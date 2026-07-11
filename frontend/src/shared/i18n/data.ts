import type { Translate } from "@/shared/i18n/translate";

// カテゴリはユーザーデータ由来のため翻訳しない。「全部」センチネルだけ UI 言語に追従する。
export function wikiCategoryLabel(category: string, t: Translate): string {
  return category === "全部" ? t("cat.all") : category;
}
