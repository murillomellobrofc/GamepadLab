import type { Palette } from '@/lib/canvas';

/** Light "paper" palette so printed charts read well regardless of theme. */
export const PAPER: Palette = {
  bg: '#ffffff',
  surface: '#ffffff',
  line: '#d9d5cb',
  lineStrong: '#a9a498',
  text: '#13171c',
  muted: '#555d68',
  faint: '#858b94',
  accent: '#0a8a60',
  raw: '#2f5fd0',
  cal: '#0a8a60',
  alt: '#b5268a',
  ok: '#0a8a60',
  warn: '#a96400',
  bad: '#c41f3d',
  grid: 'rgba(20,24,29,0.06)',
  gridStrong: 'rgba(20,24,29,0.12)',
};
