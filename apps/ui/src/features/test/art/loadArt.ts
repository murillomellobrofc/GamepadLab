/**
 * Loads a controller drawing from public/controllers and turns it into a
 * clean inline SVG. The files are ours, but they are art that can be swapped
 * (or come from a third party), so only drawing elements and geometry survive:
 * no scripts, event handlers, styles, foreign content or outside links.
 */

const ALLOWED = new Set([
  'svg',
  'g',
  'path',
  'circle',
  'ellipse',
  'rect',
  'line',
  'polyline',
  'polygon',
  'text',
  'tspan',
  'title',
  'desc',
]);

const SVG_NS = 'http://www.w3.org/2000/svg';

export function sanitizeSvg(text: string): SVGSVGElement {
  const doc = new DOMParser().parseFromString(text, 'image/svg+xml');
  const root = doc.documentElement;
  if (root.nodeName !== 'svg' || root.namespaceURI !== SVG_NS || doc.querySelector('parsererror')) {
    throw new Error('not an SVG drawing');
  }
  const walk = (el: Element) => {
    for (const child of [...el.children]) {
      if (!ALLOWED.has(child.localName) || child.namespaceURI !== SVG_NS) child.remove();
      else walk(child);
    }
    for (const { name, value } of [...el.attributes]) {
      const lower = name.toLowerCase();
      const link = lower === 'href' || lower.endsWith(':href');
      if (lower.startsWith('on') || lower === 'style' || (link && !value.startsWith('#'))) {
        el.removeAttribute(name);
      }
    }
  };
  walk(root);
  return document.importNode(root, true) as unknown as SVGSVGElement;
}

const cache = new Map<string, Promise<SVGSVGElement>>();

/** Fetches and sanitises a drawing once; callers clone the returned template. */
export function loadArt(url: string): Promise<SVGSVGElement> {
  let pending = cache.get(url);
  if (!pending) {
    pending = fetch(url)
      .then((r) => {
        if (!r.ok) throw new Error(`${url}: HTTP ${r.status}`);
        return r.text();
      })
      .then(sanitizeSvg);
    pending.catch(() => cache.delete(url));
    cache.set(url, pending);
  }
  return pending;
}
