import type { Translate } from "@/lib/i18n/translate";

const WIKI_CATEGORY_KEYS: Record<string, string> = {
  全部: "cat.all",
  茶类: "cat.teaType",
  工艺: "cat.craft",
  仓储: "cat.storageTea",
  冲泡: "cat.brewing",
  成分: "cat.component",
  器具: "cat.tool",
};

const PLUGIN_CATEGORY_KEYS: Record<string, string> = {
  全部: "cat.all",
  处理: "cat.process",
  存储: "cat.storage",
  发布: "cat.publish",
  Bot: "Bot",
};

export function wikiCategoryLabel(category: string, t: Translate): string {
  return t(WIKI_CATEGORY_KEYS[category] ?? category);
}

export function pluginCategoryLabel(category: string, t: Translate): string {
  return t(PLUGIN_CATEGORY_KEYS[category] ?? category);
}

export function qualityLabel(value: number, t: Translate): string {
  if (value >= 85) return t("quality.excellent");
  if (value >= 70) return t("quality.good");
  return t("quality.needsWork");
}
