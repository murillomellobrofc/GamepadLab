import { useRef } from 'react';
import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import { live, useAnimationFrame } from '@/state/live';
import css from './TriggerBar.module.css';

/** 0–100 % bar with the raw byte/value, updated every animation frame. */
export function TriggerBar({
  id,
  side,
  label,
}: {
  id: string;
  side: 'left' | 'right';
  label: string;
}) {
  const t = useT();
  const fill = useRef<HTMLDivElement>(null);
  const pct = useRef<HTMLOutputElement>(null);
  const raw = useRef<HTMLSpanElement>(null);
  const calibrated = useApp((s) => s.viewMode === 'calibrated');

  useAnimationFrame(() => {
    const e = live.get(id);
    if (!e || !fill.current) return;
    const c = calibrated ? e.calibrated : null;
    const v = c
      ? side === 'left'
        ? c.leftTrigger
        : c.rightTrigger
      : side === 'left'
        ? e.snapshot.leftTrigger
        : e.snapshot.rightTrigger;
    fill.current.style.transform = `scaleY(${v})`;
    if (pct.current) pct.current.textContent = `${(v * 100).toFixed(1)}%`;
    const axis = e.snapshot.rawAxes.find((a) => a.label === (side === 'left' ? 'L2' : 'R2'));
    if (raw.current)
      raw.current.textContent =
        axis?.raw != null
          ? String(axis.raw)
          : (side === 'left' ? e.snapshot.leftTrigger : e.snapshot.rightTrigger).toFixed(3);
  });

  return (
    <div className={css.wrap}>
      <div
        className={css.track}
        role="meter"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div ref={fill} className={css.fill} data-cal={calibrated || undefined} />
        <div className={css.ticks} aria-hidden="true">
          {[0, 25, 50, 75, 100].map((n) => (
            <span key={n} style={{ bottom: `${n}%` }} />
          ))}
        </div>
      </div>
      <div className={css.meta}>
        <span className={css.label}>{label}</span>
        <output ref={pct} className={`mono ${css.pct}`}>
          0%
        </output>
        <span className={css.raw}>
          {t('test.raw')}{' '}
          <span ref={raw} className="mono">
            0
          </span>
        </span>
      </div>
    </div>
  );
}
