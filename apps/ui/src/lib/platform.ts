import type { AppInfo } from '@/api/types';

export type Os = 'macos' | 'windows' | 'linux' | 'other';

export function detectOs(info: AppInfo | null): Os {
  const fromInfo = info?.os;
  if (fromInfo === 'macos' || fromInfo === 'windows' || fromInfo === 'linux') return fromInfo;
  const ua = typeof navigator !== 'undefined' ? navigator.userAgent : '';
  if (/Mac/i.test(ua)) return 'macos';
  if (/Win/i.test(ua)) return 'windows';
  if (/Linux|X11/i.test(ua)) return 'linux';
  return 'other';
}

export const OS_LABEL: Record<Os, string> = {
  macos: 'macOS',
  windows: 'Windows',
  linux: 'Linux',
  other: 'Web',
};
