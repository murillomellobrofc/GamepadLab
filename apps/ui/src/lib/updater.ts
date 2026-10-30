import { isTauri } from '@/api/transport';

/** Download progress 0…1, or null while the size is unknown. */
export type InstallProgress = (fraction: number | null) => void;

export type UpdateErrorReason = 'noRelease' | 'offline' | 'other';

export type UpdateStatus =
  | { state: 'disabled' }
  | { state: 'none' }
  | {
      state: 'available';
      version: string;
      install: (onProgress?: InstallProgress) => Promise<void>;
    }
  | { state: 'error'; reason: UpdateErrorReason; message: string };

/** Turns the plugin's English error text into something we can explain in the UI. */
export function updateErrorReason(message: string): UpdateErrorReason {
  if (/valid release JSON|404|not found/i.test(message)) return 'noRelease';
  if (/network|dns|connect|timed? ?out|offline|sending request/i.test(message)) return 'offline';
  return 'other';
}

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
      install: async (onProgress) => {
        let total = 0;
        let received = 0;
        await update.downloadAndInstall((ev) => {
          if (ev.event === 'Started') {
            total = ev.data.contentLength ?? 0;
            onProgress?.(total ? 0 : null);
          } else if (ev.event === 'Progress') {
            received += ev.data.chunkLength;
            onProgress?.(total ? Math.min(1, received / total) : null);
          } else {
            onProgress?.(1);
          }
        });
        const { relaunch } = await import('@tauri-apps/plugin-process');
        await relaunch();
      },
    };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return { state: 'error', reason: updateErrorReason(message), message };
  }
}
