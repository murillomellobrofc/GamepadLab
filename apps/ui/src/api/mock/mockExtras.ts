import type {
  ButtonPlan,
  HealthGrade,
  HealthProgress,
  HealthReport,
  LiveButton,
  CalibratedView,
  CalibrationProfile,
  FirmwareOp,
  HistorySession,
  Profile,
  RumbleRequest,
  StickCalibration,
} from '../types';
import { applyStick, applyTrigger } from '@/lib/calibration';
import { envelope } from '@/lib/haptics';
import reportLeft from './fixtures/report-left.json';
import reportRight from './fixtures/report-right.json';
import triggers from './fixtures/triggers.json';
import buttonHealth from './fixtures/button-health.json';
import { MockFirmware } from './mockFirmware';
import { MockProfiles } from './mockProfiles';
import type { MockBackend } from './mockTransport';

/** Live press counts for the browser mock (same 25 ms merge as the Rust analysis). */
function liveStatus(
  events: { button: number; pressed: boolean; tUs: number }[],
  nowUs: number,
): LiveButton[] {
  const by = new Map<number, { start: number; end: number | null }[]>();
  for (const e of events) {
    const list = by.get(e.button) ?? [];
    by.set(e.button, list);
    const last = list[list.length - 1];
    if (e.pressed) {
      if (last && last.end !== null && e.tUs - last.end < 25_000) last.end = null;
      else if (!last || last.end !== null) list.push({ start: e.tUs, end: null });
    } else if (last && last.end === null) {
      last.end = e.tUs;
    }
  }
  return [...by.entries()]
    .sort((x, y) => x[0] - y[0])
    .map(([button, list]) => {
      const open = list[list.length - 1]?.end === null ? list[list.length - 1] : null;
      return {
        button,
        presses: list.length,
        taps: list.filter((p) => p.end !== null && p.end - p.start < 1_500_000).length,
        held: open !== null,
        heldForUs: open ? nowUs - open.start : 0,
      };
    });
}

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

const profiles = new MockProfiles();
const history: HistorySession[] = [];
const firmware = new MockFirmware();
const DEMO_KEY = 'demo:dualsense';
let active: CalibrationProfile | null = null;
let pending: { kind: string; target: string } | null = null;

/**
 * Commands added by later phases (calibration, profiles, haptics, history).
 * Kept apart from the core mock so each phase can extend it.
 */
export const mockExtras = {
  calibrated(b: MockBackend, t: number): CalibratedView | null {
    if (!active) return null;
    const s = b.sim.sample(t);
    const lt = applyTrigger(active.leftTrigger, s.lt);
    const rt = applyTrigger(active.rightTrigger, s.rt);
    return {
      leftStick: applyStick(active.leftStick, { x: s.left[0], y: s.left[1] }),
      rightStick: applyStick(active.rightStick, { x: s.right[0], y: s.right[1] }),
      leftTrigger: lt,
      rightTrigger: rt,
      buttons: Array.from({ length: 23 }, (_, i) =>
        i === 6
          ? { pressed: lt >= 0.12, value: lt }
          : i === 7
            ? { pressed: rt >= 0.12, value: rt }
            : { pressed: s.pressed.includes(i), value: s.pressed.includes(i) ? 1 : 0 },
      ),
    };
  },
  handlers(b: MockBackend): Record<string, (a: Record<string, unknown>) => unknown> {
    const c = (id: unknown) => String(id);
    active = profiles.active(DEMO_KEY);
    const refresh = () => {
      active = profiles.active(DEMO_KEY);
    };
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

      // Button health: live counts from the simulated edges; the final report is
      // the one the Rust analysis produced for the demo (fixtures/button-health.json).
      button_health_start: () => {
        b.health = { start: -1, last: 0, prev: [], events: [] };
      },
      button_health_progress: (): HealthProgress => {
        const h = b.health;
        if (!h) throw { code: 'invalidInput', message: 'no button health session running' };
        const elapsed = Math.max(0, h.last - Math.max(h.start, 0));
        return {
          controller: 'demo:dualsense',
          precision: { kind: 'report', intervalUs: 4000 },
          elapsedMs: elapsed / 1000,
          buttons: liveStatus(h.events, elapsed),
        };
      },
      button_health_stop: (a): HealthReport => {
        if (!b.health) throw { code: 'invalidInput', message: 'no button health session running' };
        b.health = null;
        const plans = a.plans as ButtonPlan[];
        const report = buttonHealth as unknown as HealthReport;
        const buttons = plans.flatMap((p) => report.buttons.filter((x) => x.button === p.button));
        const rank: Record<HealthGrade, number> = { good: 0, warning: 1, bad: 2 };
        const overall = buttons.reduce<HealthGrade>(
          (w, x) => (rank[x.grade] > rank[w] ? x.grade : w),
          'good',
        );
        return { ...report, buttons, overall };
      },
      button_health_cancel: () => {
        b.health = null;
      },
      capture_start: (a) => {
        pending = { kind: String(a.kind), target: String(a.target) };
        const isTrigger = pending.kind.startsWith('trigger');
        b.capture = {
          kind: pending.kind,
          target: isTrigger ? `${pending.target}Trigger` : `${pending.target}Stick`,
          samples: [],
          start: -1,
        };
      },
      capture_stop: () => {
        if (!b.capture || !pending) throw { code: 'invalidInput', message: 'no capture running' };
        const cap = b.capture;
        b.capture = null;
        const result = {
          kind: pending.kind,
          target: pending.target,
          samples: cap.samples,
          durationMs: (cap.samples[cap.samples.length - 1]?.t ?? 0) / 1000,
        };
        pending = null;
        return result;
      },
      capture_clear: () => {
        b.capture = null;
        pending = null;
      },
      analyze_stick: (a) => {
        const fx = a.stick === 'left' ? reportLeft : reportRight;
        if (!a.calibration) return fx.before;
        return { ...fx.after, suggested: a.calibration as StickCalibration };
      },
      analyze_trigger_cmd: (a) => (a.sideName === 'left' ? triggers.left : triggers.right),
      set_active_calibration: (a) => {
        active = (a.profile as CalibrationProfile | null) ?? null;
      },
      profiles_load: () => {
        const d = profiles.load(DEMO_KEY, 'DualSense (demo)');
        refresh();
        return d;
      },
      profiles_create: (a) => {
        const p = profiles.create(
          c(a.deviceKey),
          c(a.name),
          (a.calibration as CalibrationProfile) ?? null,
        );
        refresh();
        return p;
      },
      profiles_update: (a) => {
        const p = profiles.update(c(a.deviceKey), a.profile as Profile);
        refresh();
        return p;
      },
      profiles_rename: (a) => profiles.rename(c(a.deviceKey), c(a.profileId), c(a.name)),
      profiles_duplicate: (a) => profiles.duplicate(c(a.deviceKey), c(a.profileId)),
      profiles_delete: (a) => {
        profiles.delete(c(a.deviceKey), c(a.profileId));
        refresh();
      },
      profiles_set_active: (a) => {
        profiles.setActive(c(a.deviceKey), c(a.profileId));
        refresh();
      },
      profiles_reset: (a) => {
        const p = profiles.reset(c(a.deviceKey), c(a.profileId));
        refresh();
        return p;
      },
      profiles_export: (a) => profiles.export(c(a.deviceKey), c(a.profileId)),
      profiles_import: (a) => profiles.import(c(a.deviceKey), c(a.json)),
      controller_profile_key: () => DEMO_KEY,
      history_add: (a) => {
        const s = a.session as Omit<HistorySession, 'id' | 'createdAt'>;
        const id = history.length ? history[0].id + 1 : 1;
        history.unshift({ ...s, id, createdAt: new Date().toISOString() });
        return id;
      },
      history_list: (a) => history.slice(0, Number(a.limit) || 200),
      history_delete: (a) => {
        const i = history.findIndex((h) => h.id === Number(a.id));
        if (i < 0) throw { code: 'notFound', message: `session ${a.id}` };
        history.splice(i, 1);
      },
      history_clear: () => {
        history.length = 0;
      },
      firmware: (a) => {
        const op = a.op as FirmwareOp;
        const r = firmware.run(op, b.sim);
        if (op.op === 'savePermanent') {
          history.unshift({
            id: (history[0]?.id ?? 0) + 1,
            createdAt: new Date().toISOString(),
            kind: 'firmware',
            controllerName: 'DualSense (demo)',
            controllerKey: DEMO_KEY,
            score: null,
            summary: 'Calibração gravada permanentemente no controle',
            payload: { op },
          });
        }
        if (op.op === 'reboot') {
          b.setDemo(false);
          window.setTimeout(() => b.setDemo(true), 600);
        }
        return r;
      },
      firmware_switch_propose: () => {
        throw { code: 'rumbleUnsupported', message: 'Switch operations on the demo DualSense' };
      },
      overlay_open: () => undefined,
      overlay_update: () => undefined,
      overlay_close: () => undefined,
      overlay_state: () => null,
      overlay_is_open: () => false,
      save_text_file: () => undefined,
      save_binary_file: () => undefined,
      read_text_file: () => '',
    };
  },
};
