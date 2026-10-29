import { useEffect } from 'react';
import { prefersReducedMotion, useSettings } from '@/state/settings';

/** Mirrors theme, contrast, text size, motion and language onto <html>. */
export function useDocumentSettings() {
  const { theme, highContrast, textScale, motion, locale } = useSettings();

  useEffect(() => {
    const root = document.documentElement;
    const apply = () => {
      const dark =
        theme === 'dark' ||
        (theme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches);
      root.dataset.theme = dark ? 'dark' : 'light';
    };
    apply();
    if (theme !== 'auto') return;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  }, [theme]);

  useEffect(() => {
    const root = document.documentElement;
    if (highContrast) root.dataset.contrast = 'high';
    else delete root.dataset.contrast;
  }, [highContrast]);

  useEffect(() => {
    document.documentElement.style.setProperty('--text-scale', String(textScale));
  }, [textScale]);

  useEffect(() => {
    const root = document.documentElement;
    const apply = () => {
      root.dataset.motion = prefersReducedMotion(motion) ? 'reduced' : 'full';
    };
    apply();
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    mq.addEventListener('change', apply);
    return () => mq.removeEventListener('change', apply);
  }, [motion]);

  useEffect(() => {
    document.documentElement.lang = locale;
  }, [locale]);
}

export function isReducedMotion(): boolean {
  return document.documentElement.dataset.motion === 'reduced';
}
