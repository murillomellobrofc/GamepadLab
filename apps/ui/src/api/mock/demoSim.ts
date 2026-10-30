/**
 * Browser-side port of the Rust demo controller (crates/input/src/demo.rs),
 * used only by the mock transport (plain `vite dev` and Playwright).
 */
import { BUTTONS, type DemoScenario, type InputSnapshot } from '../types';

const TAU = Math.PI * 2;
export const DIRECTIONS_DEG = [90, 45, 0, 315, 270, 225, 180, 135];
// Guide (16) is left out: it toggles controller navigation in the UI.
const SHOWCASE_ORDER = [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18];
const LABELS = [
  'Cross',
  'Circle',
  'Square',
  'Triangle',
  'L1',
  'R1',
  'L2',
  'R2',
  'Share',
  'Options',
  'L3',
  'R3',
  'Up',
  'Down',
  'Left',
  'Right',
  'PS',
  'Touchpad',
  'Mute',
];

interface StickModel {
  drift: [number, number];
  noise: number;
  bulge: number;
  base: number;
  dent: [number, number, number] | null;
}
const LEFT_MODEL: StickModel = {
  drift: [0.034, -0.021],
  noise: 0.0065,
  bulge: 0.04,
  base: 0.965,
  dent: [200, 22, 0.8],
};
const RIGHT_MODEL: StickModel = {
  drift: [0.004, 0.006],
  noise: 0.0025,
  bulge: 0.05,
  base: 1,
  dent: null,
};

function reach(m: StickModel, a: number): number {
  let r = m.base + m.bulge * (1 - Math.cos(4 * a)) * 0.5;
  if (m.dent) {
    const [c, half, depth] = m.dent;
    const d = Math.abs((((((a * 180) / Math.PI - c) % 360) + 540) % 360) - 180);
    if (d < half) {
      const k = 0.5 + 0.5 * Math.cos((Math.PI * d) / half);
      r = r * (1 - k) + depth * k;
    }
  }
  return r;
}

function rotationAngle(secs: number): number {
  const s = secs % 16;
  const slow = 0.4 * TAU;
  const fast = 1.3 * TAU;
  if (s < 5) return s * slow;
  if (s < 10) return 5 * slow - (s - 5) * slow;
  if (s < 13) return (s - 10) * fast;
  return 3 * fast - (s - 13) * fast;
}

let seed = 0x5eedcafe;
function rand(): number {
  seed ^= seed << 13;
  seed ^= seed >>> 17;
  seed ^= seed << 5;
  return ((seed >>> 0) % 1_000_000) / 1_000_000;
}
function gauss(): number {
  let s = 0;
  for (let i = 0; i < 6; i++) s += rand();
  return s - 3;
}

const q8 = (v: number) => Math.round((Math.max(-1, Math.min(1, v)) + 1) * 127.5);

export interface ManualState {
  leftStick: { x: number; y: number };
  rightStick: { x: number; y: number };
  leftTrigger: number;
  rightTrigger: number;
  buttons: boolean[];
}

/* Button health, mirroring crates/input/src/demo.rs: the demo R1 (5) is going
   bad, so some taps never arrive, some chatter or bounce, and it lets go while held. */
const HEALTH_FAULTY = 5;
const HEALTH_MISSED = [3, 9, 15];
const HEALTH_CHATTER = [1, 6, 11, 17];
const HEALTH_BOUNCE = [4, 13];

function healthTapDown(button: number, presses: number, secs: number): boolean {
  const t = secs - 0.4;
  if (t < 0) return false;
  const k = Math.floor(t / 0.26);
  if (k >= presses) return false;
  const w = t - k * 0.26;
  if (button !== HEALTH_FAULTY) return w < 0.1;
  if (HEALTH_MISSED.includes(k)) return false;
  if (HEALTH_CHATTER.includes(k)) return w < 0.008 || (w >= 0.016 && w < 0.1);
  if (HEALTH_BOUNCE.includes(k)) return w < 0.1 || (w >= 0.108 && w < 0.116);
  return w < 0.1;
}

function healthHoldDown(button: number, secs: number): boolean {
  const dropped =
    button === HEALTH_FAULTY && ((secs >= 1.4 && secs < 1.412) || (secs >= 2.2 && secs < 2.208));
  return secs >= 0.3 && secs < 2.6 && !dropped;
}

export class DemoSim {
  scenario: DemoScenario = 'showcase';
  scenarioStart = 0;
  manual: ManualState = {
    leftStick: { x: 0, y: 0 },
    rightStick: { x: 0, y: 0 },
    leftTrigger: 0,
    rightTrigger: 0,
    buttons: [],
  };
  rumble = { weak: 0, strong: 0 };
  /** Simulated firmware calibration (see MockFirmware). */
  calibrated = { center: false, range: false };
  savedCalibration = { center: false, range: false };
  private seq = 0;
  private ranges = new Map<number, [number, number]>();
  private presses = new Map<number, [boolean, number, number]>();
  private stdPrev: boolean[] = [];
  private stdPresses: number[] = new Array(23).fill(0);

  setScenario(s: DemoScenario, nowUs: number) {
    this.scenario = s;
    this.scenarioStart = nowUs;
  }

  resetMinMax() {
    this.ranges.clear();
    this.presses.clear();
  }

  /** Raw stick/trigger values at time t (µs), without bookkeeping. */
  sample(t: number) {
    const secs = (t - this.scenarioStart) / 1e6;
    const abs = t / 1e6;
    const fixRange = (m: StickModel): StickModel =>
      this.calibrated.range ? { ...m, base: 1, bulge: 0.035 } : m;
    const fixCenter = (m: StickModel): StickModel =>
      this.calibrated.center ? { ...m, drift: [0.003, -0.002] } : m;
    const LEFT = fixRange(fixCenter(LEFT_MODEL));
    const RIGHT = fixRange(fixCenter(RIGHT_MODEL));
    let left: [number, number] = [...LEFT.drift];
    let right: [number, number] = [...RIGHT.drift];
    let lt = 0;
    let rt = 0;
    const pressed: number[] = [];
    const sc = this.scenario;
    const polar = (m: StickModel, a: number, k = 1): [number, number] => [
      m.drift[0] * 0.3 + reach(m, a) * k * Math.cos(a),
      m.drift[1] * 0.3 + reach(m, a) * k * Math.sin(a),
    ];
    if (sc === 'showcase') {
      const a = abs * TAU * 0.33;
      const r = abs % 6 < 3 ? 1 : 0.55;
      left = [
        left[0] + r * reach(LEFT, a) * Math.cos(a),
        left[1] + r * reach(LEFT, a) * Math.sin(a),
      ];
      right = [right[0] + Math.sin(abs * 1.7) * 0.8, right[1] + Math.sin(abs * 2.3) * 0.6];
      lt = (Math.sin(abs * 1.3) + 1) / 2;
      rt = abs * 0.6 - Math.floor(abs * 0.6);
      if (abs % 0.45 < 0.26)
        pressed.push(SHOWCASE_ORDER[Math.floor(abs / 0.45) % SHOWCASE_ORDER.length]);
    } else if (sc === 'triggerRest') {
      lt = 0.018;
      rt = 0.004;
    } else if (sc === 'rotateLeft' || sc === 'rotateRight') {
      const a = rotationAngle(secs);
      const v = polar(sc === 'rotateLeft' ? LEFT : RIGHT, a, 1 - 0.012 * rand());
      if (sc === 'rotateLeft') left = v;
      else right = v;
    } else if (sc === 'rotateBoth') {
      const a = rotationAngle(secs);
      left = polar(LEFT, a);
      right = polar(RIGHT, -a + 0.7);
    } else if (sc === 'directionsLeft' || sc === 'directionsRight') {
      const slot = Math.floor(secs / 0.95) % 8;
      const within = secs % 0.95;
      if (within > 0.12 && within < 0.75) {
        const a = (DIRECTIONS_DEG[slot] * Math.PI) / 180;
        const v = polar(sc === 'directionsLeft' ? LEFT : RIGHT, a);
        if (sc === 'directionsLeft') left = v;
        else right = v;
      }
    } else if (sc === 'triggerFull') {
      const k = Math.min(1, secs / 0.6);
      lt = 0.018 + (0.962 - 0.018) * k;
      rt = 0.004 + (1 - 0.004) * k;
    } else if (sc === 'buttons') {
      if (secs % 0.35 < 0.2)
        pressed.push(SHOWCASE_ORDER[Math.floor(secs / 0.35) % SHOWCASE_ORDER.length]);
    } else if (sc === 'manual') {
      left = [this.manual.leftStick.x, this.manual.leftStick.y];
      right = [this.manual.rightStick.x, this.manual.rightStick.y];
      lt = this.manual.leftTrigger;
      rt = this.manual.rightTrigger;
      this.manual.buttons.forEach((b, i) => b && pressed.push(i));
    } else if (typeof sc === 'object' && 'pressButton' in sc) {
      if (secs >= 0.4 && secs < 0.7) {
        pressed.push(sc.pressButton);
        if (sc.pressButton === 6) lt = 1;
        if (sc.pressButton === 7) rt = 1;
      }
    } else if (typeof sc === 'object' && 'healthTaps' in sc) {
      const { button, presses } = sc.healthTaps;
      if (healthTapDown(button, presses, secs)) {
        pressed.push(button);
        if (button === 6) lt = 1;
        if (button === 7) rt = 1;
      }
    } else if (typeof sc === 'object' && 'healthHold' in sc) {
      if (healthHoldDown(sc.healthHold, secs)) {
        pressed.push(sc.healthHold);
        if (sc.healthHold === 6) lt = 1;
        if (sc.healthHold === 7) rt = 1;
      }
    } else if (typeof sc === 'object' && 'chord' in sc) {
      if (secs >= 0.3 && secs < 1.8) {
        const ghost = sc.chord.length >= 4;
        for (const i of sc.chord) {
          if (ghost && i === 2) continue;
          pressed.push(i);
          if (i === 6) lt = 1;
          if (i === 7) rt = 1;
        }
      }
    }
    if (sc !== 'manual') {
      left = [left[0] + gauss() * LEFT.noise, left[1] + gauss() * LEFT.noise];
      right = [right[0] + gauss() * RIGHT.noise, right[1] + gauss() * RIGHT.noise];
      if (t % 2_700_000 < 4000) left = [left[0] + 0.22, left[1] - 0.17];
      lt += gauss() * 0.002;
      rt += gauss() * 0.0015;
    }
    return {
      left,
      right,
      lt: Math.max(0, Math.min(1, lt)),
      rt: Math.max(0, Math.min(1, rt)),
      pressed,
      abs,
    };
  }

  snapshot(id: string, t: number): InputSnapshot {
    const { left, right, lt, rt, pressed, abs } = this.sample(t);
    const bx = [q8(left[0]), q8(-left[1]), q8(right[0]), q8(-right[1])];
    const lx = bx[0] / 127.5 - 1;
    const ly = -(bx[1] / 127.5 - 1);
    const rx = bx[2] / 127.5 - 1;
    const ry = -(bx[3] / 127.5 - 1);
    const ltb = Math.round(lt * 255);
    const rtb = Math.round(rt * 255);
    const buttons = BUTTONS.map((_, i) => ({
      pressed: pressed.includes(i),
      value: pressed.includes(i) ? 1 : 0,
    }));
    buttons[6] = { pressed: ltb / 255 >= 0.12, value: ltb / 255 };
    buttons[7] = { pressed: rtb / 255 >= 0.12, value: rtb / 255 };
    const axes = [
      ['LX', lx, bx[0]],
      ['LY', ly, bx[1]],
      ['RX', rx, bx[2]],
      ['RY', ry, bx[3]],
      ['L2', (ltb / 255) * 2 - 1, ltb],
      ['R2', (rtb / 255) * 2 - 1, rtb],
    ] as const;
    const rawAxes = axes.map(([label, value, raw], code) => {
      const r = this.ranges.get(code) ?? [value, value];
      r[0] = Math.min(r[0], value);
      r[1] = Math.max(r[1], value);
      this.ranges.set(code, r);
      return { code, label, value, raw, min: r[0], max: r[1] };
    });
    const rawButtons = LABELS.map((label, code) => {
      const b = buttons[code];
      const p = this.presses.get(code) ?? [false, 0, 0];
      if (b.pressed && !p[0]) p[1]++;
      p[0] = b.pressed;
      p[2] = Math.max(p[2], b.value);
      this.presses.set(code, p);
      return { code, label, pressed: b.pressed, value: b.value, maxValue: p[2], pressCount: p[1] };
    });
    buttons.forEach((b, i) => {
      if (b.pressed && !this.stdPrev[i]) this.stdPresses[i]++;
      this.stdPrev[i] = b.pressed;
    });
    const showcase = this.scenario === 'showcase';
    const k = showcase ? 1 : 0.08;
    const shake = (this.rumble.weak * 0.35 + this.rumble.strong * 0.8) * gauss() * 0.15;
    const touching = showcase && abs % 5 < 2.6;
    const ta = abs * TAU * 0.4;
    return {
      id,
      seq: ++this.seq,
      timestampUs: t,
      buttons,
      leftStick: { x: lx, y: ly },
      rightStick: { x: rx, y: ry },
      leftTrigger: ltb / 255,
      rightTrigger: rtb / 255,
      rawAxes,
      rawButtons,
      buttonPresses: [...this.stdPresses],
      motion: {
        gyro: [
          Math.sin(abs * 1.1) * 45 * k,
          Math.cos(abs * 0.7) * 30 * k,
          Math.sin(abs * 1.9) * 12 * k,
        ],
        accel: [
          Math.sin(abs * 0.5) * 0.18 * k + shake,
          -0.98 + Math.cos(abs * 0.8) * 0.05 * k + shake,
          Math.cos(abs * 0.5) * 0.2 * k,
        ],
        timestampUs: t,
      },
      touch: {
        pressed: showcase && abs % 5 > 2.2 && abs % 5 < 2.5,
        points: [
          { id: 1, active: touching, x: 0.5 + 0.3 * Math.cos(ta), y: 0.5 + 0.3 * Math.sin(ta) },
          { id: 2, active: touching && abs % 5 > 1.2, x: 0.25, y: 0.7 },
        ],
      },
      battery: { level: 78, state: 'discharging' },
    };
  }
}
