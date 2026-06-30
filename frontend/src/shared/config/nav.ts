// アプリのナビゲーション構造（モックデータではなく固定の画面構成）。

import type { IconName } from "@/shared/ui/icon";

export type RouteId = "dash" | "workshop" | "wiki" | "graph";

export type NavItem = {
  id: RouteId;
  href: string;
  icon: IconName;
  tone?: "ai";
};

export const NAV: NavItem[] = [
  { id: "dash", href: "/", icon: "dash" },
  { id: "workshop", href: "/workshop", icon: "merge" },
  { id: "wiki", href: "/wiki", icon: "book" },
  { id: "graph", href: "/graph", icon: "graph", tone: "ai" },
];
