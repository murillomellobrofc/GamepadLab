import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import { Dialog } from './Dialog';
import { Icon } from './icons';

export function AboutDialog() {
  const t = useT();
  const open = useApp((s) => s.dialog === 'about');
  const openDialog = useApp((s) => s.openDialog);
  const info = useApp((s) => s.appInfo);
  return (
    <Dialog open={open} title={t('app.about.title')} onClose={() => openDialog(null)}>
      <div style={{ display: 'flex', gap: 16, alignItems: 'center' }}>
        <img src="/favicon.svg" alt="" width={56} height={56} />
        <div>
          <p style={{ fontSize: 'var(--fs-xl)', fontWeight: 700 }}>GamepadLab</p>
          <p className={`${ui.muted} mono`}>
            {t('app.about.version', { version: info?.version ?? '0.1.0' })}
          </p>
        </div>
      </div>
      <p>{t('app.about.description')}</p>
      <div className={`${ui.callout} ${ui.calloutInfo}`} role="note">
        <Icon name="shield" />
        <div>
          <strong>{t('app.about.privacyTitle')}</strong>
          <p>{t('app.about.privacy')}</p>
          <p className={ui.muted} style={{ marginTop: 4 }}>
            {t('app.about.network')}
          </p>
        </div>
      </div>
      <dl style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '6px 16px', margin: 0 }}>
        <dt className={ui.faint}>{t('app.about.platform')}</dt>
        <dd className="mono" style={{ margin: 0 }}>
          {info ? `${info.os} ${info.osVersion} (${info.arch})` : '—'}
        </dd>
        <dt className={ui.faint}>{t('app.about.backends')}</dt>
        <dd className="mono" style={{ margin: 0 }}>
          {info?.backends.join(', ') ?? '—'}
        </dd>
        <dt className={ui.faint}>{t('app.about.dataDir')}</dt>
        <dd className="mono" style={{ margin: 0, wordBreak: 'break-all', userSelect: 'text' }}>
          {info?.dataDir ?? '—'}
        </dd>
      </dl>
      <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
        {t('app.about.credits')} · {t('app.about.license')}
      </p>
    </Dialog>
  );
}
