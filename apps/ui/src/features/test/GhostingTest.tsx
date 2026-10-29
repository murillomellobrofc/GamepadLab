import { useEffect, useRef, useState } from 'react';
import { api } from '@/api/client';
import type { ControllerInfo } from '@/api/types';
import { Icon } from '@/app/icons';
import { useT, type TKey } from '@/i18n';
import { buttonName } from '@/lib/buttonNames';
import { useApp } from '@/state/app';
import { live } from '@/state/live';
import ui from '@/styles/ui.module.css';
import css from './GhostingTest.module.css';
import { analyzeChord, type GhostResult } from './ghosting';
import { buttonsFor } from './layouts';

const PRESETS: Record<string, number[]> = {
  face: [0, 1, 2, 3],
  shoulders: [4, 5, 6, 7],
  faceShoulders: [0, 1, 2, 3, 4, 5],
  dpad: [12, 13, 14, 15],
};

export function GhostingTest({ c }: { c: ControllerInfo }) {
  const t = useT();
  const lockNav = useApp((s) => s.lockNav);
  const available = buttonsFor(c);
  const [preset, setPreset] = useState<string>('face');
  const [targets, setTargets] = useState<number[]>(PRESETS.face);
  const [phase, setPhase] = useState<'setup' | 'running' | 'done'>('setup');
  const [result, setResult] = useState<GhostResult | null>(null);
  const frames = useRef<number[][]>([]);
  const [held, setHeld] = useState<number[]>([]);

  const choosePreset = (p: string) => {
    setPreset(p);
    setTargets(p === 'all' ? available : PRESETS[p].filter((b) => available.includes(b)));
  };

  const finish = () => {
    setResult(analyzeChord(targets, frames.current));
    setPhase('done');
    if (c.isDemo) void api.demoScenario('showcase');
  };

  const start = () => {
    frames.current = [];
    setResult(null);
    setPhase('running');
    if (c.isDemo) void api.demoScenario({ chord: targets });
  };

  useEffect(() => {
    if (phase !== 'running') return;
    const release = lockNav();
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && setPhase('setup');
    window.addEventListener('keydown', onKey);
    const auto = c.isDemo ? window.setTimeout(finish, 2200) : 0;
    return () => {
      release();
      window.removeEventListener('keydown', onKey);
      window.clearTimeout(auto);
    };
    // finish reads the latest refs; only (re)arm when the phase changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase]);

  // Every backend frame counts (not only animation frames).
  useEffect(() => {
    if (phase !== 'running') return;
    return live.subscribe(() => {
      const e = live.get(c.id);
      if (!e) return;
      const pressed = e.snapshot.buttons.flatMap((b, i) => (b.pressed ? [i] : []));
      frames.current.push(pressed);
      if (frames.current.length > 20000) frames.current.shift();
      const now = pressed.filter((b) => targets.includes(b));
      setHeld((old) => (now.join() !== old.join() ? now : old));
    });
  }, [phase, c.id, targets]);

  return (
    <section className={ui.panel} aria-labelledby="ghost-title">
      <div className={ui.panelHead}>
        <h2 id="ghost-title" className={ui.panelTitle}>
          {t('test.ghosting.title')}
        </h2>
      </div>
      <p className={css.intro}>{t('test.ghosting.intro')}</p>
      {c.isDemo && <p className={ui.faint}>{t('test.ghosting.demoNote')}</p>}

      {phase === 'setup' && (
        <div className={css.setup}>
          <div className={css.presets} role="radiogroup" aria-label={t('test.ghosting.preset')}>
            {['face', 'shoulders', 'faceShoulders', 'dpad', 'all'].map((p) => (
              <button
                key={p}
                type="button"
                role="radio"
                aria-checked={preset === p}
                className={`${ui.btn} ${ui.small}`}
                onClick={() => choosePreset(p)}
              >
                {t(`test.ghosting.presets.${p}` as TKey)}
              </button>
            ))}
          </div>
          <fieldset className={css.choose}>
            <legend>{t('test.ghosting.choose')}</legend>
            {available.map((b) => (
              <label key={b}>
                <input
                  type="checkbox"
                  checked={targets.includes(b)}
                  onChange={(e) =>
                    setTargets((ts) =>
                      e.target.checked
                        ? [...ts, b].sort((x, y) => x - y)
                        : ts.filter((x) => x !== b),
                    )
                  }
                />
                {buttonName(c.kind, b)}
              </label>
            ))}
          </fieldset>
          <button
            type="button"
            className={`${ui.btn} ${ui.primary}`}
            disabled={targets.length < 2}
            onClick={start}
            data-testid="ghost-start"
          >
            <Icon name="play" size={16} /> {t('test.ghosting.start')}
          </button>
        </div>
      )}

      {phase === 'running' && (
        <div className={css.running}>
          <p className={css.holdLabel}>{t('test.ghosting.hold')}</p>
          <ul className={css.keys} aria-live="polite">
            {targets.map((b) => (
              <li key={b} data-on={held.includes(b) || undefined}>
                {buttonName(c.kind, b)}
              </li>
            ))}
          </ul>
          <p className={`mono ${css.count}`}>
            {held.length}/{targets.length}
          </p>
          <button type="button" className={`${ui.btn} ${ui.primary}`} onClick={finish}>
            {t('test.ghosting.finish')}
          </button>
        </div>
      )}

      {phase === 'done' && result && (
        <div className={css.result} data-testid="ghost-result">
          <div className={ui.stat}>
            <span className={ui.statLabel}>{t('test.ghosting.rollover')}</span>
            <span className={ui.statValue}>
              {result.rollover}
              <span className={ui.statUnit}>/ {result.targets.length}</span>
            </span>
            <span className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
              {t('test.ghosting.rolloverHint')}
            </span>
          </div>
          <ul className={css.keys}>
            {result.targets.map((b) => {
              const status = result.never.includes(b)
                ? 'never'
                : result.suspects.includes(b)
                  ? 'suspect'
                  : 'ok';
              return (
                <li key={b} data-status={status}>
                  {buttonName(c.kind, b)}
                  <small>
                    {status === 'never'
                      ? t('test.ghosting.never')
                      : status === 'suspect'
                        ? t('test.ghosting.registered')
                        : t('test.ghosting.together')}
                  </small>
                </li>
              );
            })}
          </ul>
          <p className={result.suspects.length || result.never.length ? css.bad : css.good}>
            {result.suspects.length || result.never.length
              ? t('test.ghosting.ghostFound', {
                  list: [...new Set([...result.suspects, ...result.never])]
                    .map((b) => buttonName(c.kind, b))
                    .join(', '),
                })
              : t('test.ghosting.noGhost')}
          </p>
          <button type="button" className={ui.btn} onClick={() => setPhase('setup')}>
            <Icon name="refresh" size={16} /> {t('test.guided.restart')}
          </button>
        </div>
      )}
    </section>
  );
}
