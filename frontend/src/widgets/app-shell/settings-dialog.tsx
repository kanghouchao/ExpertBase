"use client";

import { cn } from "@/shared/lib/utils";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Switch } from "@/shared/ui/switch";
import { ACCENTS, useI18n, useTheme } from "@/shared/providers/providers";
import { LANGS } from "@/shared/i18n/dictionaries";

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
        </div>
      </DialogContent>
    </Dialog>
  );
}
