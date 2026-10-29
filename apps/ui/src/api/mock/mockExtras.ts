import type { CalibratedView, RumbleRequest } from '../types';
import { envelope } from '@/lib/haptics';
import type { MockBackend } from './mockTransport';

/** Plays a rumble program in the browser mock and emits gp://rumble frames. */
function playRumble(b: MockBackend, id: string, program: RumbleRequest[]) {
  const total = program.reduce((a, r) => a + r.durationMs, 0);
  const start = performance.now();
  const w = window as unknown as { __mockRumble?: number };
  if (w.__mockRumble) window.clearInterval(w.__mockRumble);
  w.__mockRumble = window.setInterval(() => {
    const t = performance.now() - start;
    let offset = 0;
    let weak = 0;
    let strong = 0;
    for (const r of program) {
      if (t < offset + r.durationMs) {
        const k = envelope(r, t - offset);
        weak = r.weak * k;
        strong = r.strong * k;
        break;
      }
      offset += r.durationMs;
    }
    const active = t < total;
    b.sim.rumble = { weak, strong };
    b.emit('gp://rumble', { controllerId: id, weak, strong, active });
    if (!active) window.clearInterval(w.__mockRumble);
  }, 30);
}

/**
 * Commands added by later phases (calibration, profiles, haptics, history).
 * Kept apart from the core mock so each phase can extend it.
 */
export const mockExtras = {
  calibrated(_b: MockBackend, _t: number): CalibratedView | null {
    return null;
  },
  handlers(b: MockBackend): Record<string, (a: Record<string, unknown>) => unknown> {
    const c = (id: unknown) => String(id);
    return {
      rumble_play: (a) => playRumble(b, c(a.id), [a.request as RumbleRequest]),
      rumble_sequence: (a) => playRumble(b, c(a.id), a.requests as RumbleRequest[]),
      rumble_motor_test: (a) => {
        const i = Number(a.intensity);
        const s = (weak: number, strong: number, durationMs: number): RumbleRequest => ({
          weak,
          strong,
          durationMs,
          pattern: 'constant',
          periodMs: 500,
        });
        playRumble(b, c(a.id), [
          s(i, 0, 1000),
          s(0, 0, 400),
          s(0, i, 1000),
          s(0, 0, 400),
          s(i, i, 1000),
        ]);
      },
      rumble_stop: (a) => playRumble(b, c(a.id), []),
      set_trigger_effects: () => undefined,
      set_lightbar: () => undefined,
      set_player_leds: () => undefined,
    };
  },
};
