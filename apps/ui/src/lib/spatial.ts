export type Dir = 'up' | 'down' | 'left' | 'right';

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

const center = (b: Box) => ({ x: b.x + b.w / 2, y: b.y + b.h / 2 });

/**
 * Picks the best candidate in a direction: it must lie on that side, and
 * closer-and-better-aligned wins (distance along the axis plus a penalty
 * for sideways offset).
 */
export function pickNext(from: Box, candidates: Box[], dir: Dir): number {
  const a = center(from);
  let best = -1;
  let bestScore = Infinity;
  candidates.forEach((b, i) => {
    const c = center(b);
    const dx = c.x - a.x;
    const dy = c.y - a.y;
    const along = dir === 'left' ? -dx : dir === 'right' ? dx : dir === 'up' ? -dy : dy;
    const side = dir === 'left' || dir === 'right' ? Math.abs(dy) : Math.abs(dx);
    if (along <= 1) return;
    const score = along + side * 2.5;
    if (score < bestScore) {
      bestScore = score;
      best = i;
    }
  });
  return best;
}
