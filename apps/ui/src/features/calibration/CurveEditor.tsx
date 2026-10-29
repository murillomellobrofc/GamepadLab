import { useRef, useState } from 'react';
import type { ResponseCurve, StickCalibration, Vec2 } from '@/api/types';
import { useT } from '@/i18n';
import { antiDeadzone, deadzoneScale, evalCurve } from '@/lib/calibration';
import ui from '@/styles/ui.module.css';
import css from './Adjustments.module.css';

const S = 240;
const PAD = 14;
const MAX_POINTS = 16;
const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

/**
 * Response curve editor. Custom curves get draggable points; everything is
 * reachable by keyboard (Tab to a point, arrows to move, Delete to remove).
 */
export function CurveEditor({
  cal,
  onChange,
}: {
  cal: StickCalibration;
  onChange: (curve: ResponseCurve) => void;
}) {
  const t = useT();
  const svg = useRef<SVGSVGElement>(null);
  const [drag, setDrag] = useState<number | null>(null);
  const curve = cal.curve;
  const custom = curve.kind === 'custom';
  const X = (x: number) => PAD + x * (S - 2 * PAD);
  const Y = (y: number) => S - PAD - y * (S - 2 * PAD);

  // Full stick response (deadzones + curve + anti-deadzone) for the magnitude.
  const path = Array.from({ length: 81 }, (_, i) => {
    const x = i / 80;
    const s = deadzoneScale(x, cal.innerDeadzone, cal.outerDeadzone);
    const y =
      s > 0 ? Math.min(1, antiDeadzone(evalCurve(curve, Math.min(s, 1)), cal.antiDeadzone)) : 0;
    return `${i ? 'L' : 'M'} ${X(x).toFixed(1)} ${Y(y).toFixed(1)}`;
  }).join(' ');

  const toCurve = (clientX: number, clientY: number): Vec2 => {
    const r = svg.current!.getBoundingClientRect();
    const k = S / r.width;
    return {
      x: clamp01(((clientX - r.left) * k - PAD) / (S - 2 * PAD)),
      y: clamp01((S - PAD - (clientY - r.top) * k) / (S - 2 * PAD)),
    };
  };

  const setPoint = (i: number, p: Vec2) => {
    const pts = curve.points.map((q, j) =>
      j === i ? { x: Math.min(0.99, Math.max(0.01, p.x)), y: p.y } : q,
    );
    onChange({ kind: 'custom', points: pts });
  };

  const add = (p: Vec2) => {
    if (curve.points.length >= MAX_POINTS) return;
    const pts = [...curve.points, p].sort((a, b) => a.x - b.x);
    onChange({ kind: 'custom', points: pts });
  };

  const remove = (i: number) =>
    onChange({ kind: 'custom', points: curve.points.filter((_, j) => j !== i) });

  return (
    <div className={css.curve}>
      <svg
        ref={svg}
        viewBox={`0 0 ${S} ${S}`}
        className={css.curveSvg}
        role="group"
        aria-label={t('calibration.adjust.curveEditor')}
        onDoubleClick={(e) => custom && add(toCurve(e.clientX, e.clientY))}
        onPointerMove={(e) => drag !== null && setPoint(drag, toCurve(e.clientX, e.clientY))}
        onPointerUp={() => setDrag(null)}
        onPointerLeave={() => setDrag(null)}
      >
        {[0, 0.25, 0.5, 0.75, 1].map((g) => (
          <g key={g} className={css.curveGrid}>
            <line x1={X(g)} y1={Y(0)} x2={X(g)} y2={Y(1)} />
            <line x1={X(0)} y1={Y(g)} x2={X(1)} y2={Y(g)} />
          </g>
        ))}
        <line x1={X(0)} y1={Y(0)} x2={X(1)} y2={Y(1)} className={css.curveDiag} />
        <path d={path} className={css.curvePath} />
        <text x={X(1)} y={S - 1} textAnchor="end" className={css.curveAxis}>
          {t('calibration.adjust.input')} →
        </text>
        <text x={2} y={PAD - 3} className={css.curveAxis}>
          ↑ {t('calibration.adjust.output')}
        </text>
        {custom &&
          curve.points.map((p, i) => (
            <circle
              key={i}
              cx={X(p.x)}
              cy={Y(p.y)}
              r={7}
              tabIndex={0}
              role="slider"
              aria-label={t('calibration.adjust.point', {
                n: i + 1,
                x: `${Math.round(p.x * 100)}%`,
                y: `${Math.round(p.y * 100)}%`,
              })}
              aria-valuenow={Math.round(p.y * 100)}
              aria-valuemin={0}
              aria-valuemax={100}
              className={css.curvePoint}
              data-active={drag === i || undefined}
              onPointerDown={(e) => {
                (e.target as Element).setPointerCapture?.(e.pointerId);
                setDrag(i);
              }}
              onKeyDown={(e) => {
                const step = e.shiftKey ? 0.05 : 0.01;
                const d: Record<string, [number, number]> = {
                  ArrowLeft: [-step, 0],
                  ArrowRight: [step, 0],
                  ArrowUp: [0, step],
                  ArrowDown: [0, -step],
                };
                if (d[e.key]) {
                  e.preventDefault();
                  setPoint(i, { x: p.x + d[e.key][0], y: clamp01(p.y + d[e.key][1]) });
                } else if (e.key === 'Delete' || e.key === 'Backspace') {
                  e.preventDefault();
                  remove(i);
                }
              }}
            />
          ))}
      </svg>
      {custom && (
        <div className={css.curveTools}>
          <button
            type="button"
            className={`${ui.btn} ${ui.small}`}
            onClick={() => add({ x: 0.5, y: evalCurve(curve, 0.5) })}
            disabled={curve.points.length >= MAX_POINTS}
          >
            + {t('calibration.adjust.addPoint')}
          </button>
          <p>{t('calibration.adjust.curveHelp')}</p>
        </div>
      )}
    </div>
  );
}
