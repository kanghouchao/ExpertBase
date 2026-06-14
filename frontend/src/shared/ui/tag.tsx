import { cn } from "@/shared/lib/utils";

// Tag コンポーネントの色トーン語彙（UI プリミティブ固有の見た目）。
export type TagTone = "line" | "accent" | "ai" | "muted" | "gold";

const TONES: Record<TagTone, string> = {
  line: "bg-transparent text-ink-muted border-line-strong",
  accent: "bg-brand-wash text-brand border-transparent",
  ai: "bg-ai-wash text-ai border-transparent",
  muted: "bg-surface-2 text-ink-muted border-transparent",
  gold: "bg-gold/15 text-gold border-transparent",
};

// Mono-cased status/category pill.
export function Tag({
  children,
  tone = "line",
  className,
}: {
  children: React.ReactNode;
  tone?: TagTone;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.75 font-mono text-xs leading-snug font-semibold tracking-[0.01em]",
        TONES[tone],
        className
      )}
    >
      {children}
    </span>
  );
}
