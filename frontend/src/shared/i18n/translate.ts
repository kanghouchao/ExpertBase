import type { AppError } from "@/shared/api/tauri/client";

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

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && typeof (e as { code?: unknown }).code === "string";
}

/** Rust 側の AppError（{code, params}）を辞書で翻訳する。AppError でなければ String(e) にフォールバック。*/
export function translateError(t: Translate, e: unknown): string {
  if (isAppError(e)) return t(e.code, e.params);
  return String(e);
}
