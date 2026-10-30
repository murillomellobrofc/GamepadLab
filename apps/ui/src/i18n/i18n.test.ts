import { describe, expect, it } from 'vitest';
import { flattenKeys, LOCALES, translate, type Locale } from './index';

const others = (Object.keys(LOCALES) as Locale[]).filter((l) => l !== 'pt-BR');

describe('i18n', () => {
  it.each(others)('%s defines exactly the same keys as PT-BR', (locale) => {
    const pt = flattenKeys(LOCALES['pt-BR']).sort();
    const keys = flattenKeys(LOCALES[locale]).sort();
    expect(keys.filter((k) => !pt.includes(k))).toEqual([]);
    expect(pt.filter((k) => !keys.includes(k))).toEqual([]);
  });

  it.each(others)('%s keeps the same {placeholders} as PT-BR', (locale) => {
    const vars = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join(',');
    for (const key of flattenKeys(LOCALES['pt-BR'])) {
      expect(vars(translate(locale, key)), key).toBe(vars(translate('pt-BR', key)));
    }
  });

  it('interpolates variables', () => {
    expect(translate('en', 'app.toast.connected', { name: 'Pad' })).toContain('Pad');
    expect(translate('es', 'app.toast.connected', { name: 'Pad' })).toBe('Pad conectado');
  });
});
