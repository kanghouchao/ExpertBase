import { DICT, type Lang } from "./dictionaries";

export type Translate = (key: string) => string;

// Returns a translator bound to `lang`, falling back lang → zh → the key itself.
export function createT(lang: Lang): Translate {
  const dict = DICT[lang] ?? DICT.zh;
  return (key) => dict[key] ?? DICT.zh[key] ?? key;
}
