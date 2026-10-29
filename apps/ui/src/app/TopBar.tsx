import { useRef } from 'react';
import { useT, type TKey } from '@/i18n';
import { TABS, useApp, type TabId } from '@/state/app';
import { useSettings } from '@/state/settings';
import ui from '@/styles/ui.module.css';
import css from './AppShell.module.css';
import { Segmented } from './controls';
import { Icon, type IconName } from './icons';

const TAB_ICONS: Record<TabId, IconName> = {
  test: 'gamepad',
  calibration: 'target',
  latency: 'activity',
  sensors: 'compass',
  history: 'history',
  overlay: 'cast',
};

export function TopBar() {
  const t = useT();
  const tab = useApp((s) => s.tab);
  const setTab = useApp((s) => s.setTab);
  const viewMode = useApp((s) => s.viewMode);
  const setViewMode = useApp((s) => s.setViewMode);
  const rawOpen = useSettings((s) => s.showRawPanel);
  const setSettings = useSettings((s) => s.set);
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});

  const onKey = (e: React.KeyboardEvent) => {
    const idx = TABS.indexOf(tab);
    let next: TabId | null = null;
    if (e.key === 'ArrowRight') next = TABS[(idx + 1) % TABS.length];
    if (e.key === 'ArrowLeft') next = TABS[(idx - 1 + TABS.length) % TABS.length];
    if (e.key === 'Home') next = TABS[0];
    if (e.key === 'End') next = TABS[TABS.length - 1];
    if (next) {
      e.preventDefault();
      setTab(next);
      refs.current[next]?.focus();
    }
  };

  return (
    <header className={css.topbar}>
      <div className={css.tabs} role="tablist" aria-label={t('app.tabs.label')} onKeyDown={onKey}>
        {TABS.map((id, i) => (
          <button
            key={id}
            ref={(el) => {
              refs.current[id] = el;
            }}
            type="button"
            role="tab"
            id={`tab-${id}`}
            aria-selected={tab === id}
            aria-controls="main-panel"
            tabIndex={tab === id ? 0 : -1}
            className={css.tab}
            onClick={() => setTab(id)}
            data-testid={`tab-${id}`}
          >
            <Icon name={TAB_ICONS[id]} size={16} />
            {t(`app.tabs.${id}` as TKey)}
            <kbd aria-hidden="true">{i + 1}</kbd>
          </button>
        ))}
      </div>
      <div className={css.topActions}>
        <Segmented
          label={t('app.topbar.view')}
          value={viewMode}
          onChange={setViewMode}
          options={[
            { value: 'raw', label: t('app.topbar.raw') },
            { value: 'calibrated', label: t('app.topbar.calibrated') },
          ]}
        />
        <button
          type="button"
          className={`${ui.btn} ${ui.ghost} ${ui.iconBtn}`}
          aria-pressed={rawOpen}
          aria-label={rawOpen ? t('app.topbar.hideRawPanel') : t('app.topbar.showRawPanel')}
          title={rawOpen ? t('app.topbar.hideRawPanel') : t('app.topbar.showRawPanel')}
          onClick={() => setSettings({ showRawPanel: !rawOpen })}
        >
          <Icon name="panel" />
        </button>
      </div>
    </header>
  );
}
