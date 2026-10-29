import { useEffect, useRef, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type {
  CaptureResult,
  ControllerInfo,
  DemoScenario,
  StickReport,
  TriggerAnalysis,
  Vec2,
} from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { Segmented, Switch } from '@/app/controls';
import { Icon } from '@/app/icons';
import { useT, type TKey } from '@/i18n';
import { useApp } from '@/state/app';
import { downsample, saveSession } from '@/lib/history';
import { SaveToHistory } from '@/app/SaveToHistory';
import { live } from '@/state/live';
import { useActiveProfile, useProfiles } from '@/state/profiles';
import ui from '@/styles/ui.module.css';
import { StickReportView } from './StickReportView';
import css from './Wizard.module.css';

type Side = 'left' | 'right';
type Step = 'intro' | 'rest' | 'rotation' | 'directions' | 'triggers' | 'analyzing' | 'result';
const STEPS: Step[] = ['rest', 'rotation', 'directions', 'triggers', 'result'];
const DIR_NAMES = ['right', 'upRight', 'up', 'upLeft', 'left', 'downLeft', 'down', 'downRight'];
const SECTORS = 32;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

function meanOf(c: CaptureResult | undefined): Vec2 {
  if (!c?.samples.length) return { x: 0, y: 0 };
  let x = 0;
  let y = 0;
  for (const s of c.samples) {
    x += s.x;
    y += s.y;
  }
  return { x: x / c.samples.length, y: y / c.samples.length };
}

function stickOf(side: Side) {
  return (id: string): Vec2 | null => {
    const e = live.get(id);
    return e ? (side === 'left' ? e.snapshot.leftStick : e.snapshot.rightStick) : null;
  };
}

export function Wizard({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const announce = useApp((s) => s.announce);
  const lockNav = useApp((s) => s.lockNav);
  const profile = useActiveProfile();
  const edit = useProfiles((s) => s.edit);
  const flush = useProfiles((s) => s.flush);

  const [side, setSide] = useState<Side>('left');
  const [withTriggers, setWithTriggers] = useState(true);
  const [step, setStep] = useState<Step>('intro');
  const [caps, setCaps] = useState<Partial<Record<string, CaptureResult>>>({});
  const [recording, setRecording] = useState(false);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [coverage, setCoverage] = useState<boolean[]>(() => new Array(SECTORS).fill(false));
  const [dirsDone, setDirsDone] = useState<boolean[]>(() => new Array(8).fill(false));
  const [trigPhase, setTrigPhase] = useState<'rest' | 'left' | 'right'>('rest');
  const [report, setReport] = useState<StickReport | null>(null);
  const [after, setAfter] = useState<StickReport | null>(null);
  const [trigReport, setTrigReport] = useState<{
    left: TriggerAnalysis | null;
    right: TriggerAnalysis | null;
  } | null>(null);
  const [triggerValue, setTriggerValue] = useState(0);
  const hold = useRef<{ dir: number; since: number }>({ dir: -1, since: 0 });

  const stickName = t(
    side === 'left' ? 'calibration.wizard.left' : 'calibration.wizard.right',
  ).toLowerCase();
  const demo = (s: DemoScenario) => (c.isDemo ? api.demoScenario(s) : Promise.resolve());
  const fail = (e: unknown) => {
    setRecording(false);
    setCountdown(null);
    toast(errorToast(errorCode(e), errorMessage(e)));
  };

  useEffect(() => {
    if (step === 'intro' || step === 'result') return;
    return lockNav();
  }, [step, lockNav]);

  useEffect(
    () => () => {
      void api.captureClear().catch(() => {});
      if (c.isDemo) void api.demoScenario('showcase').catch(() => {});
    },
    [c.id, c.isDemo],
  );

  const go = (s: Step) => {
    setStep(s);
    setRecording(false);
    const idx = STEPS.indexOf(s);
    if (idx >= 0)
      announce(
        `${t('calibration.wizard.stepOf', { n: idx + 1, total: STEPS.length })}: ${t(`calibration.wizard.steps.${s}` as TKey)}`,
      );
  };

  const start = async () => {
    setCaps({});
    setReport(null);
    setAfter(null);
    setTrigReport(null);
    await api.captureClear().catch(() => {});
    go('rest');
  };

  /** Records `ms` milliseconds with a visible countdown. */
  const timed = async (kind: string, target: Side, ms: number) => {
    await api.captureStart(c.id, kind as never, target);
    const end = Date.now() + ms;
    while (Date.now() < end) {
      setCountdown(Math.ceil((end - Date.now()) / 1000));
      await sleep(100);
    }
    setCountdown(null);
    return api.captureStop();
  };

  const measureRest = async () => {
    try {
      setRecording(true);
      await demo('idle');
      await sleep(c.isDemo ? 250 : 0);
      const r = await timed('rest', side, 2000);
      setCaps((p) => ({ ...p, rest: r }));
      setRecording(false);
      announce(t('calibration.wizard.rest.done', { n: r.samples.length }));
    } catch (e) {
      fail(e);
    }
  };

  const startRecording = async (kind: 'rotation' | 'directions') => {
    try {
      setCoverage(new Array(SECTORS).fill(false));
      setDirsDone(new Array(8).fill(false));
      hold.current = { dir: -1, since: 0 };
      await demo(
        kind === 'rotation'
          ? side === 'left'
            ? 'rotateLeft'
            : 'rotateRight'
          : side === 'left'
            ? 'directionsLeft'
            : 'directionsRight',
      );
      await api.captureStart(c.id, kind, side);
      setRecording(true);
    } catch (e) {
      fail(e);
    }
  };

  const finishRecording = async (kind: 'rotation' | 'directions') => {
    try {
      const r = await api.captureStop();
      setCaps((p) => ({ ...p, [kind]: r }));
      setRecording(false);
      await demo('idle');
      go(kind === 'rotation' ? 'directions' : withTriggers ? 'triggers' : 'analyzing');
      if (kind === 'directions' && !withTriggers) void analyze();
      if (kind === 'directions' && withTriggers) setTrigPhase('rest');
    } catch (e) {
      fail(e);
    }
  };

  // Live feedback while recording rotation/directions: every backend frame
  // is processed (not only animation frames), so it also works while the
  // window is in the background.
  const center = meanOf(caps.rest);
  useEffect(() => {
    if (!recording || (step !== 'rotation' && step !== 'directions')) return;
    const read = stickOf(side);
    return live.subscribe(() => {
      const p = read(c.id);
      if (!p) return;
      const x = p.x - center.x;
      const y = p.y - center.y;
      const r = Math.hypot(x, y);
      let ang = Math.atan2(y, x);
      if (ang < 0) ang += Math.PI * 2;
      if (step === 'rotation' && r > 0.5) {
        const idx = Math.min(SECTORS - 1, Math.floor((ang / (Math.PI * 2)) * SECTORS));
        setCoverage((cv) => (cv[idx] ? cv : cv.map((v, i) => v || i === idx)));
      }
      if (step === 'directions') {
        const d = r > 0.8 ? Math.round(ang / (Math.PI / 4)) % 8 : -1;
        const now = performance.now();
        if (d !== hold.current.dir) hold.current = { dir: d, since: now };
        else if (d >= 0 && now - hold.current.since > 300) {
          setDirsDone((dd) => {
            if (dd[d]) return dd;
            announce(t(`calibration.wizard.directions.names.${DIR_NAMES[d]}` as TKey));
            return dd.map((v, i) => v || i === d);
          });
        }
      }
    });
    // center is derived from caps.rest, which is fixed while recording
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recording, step, side, c.id, caps.rest]);

  // Auto-finish directions once all 8 are in.
  useEffect(() => {
    if (step === 'directions' && recording && dirsDone.every(Boolean)) {
      const id = window.setTimeout(() => void finishRecording('directions'), 400);
      return () => window.clearTimeout(id);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dirsDone, step, recording]);

  // Trigger value readout.
  useEffect(() => {
    if (step !== 'triggers' || !recording || trigPhase === 'rest') return;
    return live.subscribe(() => {
      const e = live.get(c.id);
      if (!e) return;
      const v = trigPhase === 'left' ? e.snapshot.leftTrigger : e.snapshot.rightTrigger;
      setTriggerValue((old) => (Math.abs(v - old) > 0.01 ? v : old));
    });
  }, [step, recording, trigPhase, c.id]);

  const triggerRest = async () => {
    try {
      setRecording(true);
      await demo('triggerRest');
      await sleep(c.isDemo ? 200 : 0);
      await timed('triggerRest', 'left', 1000);
      await timed('triggerRest', 'right', 1000);
      setRecording(false);
      setTrigPhase('left');
    } catch (e) {
      fail(e);
    }
  };

  const triggerPress = async (s: Side) => {
    try {
      if (s === 'left') await demo('triggerFull');
      await api.captureStart(c.id, 'triggerFull', s);
      setRecording(true);
    } catch (e) {
      fail(e);
    }
  };

  const triggerDone = async (s: Side) => {
    try {
      await api.captureStop();
      setRecording(false);
      if (s === 'left') setTrigPhase('right');
      else void analyze();
    } catch (e) {
      fail(e);
    }
  };

  const analyze = async () => {
    setStep('analyzing');
    try {
      await demo('showcase');
      const rep = await api.analyzeStick(side, null);
      let trig = null;
      if (withTriggers) {
        const [l, r] = await Promise.all([
          api.analyzeTrigger('left').catch(() => null),
          api.analyzeTrigger('right').catch(() => null),
        ]);
        trig = { left: l, right: r };
      }
      setReport(rep);
      setTrigReport(trig);
      setAfter(null);
      go('result');
      announce(`${t('calibration.report.score')}: ${rep.score.score} (${rep.score.grade})`);
    } catch (e) {
      fail(e);
      setStep('directions');
    }
  };

  const applySuggestion = async () => {
    if (!report || !profile) return;
    const key = side === 'left' ? 'leftStick' : 'rightStick';
    const s = report.suggested;
    let merged = profile.calibration[key];
    edit((cal) => {
      merged = {
        ...cal[key],
        center: s.center,
        rangePos: s.rangePos,
        rangeNeg: s.rangeNeg,
        innerDeadzone: s.innerDeadzone,
        outerDeadzone: s.outerDeadzone,
      };
      const next = { ...cal, [key]: merged, measuredAt: new Date().toISOString() };
      if (trigReport?.left)
        next.leftTrigger = {
          ...cal.leftTrigger,
          deadzone: trigReport.left.suggestedDeadzone,
          maxLimit: trigReport.left.suggestedMax,
        };
      if (trigReport?.right)
        next.rightTrigger = {
          ...cal.rightTrigger,
          deadzone: trigReport.right.suggestedDeadzone,
          maxLimit: trigReport.right.suggestedMax,
        };
      return next;
    });
    await flush();
    try {
      setAfter(await api.analyzeStick(side, merged));
      toast({
        tone: 'ok',
        title: t('calibration.report.suggestion.applied', { name: profile.name }),
      });
    } catch (e) {
      fail(e);
    }
  };

  const stepIdx = STEPS.indexOf(step === 'analyzing' ? 'result' : step);
  const covered = coverage.filter(Boolean).length / SECTORS;

  if (step === 'intro') {
    return (
      <section className={`${ui.panel} ${css.intro}`} aria-labelledby="wiz-title">
        <h2 id="wiz-title" className={css.title}>
          {t('calibration.wizard.title')}
        </h2>
        <p>{t('calibration.wizard.intro')}</p>
        <div className={css.options}>
          <div>
            <p className={ui.fieldLabel}>{t('calibration.wizard.chooseStick')}</p>
            <Segmented<Side>
              label={t('calibration.wizard.chooseStick')}
              value={side}
              onChange={setSide}
              options={[
                { value: 'left', label: t('calibration.wizard.left') },
                { value: 'right', label: t('calibration.wizard.right') },
              ]}
            />
          </div>
          <Switch
            label={t('calibration.wizard.includeTriggers')}
            checked={withTriggers}
            onChange={setWithTriggers}
          />
        </div>
        {c.isDemo && <p className={ui.faint}>{t('calibration.wizard.demoNote')}</p>}
        <p className={ui.faint}>{t('calibration.wizard.navPaused')}</p>
        <button
          type="button"
          className={`${ui.btn} ${ui.primary}`}
          onClick={start}
          data-testid="wizard-start"
        >
          <Icon name="play" size={16} /> {t('calibration.wizard.start')}
        </button>
      </section>
    );
  }

  return (
    <div className={css.wrap}>
      <ol className={css.stepper} aria-label={t('calibration.wizard.title')}>
        {STEPS.filter((s) => s !== 'triggers' || withTriggers).map((s) => (
          <li
            key={s}
            data-state={
              STEPS.indexOf(s) < stepIdx
                ? 'done'
                : STEPS.indexOf(s) === stepIdx
                  ? 'current'
                  : 'todo'
            }
            aria-current={STEPS.indexOf(s) === stepIdx ? 'step' : undefined}
          >
            <span>{STEPS.indexOf(s) + 1}</span>
            {t(`calibration.wizard.steps.${s}` as TKey)}
          </li>
        ))}
      </ol>

      {step === 'rest' && (
        <section className={`${ui.panel} ${css.step}`} aria-live="polite" data-testid="wizard-rest">
          <h2 className={css.title}>{t('calibration.wizard.rest.title')}</h2>
          <p>{t('calibration.wizard.rest.body')}</p>
          {countdown != null && (
            <p className={css.count}>{t('calibration.wizard.rest.measuring', { s: countdown })}</p>
          )}
          {caps.rest && (
            <p className={css.good}>
              {t('calibration.wizard.rest.done', { n: caps.rest.samples.length })}
            </p>
          )}
          <div className={css.actions}>
            {!caps.rest ? (
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                disabled={recording}
                onClick={measureRest}
                data-testid="rest-measure"
              >
                {t('calibration.wizard.rest.measure')}
              </button>
            ) : (
              <>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  onClick={() => go('rotation')}
                  data-testid="wizard-next"
                  autoFocus
                >
                  {t('calibration.wizard.next')}
                </button>
                <button type="button" className={ui.btn} onClick={measureRest} disabled={recording}>
                  {t('calibration.wizard.retry')}
                </button>
              </>
            )}
            <button
              type="button"
              className={`${ui.btn} ${ui.ghost}`}
              onClick={() => setStep('intro')}
            >
              {t('calibration.wizard.cancel')}
            </button>
          </div>
        </section>
      )}

      {step === 'rotation' && (
        <section className={`${ui.panel} ${css.step} ${css.twoCol}`} data-testid="wizard-rotation">
          <div>
            <h2 className={css.title}>
              {t('calibration.wizard.rotation.title', { stick: stickName })}
            </h2>
            <p>{t('calibration.wizard.rotation.body')}</p>
            <p className={css.count} aria-live="polite">
              {t('calibration.wizard.rotation.coverage', { pct: `${Math.round(covered * 100)}%` })}
            </p>
            {recording && covered < 0.9 && (
              <p className={ui.faint}>{t('calibration.wizard.rotation.lowCoverage')}</p>
            )}
            <div className={css.actions}>
              {!recording ? (
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  onClick={() => startRecording('rotation')}
                  data-testid="rotation-start"
                >
                  {t('calibration.wizard.rotation.start')}
                </button>
              ) : (
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  onClick={() => finishRecording('rotation')}
                  data-testid="rotation-finish"
                >
                  {t('calibration.wizard.rotation.finish')}
                </button>
              )}
              <button
                type="button"
                className={`${ui.btn} ${ui.ghost}`}
                onClick={() => setStep('intro')}
              >
                {t('calibration.wizard.cancel')}
              </button>
            </div>
          </div>
          <svg viewBox="-110 -110 220 220" className={css.ring} aria-hidden="true">
            {coverage.map((on, i) => {
              const a0 = (i / SECTORS) * Math.PI * 2;
              const a1 = ((i + 1) / SECTORS) * Math.PI * 2 - 0.03;
              const p = (a: number, r: number) =>
                `${(Math.cos(a) * r).toFixed(1)} ${(-Math.sin(a) * r).toFixed(1)}`;
              return (
                <path
                  key={i}
                  d={`M ${p(a0, 78)} A 78 78 0 0 0 ${p(a1, 78)} L ${p(a1, 100)} A 100 100 0 0 1 ${p(a0, 100)} Z`}
                  data-on={on || undefined}
                />
              );
            })}
            <text x="0" y="8" textAnchor="middle" className={css.ringPct}>
              {Math.round(covered * 100)}%
            </text>
          </svg>
        </section>
      )}

      {step === 'directions' && (
        <section
          className={`${ui.panel} ${css.step} ${css.twoCol}`}
          data-testid="wizard-directions"
        >
          <div>
            <h2 className={css.title}>{t('calibration.wizard.directions.title')}</h2>
            <p>{t('calibration.wizard.directions.body', { stick: stickName })}</p>
            <p className={css.count} aria-live="polite">
              {t('calibration.wizard.directions.progress', { n: dirsDone.filter(Boolean).length })}
            </p>
            <div className={css.actions}>
              {!recording ? (
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  onClick={() => startRecording('directions')}
                  data-testid="directions-start"
                >
                  {t('calibration.wizard.directions.start')}
                </button>
              ) : (
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  onClick={() => finishRecording('directions')}
                  data-testid="directions-finish"
                >
                  {t('calibration.wizard.directions.finish')}
                </button>
              )}
            </div>
          </div>
          <svg viewBox="-110 -110 220 220" className={css.compass} aria-hidden="true">
            <circle r="30" className={css.hub} />
            {DIR_NAMES.map((n, i) => {
              const a = (i * Math.PI) / 4;
              const x = Math.cos(a) * 76;
              const y = -Math.sin(a) * 76;
              return (
                <g
                  key={n}
                  transform={`translate(${x} ${y}) rotate(${-i * 45})`}
                  data-on={dirsDone[i] || undefined}
                >
                  <path d="M -16 -12 L 16 0 L -16 12 Z" />
                </g>
              );
            })}
          </svg>
        </section>
      )}

      {step === 'triggers' && (
        <section className={`${ui.panel} ${css.step}`} data-testid="wizard-triggers">
          <h2 className={css.title}>{t('calibration.wizard.triggers.title')}</h2>
          {trigPhase === 'rest' ? (
            <>
              <p>{t('calibration.wizard.triggers.releaseBody')}</p>
              {countdown != null && (
                <p className={css.count}>
                  {t('calibration.wizard.rest.measuring', { s: countdown })}
                </p>
              )}
              <div className={css.actions}>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  disabled={recording}
                  onClick={triggerRest}
                  data-testid="trigger-rest"
                >
                  {t('calibration.wizard.triggers.measureRest')}
                </button>
                <button
                  type="button"
                  className={ui.btn}
                  onClick={() => {
                    setWithTriggers(false);
                    void analyze();
                  }}
                >
                  {t('calibration.wizard.triggers.skip')}
                </button>
              </div>
            </>
          ) : (
            <>
              <p className={css.ask}>
                {t('calibration.wizard.triggers.press', {
                  trigger:
                    trigPhase === 'left'
                      ? c.kind === 'xbox'
                        ? 'LT'
                        : 'L2'
                      : c.kind === 'xbox'
                        ? 'RT'
                        : 'R2',
                })}
              </p>
              <p className={ui.faint}>{t('calibration.wizard.triggers.holdHint')}</p>
              {recording && (
                <div
                  className={css.trigBar}
                  role="meter"
                  aria-valuenow={Math.round(triggerValue * 100)}
                  aria-valuemin={0}
                  aria-valuemax={100}
                >
                  <i style={{ transform: `scaleX(${triggerValue})` }} />
                  <span className="mono">
                    {t('calibration.wizard.triggers.value', {
                      pct: `${Math.round(triggerValue * 100)}%`,
                    })}
                  </span>
                </div>
              )}
              <div className={css.actions}>
                {!recording ? (
                  <button
                    type="button"
                    className={`${ui.btn} ${ui.primary}`}
                    onClick={() => triggerPress(trigPhase as Side)}
                    data-testid="trigger-press"
                  >
                    {t('calibration.wizard.rotation.start')}
                  </button>
                ) : (
                  <button
                    type="button"
                    className={`${ui.btn} ${ui.primary}`}
                    onClick={() => triggerDone(trigPhase as Side)}
                    data-testid="trigger-done"
                  >
                    {t('calibration.wizard.triggers.done')}
                  </button>
                )}
              </div>
            </>
          )}
        </section>
      )}

      {step === 'analyzing' && (
        <section className={`${ui.panel} ${css.step}`} aria-live="polite">
          <p className={css.count}>{t('calibration.wizard.analyzing')}</p>
        </section>
      )}

      {step === 'result' && report && (
        <>
          <section className={`${ui.panel} ${css.suggest}`}>
            <div>
              <h2 className={ui.panelTitle}>{t('calibration.report.suggestion.title')}</h2>
              <p className={ui.muted}>{t('calibration.report.suggestion.body')}</p>
              <p className="mono" style={{ fontSize: 'var(--fs-xs)' }}>
                center ({report.suggested.center.x.toFixed(3)},{' '}
                {report.suggested.center.y.toFixed(3)}) · dz{' '}
                {(report.suggested.innerDeadzone * 100).toFixed(1)}% · range +(
                {report.suggested.rangePos.x.toFixed(2)}, {report.suggested.rangePos.y.toFixed(2)})
                −({report.suggested.rangeNeg.x.toFixed(2)}, {report.suggested.rangeNeg.y.toFixed(2)}
                )
              </p>
            </div>
            <div className={css.actions}>
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                onClick={applySuggestion}
                disabled={!profile}
                data-testid="apply-suggestion"
              >
                <Icon name="check" size={16} /> {t('calibration.report.suggestion.apply')}
              </button>
              <button
                type="button"
                className={ui.btn}
                onClick={() => {
                  setSide(side === 'left' ? 'right' : 'left');
                  setWithTriggers(false);
                  void start();
                }}
              >
                {t('calibration.wizard.otherStick')}
              </button>
              <SaveToHistory
                label={t('calibration.report.save')}
                onSave={() =>
                  saveSession(
                    c,
                    'calibration',
                    (after ?? report).score.score,
                    `${t(side === 'left' ? 'calibration.wizard.left' : 'calibration.wizard.right')}: ${report.score.grade} ${report.score.score}${after ? ` → ${after.score.grade} ${after.score.score}` : ''}`,
                    {
                      stick: side,
                      before: report,
                      after,
                      rotation: downsample(caps.rotation?.samples ?? [], 2000),
                      triggers: trigReport,
                    },
                  )
                }
              />
              <button
                type="button"
                className={`${ui.btn} ${ui.ghost}`}
                onClick={() => setStep('intro')}
              >
                {t('calibration.wizard.restart')}
              </button>
            </div>
          </section>
          <StickReportView
            report={report}
            after={after}
            rotation={caps.rotation?.samples ?? []}
            triggers={trigReport}
          />
        </>
      )}
    </div>
  );
}
