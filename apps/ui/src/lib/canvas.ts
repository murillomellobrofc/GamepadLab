import { useEffect, useRef, useState } from 'react';

/** Reads the current value of a CSS custom property on <html>. */
export function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#888';
}

export interface Palette {
  bg: string;
  surface: string;
  line: string;
  lineStrong: string;
  text: string;
  muted: string;
  faint: string;
  accent: string;
  raw: string;
  cal: string;
  alt: string;
  ok: string;
  warn: string;
  bad: string;
  grid: string;
  gridStrong: string;
}

export function readPalette(): Palette {
  return {
    bg: cssVar('--bg'),
    surface: cssVar('--surface'),
    line: cssVar('--line'),
    lineStrong: cssVar('--line-strong'),
    text: cssVar('--text'),
    muted: cssVar('--text-muted'),
    faint: cssVar('--text-faint'),
    accent: cssVar('--accent'),
    raw: cssVar('--sig-raw'),
    cal: cssVar('--sig-cal'),
    alt: cssVar('--sig-alt'),
    ok: cssVar('--ok'),
    warn: cssVar('--warn'),
    bad: cssVar('--bad'),
    grid: cssVar('--graticule'),
    gridStrong: cssVar('--graticule-strong'),
  };
}

/** Palette that refreshes when theme/contrast attributes change. */
export function usePalette(): Palette {
  const [p, setP] = useState(readPalette);
  useEffect(() => {
    const obs = new MutationObserver(() => setP(readPalette()));
    obs.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['data-theme', 'data-contrast'],
    });
    return () => obs.disconnect();
  }, []);
  return p;
}

/**
 * Keeps a canvas' backing store matched to its CSS size × devicePixelRatio.
 * Returns the canvas ref and the current CSS size.
 */
export function useHiDpiCanvas() {
  const ref = useRef<HTMLCanvasElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const apply = () => {
      const r = el.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const w = Math.max(1, Math.round(r.width));
      const h = Math.max(1, Math.round(r.height));
      if (el.width !== Math.round(w * dpr) || el.height !== Math.round(h * dpr)) {
        el.width = Math.round(w * dpr);
        el.height = Math.round(h * dpr);
      }
      setSize({ w, h });
    };
    apply();
    const ro = new ResizeObserver(apply);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return { ref, size };
}

export function begin(canvas: HTMLCanvasElement): CanvasRenderingContext2D | null {
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;
  const dpr = canvas.width / Math.max(1, canvas.getBoundingClientRect().width);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  return ctx;
}

/** Oscilloscope-style graticule inside a square of side `s` centred at (cx, cy). */
export function graticule(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  r: number,
  p: Palette,
) {
  ctx.save();
  ctx.strokeStyle = p.gridStrong;
  ctx.lineWidth = 1;
  for (let i = -4; i <= 4; i++) {
    const o = (i / 4) * r;
    ctx.globalAlpha = i === 0 ? 1 : 0.55;
    ctx.beginPath();
    ctx.moveTo(cx + o, cy - r);
    ctx.lineTo(cx + o, cy + r);
    ctx.moveTo(cx - r, cy + o);
    ctx.lineTo(cx + r, cy + o);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
  ctx.strokeStyle = p.lineStrong;
  ctx.beginPath();
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
  ctx.stroke();
  ctx.setLineDash([3, 4]);
  ctx.beginPath();
  ctx.arc(cx, cy, r * 0.5, 0, Math.PI * 2);
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.restore();
}
