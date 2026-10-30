import type { ControllerInfo, HealthGrade, HealthReport } from '@/api/types';

/** How long the hold step asks the user to keep the button down. */
export const HOLD_TARGET_US = 2_000_000;
export const PRESS_OPTIONS = [10, 20, 30] as const;
export const DEFAULT_PRESSES = 20;

/** Lead-in plus one tap every 260 ms, like DemoScenario::HealthTaps in crates/input. */
export const demoTapsDurationMs = (presses: number) => 400 + presses * 260;
export const QUICK_PICKS: Record<'shoulders' | 'face', number[]> = {
  shoulders: [4, 5, 6, 7],
  face: [0, 1, 2, 3],
};

/**
 * Sources that only report state changes at the system's polling rate
 * (gilrs: XInput/WGI, IOKit, evdev) can miss fast chatter; direct HID reads
 * every report. The demo behaves like HID.
 */
export function isEventBased(c: Pick<ControllerInfo, 'backend'>): boolean {
  return c.backend !== 'hid' && c.backend !== 'demo';
}

export function gradeCounts(r: HealthReport): Record<HealthGrade, number> {
  const out: Record<HealthGrade, number> = { good: 0, warning: 0, bad: 0 };
  for (const b of r.buttons) out[b.grade] += 1;
  return out;
}

/** History score: good buttons count fully, "attention" half, bad ones not at all. */
export function healthScore(r: HealthReport): number | null {
  if (!r.buttons.length) return null;
  const points: Record<HealthGrade, number> = { good: 100, warning: 50, bad: 0 };
  return Math.round(r.buttons.reduce((a, b) => a + points[b.grade], 0) / r.buttons.length);
}
