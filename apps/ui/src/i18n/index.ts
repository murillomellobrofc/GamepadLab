import { useCallback } from 'react';
import ptBR from './locales/pt-BR';
import en from './locales/en';
import es from './locales/es';
import { useSettings } from '@/state/settings';

export const LOCALES = { 'pt-BR': ptBR, en, es } as const;
export type Locale = keyof typeof LOCALES;
export const LOCALE_NAMES: Record<Locale, string> = {
  'pt-BR': 'Português (Brasil)',
  en: 'English',
  es: 'Español',
};

type Leaves<T, P extends string = ''> = {
  [K in keyof T & string]: T[K] extends string ? `${P}${K}` : Leaves<T[K], `${P}${K}.`>;
}[keyof T & string];

/** Every translation key, derived from the PT-BR source file. */
export type TKey = Leaves<typeof ptBR>;
export type TVars = Record<string, string | number>;
export type TFn = (key: TKey, vars?: TVars) => string;

// Compile-time guard: every language must provide every PT-BR key.
const _enMatchesShape: typeof ptBR = en;
const _esMatchesShape: typeof ptBR = es;
void _enMatchesShape;
void _esMatchesShape;

function lookup(dict: unknown, key: string): string | undefined {
  let node: unknown = dict;
  for (const part of key.split('.')) {
    if (node && typeof node === 'object' && part in (node as Record<string, unknown>)) {
      node = (node as Record<string, unknown>)[part];
    } else {
      return undefined;
    }
  }
  return typeof node === 'string' ? node : undefined;
}

export function translate(locale: Locale, key: TKey | string, vars?: TVars): string {
  const raw = lookup(LOCALES[locale], key) ?? lookup(LOCALES['pt-BR'], key) ?? key;
  if (!vars) return raw;
  return raw.replace(/\{(\w+)\}/g, (_, name: string) =>
    name in vars ? String(vars[name]) : `{${name}}`,
  );
}

export function useT(): TFn {
  const locale = useSettings((s) => s.locale);
  return useCallback((key: TKey, vars?: TVars) => translate(locale, key, vars), [locale]);
}

/** Collects all leaf keys of a nested dictionary (used by the parity test). */
export function flattenKeys(dict: object, prefix = ''): string[] {
  return Object.entries(dict).flatMap(([k, v]) =>
    typeof v === 'string' ? [`${prefix}${k}`] : flattenKeys(v as object, `${prefix}${k}.`),
  );
}

/** Date and time in the language picked in the app, not the system's. */
export function formatDateTime(value: Date | string | number): string {
  const d = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(d.getTime())) return String(value);
  return d.toLocaleString(useSettings.getState().locale);
}
