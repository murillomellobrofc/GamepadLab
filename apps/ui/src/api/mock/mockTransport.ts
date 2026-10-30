import type { Transport } from '../transport';
import type {
  AppInfo,
  ControllerInfo,
  DemoScenario,
  FrameEntry,
  PollStats,
  UiFrame,
} from '../types';
import { DemoSim, type ManualState } from './demoSim';
import { mockExtras } from './mockExtras';

type Listener = (payload: unknown) => void;

export const DEMO_INFO: ControllerInfo = {
  id: 'demo:dualsense',
  name: 'DualSense (demo)',
  vendorId: 0x054c,
  productId: 0x0ce6,
  serial: 'DEMO-0001',
  kind: 'ps5',
  connection: 'usb',
  buttonCount: 19,
  axisCount: 6,
  capabilities: {
    rumble: true,
    triggerRumble: false,
    adaptiveTriggers: true,
    gyro: true,
    accel: true,
    touchpad: true,
    battery: true,
    lightbar: true,
    playerLeds: true,
  },
  battery: { level: 78, state: 'discharging' },
  backend: 'demo',
  isVirtual: false,
  isDemo: true,
  profileKey: 'demo:dualsense',
};

/** Shared state of the in-browser backend, also used by mockExtras. */
export class MockBackend {
  listeners = new Map<string, Set<Listener>>();
  sim = new DemoSim();
  demo = false;
  uiRate = 60;
  timer: number | null = null;
  clockStart = performance.now();
  lastT = 0;
  reportCount = 0;
  intervals: number[] = [];
  capture: {
    target: string;
    kind: string;
    samples: { t: number; x: number; y: number }[];
    start: number;
  } | null = null;

  now(): number {
    return Math.round((performance.now() - this.clockStart) * 1000);
  }

  emit(event: string, payload: unknown) {
    this.listeners.get(event)?.forEach((l) => l(payload));
  }

  controllers(): ControllerInfo[] {
    return this.demo ? [DEMO_INFO] : [];
  }

  setDemo(on: boolean) {
    if (on === this.demo) return;
    this.demo = on;
    this.emit('gp://connection', { controller: DEMO_INFO, connected: on });
    if (on) {
      this.sim.setScenario('showcase', this.now());
      this.lastT = this.now();
      this.startLoop();
    } else if (this.timer !== null) {
      window.clearInterval(this.timer);
      this.timer = null;
    }
  }

  startLoop() {
    if (this.timer !== null) window.clearInterval(this.timer);
    this.timer = window.setInterval(() => this.tick(), 1000 / this.uiRate);
  }

  /** Simulates 250 Hz reports since the last tick, then emits one frame. */
  tick() {
    const now = this.now();
    let t = this.lastT;
    const recent: number[] = [];
    while (t + 4000 <= now) {
      const dt = Math.round(4000 + (Math.random() - 0.5) * 300);
      t += dt;
      recent.push(dt);
      this.reportCount++;
      if (this.capture) {
        const s = this.sim.sample(t);
        const isTrigger = this.capture.target.endsWith('Trigger');
        const tgt = this.capture.target;
        const [x, y] = isTrigger
          ? [tgt === 'leftTrigger' ? s.lt : s.rt, 0]
          : tgt === 'leftStick'
            ? s.left
            : s.right;
        if (this.capture.start < 0) this.capture.start = t;
        this.capture.samples.push({ t: t - this.capture.start, x, y });
      }
    }
    this.lastT = t;
    this.intervals.push(...recent);
    if (this.intervals.length > 250) this.intervals.splice(0, this.intervals.length - 250);
    const frame: UiFrame = {
      emittedAtUs: now,
      entries: [
        {
          snapshot: this.sim.snapshot(DEMO_INFO.id, t),
          calibrated: mockExtras.calibrated(this, t),
          stats: this.stats(recent),
        } as FrameEntry,
      ],
    };
    this.emit('gp://frame', frame);
  }

  stats(recent: number[]): PollStats {
    const v = this.intervals;
    const n = v.length || 1;
    const mean = v.reduce((a, b) => a + b, 0) / n;
    const sd = Math.sqrt(v.reduce((a, b) => a + (b - mean) ** 2, 0) / n);
    const sorted = [...v].sort((a, b) => a - b);
    return {
      hz: mean ? 1e6 / mean : 0,
      avgMs: mean / 1000,
      minMs: (sorted[0] ?? 0) / 1000,
      maxMs: (sorted[sorted.length - 1] ?? 0) / 1000,
      jitterMs: sd / 1000,
      p99Ms: (sorted[Math.max(0, Math.ceil(sorted.length * 0.99) - 1)] ?? 0) / 1000,
      samples: v.length,
      reportCount: this.reportCount,
      source: 'report',
      recentIntervalsUs: recent,
    };
  }
}

export function createMockTransport(): Transport {
  const b = new MockBackend();
  // Test hook for Playwright: lets a test "press" demo buttons.
  (window as unknown as { __gamepadlabMock?: MockBackend }).__gamepadlabMock = b;
  const info: AppInfo = {
    version: '0.1.0',
    os: 'browser',
    osVersion: navigator.userAgent.split(') ')[0].replace('Mozilla/5.0 (', ''),
    arch: 'web',
    backends: ['mock'],
    updaterConfigured: false,
    packageManaged: false,
    dataDir: '(navegador, somente memória)',
  };
  const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
    app_info: () => info,
    list_controllers: () => b.controllers(),
    platform_warnings: () => [],
    set_ui_rate: (a) => {
      b.uiRate = Number(a.hz) || 60;
      if (b.demo) b.startLoop();
    },
    set_log_level: () => undefined,
    log_frontend: () => undefined,
    reset_minmax: () => b.sim.resetMinMax(),
    set_demo: (a) => b.setDemo(Boolean(a.enabled)),
    demo_scenario: (a) => b.sim.setScenario(a.scenario as DemoScenario, b.now()),
    demo_manual: (a) => {
      b.sim.manual = a.input as ManualState;
    },
    enable_sensors: () => false,
    diagnostics: () =>
      [
        'GamepadLab diagnostics (browser mock)',
        `version: ${info.version}`,
        `os: ${info.osVersion}`,
        `controllers: ${
          b
            .controllers()
            .map((c) => c.name)
            .join(', ') || '(none)'
        }`,
        'No telemetry: this report is only copied to your clipboard.',
      ].join('\n'),
    open_privacy_settings: () => undefined,
    ...mockExtras.handlers(b),
  };
  return {
    kind: 'mock',
    async invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
      const h = handlers[cmd];
      if (!h) throw { code: 'internal', message: `mock: command ${cmd} not implemented` };
      return (await h(args)) as T;
    },
    async listen<T>(event: string, cb: (payload: T) => void) {
      const set = b.listeners.get(event) ?? new Set();
      set.add(cb as Listener);
      b.listeners.set(event, set);
      return () => set.delete(cb as Listener);
    },
  };
}
