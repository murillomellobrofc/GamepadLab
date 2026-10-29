import { useRef } from 'react';
import type { ControllerInfo, FrameEntry } from '@/api/types';
import { useT } from '@/i18n';
import { live, useAnimationFrame, useLive } from '@/state/live';
import css from './GenericDiagram.module.css';

const shape = (e: FrameEntry) =>
  e.snapshot.rawButtons.map((b) => b.label).join('|') +
  '#' +
  e.snapshot.rawAxes.map((a) => a.label).join('|');

/**
 * Layout that builds itself from whatever buttons and axes the controller
 * reports: one cell per button, one bar per axis.
 */
export function GenericDiagram({
  controller,
}: {
  controller: ControllerInfo;
  calibrated: boolean;
}) {
  const t = useT();
  useLive(controller.id, shape, 2);
  const entry = live.get(controller.id);
  const root = useRef<HTMLDivElement>(null);

  useAnimationFrame(() => {
    const e = live.get(controller.id);
    const el = root.current;
    if (!e || !el) return;
    el.querySelectorAll<HTMLElement>('[data-rb]').forEach((cell) => {
      const b = e.snapshot.rawButtons[Number(cell.dataset.rb)];
      if (!b) return;
      cell.dataset.on = String(b.pressed);
      cell.style.setProperty('--v', String(b.value));
    });
    el.querySelectorAll<HTMLElement>('[data-ra]').forEach((bar) => {
      const a = e.snapshot.rawAxes[Number(bar.dataset.ra)];
      if (!a) return;
      bar.style.setProperty('--v', String(a.value));
      const out = bar.querySelector('output');
      if (out) out.textContent = a.value.toFixed(3);
    });
  });

  if (!entry) return <p className={css.empty}>{t('test.waiting')}</p>;
  const { rawButtons, rawAxes } = entry.snapshot;
  return (
    <div ref={root} className={css.wrap} role="group" aria-label={t('test.genericLayout')}>
      <div className={css.buttons}>
        {rawButtons.map((b, i) => (
          <div key={b.code} className={css.cell} data-rb={i} title={`code ${b.code}`}>
            <span>{b.label}</span>
          </div>
        ))}
      </div>
      <div className={css.axes}>
        {rawAxes.map((a, i) => (
          <div key={a.code} className={css.axis} data-ra={i}>
            <span className={css.axisLabel}>{a.label}</span>
            <div className={css.track}>
              <div className={css.fill} />
            </div>
            <output className="mono">0.000</output>
          </div>
        ))}
      </div>
    </div>
  );
}
