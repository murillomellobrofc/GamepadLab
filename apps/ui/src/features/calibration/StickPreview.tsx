import type { StickCalibration } from '@/api/types';
import { useT } from '@/i18n';
import { applyStick, gateRadius } from '@/lib/calibration';
import { begin, graticule, useHiDpiCanvas, usePalette } from '@/lib/canvas';
import { live, useAnimationFrame } from '@/state/live';
import css from './Adjustments.module.css';

/** Live raw (blue) and calibrated (mint) position with deadzones and gate. */
export function StickPreview({
  id,
  side,
  cal,
}: {
  id: string;
  side: 'left' | 'right';
  cal: StickCalibration;
}) {
  const t = useT();
  const { ref } = useHiDpiCanvas();
  const p = usePalette();
  useAnimationFrame(() => {
    const canvas = ref.current;
    const e = live.get(id);
    if (!canvas) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const w = canvas.getBoundingClientRect().width;
    const c = w / 2;
    const r = w / 2 - 10;
    ctx.clearRect(0, 0, w, w);
    graticule(ctx, c, c, r, p);
    const cx = c + cal.center.x * r;
    const cy = c - cal.center.y * r;
    // Inner deadzone (in raw units around the corrected centre).
    ctx.fillStyle = p.warn;
    ctx.globalAlpha = 0.14;
    ctx.beginPath();
    if (cal.deadzoneMode === 'radial') ctx.arc(cx, cy, cal.innerDeadzone * r, 0, Math.PI * 2);
    else {
      ctx.rect(cx - cal.innerDeadzone * r, c - r, cal.innerDeadzone * 2 * r, 2 * r);
      ctx.rect(c - r, cy - cal.innerDeadzone * r, 2 * r, cal.innerDeadzone * 2 * r);
    }
    ctx.fill();
    ctx.globalAlpha = 1;
    // Outer deadzone ring.
    if (cal.outerDeadzone > 0) {
      ctx.strokeStyle = p.warn;
      ctx.setLineDash([4, 4]);
      ctx.beginPath();
      ctx.arc(cx, cy, (1 - cal.outerDeadzone) * r, 0, Math.PI * 2);
      ctx.stroke();
      ctx.setLineDash([]);
    }
    // Gate outline (output space).
    if (cal.gate !== 'none') {
      ctx.strokeStyle = p.cal;
      ctx.globalAlpha = 0.6;
      ctx.beginPath();
      for (let i = 0; i <= 64; i++) {
        const a = (i / 64) * Math.PI * 2;
        const gr =
          cal.gate === 'square'
            ? 1 / Math.max(Math.abs(Math.cos(a)), Math.abs(Math.sin(a)))
            : gateRadius(cal.gate, a);
        const x = c + Math.cos(a) * gr * r;
        const y = c - Math.sin(a) * gr * r;
        if (i) ctx.lineTo(x, y);
        else ctx.moveTo(x, y);
      }
      ctx.stroke();
      ctx.globalAlpha = 1;
    }
    if (!e) return;
    const raw = side === 'left' ? e.snapshot.leftStick : e.snapshot.rightStick;
    const out = applyStick(cal, raw);
    ctx.strokeStyle = p.raw;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(c + raw.x * r, c - raw.y * r, 6, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = p.cal;
    ctx.shadowColor = p.cal;
    ctx.shadowBlur = 10;
    ctx.beginPath();
    ctx.arc(c + out.x * r, c - out.y * r, 6, 0, Math.PI * 2);
    ctx.fill();
    ctx.shadowBlur = 0;
  });
  return (
    <figure className={css.preview}>
      <canvas ref={ref} role="img" aria-label={t('calibration.adjust.preview')} />
      <figcaption>
        <span>
          <i style={{ borderColor: 'var(--sig-raw)' }} /> {t('calibration.adjust.rawDot')}
        </span>
        <span>
          <i style={{ background: 'var(--sig-cal)', borderColor: 'var(--sig-cal)' }} />{' '}
          {t('calibration.adjust.calDot')}
        </span>
      </figcaption>
    </figure>
  );
}
