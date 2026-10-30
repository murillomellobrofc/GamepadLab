import { describe, expect, it } from 'vitest';
import type { HealthReport } from '@/api/types';
import demo from '@/api/mock/fixtures/button-health.json';
import { gradeCounts, healthScore, isEventBased } from './buttonHealth';

const report = demo as unknown as HealthReport;

describe('button health helpers', () => {
  it('reads the demo report the Rust analysis produced: only R1 is bad', () => {
    expect(report.buttons).toHaveLength(19);
    expect(gradeCounts(report)).toEqual({ good: 18, warning: 0, bad: 1 });
    const r1 = report.buttons.find((b) => b.button === 5)!;
    expect(r1.reasons.map((r) => r.code)).toEqual(
      expect.arrayContaining(['missedPresses', 'chatter', 'releaseBounce', 'selfRelease']),
    );
    expect(r1.advice[0]).toBe('replaceSwitch');
  });

  it('scores good as 100, attention as 50 and bad as 0', () => {
    const only = (grades: HealthReport['buttons'][number]['grade'][]): HealthReport => ({
      ...report,
      buttons: grades.map((grade, i) => ({ ...report.buttons[i], grade })),
    });
    expect(healthScore(only(['good', 'good']))).toBe(100);
    expect(healthScore(only(['good', 'warning', 'bad', 'bad']))).toBe(38);
    expect(healthScore({ ...report, buttons: [] })).toBeNull();
  });

  it('warns about precision for system-read controllers only', () => {
    expect(isEventBased({ backend: 'gilrs' })).toBe(true);
    expect(isEventBased({ backend: 'hid' })).toBe(false);
    expect(isEventBased({ backend: 'demo' })).toBe(false);
  });
});
