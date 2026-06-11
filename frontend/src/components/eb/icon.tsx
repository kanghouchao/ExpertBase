import type { CSSProperties } from "react";

// The design's hand-drawn icon set (single-path, 24×24, stroked). Ported from
// the prototype's ui.jsx so the whole app shares one cohesive line style.
const PATHS = {
  dash: "M3 12l9-8 9 8M5 10v10h5v-6h4v6h5V10",
  inbox: "M3 13h5l1.5 3h5L21 13M3 13l3-8h12l3 8M3 13v6h18v-6",
  spark: "M12 3l1.6 5.4L19 10l-5.4 1.6L12 17l-1.6-5.4L5 10l5.4-1.6zM19 14l.8 2.2L22 17l-2.2.8L19 20l-.8-2.2L16 17l2.2-.8z",
  book: "M4 5a2 2 0 0 1 2-2h13v15H6a2 2 0 0 0-2 2zM4 5v14M19 18v3H6a2 2 0 0 1-2-2",
  graph: "M6 7a2 2 0 1 0 0-.01M18 6a2 2 0 1 0 0-.01M17 18a2 2 0 1 0 0-.01M7 8.5l9-1.5M16.5 8l.4 8M15.5 17l-7-7",
  eye: "M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7-10-7-10-7zM12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6z",
  bot: "M12 3v3M8 8h8a2 2 0 0 1 2 2v6a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2v-6a2 2 0 0 1 2-2zM9.5 12v1.5M14.5 12v1.5M4 12v3M20 12v3",
  plug: "M9 3v5M15 3v5M6 8h12v4a6 6 0 0 1-12 0zM12 18v3",
  gear: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM12 2l1.3 2.4 2.7-.5.5 2.7L19 8l-1.4 2.3 1.4 2.3-2.5 1.2-.5 2.7-2.7-.5L12 22l-1.3-2.4-2.7.5-.5-2.7L5 16l1.4-2.3L5 11.4l2.5-1.2.5-2.7 2.7.5z",
  upload: "M12 16V4M8 8l4-4 4 4M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3",
  mic: "M12 3a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0V6a3 3 0 0 1 3-3zM6 11a6 6 0 0 0 12 0M12 17v4M9 21h6",
  type: "M5 7V5h14v2M12 5v14M9 19h6",
  wave: "M3 12h2l2-6 3 13 3-19 3 18 2-6h3",
  video: "M3 6a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM16 10l5-3v10l-5-3z",
  pdf: "M7 3h7l5 5v13a0 0 0 0 1 0 0H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zM14 3v5h5M8 14h1.5a1.5 1.5 0 0 1 0 3H8zM8 14v6M13 14v6h1.5a1.5 1.5 0 0 0 0-6zM18 14h-2v6M18 17h-2",
  doc: "M7 3h7l5 5v13H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2zM14 3v5h5M9 13h6M9 16h6M9 10h2",
  note: "M5 4a1 1 0 0 1 1-1h9l4 4v12a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1zM9 9h6M9 13h6M9 17h3",
  search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM16 16l5 5",
  plus: "M12 5v14M5 12h14",
  link: "M9 15l6-6M10.5 7l1.8-1.8a3.5 3.5 0 0 1 5 5L15.5 12M13.5 17l-1.8 1.8a3.5 3.5 0 0 1-5-5L8.5 12",
  check: "M5 12.5l4.5 4.5L19 6",
  x: "M6 6l12 12M18 6L6 18",
  chevR: "M9 6l6 6-6 6",
  chevD: "M6 9l6 6 6-6",
  chevL: "M15 6l-6 6 6 6",
  db: "M12 3c4.4 0 8 1.3 8 3s-3.6 3-8 3-8-1.3-8-3 3.6-3 8-3zM4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3",
  cloud: "M7 18a4 4 0 0 1 0-8 5 5 0 0 1 9.6-1.5A3.5 3.5 0 0 1 18 18z",
  scan: "M4 7V5a1 1 0 0 1 1-1h2M17 4h2a1 1 0 0 1 1 1v2M20 17v2a1 1 0 0 1-1 1h-2M7 20H5a1 1 0 0 1-1-1v-2M4 12h16",
  chat: "M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H9l-4 4v-4H5a1 1 0 0 1-1-1z",
  send: "M21 4L3 11l7 3 3 7z M10 14l4-4",
  globe: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c2.5 2.5 3.5 6 3.5 9s-1 6.5-3.5 9c-2.5-2.5-3.5-6-3.5-9s1-6.5 3.5-9z",
  page: "M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 15h6",
  phone: "M7 3h10a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zM10 18h4",
  shield: "M12 3l8 3v5c0 5-3.4 8.5-8 10-4.6-1.5-8-5-8-10V6z M9 12l2 2 4-4",
  more: "M5 12h.01M12 12h.01M19 12h.01",
  play: "M7 4l13 8-13 8z",
  pause: "M8 5v14M16 5v14",
  sun: "M12 6a6 6 0 1 0 0 12 6 6 0 0 0 0-12zM12 2v2M12 20v2M4 12H2M22 12h-2M5 5l1.5 1.5M17.5 17.5L19 19M19 5l-1.5 1.5M6.5 17.5L5 19",
  moon: "M20 14a8 8 0 1 1-9-11 6 6 0 0 0 9 11z",
  arrowR: "M5 12h14M13 6l6 6-6 6",
  edit: "M4 20l4-1L19 8l-3-3L5 16zM14 6l3 3",
  trash: "M5 7h14M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M6 7l1 13h10l1-13",
  star: "M12 4l2.3 5 5.4.5-4.1 3.6 1.3 5.3L12 20.7 7.1 23.4l1.3-5.3L4.3 9.5 9.7 9z",
  filter: "M3 5h18l-7 8v6l-4-2v-4z",
  layers: "M12 3l9 5-9 5-9-5zM3 13l9 5 9-5M3 16.5l9 5 9-5",
  clock: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM12 8v4l3 2",
  flag: "M5 21V4h11l-1.5 4L16 12H5",
  merge: "M6 3v6a6 6 0 0 0 6 6h6M18 12l3 3-3 3M14 3v3a6 6 0 0 0 4 5.6",
  drag: "M9 5h.01M15 5h.01M9 12h.01M15 12h.01M9 19h.01M15 19h.01",
  audio: "M3 12h2l2-6 3 13 3-19 3 18 2-6h3",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({
  name,
  size = 18,
  sw = 1.6,
  className,
  style,
  fill = "none",
}: {
  name: IconName;
  size?: number;
  sw?: number;
  className?: string;
  style?: CSSProperties;
  fill?: string;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill={fill}
      stroke="currentColor"
      strokeWidth={sw}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      style={{ flex: "none", ...style }}
      aria-hidden="true"
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
