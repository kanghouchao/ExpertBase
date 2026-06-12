import { DICT, type Lang } from "./dictionaries";

export type TranslateParams = Record<string, string | number>;
export type Translate = (key: string, params?: TranslateParams) => string;

const PARAM_RE = /\{(\w+)\}/g;

// Returns a translator bound to `lang`, falling back lang → zh → the key itself.
export function createT(lang: Lang): Translate {
  const dict = DICT[lang] ?? DICT.zh;
  return (key, params) => {
    const template = dict[key] ?? DICT.zh[key] ?? key;
    if (!params) return template;
    return template.replace(PARAM_RE, (_, name: string) =>
      params[name] === undefined ? `{${name}}` : String(params[name])
    );
  };
}
