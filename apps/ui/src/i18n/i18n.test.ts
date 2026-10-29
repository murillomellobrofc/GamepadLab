import { describe, expect, it } from 'vitest';
import { flattenKeys, LOCALES, translate } from './index';

describe('i18n', () => {
  it('PT-BR and English define exactly the same keys', () => {
    const pt = flattenKeys(LOCALES['pt-BR']).sort();
    const en = flattenKeys(LOCALES.en).sort();
    expect(en.filter((k) => !pt.includes(k))).toEqual([]);
    expect(pt.filter((k) => !en.includes(k))).toEqual([]);
  });

  it('keeps the same {placeholders} in both languages', () => {
    for (const key of flattenKeys(LOCALES['pt-BR'])) {
      const vars = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join(',');
      expect(vars(translate('en', key)), key).toBe(vars(translate('pt-BR', key)));
    }
  });

  it('interpolates variables', () => {
    expect(translate('en', 'app.toast.connected', { name: 'Pad' })).toContain('Pad');
  });
});
