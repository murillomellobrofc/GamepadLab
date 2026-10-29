import type { ReactNode } from 'react';
import { api } from '@/api/client';
import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import { useSettings } from '@/state/settings';
import ui from '@/styles/ui.module.css';
import { AboutDialog } from './AboutDialog';
import { Announcer } from './Announcer';
import css from './AppShell.module.css';
import { Icon } from './icons';
import { RawPanel } from './RawPanel';
import { SettingsDialog } from './SettingsDialog';
import { Sidebar } from './Sidebar';
import { Toasts } from './Toasts';
import { useGamepadNav } from './useGamepadNav';
import { TopBar } from './TopBar';

export function AppShell({ children }: { children: ReactNode }) {
  const t = useT();
  const rawOpen = useSettings((s) => s.showRawPanel);
  const tab = useApp((s) => s.tab);
  const transportKind = useApp((s) => s.transportKind);
  const warnings = useApp((s) => s.warnings);
  const gpNav = useApp((s) => s.gamepadNav);
  const navLocks = useApp((s) => s.navLocks);
  useGamepadNav();
  return (
    <div className={css.shell} data-raw={rawOpen ? 'open' : 'closed'}>
      <a href="#main-panel" className="skip-link">
        {t('app.skipToContent')}
      </a>
      <Sidebar />
      <main className={css.main}>
        <TopBar />
        <div
          id="main-panel"
          className={css.content}
          role="tabpanel"
          aria-labelledby={`tab-${tab}`}
          tabIndex={-1}
        >
          {transportKind === 'mock' && (
            <div className={`${ui.callout} ${ui.calloutInfo} ${css.banner}`} role="note">
              <Icon name="info" />
              <span>{t('app.browserMode')}</span>
            </div>
          )}
          {warnings.map((w) => (
            <div
              key={w.code}
              className={`${ui.callout} ${ui.calloutWarn} ${css.banner}`}
              role="alert"
            >
              <Icon name="alert" />
              <span style={{ flex: 1 }}>{t(`errors.warnings.${w.code}`)}</span>
              {w.code === 'inputMonitoring' && (
                <button
                  type="button"
                  className={`${ui.btn} ${ui.small}`}
                  onClick={() => void api.openPrivacySettings()}
                >
                  {t('errors.warnings.openSettings')}
                </button>
              )}
            </div>
          ))}
          {children}
        </div>
      </main>
      {rawOpen ? <RawPanel /> : <span />}
      {gpNav && (
        <div className={css.gpBadge} role="status" data-paused={navLocks > 0 || undefined}>
          <Icon name="gamepad" size={14} />
          {navLocks > 0 ? t('a11y.gamepadNavPaused') : t('a11y.gamepadNavHint')}
        </div>
      )}
      <Toasts />
      <Announcer />
      <SettingsDialog />
      <AboutDialog />
    </div>
  );
}
