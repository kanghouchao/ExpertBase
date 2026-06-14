"use client";

import { useEffect, useState } from "react";

import { Icon } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { EmptyState } from "@/components/eb/empty-state";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/components/providers";
import {
  aiHasKey,
  listInbox,
  readEntry,
  workshopConfirm,
  workshopDraft,
  type InboxItem,
} from "@/lib/tauri/client";
import { inboxToMaterial } from "@/lib/data/adapt";
import { useKbStore } from "@/lib/kb/store";
import { MaterialRow } from "../_components/material-row";

export function WorkshopView() {
  const { t } = useI18n();
  const { available } = useKbStore();
  const [pending, setPending] = useState<InboxItem[]>([]);
  const [selected, setSelected] = useState<InboxItem | null>(null);
  const [source, setSource] = useState("");
  const [instruction, setInstruction] = useState("");
  const [title, setTitle] = useState("");
  const [cat, setCat] = useState("");
  const [body, setBody] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!available) return;
    void (async () => {
      const [inbox, key] = await Promise.all([listInbox(), aiHasKey()]);
      setPending(inbox.filter((item) => item.status !== "processed"));
      setHasKey(key);
    })();
  }, [available]);

  async function refreshPending() {
    const inbox = await listInbox();
    setPending(inbox.filter((item) => item.status !== "processed"));
  }

  async function select(item: InboxItem) {
    setSelected(item);
    setError(null);
    setTitle("");
    setCat("");
    setBody("");
    setInstruction("");
    setSource("");
    const raw = await readEntry(item.path);
    setSource(raw);
  }

  async function handleGenerate() {
    if (!selected) return;
    setBusy(true);
    setError(null);
    try {
      const result = await workshopDraft(selected.path, instruction);
      setTitle(result.title);
      setCat(result.cat);
      setBody(result.bodyMarkdown);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleConfirm() {
    if (!selected || !title.trim() || !body.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await workshopConfirm({ inboxPath: selected.path, title, cat, body });
      setSelected(null);
      await refreshPending();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="view-enter">
      <PageHead eyebrow={t("workshop.eyebrow")} title={t("workshop.title")} sub={t("workshop.sub")} />

      <div className="grid grid-cols-[1fr_1.4fr] gap-5">
        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            {t("workshop.pendingMaterials")}
          </h2>
          <Panel pad={0}>
            {pending.length === 0 && (
              <EmptyState icon="merge" title={t("empty.materials")} sub={t("empty.materials.sub")} />
            )}
            {pending.map((item) => (
              <MaterialRow
                key={item.path}
                material={inboxToMaterial(item)}
                action={
                  <Button
                    size="sm"
                    variant={selected?.path === item.path ? "default" : "outline"}
                    onClick={() => void select(item)}
                  >
                    {t("workshop.process")}
                  </Button>
                }
              />
            ))}
          </Panel>
        </div>

        <div>
          <h2 className="mb-3 font-mono text-[12px] font-bold tracking-[0.12em] text-ink-muted uppercase">
            {t("workshop.result")}
          </h2>
          {!selected ? (
            <Panel pad={0}>
              <EmptyState icon="shield" title={t("workshop.selectHint")} sub={t("workshop.sub")} />
            </Panel>
          ) : (
            <Panel className="flex flex-col gap-4">
              {/* ソース（読み取り専用） */}
              <div>
                <div className="mb-1.5 font-mono text-[11.5px] font-bold tracking-widest text-ink-muted uppercase">
                  {t("workshop.source")}
                </div>
                <div className="max-h-40 overflow-auto rounded-lg border border-line bg-surface-2 p-3 text-[13px] leading-relaxed whitespace-pre-wrap text-ink-soft">
                  {source}
                </div>
              </div>

              {/* 指示 + AI 生成 */}
              <div>
                <div className="mb-1.5 font-mono text-[11.5px] font-bold tracking-widest text-ink-muted uppercase">
                  {t("workshop.instruction")}
                </div>
                <div className="flex items-center gap-2">
                  <input
                    value={instruction}
                    onChange={(event) => setInstruction(event.target.value)}
                    placeholder={t("workshop.instruction.ph")}
                    className="min-w-0 flex-1 rounded-lg border border-line-strong bg-surface px-3 py-2 text-[13.5px] text-ink outline-none"
                  />
                  <Button size="sm" disabled={!hasKey || busy} onClick={handleGenerate}>
                    <Icon name="spark" size={15} />
                    {t("workshop.generate")}
                  </Button>
                </div>
                {!hasKey && (
                  <div className="mt-1.5 text-[12px] text-ink-faint">{t("workshop.noKey")}</div>
                )}
              </div>

              {/* 結果（編集可能・手動パスもここで書ける） */}
              <div className="grid gap-2">
                <input
                  value={title}
                  onChange={(event) => setTitle(event.target.value)}
                  placeholder={t("workshop.titleField")}
                  className="w-full rounded-lg border border-line-strong bg-surface px-3 py-2 font-serif text-[18px] font-semibold text-ink outline-none"
                />
                <input
                  value={cat}
                  onChange={(event) => setCat(event.target.value)}
                  placeholder={t("workshop.catField")}
                  className="w-full rounded-lg border border-line-strong bg-surface px-3 py-2 font-mono text-[13px] text-ink outline-none"
                />
                <textarea
                  value={body}
                  onChange={(event) => setBody(event.target.value)}
                  className="min-h-60 w-full resize-y rounded-lg border border-line-strong bg-surface-2 p-3.5 font-mono text-[13.5px] leading-relaxed text-ink outline-none"
                />
              </div>

              {error && <div className="text-[12.5px] font-semibold text-brand">{error}</div>}

              <div className="flex justify-end">
                <Button disabled={!title.trim() || !body.trim() || busy} onClick={handleConfirm}>
                  <Icon name="check" size={15} />
                  {t("workshop.confirm")}
                </Button>
              </div>
            </Panel>
          )}
        </div>
      </div>
    </div>
  );
}
