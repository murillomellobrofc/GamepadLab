import { useT } from '@/i18n';
import ui from '@/styles/ui.module.css';
import { Dialog } from './Dialog';

export function ConfirmDialog({
  open,
  title,
  body,
  confirmLabel,
  danger,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  body: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const t = useT();
  return (
    <Dialog open={open} title={title} onClose={onCancel}>
      <p>{body}</p>
      <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
        <button type="button" className={ui.btn} onClick={onCancel} autoFocus>
          {t('app.common.cancel')}
        </button>
        <button
          type="button"
          className={`${ui.btn} ${danger ? ui.danger : ui.primary}`}
          onClick={onConfirm}
          data-testid="confirm-ok"
        >
          {confirmLabel ?? t('app.common.confirm')}
        </button>
      </div>
    </Dialog>
  );
}
