import { api } from '@/api/client';
import { useT, type TKey } from '@/i18n';
import { detectOs, OS_LABEL } from '@/lib/platform';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import css from './EmptyState.module.css';
import { Icon } from './icons';

const TIPS: Record<string, TKey[]> = {
  macos: ['app.empty.mac1', 'app.empty.mac2', 'app.empty.mac3'],
  windows: ['app.empty.win1', 'app.empty.win2', 'app.empty.win3'],
  linux: ['app.empty.linux1', 'app.empty.linux2', 'app.empty.linux3'],
  other: ['app.empty.other1'],
};

export function EmptyState() {
  const t = useT();
  const info = useApp((s) => s.appInfo);
  const os = detectOs(info);
  return (
    <div className={css.wrap} data-testid="empty-state">
      <div className={css.scope} aria-hidden="true">
        <svg viewBox="0 0 200 200">
          <circle cx="100" cy="100" r="78" className={css.ring} />
          <circle cx="100" cy="100" r="40" className={css.ringInner} />
          <line x1="100" y1="12" x2="100" y2="188" className={css.axis} />
          <line x1="12" y1="100" x2="188" y2="100" className={css.axis} />
          <circle cx="100" cy="100" r="7" className={css.dot} />
          <circle cx="100" cy="100" r="7" className={css.ping} />
        </svg>
      </div>
      <h1 className={css.title}>{t('app.empty.title')}</h1>
      <p className={css.body}>{t('app.empty.body')}</p>
      <div className={css.actions}>
        <button
          type="button"
          className={`${ui.btn} ${ui.primary}`}
          onClick={() => void api.setDemo(true)}
          data-testid="empty-demo"
        >
          <Icon name="play" size={16} /> {t('app.empty.demoCta')}
        </button>
      </div>
      <p className={css.hint}>{t('app.empty.demoHint')}</p>
      <section className={`${ui.panel} ${css.tips}`} aria-labelledby="os-tips">
        <div className={ui.panelHead}>
          <h2 id="os-tips" className={ui.panelTitle}>
            {t('app.empty.osTitle', { os: OS_LABEL[os] })}
          </h2>
        </div>
        <ol>
          {TIPS[os].map((k) => (
            <li key={k}>{t(k)}</li>
          ))}
        </ol>
      </section>
    </div>
  );
}
