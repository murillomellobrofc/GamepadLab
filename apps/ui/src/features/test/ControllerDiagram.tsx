import { useLayoutEffect, useRef } from 'react';
import type { ControllerInfo, FrameEntry } from '@/api/types';
import { buttonName } from '@/lib/buttonNames';
import { useApp } from '@/state/app';
import { live, useAnimationFrame } from '@/state/live';
import { GenericDiagram } from './GenericDiagram';
import css from './ControllerDiagram.module.css';
import { layoutFor, type Shape } from './layouts';

function pick(entry: FrameEntry, calibrated: boolean) {
  const c = calibrated ? entry.calibrated : null;
  return {
    buttons: c?.buttons ?? entry.snapshot.buttons,
    left: c?.leftStick ?? entry.snapshot.leftStick,
    right: c?.rightStick ?? entry.snapshot.rightStick,
    lt: c?.leftTrigger ?? entry.snapshot.leftTrigger,
    rt: c?.rightTrigger ?? entry.snapshot.rightTrigger,
  };
}

function ShapeEl({ s, kind }: { s: Shape; kind: ControllerInfo['kind'] }) {
  switch (s.kind) {
    case 'circle':
      return (
        <g data-btn={s.btn} className={css.btn}>
          <title>{s.title}</title>
          <circle cx={s.x} cy={s.y} r={s.r} />
          {s.label && (
            <text x={s.x} y={s.y} className={css.glyph} style={{ fontSize: s.r * 0.95 }}>
              {s.label}
            </text>
          )}
        </g>
      );
    case 'rect':
      return (
        <g data-btn={s.btn} className={css.btn} data-back={s.back || undefined}>
          <title>{s.title}</title>
          <rect x={s.x} y={s.y} width={s.w} height={s.h} rx={s.rx ?? 4} />
          {s.label && (
            <text
              x={s.x + s.w / 2}
              y={s.y + s.h / 2}
              className={css.glyph}
              style={{ fontSize: 11 }}
            >
              {s.label}
            </text>
          )}
        </g>
      );
    case 'dpad': {
      const a = s.size / 3;
      const arms: [number, number, number, string][] = [
        [12, s.x - a / 2, s.y - s.size / 2, '▲'],
        [13, s.x - a / 2, s.y + a / 2, '▼'],
        [14, s.x - s.size / 2, s.y - a / 2, '◀'],
        [15, s.x + a / 2, s.y - a / 2, '▶'],
      ];
      return (
        <g>
          <rect x={s.x - a / 2} y={s.y - a / 2} width={a} height={a} className={css.dpadHub} />
          {arms.map(([btn, x, y, glyph]) => (
            <g key={btn} data-btn={btn} className={css.btn}>
              <title>{buttonName(kind, btn)}</title>
              <rect x={x} y={y} width={a} height={a} rx={4} />
              <text x={x + a / 2} y={y + a / 2} className={css.glyphSmall}>
                {glyph}
              </text>
            </g>
          ))}
        </g>
      );
    }
    case 'stick':
      return (
        <g>
          <circle cx={s.x} cy={s.y} r={s.r + 8} className={css.well} />
          <g data-stick={s.side} data-btn={s.side === 'left' ? 10 : 11} className={css.btn}>
            <title>{s.side === 'left' ? 'L3' : 'R3'}</title>
            <circle cx={s.x} cy={s.y} r={s.r * 0.72} className={css.cap} />
            <circle cx={s.x} cy={s.y} r={s.r * 0.45} className={css.capInner} />
          </g>
        </g>
      );
    case 'trigger':
      return (
        <g data-btn={s.side === 'left' ? 6 : 7} className={css.btn}>
          <title>{s.label}</title>
          <rect x={s.x} y={s.y} width={s.w} height={s.h} rx={10} className={css.triggerFrame} />
          <rect
            data-trigger-fill={s.side}
            x={s.x}
            y={s.y}
            width={0}
            height={s.h}
            rx={10}
            className={css.triggerFill}
          />
          <text x={s.x + s.w / 2} y={s.y + s.h / 2} className={css.glyph} style={{ fontSize: 12 }}>
            <tspan>{s.label} </tspan>
            <tspan data-trigger-text={s.side} className={css.triggerPct}>
              0%
            </tspan>
          </text>
        </g>
      );
    case 'touchpad':
      return (
        <g data-btn={17} className={css.btn}>
          <title>Touchpad</title>
          <rect x={s.x} y={s.y} width={s.w} height={s.h} rx={12} className={css.touchpad} />
          {[0, 1].map((i) => (
            <circle key={i} data-touch={i} r={7} cx={s.x} cy={s.y} className={css.touchDot} />
          ))}
        </g>
      );
  }
}

export function ControllerDiagram({
  controller,
  forceCalibrated,
  testHighlight,
}: {
  controller: ControllerInfo;
  forceCalibrated?: boolean;
  /** Button the guided test is asking for: drawn with a pulsing ring. */
  testHighlight?: number | null;
}) {
  const spec = layoutFor(controller);
  const svgRef = useRef<SVGSVGElement>(null);
  const viewMode = useApp((s) => s.viewMode);
  const rumble = useApp((s) => s.rumble[controller.id]);
  const calibrated = forceCalibrated ?? viewMode === 'calibrated';
  const els = useRef<{
    btns: [Element, number][];
    sticks: [SVGGElement, 'left' | 'right', number][];
    fills: [SVGRectElement, 'left' | 'right', number][];
    texts: [Element, 'left' | 'right'][];
    touch: SVGCircleElement[];
    pad: { x: number; y: number; w: number; h: number } | null;
  } | null>(null);

  useLayoutEffect(() => {
    const svg = svgRef.current;
    if (!svg || !spec) return;
    const stickR = new Map(
      spec.shapes.filter((s) => s.kind === 'stick').map((s) => [s.side, s.r] as const),
    );
    const trigW = new Map(
      spec.shapes.filter((s) => s.kind === 'trigger').map((s) => [s.side, s.w] as const),
    );
    const pad = spec.shapes.find((s) => s.kind === 'touchpad');
    els.current = {
      btns: [...svg.querySelectorAll('[data-btn]')].map((e) => [
        e,
        Number(e.getAttribute('data-btn')),
      ]),
      sticks: [...svg.querySelectorAll<SVGGElement>('[data-stick]')].map((e) => {
        const side = e.getAttribute('data-stick') as 'left' | 'right';
        return [e, side, stickR.get(side) ?? 30];
      }),
      fills: [...svg.querySelectorAll<SVGRectElement>('[data-trigger-fill]')].map((e) => {
        const side = e.getAttribute('data-trigger-fill') as 'left' | 'right';
        return [e, side, trigW.get(side) ?? 90];
      }),
      texts: [...svg.querySelectorAll('[data-trigger-text]')].map((e) => [
        e,
        e.getAttribute('data-trigger-text') as 'left' | 'right',
      ]),
      touch: [...svg.querySelectorAll<SVGCircleElement>('[data-touch]')],
      pad: pad && pad.kind === 'touchpad' ? { x: pad.x, y: pad.y, w: pad.w, h: pad.h } : null,
    };
  }, [spec]);

  useAnimationFrame(() => {
    const e = els.current;
    const entry = live.get(controller.id);
    if (!e || !entry) return;
    const v = pick(entry, calibrated);
    for (const [el, i] of e.btns) {
      const b = v.buttons[i];
      const on = !!b?.pressed;
      if (el.getAttribute('data-on') !== String(on)) el.setAttribute('data-on', String(on));
      (el as SVGElement).style.setProperty('--v', String(b?.value ?? 0));
    }
    for (const [el, side, r] of e.sticks) {
      const s = side === 'left' ? v.left : v.right;
      el.setAttribute('transform', `translate(${(s.x * r).toFixed(2)} ${(-s.y * r).toFixed(2)})`);
    }
    for (const [el, side, w] of e.fills) {
      el.setAttribute('width', ((side === 'left' ? v.lt : v.rt) * w).toFixed(1));
    }
    for (const [el, side] of e.texts) {
      el.textContent = `${Math.round((side === 'left' ? v.lt : v.rt) * 100)}%`;
    }
    const touch = entry.snapshot.touch;
    if (e.pad && touch) {
      e.touch.forEach((dot, i) => {
        const p = touch.points[i];
        if (p?.active) {
          dot.setAttribute('cx', (e.pad!.x + p.x * e.pad!.w).toFixed(1));
          dot.setAttribute('cy', (e.pad!.y + p.y * e.pad!.h).toFixed(1));
          dot.setAttribute('data-active', 'true');
        } else {
          dot.setAttribute('data-active', 'false');
        }
      });
    }
  });

  if (!spec) return <GenericDiagram controller={controller} calibrated={calibrated} />;

  const vibrating = !!rumble?.active && (rumble.weak > 0.02 || rumble.strong > 0.02);
  return (
    <svg
      ref={svgRef}
      viewBox="0 0 600 400"
      className={css.svg}
      data-vibrating={vibrating || undefined}
      data-highlight={testHighlight ?? undefined}
      role="img"
      aria-label={controller.name}
      style={{ ['--rumble' as string]: String(Math.max(rumble?.weak ?? 0, rumble?.strong ?? 0)) }}
    >
      <path d={spec.body} className={css.body} />
      {spec.shapes.map((s, i) => (
        <ShapeEl key={i} s={s} kind={controller.kind} />
      ))}
      {testHighlight != null && <HighlightRing spec={spec.shapes} btn={testHighlight} />}
    </svg>
  );
}

function HighlightRing({ spec, btn }: { spec: Shape[]; btn: number }) {
  for (const s of spec) {
    if ((s.kind === 'circle' || s.kind === 'rect') && s.btn === btn) {
      const [cx, cy, r] =
        s.kind === 'circle'
          ? [s.x, s.y, s.r + 9]
          : [s.x + s.w / 2, s.y + s.h / 2, Math.max(s.w, s.h) / 2 + 10];
      return <circle cx={cx} cy={cy} r={r} className={css.ring} />;
    }
    if (s.kind === 'stick' && (s.side === 'left' ? 10 : 11) === btn) {
      return <circle cx={s.x} cy={s.y} r={s.r + 14} className={css.ring} />;
    }
    if (s.kind === 'trigger' && (s.side === 'left' ? 6 : 7) === btn) {
      return (
        <rect
          x={s.x - 6}
          y={s.y - 6}
          width={s.w + 12}
          height={s.h + 12}
          rx={14}
          className={css.ring}
        />
      );
    }
    if (s.kind === 'dpad' && btn >= 12 && btn <= 15) {
      const a = s.size / 3;
      const off: Record<number, [number, number]> = {
        12: [0, -a],
        13: [0, a],
        14: [-a, 0],
        15: [a, 0],
      };
      const [dx, dy] = off[btn];
      return <circle cx={s.x + dx} cy={s.y + dy} r={a / 2 + 9} className={css.ring} />;
    }
    if (s.kind === 'touchpad' && btn === 17) {
      return (
        <rect
          x={s.x - 6}
          y={s.y - 6}
          width={s.w + 12}
          height={s.h + 12}
          rx={16}
          className={css.ring}
        />
      );
    }
  }
  return null;
}
