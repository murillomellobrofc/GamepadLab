import { Fragment, useCallback, useEffect, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type { HistorySession, SessionKind } from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { ConfirmDialog } from '@/app/ConfirmDialog';
import { Icon } from '@/app/icons';
import { formatDateTime, useT, type TKey } from '@/i18n';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import { ReportDialog } from '../report/ReportDialog';
import css from './HistoryView.module.css';

const KINDS: SessionKind[] = ['buttons', 'calibration', 'latency', 'ghosting', 'test', 'vibration'];

export function HistoryView() {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const [items, setItems] = useState<HistorySession[]>([]);
  const [filter, setFilter] = useState<SessionKind | 'all'>('all');
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [confirmClear, setConfirmClear] = useState(false);
  const [reportOpen, setReportOpen] = useState(false);
  const [open, setOpen] = useState<number | null>(null);

  const load = useCallback(async () => {
    try {
      setItems(await api.historyList(500));
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load]);

  const shown = items.filter((s) => filter === 'all' || s.kind === filter);
  const chosen = items.filter((s) => selected.has(s.id));
  const toggle = (id: number) =>
    setSelected((cur) => {
      const n = new Set(cur);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });

  return (
    <section className={ui.panel} aria-labelledby="history-title">
      <div className={ui.panelHead}>
        <h1 id="history-title" className={ui.panelTitle}>
          {t('history.title')}
        </h1>
        <span className={ui.spacer} />
        <button
          type="button"
          className={`${ui.btn} ${ui.small} ${ui.primary}`}
          disabled={!chosen.length}
          onClick={() => setReportOpen(true)}
          data-testid="history-report"
        >
          <Icon name="file" size={14} /> {t('history.report')}{' '}
          {chosen.length ? `(${chosen.length})` : ''}
        </button>
        <button
          type="button"
          className={`${ui.btn} ${ui.small} ${ui.danger}`}
          disabled={!items.length}
          onClick={() => setConfirmClear(true)}
        >
          <Icon name="trash" size={14} /> {t('history.clear')}
        </button>
      </div>
      <p className={css.local}>
        <Icon name="shield" size={13} /> {t('history.local')}
      </p>
      <div className={css.filters} role="radiogroup" aria-label={t('history.filter')}>
        {(['all', ...KINDS] as const).map((k) => (
          <button
            key={k}
            type="button"
            role="radio"
            aria-checked={filter === k}
            className={`${ui.btn} ${ui.small}`}
            onClick={() => setFilter(k)}
          >
            {k === 'all' ? t('history.all') : t(`history.kinds.${k}` as TKey)}
          </button>
        ))}
      </div>
      {shown.length === 0 ? (
        <p className={css.empty}>{t('history.empty')}</p>
      ) : (
        <table className={`${ui.table} ${css.table}`} data-testid="history-table">
          <thead>
            <tr>
              <th />
              <th>{t('history.date')}</th>
              <th>{t('history.kind')}</th>
              <th>{t('history.controller')}</th>
              <th>{t('history.score')}</th>
              <th>{t('history.summary')}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {shown.map((s) => (
              <Fragment key={s.id}>
                <tr data-selected={selected.has(s.id) || undefined}>
                  <td>
                    <input
                      type="checkbox"
                      checked={selected.has(s.id)}
                      onChange={() => toggle(s.id)}
                      aria-label={t('history.select', { id: s.id })}
                    />
                  </td>
                  <td>{formatDateTime(s.createdAt)}</td>
                  <td className={css.ui}>{t(`history.kinds.${s.kind}` as TKey)}</td>
                  <td className={css.ui}>{s.controllerName}</td>
                  <td>{s.score ?? '—'}</td>
                  <td className={css.ui}>{s.summary}</td>
                  <td className={css.actions}>
                    <button
                      type="button"
                      className={`${ui.btn} ${ui.ghost} ${ui.small}`}
                      aria-expanded={open === s.id}
                      onClick={() => setOpen(open === s.id ? null : s.id)}
                    >
                      {t('history.details')}
                    </button>
                    <button
                      type="button"
                      className={`${ui.btn} ${ui.ghost} ${ui.small} ${ui.iconBtn}`}
                      aria-label={t('history.delete')}
                      onClick={async () => {
                        await api
                          .historyDelete(s.id)
                          .catch((e) => toast(errorToast(errorCode(e), errorMessage(e))));
                        setSelected((cur) => {
                          const n = new Set(cur);
                          n.delete(s.id);
                          return n;
                        });
                        void load();
                      }}
                    >
                      <Icon name="trash" size={14} />
                    </button>
                  </td>
                </tr>
                {open === s.id && (
                  <tr className={css.detail}>
                    <td colSpan={7}>
                      <pre>
                        {JSON.stringify(
                          s.payload,
                          (k, v) =>
                            k === 'rotation' || k === 'intervalsUs'
                              ? `[${(v as unknown[]).length} items]`
                              : v,
                          2,
                        )}
                      </pre>
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
          </tbody>
        </table>
      )}
      <ConfirmDialog
        open={confirmClear}
        danger
        title={t('history.clear')}
        body={t('history.confirmClear')}
        onCancel={() => setConfirmClear(false)}
        onConfirm={async () => {
          setConfirmClear(false);
          await api.historyClear().catch((e) => toast(errorToast(errorCode(e), errorMessage(e))));
          setSelected(new Set());
          void load();
        }}
      />
      <ReportDialog open={reportOpen} sessions={chosen} onClose={() => setReportOpen(false)} />
    </section>
  );
}
