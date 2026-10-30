import { getTransport } from './transport';
import type {
  AppErrorEvent,
  AppInfo,
  ButtonPlan,
  CalibrationProfile,
  CaptureKind,
  CaptureResult,
  ConnectionEvent,
  ControllerInfo,
  DemoScenario,
  DeviceProfiles,
  FirmwareOp,
  FirmwareResult,
  SwitchProposals,
  HealthProgress,
  HealthReport,
  HistorySession,
  PlatformWarning,
  Profile,
  RumbleFrame,
  RumbleRequest,
  StickCalibration,
  StickReport,
  StickSide,
  TriggerAnalysis,
  TriggerEffect,
  TriggerSide,
  UiFrame,
} from './types';
import type { LogLevel, OverlaySettings } from '@/state/settings';

// Calls wait for the transport instead of failing: the UI can render (and be
// clicked) before bootstrap has finished loading it.
const call = <T>(cmd: string, args?: Record<string, unknown>) =>
  getTransport().then((t) => t.invoke<T>(cmd, args));
const listen = <T>(event: string, cb: (payload: T) => void) =>
  getTransport().then((t) => t.listen<T>(event, cb));

export interface ManualDemoInput {
  leftStick: { x: number; y: number };
  rightStick: { x: number; y: number };
  leftTrigger: number;
  rightTrigger: number;
  buttons: boolean[];
}

export interface NewHistorySession {
  kind: HistorySession['kind'];
  controllerName: string;
  controllerKey: string;
  score: number | null;
  summary: string;
  payload: unknown;
}

/** Typed wrappers around every backend command (see src-tauri/src/commands.rs). */
export const api = {
  appInfo: () => call<AppInfo>('app_info'),
  listControllers: () => call<ControllerInfo[]>('list_controllers'),
  platformWarnings: () => call<PlatformWarning[]>('platform_warnings'),
  setUiRate: (hz: number) => call<void>('set_ui_rate', { hz }),
  setAppIcon: (variant: 'dark' | 'light') => call<void>('set_app_icon', { variant }),
  resetMinMax: (id: string) => call<void>('reset_minmax', { id }),

  setDemo: (enabled: boolean) => call<void>('set_demo', { enabled }),
  demoScenario: (scenario: DemoScenario) => call<void>('demo_scenario', { scenario }),
  demoManual: (input: ManualDemoInput) => call<void>('demo_manual', { input }),

  captureStart: (id: string, kind: CaptureKind, target: StickSide) =>
    call<void>('capture_start', { id, kind, target }),
  captureStop: () => call<CaptureResult>('capture_stop'),
  captureClear: () => call<void>('capture_clear'),
  buttonHealthStart: (id: string) => call<void>('button_health_start', { id }),
  buttonHealthProgress: () => call<HealthProgress>('button_health_progress'),
  buttonHealthStop: (plans: ButtonPlan[]) => call<HealthReport>('button_health_stop', { plans }),
  buttonHealthCancel: () => call<void>('button_health_cancel'),
  analyzeStick: (stick: StickSide, calibration: StickCalibration | null) =>
    call<StickReport>('analyze_stick', { stick, calibration }),
  analyzeTrigger: (side: TriggerSide) =>
    call<TriggerAnalysis>('analyze_trigger_cmd', { sideName: side }),
  setActiveCalibration: (id: string, profile: CalibrationProfile | null) =>
    call<void>('set_active_calibration', { id, profile }),

  profilesLoad: (id: string) => call<DeviceProfiles>('profiles_load', { id }),
  profilesCreate: (deviceKey: string, name: string, calibration: CalibrationProfile | null) =>
    call<Profile>('profiles_create', { deviceKey, name, calibration }),
  profilesUpdate: (deviceKey: string, profile: Profile) =>
    call<Profile>('profiles_update', { deviceKey, profile }),
  profilesRename: (deviceKey: string, profileId: string, name: string) =>
    call<void>('profiles_rename', { deviceKey, profileId, name }),
  profilesDuplicate: (deviceKey: string, profileId: string) =>
    call<Profile>('profiles_duplicate', { deviceKey, profileId }),
  profilesDelete: (deviceKey: string, profileId: string) =>
    call<void>('profiles_delete', { deviceKey, profileId }),
  profilesSetActive: (deviceKey: string, profileId: string) =>
    call<void>('profiles_set_active', { deviceKey, profileId }),
  profilesReset: (deviceKey: string, profileId: string) =>
    call<Profile>('profiles_reset', { deviceKey, profileId }),
  profilesExport: (deviceKey: string, profileId: string) =>
    call<string>('profiles_export', { deviceKey, profileId }),
  profilesImport: (deviceKey: string, json: string) =>
    call<Profile>('profiles_import', { deviceKey, json }),

  rumblePlay: (id: string, request: RumbleRequest) => call<void>('rumble_play', { id, request }),
  rumbleSequence: (id: string, requests: RumbleRequest[]) =>
    call<void>('rumble_sequence', { id, requests }),
  rumbleMotorTest: (id: string, intensity: number) =>
    call<void>('rumble_motor_test', { id, intensity }),
  rumbleStop: (id: string) => call<void>('rumble_stop', { id }),
  setTriggerEffects: (id: string, left: TriggerEffect, right: TriggerEffect) =>
    call<void>('set_trigger_effects', { id, left, right }),
  setLightbar: (id: string, r: number, g: number, b: number) =>
    call<void>('set_lightbar', { id, r, g, b }),
  setPlayerLeds: (id: string, mask: number) => call<void>('set_player_leds', { id, mask }),
  enableSensors: (id: string) => call<void>('enable_sensors', { id }),

  firmware: (id: string, op: FirmwareOp) => call<FirmwareResult>('firmware', { id, op }),
  firmwareSwitchPropose: (id: string) => call<SwitchProposals>('firmware_switch_propose', { id }),

  historyAdd: (session: NewHistorySession) => call<number>('history_add', { session }),
  historyList: (limit = 200) => call<HistorySession[]>('history_list', { limit }),
  historyDelete: (id: number) => call<void>('history_delete', { id }),
  historyClear: () => call<void>('history_clear'),

  saveTextFile: (path: string, contents: string) =>
    call<void>('save_text_file', { path, contents }),
  saveBinaryFile: (path: string, base64: string) =>
    call<void>('save_binary_file', { path, base64 }),
  readTextFile: (path: string) => call<string>('read_text_file', { path }),

  diagnostics: () => call<string>('diagnostics'),
  setLogLevel: (level: LogLevel) => call<void>('set_log_level', { level }),
  logFrontend: (level: LogLevel, message: string) => call<void>('log_frontend', { level, message }),
  openPrivacySettings: () => call<void>('open_privacy_settings'),

  overlayOpen: (settings: OverlaySettings, controllerId: string | null) =>
    call<void>('overlay_open', { settings, controllerId }),
  overlayClose: () => call<void>('overlay_close'),
  overlayUpdate: (settings: OverlaySettings, controllerId: string | null) =>
    call<void>('overlay_update', { settings, controllerId }),
};

export const events = {
  frame: (cb: (f: UiFrame) => void) => listen<UiFrame>('gp://frame', cb),
  connection: (cb: (e: ConnectionEvent) => void) => listen<ConnectionEvent>('gp://connection', cb),
  controllerUpdated: (cb: (c: ControllerInfo) => void) =>
    listen<ControllerInfo>('gp://controller-updated', cb),
  error: (cb: (e: AppErrorEvent) => void) => listen<AppErrorEvent>('gp://error', cb),
  rumble: (cb: (f: RumbleFrame) => void) => listen<RumbleFrame>('gp://rumble', cb),
  overlaySettings: (cb: (s: { settings: OverlaySettings; controllerId: string | null }) => void) =>
    listen<{ settings: OverlaySettings; controllerId: string | null }>('overlay://settings', cb),
};

/** Turns any thrown backend error into a human-readable string. */
export function errorCode(e: unknown): string | null {
  if (e && typeof e === 'object' && 'code' in e) return String((e as { code: unknown }).code);
  return null;
}

export function errorMessage(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object' && 'message' in e)
    return String((e as { message: unknown }).message);
  return String(e);
}
