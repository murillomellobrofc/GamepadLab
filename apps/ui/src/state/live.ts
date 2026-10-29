import { useEffect, useRef, useState } from 'react';
import type { FrameEntry, UiFrame } from '@/api/types';

type Listener = (frame: UiFrame) => void;

/**
 * High-rate frame bus that lives outside React. Backend frames (60–120 Hz)
 * land here; canvases read it inside requestAnimationFrame and text widgets
 * subscribe through `useLive` with their own (lower) refresh rate, so the
 * React tree never re-renders at input rate.
 */
class LiveBus {
  private latest = new Map<string, FrameEntry>();
  private listeners = new Set<Listener>();
  framesReceived = 0;

  push(frame: UiFrame) {
    this.framesReceived++;
    for (const entry of frame.entries) this.latest.set(entry.snapshot.id, entry);
    for (const l of this.listeners) l(frame);
  }

  get(id: string | null | undefined): FrameEntry | undefined {
    return id ? this.latest.get(id) : undefined;
  }

  drop(id: string) {
    this.latest.delete(id);
  }

  clear() {
    this.latest.clear();
  }

  subscribe(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }
}

export const live = new LiveBus();

/**
 * Reads a derived value from the latest frame of a controller, re-rendering
 * at most `hz` times per second and only when the value changes.
 */
export function useLive<T>(
  id: string | null | undefined,
  select: (entry: FrameEntry) => T,
  hz = 15,
  equal: (a: T, b: T) => boolean = Object.is,
): T | undefined {
  const selectRef = useRef(select);
  selectRef.current = select;
  const [value, setValue] = useState<T | undefined>(() => {
    const e = live.get(id);
    return e ? select(e) : undefined;
  });
  useEffect(() => {
    let last = 0;
    let current: T | undefined;
    const minGap = 1000 / hz;
    const read = () => {
      const e = live.get(id);
      const next = e ? selectRef.current(e) : undefined;
      if (current === undefined || next === undefined || !equal(current, next)) {
        current = next;
        setValue(next);
      }
    };
    read();
    return live.subscribe(() => {
      const now = performance.now();
      if (now - last < minGap) return;
      last = now;
      read();
    });
  }, [id, hz, equal]);
  return value;
}

/** Runs `draw` on every animation frame while mounted and the tab is visible. */
export function useAnimationFrame(draw: (t: number) => void, active = true) {
  const drawRef = useRef(draw);
  drawRef.current = draw;
  useEffect(() => {
    if (!active) return;
    let raf = 0;
    const loop = (t: number) => {
      drawRef.current(t);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [active]);
}
