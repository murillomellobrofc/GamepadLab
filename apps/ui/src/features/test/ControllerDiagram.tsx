import { useEffect, useMemo, useRef, useState } from 'react';
import type { ControllerInfo, FrameEntry } from '@/api/types';
import { useT } from '@/i18n';
import { buttonName } from '@/lib/buttonNames';
import { useApp } from '@/state/app';
import { live, useAnimationFrame } from '@/state/live';
import { bindArt, type ArtBinding, type ArtValues } from './art/bindArt';
import { loadArt } from './art/loadArt';
import css from './ControllerDiagram.module.css';
import { artFor, buttonsFor } from './layouts';

function pick(entry: FrameEntry, calibrated: boolean): ArtValues {
  const c = calibrated ? entry.calibrated : null;
  return {
    buttons: c?.buttons ?? entry.snapshot.buttons,
    left: c?.leftStick ?? entry.snapshot.leftStick,
    right: c?.rightStick ?? entry.snapshot.rightStick,
    lt: c?.leftTrigger ?? entry.snapshot.leftTrigger,
    rt: c?.rightTrigger ?? entry.snapshot.rightTrigger,
  };
}

/**
 * Live drawing of a controller: the family's SVG (public/controllers) with
 * its parts lit, moved and filled by ID every animation frame.
 */
export function ControllerDiagram({
  controller,
  forceCalibrated,
  testHighlight,
}: {
  controller: ControllerInfo;
  forceCalibrated?: boolean;
  /** Button the guided test is asking for: drawn with a moving ring. */
  testHighlight?: number | null;
}) {
  const t = useT();
  const spec = artFor(controller);
  const host = useRef<HTMLDivElement>(null);
  const binding = useRef<ArtBinding | null>(null);
  const [ready, setReady] = useState(false);
  const [failed, setFailed] = useState(false);
  const viewMode = useApp((s) => s.viewMode);
  const rumble = useApp((s) => s.rumble[controller.id]);
  const calibrated = forceCalibrated ?? viewMode === 'calibrated';
  const { kind, buttonCount, name } = controller;
  const present = useMemo(
    () => buttonsFor(controller),
    // only what decides the button list
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [kind, buttonCount, name],
  );

  useEffect(() => {
    let alive = true;
    setReady(false);
    setFailed(false);
    loadArt(spec.url)
      .then((template) => {
        if (!alive || !host.current) return;
        const svg = template.cloneNode(true) as SVGSVGElement;
        svg.setAttribute('class', css.svg);
        svg.setAttribute('aria-hidden', 'true');
        svg.setAttribute('focusable', 'false');
        host.current.replaceChildren(svg);
        binding.current = bindArt(svg, spec, present, (b) => buttonName(kind, b), css.ring);
        setReady(true);
      })
      .catch((e: unknown) => {
        console.warn('controller drawing unavailable', e);
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
      binding.current = null;
    };
  }, [spec, present, kind]);

  useEffect(() => {
    if (ready) binding.current?.highlight(testHighlight ?? null);
  }, [ready, testHighlight]);

  useAnimationFrame(() => {
    const entry = live.get(controller.id);
    if (binding.current && entry)
      binding.current.update(pick(entry, calibrated), entry.snapshot.touch);
  });

  const vibrating = !!rumble?.active && (rumble.weak > 0.02 || rumble.strong > 0.02);
  return (
    <div
      ref={host}
      className={css.art}
      role="img"
      aria-label={controller.name}
      data-family={spec.family}
      data-vibrating={vibrating || undefined}
      data-failed={failed || undefined}
      title={failed ? t('test.drawingUnavailable') : undefined}
      style={{ ['--rumble' as string]: String(Math.max(rumble?.weak ?? 0, rumble?.strong ?? 0)) }}
    />
  );
}
