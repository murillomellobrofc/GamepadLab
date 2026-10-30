import { describe, expect, it } from 'vitest';
import { sanitizeSvg } from './loadArt';

describe('sanitizeSvg', () => {
  it('keeps drawing elements, geometry, classes and IDs', () => {
    const svg = sanitizeSvg(
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><g id="btn-cross"><circle class="gl-part" cx="5" cy="5" r="4"/></g></svg>',
    );
    expect(svg.getAttribute('viewBox')).toBe('0 0 10 10');
    const circle = svg.querySelector('#btn-cross circle');
    expect(circle?.getAttribute('class')).toBe('gl-part');
    expect(circle?.getAttribute('r')).toBe('4');
  });

  it('drops scripts, handlers, styles, foreign content and outside links', () => {
    const svg =
      sanitizeSvg(`<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" onload="alert(1)">
      <script>alert(2)</script>
      <style>.a{fill:red}</style>
      <foreignObject><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject>
      <image href="https://example.com/x.png"/>
      <a xlink:href="javascript:alert(3)"><rect width="1" height="1"/></a>
      <path d="M0 0L1 1" onclick="alert(4)" style="fill:red"/>
      <rect id="ok" width="2" height="2"/>
    </svg>`);
    const html = svg.outerHTML;
    for (const bad of [
      'script',
      'alert',
      'style',
      'foreignObject',
      'image',
      'javascript',
      'example.com',
    ]) {
      expect(html, bad).not.toContain(bad);
    }
    expect(svg.querySelector('#ok')).not.toBeNull();
    expect(svg.querySelector('path')?.getAttribute('d')).toBe('M0 0L1 1');
  });

  it('refuses anything that is not an SVG', () => {
    expect(() => sanitizeSvg('<html><body>nope</body></html>')).toThrow();
    expect(() => sanitizeSvg('<svg xmlns="http://www.w3.org/2000/svg"><g>')).toThrow();
  });
});
