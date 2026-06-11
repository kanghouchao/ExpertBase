// Circular quality ring. Color steps with the value: sage ≥85, gold ≥70, else
// terracotta. The dash offset animates via CSS when the value changes.
export function Ring({ value, size = 38, sw = 4 }: { value: number; size?: number; sw?: number }) {
  const r = (size - sw) / 2;
  const c = 2 * Math.PI * r;
  const color = value >= 85 ? "var(--ai)" : value >= 70 ? "var(--gold)" : "var(--brand)";
  return (
    <div className="relative flex-none" style={{ width: size, height: size }}>
      <svg width={size} height={size} style={{ transform: "rotate(-90deg)" }}>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--line)" strokeWidth={sw} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color}
          strokeWidth={sw}
          strokeDasharray={c}
          strokeDashoffset={c * (1 - value / 100)}
          strokeLinecap="round"
          style={{ transition: "stroke-dashoffset .8s cubic-bezier(.2,.7,.2,1)" }}
        />
      </svg>
      <div
        className="absolute inset-0 grid place-items-center font-mono font-bold"
        style={{ fontSize: size * 0.3, color }}
      >
        {value}
      </div>
    </div>
  );
}
