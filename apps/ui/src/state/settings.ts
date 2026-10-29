import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

export type ThemePref = 'dark' | 'light' | 'auto';
export type MotionPref = 'system' | 'reduced' | 'full';
export type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';
export type ChromaKey = 'transparent' | 'green' | 'blue' | 'magenta';

export interface OverlaySettings {
  background: ChromaKey;
  scale: number;
  showSticks: boolean;
  showTriggers: boolean;
  showTrail: boolean;
  calibrated: boolean;
  alwaysOnTop: boolean;
}

export interface Settings {
  theme: ThemePref;
  highContrast: boolean;
  locale: 'pt-BR' | 'en';
  textScale: number;
  motion: MotionPref;
  uiRate: 60 | 120;
  gamepadNav: boolean;
  logLevel: LogLevel;
  autoUpdate: boolean;
  showRawPanel: boolean;
  overlay: OverlaySettings;
  set: (patch: Partial<Omit<Settings, 'set' | 'setOverlay'>>) => void;
  setOverlay: (patch: Partial<OverlaySettings>) => void;
}

function detectLocale(): Settings['locale'] {
  try {
    return navigator.language?.toLowerCase().startsWith('pt') ? 'pt-BR' : 'en';
  } catch {
    return 'pt-BR';
  }
}

/** Storage that never throws (private windows, blocked storage, previews). */
const safeStorage = createJSONStorage(() => ({
  getItem: (k: string) => {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  },
  setItem: (k: string, v: string) => {
    try {
      localStorage.setItem(k, v);
    } catch {
      /* ignore */
    }
  },
  removeItem: (k: string) => {
    try {
      localStorage.removeItem(k);
    } catch {
      /* ignore */
    }
  },
}));

export const useSettings = create<Settings>()(
  persist(
    (set) => ({
      theme: 'dark',
      highContrast: false,
      locale: detectLocale(),
      textScale: 1,
      motion: 'system',
      uiRate: 60,
      gamepadNav: true,
      logLevel: 'info',
      autoUpdate: true,
      showRawPanel: true,
      overlay: {
        background: 'transparent',
        scale: 1,
        showSticks: true,
        showTriggers: true,
        showTrail: true,
        calibrated: false,
        alwaysOnTop: true,
      },
      set: (patch) => set(patch),
      setOverlay: (patch) => set((s) => ({ overlay: { ...s.overlay, ...patch } })),
    }),
    {
      name: 'gamepadlab.settings',
      version: 1,
      storage: safeStorage,
      partialize: ({ set: _s, setOverlay: _o, ...rest }) => rest,
    },
  ),
);

export function prefersReducedMotion(pref: MotionPref): boolean {
  if (pref === 'reduced') return true;
  if (pref === 'full') return false;
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}
