/** Summary statistics over raw report intervals (µs). */
export function summarizeIntervals(intervalsUs: number[]) {
  const v = intervalsUs.filter((x) => x > 0 && x <= 250_000);
  const n = v.length;
  if (!n) return { hz: 0, avgMs: 0, minMs: 0, maxMs: 0, jitterMs: 0, p99Ms: 0, samples: 0 };
  const mean = v.reduce((a, b) => a + b, 0) / n;
  const sd = Math.sqrt(v.reduce((a, b) => a + (b - mean) ** 2, 0) / n);
  const sorted = [...v].sort((a, b) => a - b);
  return {
    hz: 1e6 / mean,
    avgMs: mean / 1000,
    minMs: sorted[0] / 1000,
    maxMs: sorted[n - 1] / 1000,
    jitterMs: sd / 1000,
    p99Ms: sorted[Math.max(0, Math.ceil(n * 0.99) - 1)] / 1000,
    samples: n,
  };
}

/** Equal-width histogram between the minimum and maximum value. */
export function histogram(values: number[], bins: number) {
  if (!values.length) return { min: 0, max: 0, counts: [] as number[] };
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const counts = new Array(bins).fill(0);
  for (const v of values) counts[Math.min(bins - 1, Math.floor(((v - min) / span) * bins))]++;
  return { min, max, counts };
}
