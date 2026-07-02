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
  DEFAULT_AI_SETTINGS,
  DEFAULT_PROVIDER_URL,
  getAiSettings,
  listModels,
  setAiSettings,
  type AiProvider,
  type AiSettings,
  type OllamaModel,
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
  // 「検証」で発見したモデル（既定モデル入力の候補になる）と検証状態。provider/URL 変更で無効化。
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [verify, setVerify] = useState<"idle" | "loading" | "ok" | "error">("idle");
  useEffect(() => {
    if (!open) return;
    // 開くたびに設定を読み直す。読み込み失敗（不正 ai.toml 等）でも既定値へ倒し、AI 節が静かに
    // 消えないようにする。前回のモデル候補/検証状態は finally でクリア（effect 本体での同期 setState を避ける）。
    void getAiSettings()
      .then(setAi, () => setAi(DEFAULT_AI_SETTINGS))
      .finally(() => {
        setModels([]);
        setVerify("idle");
      });
  }, [open]);

  function saveAi(next: AiSettings) {
    setAi(next);
    void setAiSettings(next);
  }

  // provider ごとの URL（空欄=既定へフォールバック。表示はプレースホルダで既定を示す）。
  const currentUrl = ai ? (ai.provider === "ollama" ? ai.ollamaUrl : ai.llamaAppUrl) : "";
  function setUrl(url: string) {
    if (!ai) return;
    setAi(ai.provider === "ollama" ? { ...ai, ollamaUrl: url } : { ...ai, llamaAppUrl: url });
  }

  async function runVerify() {
    if (!ai) return;
    setVerify("loading");
    try {
      const list = await listModels(ai.provider, currentUrl);
      setModels(list);
      setVerify("ok");
    } catch {
      setModels([]);
      setVerify("error");
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[calc(100vh-2rem)] overflow-y-auto sm:max-w-sm">
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
                  onChange={(provider) => {
                    // provider を替えたら前 provider のモデル候補/検証結果は無効。
                    setModels([]);
                    setVerify("idle");
                    saveAi({ ...ai, provider });
                  }}
                  label={(p) => PROVIDER_LABELS[p]}
                />
              </div>

              <div className="flex flex-col gap-2.5">
                <span className="text-sm font-medium">{t("cfg.aiUrl")}</span>
                <div className="flex gap-2">
                  <Input
                    value={currentUrl}
                    placeholder={DEFAULT_PROVIDER_URL[ai.provider]}
                    onChange={(event) => {
                      setUrl(event.target.value);
                      setModels([]);
                      setVerify("idle");
                    }}
                    onBlur={() => saveAi(ai)}
                  />
                  <button
                    type="button"
                    onClick={runVerify}
                    disabled={verify === "loading"}
                    className="flex-none rounded-lg border border-line px-3 text-[13px] font-semibold text-ink-soft transition-colors hover:bg-surface-2 disabled:opacity-50"
                  >
                    {t("cfg.aiVerify")}
                  </button>
                </div>
                {verify === "ok" && (
                  <span className="text-xs text-ink-muted">
                    {t("cfg.aiVerifyOk")} · {models.length}
                  </span>
                )}
                {verify === "error" && (
                  <span className="text-xs text-destructive">{t("cfg.aiVerifyFail")}</span>
                )}
              </div>

              <div className="flex flex-col gap-2.5">
                <span className="text-sm font-medium">{t("cfg.aiModel")}</span>
                <Input
                  list="ai-model-options"
                  value={ai.model}
                  placeholder={ai.provider === "ollama" ? "qwen3:8b" : "qwen2.5"}
                  onChange={(event) => setAi({ ...ai, model: event.target.value })}
                  onBlur={() => saveAi(ai)}
                />
                <datalist id="ai-model-options">
                  {models.map((m) => (
                    <option key={m.name} value={m.name} />
                  ))}
                </datalist>
              </div>

              <div className="font-mono text-[10px] font-semibold tracking-[0.12em] text-ink-faint uppercase">
                {t("cfg.webSearch")}
              </div>

              <div className="flex flex-col gap-2.5">
                <span className="text-sm font-medium">{t("cfg.braveApiKey")}</span>
                <Input
                  type="password"
                  autoComplete="off"
                  value={ai.braveApiKey}
                  placeholder="BSA…"
                  onChange={(event) => setAi({ ...ai, braveApiKey: event.target.value })}
                  onBlur={() => saveAi(ai)}
                />
                <span className="text-xs text-ink-muted">{t("cfg.braveApiKeyHint")}</span>
              </div>
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
