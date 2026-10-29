import type { Transport } from '../transport';
import type { AppInfo } from '../types';

/**
 * Browser-only transport (plain `vite dev`, Playwright). Phase 1 stub:
 * answers app info and nothing else.
 */
export function createMockTransport(): Transport {
  const info: AppInfo = {
    version: '0.1.0',
    os: 'browser',
    osVersion: navigator.userAgent,
    arch: 'wasm',
    backends: ['mock'],
    updaterConfigured: false,
    dataDir: '(browser)',
  };
  return {
    kind: 'mock',
    async invoke<T>(cmd: string): Promise<T> {
      if (cmd === 'app_info') return info as T;
      if (cmd === 'list_controllers' || cmd === 'platform_warnings') return [] as T;
      return undefined as T;
    },
    async listen() {
      return () => {};
    },
  };
}
