import { describe, expect, it } from 'vitest';
import golden from '../../../../fixtures/calibration_golden.json';
import type { ResponseCurve, StickCalibration, TriggerCalibration } from '@/api/types';
import { applyStick, applyTrigger, defaultProfile, evalCurve } from './calibration';

describe('calibration preview matches the Rust pipeline', () => {
  it('sticks', () => {
    for (const c of golden.sticks) {
      for (const [[x, y], [ex, ey]] of c.samples as number[][][]) {
        const o = applyStick(c.calibration as unknown as StickCalibration, { x, y });
        expect(o.x, `${c.name} (${x},${y}).x`).toBeCloseTo(ex, 4);
        expect(o.y, `${c.name} (${x},${y}).y`).toBeCloseTo(ey, 4);
      }
    }
  });

  it('triggers', () => {
    for (const c of golden.triggers) {
      for (const [v, e] of c.samples as number[][]) {
        expect(applyTrigger(c.calibration as TriggerCalibration, v), `${c.name} ${v}`).toBeCloseTo(
          e,
          4,
        );
      }
    }
  });

  it('curves', () => {
    for (const c of golden.curves) {
      for (const [x, e] of c.samples as number[][]) {
        expect(evalCurve(c.curve as ResponseCurve, x)).toBeCloseTo(e, 4);
      }
    }
  });

  it('default profile is a near-identity outside the small deadzone', () => {
    const p = defaultProfile();
    expect(applyStick(p.leftStick, { x: 0.02, y: 0 })).toEqual({ x: 0, y: 0 });
    expect(applyStick(p.leftStick, { x: 1, y: 0 }).x).toBeCloseTo(1, 5);
  });
});
