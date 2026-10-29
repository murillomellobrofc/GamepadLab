import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Transport } from './transport';

export function createTauriTransport(): Transport {
  return {
    kind: 'tauri',
    invoke: <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args),
    listen: async <T>(event: string, cb: (payload: T) => void) => {
      const unlisten = await listen<T>(event, (e) => cb(e.payload));
      return unlisten;
    },
  };
}
