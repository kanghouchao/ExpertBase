"use client";

import { useState } from "react";
import Link from "next/link";

import { Icon, type IconName } from "@/components/eb/icon";
import { PageHead } from "@/components/eb/page-head";
import { Panel } from "@/components/eb/panel";
import { Button } from "@/components/ui/button";
import { RAW_MATERIALS } from "@/lib/data/mock";
import { MaterialRow } from "../_components/material-row";
import { SegTabs } from "../_components/seg-tabs";

export function CaptureView() {
  const [tab, setTab] = useState<"上传文件" | "录音" | "手动输入">("上传文件");
  const [text, setText] = useState("");
  const [items, setItems] = useState(RAW_MATERIALS);
  const pending = items.filter((item) => item.status !== "processed");

  const addItem = () => {
    setItems((current) => [
      {
        id: `n${Date.now()}`,
        type: "pdf",
        title: "新上传文档.pdf",
        sourceKey: "raw.r2.source",
        dateKey: "time.2h",
        status: "pending",
        size: "—",
        preview: "新素材已进入收集箱，等待工作坊加工。",
        words: 0,
        tags: [],
      },
      ...current,
    ]);
  };

  return (
    <div className="view-enter mx-auto max-w-190">
      <PageHead
        eyebrow="收集箱 · Capture"
        title="把一切先收进来"
        sub="文档、音频、视频、录音、随手记先进入收集箱；整理与加工交给工作坊。"
      />

      <Panel pad={0} className="mb-5 overflow-hidden">
        <div className="border-b border-line p-4">
          <SegTabs tabs={["上传文件", "录音", "手动输入"] as const} value={tab} onChange={setTab} />
        </div>
        <div className="p-5.5">
          {tab === "上传文件" && (
            <button
              onClick={addItem}
              className="w-full rounded-2xl border-2 border-dashed border-line-strong bg-surface-2 px-6 py-12 text-center transition hover:border-brand hover:bg-brand-wash"
            >
              <span className="mx-auto mb-4 grid size-14 place-items-center rounded-[15px] bg-surface text-brand shadow-(--shadow-sm)">
                <Icon name="upload" size={26} />
              </span>
              <span className="block text-base font-semibold text-ink">拖入文件，或点击上传</span>
              <span className="mt-1.5 block text-[13px] text-ink-muted">
                支持 PDF · Word · 音频 · 视频 · 图片 · Markdown，单个最大 2GB
              </span>
              <span className="mt-4 flex justify-center gap-2 text-ink-muted">
                {(["pdf", "doc", "audio", "video", "scan", "note"] as IconName[]).map((icon) => (
                  <span
                    key={icon}
                    className="grid size-8.5 place-items-center rounded-lg border border-line bg-surface"
                  >
                    <Icon name={icon} size={17} />
                  </span>
                ))}
              </span>
            </button>
          )}

          {tab === "录音" && (
            <div className="py-8 text-center">
              <button className="mx-auto grid size-24 place-items-center rounded-full bg-ink text-paper shadow-(--shadow-md)">
                <Icon name="mic" size={34} />
              </button>
              <div className="mt-5 font-mono text-[28px] font-semibold tracking-[0.04em] text-ink">
                00:00
              </div>
              <div className="mt-2.5 text-[13.5px] text-ink-muted">点击开始录制语音备忘</div>
            </div>
          )}

          {tab === "手动输入" && (
            <div>
              <textarea
                value={text}
                onChange={(event) => setText(event.target.value)}
                placeholder={"直接输入想法、笔记、片段……\n支持 Markdown，保存后即进入收集箱"}
                className="min-h-50 w-full resize-y rounded-xl border border-line-strong bg-surface-2 p-3.5 text-[14.5px] leading-relaxed text-ink outline-none"
              />
              <div className="mt-3 flex items-center justify-between">
                <span className="font-mono text-xs text-ink-faint">{text.length} 字</span>
                <Button size="sm" disabled={!text.trim()}>
                  <Icon name="check" size={15} />
                  保存到收集箱
                </Button>
              </div>
            </div>
          )}
        </div>
        <div className="flex items-center gap-2 border-t border-line bg-surface-2 px-5.5 py-3 text-[12.5px] text-ink-muted">
          <Icon name="plug" size={15} className="text-ai" />
          录入由 Whisper 与 OCR 插件自动转写
        </div>
      </Panel>

      <div className="mb-2.5 flex items-center justify-between px-0.5">
        <div className="flex items-center gap-2 font-mono text-[12.5px] font-semibold tracking-[0.04em] text-ink-muted">
          <Icon name="clock" size={14} />
          最近录入
          <span className="text-ink-faint">
            共 {items.length} 条 · {pending.length} 条待加工
          </span>
        </div>
        <Link
          href="/workshop"
          className="flex items-center gap-1 text-[13px] font-semibold text-brand"
        >
          去工作坊加工 <Icon name="arrowR" size={14} />
        </Link>
      </div>
      <Panel pad={0}>
        {items.slice(0, 5).map((item) => (
          <MaterialRow key={item.id} material={item} />
        ))}
      </Panel>
    </div>
  );
}
