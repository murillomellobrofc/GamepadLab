import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { ControllerInfo } from '@/api/types';
import { sanitizeSvg } from './art/loadArt';
import { ART_SPECS, artFor, buttonsFor } from './layouts';

// vitest runs from the UI package (npm run test -w @gamepadlab/ui).
const PUBLIC = [resolve('public'), resolve('apps/ui/public')].find((p) => existsSync(p))!;
const drawing = (url: string) => sanitizeSvg(readFileSync(PUBLIC + url, 'utf8'));
const part = (svg: SVGSVGElement, id: string) => svg.querySelector(`[id="${id}"]`);

describe('controller drawings', () => {
  it.each(ART_SPECS.map((s) => [s.family, s] as const))(
    '%s has every part the frontend drives',
    (_, spec) => {
      const svg = drawing(spec.url);
      for (const id of Object.values(spec.ids)) expect(part(svg, id), id).not.toBeNull();
      for (const b of [10, 11]) {
        const id = spec.ids[b];
        expect(part(svg, `${id}-base`)?.querySelector('circle'), `${id}-base`).not.toBeNull();
        expect(part(svg, id)?.querySelector('circle'), id).not.toBeNull();
      }
      for (const b of [6, 7]) {
        const id = spec.ids[b];
        expect(part(svg, `${id}-fill`)?.localName, `${id}-fill`).toBe('rect');
        expect(part(svg, `${id}-value`), `${id}-value`).not.toBeNull();
      }
      // Only theme classes: no colours baked into the art.
      expect(svg.outerHTML).not.toMatch(/fill="#|stroke="#/);
    },
  );

  it('the DualSense has a touchpad with two touch points', () => {
    const svg = drawing(artFor({ kind: 'ps5' }).url);
    expect(part(svg, 'touchpad')?.querySelector('.gl-part')).not.toBeNull();
    expect(part(svg, 'touch-0')).not.toBeNull();
    expect(part(svg, 'touch-1')).not.toBeNull();
  });

  it("keeps Zacksly's original files and licence next to each derived drawing", () => {
    for (const dir of ['ps5', 'xbox', 'switch']) {
      const licence = readFileSync(`${PUBLIC}/controllers/${dir}/LICENSE.txt`, 'utf8');
      expect(licence).toContain('Creative Commons Attribution 3.0');
    }
    expect(existsSync(`${PUBLIC}/controllers/ps5/source/Outline White SVG.svg`)).toBe(true);
    const derived = readFileSync(`${PUBLIC}/controllers/ps5/dualsense.svg`, 'utf8');
    expect(derived).toContain('by Zacksly');
    expect(derived).toContain('CC BY 3.0');
    expect(readFileSync(`${PUBLIC}/controllers/gamepadlab/LICENSE.txt`, 'utf8')).toContain(
      'Attribution 4.0',
    );
  });
});

describe('buttonsFor', () => {
  const pad = (over: Partial<ControllerInfo>) =>
    ({ kind: 'ps5', buttonCount: 19, name: 'Pad', ...over }) as ControllerInfo;

  it('hides what a model does not have', () => {
    expect(buttonsFor(pad({ kind: 'ps5' }))).toContain(18);
    expect(buttonsFor(pad({ kind: 'ps5' }))).not.toContain(19);
    expect(buttonsFor(pad({ kind: 'ps5', buttonCount: 23 }))).toContain(22);
    expect(buttonsFor(pad({ kind: 'ps4' }))).not.toContain(18);
    expect(buttonsFor(pad({ kind: 'xbox', name: 'Xbox Wireless Controller' }))).not.toContain(19);
    expect(buttonsFor(pad({ kind: 'xbox', name: 'Xbox Elite Series 2' }))).toContain(19);
    expect(buttonsFor(pad({ kind: 'generic', buttonCount: 10 }))).toEqual([
      0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    ]);
  });
});
