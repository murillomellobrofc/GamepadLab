import { useEffect, useRef, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type { ControllerInfo } from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { Icon } from '@/app/icons';
import { useT } from '@/i18n';
import { begin, useHiDpiCanvas, usePalette } from '@/lib/canvas';
import { useApp } from '@/state/app';
import { live, useAnimationFrame, useLive } from '@/state/live';
import ui from '@/styles/ui.module.css';
import { ControllerHeader } from '../common/ControllerHeader';
import { ComplementaryFilter } from './orientation';
import css from './SensorsView.module.css';

const HISTORY = 360;

function useMotionHistory(id: string) {
  const buf = useRef<{ g: number[][]; a: number[][] }>({ g: [], a: [] });
  useEffect(() => {
    buf.current = { g: [], a: [] };
    let lastT = -1;
    return live.subscribe(() => {
      const m = live.get(id)?.snapshot.motion;
      if (!m || m.timestampUs === lastT) return;
      lastT = m.timestampUs;
      buf.current.g.push([...m.gyro]);
      buf.current.a.push([...m.accel]);
      if (buf.current.g.length > HISTORY) {
        buf.current.g.shift();
        buf.current.a.shift();
      }
    });
  }, [id]);
  return buf;
}

function ImuChart({ id, kind }: { id: string; kind: 'g' | 'a' }) {
  const t = useT();
  const { ref } = useHiDpiCanvas();
  const p = usePalette();
  const buf = useMotionHistory(id);
  const colors = [p.raw, p.cal, p.alt];
  useAnimationFrame(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const { width: w, height: h } = canvas.getBoundingClientRect();
    ctx.clearRect(0, 0, w, h);
    const data = buf.current[kind];
    const range =
      kind === 'g'
        ? Math.max(60, ...data.flat().map(Math.abs))
        : Math.max(1.2, ...data.flat().map(Math.abs));
    ctx.strokeStyle = p.grid;
    for (let i = 0; i <= 4; i++) {
      const y = (i / 4) * h;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }
    for (let axis = 0; axis < 3; axis++) {
      ctx.strokeStyle = colors[axis];
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      data.forEach((v, i) => {
        const x = (i / (HISTORY - 1)) * w;
        const y = h / 2 - (v[axis] / range) * (h / 2 - 4);
        if (i) ctx.lineTo(x, y);
        else ctx.moveTo(x, y);
      });
      ctx.stroke();
    }
    ctx.fillStyle = p.faint;
    ctx.font = '10px monospace';
    ctx.fillText(`±${range.toFixed(kind === 'g' ? 0 : 2)} ${kind === 'g' ? '°/s' : 'g'}`, 4, 12);
  });
  return (
    <canvas
      ref={ref}
      className={css.chart}
      role="img"
      aria-label={kind === 'g' ? t('sensors.gyro') : t('sensors.accel')}
    />
  );
}

function Readout({ id }: { id: string }) {
  const t = useT();
  const v = useLive(
    id,
    (e) => e.snapshot.motion,
    8,
    () => false,
  );
  if (!v) return null;
  const f = (n: number, d: number) => (n < 0 ? '' : '+') + n.toFixed(d);
  return (
    <div className={css.readout}>
      {(['x', 'y', 'z'] as const).map((a, i) => (
        <div key={a}>
          <span className={css.axis} data-axis={i}>
            {t(`sensors.axes.${a}`)}
          </span>
          <span className="mono">
            {f(v.gyro[i], 1)}°/s · {f(v.accel[i], 3)}g
          </span>
        </div>
      ))}
    </div>
  );
}

function Orientation3D({ id }: { id: string }) {
  const t = useT();
  const filter = useRef(new ComplementaryFilter());
  const body = useRef<HTMLDivElement>(null);
  const [angles, setAngles] = useState({ pitch: 0, roll: 0, yaw: 0 });
  useEffect(() => {
    let last = 0;
    return live.subscribe(() => {
      const m = live.get(id)?.snapshot.motion;
      if (!m) return;
      const o = filter.current.update(m.gyro, m.accel, m.timestampUs);
      if (body.current)
        body.current.style.transform = `rotateX(${-o.pitch}deg) rotateY(${o.yaw}deg) rotateZ(${o.roll}deg)`;
      const now = performance.now();
      if (now - last > 250) {
        last = now;
        setAngles({ ...o });
      }
    });
  }, [id]);
  return (
    <div className={css.orient}>
      <div className={css.stage} aria-hidden="true">
        <div ref={body} className={css.pad}>
          <div className={`${css.face} ${css.top}`}>
            <span className={css.grip} />
            <span className={css.grip} />
            <i />
          </div>
          <div className={`${css.face} ${css.front}`} />
          <div className={`${css.face} ${css.back}`} />
          <div className={`${css.face} ${css.left}`} />
          <div className={`${css.face} ${css.right}`} />
          <div className={`${css.face} ${css.bottom}`} />
        </div>
      </div>
      <dl className={css.angles}>
        <div>
          <dt>{t('sensors.pitch')}</dt>
          <dd className="mono">{angles.pitch.toFixed(1)}°</dd>
        </div>
        <div>
          <dt>{t('sensors.roll')}</dt>
          <dd className="mono">{angles.roll.toFixed(1)}°</dd>
        </div>
        <div>
          <dt>{t('sensors.yaw')}</dt>
          <dd className="mono">{angles.yaw.toFixed(1)}°</dd>
        </div>
      </dl>
      <button
        type="button"
        className={`${ui.btn} ${ui.small}`}
        title={t('sensors.tareHint')}
        onClick={() => filter.current.tare(live.get(id)?.snapshot.motion?.gyro ?? null)}
      >
        <Icon name="refresh" size={14} /> {t('sensors.tare')}
      </button>
    </div>
  );
}

function Touchpad({ id }: { id: string }) {
  const t = useT();
  const { ref } = useHiDpiCanvas();
  const p = usePalette();
  const trails = useRef<{ x: number; y: number }[][]>([[], []]);
  const info = useLive(
    id,
    (e) => {
      const tch = e.snapshot.touch;
      return tch ? `${tch.pressed ? 1 : 0}${tch.points.filter((q) => q.active).length}` : '00';
    },
    10,
  );
  useAnimationFrame(() => {
    const canvas = ref.current;
    const touch = live.get(id)?.snapshot.touch;
    if (!canvas) return;
    const ctx = begin(canvas);
    if (!ctx) return;
    const { width: w, height: h } = canvas.getBoundingClientRect();
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = touch?.pressed ? p.cal : p.surface;
    ctx.globalAlpha = touch?.pressed ? 0.15 : 1;
    ctx.fillRect(0, 0, w, h);
    ctx.globalAlpha = 1;
    ctx.strokeStyle = p.gridStrong;
    for (let i = 1; i < 4; i++) {
      ctx.beginPath();
      ctx.moveTo((i / 4) * w, 0);
      ctx.lineTo((i / 4) * w, h);
      ctx.stroke();
    }
    touch?.points.forEach((pt, i) => {
      const tr = trails.current[i] ?? (trails.current[i] = []);
      if (pt.active) tr.push({ x: pt.x * w, y: pt.y * h });
      else tr.length = 0;
      if (tr.length > 40) tr.shift();
      ctx.strokeStyle = i ? p.alt : p.accent;
      ctx.lineWidth = 3;
      ctx.globalAlpha = 0.5;
      ctx.beginPath();
      tr.forEach((q, j) => (j ? ctx.lineTo(q.x, q.y) : ctx.moveTo(q.x, q.y)));
      ctx.stroke();
      ctx.globalAlpha = 1;
      if (pt.active) {
        ctx.fillStyle = i ? p.alt : p.accent;
        ctx.beginPath();
        ctx.arc(pt.x * w, pt.y * h, 12, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = p.bg;
        ctx.font = 'bold 11px sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText(String(i + 1), pt.x * w, pt.y * h + 4);
      }
    });
  });
  const pressed = info?.[0] === '1';
  const n = Number(info?.[1] ?? 0);
  return (
    <div className={css.touch}>
      <canvas ref={ref} className={css.touchCanvas} role="img" aria-label={t('sensors.touchpad')} />
      <p className={css.touchInfo} aria-live="polite">
        {t('sensors.touches', { n })}{' '}
        {pressed && <span className={`${ui.chip} ${ui.chipOk}`}>{t('sensors.pressed')}</span>}
      </p>
    </div>
  );
}

export function SensorsView({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const hasMotion = useLive(c.id, (e) => !!e.snapshot.motion, 2) ?? false;
  const showSwitch = c.kind === 'switchPro' && c.backend !== 'hid';
  return (
    <>
      <ControllerHeader c={c} />
      {showSwitch && (
        <div className={`${ui.callout} ${ui.calloutInfo}`} style={{ marginBottom: 16 }}>
          <Icon name="info" />
          <div>
            <p>{t('sensors.enableSwitchHint')}</p>
            <button
              type="button"
              className={`${ui.btn} ${ui.small}`}
              style={{ marginTop: 8 }}
              onClick={async () => {
                try {
                  await api.enableSensors(c.id);
                  toast({ tone: 'ok', title: t('sensors.enabled') });
                } catch (e) {
                  toast(errorToast(errorCode(e), errorMessage(e)));
                }
              }}
            >
              {t('sensors.enableSwitch')}
            </button>
          </div>
        </div>
      )}
      {!hasMotion && !c.capabilities.gyro ? (
        <p className={ui.muted}>{t('sensors.none')}</p>
      ) : (
        <div className={css.grid}>
          <section className={ui.panel}>
            <div className={ui.panelHead}>
              <h2 className={ui.panelTitle}>{t('sensors.orientation')}</h2>
            </div>
            <Orientation3D id={c.id} />
          </section>
          <section className={ui.panel}>
            <div className={ui.panelHead}>
              <h2 className={ui.panelTitle}>{t('sensors.gyro')}</h2>
              <span className={ui.channel}>°/s</span>
            </div>
            <ImuChart id={c.id} kind="g" />
            <h2 className={ui.panelTitle} style={{ margin: '16px 0 8px' }}>
              {t('sensors.accel')}
            </h2>
            <ImuChart id={c.id} kind="a" />
            <Readout id={c.id} />
          </section>
          <section className={`${ui.panel} ${css.wide}`}>
            <div className={ui.panelHead}>
              <h2 className={ui.panelTitle}>{t('sensors.touchpad')}</h2>
            </div>
            {c.capabilities.touchpad ? (
              <Touchpad id={c.id} />
            ) : (
              <p className={ui.muted}>{t('sensors.noTouch')}</p>
            )}
          </section>
        </div>
      )}
    </>
  );
}
