import { describe, expect, it } from 'vitest';
import { ComplementaryFilter } from './orientation';

describe('complementary filter', () => {
  it('integrates a constant yaw rate', () => {
    const f = new ComplementaryFilter();
    for (let i = 0; i <= 250; i++) f.update([0, 90, 0], [0, -1, 0], i * 4000);
    expect(f.o.yaw).toBeCloseTo(90, 0);
    expect(Math.abs(f.o.pitch)).toBeLessThan(1);
  });

  it('pulls pitch towards gravity', () => {
    const f = new ComplementaryFilter(0.9);
    const s = Math.sin(Math.PI / 6);
    const c = Math.cos(Math.PI / 6);
    for (let i = 0; i <= 500; i++) f.update([0, 0, 0], [0, -c, s], i * 4000);
    expect(f.o.pitch).toBeCloseTo(30, 0);
  });

  it('tare removes gyro bias', () => {
    const f = new ComplementaryFilter();
    f.tare([2, 3, 4]);
    for (let i = 0; i <= 100; i++) f.update([2, 3, 4], [0, -1, 0], i * 4000);
    expect(Math.abs(f.o.yaw)).toBeLessThan(1e-6);
  });

  it('ignores time going backwards or long gaps', () => {
    const f = new ComplementaryFilter();
    f.update([0, 100, 0], [0, -1, 0], 1_000_000);
    f.update([0, 100, 0], [0, -1, 0], 900_000);
    f.update([0, 100, 0], [0, -1, 0], 5_000_000);
    expect(f.o.yaw).toBe(0);
  });
});
