import { useApp } from '@/state/app';
import { Icon } from './icons';
import css from './Toasts.module.css';
import { useT } from '@/i18n';

export function Toasts() {
  const toasts = useApp((s) => s.toasts);
  const dismiss = useApp((s) => s.dismissToast);
  const t = useT();
  return (
    <div className={css.stack} role="region" aria-label="Notificações" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={css.toast} data-tone={toast.tone} role="status">
          <Icon
            name={toast.tone === 'ok' ? 'check' : toast.tone === 'info' ? 'info' : 'alert'}
            className={css.icon}
          />
          <div className={css.text}>
            <strong>{toast.title}</strong>
            {toast.body && <p>{toast.body}</p>}
            {toast.action && (
              <button
                type="button"
                className={css.action}
                onClick={() => {
                  toast.action?.run();
                  dismiss(toast.id);
                }}
              >
                {toast.action.label}
              </button>
            )}
          </div>
          <button
            type="button"
            className={css.close}
            onClick={() => dismiss(toast.id)}
            aria-label={t('app.common.close')}
          >
            <Icon name="x" size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
