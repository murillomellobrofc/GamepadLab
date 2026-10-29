export interface GhostResult {
  targets: number[];
  rollover: number;
  bestChord: number[];
  suspects: number[];
  never: number[];
}

/** Pure analysis of what was seen during a hold (exported for tests). */
export function analyzeChord(targets: number[], frames: number[][]): GhostResult {
  let best: number[] = [];
  const ever = new Set<number>();
  for (const pressed of frames) {
    const held = pressed.filter((b) => targets.includes(b));
    held.forEach((b) => ever.add(b));
    if (held.length > best.length) best = held;
  }
  const never = targets.filter((b) => !ever.has(b));
  const suspects =
    best.length >= 2 && best.length < targets.length
      ? targets.filter((b) => !best.includes(b))
      : [];
  return { targets, rollover: best.length, bestChord: best, suspects, never };
}
