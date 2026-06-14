import type { ReactNode } from "react";

// Section header: mono eyebrow + serif display title + optional sub + right slot.
export function PageHead({
  eyebrow,
  title,
  sub,
  right,
}: {
  eyebrow?: ReactNode;
  title: ReactNode;
  sub?: ReactNode;
  right?: ReactNode;
}) {
  return (
    <div className="mb-7 flex flex-wrap items-end justify-between gap-5">
      <div>
        {eyebrow && (
          <div className="mb-2.5 font-mono text-xs font-semibold tracking-[0.16em] text-brand uppercase">
            {eyebrow}
          </div>
        )}
        <h1 className="font-serif text-[34px] leading-[1.05] font-medium tracking-[-0.01em] text-ink">
          {title}
        </h1>
        {sub && <p className="mt-2.5 max-w-140 text-[14.5px] leading-relaxed text-ink-muted">{sub}</p>}
      </div>
      {right && <div className="flex items-center gap-2.5">{right}</div>}
    </div>
  );
}
