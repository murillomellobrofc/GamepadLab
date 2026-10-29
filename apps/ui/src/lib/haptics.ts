import type { RumblePatternKind, RumbleRequest } from '@/api/types';

/** Port of crates/haptics/src/pattern.rs for previews (kept in sync by a golden test). */
export function envelope(
  r: Pick<RumbleRequest, 'durationMs' | 'pattern' | 'periodMs'>,
  t: number,
): number {
  if (t < 0 || t >= r.durationMs) return 0;
  const period = Math.max(1, r.periodMs);
  const phase = (t % period) / period;
  switch (r.pattern) {
    case 'constant':
      return 1;
    case 'pulse':
      return phase < 0.5 ? 1 : 0;
    case 'wave':
      return 0.5 - 0.5 * Math.cos(phase * Math.PI * 2);
    case 'rampUp':
      return t / r.durationMs;
    case 'rampDown':
      return 1 - t / r.durationMs;
    case 'heartbeat':
      return phase < 0.12 || (phase >= 0.25 && phase < 0.37) ? 1 : 0;
  }
}

export const PATTERNS: RumblePatternKind[] = [
  'constant',
  'pulse',
  'wave',
  'rampUp',
  'rampDown',
  'heartbeat',
];

export function motorTestProgram(intensity: number): RumbleRequest[] {
  const c = (weak: number, strong: number, durationMs: number): RumbleRequest => ({
    weak,
    strong,
    durationMs,
    pattern: 'constant',
    periodMs: 500,
  });
  return [
    c(intensity, 0, 1000),
    c(0, 0, 400),
    c(0, intensity, 1000),
    c(0, 0, 400),
    c(intensity, intensity, 1000),
  ];
}
