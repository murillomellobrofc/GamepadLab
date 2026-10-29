import { useRef } from 'react';
import type { Vec2 } from '@/api/types';
import { useT } from '@/i18n';
import { begin, graticule, useHiDpiCanvas, usePalette } from '@/lib/canvas';
import { signed } from '@/lib/format';
import { useApp } from '@/state/app';
import { live, useAnimationFrame, useLive } from '@/state/live';
import css from './StickScope.module.css';

const TRAIL = 90;

function read(
  id: string,
  side: 'left' | 'right',
  calibrated: boolean,
): { raw: Vec2; cal: Vec2 | null } | null {
  const e = live.get(id);
  if (!e) return null;
  const raw = side === 'left' ? e.snapshot.leftStick : e.snapshot.rightStick;
  const c = e.calibrated;
  const cal = calibrated && c ? (side === 'left' ? c.leftStick : c.rightStick) : null;
  return { raw, cal };
}

/** Live stick view: graticule, unit circle, trail, dot and readouts. */
export function StickScope({
  id,
  side,
  size = 220,
  showTrail = true,
}: {
  id: string;
  side: 'left' | 'right';
  size?: number;
  showTrail?: boolean;
}) {
  const t = useT();
  const { ref } = useHiDpiCanvas();
  const p = usePalette();
  const calibrated = useApp((s) => s.viewMode === 'calibrated');
  const trail = useRef<Vec2[]>([]);

  useAnimationFrame(() => {
    const canvas = ref.current;
    const v = read(id, side, calibrated);
    if (!canvas || !v) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const w = canvas.getBoundingClientRect().width;
    const r = w / 2 - 10;
    const c = w / 2;
    ctx.clearRect(0, 0, w, w);
    graticule(ctx, c, c, r, p);
    const shown = v.cal ?? v.raw;
    trail.current.push(shown);
    if (trail.current.length > TRAIL) trail.current.shift();
    if (showTrail) {
      ctx.lineWidth = 2;
      ctx.lineCap = 'round';
      for (let i = 1; i < trail.current.length; i++) {
        const a = trail.current[i - 1];
        const b = trail.current[i];
        ctx.globalAlpha = (i / trail.current.length) * 0.8;
        ctx.strokeStyle = v.cal ? p.cal : p.raw;
        ctx.beginPath();
        ctx.moveTo(c + a.x * r, c - a.y * r);
        ctx.lineTo(c + b.x * r, c - b.y * r);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
    }
    if (v.cal) {
      // Raw position as a hollow marker so both are visible.
      ctx.strokeStyle = p.raw;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.arc(c + v.raw.x * r, c - v.raw.y * r, 5, 0, Math.PI * 2);
      ctx.stroke();
    }
    ctx.fillStyle = v.cal ? p.cal : p.raw;
    ctx.shadowColor = ctx.fillStyle;
    ctx.shadowBlur = 12;
    ctx.beginPath();
    ctx.arc(c + shown.x * r, c - shown.y * r, 6, 0, Math.PI * 2);
    ctx.fill();
    ctx.shadowBlur = 0;
  });

  const values = useLive(
    id,
    (e) => {
      const c = calibrated && e.calibrated ? e.calibrated : null;
      const s = c
        ? side === 'left'
          ? c.leftStick
          : c.rightStick
        : side === 'left'
          ? e.snapshot.leftStick
          : e.snapshot.rightStick;
      return `${s.x.toFixed(3)}|${s.y.toFixed(3)}`;
    },
    12,
  );
  const [x, y] = (values ?? '0|0').split('|').map(Number);
  const mag = Math.hypot(x, y);
  let ang = (Math.atan2(y, x) * 180) / Math.PI;
  if (ang < 0) ang += 360;
  const label = side === 'left' ? t('test.leftStick') : t('test.rightStick');

  return (
    <figure className={css.scope} style={{ ['--size' as string]: `${size}px` }}>
      <canvas ref={ref} className={css.canvas} role="img" aria-label={label} />
      <figcaption className={css.readout}>
        <span className={css.title}>{label}</span>
        <dl>
          <div>
            <dt>X</dt>
            <dd className="mono">{signed(x)}</dd>
          </div>
          <div>
            <dt>Y</dt>
            <dd className="mono">{signed(y)}</dd>
          </div>
          <div>
            <dt>{t('test.angle')}</dt>
            <dd className="mono">{mag > 0.05 ? `${ang.toFixed(1)}°` : '—'}</dd>
          </div>
          <div>
            <dt>{t('test.magnitude')}</dt>
            <dd className="mono">{mag.toFixed(3)}</dd>
          </div>
        </dl>
      </figcaption>
    </figure>
  );
}
