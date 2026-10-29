import { useEffect, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type { ControllerInfo, RumblePatternKind, RumbleRequest } from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { Field, Slider } from '@/app/controls';
import { Icon } from '@/app/icons';
import { useT, type TKey } from '@/i18n';
import { begin, useHiDpiCanvas, usePalette } from '@/lib/canvas';
import { envelope, PATTERNS } from '@/lib/haptics';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import { DualSenseExtras } from './DualSenseExtras';
import css from './VibrationPanel.module.css';

function Preview({ req }: { req: RumbleRequest }) {
  const { ref, size } = useHiDpiCanvas();
  const p = usePalette();
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas || !size.w) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const { w, h } = size;
    ctx.clearRect(0, 0, w, h);
    ctx.strokeStyle = p.grid;
    for (let i = 0; i <= 4; i++) {
      const y = (i / 4) * (h - 8) + 4;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }
    const plot = (amp: number, color: string) => {
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      for (let x = 0; x <= w; x++) {
        const tms = (x / w) * req.durationMs;
        const y = h - 4 - envelope(req, Math.min(tms, req.durationMs - 0.001)) * amp * (h - 8);
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
    };
    plot(req.strong, p.alt);
    plot(req.weak, p.cal);
  }, [req, size, p, ref]);
  return <canvas ref={ref} className={css.preview} aria-hidden="true" />;
}

export function LiveRumble({ id }: { id: string }) {
  const t = useT();
  const r = useApp((s) => s.rumble[id]);
  const active = !!r?.active && (r.weak > 0.01 || r.strong > 0.01);
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  return (
    <div className={css.live} data-active={active || undefined} role="status" aria-live="polite">
      <span className={css.liveLabel}>
        <Icon name="vibrate" size={16} /> {t('test.vibration.live')}
      </span>
      <div className={css.meters}>
        <div className={css.meter}>
          <span>{t('test.vibration.weak')}</span>
          <div>
            <i style={{ transform: `scaleX(${r?.weak ?? 0})`, background: 'var(--sig-cal)' }} />
          </div>
        </div>
        <div className={css.meter}>
          <span>{t('test.vibration.strong')}</span>
          <div>
            <i style={{ transform: `scaleX(${r?.strong ?? 0})`, background: 'var(--sig-alt)' }} />
          </div>
        </div>
      </div>
      <span className={css.liveText}>
        {active
          ? t('test.vibration.active', { weak: pct(r.weak), strong: pct(r.strong) })
          : t('test.vibration.idle')}
      </span>
    </div>
  );
}

export function VibrationPanel({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const [req, setReq] = useState<RumbleRequest>({
    weak: 0.6,
    strong: 0.8,
    durationMs: 1500,
    pattern: 'constant',
    periodMs: 300,
  });
  const set = (patch: Partial<RumbleRequest>) => setReq((r) => ({ ...r, ...patch }));
  const run = async (fn: () => Promise<void>) => {
    try {
      await fn();
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  };
  const periodic = req.pattern === 'pulse' || req.pattern === 'wave' || req.pattern === 'heartbeat';

  return (
    <div className={css.grid}>
      <section className={ui.panel} aria-labelledby="vib-title">
        <div className={ui.panelHead}>
          <h2 id="vib-title" className={ui.panelTitle}>
            {t('test.vibration.title')}
          </h2>
        </div>
        {!c.capabilities.rumble ? (
          <p className={ui.muted}>{t('test.vibration.noRumble')}</p>
        ) : (
          <div className={css.form}>
            <Slider
              label={t('test.vibration.weak')}
              value={req.weak}
              min={0}
              max={1}
              step={0.01}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(weak) => set({ weak })}
            />
            <Slider
              label={t('test.vibration.strong')}
              value={req.strong}
              min={0}
              max={1}
              step={0.01}
              format={(v) => `${Math.round(v * 100)}%`}
              onChange={(strong) => set({ strong })}
            />
            <Slider
              label={t('test.vibration.duration')}
              value={req.durationMs}
              min={100}
              max={5000}
              step={50}
              format={(v) => `${(v / 1000).toFixed(2)} s`}
              onChange={(durationMs) => set({ durationMs })}
            />
            <Field label={t('test.vibration.pattern')}>
              <div
                className={css.patterns}
                role="radiogroup"
                aria-label={t('test.vibration.pattern')}
              >
                {PATTERNS.map((p) => (
                  <button
                    key={p}
                    type="button"
                    role="radio"
                    aria-checked={req.pattern === p}
                    className={`${ui.btn} ${ui.small}`}
                    onClick={() => set({ pattern: p as RumblePatternKind })}
                  >
                    {t(`test.vibration.patterns.${p}` as TKey)}
                  </button>
                ))}
              </div>
            </Field>
            <Slider
              label={t('test.vibration.period')}
              value={req.periodMs}
              min={60}
              max={2000}
              step={10}
              disabled={!periodic}
              format={(v) => `${v} ms`}
              onChange={(periodMs) => set({ periodMs })}
            />
            <Field label={t('test.vibration.preview')}>
              <Preview req={req} />
            </Field>
            <div className={css.actions}>
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                onClick={() => run(() => api.rumblePlay(c.id, req))}
                data-testid="rumble-play"
              >
                <Icon name="vibrate" size={16} /> {t('test.vibration.play')}
              </button>
              <button
                type="button"
                className={ui.btn}
                onClick={() => run(() => api.rumbleStop(c.id))}
              >
                <Icon name="stop" size={16} /> {t('test.vibration.stop')}
              </button>
              <button
                type="button"
                className={ui.btn}
                onClick={() => run(() => api.rumbleMotorTest(c.id, 0.8))}
                title={t('test.vibration.motorTestHint')}
              >
                {t('test.vibration.motorTest')}
              </button>
            </div>
            <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
              {t('test.vibration.motorTestHint')}
            </p>
          </div>
        )}
      </section>
      <div className={css.side}>
        <section className={ui.panel}>
          <LiveRumble id={c.id} />
          <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)', marginTop: 12 }}>
            {t('test.vibration.visualNote')}
          </p>
        </section>
        {(c.capabilities.adaptiveTriggers || c.capabilities.lightbar) && <DualSenseExtras c={c} />}
      </div>
    </div>
  );
}
