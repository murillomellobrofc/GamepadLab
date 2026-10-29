import { api } from '@/api/client';
import type { FrameEntry } from '@/api/types';
import { useT } from '@/i18n';
import { useApp, useSelectedController } from '@/state/app';
import { useLive } from '@/state/live';
import { useSettings } from '@/state/settings';
import ui from '@/styles/ui.module.css';
import css from './RawPanel.module.css';
import { Icon } from './icons';

function fmt(v: number) {
  return (v < 0 ? '' : '+') + v.toFixed(3);
}

function Body({ id }: { id: string }) {
  const t = useT();
  const entry = useLive<FrameEntry | undefined>(
    id,
    (e) => e,
    15,
    () => false,
  );
  if (!entry) return <p className={css.empty}>{t('test.waiting')}</p>;
  const { snapshot: s, stats } = entry;
  return (
    <>
      <dl className={css.report}>
        <div>
          <dt>{t('raw.seq')}</dt>
          <dd className="mono">{s.seq}</dd>
        </div>
        <div>
          <dt>{t('raw.timestamp')}</dt>
          <dd className="mono">{(s.timestampUs / 1e6).toFixed(4)}s</dd>
        </div>
        <div>
          <dt>{t('raw.rate')}</dt>
          <dd className="mono">{stats.hz ? `${stats.hz.toFixed(0)} Hz` : '—'}</dd>
        </div>
      </dl>
      <h3 className={css.sub}>
        {t('raw.axes')} <span>{s.rawAxes.length}</span>
      </h3>
      <table className={ui.table} aria-label={t('raw.axes')}>
        <thead>
          <tr>
            <th>#</th>
            <th>{t('raw.name')}</th>
            <th>{t('raw.value')}</th>
            <th>{t('raw.rawCol')}</th>
            <th>{t('raw.min')}</th>
            <th>{t('raw.max')}</th>
          </tr>
        </thead>
        <tbody>
          {s.rawAxes.map((a, i) => (
            <tr key={a.code}>
              <td className={css.idx}>{i}</td>
              <td className={css.name} title={`code ${a.code}`}>
                {a.label}
              </td>
              <td>
                <span className={css.val}>{fmt(a.value)}</span>
                <span
                  className={css.bar}
                  style={{ ['--v' as string]: a.value }}
                  aria-hidden="true"
                />
              </td>
              <td className={css.dim}>{a.raw ?? '—'}</td>
              <td className={css.dim}>{fmt(a.min)}</td>
              <td className={css.dim}>{fmt(a.max)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <h3 className={css.sub}>
        {t('raw.buttons')} <span>{s.rawButtons.length}</span>
      </h3>
      <table className={ui.table} aria-label={t('raw.buttons')}>
        <thead>
          <tr>
            <th>#</th>
            <th>{t('raw.name')}</th>
            <th>{t('raw.value')}</th>
            <th>{t('raw.max')}</th>
            <th>{t('raw.presses')}</th>
          </tr>
        </thead>
        <tbody>
          {s.rawButtons.map((b, i) => (
            <tr key={b.code} data-on={b.pressed || undefined} className={css.btnRow}>
              <td className={css.idx}>{i}</td>
              <td className={css.name} title={`code ${b.code}`}>
                {b.label}
              </td>
              <td>{b.value.toFixed(2)}</td>
              <td className={css.dim}>{b.maxValue.toFixed(2)}</td>
              <td className={css.dim}>{b.pressCount}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
}

export function RawPanel() {
  const t = useT();
  const c = useSelectedController();
  const toast = useApp((s) => s.toast);
  const close = () => useSettings.getState().set({ showRawPanel: false });
  return (
    <aside
      className={css.panel}
      aria-label={t('app.topbar.rawPanel')}
      onKeyDown={(e) => e.key === 'Escape' && close()}
    >
      <div className={css.head}>
        <h2 className={ui.panelTitle}>{t('app.topbar.rawPanel')}</h2>
        {c && (
          <button
            type="button"
            className={`${ui.btn} ${ui.small}`}
            onClick={async () => {
              await api.resetMinMax(c.id);
              toast({ tone: 'info', title: t('raw.resetDone') });
            }}
            data-testid="raw-reset"
          >
            <Icon name="refresh" size={14} /> {t('raw.reset')}
          </button>
        )}
        <button
          type="button"
          className={`${ui.btn} ${ui.ghost} ${ui.iconBtn} ${ui.small}`}
          onClick={close}
          aria-label={t('app.topbar.hideRawPanel')}
        >
          <Icon name="x" size={14} />
        </button>
      </div>
      <div className={css.scroll}>
        {c ? <Body id={c.id} /> : <p className={css.empty}>{t('app.common.selectController')}</p>}
      </div>
    </aside>
  );
}
