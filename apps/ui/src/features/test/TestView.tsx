import type { ControllerInfo } from '@/api/types';
import { useT } from '@/i18n';
import ui from '@/styles/ui.module.css';
import { ControllerHeader } from '../common/ControllerHeader';
import { ControllerDiagram } from './ControllerDiagram';
import { StickScope } from './StickScope';
import { TriggerBar } from './TriggerBar';
import css from './TestView.module.css';

export function LiveOverview({ c }: { c: ControllerInfo }) {
  const t = useT();
  return (
    <div className={css.overview}>
      <section className={`${ui.panel} ${css.diagram}`} aria-label={t('test.diagram')}>
        <div className={ui.panelHead}>
          <h2 className={ui.panelTitle}>{t('test.diagram')}</h2>
          <span className={ui.channel}>CH·00</span>
        </div>
        <ControllerDiagram controller={c} />
      </section>
      <section className={`${ui.panel} ${css.sticks}`} aria-label={t('test.sticks')}>
        <div className={ui.panelHead}>
          <h2 className={ui.panelTitle}>{t('test.sticks')}</h2>
          <span className={ui.channel}>CH·01–02</span>
        </div>
        <div className={css.stickRow}>
          <StickScope id={c.id} side="left" />
          <StickScope id={c.id} side="right" />
        </div>
      </section>
      <section className={`${ui.panel} ${css.triggers}`} aria-label={t('test.triggers')}>
        <div className={ui.panelHead}>
          <h2 className={ui.panelTitle}>{t('test.triggers')}</h2>
          <span className={ui.channel}>CH·03–04</span>
        </div>
        <div className={css.triggerRow}>
          <TriggerBar
            id={c.id}
            side="left"
            label={c.kind === 'xbox' ? 'LT' : c.kind === 'switchPro' ? 'ZL' : 'L2'}
          />
          <TriggerBar
            id={c.id}
            side="right"
            label={c.kind === 'xbox' ? 'RT' : c.kind === 'switchPro' ? 'ZR' : 'R2'}
          />
        </div>
      </section>
    </div>
  );
}

export function TestView({ c }: { c: ControllerInfo }) {
  return (
    <>
      <ControllerHeader c={c} />
      <LiveOverview c={c} />
    </>
  );
}
