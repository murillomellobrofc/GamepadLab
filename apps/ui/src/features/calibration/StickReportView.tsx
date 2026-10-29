import type { CaptureSample, StickReport, TriggerAnalysis } from '@/api/types';
import { Icon } from '@/app/icons';
import { useT, type TKey } from '@/i18n';
import ui from '@/styles/ui.module.css';
import { CoverageChart } from './CoverageChart';
import { compareRows } from './report';
import css from './StickReportView.module.css';

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className={css.row}>
      <dt>{label}</dt>
      <dd className="mono">{value}</dd>
    </div>
  );
}

export function StickReportView({
  report,
  after,
  rotation,
  triggers,
}: {
  report: StickReport;
  after?: StickReport | null;
  rotation: CaptureSample[];
  triggers?: { left: TriggerAnalysis | null; right: TriggerAnalysis | null } | null;
}) {
  const t = useT();
  const stickName = t(
    report.stick === 'left' ? 'calibration.wizard.left' : 'calibration.wizard.right',
  );
  const { rest, circularity: circ, directions: dirs, spikes } = report;
  const pct = (v: number, d = 1) => `${v.toFixed(d)}%`;
  return (
    <div className={css.grid} data-testid="stick-report">
      <section className={`${ui.panel} ${css.score}`}>
        <h2 className={ui.panelTitle}>{t('calibration.report.title', { stick: stickName })}</h2>
        <div className={css.scoreLine}>
          <span className={css.big} data-grade={report.score.grade}>
            {report.score.grade}
          </span>
          <div>
            <p className={css.scoreNum}>
              {report.score.score} <small>{t('calibration.report.of100')}</small>
            </p>
            <p className={ui.faint}>{t('calibration.report.score')}</p>
          </div>
        </div>
        {report.score.issues.length ? (
          <ul className={css.issues}>
            {report.score.issues.map((i) => (
              <li key={i}>
                <Icon name="alert" size={14} /> {t(`calibration.report.issues.${i}` as TKey)}
              </li>
            ))}
          </ul>
        ) : (
          <p className={css.ok}>
            <Icon name="check" size={14} /> {t('calibration.report.noIssues')}
          </p>
        )}
        <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
          {t('calibration.report.hardwareNote')}
        </p>
      </section>

      {circ && (
        <section className={`${ui.panel} ${css.chart}`}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.report.circ.title')}</h3>
          </div>
          <CoverageChart
            samples={rotation}
            center={rest?.mean ?? { x: 0, y: 0 }}
            before={circ}
            after={after?.circularity ?? null}
            calibration={after ? after.suggested : null}
          />
          <dl className={css.list}>
            <Row label={t('calibration.report.circ.roundness')} value={pct(circ.roundnessPct)} />
            <Row label={t('calibration.report.circ.error')} value={pct(circ.avgErrorPct)} />
            <Row
              label={t('calibration.report.circ.radii')}
              value={`${circ.radiusMin.toFixed(3)} / ${circ.radiusAvg.toFixed(3)} / ${circ.radiusMax.toFixed(3)}`}
            />
            <Row label={t('calibration.report.circ.coverage')} value={pct(circ.coveragePct, 0)} />
            <Row
              label={t('calibration.report.circ.failing')}
              value={
                circ.failingSectors.length
                  ? circ.failingSectors
                      .map((i) => `${Math.round(circ.sectors[i].angleDeg)}°`)
                      .join(', ')
                  : t('calibration.report.circ.none')
              }
            />
          </dl>
          {circ.incompleteTravel && (
            <p className={`${ui.callout} ${ui.calloutWarn}`}>
              {t('calibration.report.circ.incomplete')}
            </p>
          )}
        </section>
      )}

      {rest && (
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.report.rest.title')}</h3>
            <span className={ui.spacer} />
            <span
              className={`${ui.chip} ${rest.grade === 'excellent' || rest.grade === 'acceptable' ? ui.chipOk : rest.grade === 'poor' ? ui.chipWarn : ui.chipBad}`}
            >
              {t(`calibration.report.rest.grades.${rest.grade}`)}
            </span>
          </div>
          <dl className={css.list}>
            <Row label={t('calibration.report.rest.offset')} value={pct(rest.offsetPct, 2)} />
            <Row
              label=""
              value={t('calibration.report.rest.offsetXY', {
                x: pct(rest.offsetPctX, 2),
                y: pct(rest.offsetPctY, 2),
              })}
            />
            <Row label={t('calibration.report.rest.noise')} value={rest.stdRadial.toFixed(4)} />
            <Row label={t('calibration.report.rest.maxDev')} value={rest.maxDeviation.toFixed(4)} />
            <Row
              label={t('calibration.report.rest.rate')}
              value={`${rest.driftRatePerSec.toFixed(4)}/s`}
            />
            <Row
              label={t('calibration.report.rest.suggestedDz')}
              value={pct(rest.suggestedDeadzone * 100, 1)}
            />
          </dl>
        </section>
      )}

      {dirs && (
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.report.dirs.title')}</h3>
            <span className={ui.spacer} />
            <span className={ui.chip}>
              {t('calibration.report.dirs.gate')}: {t(`calibration.report.gates.${dirs.gateShape}`)}
            </span>
          </div>
          <table className={ui.table}>
            <thead>
              <tr>
                <th />
                <th>{t('calibration.report.dirs.reach')}</th>
                <th>{t('calibration.report.dirs.aim')}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {dirs.directions.map((d) => (
                <tr key={d.name}>
                  <td style={{ fontFamily: 'var(--font-ui)' }}>
                    {t(`calibration.wizard.directions.names.${d.name}` as TKey)}
                  </td>
                  <td>{d.samples ? d.reach.toFixed(3) : '—'}</td>
                  <td>{d.angleErrorDeg != null ? `${d.angleErrorDeg.toFixed(1)}°` : '—'}</td>
                  <td>{d.ok ? <Icon name="check" size={14} /> : <Icon name="x" size={14} />}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)', marginTop: 8 }}>
            {t('calibration.report.dirs.ratio')}:{' '}
            <span className="mono">{dirs.diagonalRatio.toFixed(3)}</span>
          </p>
        </section>
      )}

      {spikes && (
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.report.spikes.title')}</h3>
            <span className={ui.spacer} />
            <span
              className={`${ui.chip} ${spikes.verdict === 'clean' ? ui.chipOk : spikes.verdict === 'noisy' ? ui.chipWarn : ui.chipBad}`}
            >
              {t(`calibration.report.spikes.verdicts.${spikes.verdict}`)}
            </span>
          </div>
          <dl className={css.list}>
            <Row label={t('calibration.report.spikes.count')} value={String(spikes.spikeCount)} />
            <Row label={t('calibration.report.spikes.jumps')} value={String(spikes.jumpCount)} />
            <Row label={t('calibration.report.spikes.noise')} value={spikes.noiseRms.toFixed(4)} />
          </dl>
        </section>
      )}

      {triggers && (triggers.left || triggers.right) && (
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.report.triggers.title')}</h3>
          </div>
          {(['left', 'right'] as const).map((side) => {
            const a = triggers[side];
            if (!a) return null;
            return (
              <dl key={side} className={css.list}>
                <Row label={side === 'left' ? 'L2 / LT' : 'R2 / RT'} value="" />
                <Row
                  label={t('calibration.report.triggers.rest')}
                  value={`${a.restMin.toFixed(3)}–${a.restMax.toFixed(3)}`}
                />
                <Row label={t('calibration.report.triggers.max')} value={pct(a.maxReached * 100)} />
                <Row
                  label={t('calibration.report.triggers.suggested')}
                  value={pct(a.suggestedDeadzone * 100)}
                />
                <Row
                  label={t('calibration.report.triggers.full')}
                  value={
                    a.fullTravel
                      ? '✓'
                      : t('calibration.report.triggers.incomplete', {
                          pct: pct(a.suggestedMax * 100),
                        })
                  }
                />
              </dl>
            );
          })}
        </section>
      )}

      {after && (
        <section className={`${ui.panel} ${css.compare}`} data-testid="compare">
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.report.compare.title')}</h3>
          </div>
          <table className={ui.table}>
            <thead>
              <tr>
                <th>{t('calibration.report.compare.metric')}</th>
                <th>{t('calibration.report.compare.before')}</th>
                <th>{t('calibration.report.compare.after')}</th>
              </tr>
            </thead>
            <tbody>
              {compareRows(report, after).map((r) => (
                <tr key={r.key}>
                  <td style={{ fontFamily: 'var(--font-ui)' }}>{t(r.key as TKey)}</td>
                  <td>{r.before}</td>
                  <td className={css.afterCell}>{r.after}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </div>
  );
}
