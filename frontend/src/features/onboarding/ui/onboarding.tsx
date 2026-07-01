"use client";

// ナレッジベース初期化ウィザード（docs/expertbase/project/views_onboarding.jsx が設計原典）。
// ナレッジベースが 1 つも無いときの初回起動画面であり、追加作成時のオーバーレイでもある。
// 原典の「AI 加工エンジン」「対外チャネル」ステップはプラグイン基盤の実装後に追加する。

import { useState } from "react";

import { Icon, type IconName } from "@/shared/ui/icon";
import { Logo } from "@/shared/ui/logo";
import { Button } from "@/shared/ui/button";
import { useI18n } from "@/shared/providers/providers";
import { LANGS } from "@/shared/i18n/dictionaries";
import { translateError } from "@/shared/i18n/translate";
import { createAndActivateKb, useKbStore } from "@/entities/knowledge-base";

const STEP_COUNT = 4;

export function Onboarding({ onCancel }: { onCancel?: () => void }) {
  const { t, lang, setLang } = useI18n();
  const { defaultParent } = useKbStore();

  const [step, setStep] = useState(0);
  const [name, setName] = useState("");
  const [desc, setDesc] = useState("");
  const [path, setPath] = useState("");
  const [pathTouched, setPathTouched] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ユーザーがパスを手で編集するまでは「親ディレクトリ/名前」を提案し続ける
  const suggestedPath = name.trim() ? `${defaultParent}/${name.trim()}` : defaultParent;
  const effectivePath = pathTouched ? path : suggestedPath;

  const steps = [t("ob.steps.0"), t("ob.steps.1"), t("ob.steps.2"), t("ob.steps.3")];
  const canContinue = step !== 1 || name.trim().length > 0;

  const next = () => setStep((s) => Math.min(STEP_COUNT - 1, s + 1));
  const back = () => setStep((s) => Math.max(0, s - 1));

  const finish = async () => {
    setBusy(true);
    setError(null);
    try {
      await createAndActivateKb({
        name: name.trim(),
        description: desc.trim(),
        path: effectivePath,
      });
      onCancel?.();
    } catch (e) {
      setError(translateError(t, e));
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-100 flex bg-paper">
      {/* ブランドレール */}
      <div className="flex w-75 flex-none flex-col border-r border-line bg-linear-170 from-brand-wash to-surface-2 px-8 py-10">
        <div className="mb-11 flex items-center gap-2.75">
          <Logo size={36} />
          <div>
            <div className="font-serif text-[20px] leading-none font-semibold">ExpertBase</div>
            <div className="mt-0.75 font-mono text-[10.5px] tracking-[0.06em] text-ink-muted">
              {t("ob.setup")}
            </div>
          </div>
          <div className="ml-auto flex gap-0.5 rounded-lg border border-line bg-surface-2 p-0.5">
            {LANGS.map((item) => (
              <button
                key={item.value}
                type="button"
                onClick={() => setLang(item.value)}
                className={
                  "rounded-md px-2 py-0.75 font-mono text-[11px] font-bold " +
                  (lang === item.value ? "bg-surface text-brand" : "text-ink-muted")
                }
              >
                {item.value === "zh" ? "中" : item.value === "en" ? "EN" : "日"}
              </button>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-1">
          {steps.map((label, i) => {
            const done = i < step;
            const cur = i === step;
            return (
              <div
                key={label}
                className={
                  "flex items-center gap-3 rounded-[10px] px-3 py-2.5 " +
                  (cur ? "bg-surface shadow-(--shadow-sm)" : "")
                }
              >
                <span
                  className={
                    "grid size-6 flex-none place-items-center rounded-full font-mono text-xs font-bold " +
                    (done
                      ? "bg-brand text-white"
                      : cur
                        ? "bg-brand-wash text-brand"
                        : "border border-line-strong bg-surface text-ink-muted")
                  }
                >
                  {done ? <Icon name="check" size={13} /> : i + 1}
                </span>
                <span
                  className={
                    "text-[13.5px] " +
                    (cur ? "font-bold text-ink" : done ? "font-medium text-ink-soft" : "text-ink-muted")
                  }
                >
                  {label}
                </span>
              </div>
            );
          })}
        </div>

        <div className="mt-auto text-[11.5px] leading-relaxed text-ink-faint">
          {t("ob.foot")}
          <br />
          {t("ob.foot2")}
        </div>
      </div>

      {/* ステップ内容 */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <div className="grid flex-1 place-items-center overflow-auto p-10">
          <div key={step} className="view-enter w-full max-w-140">
            {step === 0 && (
              <div className="text-center">
                <div className="mx-auto mb-6 grid size-19.5 place-items-center rounded-[20px] bg-linear-150 from-brand to-[#9a4329] text-white shadow-(--shadow-md)">
                  <Icon name="book" size={38} />
                </div>
                <h1 className="mb-3.5 font-serif text-[40px] leading-tight font-semibold tracking-[-0.02em] text-ink">
                  {t("ob.w.title.a")}
                  <br />
                  {t("ob.w.title.b")}
                  <span className="text-brand italic">{t("ob.w.title.c")}</span>
                </h1>
                <p className="mx-auto mb-8 max-w-105 text-[15.5px] leading-relaxed text-ink-soft">
                  {t("ob.w.sub")}
                </p>
                <div className="flex justify-center gap-4.5">
                  {(
                    [
                      ["inbox", t("ob.w.collect")],
                      ["spark", t("ob.w.work")],
                      ["graph", t("ob.w.link")],
                      ["bot", t("ob.w.serve")],
                    ] as [IconName, string][]
                  ).map(([icon, label]) => (
                    <div key={icon} className="flex flex-col items-center gap-1.75">
                      <span className="grid size-11.5 place-items-center rounded-xl border border-line bg-surface text-brand">
                        <Icon name={icon} size={22} />
                      </span>
                      <span className="text-xs font-semibold text-ink-muted">{label}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {step === 1 && (
              <div>
                <h2 className="mb-2.5 font-serif text-[30px] font-semibold text-ink">
                  {t("ob.n.title")}
                </h2>
                <p className="mb-7 text-[14.5px] text-ink-muted">{t("ob.n.sub")}</p>
                <label className="mb-2 block text-[13px] font-semibold text-ink">
                  {t("ob.n.name")}
                </label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  autoFocus
                  className="mb-5 w-full rounded-[11px] border border-line-strong bg-surface px-4 py-3.25 font-serif text-[17px] text-ink outline-none focus:border-brand"
                />
                <label className="mb-2 block text-[13px] font-semibold text-ink">
                  {t("ob.n.desc")}{" "}
                  <span className="font-normal text-ink-faint">{t("ob.n.optional")}</span>
                </label>
                <textarea
                  value={desc}
                  onChange={(e) => setDesc(e.target.value)}
                  className="min-h-22.5 w-full resize-y rounded-[11px] border border-line-strong bg-surface px-4 py-3.25 text-[14.5px] leading-relaxed text-ink outline-none focus:border-brand"
                />
              </div>
            )}

            {step === 2 && (
              <div>
                <h2 className="mb-2.5 font-serif text-[30px] font-semibold text-ink">
                  {t("ob.l.title")}
                </h2>
                <p className="mb-6 text-[14.5px] text-ink-muted">{t("ob.l.sub")}</p>
                <div className="mb-5 flex items-center gap-3.5 rounded-[13px] border-[1.5px] border-brand bg-brand-wash px-4.25 py-3.75">
                  <span className="grid size-10.5 flex-none place-items-center rounded-[11px] bg-ai text-white">
                    <Icon name="db" size={21} />
                  </span>
                  <div className="flex-1">
                    <div className="text-[14.5px] font-semibold text-ink">{t("ob.l.local")}</div>
                    <div className="mt-0.5 text-[12.5px] leading-relaxed text-ink-muted">
                      {t("ob.l.local.d")}
                    </div>
                  </div>
                  <span className="grid size-5.5 flex-none place-items-center rounded-full bg-brand text-white">
                    <Icon name="check" size={13} />
                  </span>
                </div>
                <label className="mb-2 block text-[13px] font-semibold text-ink">
                  {t("ob.l.path")}
                </label>
                <input
                  value={effectivePath}
                  onChange={(e) => {
                    setPathTouched(true);
                    setPath(e.target.value);
                  }}
                  className="w-full rounded-[11px] border border-line-strong bg-surface px-4 py-3.25 font-mono text-[13.5px] text-ink outline-none focus:border-brand"
                />
                <div className="mt-4 flex gap-2 rounded-[10px] bg-ai-wash px-3.5 py-2.75">
                  <Icon name="shield" size={16} className="mt-0.5 flex-none text-ai" />
                  <span className="text-[12.5px] leading-relaxed text-ink-soft">
                    {t("ob.l.note")}
                  </span>
                </div>
              </div>
            )}

            {step === 3 && (
              <div className="text-center">
                <div className="mx-auto mb-5.5 grid size-18 place-items-center rounded-[20px] bg-ai text-white shadow-(--shadow-md)">
                  <Icon name="check" size={38} />
                </div>
                <h2 className="mb-2.5 font-serif text-[32px] font-semibold text-ink">
                  {t("ob.f.title")}
                </h2>
                <p className="mb-6.5 text-[15px] text-ink-soft">
                  {t("ob.f.sub.a")}
                  {name.trim()}
                  {t("ob.f.sub.b")}
                </p>
                <div className="mx-auto max-w-100 rounded-[14px] border border-line bg-surface px-4.5 py-1.5 text-left">
                  <div className="flex justify-between py-3">
                    <span className="text-[13.5px] text-ink-muted">{t("ob.f.name")}</span>
                    <span className="text-[13.5px] font-semibold text-ink">{name.trim()}</span>
                  </div>
                  <div className="flex items-center justify-between gap-4 border-t border-line py-3">
                    <span className="flex-none text-[13.5px] text-ink-muted">{t("ob.f.path")}</span>
                    <span className="truncate font-mono text-[12px] font-semibold text-ink">
                      {effectivePath}
                    </span>
                  </div>
                </div>
                {error && (
                  <p className="mx-auto mt-5 max-w-100 text-[13px] leading-relaxed text-brand">
                    {error}
                  </p>
                )}
              </div>
            )}
          </div>
        </div>

        {/* フッターナビ */}
        <div className="flex flex-none items-center gap-3 border-t border-line bg-paper-2 px-10 py-4.5">
          {step > 0 ? (
            <Button variant="ghost" onClick={back} disabled={busy}>
              <Icon name="chevL" size={15} />
              {t("ob.back")}
            </Button>
          ) : (
            onCancel && (
              <Button variant="ghost" onClick={onCancel}>
                {t("ob.cancel")}
              </Button>
            )
          )}
          <div className="flex-1" />
          <span className="font-mono text-[12.5px] text-ink-faint">
            {step + 1} / {STEP_COUNT}
          </span>
          {step < STEP_COUNT - 1 ? (
            <Button size="lg" onClick={next} disabled={!canContinue}>
              {step === 0 ? t("ob.start") : t("ob.continue")}
              <Icon name="chevR" size={16} />
            </Button>
          ) : (
            <Button size="lg" onClick={finish} disabled={busy}>
              <Icon name="arrowR" size={16} />
              {t("ob.enter")}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
