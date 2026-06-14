// アプリのナビゲーション構造（モックデータではなく固定の画面構成）。

import type { IconName } from "@/shared/ui/icon";

export type RouteId =
  | "dash"
  | "capture"
  | "workshop"
  | "wiki"
  | "graph"
  | "publish"
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
  { id: "publish", href: "/publish", icon: "broadcast" },
  { id: "bots", href: "/bots", icon: "bot" },
  { id: "plugins", href: "/plugins", icon: "plug" },
];
