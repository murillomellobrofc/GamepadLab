import { useEffect, useRef, useState } from 'react';
import type { ControllerInfo, PollStats } from '@/api/types';
import { Icon } from '@/app/icons';
import { useT } from '@/i18n';
import { begin, useHiDpiCanvas, usePalette } from '@/lib/canvas';
import { live, useAnimationFrame, useLive } from '@/state/live';
import ui from '@/styles/ui.module.css';
import { ControllerHeader } from '../common/ControllerHeader';
import css from './LatencyView.module.css';
import { histogram, summarizeIntervals } from './latency';
import { downsample, saveSession } from '@/lib/history';
import { SaveToHistory } from '@/app/SaveToHistory';

const WINDOW_US = 5_000_000;

function useIntervalHistory(id: string) {
  const points = useRef<{ t: number; ms: number }[]>([]);
  useEffect(() => {
    points.current = [];
    let clock = 0;
    return live.subscribe(() => {
      const e = live.get(id);
      if (!e) return;
      for (const us of e.stats.recentIntervalsUs) {
        clock += us;
        points.current.push({ t: clock, ms: us / 1000 });
      }
      const cutoff = clock - WINDOW_US;
      while (points.current.length && points.current[0].t < cutoff) points.current.shift();
    });
  }, [id]);
  return points;
}

function Chart({ id }: { id: string }) {
  const t = useT();
  const { ref } = useHiDpiCanvas();
  const p = usePalette();
  const pts = useIntervalHistory(id);
  useAnimationFrame(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const r = canvas.getBoundingClientRect();
    const w = r.width;
    const h = r.height;
    ctx.clearRect(0, 0, w, h);
    const data = pts.current;
    const maxMs = Math.max(4, ...data.map((d) => d.ms)) * 1.15;
    ctx.strokeStyle = p.grid;
    ctx.fillStyle = p.faint;
    ctx.font = '10px ' + getComputedStyle(document.body).getPropertyValue('--font-mono');
    for (let i = 0; i <= 4; i++) {
      const y = h - 18 - (i / 4) * (h - 28);
      ctx.beginPath();
      ctx.moveTo(36, y);
      ctx.lineTo(w, y);
      ctx.stroke();
      ctx.fillText(`${((i / 4) * maxMs).toFixed(1)}`, 2, y + 3);
    }
    if (!data.length) {
      ctx.fillStyle = p.muted;
      ctx.fillText(t('latency.noData'), 44, h / 2);
      return;
    }
    const t1 = data[data.length - 1].t;
    const x = (tt: number) => 36 + ((tt - (t1 - WINDOW_US)) / WINDOW_US) * (w - 36);
    const y = (ms: number) => h - 18 - (ms / maxMs) * (h - 28);
    ctx.strokeStyle = p.accent;
    ctx.lineWidth = 1.25;
    ctx.beginPath();
    data.forEach((d, i) => (i ? ctx.lineTo(x(d.t), y(d.ms)) : ctx.moveTo(x(d.t), y(d.ms))));
    ctx.stroke();
    ctx.fillStyle = p.accent;
    for (const d of data) ctx.fillRect(x(d.t) - 1, y(d.ms) - 1, 2, 2);
  });
  return <canvas ref={ref} className={css.chart} role="img" aria-label={t('latency.chart')} />;
}

function Histogram({ id }: { id: string }) {
  const t = useT();
  const { ref } = useHiDpiCanvas();
  const p = usePalette();
  const pts = useIntervalHistory(id);
  useAnimationFrame(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const r = canvas.getBoundingClientRect();
    ctx.clearRect(0, 0, r.width, r.height);
    const bins = histogram(
      pts.current.map((d) => d.ms),
      24,
    );
    if (!bins.counts.length) return;
    const max = Math.max(...bins.counts, 1);
    const bw = r.width / bins.counts.length;
    ctx.fillStyle = p.raw;
    bins.counts.forEach((c, i) => {
      const bh = (c / max) * (r.height - 16);
      ctx.fillRect(i * bw + 1, r.height - 14 - bh, bw - 2, bh);
    });
    ctx.fillStyle = p.faint;
    ctx.font = '10px ' + getComputedStyle(document.body).getPropertyValue('--font-mono');
    ctx.fillText(`${bins.min.toFixed(2)} ms`, 2, r.height - 2);
    const label = `${bins.max.toFixed(2)} ms`;
    ctx.fillText(label, r.width - ctx.measureText(label).width - 2, r.height - 2);
  });
  return (
    <canvas ref={ref} className={css.histogram} role="img" aria-label={t('latency.histogram')} />
  );
}

function Stat({ label, value, unit }: { label: string; value: string; unit?: string }) {
  return (
    <div className={ui.stat}>
      <span className={ui.statLabel}>{label}</span>
      <span className={ui.statValue}>
        {value}
        {unit && <span className={ui.statUnit}>{unit}</span>}
      </span>
    </div>
  );
}

function Measurement({ c }: { c: ControllerInfo }) {
  const t = useT();
  const [left, setLeft] = useState<number | null>(null);
  const [result, setResult] = useState<ReturnType<typeof summarizeIntervals> | null>(null);
  const collected = useRef<number[]>([]);
  useEffect(() => {
    if (left === null) return;
    const unsub = live.subscribe(() => {
      const e = live.get(c.id);
      if (e) collected.current.push(...e.stats.recentIntervalsUs);
    });
    const timer = window.setInterval(() => {
      setLeft((s) => {
        if (s === null) return null;
        if (s <= 1) {
          setResult(summarizeIntervals(collected.current));
          return null;
        }
        return s - 1;
      });
    }, 1000);
    return () => {
      unsub();
      window.clearInterval(timer);
    };
  }, [left === null, c.id]); // eslint-disable-line react-hooks/exhaustive-deps
  return (
    <section className={ui.panel}>
      <div className={ui.panelHead}>
        <h2 className={ui.panelTitle}>{t('latency.result')}</h2>
        <span className={ui.spacer} />
        <button
          type="button"
          className={`${ui.btn} ${ui.primary} ${ui.small}`}
          disabled={left !== null}
          onClick={() => {
            collected.current = [];
            setResult(null);
            setLeft(10);
          }}
          data-testid="latency-measure"
        >
          <Icon name="activity" size={14} />
          {left !== null ? t('latency.measuring', { s: left }) : t('latency.measure')}
        </button>
      </div>
      {result ? (
        <div className={css.stats} data-testid="latency-result">
          <Stat label={t('latency.rate')} value={result.hz.toFixed(0)} unit="Hz" />
          <Stat label={t('latency.avg')} value={result.avgMs.toFixed(2)} unit="ms" />
          <Stat label={t('latency.jitter')} value={result.jitterMs.toFixed(2)} unit="ms" />
          <Stat label={t('latency.min')} value={result.minMs.toFixed(2)} unit="ms" />
          <Stat label={t('latency.max')} value={result.maxMs.toFixed(2)} unit="ms" />
          <Stat label={t('latency.p99')} value={result.p99Ms.toFixed(2)} unit="ms" />
          <p className={ui.faint}>{t('latency.samples', { n: result.samples })}</p>
          <SaveToHistory
            label={t('latency.save')}
            onSave={() =>
              saveSession(
                c,
                'latency',
                null,
                `${result.hz.toFixed(0)} Hz · jitter ${result.jitterMs.toFixed(2)} ms`,
                { summary: result, intervalsUs: downsample(collected.current, 5000) },
              )
            }
          />
        </div>
      ) : (
        <p className={ui.faint}>{t('latency.explain')}</p>
      )}
    </section>
  );
}

export function LatencyView({ c }: { c: ControllerInfo }) {
  const t = useT();
  const stats = useLive<PollStats>(
    c.id,
    (e) => e.stats,
    4,
    () => false,
  );
  const jitterRatio = stats && stats.avgMs ? stats.jitterMs / stats.avgMs : 0;
  return (
    <>
      <ControllerHeader c={c} />
      <div className={css.layout}>
        <section className={`${ui.panel} ${css.live}`}>
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t('latency.title')}</h2>
            <span className={ui.channel}>LIVE</span>
            <span className={ui.spacer} />
            {stats && stats.samples > 10 && (
              <span className={`${ui.chip} ${jitterRatio < 0.15 ? ui.chipOk : ui.chipWarn}`}>
                {jitterRatio < 0.15 ? t('latency.stable') : t('latency.unstable')}
              </span>
            )}
          </div>
          <div className={css.stats} data-testid="latency-live">
            <Stat
              label={t('latency.rate')}
              value={stats?.hz ? stats.hz.toFixed(0) : '—'}
              unit="Hz"
            />
            <Stat
              label={t('latency.avg')}
              value={stats?.avgMs ? stats.avgMs.toFixed(2) : '—'}
              unit="ms"
            />
            <Stat
              label={t('latency.jitter')}
              value={stats?.samples ? stats.jitterMs.toFixed(2) : '—'}
              unit="ms"
            />
            <Stat
              label={t('latency.min')}
              value={stats?.samples ? stats.minMs.toFixed(2) : '—'}
              unit="ms"
            />
            <Stat
              label={t('latency.max')}
              value={stats?.samples ? stats.maxMs.toFixed(2) : '—'}
              unit="ms"
            />
            <Stat label={t('latency.reports')} value={String(stats?.reportCount ?? 0)} />
          </div>
          <p
            className={`${ui.callout} ${stats?.source === 'change' ? ui.calloutWarn : ui.calloutInfo}`}
          >
            <Icon name="info" />
            {stats?.source === 'change' ? t('latency.sourceChange') : t('latency.sourceReport')}
          </p>
          <h3 className={css.sub}>{t('latency.chart')}</h3>
          <Chart id={c.id} />
          <h3 className={css.sub}>{t('latency.histogram')}</h3>
          <Histogram id={c.id} />
        </section>
        <Measurement c={c} />
      </div>
    </>
  );
}
