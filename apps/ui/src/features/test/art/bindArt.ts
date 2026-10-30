import type { ButtonState, TouchState, Vec2 } from '@/api/types';
import type { ArtSpec } from '../layouts';

/** What a frame shows (raw or calibrated). */
export interface ArtValues {
  buttons: ButtonState[];
  left: Vec2;
  right: Vec2;
  lt: number;
  rt: number;
}

export interface ArtBinding {
  update(v: ArtValues, touch: TouchState | null | undefined): void;
  /** Draws the guided-test ring around a button (null clears it). */
  highlight(button: number | null): void;
}

const SVG_NS = 'http://www.w3.org/2000/svg';

/** Radius of a part's circle (the part itself or its first circle). */
function radius(el: Element | null): number {
  const c = el?.localName === 'circle' ? el : el?.querySelector('circle');
  return Number(c?.getAttribute('r') ?? 0);
}

/**
 * Wires a freshly cloned drawing to live values. IDs become `data-part`
 * attributes, so several drawings of the same family can share a page.
 */
export function bindArt(
  svg: SVGSVGElement,
  spec: ArtSpec,
  present: number[],
  name: (button: number) => string,
  ringClass: string,
): ArtBinding {
  for (const el of svg.querySelectorAll('[id]')) {
    el.setAttribute('data-part', el.id);
    el.removeAttribute('id');
  }
  const part = (id: string) => svg.querySelector<SVGGraphicsElement>(`[data-part="${id}"]`);

  const buttons: [SVGGraphicsElement, number][] = [];
  for (const [key, id] of Object.entries(spec.ids)) {
    const b = Number(key);
    const el = part(id);
    if (!el) continue;
    if (!present.includes(b)) {
      el.setAttribute('data-absent', 'true');
      continue;
    }
    const title = document.createElementNS(SVG_NS, 'title');
    title.textContent = name(b);
    el.prepend(title);
    buttons.push([el, b]);
  }

  const sticks = ([10, 11] as const).flatMap((b) => {
    const id = spec.ids[b];
    const cap = id ? part(id) : null;
    if (!cap) return [];
    const travel = Math.max(0, radius(part(`${id}-base`)) - radius(cap));
    return [{ cap, travel, side: b === 10 ? ('left' as const) : ('right' as const) }];
  });

  const triggers = ([6, 7] as const).flatMap((b) => {
    const id = spec.ids[b];
    const fill = id ? part(`${id}-fill`) : null;
    if (!fill) return [];
    return [
      {
        fill,
        full: Number(fill.getAttribute('width') ?? 0),
        value: part(`${id}-value`),
        side: b === 6 ? ('left' as const) : ('right' as const),
      },
    ];
  });

  const pad = part('touchpad');
  const padArea = pad?.querySelector<SVGGraphicsElement>('.gl-part') ?? null;
  const dots = [part('touch-0'), part('touch-1')];
  let padBox: DOMRect | null = null;

  const overlay = document.createElementNS(SVG_NS, 'g');
  overlay.setAttribute('aria-hidden', 'true');
  svg.append(overlay);

  return {
    update(v, touch) {
      for (const [el, b] of buttons) {
        const s = v.buttons[b];
        const on = String(!!s?.pressed);
        if (el.getAttribute('data-on') !== on) el.setAttribute('data-on', on);
        el.style.setProperty('--v', String(s?.value ?? 0));
      }
      for (const { cap, travel, side } of sticks) {
        const s = side === 'left' ? v.left : v.right;
        cap.setAttribute(
          'transform',
          `translate(${(s.x * travel).toFixed(1)} ${(-s.y * travel).toFixed(1)})`,
        );
      }
      for (const { fill, full, value, side } of triggers) {
        const t = side === 'left' ? v.lt : v.rt;
        fill.setAttribute('width', (Math.max(0, Math.min(1, t)) * full).toFixed(1));
        if (value) value.textContent = `${Math.round(t * 100)}%`;
      }
      if (padArea && touch) {
        if (!padBox || padBox.width === 0) padBox = padArea.getBBox();
        dots.forEach((dot, i) => {
          const p = touch.points[i];
          if (!dot) return;
          if (p?.active && padBox) {
            dot.setAttribute('cx', (padBox.x + p.x * padBox.width).toFixed(1));
            dot.setAttribute('cy', (padBox.y + p.y * padBox.height).toFixed(1));
            dot.setAttribute('data-active', 'true');
          } else {
            dot.setAttribute('data-active', 'false');
          }
        });
      }
    },
    highlight(button) {
      overlay.replaceChildren();
      const el = button == null ? null : buttons.find(([, b]) => b === button)?.[0];
      if (!el) return;
      const box = el.getBBox();
      if (!box.width) return;
      const pad = 22;
      const ring = document.createElementNS(SVG_NS, 'rect');
      ring.setAttribute('x', String(box.x - pad));
      ring.setAttribute('y', String(box.y - pad));
      ring.setAttribute('width', String(box.width + pad * 2));
      ring.setAttribute('height', String(box.height + pad * 2));
      ring.setAttribute('rx', String(Math.min(box.width, box.height) / 2 + pad));
      ring.setAttribute('class', ringClass);
      overlay.append(ring);
    },
  };
}
