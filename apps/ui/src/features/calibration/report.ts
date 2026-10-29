import type { StickReport } from '@/api/types';

/** Rows of the before/after comparison table. */
export function compareRows(before: StickReport, after: StickReport) {
  const pct = (v: number | undefined, d = 1) => (v == null ? '—' : `${v.toFixed(d)}%`);
  const num = (v: number | undefined, d = 3) => (v == null ? '—' : v.toFixed(d));
  return [
    {
      key: 'calibration.report.rest.offset',
      before: pct(before.rest?.offsetPct, 2),
      after: pct(after.rest?.offsetPct, 2),
    },
    {
      key: 'calibration.report.rest.maxDev',
      before: num(before.rest?.maxDeviation),
      after: num(after.rest?.maxDeviation),
    },
    {
      key: 'calibration.report.circ.roundness',
      before: pct(before.circularity?.roundnessPct),
      after: pct(after.circularity?.roundnessPct),
    },
    {
      key: 'calibration.report.circ.error',
      before: pct(before.circularity?.avgErrorPct),
      after: pct(after.circularity?.avgErrorPct),
    },
    {
      key: 'calibration.report.circ.radii',
      before: before.circularity
        ? `${num(before.circularity.radiusMin, 2)} / ${num(before.circularity.radiusAvg, 2)} / ${num(before.circularity.radiusMax, 2)}`
        : '—',
      after: after.circularity
        ? `${num(after.circularity.radiusMin, 2)} / ${num(after.circularity.radiusAvg, 2)} / ${num(after.circularity.radiusMax, 2)}`
        : '—',
    },
    {
      key: 'calibration.report.score',
      before: `${before.score.score} (${before.score.grade})`,
      after: `${after.score.score} (${after.score.grade})`,
    },
  ] as const;
}
