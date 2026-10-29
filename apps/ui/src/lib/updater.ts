import { isTauri } from '@/api/transport';

export type UpdateStatus =
  | { state: 'disabled' }
  | { state: 'none' }
  | { state: 'available'; version: string; install: () => Promise<void> }
  | { state: 'error'; message: string };

/** Asks the Tauri updater for a newer release. Never runs in browser mode. */
export async function checkForUpdate(configured: boolean): Promise<UpdateStatus> {
  if (!isTauri() || !configured) return { state: 'disabled' };
  try {
    const { check } = await import('@tauri-apps/plugin-updater');
    const update = await check();
    if (!update) return { state: 'none' };
    return {
      state: 'available',
      version: update.version,
      install: async () => {
        await update.downloadAndInstall();
        const { relaunch } = await import('@tauri-apps/plugin-process');
        await relaunch();
      },
    };
  } catch (e) {
    return { state: 'error', message: e instanceof Error ? e.message : String(e) };
  }
}
