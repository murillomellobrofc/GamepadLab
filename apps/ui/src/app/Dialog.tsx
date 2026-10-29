import { useEffect, useRef, type ReactNode } from 'react';
import { Icon } from './icons';
import ui from '@/styles/ui.module.css';
import css from './Dialog.module.css';
import { useT } from '@/i18n';

/** Accessible modal built on <dialog> (native focus trap, Esc closes). */
export function Dialog({
  open,
  title,
  onClose,
  children,
  wide,
}: {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  const t = useT();
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (open && !el.open) {
      try {
        el.showModal();
      } catch {
        el.setAttribute('open', '');
      }
    } else if (!open && el.open) {
      el.close();
    }
  }, [open]);
  return (
    <dialog
      ref={ref}
      className={css.dialog}
      data-wide={wide || undefined}
      aria-labelledby="dlg-title"
      onClose={onClose}
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onClick={(e) => {
        if (e.target === ref.current) onClose();
      }}
    >
      {open && (
        <div className={css.body}>
          <header className={css.head}>
            <h2 id="dlg-title">{title}</h2>
            <button
              type="button"
              className={`${ui.btn} ${ui.ghost} ${ui.iconBtn}`}
              onClick={onClose}
              aria-label={t('app.common.close')}
            >
              <Icon name="x" />
            </button>
          </header>
          <div className={css.content}>{children}</div>
        </div>
      )}
    </dialog>
  );
}
