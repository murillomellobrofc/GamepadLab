import { useEffect } from 'react';
import { TABS, useApp } from '@/state/app';
import { useSettings } from '@/state/settings';

/** Global keyboard shortcuts: Ctrl/⌘+1…6 tabs, Ctrl/⌘+. settings, Ctrl/⌘+D raw panel. */
export function useShortcuts() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const n = Number(e.key);
      if (n >= 1 && n <= TABS.length) {
        e.preventDefault();
        useApp.getState().setTab(TABS[n - 1]);
      } else if (e.key === '.') {
        e.preventDefault();
        useApp.getState().openDialog('settings');
      } else if (e.key.toLowerCase() === 'd') {
        e.preventDefault();
        const s = useSettings.getState();
        s.set({ showRawPanel: !s.showRawPanel });
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}
