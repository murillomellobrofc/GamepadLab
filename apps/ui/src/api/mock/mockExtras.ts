import type { CalibratedView } from '../types';
import type { MockBackend } from './mockTransport';

/**
 * Commands added by later phases (calibration, profiles, haptics, history).
 * Kept apart from the core mock so each phase can extend it.
 */
export const mockExtras = {
  calibrated(_b: MockBackend, _t: number): CalibratedView | null {
    return null;
  },
  handlers(_b: MockBackend): Record<string, (a: Record<string, unknown>) => unknown> {
    return {};
  },
};
