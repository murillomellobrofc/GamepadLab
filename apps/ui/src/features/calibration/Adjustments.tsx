import { useRef, useState } from 'react';
import type {
  ControllerInfo,
  CurveKind,
  DeadzoneMode,
  GateShape,
  StickCalibration,
  TriggerCalibration,
} from '@/api/types';
import { Field, Segmented, Slider, Switch } from '@/app/controls';
import { useT } from '@/i18n';
import { applyTrigger, defaultStick } from '@/lib/calibration';
import { live, useAnimationFrame } from '@/state/live';
import { useActiveProfile, useProfiles } from '@/state/profiles';
import ui from '@/styles/ui.module.css';
import css from './Adjustments.module.css';
import { CurveEditor } from './CurveEditor';
import { StickPreview } from './StickPreview';

type Part = 'left' | 'right' | 'triggers';
const pct = (v: number) => `${Math.round(v * 100)}%`;

function StickEditor({ c, side }: { c: ControllerInfo; side: 'left' | 'right' }) {
  const t = useT();
  const profile = useActiveProfile();
  const edit = useProfiles((s) => s.edit);
  if (!profile) return null;
  const key = side === 'left' ? 'leftStick' : 'rightStick';
  const cal = profile.calibration[key];
  const set = (patch: Partial<StickCalibration>) =>
    edit((p) => ({ ...p, [key]: { ...p[key], ...patch } }));

  return (
    <div className={css.stick}>
      <div className={css.visuals}>
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.adjust.preview')}</h3>
          </div>
          <StickPreview id={c.id} side={side} cal={cal} />
        </section>
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h3 className={ui.panelTitle}>{t('calibration.adjust.curveEditor')}</h3>
          </div>
          <CurveEditor cal={cal} onChange={(curve) => set({ curve })} />
        </section>
      </div>
      <section className={`${ui.panel} ${css.controls}`} aria-label={t('calibration.adjust.title')}>
        <Slider
          label={t('calibration.adjust.innerDz')}
          value={cal.innerDeadzone}
          min={0}
          max={0.5}
          step={0.005}
          format={pct}
          onChange={(innerDeadzone) => set({ innerDeadzone })}
        />
        <Slider
          label={t('calibration.adjust.outerDz')}
          value={cal.outerDeadzone}
          min={0}
          max={0.3}
          step={0.005}
          format={pct}
          onChange={(outerDeadzone) => set({ outerDeadzone })}
        />
        <Field label={t('calibration.adjust.mode')}>
          <Segmented<DeadzoneMode>
            label={t('calibration.adjust.mode')}
            value={cal.deadzoneMode}
            onChange={(deadzoneMode) => set({ deadzoneMode })}
            options={[
              { value: 'radial', label: t('calibration.adjust.radial') },
              { value: 'axial', label: t('calibration.adjust.axial') },
            ]}
          />
        </Field>
        <Field label={t('calibration.adjust.curve')}>
          <Segmented<CurveKind>
            label={t('calibration.adjust.curve')}
            value={cal.curve.kind}
            onChange={(kind) =>
              set({
                curve:
                  kind === 'custom'
                    ? {
                        kind,
                        points: cal.curve.points.length
                          ? cal.curve.points
                          : [
                              { x: 0.3, y: 0.15 },
                              { x: 0.7, y: 0.6 },
                            ],
                      }
                    : { kind, points: cal.curve.points },
              })
            }
            options={(['linear', 'smooth', 'aggressive', 'sCurve', 'custom'] as CurveKind[]).map(
              (k) => ({
                value: k,
                label: t(`calibration.adjust.curves.${k}`),
              }),
            )}
          />
        </Field>
        <div className={css.pair}>
          <Slider
            label={t('calibration.adjust.sensX')}
            value={cal.sensitivity.x}
            min={0.1}
            max={3}
            step={0.05}
            format={(v) => `${v.toFixed(2)}×`}
            onChange={(x) => set({ sensitivity: { ...cal.sensitivity, x } })}
          />
          <Slider
            label={t('calibration.adjust.sensY')}
            value={cal.sensitivity.y}
            min={0.1}
            max={3}
            step={0.05}
            format={(v) => `${v.toFixed(2)}×`}
            onChange={(y) => set({ sensitivity: { ...cal.sensitivity, y } })}
          />
        </div>
        <Switch
          label={t('calibration.adjust.invertX')}
          checked={cal.invertX}
          onChange={(invertX) => set({ invertX })}
        />
        <Switch
          label={t('calibration.adjust.invertY')}
          checked={cal.invertY}
          onChange={(invertY) => set({ invertY })}
        />
        <Switch
          label={t('calibration.adjust.swap')}
          checked={cal.swapAxes}
          onChange={(swapAxes) => set({ swapAxes })}
        />
        <Slider
          label={t('calibration.adjust.antiDz')}
          hint={t('calibration.adjust.antiDzHint')}
          value={cal.antiDeadzone}
          min={0}
          max={0.4}
          step={0.005}
          format={pct}
          onChange={(antiDeadzone) => set({ antiDeadzone })}
        />
        <Field label={t('calibration.adjust.gate')}>
          <Segmented<GateShape>
            label={t('calibration.adjust.gate')}
            value={cal.gate}
            onChange={(gate) => set({ gate })}
            options={(['none', 'circle', 'octagon', 'square'] as GateShape[]).map((g) => ({
              value: g,
              label: t(`calibration.adjust.gates.${g}`),
            }))}
          />
        </Field>
        <dl className={css.readout}>
          <dt>{t('calibration.adjust.center')}</dt>
          <dd className="mono">
            ({cal.center.x.toFixed(3)}, {cal.center.y.toFixed(3)})
          </dd>
          <dt>{t('calibration.adjust.ranges')}</dt>
          <dd className="mono">
            +({cal.rangePos.x.toFixed(2)}, {cal.rangePos.y.toFixed(2)}) −(
            {cal.rangeNeg.x.toFixed(2)}, {cal.rangeNeg.y.toFixed(2)})
          </dd>
        </dl>
        <button
          type="button"
          className={`${ui.btn} ${ui.small}`}
          onClick={() => edit((p) => ({ ...p, [key]: defaultStick() }))}
        >
          {t('calibration.adjust.resetStick')}
        </button>
        <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
          {t('calibration.adjust.liveNote')}
        </p>
      </section>
    </div>
  );
}

function TriggerLive({
  id,
  side,
  cal,
}: {
  id: string;
  side: 'left' | 'right';
  cal: TriggerCalibration;
}) {
  const raw = useRef<HTMLElement>(null);
  const out = useRef<HTMLElement>(null);
  useAnimationFrame(() => {
    const e = live.get(id);
    if (!e) return;
    const v = side === 'left' ? e.snapshot.leftTrigger : e.snapshot.rightTrigger;
    if (raw.current) raw.current.style.transform = `scaleX(${v})`;
    if (out.current) out.current.style.transform = `scaleX(${applyTrigger(cal, v)})`;
  });
  return (
    <div className={css.trigLive} aria-hidden="true">
      <div>
        <i ref={raw} style={{ background: 'var(--sig-raw)' }} />
      </div>
      <div>
        <i ref={out} style={{ background: 'var(--sig-cal)' }} />
      </div>
    </div>
  );
}

function TriggerEditor({ c }: { c: ControllerInfo }) {
  const t = useT();
  const profile = useActiveProfile();
  const edit = useProfiles((s) => s.edit);
  if (!profile) return null;
  return (
    <div className={css.triggers}>
      {(['left', 'right'] as const).map((side) => {
        const key = side === 'left' ? 'leftTrigger' : 'rightTrigger';
        const cal = profile.calibration[key];
        const set = (patch: Partial<TriggerCalibration>) =>
          edit((p) => ({ ...p, [key]: { ...p[key], ...patch } }));
        return (
          <section key={side} className={`${ui.panel} ${css.controls}`}>
            <div className={ui.panelHead}>
              <h3 className={ui.panelTitle}>{t(`calibration.adjust.trig.${side}`)}</h3>
            </div>
            <TriggerLive id={c.id} side={side} cal={cal} />
            <Slider
              label={t('calibration.adjust.trig.deadzone')}
              value={cal.deadzone}
              min={0}
              max={0.5}
              step={0.005}
              format={pct}
              onChange={(deadzone) =>
                set({ deadzone, maxLimit: Math.max(cal.maxLimit, deadzone + 0.15) })
              }
            />
            <Slider
              label={t('calibration.adjust.trig.max')}
              value={cal.maxLimit}
              min={0.5}
              max={1}
              step={0.005}
              format={pct}
              onChange={(maxLimit) => set({ maxLimit })}
            />
            <Switch
              label={t('calibration.adjust.trig.digital')}
              checked={cal.digital}
              onChange={(digital) => set({ digital })}
            />
            <Slider
              label={t('calibration.adjust.trig.activation')}
              value={cal.activationPoint}
              min={0.05}
              max={0.95}
              step={0.01}
              disabled={!cal.digital}
              format={pct}
              onChange={(activationPoint) => set({ activationPoint })}
            />
          </section>
        );
      })}
    </div>
  );
}

export function Adjustments({ c }: { c: ControllerInfo }) {
  const t = useT();
  const [part, setPart] = useState<Part>('left');
  return (
    <div className={css.wrap}>
      <Segmented<Part>
        label={t('calibration.adjust.title')}
        value={part}
        onChange={setPart}
        options={[
          {
            value: 'left',
            label: `${t('calibration.adjust.stick')} ${t('calibration.adjust.left').toLowerCase()}`,
          },
          {
            value: 'right',
            label: `${t('calibration.adjust.stick')} ${t('calibration.adjust.right').toLowerCase()}`,
          },
          { value: 'triggers', label: t('calibration.adjust.trigger') },
        ]}
      />
      {part === 'triggers' ? <TriggerEditor c={c} /> : <StickEditor key={part} c={c} side={part} />}
    </div>
  );
}
