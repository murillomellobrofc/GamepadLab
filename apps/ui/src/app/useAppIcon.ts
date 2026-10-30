import { useEffect, useState } from 'react';
import { api } from '@/api/client';
import { isTauri } from '@/api/transport';
import { useSettings, type AppIconPref, type ThemePref } from '@/state/settings';

export type IconVariant = 'dark' | 'light';

function systemDark(): boolean {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  } catch {
    return true;
  }
}

/** The chosen symbol, or with "auto" the one matching the app's theme. */
export function resolveIconVariant(
  appIcon: AppIconPref,
  theme: ThemePref,
  systemIsDark: boolean,
): IconVariant {
  if (appIcon !== 'auto') return appIcon;
  return theme === 'dark' || (theme === 'auto' && systemIsDark) ? 'dark' : 'light';
}

/** The symbol to show: the chosen one, or the one matching the app's theme. */
export function useIconVariant(): IconVariant {
  const appIcon = useSettings((s) => s.appIcon);
  const theme = useSettings((s) => s.theme);
  const [dark, setDark] = useState(systemDark);
  useEffect(() => {
    if (theme !== 'auto') return;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => setDark(mq.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, [theme]);
  return resolveIconVariant(appIcon, theme, dark);
}

/** Keeps the Dock / window icon of the desktop app on the current variant. */
export function useAppIcon(): IconVariant {
  const variant = useIconVariant();
  useEffect(() => {
    if (isTauri()) void api.setAppIcon(variant).catch(() => undefined);
  }, [variant]);
  return variant;
}

/** URL of the in-app logo for a variant (public/brand). */
export const brandUrl = (variant: IconVariant) => `/brand/gamepadlab-${variant}.svg`;
