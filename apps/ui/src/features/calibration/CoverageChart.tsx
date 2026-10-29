import { useEffect } from 'react';
import type { CaptureSample, CircularityAnalysis, StickCalibration, Vec2 } from '@/api/types';
import { useT } from '@/i18n';
import { applyStick } from '@/lib/calibration';
import { begin, graticule, useHiDpiCanvas, usePalette } from '@/lib/canvas';
import css from './CoverageChart.module.css';

const SCALE = 1.3;

/**
 * Circular coverage plot: recorded points, the ideal unit circle, measured
 * reach per sector, failing sectors highlighted and, optionally, the same
 * capture after calibration.
 */
export function CoverageChart({
  samples,
  center,
  before,
  after,
  calibration,
}: {
  samples: CaptureSample[];
  center: Vec2;
  before: CircularityAnalysis | null;
  after?: CircularityAnalysis | null;
  calibration?: StickCalibration | null;
}) {
  const t = useT();
  const { ref, size } = useHiDpiCanvas();
  const p = usePalette();

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas || !size.w) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const w = size.w;
    const c = w / 2;
    const r = (w / 2 - 8) / SCALE;
    ctx.clearRect(0, 0, w, w);
    graticule(ctx, c, c, r, p);
    const X = (x: number) => c + x * r;
    const Y = (y: number) => c - y * r;

    if (before) {
      const n = before.sectors.length;
      const wedge = (Math.PI * 2) / n;
      for (const s of before.sectors) {
        if (s.status === 'ok') continue;
        const a0 = s.index * wedge;
        ctx.fillStyle = s.status === 'missing' ? p.warn : p.bad;
        ctx.globalAlpha = 0.16;
        ctx.beginPath();
        ctx.moveTo(c, c);
        ctx.arc(c, c, r * SCALE, -a0, -(a0 + wedge), true);
        ctx.closePath();
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    }

    ctx.fillStyle = p.raw;
    ctx.globalAlpha = 0.35;
    const step = Math.max(1, Math.floor(samples.length / 4000));
    for (let i = 0; i < samples.length; i += step) {
      const s = samples[i];
      ctx.fillRect(X(s.x - center.x) - 1, Y(s.y - center.y) - 1, 2, 2);
    }
    ctx.globalAlpha = 1;

    if (calibration) {
      ctx.fillStyle = p.cal;
      ctx.globalAlpha = 0.35;
      for (let i = 0; i < samples.length; i += step) {
        const o = applyStick(calibration, samples[i]);
        ctx.fillRect(X(o.x) - 1, Y(o.y) - 1, 2, 2);
      }
      ctx.globalAlpha = 1;
    }

    ctx.setLineDash([5, 5]);
    ctx.strokeStyle = p.text;
    ctx.lineWidth = 1.25;
    ctx.beginPath();
    ctx.arc(c, c, r, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);

    const reach = (a: CircularityAnalysis, color: string) => {
      const pts = a.sectors.filter((s) => s.samples > 0);
      if (pts.length < 2) return;
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      pts.forEach((s, i) => {
        const ang = (s.angleDeg * Math.PI) / 180;
        const x = X(Math.cos(ang) * s.maxRadius);
        const y = Y(Math.sin(ang) * s.maxRadius);
        if (i) ctx.lineTo(x, y);
        else ctx.moveTo(x, y);
      });
      ctx.closePath();
      ctx.stroke();
    };
    if (before) reach(before, p.raw);
    if (after) reach(after, p.cal);
  }, [samples, center, before, after, calibration, size, p, ref]);

  return (
    <figure className={css.figure}>
      <canvas
        ref={ref}
        className={css.canvas}
        role="img"
        aria-label={t('calibration.report.circ.title')}
      />
      <figcaption className={css.legend}>
        <span>
          <i className={css.ideal} /> {t('calibration.report.circ.legendIdeal')}
        </span>
        <span>
          <i className={css.reach} /> {t('calibration.report.circ.legendReach')}
        </span>
        <span>
          <i className={css.points} /> {t('calibration.report.circ.legendPoints')}
        </span>
        <span>
          <i className={css.fail} /> {t('calibration.report.circ.legendFail')}
        </span>
        {(after || calibration) && (
          <span>
            <i className={css.after} /> {t('calibration.report.circ.legendAfter')}
          </span>
        )}
      </figcaption>
    </figure>
  );
}
