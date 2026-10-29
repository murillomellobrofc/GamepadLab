import { describe, expect, it } from 'vitest';
import { histogram, summarizeIntervals } from './latency';

describe('latency math', () => {
  it('summarises a steady 1 kHz stream', () => {
    const s = summarizeIntervals(new Array(1000).fill(1000));
    expect(s.hz).toBeCloseTo(1000);
    expect(s.jitterMs).toBe(0);
    expect(s.p99Ms).toBe(1);
  });

  it('measures jitter and drops idle gaps', () => {
    const s = summarizeIntervals([3000, 5000, 3000, 5000, 900_000]);
    expect(s.samples).toBe(4);
    expect(s.avgMs).toBeCloseTo(4);
    expect(s.jitterMs).toBeCloseTo(1);
  });

  it('builds a histogram', () => {
    const h = histogram([1, 1, 2, 3], 2);
    expect(h.counts).toEqual([2, 2]);
    expect(histogram([], 4).counts).toEqual([]);
  });
});
