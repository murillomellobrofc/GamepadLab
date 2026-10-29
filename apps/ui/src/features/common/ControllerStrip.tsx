import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import css from './ControllerStrip.module.css';

/** Tab strip to switch between connected controllers (shown with 2+). */
export function ControllerStrip() {
  const t = useT();
  const controllers = useApp((s) => s.controllers);
  const selectedId = useApp((s) => s.selectedId);
  const multi = useApp((s) => s.multiView);
  const select = useApp((s) => s.select);
  const setMulti = useApp((s) => s.setMultiView);
  if (controllers.length < 2) return null;
  return (
    <div className={css.strip} role="tablist" aria-label={t('test.multi.tabsLabel')}>
      <button
        type="button"
        role="tab"
        aria-selected={multi}
        className={css.tab}
        onClick={() => setMulti(true)}
      >
        {t('test.multi.all')} <span className="mono">{controllers.length}</span>
      </button>
      {controllers.map((c, i) => (
        <button
          key={c.id}
          type="button"
          role="tab"
          aria-selected={!multi && c.id === selectedId}
          className={css.tab}
          onClick={() => select(c.id)}
        >
          <span className="mono">P{i + 1}</span> {c.name}
        </button>
      ))}
    </div>
  );
}
