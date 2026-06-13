// アプリのナビゲーション構造（モックデータではなく固定の画面構成）。

import type { IconName } from "@/components/eb/icon";

export type RouteId =
  | "dash"
  | "capture"
  | "workshop"
  | "wiki"
  | "graph"
  | "showcase"
  | "bots"
  | "plugins";

export type NavItem = {
  id: RouteId;
  href: string;
  icon: IconName;
  tone?: "ai";
};

export const NAV: NavItem[] = [
  { id: "dash", href: "/", icon: "dash" },
  { id: "capture", href: "/capture", icon: "inbox" },
  { id: "workshop", href: "/workshop", icon: "merge" },
  { id: "wiki", href: "/wiki", icon: "book" },
  { id: "graph", href: "/graph", icon: "graph", tone: "ai" },
  { id: "showcase", href: "/showcase", icon: "eye" },
  { id: "bots", href: "/bots", icon: "bot" },
  { id: "plugins", href: "/plugins", icon: "plug" },
];
