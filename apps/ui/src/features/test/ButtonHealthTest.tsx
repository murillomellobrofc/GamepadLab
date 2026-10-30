import { useEffect, useRef, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type {
  ButtonHealth,
  ButtonPlan,
  ControllerInfo,
  HealthGrade,
  HealthPrecision,
  HealthProgress,
  HealthReport,
} from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { Segmented, Switch } from '@/app/controls';
import { Icon, type IconName } from '@/app/icons';
import { SaveToHistory } from '@/app/SaveToHistory';
import { useT, type TFn, type TKey } from '@/i18n';
import { buttonName, buttonSpoken } from '@/lib/buttonNames';
import { saveSession } from '@/lib/history';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import css from './ButtonHealthTest.module.css';
import {
  DEFAULT_PRESSES,
  HOLD_TARGET_US,
  PRESS_OPTIONS,
  QUICK_PICKS,
  demoTapsDurationMs,
  gradeCounts,
  healthScore,
  isEventBased,
} from './buttonHealth';
import { ControllerDiagram } from './ControllerDiagram';
import { buttonsFor } from './layouts';

type Phase = 'setup' | 'taps' | 'hold' | 'heldEarly' | 'analyzing' | 'done';
const RUNNING: Phase[] = ['taps', 'hold', 'heldEarly', 'analyzing'];
const POLL_MS = 120;
const GRADE_ICON: Record<HealthGrade, IconName> = { good: 'check', warning: 'alert', bad: 'x' };

function precisionNote(t: TFn, p: HealthPrecision): string {
  return p.kind === 'report'
    ? t('test.health.precisionReport', { ms: Math.max(1, Math.round(p.intervalUs / 1000)) })
    : t('test.health.precisionEvent');
}

export function ButtonHealthTest({ c, initial }: { c: ControllerInfo; initial?: number[] }) {
  const t = useT();
  const announce = useApp((s) => s.announce);
  const lockNav = useApp((s) => s.lockNav);
  const toast = useApp((s) => s.toast);
  const available = buttonsFor(c);
  const [selected, setSelected] = useState<number[]>(() =>
    (initial ?? []).filter((b) => available.includes(b)),
  );
  const [presses, setPresses] = useState<number>(DEFAULT_PRESSES);
  const [withHold, setWithHold] = useState(true);
  const [phase, setPhase] = useState<Phase>('setup');
  const [queue, setQueue] = useState<number[]>([]);
  const [idx, setIdx] = useState(0);
  const [base, setBase] = useState({ taps: 0, presses: 0 });
  const [progress, setProgress] = useState<HealthProgress | null>(null);
  const [report, setReport] = useState<HealthReport | null>(null);
  const [canRelease, setCanRelease] = useState(false);
  const plans = useRef(new Map<number, ButtonPlan>());
  // Hold step bookkeeping, only touched from effects and handlers.
  const hold = useRef({ started: false, reached: false, releasedPolls: 0 });
  const phaseRef = useRef(phase);
  useEffect(() => {
    phaseRef.current = phase;
  }, [phase]);

  const current = queue[idx];
  const liveOf = (b: number | undefined) => progress?.buttons.find((x) => x.button === b);
  const live = liveOf(current);
  const roundTaps = Math.max(0, (live?.taps ?? 0) - base.taps);
  const heldFor = live?.held ? live.heldForUs : 0;
  const holdPct = Math.round(Math.min(1, heldFor / HOLD_TARGET_US) * 100);
  const name = (b: number) => buttonName(c.kind, b);

  /** `fresh`: first button of a new session, so earlier counts do not apply. */
  const beginTaps = (b: number, fresh = false) => {
    const l = fresh ? undefined : liveOf(b);
    setBase({ taps: l?.taps ?? 0, presses: l?.presses ?? 0 });
    setPhase('taps');
    announce(t('test.health.tapPrompt', { button: buttonSpoken(c.kind, b), n: presses }));
    if (c.isDemo) void api.demoScenario({ healthTaps: { button: b, presses } });
  };

  const beginHold = (b: number) => {
    hold.current = { started: false, reached: false, releasedPolls: 0 };
    setCanRelease(false);
    setPhase('hold');
    announce(t('test.health.holdPrompt', { button: buttonSpoken(c.kind, b) }));
    if (c.isDemo) void api.demoScenario({ healthHold: b });
  };

  const finish = async () => {
    const list = [...plans.current.values()];
    if (c.isDemo) void api.demoScenario('showcase');
    if (!list.length) {
      void api.buttonHealthCancel();
      setPhase('setup');
      return;
    }
    setPhase('analyzing');
    try {
      const r = await api.buttonHealthStop(list);
      setReport(r);
      setPhase('done');
      announce(t('test.health.summary', gradeCounts(r)));
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
      setPhase('setup');
    }
  };

  const advance = () => {
    if (idx + 1 < queue.length) {
      setIdx(idx + 1);
      beginTaps(queue[idx + 1]);
    } else {
      void finish();
    }
  };

  const finishTaps = () => {
    if (current === undefined) return;
    // Taps that arrived before this round (an accidental press earlier) are
    // not counted against it.
    plans.current.set(current, {
      button: current,
      requested: presses + base.taps,
      reportedSelfReleases: 0,
    });
    if (withHold) beginHold(current);
    else advance();
  };

  const start = async () => {
    const order = available.filter((b) => selected.includes(b));
    if (!order.length) return;
    plans.current = new Map();
    setReport(null);
    setProgress(null);
    try {
      await api.buttonHealthStart(c.id);
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
      return;
    }
    setQueue(order);
    setIdx(0);
    beginTaps(order[0], true);
  };

  const cancel = () => {
    void api.buttonHealthCancel();
    if (c.isDemo) void api.demoScenario('showcase');
    setPhase('setup');
  };

  const reportSelfRelease = () => {
    const p = plans.current.get(current);
    if (p) plans.current.set(current, { ...p, reportedSelfReleases: p.reportedSelfReleases + 1 });
    advance();
  };

  // The controller is busy with the test: pause gamepad navigation; Esc cancels.
  const running = RUNNING.includes(phase);
  useEffect(() => {
    if (!running) return;
    const release = lockNav();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => {
      release();
      window.removeEventListener('keydown', onKey);
    };
    // cancel only touches stable setters and the API
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [running]);

  // Leaving the screen mid-test drops the recording.
  useEffect(
    () => () => {
      if (RUNNING.includes(phaseRef.current)) void api.buttonHealthCancel();
    },
    [],
  );

  // Live counts from the backend while a round runs.
  const polling = phase === 'taps' || phase === 'hold' || phase === 'heldEarly';
  useEffect(() => {
    if (!polling) return;
    let busy = false;
    const timer = window.setInterval(async () => {
      if (busy) return;
      busy = true;
      try {
        setProgress(await api.buttonHealthProgress());
      } catch {
        /* session ended underneath us: the next phase handles it */
      } finally {
        busy = false;
      }
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [polling]);

  // Tap round: done once the requested taps arrived (or the user says so).
  useEffect(() => {
    if (phase === 'taps' && roundTaps >= presses) finishTaps();
    // finishTaps reads the current render; re-run only on new counts
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, roundTaps, presses]);

  // Demo: the simulated pad taps on its own; finish when its round is over.
  useEffect(() => {
    if (phase !== 'taps' || !c.isDemo) return;
    const timer = window.setTimeout(finishTaps, demoTapsDurationMs(presses) + 700);
    return () => window.clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase, idx]);

  // Hold step: fill the bar while held; a release before it fills asks the user.
  useEffect(() => {
    if (phase !== 'hold' || current === undefined) return;
    const h = hold.current;
    if (live?.held && heldFor > 150_000) h.started = true;
    if (heldFor >= HOLD_TARGET_US && !h.reached) {
      h.reached = true;
      setCanRelease(true);
      announce(t('test.health.holdRelease'));
    }
    // Two polls in a row without the button: a real release, not a dropout.
    h.releasedPolls = live?.held ? 0 : h.releasedPolls + 1;
    if (h.releasedPolls >= 2) {
      if (h.reached) advance();
      else if (h.started) {
        setPhase('heldEarly');
        announce(t('test.health.heldEarly'));
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [progress]);

  if (phase === 'setup') {
    const lowPrecision = isEventBased(c);
    return (
      <section className={ui.panel} aria-labelledby="health-title">
        <div className={ui.panelHead}>
          <h2 id="health-title" className={ui.panelTitle}>
            {t('test.health.title')}
          </h2>
        </div>
        <div className={css.setup}>
          <p className={css.text}>{t('test.health.intro')}</p>
          <p
            className={lowPrecision ? css.warn : css.note}
            role={lowPrecision ? 'alert' : undefined}
            data-testid="health-precision"
          >
            <Icon name={lowPrecision ? 'alert' : 'info'} size={16} />
            {lowPrecision
              ? t('test.health.precisionSystem', { backend: c.backend })
              : t('test.health.precisionHid')}
          </p>
          {c.isDemo && <p className={ui.faint}>{t('test.health.demoNote')}</p>}
          {!!initial?.length && <p className={ui.muted}>{t('test.health.fromGuided')}</p>}

          <div className={css.picks} role="group" aria-label={t('test.health.quick.label')}>
            {(['shoulders', 'face'] as const).map((p) => (
              <button
                key={p}
                type="button"
                className={`${ui.btn} ${ui.small}`}
                onClick={() => setSelected(QUICK_PICKS[p].filter((b) => available.includes(b)))}
              >
                {t(`test.health.quick.${p}`)}
              </button>
            ))}
            <button
              type="button"
              className={`${ui.btn} ${ui.small}`}
              onClick={() => setSelected(available)}
            >
              {t('test.health.quick.all')}
            </button>
            <button
              type="button"
              className={`${ui.btn} ${ui.small}`}
              onClick={() => setSelected([])}
            >
              {t('test.health.quick.none')}
            </button>
          </div>
          <fieldset className={css.choose}>
            <legend>{t('test.health.choose')}</legend>
            {available.map((b) => (
              <label key={b}>
                <input
                  type="checkbox"
                  checked={selected.includes(b)}
                  onChange={(e) =>
                    setSelected((s) => (e.target.checked ? [...s, b] : s.filter((x) => x !== b)))
                  }
                />
                {name(b)}
              </label>
            ))}
          </fieldset>
          <p className={ui.faint}>{t('test.health.chooseHint')}</p>
          <Segmented<number>
            label={t('test.health.presses')}
            value={presses}
            onChange={setPresses}
            options={PRESS_OPTIONS.map((n) => ({ value: n, label: String(n) }))}
          />
          <Switch
            label={t('test.health.includeHold')}
            hint={t('test.health.includeHoldHint')}
            checked={withHold}
            onChange={setWithHold}
          />
          <p className={ui.muted}>{t('test.health.navPaused')}</p>
          <button
            type="button"
            className={`${ui.btn} ${ui.primary}`}
            disabled={!selected.length}
            onClick={() => void start()}
            data-testid="health-start"
          >
            <Icon name="play" size={16} /> {t('test.health.start')}
          </button>
        </div>
      </section>
    );
  }

  if (phase === 'done' && report) {
    const counts = gradeCounts(report);
    const summary = t('test.health.summary', counts);
    return (
      <section className={ui.panel} aria-labelledby="health-result" data-testid="health-result">
        <div className={ui.panelHead}>
          <h2 id="health-result" className={ui.panelTitle}>
            {t('test.health.resultTitle')}
          </h2>
        </div>
        <p className={css.summary}>{summary}</p>
        <p className={report.precision.kind === 'event' ? css.warn : css.note}>
          <Icon name={report.precision.kind === 'event' ? 'alert' : 'info'} size={16} />
          {precisionNote(t, report.precision)}
        </p>
        <ul className={css.cards}>
          {report.buttons.map((b) => (
            <HealthCard key={b.button} b={b} label={name(b.button)} />
          ))}
        </ul>
        <div className={css.actions}>
          <SaveToHistory
            label={t('test.health.save')}
            onSave={() =>
              saveSession(c, 'buttonHealth', healthScore(report), summary, {
                report,
                plans: [...plans.current.values()],
              })
            }
          />
          <button type="button" className={ui.btn} onClick={() => setPhase('setup')}>
            <Icon name="refresh" size={16} /> {t('test.health.restart')}
          </button>
        </div>
      </section>
    );
  }

  return (
    <div className={css.layout}>
      <section className={`${ui.panel} ${css.prompt}`} aria-labelledby="health-step">
        <span className={css.step}>
          {t('test.health.step', { i: idx + 1, total: queue.length })}
        </span>
        {current !== undefined && (
          <div
            className={css.glyph}
            aria-hidden="true"
            data-held={live?.held || undefined}
            style={{ ['--chars' as string]: name(current).length }}
          >
            {name(current)}
          </div>
        )}

        {phase === 'taps' && current !== undefined && (
          <>
            <h2 id="health-step" className={css.ask}>
              {t('test.health.tapPrompt', { button: buttonSpoken(c.kind, current), n: presses })}
            </h2>
            <p className={css.hint}>{t('test.health.tapHint', { n: presses })}</p>
            <div className={css.counter} role="status" aria-live="polite" aria-atomic="true">
              <span className={ui.faint}>{t('test.health.counterLabel')}</span>
              <strong data-testid="health-counter">
                {t('test.health.counter', { taps: roundTaps, n: presses })}
              </strong>
            </div>
            <div className={css.actions}>
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                onClick={finishTaps}
                data-testid="health-taps-done"
              >
                <Icon name="check" size={16} /> {t('test.health.tapsDone', { n: presses })}
              </button>
              <button type="button" className={ui.btn} onClick={advance}>
                {t('test.health.skip')}
              </button>
              <button type="button" className={`${ui.btn} ${ui.ghost}`} onClick={cancel}>
                {t('test.health.cancel')}
              </button>
            </div>
          </>
        )}

        {phase === 'hold' && current !== undefined && (
          <>
            <h2 id="health-step" className={css.ask}>
              {t('test.health.holdPrompt', { button: buttonSpoken(c.kind, current) })}
            </h2>
            <p className={css.hint}>
              {t('test.health.holdHint', { s: HOLD_TARGET_US / 1_000_000 })}
            </p>
            <div
              className={css.bar}
              role="progressbar"
              aria-label={t('test.health.holdProgress', { button: name(current) })}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={holdPct}
              data-full={canRelease || undefined}
            >
              <span style={{ width: `${holdPct}%` }} />
            </div>
            <p className={css.release} role="status" aria-live="polite">
              {canRelease ? t('test.health.holdRelease') : ''}
            </p>
            <div className={css.actions}>
              <button type="button" className={ui.btn} onClick={advance}>
                {t('test.health.skip')}
              </button>
              <button type="button" className={`${ui.btn} ${ui.ghost}`} onClick={cancel}>
                {t('test.health.cancel')}
              </button>
            </div>
          </>
        )}

        {phase === 'heldEarly' && current !== undefined && (
          <>
            <h2 id="health-step" className={css.ask}>
              {t('test.health.heldEarly')}
            </h2>
            <div className={css.actions}>
              <button type="button" className={ui.btn} onClick={() => beginHold(current)}>
                {t('test.health.iReleased')}
              </button>
              <button
                type="button"
                className={`${ui.btn} ${ui.danger}`}
                onClick={reportSelfRelease}
              >
                {t('test.health.notReleased')}
              </button>
              <button type="button" className={`${ui.btn} ${ui.ghost}`} onClick={cancel}>
                {t('test.health.cancel')}
              </button>
            </div>
          </>
        )}

        {phase === 'analyzing' && (
          <h2 id="health-step" className={css.ask} role="status">
            {t('test.health.analyzing')}
          </h2>
        )}
        {progress && <p className={css.precision}>{precisionNote(t, progress.precision)}</p>}
      </section>
      <section className={`${ui.panel} ${css.diagram}`}>
        <ControllerDiagram controller={c} testHighlight={phase === 'analyzing' ? null : current} />
      </section>
    </div>
  );
}

function HealthCard({ b, label }: { b: ButtonHealth; label: string }) {
  const t = useT();
  const m = b.metrics;
  const ms = (v: number | null) => (v == null ? t('app.common.na') : `${Math.round(v)} ms`);
  const rows: [TKey, string][] = [
    ['test.health.metrics.taps', m.requested ? `${m.taps} / ${m.requested}` : String(m.taps)],
    ['test.health.metrics.chatter', String(m.chatter)],
    ['test.health.metrics.releaseBounces', String(m.releaseBounces)],
    ['test.health.metrics.selfReleases', String(m.selfReleases)],
    ['test.health.metrics.slowReleases', String(m.slowReleases)],
    ['test.health.metrics.medianTap', ms(m.medianTapMs)],
    ['test.health.metrics.longest', ms(m.longestPressMs)],
  ];
  return (
    <li className={css.card} data-grade={b.grade} data-testid={`health-card-${b.button}`}>
      <div className={css.cardHead}>
        <strong className={css.cardName}>{label}</strong>
        <span className={css.badge} data-grade={b.grade}>
          <Icon name={GRADE_ICON[b.grade]} size={14} />
          {t(`test.health.grades.${b.grade}`)}
        </span>
      </div>
      <dl className={css.metrics}>
        {rows.map(([k, v]) => (
          <div key={k}>
            <dt>{t(k)}</dt>
            <dd className="mono">{v}</dd>
          </div>
        ))}
      </dl>
      {b.reasons.length ? (
        <ul className={css.reasons}>
          {b.reasons.map((r) => (
            <li key={r.code} data-grade={r.grade}>
              {t(`test.health.reasons.${r.code}`, { count: r.count })}
            </li>
          ))}
        </ul>
      ) : (
        <p className={css.ok}>{t('test.health.noIssues')}</p>
      )}
      {b.advice.length > 0 && (
        <div className={css.advice}>
          <h3>{t('test.health.adviceTitle')}</h3>
          <ul>
            {b.advice.map((a) => (
              <li key={a}>{t(`test.health.advice.${a}`)}</li>
            ))}
          </ul>
        </div>
      )}
    </li>
  );
}
