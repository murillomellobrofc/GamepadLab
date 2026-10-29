import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { api } from '@/api/client';
import type { ControllerInfo } from '@/api/types';
import { Icon } from '@/app/icons';
import { useT } from '@/i18n';
import { buttonName, buttonSpoken } from '@/lib/buttonNames';
import { useApp } from '@/state/app';
import { saveSession } from '@/lib/history';
import { SaveToHistory } from '@/app/SaveToHistory';
import { live } from '@/state/live';
import ui from '@/styles/ui.module.css';
import { ControllerDiagram } from './ControllerDiagram';
import css from './GuidedButtonTest.module.css';
import { summarize, type ButtonResult, type ButtonStatus } from './guided';
import { buttonsFor } from './layouts';

export function GuidedButtonTest({
  c,
  onFinished,
}: {
  c: ControllerInfo;
  onFinished?: (results: ButtonResult[]) => void;
}) {
  const t = useT();
  const announce = useApp((s) => s.announce);
  const lockNav = useApp((s) => s.lockNav);
  const order = buttonsFor(c);
  const [phase, setPhase] = useState<'intro' | 'running' | 'done'>('intro');
  const [idx, setIdx] = useState(0);
  const [results, setResults] = useState<ButtonResult[]>([]);
  const [wrong, setWrong] = useState<{ got: number; want: number } | null>(null);
  const [justPassed, setJustPassed] = useState(false);
  const baseline = useRef<number[]>([]);
  const maxValue = useRef(0);
  const current = order[idx];

  const start = () => {
    const entry = live.get(c.id);
    baseline.current = [...(entry?.snapshot.buttonPresses ?? [])];
    setResults(
      order.map((b) => ({
        button: b,
        name: buttonName(c.kind, b),
        status: 'pending',
        maxValue: 0,
      })),
    );
    setIdx(0);
    setWrong(null);
    setPhase('running');
  };

  const finish = (final: ButtonResult[]) => {
    setPhase('done');
    const s = summarize(final);
    announce(t('test.guided.summary', { pass: s.pass, fail: s.fail, skip: s.skip }));
    onFinished?.(final);
    if (c.isDemo) void api.demoScenario('showcase');
  };

  const mark = (status: ButtonStatus) => {
    const next = results.map((r) =>
      r.button === current
        ? { ...r, status, maxValue: status === 'pass' ? maxValue.current : r.maxValue }
        : r,
    );
    setResults(next);
    setWrong(null);
    maxValue.current = 0;
    if (idx + 1 >= order.length) finish(next);
    else setIdx(idx + 1);
  };

  // Pause gamepad UI navigation while the test needs every press.
  useEffect(() => {
    if (phase !== 'running') return;
    return lockNav();
  }, [phase, lockNav]);

  // Announce each prompt and let the demo pad "press" it.
  useEffect(() => {
    if (phase !== 'running' || current === undefined) return;
    announce(t('test.guided.prompt', { button: buttonSpoken(c.kind, current) }));
    if (c.isDemo) {
      const timer = window.setTimeout(() => void api.demoScenario({ pressButton: current }), 500);
      return () => window.clearTimeout(timer);
    }
  }, [phase, current, c.kind, c.isDemo, announce, t]);

  // Watch full-rate press counters for the requested button.
  useEffect(() => {
    if (phase !== 'running' || current === undefined) return;
    let passed = false;
    return live.subscribe(() => {
      const e = live.get(c.id);
      if (!e || passed) return;
      const s = e.snapshot;
      if (current === 6) maxValue.current = Math.max(maxValue.current, s.leftTrigger);
      else if (current === 7) maxValue.current = Math.max(maxValue.current, s.rightTrigger);
      else maxValue.current = Math.max(maxValue.current, s.buttons[current]?.value ?? 0);
      const presses = s.buttonPresses;
      for (let i = 0; i < presses.length; i++) {
        const before = baseline.current[i] ?? 0;
        if (presses[i] > before) {
          baseline.current[i] = presses[i];
          if (i === current) {
            passed = true;
            setJustPassed(true);
            window.setTimeout(() => {
              setJustPassed(false);
              mark('pass');
            }, 320);
            return;
          }
          if (order.includes(i)) setWrong({ got: i, want: current });
        }
      }
    });
    // mark is stable enough for this effect; re-subscribing per prompt is intended
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, current, c.id]);

  useEffect(() => {
    if (phase !== 'running') return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setPhase('intro');
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [phase]);

  if (phase === 'intro') {
    return (
      <section className={ui.panel} aria-labelledby="guided-title">
        <div className={ui.panelHead}>
          <h2 id="guided-title" className={ui.panelTitle}>
            {t('test.guided.title')}
          </h2>
        </div>
        <div className={css.intro}>
          <p>{t('test.guided.intro')}</p>
          <p className={ui.muted}>{t('test.guided.navPaused')}</p>
          {c.isDemo && <p className={ui.faint}>{t('test.guided.demoNote')}</p>}
          <button
            type="button"
            className={`${ui.btn} ${ui.primary}`}
            onClick={start}
            data-testid="guided-start"
          >
            <Icon name="play" size={16} /> {t('test.guided.start')}
          </button>
        </div>
      </section>
    );
  }

  const s = summarize(results);
  return (
    <div className={css.layout}>
      {phase === 'running' && current !== undefined && (
        <section
          className={`${ui.panel} ${css.prompt}`}
          aria-live="polite"
          data-passed={justPassed || undefined}
        >
          <span className={css.progress}>
            {t('test.guided.progress', { n: idx + 1, total: order.length })}
          </span>
          <div
            className={css.glyph}
            aria-hidden="true"
            // Shrinks long names ("Options", "Touchpad") so they stay inside the circle.
            style={{ '--chars': buttonName(c.kind, current).length } as CSSProperties}
          >
            {justPassed ? <Icon name="check" size={56} /> : buttonName(c.kind, current)}
          </div>
          <p className={css.ask} data-testid="guided-prompt">
            {t('test.guided.prompt', { button: buttonSpoken(c.kind, current) })}
          </p>
          <p className={ui.faint}>{t('test.guided.promptHint')}</p>
          {wrong && (
            <p className={css.wrong} role="status">
              {t('test.guided.wrong', {
                got: buttonName(c.kind, wrong.got),
                want: buttonName(c.kind, wrong.want),
              })}
            </p>
          )}
          <div className={css.actions}>
            <button type="button" className={ui.btn} onClick={() => mark('skip')}>
              {t('test.guided.skip')}
            </button>
            <button type="button" className={`${ui.btn} ${ui.danger}`} onClick={() => mark('fail')}>
              {t('test.guided.fail')}
            </button>
            <button
              type="button"
              className={`${ui.btn} ${ui.ghost}`}
              onClick={() => setPhase('intro')}
            >
              {t('test.guided.exit')}
            </button>
          </div>
        </section>
      )}
      {phase === 'done' && (
        <section className={`${ui.panel} ${css.prompt}`} data-testid="guided-result">
          <h2 className={ui.panelTitle}>{t('test.guided.resultTitle')}</h2>
          <p className={css.summary}>
            {t('test.guided.summary', { pass: s.pass, fail: s.fail, skip: s.skip })}
          </p>
          <p className={s.fail ? css.bad : css.good}>
            {s.fail
              ? t('test.guided.failedList', { list: s.failed.join(', ') })
              : t('test.guided.allGood')}
          </p>
          <div className={css.actions}>
            <button type="button" className={`${ui.btn} ${ui.primary}`} onClick={start}>
              <Icon name="refresh" size={16} /> {t('test.guided.restart')}
            </button>
            <SaveToHistory
              label={t('test.guided.save')}
              onSave={() =>
                saveSession(
                  c,
                  'buttons',
                  results.length ? Math.round((s.pass / results.length) * 100) : null,
                  t('test.guided.summary', { pass: s.pass, fail: s.fail, skip: s.skip }),
                  { results },
                )
              }
            />
          </div>
        </section>
      )}
      <section className={`${ui.panel} ${css.diagram}`}>
        <ControllerDiagram controller={c} testHighlight={phase === 'running' ? current : null} />
      </section>
      <section className={`${ui.panel} ${css.checklist}`} aria-label={t('test.guided.resultTitle')}>
        <ul>
          {results.map((r, i) => (
            <li
              key={r.button}
              data-status={r.status}
              data-current={phase === 'running' && i === idx ? true : undefined}
            >
              <span className={css.name}>{r.name}</span>
              <span className={css.status}>
                {r.status === 'pass' && <Icon name="check" size={14} />}
                {r.status === 'fail' && <Icon name="x" size={14} />}
                {t(
                  `test.guided.${r.status === 'pass' ? 'pass' : r.status === 'fail' ? 'failed' : r.status === 'skip' ? 'skipped' : 'pending'}`,
                )}
                {(r.button === 6 || r.button === 7) && r.status === 'pass' && (
                  <em> · {t('test.guided.travel', { pct: `${Math.round(r.maxValue * 100)}%` })}</em>
                )}
              </span>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
