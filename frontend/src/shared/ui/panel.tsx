import { cn } from "@/shared/lib/utils";

// The design's core surface: a bordered "paper" card with a soft shadow. shadcn's
// Card uses a ring + different radius, so this thin primitive matches the
// prototype's look (and repeats everywhere, per the styling guidelines).
export function Panel({
  children,
  className,
  pad = 20,
  hover = false,
  style,
}: {
  children: React.ReactNode;
  className?: string;
  pad?: number;
  hover?: boolean;
  style?: React.CSSProperties;
}) {
  return (
    <div
      className={cn(
        "rounded-[14px] border border-line bg-surface shadow-(--shadow-sm) transition-all duration-200",
        hover && "hover:-translate-y-0.5 hover:shadow-(--shadow-md)",
        className
      )}
      style={{ padding: pad, ...style }}
    >
      {children}
    </div>
  );
}
