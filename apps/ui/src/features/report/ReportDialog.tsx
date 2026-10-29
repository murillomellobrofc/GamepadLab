import { useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { errorMessage } from '@/api/client';
import type { HistorySession } from '@/api/types';
import { Field, Segmented } from '@/app/controls';
import { Dialog } from '@/app/Dialog';
import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import { exportReport, type ReportFormat } from './export';
import { ReportSheet } from './ReportSheet';

export function ReportDialog({
  open,
  sessions,
  onClose,
}: {
  open: boolean;
  sessions: HistorySession[];
  onClose: () => void;
}) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const version = useApp((s) => s.appInfo?.version ?? '0.1.0');
  const [format, setFormat] = useState<ReportFormat>('pdf');
  const [busy, setBusy] = useState(false);
  const sheet = useRef<HTMLDivElement>(null);

  const generate = async () => {
    setBusy(true);
    try {
      if (await exportReport(format, sessions, sheet.current, version)) {
        toast({ tone: 'ok', title: t('report.saved') });
        onClose();
      }
    } catch (e) {
      toast({ tone: 'bad', title: t('app.toast.error'), body: errorMessage(e) });
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <Dialog open={open} title={t('report.dialogTitle')} onClose={onClose}>
        <p className={ui.muted}>{t('report.contents')}</p>
        <Field label={t('report.format')}>
          <Segmented<ReportFormat>
            label={t('report.format')}
            value={format}
            onChange={setFormat}
            options={[
              { value: 'pdf', label: t('report.pdf') },
              { value: 'png', label: t('report.png') },
              { value: 'json', label: t('report.json') },
            ]}
          />
        </Field>
        <p className={ui.faint}>{t('history.selected', { n: sessions.length })}</p>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
          <button type="button" className={ui.btn} onClick={onClose}>
            {t('app.common.cancel')}
          </button>
          <button
            type="button"
            className={`${ui.btn} ${ui.primary}`}
            onClick={generate}
            disabled={busy}
            data-testid="report-generate"
          >
            {busy ? t('report.generating') : t('report.generate')}
          </button>
        </div>
      </Dialog>
      {open &&
        createPortal(
          <div
            style={{ position: 'fixed', left: -10000, top: 0, pointerEvents: 'none' }}
            aria-hidden="true"
          >
            <ReportSheet ref={sheet} sessions={sessions} version={version} />
          </div>,
          document.body,
        )}
    </>
  );
}
