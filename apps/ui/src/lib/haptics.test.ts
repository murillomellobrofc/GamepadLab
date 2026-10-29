import { describe, expect, it } from 'vitest';
import golden from '../../../../fixtures/haptics_golden.json';
import type { RumbleRequest } from '@/api/types';
import { envelope, motorTestProgram } from './haptics';

describe('rumble envelope', () => {
  it('matches the Rust golden values', () => {
    for (const c of golden.cases) {
      const r = c.request as RumbleRequest;
      for (const [t, expected] of c.samples) {
        expect(envelope(r, t)).toBeCloseTo(expected, 4);
      }
    }
  });

  it('motor test isolates each motor', () => {
    const p = motorTestProgram(0.7);
    expect(p.map((s) => [s.weak, s.strong])).toEqual([
      [0.7, 0],
      [0, 0],
      [0, 0.7],
      [0, 0],
      [0.7, 0.7],
    ]);
  });
});
