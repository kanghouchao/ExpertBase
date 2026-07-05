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

// 現在のパスとワークショップ会話 ID から、ハイライトすべきナビ項目を決める。
// 特定の会話（履歴のサブ項目）を開いている間は、親「ワークショップ」を選択状態にしない。
export function resolveActiveNav(pathname: string, conversationId: number | null): RouteId | null {
  const matched = NAV.find(
    (item) => pathname === item.href || (item.href !== "/" && pathname.startsWith(`${item.href}/`))
  );
  const id = matched?.id ?? "dash";
  if (id === "workshop" && conversationId !== null) return null;
  return id;
}
