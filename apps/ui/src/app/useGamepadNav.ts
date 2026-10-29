import { useEffect } from 'react';
import { translate } from '@/i18n';
import { pickNext, type Dir } from '@/lib/spatial';
import { TABS, useApp } from '@/state/app';
import { live } from '@/state/live';
import { useSettings } from '@/state/settings';

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
const REPEAT_DELAY = 420;
const REPEAT_EVERY = 140;
const TOGGLE_HOLD_MS = 800;

function visible(el: HTMLElement) {
  const r = el.getBoundingClientRect();
  return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
}

function scope(): ParentNode {
  return document.querySelector('dialog[open]') ?? document;
}

function move(dir: Dir) {
  const active = document.activeElement as HTMLElement | null;
  if (
    active instanceof HTMLInputElement &&
    active.type === 'range' &&
    (dir === 'left' || dir === 'right')
  ) {
    const step = Number(active.step) || 1;
    const next = Number(active.value) + (dir === 'right' ? step : -step);
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    setter?.call(active, String(Math.min(Number(active.max), Math.max(Number(active.min), next))));
    active.dispatchEvent(new Event('input', { bubbles: true }));
    return;
  }
  const all = [...scope().querySelectorAll<HTMLElement>(FOCUSABLE)].filter(visible);
  if (!all.length) return;
  if (!active || active === document.body || !all.includes(active)) {
    all[0].focus();
    return;
  }
  const rect = (e: HTMLElement) => {
    const r = e.getBoundingClientRect();
    return { x: r.left, y: r.top, w: r.width, h: r.height };
  };
  const others = all.filter((e) => e !== active);
  const i = pickNext(rect(active), others.map(rect), dir);
  if (i >= 0) {
    others[i].focus();
    others[i].scrollIntoView({ block: 'nearest', inline: 'nearest' });
  }
}

function activate() {
  const el = document.activeElement as HTMLElement | null;
  if (el && el !== document.body) el.click();
}

function back() {
  const dialog = document.querySelector<HTMLDialogElement>('dialog[open]');
  if (dialog) dialog.close();
  else (document.activeElement as HTMLElement | null)?.blur();
}

/**
 * Lets people drive the whole app with the controller. Toggled with the
 * Guide/PS/Home button (or Select+Start held), paused while a test needs
 * every press.
 */
export function useGamepadNav() {
  useEffect(() => {
    const state = new Map<string, { down: boolean; next: number }>();
    let comboSince = 0;
    // Guide press count last seen per controller (first frame only syncs).
    const lastGuide = new Map<string, number>();
    const root = document.documentElement;

    const edge = (key: string, pressed: boolean, now: number, repeat: boolean): boolean => {
      const s = state.get(key) ?? { down: false, next: 0 };
      let fire = false;
      if (pressed && !s.down) {
        fire = true;
        s.next = now + REPEAT_DELAY;
      } else if (pressed && repeat && now >= s.next) {
        fire = true;
        s.next = now + REPEAT_EVERY;
      }
      s.down = pressed;
      state.set(key, s);
      return fire;
    };

    const setOn = (on: boolean) => {
      const app = useApp.getState();
      app.setGamepadNav(on);
      if (on) root.dataset.gpnav = 'on';
      else delete root.dataset.gpnav;
      app.announce(
        translate(
          useSettings.getState().locale,
          on ? 'a11y.gamepadNavOn' : 'a11y.gamepadNavPaused',
        ),
      );
    };

    const unsub = live.subscribe(() => {
      if (!useSettings.getState().gamepadNav) return;
      const app = useApp.getState();
      const e = live.get(app.selectedId);
      if (!e) return;
      const b = e.snapshot.buttons;
      const now = performance.now();

      // Toggle: Guide press, or Select+Start held.
      const guidePresses = e.snapshot.buttonPresses[16] ?? 0;
      const seen = lastGuide.get(e.snapshot.id);
      lastGuide.set(e.snapshot.id, guidePresses);
      if (seen !== undefined && guidePresses > seen && app.navLocks === 0) setOn(!app.gamepadNav);
      if (b[8]?.pressed && b[9]?.pressed) {
        if (!comboSince) comboSince = now;
        else if (now - comboSince > TOGGLE_HOLD_MS && app.navLocks === 0) {
          comboSince = Infinity;
          setOn(!app.gamepadNav);
        }
      } else comboSince = 0;

      if (!app.gamepadNav || app.navLocks > 0) {
        state.clear();
        return;
      }
      const s = e.snapshot.leftStick;
      const dirs: [Dir, boolean][] = [
        ['up', !!b[12]?.pressed || s.y > 0.6],
        ['down', !!b[13]?.pressed || s.y < -0.6],
        ['left', !!b[14]?.pressed || s.x < -0.6],
        ['right', !!b[15]?.pressed || s.x > 0.6],
      ];
      for (const [d, pressed] of dirs) if (edge(d, pressed, now, true)) move(d);
      if (edge('a', !!b[0]?.pressed, now, false)) activate();
      if (edge('b', !!b[1]?.pressed, now, false)) back();
      if (edge('lb', !!b[4]?.pressed, now, false) || edge('rb', !!b[5]?.pressed, now, false)) {
        const i = TABS.indexOf(app.tab);
        const delta = b[5]?.pressed ? 1 : -1;
        app.setTab(TABS[(i + delta + TABS.length) % TABS.length]);
      }
    });
    const onMouse = () => delete root.dataset.gpnav;
    window.addEventListener('mousemove', onMouse);
    return () => {
      unsub();
      window.removeEventListener('mousemove', onMouse);
    };
  }, []);
}
