"use client";

import { useEffect, useState } from "react";

import { getKbStatus } from "@/lib/tauri/client";

export function KbStatus() {
  const [root, setRoot] = useState<string | null>(null);

  useEffect(() => {
    getKbStatus().then(
      (status) => setRoot(status?.root ?? null),
      () => setRoot(null),
    );
  }, []);

  if (!root) return null;
  return (
    <div
      className="truncate px-3.25 pt-3 font-mono text-[10px] text-ink-faint"
      title={root}
    >
      {root}
    </div>
  );
}
