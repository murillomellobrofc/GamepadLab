import { useState } from 'react';
import type { ControllerInfo } from '@/api/types';
import { Segmented } from '@/app/controls';
import { useT } from '@/i18n';
import ui from '@/styles/ui.module.css';
import { ControllerHeader } from '../common/ControllerHeader';
import { ControllerDiagram } from './ControllerDiagram';
import { GhostingTest } from './GhostingTest';
import { GuidedButtonTest } from './GuidedButtonTest';
import { StickScope } from './StickScope';
import { TriggerBar } from './TriggerBar';
import css from './TestView.module.css';
import { VibrationPanel } from './VibrationPanel';

type Sub = 'overview' | 'guided' | 'vibration' | 'ghosting';

export function LiveOverview({ c }: { c: ControllerInfo }) {
  const t = useT();
  const trig = (side: 'L' | 'R') =>
    c.kind === 'xbox' ? `${side}T` : c.kind === 'switchPro' ? `Z${side}` : `${side}2`;
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
          <TriggerBar id={c.id} side="left" label={trig('L')} />
          <TriggerBar id={c.id} side="right" label={trig('R')} />
        </div>
      </section>
    </div>
  );
}

export function TestView({ c }: { c: ControllerInfo }) {
  const t = useT();
  const [sub, setSub] = useState<Sub>('overview');
  return (
    <>
      <ControllerHeader c={c} />
      <div className={css.subnav}>
        <Segmented<Sub>
          label={t('test.tabs.label')}
          value={sub}
          onChange={setSub}
          options={[
            { value: 'overview', label: t('test.tabs.overview') },
            { value: 'guided', label: t('test.tabs.guided') },
            { value: 'vibration', label: t('test.tabs.vibration') },
            { value: 'ghosting', label: t('test.tabs.ghosting') },
          ]}
        />
      </div>
      {sub === 'overview' && <LiveOverview c={c} />}
      {sub === 'guided' && <GuidedButtonTest c={c} />}
      {sub === 'vibration' && <VibrationPanel c={c} />}
      {sub === 'ghosting' && <GhostingTest c={c} />}
    </>
  );
}
