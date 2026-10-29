import { useT } from '@/i18n';
import ui from '@/styles/ui.module.css';
import css from './AppShell.module.css';

export function RawPanel() {
  const t = useT();
  return (
    <aside className={css.rawpanel} aria-label={t('app.topbar.rawPanel')}>
      <div className={ui.panelHead} style={{ padding: '16px 16px 0' }}>
        <h2 className={ui.panelTitle}>{t('app.topbar.rawPanel')}</h2>
      </div>
      <p className={ui.faint} style={{ padding: 16, fontSize: 'var(--fs-sm)' }}>
        {t('app.common.selectController')}
      </p>
    </aside>
  );
}
