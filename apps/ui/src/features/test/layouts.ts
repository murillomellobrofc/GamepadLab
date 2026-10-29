import type { ControllerInfo } from '@/api/types';

/** Shapes of one controller diagram, in a 600×400 viewBox. */
export type Shape =
  | { kind: 'circle'; btn: number; x: number; y: number; r: number; label?: string; title: string }
  | {
      kind: 'rect';
      btn: number;
      x: number;
      y: number;
      w: number;
      h: number;
      rx?: number;
      label?: string;
      title: string;
      back?: boolean;
    }
  | { kind: 'dpad'; x: number; y: number; size: number }
  | { kind: 'stick'; side: 'left' | 'right'; x: number; y: number; r: number }
  | {
      kind: 'trigger';
      side: 'left' | 'right';
      x: number;
      y: number;
      w: number;
      h: number;
      label: string;
    }
  | { kind: 'touchpad'; x: number; y: number; w: number; h: number };

export interface LayoutSpec {
  id: 'xbox' | 'dualsense' | 'dualshock4' | 'switchPro';
  body: string;
  shapes: Shape[];
}

const XBOX_BODY =
  'M150 70C200 55 400 55 450 70C510 85 545 120 560 175C580 245 600 320 575 360C555 392 505 392 480 360C455 330 430 300 400 295L200 295C170 300 145 330 120 360C95 392 45 392 25 360C0 320 20 245 40 175C55 120 90 85 150 70Z';
const PS_BODY =
  'M130 80C190 62 410 62 470 80C525 96 555 130 568 185C585 255 598 330 572 372C550 400 500 398 478 368C455 336 432 310 400 306L200 306C168 310 145 336 122 368C100 398 50 400 28 372C2 330 15 255 32 185C45 130 75 96 130 80Z';

const shoulders = (lb: string, rb: string, lt: string, rt: string, top = 6): Shape[] => [
  { kind: 'trigger', side: 'left', x: 118, y: top, w: 94, h: 30, label: lt },
  { kind: 'trigger', side: 'right', x: 388, y: top, w: 94, h: 30, label: rt },
  { kind: 'rect', btn: 4, x: 108, y: top + 38, w: 114, h: 18, rx: 9, label: lb, title: lb },
  { kind: 'rect', btn: 5, x: 378, y: top + 38, w: 114, h: 18, rx: 9, label: rb, title: rb },
];

export const XBOX: LayoutSpec = {
  id: 'xbox',
  body: XBOX_BODY,
  shapes: [
    ...shoulders('LB', 'RB', 'LT', 'RT'),
    { kind: 'stick', side: 'left', x: 180, y: 150, r: 34 },
    { kind: 'dpad', x: 238, y: 232, size: 66 },
    { kind: 'stick', side: 'right', x: 362, y: 232, r: 34 },
    { kind: 'circle', btn: 3, x: 432, y: 112, r: 17, label: 'Y', title: 'Y' },
    { kind: 'circle', btn: 1, x: 468, y: 148, r: 17, label: 'B', title: 'B' },
    { kind: 'circle', btn: 0, x: 432, y: 184, r: 17, label: 'A', title: 'A' },
    { kind: 'circle', btn: 2, x: 396, y: 148, r: 17, label: 'X', title: 'X' },
    { kind: 'rect', btn: 8, x: 248, y: 141, w: 26, h: 14, rx: 7, title: 'View' },
    { kind: 'rect', btn: 9, x: 326, y: 141, w: 26, h: 14, rx: 7, title: 'Menu' },
    { kind: 'circle', btn: 16, x: 300, y: 100, r: 19, label: 'X', title: 'Guide' },
    { kind: 'rect', btn: 18, x: 290, y: 168, w: 20, h: 10, rx: 5, title: 'Share' },
    { kind: 'rect', btn: 19, x: 150, y: 330, w: 40, h: 12, rx: 6, title: 'P1', back: true },
    { kind: 'rect', btn: 20, x: 410, y: 330, w: 40, h: 12, rx: 6, title: 'P2', back: true },
  ],
};

const psCommon = (select: string, start: string): Shape[] => [
  ...shoulders('L1', 'R1', 'L2', 'R2', 8),
  { kind: 'touchpad', x: 222, y: 74, w: 156, h: 84 },
  { kind: 'dpad', x: 165, y: 172, size: 64 },
  { kind: 'circle', btn: 3, x: 435, y: 132, r: 16, label: '△', title: 'Triangle' },
  { kind: 'circle', btn: 1, x: 471, y: 168, r: 16, label: '○', title: 'Circle' },
  { kind: 'circle', btn: 0, x: 435, y: 204, r: 16, label: '✕', title: 'Cross' },
  { kind: 'circle', btn: 2, x: 399, y: 168, r: 16, label: '□', title: 'Square' },
  { kind: 'rect', btn: 8, x: 200, y: 88, w: 10, h: 24, rx: 5, title: select },
  { kind: 'rect', btn: 9, x: 390, y: 88, w: 10, h: 24, rx: 5, title: start },
  { kind: 'stick', side: 'left', x: 230, y: 250, r: 33 },
  { kind: 'stick', side: 'right', x: 370, y: 250, r: 33 },
  { kind: 'circle', btn: 16, x: 300, y: 262, r: 13, label: 'PS', title: 'PS' },
];

export const DUALSENSE: LayoutSpec = {
  id: 'dualsense',
  body: PS_BODY,
  shapes: [
    ...psCommon('Share', 'Options'),
    { kind: 'rect', btn: 18, x: 287, y: 284, w: 26, h: 9, rx: 4.5, title: 'Mute' },
    { kind: 'rect', btn: 19, x: 238, y: 296, w: 22, h: 10, rx: 5, title: 'Fn L' },
    { kind: 'rect', btn: 20, x: 340, y: 296, w: 22, h: 10, rx: 5, title: 'Fn R' },
    { kind: 'rect', btn: 21, x: 60, y: 334, w: 44, h: 13, rx: 6, title: 'Back L', back: true },
    { kind: 'rect', btn: 22, x: 496, y: 334, w: 44, h: 13, rx: 6, title: 'Back R', back: true },
  ],
};

export const DUALSHOCK4: LayoutSpec = {
  id: 'dualshock4',
  body: PS_BODY,
  shapes: psCommon('Share', 'Options'),
};

export const SWITCH_PRO: LayoutSpec = {
  id: 'switchPro',
  body: XBOX_BODY,
  shapes: [
    ...shoulders('L', 'R', 'ZL', 'ZR'),
    { kind: 'stick', side: 'left', x: 180, y: 150, r: 34 },
    { kind: 'dpad', x: 238, y: 232, size: 66 },
    { kind: 'stick', side: 'right', x: 362, y: 232, r: 34 },
    { kind: 'circle', btn: 3, x: 432, y: 112, r: 17, label: 'X', title: 'X' },
    { kind: 'circle', btn: 1, x: 468, y: 148, r: 17, label: 'A', title: 'A' },
    { kind: 'circle', btn: 0, x: 432, y: 184, r: 17, label: 'B', title: 'B' },
    { kind: 'circle', btn: 2, x: 396, y: 148, r: 17, label: 'Y', title: 'Y' },
    { kind: 'circle', btn: 8, x: 255, y: 110, r: 10, label: '−', title: 'Minus' },
    { kind: 'circle', btn: 9, x: 345, y: 110, r: 10, label: '+', title: 'Plus' },
    { kind: 'rect', btn: 18, x: 258, y: 142, w: 18, h: 18, rx: 4, title: 'Capture' },
    { kind: 'circle', btn: 16, x: 333, y: 151, r: 11, title: 'Home' },
  ],
};

/** Picks a drawn layout for known families; null means "auto (generic)". */
export function layoutFor(c: ControllerInfo): LayoutSpec | null {
  switch (c.kind) {
    case 'xbox':
      return XBOX;
    case 'ps5':
      return DUALSENSE;
    case 'ps4':
      return DUALSHOCK4;
    case 'switchPro':
      return SWITCH_PRO;
    default:
      return null;
  }
}

/** Buttons that exist on a family (used by the guided test checklist). */
export function buttonsFor(c: ControllerInfo): number[] {
  const spec = layoutFor(c);
  if (!spec) {
    return Array.from({ length: Math.min(17, Math.max(c.buttonCount, 4)) }, (_, i) => i);
  }
  const set = new Set<number>();
  for (const s of spec.shapes) {
    if (s.kind === 'circle' || s.kind === 'rect') set.add(s.btn);
    if (s.kind === 'dpad') [12, 13, 14, 15].forEach((b) => set.add(b));
    if (s.kind === 'stick') set.add(s.side === 'left' ? 10 : 11);
    if (s.kind === 'trigger') set.add(s.side === 'left' ? 6 : 7);
    if (s.kind === 'touchpad') set.add(17);
  }
  // Paddles only on controllers that report them.
  const extras = c.buttonCount >= 23 ? [] : [19, 20, 21, 22];
  if (spec.id === 'xbox' && !c.name.toLowerCase().includes('elite')) extras.push(19, 20);
  return [...set].filter((b) => !extras.includes(b)).sort((a, b) => a - b);
}
