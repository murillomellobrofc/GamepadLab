import { create } from 'zustand';
import type { AppInfo, ControllerInfo, PlatformWarning, RumbleFrame } from '@/api/types';

export const TABS = ['test', 'calibration', 'latency', 'sensors', 'history', 'overlay'] as const;
export type TabId = (typeof TABS)[number];
export type ViewMode = 'raw' | 'calibrated';

export interface Toast {
  id: number;
  tone: 'ok' | 'warn' | 'bad' | 'info';
  title: string;
  body?: string;
  action?: { label: string; run: () => void };
}

interface AppState {
  ready: boolean;
  transportKind: 'tauri' | 'mock' | null;
  appInfo: AppInfo | null;
  controllers: ControllerInfo[];
  selectedId: string | null;
  tab: TabId;
  viewMode: ViewMode;
  toasts: Toast[];
  warnings: PlatformWarning[];
  dialog: 'settings' | 'about' | null;
  /** >0 while a test needs every button press (gamepad UI navigation pauses). */
  navLocks: number;
  rumble: Record<string, RumbleFrame>;
  /** Shows every connected controller side by side. */
  multiView: boolean;
  /** The controller is currently driving the UI (toggled with Guide). */
  gamepadNav: boolean;
  announcement: { text: string; n: number };

  setReady: (kind: 'tauri' | 'mock', info: AppInfo | null) => void;
  setControllers: (list: ControllerInfo[]) => void;
  upsertController: (c: ControllerInfo) => void;
  removeController: (id: string) => void;
  select: (id: string | null) => void;
  setTab: (tab: TabId) => void;
  setViewMode: (m: ViewMode) => void;
  toast: (t: Omit<Toast, 'id'>) => void;
  dismissToast: (id: number) => void;
  setWarnings: (w: PlatformWarning[]) => void;
  openDialog: (d: AppState['dialog']) => void;
  lockNav: () => () => void;
  setRumble: (f: RumbleFrame) => void;
  announce: (text: string) => void;
  setMultiView: (on: boolean) => void;
  setGamepadNav: (on: boolean) => void;
}

let toastSeq = 1;

export const MAX_CONTROLLERS = 8;

export const useApp = create<AppState>()((set, get) => ({
  ready: false,
  transportKind: null,
  appInfo: null,
  controllers: [],
  selectedId: null,
  tab: 'test',
  viewMode: 'raw',
  toasts: [],
  warnings: [],
  dialog: null,
  navLocks: 0,
  rumble: {},
  multiView: false,
  gamepadNav: false,
  announcement: { text: '', n: 0 },

  setReady: (kind, info) => set({ ready: true, transportKind: kind, appInfo: info }),
  setControllers: (list) =>
    set((s) => {
      const controllers = list.slice(0, MAX_CONTROLLERS);
      const selectedId =
        s.selectedId && controllers.some((c) => c.id === s.selectedId)
          ? s.selectedId
          : (controllers[0]?.id ?? null);
      return { controllers, selectedId };
    }),
  upsertController: (c) =>
    set((s) => {
      const exists = s.controllers.some((x) => x.id === c.id);
      const controllers = exists
        ? s.controllers.map((x) => (x.id === c.id ? c : x))
        : [...s.controllers, c].slice(0, MAX_CONTROLLERS);
      return { controllers, selectedId: s.selectedId ?? c.id };
    }),
  removeController: (id) =>
    set((s) => {
      const controllers = s.controllers.filter((c) => c.id !== id);
      const selectedId = s.selectedId === id ? (controllers[0]?.id ?? null) : s.selectedId;
      const rumble = { ...s.rumble };
      delete rumble[id];
      return { controllers, selectedId, rumble };
    }),
  select: (id) => set({ selectedId: id, multiView: false }),
  setTab: (tab) => set({ tab }),
  setViewMode: (viewMode) => set({ viewMode }),
  toast: (t) => {
    const id = toastSeq++;
    set((s) => ({ toasts: [...s.toasts.slice(-4), { ...t, id }] }));
    window.setTimeout(() => get().dismissToast(id), t.tone === 'bad' ? 9000 : 5000);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  setWarnings: (warnings) => set({ warnings }),
  openDialog: (dialog) => set({ dialog }),
  lockNav: () => {
    set((s) => ({ navLocks: s.navLocks + 1 }));
    let released = false;
    return () => {
      if (released) return;
      released = true;
      set((s) => ({ navLocks: Math.max(0, s.navLocks - 1) }));
    };
  },
  setRumble: (f) => set((s) => ({ rumble: { ...s.rumble, [f.controllerId]: f } })),
  announce: (text) => set((s) => ({ announcement: { text, n: s.announcement.n + 1 } })),
  setMultiView: (multiView) => set({ multiView }),
  setGamepadNav: (gamepadNav) => set({ gamepadNav }),
}));

export function useSelectedController(): ControllerInfo | null {
  return useApp((s) => s.controllers.find((c) => c.id === s.selectedId) ?? null);
}
