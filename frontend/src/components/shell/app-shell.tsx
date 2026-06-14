"use client";

import { useEffect, useState } from "react";

import { TitleBar } from "@/components/shell/title-bar";
import { Sidebar } from "@/components/shell/sidebar";
import { SettingsDialog } from "@/components/shell/settings-dialog";
import { Onboarding } from "@/components/onboarding/onboarding";
import { useI18n } from "@/shared/providers/providers";
import { refreshKbs, useKbStore } from "@/lib/kb/store";
import { Button } from "@/shared/ui/button";
import { Icon } from "@/shared/ui/icon";

export function AppShell({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
  const [settingsOpen, setSettingsOpen] = useState(false);
  // ナレッジベース追加用のウィザード表示（既存ライブラリがある場合のみキャンセル可）
  const [wizardOpen, setWizardOpen] = useState(false);
  const { loaded, available, kbs, active, error } = useKbStore();

  useEffect(() => {
    void refreshKbs();
  }, []);

  // 起動直後のちらつきを避けるため、読み込み完了まで何も描画しない
  if (!loaded) return null;

  // Tauri 側の設定が壊れている場合は、通常画面へ進ませず復旧操作を促す
  if (available && error) {
    return (
      <div className="grid h-screen place-items-center bg-paper px-6">
        <div className="w-full max-w-112 rounded-2xl border border-line bg-surface p-8 text-center shadow-(--shadow-md)">
          <span className="mx-auto mb-4 grid size-12 place-items-center rounded-[13px] bg-brand-wash text-brand">
            <Icon name="shield" size={23} />
          </span>
          <h1 className="font-serif text-[28px] font-semibold text-ink">{t("kb.error.title")}</h1>
          <p className="mt-3 text-[13.5px] leading-relaxed text-ink-muted">{error}</p>
          <Button className="mt-6" onClick={() => void refreshKbs()}>
            <Icon name="arrowR" size={15} />
            {t("kb.error.retry")}
          </Button>
        </div>
      </div>
    );
  }

  // ナレッジベースが未設定なら、初期化ウィザードだけを表示する
  if (available && kbs.length === 0) {
    return <Onboarding />;
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <TitleBar kbLabel={active?.name ?? ""} onSettings={() => setSettingsOpen(true)} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar onAddKb={() => setWizardOpen(true)} />
        <main id="main" className="flex-1 overflow-auto px-10 pt-8.5 pb-15">
          {children}
        </main>
      </div>
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
      {wizardOpen && <Onboarding onCancel={() => setWizardOpen(false)} />}
    </div>
  );
}
