"use client";

import { useEffect, useState } from "react";

import { cn } from "@/shared/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Switch } from "@/shared/ui/switch";
import { Input } from "@/shared/ui/input";
import { SegTabs } from "@/shared/ui/seg-tabs";
import { ACCENTS, useI18n, useTheme } from "@/shared/providers/providers";
import { LANGS } from "@/shared/i18n/dictionaries";
import {
  getAiSettings,
  setAiSettings,
  type AiProvider,
  type AiSettings,
} from "@/shared/api/tauri/client";

const PROVIDERS = ["ollama", "llamaApp"] as const;
const PROVIDER_LABELS: Record<AiProvider, string> = {
  ollama: "Ollama",
  llamaApp: "llama.app",
};

// The product equivalent of the prototype's dev-only Tweaks panel + Profile
// modal: appearance (dark + accent) and interface language.
export function SettingsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { dark, setDark, accent, setAccent } = useTheme();
  const { lang, setLang, t } = useI18n();

  // AI 設定はダイアログを開くたびに読み直す。編集は即保存（テーマ/言語と同じ即時反映）。
  const [ai, setAi] = useState<AiSettings | null>(null);
  useEffect(() => {
    if (!open) return;
    void getAiSettings().then(setAi);
  }, [open]);

  function saveAi(next: AiSettings) {
    setAi(next);
    void setAiSettings(next);
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-sm">
        <DialogHeader>
          <DialogTitle>{t("cfg.title")}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-5">
          <div className="font-mono text-[10px] font-semibold tracking-[0.12em] text-ink-faint uppercase">
            {t("tw.theme")}
          </div>

          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{t("tw.dark")}</span>
            <Switch checked={dark} onCheckedChange={setDark} />
          </div>

          <div className="flex flex-col gap-2.5">
            <span className="text-sm font-medium">{t("tw.accent")}</span>
            <div className="flex gap-2.5">
              {ACCENTS.map((a) => (
                <button
                  key={a.value}
                  type="button"
                  title={a.value}
                  aria-pressed={accent === a.value}
                  onClick={() => setAccent(a.value)}
                  className={cn(
                    "size-8 rounded-full border border-black/10 transition-transform hover:scale-105",
                    accent === a.value && "ring-2 ring-ink ring-offset-2 ring-offset-surface"
                  )}
                  style={{ background: a.color }}
                />
              ))}
            </div>
          </div>

          <div className="flex flex-col gap-2.5">
            <span className="text-sm font-medium">{t("tw.lang")}</span>
            <div className="inline-flex self-start gap-0.75 rounded-[11px] border border-line bg-surface-2 p-0.75">
              {LANGS.map((l) => {
                const on = l.value === lang;
                return (
                  <button
                    key={l.value}
                    type="button"
                    aria-pressed={on}
                    onClick={() => setLang(l.value)}
                    className={cn(
                      "rounded-lg px-3 py-1.5 text-[13px] font-semibold transition-colors",
                      on ? "bg-surface text-ink shadow-(--shadow-sm)" : "text-ink-muted"
                    )}
                  >
                    {l.label}
                  </button>
                );
              })}
            </div>
          </div>

          {ai && (
            <>
              <div className="font-mono text-[10px] font-semibold tracking-[0.12em] text-ink-faint uppercase">
                {t("cfg.ai")}
              </div>

              <div className="flex flex-col gap-2.5">
                <span className="text-sm font-medium">{t("cfg.aiProvider")}</span>
                <SegTabs<AiProvider>
                  tabs={PROVIDERS}
                  value={ai.provider}
                  onChange={(provider) => saveAi({ ...ai, provider })}
                  label={(p) => PROVIDER_LABELS[p]}
                />
              </div>

              <div className="flex flex-col gap-2.5">
                <span className="text-sm font-medium">{t("cfg.aiModel")}</span>
                <Input
                  value={ai.model}
                  placeholder={ai.provider === "ollama" ? "qwen3:8b" : "qwen2.5"}
                  onChange={(event) => setAi({ ...ai, model: event.target.value })}
                  onBlur={() => saveAi(ai)}
                />
              </div>

              {ai.provider === "llamaApp" && (
                <div className="flex flex-col gap-2.5">
                  <span className="text-sm font-medium">{t("cfg.aiUrl")}</span>
                  <Input
                    value={ai.llamaAppUrl}
                    placeholder="http://127.0.0.1:8080/v1"
                    onChange={(event) => setAi({ ...ai, llamaAppUrl: event.target.value })}
                    onBlur={() => saveAi(ai)}
                  />
                </div>
              )}
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
