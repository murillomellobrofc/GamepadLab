import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import { useLive } from '@/state/live';
import ui from '@/styles/ui.module.css';
import { ControllerDiagram } from '../test/ControllerDiagram';
import css from './MultiView.module.css';

function Hz({ id }: { id: string }) {
  const hz = useLive(id, (e) => Math.round(e.stats.hz), 3);
  return <span className="mono">{hz ? `${hz} Hz` : '—'}</span>;
}

/** Every connected controller live at the same time (up to 8). */
export function MultiView() {
  const t = useT();
  const controllers = useApp((s) => s.controllers);
  const select = useApp((s) => s.select);
  return (
    <section aria-label={t('test.multi.allTitle')}>
      <h1 className={css.title}>{t('test.multi.allTitle')}</h1>
      <div className={css.grid}>
        {controllers.map((c, i) => (
          <article key={c.id} className={ui.panel}>
            <div className={ui.panelHead}>
              <span className={`${ui.chip} ${ui.chipInfo}`}>P{i + 1}</span>
              <h2 className={css.name}>{c.name}</h2>
              <span className={ui.spacer} />
              <Hz id={c.id} />
            </div>
            <ControllerDiagram controller={c} />
            <button type="button" className={`${ui.btn} ${ui.small}`} onClick={() => select(c.id)}>
              {t('test.multi.open')}
            </button>
          </article>
        ))}
      </div>
    </section>
  );
}
