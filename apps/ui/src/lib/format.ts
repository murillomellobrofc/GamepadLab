export function hex4(n: number): string {
  return n.toString(16).padStart(4, '0').toUpperCase();
}

/** Fixed-width signed number, e.g. +0.512 / -0.031. */
export function signed(v: number, digits = 3): string {
  const s = Math.abs(v).toFixed(digits);
  return (v < 0 ? '−' : '+') + s;
}

export function pct(v: number, digits = 0): string {
  return `${(v * 100).toFixed(digits)}%`;
}

export function clamp(v: number, lo: number, hi: number): number {
  return v < lo ? lo : v > hi ? hi : v;
}
