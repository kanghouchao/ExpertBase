// Brand mark: a gradient rounded square holding a book glyph.
export function Logo({ size = 34 }: { size?: number }) {
  return (
    <div
      className="grid flex-none place-items-center rounded-[9px] text-white shadow-(--shadow-sm)"
      style={{
        width: size,
        height: size,
        background: "linear-gradient(150deg, var(--brand), #9a4329)",
      }}
    >
      <svg
        width={size * 0.62}
        height={size * 0.62}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M4 5a2 2 0 0 1 2-2h13v15H6a2 2 0 0 0-2 2zM4 5v14" />
        <path d="M12 7v6M9 10h6" opacity=".9" />
      </svg>
    </div>
  );
}
