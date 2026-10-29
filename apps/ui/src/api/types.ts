/**
 * TypeScript mirror of the Rust data model (crates/input, crates/calibration,
 * crates/profiles, crates/haptics). Field names follow serde camelCase.
 *
 * Stick convention: x right = +1, y UP = +1 (math convention, like gilrs and
 * XInput). Canvas code flips y when drawing.
 */

export type ControllerKind = 'xbox' | 'ps4' | 'ps5' | 'switchPro' | 'generic';
export type ConnectionType = 'usb' | 'bluetooth' | 'unknown';
export type BatteryState = 'discharging' | 'charging' | 'full' | 'wired' | 'unknown';

export interface Capabilities {
  rumble: boolean;
  triggerRumble: boolean;
  adaptiveTriggers: boolean;
  gyro: boolean;
  accel: boolean;
  touchpad: boolean;
  battery: boolean;
  lightbar: boolean;
  playerLeds: boolean;
}

export interface BatteryInfo {
  level: number | null;
  state: BatteryState;
}

export interface ControllerInfo {
  id: string;
  name: string;
  vendorId: number;
  productId: number;
  serial: string | null;
  kind: ControllerKind;
  connection: ConnectionType;
  buttonCount: number;
  axisCount: number;
  capabilities: Capabilities;
  battery: BatteryInfo | null;
  backend: string;
  isVirtual: boolean;
  isDemo: boolean;
  profileKey: string;
}

/** Index order of InputSnapshot.buttons (W3C "standard" layout + extras). */
export const BUTTONS = [
  'south',
  'east',
  'west',
  'north',
  'leftBumper',
  'rightBumper',
  'leftTrigger',
  'rightTrigger',
  'select',
  'start',
  'leftStick',
  'rightStick',
  'dpadUp',
  'dpadDown',
  'dpadLeft',
  'dpadRight',
  'guide',
  'touchpad',
  'misc',
  'paddle1',
  'paddle2',
  'paddle3',
  'paddle4',
] as const;
export type ButtonName = (typeof BUTTONS)[number];
export const BUTTON_INDEX = Object.fromEntries(BUTTONS.map((b, i) => [b, i])) as Record<
  ButtonName,
  number
>;

export interface Vec2 {
  x: number;
  y: number;
}

export interface ButtonState {
  pressed: boolean;
  value: number;
}

export interface RawAxis {
  code: number;
  label: string;
  value: number;
  raw: number | null;
  min: number;
  max: number;
}

export interface RawButton {
  code: number;
  label: string;
  pressed: boolean;
  value: number;
  maxValue: number;
  pressCount: number;
}

export interface MotionSample {
  gyro: [number, number, number];
  accel: [number, number, number];
  timestampUs: number;
}

export interface TouchPoint {
  id: number;
  active: boolean;
  x: number;
  y: number;
}

export interface TouchState {
  pressed: boolean;
  points: TouchPoint[];
}

export interface InputSnapshot {
  id: string;
  seq: number;
  timestampUs: number;
  buttons: ButtonState[];
  leftStick: Vec2;
  rightStick: Vec2;
  leftTrigger: number;
  rightTrigger: number;
  rawAxes: RawAxis[];
  rawButtons: RawButton[];
  /** Rising edges per standard button, counted at full report rate. */
  buttonPresses: number[];
  motion: MotionSample | null;
  touch: TouchState | null;
  battery: BatteryInfo | null;
}

export interface CalibratedView {
  leftStick: Vec2;
  rightStick: Vec2;
  leftTrigger: number;
  rightTrigger: number;
  buttons: ButtonState[];
}

export interface PollStats {
  hz: number;
  avgMs: number;
  minMs: number;
  maxMs: number;
  jitterMs: number;
  p99Ms: number;
  samples: number;
  reportCount: number;
  source: 'report' | 'change';
  recentIntervalsUs: number[];
}

export interface FrameEntry {
  snapshot: InputSnapshot;
  calibrated: CalibratedView | null;
  stats: PollStats;
}

export interface UiFrame {
  emittedAtUs: number;
  entries: FrameEntry[];
}

export interface ConnectionEvent {
  controller: ControllerInfo;
  connected: boolean;
}

export interface AppErrorEvent {
  code: AppErrorCode;
  message: string;
  controllerId: string | null;
}

export type AppErrorCode =
  | 'disconnected'
  | 'permissionDenied'
  | 'driverMissing'
  | 'backendUnavailable'
  | 'rumbleUnsupported'
  | 'hidAccess'
  | 'io'
  | 'invalidInput'
  | 'notFound'
  | 'internal';

/* ---------------------------------------------------------------- calibration */

export type DeadzoneMode = 'radial' | 'axial' | 'scaledRadial';
export type GateShape = 'none' | 'circle' | 'octagon' | 'square';
export type CurveKind = 'linear' | 'smooth' | 'aggressive' | 'sCurve' | 'custom';

export interface ResponseCurve {
  kind: CurveKind;
  /** Control points for 'custom', x and y in 0..1, sorted by x. */
  points: Vec2[];
}

export interface StickCalibration {
  center: Vec2;
  rangePos: Vec2;
  rangeNeg: Vec2;
  innerDeadzone: number;
  outerDeadzone: number;
  deadzoneMode: DeadzoneMode;
  curve: ResponseCurve;
  sensitivity: Vec2;
  invertX: boolean;
  invertY: boolean;
  swapAxes: boolean;
  antiDeadzone: number;
  gate: GateShape;
}

export interface TriggerCalibration {
  deadzone: number;
  maxLimit: number;
  activationPoint: number;
  digital: boolean;
  restValue: number;
  fullValue: number;
}

export interface CalibrationProfile {
  leftStick: StickCalibration;
  rightStick: StickCalibration;
  leftTrigger: TriggerCalibration;
  rightTrigger: TriggerCalibration;
  measuredAt: string | null;
}

export type DriftGrade = 'excellent' | 'acceptable' | 'poor' | 'defective';

export interface RestAnalysis {
  samples: number;
  durationMs: number;
  mean: Vec2;
  std: Vec2;
  stdRadial: number;
  maxDeviation: number;
  offsetPct: number;
  offsetPctX: number;
  offsetPctY: number;
  driftRatePerSec: number;
  noiseP2p: number;
  grade: DriftGrade;
  suggestedDeadzone: number;
}

export type SectorStatus = 'ok' | 'under' | 'over' | 'missing';

export interface Sector {
  index: number;
  angleDeg: number;
  maxRadius: number;
  samples: number;
  status: SectorStatus;
}

export interface CircularityAnalysis {
  samples: number;
  sectors: Sector[];
  radiusMin: number;
  radiusAvg: number;
  radiusMax: number;
  roundnessPct: number;
  avgErrorPct: number;
  coveragePct: number;
  failingSectors: number[];
  incompleteTravel: boolean;
  suggestedOuterDeadzone: number;
  suggestedScale: number;
}

export interface DirectionResult {
  name: string;
  targetDeg: number;
  measuredDeg: number | null;
  angleErrorDeg: number | null;
  reach: number;
  samples: number;
  ok: boolean;
}

export interface DirectionsAnalysis {
  directions: DirectionResult[];
  gateShape: GateShape;
  diagonalRatio: number;
  rangePos: Vec2;
  rangeNeg: Vec2;
}

export interface SpikeEvent {
  index: number;
  timestampUs: number;
  magnitude: number;
}

export interface SpikeReport {
  spikeCount: number;
  jumpCount: number;
  noiseRms: number;
  events: SpikeEvent[];
  verdict: 'clean' | 'noisy' | 'faulty';
}

export interface TriggerAnalysis {
  restMin: number;
  restMax: number;
  restNoise: number;
  maxReached: number;
  fullTravel: boolean;
  suggestedDeadzone: number;
  suggestedMax: number;
}

export interface StickReport {
  stick: 'left' | 'right';
  rest: RestAnalysis | null;
  circularity: CircularityAnalysis | null;
  directions: DirectionsAnalysis | null;
  spikes: SpikeReport | null;
  score: QualityScore;
  suggested: StickCalibration;
}

export interface QualityScore {
  score: number;
  grade: 'A' | 'B' | 'C' | 'D' | 'F';
  issues: string[];
}

export type CaptureKind = 'rest' | 'rotation' | 'directions' | 'triggerRest' | 'triggerFull';
export type StickSide = 'left' | 'right';
export type TriggerSide = 'left' | 'right';

export interface CaptureSample {
  t: number;
  x: number;
  y: number;
}

export interface CaptureResult {
  kind: CaptureKind;
  target: StickSide;
  samples: CaptureSample[];
  durationMs: number;
}

/* ------------------------------------------------------------------- profiles */

export interface Profile {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  calibration: CalibrationProfile;
}

export interface DeviceProfiles {
  schemaVersion: number;
  deviceKey: string;
  deviceName: string;
  activeProfileId: string | null;
  profiles: Profile[];
}

/* -------------------------------------------------------------------- haptics */

export type RumblePatternKind = 'constant' | 'pulse' | 'wave' | 'rampUp' | 'rampDown' | 'heartbeat';

export interface RumbleRequest {
  weak: number;
  strong: number;
  durationMs: number;
  pattern: RumblePatternKind;
  periodMs: number;
}

export type TriggerEffect =
  | { type: 'off' }
  | { type: 'feedback'; position: number; strength: number }
  | { type: 'weapon'; start: number; end: number; strength: number }
  | { type: 'vibration'; position: number; amplitude: number; frequency: number };

export interface RumbleFrame {
  controllerId: string;
  weak: number;
  strong: number;
  active: boolean;
}

/* -------------------------------------------------------------------- history */

export type SessionKind = 'test' | 'buttons' | 'calibration' | 'latency' | 'ghosting' | 'vibration';

export interface HistorySession {
  id: number;
  createdAt: string;
  kind: SessionKind;
  controllerName: string;
  controllerKey: string;
  score: number | null;
  summary: string;
  payload: unknown;
}

/* ---------------------------------------------------------------------- misc */

export interface AppInfo {
  version: string;
  os: string;
  osVersion: string;
  arch: string;
  backends: string[];
  updaterConfigured: boolean;
  dataDir: string;
}

export interface PlatformWarning {
  code: 'steamRunning' | 'inputMonitoring' | 'hidrawPermission' | 'virtualDevice';
  message: string;
}

export type DemoScenario =
  | 'showcase'
  | 'idle'
  | 'rotateLeft'
  | 'rotateRight'
  | 'directionsLeft'
  | 'directionsRight'
  | 'triggerRest'
  | 'triggerFull'
  | 'buttons'
  | 'manual'
  | { pressButton: number }
  | { chord: number[] };
