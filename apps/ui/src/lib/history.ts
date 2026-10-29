import { api, errorCode, errorMessage, type NewHistorySession } from '@/api/client';
import type { ControllerInfo, SessionKind } from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { translate } from '@/i18n';
import { useApp } from '@/state/app';
import { useSettings } from '@/state/settings';

/** Saves a finished test to the local history and tells the user. */
export async function saveSession(
  c: ControllerInfo,
  kind: SessionKind,
  score: number | null,
  summary: string,
  payload: Record<string, unknown>,
): Promise<number | null> {
  const session: NewHistorySession = {
    kind,
    controllerName: c.name,
    controllerKey: c.profileKey,
    score,
    summary,
    payload: { controller: c, ...payload },
  };
  try {
    const id = await api.historyAdd(session);
    useApp
      .getState()
      .toast({ tone: 'ok', title: translate(useSettings.getState().locale, 'history.saved') });
    return id;
  } catch (e) {
    useApp.getState().toast(errorToast(errorCode(e), errorMessage(e)));
    return null;
  }
}

/** Keeps at most `max` evenly spaced items (for storing captures). */
export function downsample<T>(items: T[], max: number): T[] {
  if (items.length <= max) return items;
  const step = items.length / max;
  return Array.from({ length: max }, (_, i) => items[Math.floor(i * step)]);
}
