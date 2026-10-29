import { describe, expect, it } from 'vitest';
import { analyzeChord } from './ghosting';

describe('analyzeChord', () => {
  it('reports full rollover when every target is held together', () => {
    const r = analyzeChord([0, 1, 2, 3], [[0], [0, 1], [0, 1, 2, 3]]);
    expect(r.rollover).toBe(4);
    expect(r.suspects).toEqual([]);
    expect(r.never).toEqual([]);
  });

  it('flags a button that only registers alone', () => {
    const r = analyzeChord([0, 1, 2, 3], [[2], [], [0, 1, 3]]);
    expect(r.rollover).toBe(3);
    expect(r.suspects).toEqual([2]);
    expect(r.never).toEqual([]);
  });

  it('lists buttons that never registered', () => {
    const r = analyzeChord([0, 1, 2], [[0, 1]]);
    expect(r.never).toEqual([2]);
    expect(r.suspects).toEqual([2]);
  });

  it('ignores non-target buttons', () => {
    const r = analyzeChord([4, 5], [[0, 1, 4, 5, 9]]);
    expect(r.rollover).toBe(2);
    expect(r.bestChord).toEqual([4, 5]);
  });
});
