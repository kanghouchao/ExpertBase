"use client";

import { useMemo, useSyncExternalStore } from "react";

import { createT } from "@/shared/i18n/translate";
import type { Lang } from "@/shared/i18n/dictionaries";

export type Accent = "terracotta" | "amber" | "plum" | "indigo";

export const ACCENTS: { value: Accent; color: string }[] = [
  { value: "terracotta", color: "#c25a3a" },
  { value: "amber", color: "#c8902f" },
  { value: "plum", color: "#a0586e" },
  { value: "indigo", color: "#5566b0" },
];

const ACCENT_VALUES = new Set<string>(ACCENTS.map((a) => a.value));

type State = { dark: boolean; accent: Accent; lang: Lang };

// Stable server snapshot — the inline <head> script applies the real stored
// values before paint, so colors never flash; useSyncExternalStore reconciles
// the client snapshot after hydration without a mismatch warning.
const SERVER: State = { dark: false, accent: "terracotta", lang: "zh" };

let state: State | null = null;
const listeners = new Set<() => void>();

function readKey(k: string): string | null {
  try {
    return localStorage.getItem(k);
  } catch {
    return null;
  }
}

function load(): State {
  const lang = readKey("eb_lang");
  const accent = readKey("eb_accent");
  return {
    dark: readKey("eb_dark") === "1",
    accent: ACCENT_VALUES.has(accent ?? "") ? (accent as Accent) : "terracotta",
    lang: lang === "en" || lang === "ja" ? lang : "zh",
  };
}

function getSnapshot(): State {
  if (state === null) state = load();
  return state;
}

function getServerSnapshot(): State {
  return SERVER;
}

function subscribe(cb: () => void): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

function update(patch: Partial<State>): void {
  state = { ...getSnapshot(), ...patch };
  try {
    if (patch.dark !== undefined) localStorage.setItem("eb_dark", patch.dark ? "1" : "0");
    if (patch.accent !== undefined) localStorage.setItem("eb_accent", patch.accent);
    if (patch.lang !== undefined) localStorage.setItem("eb_lang", patch.lang);
  } catch {}
  const e = document.documentElement;
  e.classList.toggle("dark", state.dark);
  e.setAttribute("data-accent", state.accent);
  e.setAttribute("lang", state.lang);
  listeners.forEach((l) => l());
}

function usePrefs(): State {
  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

export function useTheme() {
  const { dark, accent } = usePrefs();
  return {
    dark,
    accent,
    setDark: (v: boolean) => update({ dark: v }),
    setAccent: (v: Accent) => update({ accent: v }),
  };
}

export function useI18n() {
  const { lang } = usePrefs();
  const t = useMemo(() => createT(lang), [lang]);
  return { lang, t, setLang: (v: Lang) => update({ lang: v }) };
}
