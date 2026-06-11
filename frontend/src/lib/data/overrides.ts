// English / Japanese overrides for structured data (the prototype's data_en.jsx).
// User-authored knowledge prose stays as written; only labels/summaries swap.
// `L()` picks an override for the active language, falling back to the base
// (zh) field. Phase 1 only needs the `lint` kind — extend per view as needed.

import type { Lang } from "@/lib/i18n/dictionaries";

type Override = Record<string, Record<string, Record<string, string>>>;

const EN: Override = {
  lint: {
    l1: { title: "Orphan entries", detail: "“Yixing care” and “Cold brew” have no bidirectional links — suggest linking them to related ware/brewing entries." },
    l2: { title: "Thin content", detail: "“Roasting” is only 640 words with a single citation — suggest adding rest cycles and roast levels." },
    l3: { title: "Likely duplicate", detail: "The oxidation passages in “Tea polyphenols” and “Kill-green” overlap heavily — merge or cross-reference." },
    l4: { title: "Stale", detail: "“Cold brew” hasn’t been updated in a month, while member questions about it are rising." },
  },
};

const JA: Override = {
  lint: {
    l1: { title: "孤立項目", detail: "「紫砂壺の手入れ」「水出し」に双方向リンクがありません。関連する茶器/淹れ方の項目への関連付けを推奨。" },
    l2: { title: "内容が薄い", detail: "「焙煎」は640文字で引用も単一。火抜き周期や火の強さの追記を推奨。" },
    l3: { title: "重複の疑い", detail: "「茶ポリフェノール」と「殺青」の酸化に関する記述が大きく重複。統合または相互参照を。" },
    l4: { title: "更新が古い", detail: "「水出し」は1か月未更新で、会員からの問い合わせは増加中。" },
  },
};

const OVERRIDES: Partial<Record<Lang, Override>> = { en: EN, ja: JA };

// L('lint', finding, 'title', lang) → localized value, falling back to base[field].
export function L<T extends { id: string }>(
  kind: string,
  obj: T,
  field: keyof T & string,
  lang: Lang
): string {
  const hit = OVERRIDES[lang]?.[kind]?.[obj.id]?.[field];
  return hit ?? String(obj[field]);
}
