// Obsidian 風のフォースディレクテッド・レイアウト。ノード ID とエッジから
// 各ノードの座標を計算する純関数。seed 固定で決定論的なので、描画やドラッグ
// といった副作用から切り離してテストできる。結果は viewport（0..width, 0..height）
// に pad 付きで収まるようスケール・センタリングされる。

export type Point = { x: number; y: number };

const MAX_FORCE_NODES = 200;

function circularLayout(nodeIds: string[], width: number, height: number): Record<string, Point> {
  const pad = 70;
  const rx = (width - pad * 2) / 2;
  const ry = (height - pad * 2) / 2;
  return Object.fromEntries(
    nodeIds.map((id, i) => {
      const angle = (Math.PI * 2 * i) / nodeIds.length - Math.PI / 2;
      return [
        id,
        { x: width / 2 + rx * Math.cos(angle), y: height / 2 + ry * Math.sin(angle) },
      ];
    })
  );
}

function seededRng(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1664525 + 1013904223) % 4294967296;
    return s / 4294967296;
  };
}

export function forceLayout(
  nodeIds: string[],
  edges: [string, string][],
  width: number,
  height: number,
  seed = 7
): Record<string, Point> {
  const n = nodeIds.length;
  if (n === 0) return {};
  // 大規模グラフは O(n) 配置へ退避し、メインスレッドの O(n²) 停止を避ける。
  if (n > MAX_FORCE_NODES) return circularLayout(nodeIds, width, height);

  const rnd = seededRng(seed);
  const pos: Point[] = nodeIds.map((_, i) => ({
    x: width / 2 + Math.cos(i * 2.4 + seed) * 220 + (rnd() - 0.5) * 80,
    y: height / 2 + Math.sin(i * 2.4 + seed) * 180 + (rnd() - 0.5) * 80,
  }));

  const idx: Record<string, number> = {};
  nodeIds.forEach((id, i) => (idx[id] = i));

  const k = Math.sqrt((width * height) / n) * 0.62;
  let temp = width / 6;

  for (let it = 0; it < 340; it++) {
    const disp: Point[] = pos.map(() => ({ x: 0, y: 0 }));

    // 反発（全ノード間）
    for (let i = 0; i < n; i++) {
      for (let j = i + 1; j < n; j++) {
        const dx = pos[i].x - pos[j].x;
        const dy = pos[i].y - pos[j].y;
        const d = Math.hypot(dx, dy) || 0.01;
        const f = (k * k) / d;
        const ux = dx / d;
        const uy = dy / d;
        disp[i].x += ux * f;
        disp[i].y += uy * f;
        disp[j].x -= ux * f;
        disp[j].y -= uy * f;
      }
    }

    // 引力（エッジ）
    for (const [a, b] of edges) {
      const i = idx[a];
      const j = idx[b];
      if (i == null || j == null) continue;
      const dx = pos[i].x - pos[j].x;
      const dy = pos[i].y - pos[j].y;
      const d = Math.hypot(dx, dy) || 0.01;
      const f = (d * d) / k;
      const ux = dx / d;
      const uy = dy / d;
      disp[i].x -= ux * f;
      disp[i].y -= uy * f;
      disp[j].x += ux * f;
      disp[j].y += uy * f;
    }

    // 中央への弱い引力
    for (let i = 0; i < n; i++) {
      disp[i].x += (width / 2 - pos[i].x) * 0.022;
      disp[i].y += (height / 2 - pos[i].y) * 0.022;
    }

    // 温度で制限しながら移動（焼きなまし）
    for (let i = 0; i < n; i++) {
      const dx = disp[i].x;
      const dy = disp[i].y;
      const d = Math.hypot(dx, dy) || 0.01;
      const lim = Math.min(d, temp);
      pos[i].x += (dx / d) * lim;
      pos[i].y += (dy / d) * lim;
    }
    temp *= 0.972;
  }

  // 全体をバウンディングボックスに合わせ viewport へ収める
  let minX = Infinity;
  let maxX = -Infinity;
  let minY = Infinity;
  let maxY = -Infinity;
  for (const p of pos) {
    minX = Math.min(minX, p.x);
    maxX = Math.max(maxX, p.x);
    minY = Math.min(minY, p.y);
    maxY = Math.max(maxY, p.y);
  }
  const pad = 70;
  const s = Math.min(
    (width - pad * 2) / (maxX - minX || 1),
    (height - pad * 2) / (maxY - minY || 1)
  );
  const ox = (width - (maxX - minX) * s) / 2;
  const oy = (height - (maxY - minY) * s) / 2;

  const out: Record<string, Point> = {};
  nodeIds.forEach((id, i) => {
    out[id] = { x: (pos[i].x - minX) * s + ox, y: (pos[i].y - minY) * s + oy };
  });
  return out;
}
