import type { ControllerInfo } from '@/api/types';

/**
 * Which drawing each controller family uses and the ID of every part in it.
 * The drawings are static files in public/controllers (licences in
 * ASSETS_LICENSE.md); the frontend only knows them by these IDs:
 *
 * - `btn-*`, `dpad-*`, `touchpad`: lit while pressed (`data-on`);
 * - `stick-left` / `stick-right`: moved by the stick, lit on L3/R3; their
 *   `*-base` holds the ring the cap travels in (radius from its circle);
 * - `trigger-*`: lit past the threshold; `*-fill` (a rect) grows with the
 *   value and `*-value` shows the percentage;
 * - `touch-0` / `touch-1`: placed inside `touchpad` for each finger.
 */
export type ArtFamily = 'dualsense' | 'xbox' | 'switchPro' | 'generic';

export interface ArtSpec {
  family: ArtFamily;
  url: string;
  /** Canonical button index → part ID. */
  ids: Readonly<Record<number, string>>;
}

const STICKS_AND_DPAD = {
  10: 'stick-left',
  11: 'stick-right',
  12: 'dpad-up',
  13: 'dpad-down',
  14: 'dpad-left',
  15: 'dpad-right',
};

const DUALSENSE: ArtSpec = {
  family: 'dualsense',
  url: '/controllers/ps5/dualsense.svg',
  ids: {
    0: 'btn-cross',
    1: 'btn-circle',
    2: 'btn-square',
    3: 'btn-triangle',
    4: 'btn-l1',
    5: 'btn-r1',
    6: 'trigger-l2',
    7: 'trigger-r2',
    8: 'btn-share',
    9: 'btn-options',
    ...STICKS_AND_DPAD,
    16: 'btn-ps',
    17: 'touchpad',
    18: 'btn-mute',
    19: 'btn-fn-left',
    20: 'btn-fn-right',
    21: 'btn-back-left',
    22: 'btn-back-right',
  },
};

const XBOX: ArtSpec = {
  family: 'xbox',
  url: '/controllers/xbox/xbox.svg',
  ids: {
    0: 'btn-a',
    1: 'btn-b',
    2: 'btn-x',
    3: 'btn-y',
    4: 'btn-lb',
    5: 'btn-rb',
    6: 'trigger-lt',
    7: 'trigger-rt',
    8: 'btn-view',
    9: 'btn-menu',
    ...STICKS_AND_DPAD,
    16: 'btn-guide',
    18: 'btn-share',
    19: 'btn-p1',
    20: 'btn-p2',
  },
};

// Positional, like the input layer: South is B and East is A on Nintendo pads.
const SWITCH_PRO: ArtSpec = {
  family: 'switchPro',
  url: '/controllers/switch/switch-pro.svg',
  ids: {
    0: 'btn-b',
    1: 'btn-a',
    2: 'btn-y',
    3: 'btn-x',
    4: 'btn-l',
    5: 'btn-r',
    6: 'trigger-zl',
    7: 'trigger-zr',
    8: 'btn-minus',
    9: 'btn-plus',
    ...STICKS_AND_DPAD,
    16: 'btn-home',
    18: 'btn-capture',
  },
};

const GENERIC: ArtSpec = {
  family: 'generic',
  url: '/controllers/gamepadlab/generic.svg',
  ids: {
    0: 'btn-south',
    1: 'btn-east',
    2: 'btn-west',
    3: 'btn-north',
    4: 'btn-l1',
    5: 'btn-r1',
    6: 'trigger-l2',
    7: 'trigger-r2',
    8: 'btn-select',
    9: 'btn-start',
    ...STICKS_AND_DPAD,
    16: 'btn-home',
  },
};

export const ART_SPECS: readonly ArtSpec[] = [DUALSENSE, XBOX, SWITCH_PRO, GENERIC];

/** The drawing for a controller. The DualShock 4 uses the DualSense drawing. */
export function artFor(c: Pick<ControllerInfo, 'kind'>): ArtSpec {
  switch (c.kind) {
    case 'ps5':
    case 'ps4':
      return DUALSENSE;
    case 'xbox':
      return XBOX;
    case 'switchPro':
      return SWITCH_PRO;
    default:
      return GENERIC;
  }
}

/** Buttons that exist on this controller (the guided test checklist; the rest are hidden in the drawing). */
export function buttonsFor(c: ControllerInfo): number[] {
  const all = Object.keys(artFor(c).ids).map(Number);
  const missing = new Set<number>();
  if (c.kind === 'generic') {
    all.filter((b) => b >= Math.max(c.buttonCount, 4)).forEach((b) => missing.add(b));
  }
  if (c.kind === 'ps4') [18, 19, 20, 21, 22].forEach((b) => missing.add(b));
  // DualSense Edge paddles only when the controller reports them.
  if (c.kind === 'ps5' && c.buttonCount < 23) [19, 20, 21, 22].forEach((b) => missing.add(b));
  if (c.kind === 'xbox' && !c.name.toLowerCase().includes('elite'))
    [19, 20].forEach((b) => missing.add(b));
  return all.filter((b) => !missing.has(b)).sort((a, b) => a - b);
}
