import { describe, expect, it } from 'vitest';
import { resolveIconVariant } from './useAppIcon';

describe('resolveIconVariant', () => {
  it('uses the chosen symbol whatever the theme', () => {
    expect(resolveIconVariant('light', 'dark', true)).toBe('light');
    expect(resolveIconVariant('dark', 'light', false)).toBe('dark');
  });

  it('follows the app theme on automatic, and the system when the theme is automatic too', () => {
    expect(resolveIconVariant('auto', 'dark', false)).toBe('dark');
    expect(resolveIconVariant('auto', 'light', true)).toBe('light');
    expect(resolveIconVariant('auto', 'auto', true)).toBe('dark');
    expect(resolveIconVariant('auto', 'auto', false)).toBe('light');
  });
});
