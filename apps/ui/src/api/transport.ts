/** Thin seam between the UI and whatever produces controller data. */
export interface Transport {
  readonly kind: 'tauri' | 'mock';
  invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, cb: (payload: T) => void): Promise<() => void>;
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

let current: Transport | null = null;

export async function getTransport(): Promise<Transport> {
  if (current) return current;
  if (isTauri()) {
    const { createTauriTransport } = await import('./tauriTransport');
    current = createTauriTransport();
  } else {
    const { createMockTransport } = await import('./mock/mockTransport');
    current = createMockTransport();
  }
  return current;
}

/** Synchronous access once bootstrap has resolved the transport. */
export function transport(): Transport {
  if (!current) throw new Error('Transport not initialised yet');
  return current;
}

export function setTransportForTests(t: Transport | null) {
  current = t;
}
