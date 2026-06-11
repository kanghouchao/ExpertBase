"use client";

import { useState } from "react";

import { TitleBar } from "@/components/shell/title-bar";
import { Sidebar } from "@/components/shell/sidebar";
import { SettingsDialog } from "@/components/shell/settings-dialog";
import { useI18n } from "@/components/providers";
import { KNOWLEDGE_BASES } from "@/lib/data/mock";

export function AppShell({ children }: { children: React.ReactNode }) {
  const { t } = useI18n();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [activeKb, setActiveKb] = useState("kb1");

  // Phase 1 keeps the shell interactive for instant language/theme switching.
  // When real data arrives, fetch it above this client boundary and pass
  // minimal serialized props instead of moving server state into the shell.
  const kb = KNOWLEDGE_BASES.find((k) => k.id === activeKb) ?? KNOWLEDGE_BASES[0];
  const kbLabel = kb.primary ? t("app.kb") : (kb.name ?? "");

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      <TitleBar kbLabel={kbLabel} onSettings={() => setSettingsOpen(true)} />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar activeKb={activeKb} setActiveKb={setActiveKb} />
        <main id="main" className="flex-1 overflow-auto px-10 pt-8.5 pb-15">
          {children}
        </main>
      </div>
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </div>
  );
}
