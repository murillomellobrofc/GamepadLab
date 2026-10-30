import { forwardRef } from 'react';
import type {
  CaptureSample,
  ControllerInfo,
  HealthReport,
  HistorySession,
  StickReport,
  TriggerAnalysis,
} from '@/api/types';
import { formatDateTime, useT, type TKey } from '@/i18n';
import { buttonName } from '@/lib/buttonNames';
import { PAPER } from './paper';
import { CoverageChart } from '../calibration/CoverageChart';
import { compareRows } from '../calibration/report';
import css from './ReportSheet.module.css';

type Payload = Record<string, unknown>;

function Buttons({ p }: { p: Payload }) {
  const t = useT();
  const results = (p.results ?? []) as {
    name: string;
    status: string;
    maxValue: number;
    button: number;
  }[];
  return (
    <ul className={css.checklist}>
      {results.map((r) => (
        <li key={r.button} data-status={r.status}>
          <b>{r.status === 'pass' ? '✓' : r.status === 'fail' ? '✗' : '–'}</b> {r.name}
          <small>
            {t(
              `test.guided.${r.status === 'pass' ? 'pass' : r.status === 'fail' ? 'failed' : r.status === 'skip' ? 'skipped' : 'pending'}` as TKey,
            )}
          </small>
        </li>
      ))}
    </ul>
  );
}

function Health({ p }: { p: Payload }) {
  const t = useT();
  const report = p.report as HealthReport | undefined;
  const kind = (p.controller as ControllerInfo | undefined)?.kind ?? 'generic';
  if (!report) return null;
  const precision =
    report.precision.kind === 'report'
      ? t('test.health.precisionReport', {
          ms: Math.max(1, Math.round(report.precision.intervalUs / 1000)),
        })
      : t('test.health.precisionEvent');
  return (
    <div className={css.health}>
      <p className={css.note}>{precision}</p>
      <table>
        <thead>
          <tr>
            <th>{t('report.button')}</th>
            <th>{t('report.grade')}</th>
            <th>{t('test.health.metrics.taps')}</th>
            <th>{t('test.health.metrics.chatter')}</th>
            <th>{t('test.health.metrics.releaseBounces')}</th>
            <th>{t('test.health.metrics.selfReleases')}</th>
            <th>{t('test.health.metrics.slowReleases')}</th>
            <th>{t('test.health.adviceTitle')}</th>
          </tr>
        </thead>
        <tbody>
          {report.buttons.map((b) => (
            <tr key={b.button} data-grade={b.grade}>
              <td>
                <b>{buttonName(kind, b.button)}</b>
              </td>
              <td className={css.gradeCell}>{t(`test.health.grades.${b.grade}`)}</td>
              <td>
                {b.metrics.requested ? `${b.metrics.taps}/${b.metrics.requested}` : b.metrics.taps}
              </td>
              <td>{b.metrics.chatter}</td>
              <td>{b.metrics.releaseBounces}</td>
              <td>{b.metrics.selfReleases}</td>
              <td>{b.metrics.slowReleases}</td>
              <td className={css.advice}>
                {b.reasons.map((r) => (
                  <span key={r.code}>{t(`test.health.reasons.${r.code}`, { count: r.count })}</span>
                ))}
                {b.advice.map((a) => (
                  <em key={a}>{t(`test.health.advice.${a}`)}</em>
                ))}
                {!b.reasons.length && t('test.health.noIssues')}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Calibration({ p }: { p: Payload }) {
  const t = useT();
  const before = p.before as StickReport | undefined;
  const after = p.after as StickReport | null | undefined;
  const rotation = (p.rotation ?? []) as CaptureSample[];
  const trig = p.triggers as { left: TriggerAnalysis | null; right: TriggerAnalysis | null } | null;
  if (!before) return null;
  const r = before.rest;
  return (
    <div className={css.calib}>
      <div className={css.chart}>
        <CoverageChart
          samples={rotation}
          center={r?.mean ?? { x: 0, y: 0 }}
          before={before.circularity}
          after={after?.circularity ?? null}
          calibration={after ? after.suggested : null}
          palette={PAPER}
        />
      </div>
      <div className={css.stats}>
        <p className={css.grade}>
          {t('calibration.report.score')}:{' '}
          <b>
            {before.score.grade} · {before.score.score}
          </b>
          {after && (
            <>
              {' → '}
              <b>
                {after.score.grade} · {after.score.score}
              </b>
            </>
          )}
        </p>
        {r && (
          <dl>
            <dt>{t('calibration.report.rest.offset')}</dt>
            <dd>
              {r.offsetPct.toFixed(2)}% ({t(`calibration.report.rest.grades.${r.grade}`)})
            </dd>
            <dt>{t('calibration.report.rest.noise')}</dt>
            <dd>{r.stdRadial.toFixed(4)}</dd>
            <dt>{t('calibration.report.rest.suggestedDz')}</dt>
            <dd>{(r.suggestedDeadzone * 100).toFixed(1)}%</dd>
          </dl>
        )}
        {before.circularity && (
          <dl>
            <dt>{t('calibration.report.circ.roundness')}</dt>
            <dd>{before.circularity.roundnessPct.toFixed(1)}%</dd>
            <dt>{t('calibration.report.circ.error')}</dt>
            <dd>{before.circularity.avgErrorPct.toFixed(1)}%</dd>
            <dt>{t('calibration.report.circ.coverage')}</dt>
            <dd>{before.circularity.coveragePct.toFixed(0)}%</dd>
          </dl>
        )}
        {before.score.issues.length > 0 && (
          <ul className={css.issues}>
            {before.score.issues.map((i) => (
              <li key={i}>{t(`calibration.report.issues.${i}` as TKey)}</li>
            ))}
          </ul>
        )}
        {after && (
          <table className={css.table}>
            <tbody>
              {compareRows(before, after).map((row) => (
                <tr key={row.key}>
                  <td>{t(row.key as TKey)}</td>
                  <td>{row.before}</td>
                  <td>{row.after}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        {trig && (trig.left || trig.right) && (
          <p className={css.small}>
            {t('calibration.report.triggers.title')}:{' '}
            {(['left', 'right'] as const)
              .map(
                (s) =>
                  trig[s] &&
                  `${s === 'left' ? 'L' : 'R'} ${(trig[s]!.maxReached * 100).toFixed(0)}%`,
              )
              .filter(Boolean)
              .join(' · ')}
          </p>
        )}
      </div>
    </div>
  );
}

function Latency({ p }: { p: Payload }) {
  const t = useT();
  const s = p.summary as
    | { hz: number; avgMs: number; minMs: number; maxMs: number; jitterMs: number; p99Ms: number }
    | undefined;
  const intervals = (p.intervalsUs ?? []) as number[];
  if (!s) return null;
  const ms = intervals.map((v) => v / 1000).filter((v) => v > 0);
  const lo = Math.min(...ms);
  const hi = Math.max(...ms);
  const bins = new Array(30).fill(0);
  for (const v of ms) bins[Math.min(29, Math.floor(((v - lo) / (hi - lo || 1)) * 30))]++;
  const max = Math.max(...bins, 1);
  return (
    <div className={css.latency}>
      <dl>
        <dt>{t('latency.rate')}</dt>
        <dd>{s.hz.toFixed(0)} Hz</dd>
        <dt>{t('latency.avg')}</dt>
        <dd>{s.avgMs.toFixed(2)} ms</dd>
        <dt>{t('latency.jitter')}</dt>
        <dd>{s.jitterMs.toFixed(2)} ms</dd>
        <dt>
          {t('latency.min')} / {t('latency.max')}
        </dt>
        <dd>
          {s.minMs.toFixed(2)} / {s.maxMs.toFixed(2)} ms
        </dd>
      </dl>
      {ms.length > 0 && (
        <svg viewBox="0 0 300 80" className={css.hist} aria-label={t('latency.histogram')}>
          {bins.map((b, i) => (
            <rect
              key={i}
              x={i * 10 + 1}
              width={8}
              y={70 - (b / max) * 66}
              height={(b / max) * 66}
            />
          ))}
          <text x="0" y="79">
            {lo.toFixed(2)} ms
          </text>
          <text x="300" y="79" textAnchor="end">
            {hi.toFixed(2)} ms
          </text>
        </svg>
      )}
    </div>
  );
}

function Ghosting({ p }: { p: Payload }) {
  const t = useT();
  const r = p.result as
    | {
        rollover: number;
        targets: number[];
        suspects: number[];
        never: number[];
        names: Record<string, string>;
      }
    | undefined;
  if (!r) return null;
  return (
    <p>
      {t('test.ghosting.rollover')}:{' '}
      <b>
        {r.rollover}/{r.targets.length}
      </b>
      {r.suspects.length + r.never.length > 0
        ? ` · ${t('test.ghosting.ghostFound', { list: [...new Set([...r.suspects, ...r.never])].map((b) => r.names[b] ?? b).join(', ') })}`
        : ` · ${t('test.ghosting.noGhost')}`}
    </p>
  );
}

/** Printable report built from history sessions (rendered off-screen). */
export const ReportSheet = forwardRef<
  HTMLDivElement,
  { sessions: HistorySession[]; version: string }
>(function ReportSheet({ sessions, version }, ref) {
  const t = useT();
  const now = formatDateTime(new Date());
  return (
    <div ref={ref} className={css.sheet} data-testid="report-sheet">
      <header className={css.head}>
        <img src="/favicon.svg" alt="" width={44} height={44} />
        <div>
          <h1>{t('report.heading')}</h1>
          <p>
            {t('report.generated', { date: now })} · GamepadLab v{version}
          </p>
        </div>
      </header>
      {sessions.map((s) => {
        const p = (s.payload ?? {}) as Payload;
        return (
          <section key={s.id} className={css.section}>
            <h2>
              {t(`history.kinds.${s.kind}` as TKey)} · {s.controllerName}
              <span>{formatDateTime(s.createdAt)}</span>
            </h2>
            <p className={css.summary}>{s.summary}</p>
            {s.kind === 'buttons' && <Buttons p={p} />}
            {s.kind === 'buttonHealth' && <Health p={p} />}
            {s.kind === 'calibration' && <Calibration p={p} />}
            {s.kind === 'latency' && <Latency p={p} />}
            {s.kind === 'ghosting' && <Ghosting p={p} />}
          </section>
        );
      })}
      <footer className={css.foot}>{t('report.noTelemetry')}</footer>
    </div>
  );
});
