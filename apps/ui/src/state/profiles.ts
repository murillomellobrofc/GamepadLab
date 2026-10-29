import { create } from 'zustand';
import { api } from '@/api/client';
import type { CalibrationProfile, ControllerInfo, DeviceProfiles, Profile } from '@/api/types';

type SaveState = 'idle' | 'saving' | 'saved' | 'error';

interface ProfilesState {
  controllerId: string | null;
  doc: DeviceProfiles | null;
  error: string | null;
  save: SaveState;
  load: (c: ControllerInfo) => Promise<void>;
  reload: () => Promise<void>;
  /** Optimistic edit of the active profile: live preview now, disk later. */
  edit: (fn: (cal: CalibrationProfile) => CalibrationProfile) => void;
  flush: () => Promise<void>;
}

let saveTimer: number | null = null;
let liveTimer: number | null = null;

export const useProfiles = create<ProfilesState>()((set, get) => ({
  controllerId: null,
  doc: null,
  error: null,
  save: 'idle',

  load: async (c) => {
    if (get().controllerId !== c.id) set({ doc: null });
    set({ controllerId: c.id, error: null });
    try {
      const doc = await api.profilesLoad(c.id);
      if (get().controllerId === c.id) set({ doc });
    } catch (e) {
      set({ error: String((e as { message?: string }).message ?? e) });
    }
  },

  reload: async () => {
    const id = get().controllerId;
    if (!id) return;
    const doc = await api.profilesLoad(id);
    set({ doc });
  },

  edit: (fn) => {
    const { doc, controllerId } = get();
    if (!doc || !controllerId) return;
    const idx = doc.profiles.findIndex((p) => p.id === doc.activeProfileId);
    if (idx < 0) return;
    const updated: Profile = {
      ...doc.profiles[idx],
      calibration: fn(structuredClone(doc.profiles[idx].calibration)),
    };
    const profiles = [...doc.profiles];
    profiles[idx] = updated;
    set({ doc: { ...doc, profiles }, save: 'saving' });
    if (liveTimer === null) {
      liveTimer = window.setTimeout(() => {
        liveTimer = null;
        const cur = activeProfile(get().doc);
        if (cur) void api.setActiveCalibration(controllerId, cur.calibration).catch(() => {});
      }, 60);
    }
    if (saveTimer !== null) window.clearTimeout(saveTimer);
    saveTimer = window.setTimeout(() => void get().flush(), 600);
  },

  flush: async () => {
    if (saveTimer !== null) {
      window.clearTimeout(saveTimer);
      saveTimer = null;
    }
    const { doc } = get();
    const p = activeProfile(doc);
    if (!doc || !p) return;
    try {
      await api.profilesUpdate(doc.deviceKey, p);
      set({ save: 'saved', error: null });
    } catch (e) {
      set({ save: 'error', error: String((e as { message?: string }).message ?? e) });
    }
  },
}));

export function activeProfile(doc: DeviceProfiles | null): Profile | null {
  if (!doc) return null;
  return doc.profiles.find((p) => p.id === doc.activeProfileId) ?? doc.profiles[0] ?? null;
}

export function useActiveProfile(): Profile | null {
  return useProfiles((s) => activeProfile(s.doc));
}
