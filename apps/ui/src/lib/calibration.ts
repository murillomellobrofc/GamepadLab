/**
 * TypeScript port of crates/calibration/src/{curve,pipeline}.rs used only for
 * live previews while editing. The backend stays the source of truth; a
 * golden-fixture test keeps both in agreement.
 */
import type {
  CalibrationProfile,
  GateShape,
  ResponseCurve,
  StickCalibration,
  TriggerCalibration,
  Vec2,
} from '@/api/types';

export function defaultStick(): StickCalibration {
  return {
    center: { x: 0, y: 0 },
    rangePos: { x: 1, y: 1 },
    rangeNeg: { x: 1, y: 1 },
    innerDeadzone: 0.05,
    outerDeadzone: 0,
    deadzoneMode: 'radial',
    curve: { kind: 'linear', points: [] },
    sensitivity: { x: 1, y: 1 },
    invertX: false,
    invertY: false,
    swapAxes: false,
    antiDeadzone: 0,
    gate: 'none',
  };
}

export function defaultTrigger(): TriggerCalibration {
  return {
    deadzone: 0.02,
    maxLimit: 1,
    activationPoint: 0.5,
    digital: false,
    restValue: 0,
    fullValue: 1,
  };
}

export function defaultProfile(): CalibrationProfile {
  return {
    leftStick: defaultStick(),
    rightStick: defaultStick(),
    leftTrigger: defaultTrigger(),
    rightTrigger: defaultTrigger(),
    measuredAt: null,
  };
}

const clamp = (v: number, lo: number, hi: number) => (v < lo ? lo : v > hi ? hi : v);

export function knots(curve: ResponseCurve): Vec2[] {
  const pts = curve.points
    .filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y))
    .map((p) => ({ x: clamp(p.x, 0, 1), y: clamp(p.y, 0, 1) }))
    .filter((p) => p.x > 0 && p.x < 1);
  pts.push({ x: 0, y: 0 }, { x: 1, y: 1 });
  pts.sort((a, b) => a.x - b.x);
  const out: Vec2[] = [];
  for (const p of pts)
    if (!out.length || Math.abs(out[out.length - 1].x - p.x) >= 1e-4) out.push(p);
  let maxY = 0;
  for (const p of out) {
    maxY = Math.max(maxY, p.y);
    p.y = maxY;
  }
  out[out.length - 1].y = 1;
  return out;
}

export function monotoneCubic(k: Vec2[], x: number): number {
  const n = k.length;
  if (n < 2) return x;
  const d: number[] = [];
  for (let i = 0; i < n - 1; i++) {
    const dx = k[i + 1].x - k[i].x;
    d.push(dx > 0 ? (k[i + 1].y - k[i].y) / dx : 0);
  }
  const m = new Array(n).fill(0);
  m[0] = d[0];
  m[n - 1] = d[n - 2];
  for (let i = 1; i < n - 1; i++) m[i] = d[i - 1] * d[i] <= 0 ? 0 : (d[i - 1] + d[i]) / 2;
  for (let i = 0; i < n - 1; i++) {
    if (d[i] === 0) {
      m[i] = 0;
      m[i + 1] = 0;
      continue;
    }
    const a = m[i] / d[i];
    const b = m[i + 1] / d[i];
    const s = a * a + b * b;
    if (s > 9) {
      const t = 3 / Math.sqrt(s);
      m[i] = t * a * d[i];
      m[i + 1] = t * b * d[i];
    }
  }
  let i = k.findIndex((p) => p.x >= x);
  i = i === 0 ? 0 : i === -1 ? n - 2 : i - 1;
  const h = k[i + 1].x - k[i].x;
  if (h <= 0) return k[i].y;
  const t = (x - k[i].x) / h;
  const t2 = t * t;
  const t3 = t2 * t;
  const y =
    (2 * t3 - 3 * t2 + 1) * k[i].y +
    (t3 - 2 * t2 + t) * h * m[i] +
    (-2 * t3 + 3 * t2) * k[i + 1].y +
    (t3 - t2) * h * m[i + 1];
  return clamp(y, 0, 1);
}

export function evalCurve(curve: ResponseCurve, x: number): number {
  const v = Number.isFinite(x) ? clamp(x, 0, 1) : 0;
  switch (curve.kind) {
    case 'linear':
      return v;
    case 'smooth':
      return v * v;
    case 'aggressive':
      return Math.pow(v, 0.6);
    case 'sCurve':
      return v * v * (3 - 2 * v);
    case 'custom':
      return monotoneCubic(knots(curve), v);
  }
}

export function deadzoneScale(m: number, inner: number, outer: number): number {
  const top = Math.max(1 - outer, inner + 1e-3);
  return m <= inner ? 0 : (m - inner) / (top - inner);
}

export function antiDeadzone(m: number, ad: number): number {
  return m <= 0 || ad <= 0 ? m : ad + (1 - ad) * m;
}

const shape = (c: ResponseCurve, s: number) => evalCurve(c, Math.min(s, 1)) + Math.max(0, s - 1);

export function gateRadius(gate: GateShape, angle: number): number {
  if (gate === 'circle') return 1;
  if (gate === 'octagon') {
    const sector = Math.PI / 4;
    const local = (((angle % sector) + sector) % sector) - Math.PI / 8;
    return Math.cos(Math.PI / 8) / Math.cos(local);
  }
  return Infinity;
}

export function applyStick(c: StickCalibration, raw: Vec2): Vec2 {
  if (!Number.isFinite(raw.x) || !Number.isFinite(raw.y)) return { x: 0, y: 0 };
  let x = c.swapAxes ? raw.y : raw.x;
  let y = c.swapAxes ? raw.x : raw.y;
  x -= c.center.x;
  y -= c.center.y;
  const range = (v: number, pos: number, neg: number) => v / clamp(v >= 0 ? pos : neg, 0.2, 1.5);
  x = range(x, c.rangePos.x, c.rangeNeg.x);
  y = range(y, c.rangePos.y, c.rangeNeg.y);
  if (c.invertX) x = -x;
  if (c.invertY) y = -y;
  const inner = clamp(c.innerDeadzone, 0, 0.9);
  const outer = clamp(c.outerDeadzone, 0, 0.5);
  if (c.deadzoneMode === 'radial') {
    const m = Math.hypot(x, y);
    if (m <= 1e-6) {
      x = 0;
      y = 0;
    } else {
      const s = deadzoneScale(m, inner, outer);
      const out = s > 0 ? antiDeadzone(shape(c.curve, s), c.antiDeadzone) : 0;
      x = (x * out) / m;
      y = (y * out) / m;
    }
  } else {
    const axis = (v: number) => {
      const s = deadzoneScale(Math.abs(v), inner, outer);
      return s <= 0 ? 0 : Math.sign(v) * antiDeadzone(shape(c.curve, s), c.antiDeadzone);
    };
    x = axis(x);
    y = axis(y);
  }
  x *= c.sensitivity.x;
  y *= c.sensitivity.y;
  const limit = gateRadius(c.gate, Math.atan2(y, x));
  const len = Math.hypot(x, y);
  if (Number.isFinite(limit) && len > limit && len > 0) {
    x *= limit / len;
    y *= limit / len;
  }
  return { x: clamp(x, -1, 1), y: clamp(y, -1, 1) };
}

export function applyTrigger(c: TriggerCalibration, raw: number): number {
  if (!Number.isFinite(raw)) return 0;
  const span = Math.max(c.fullValue - c.restValue, 0.05);
  let v = clamp((raw - c.restValue) / span, 0, 1);
  const dz = clamp(c.deadzone, 0, 0.5);
  const max = clamp(c.maxLimit, dz + 0.05, 1);
  v = clamp((v - dz) / (max - dz), 0, 1);
  return c.digital ? (v >= c.activationPoint ? 1 : 0) : v;
}
