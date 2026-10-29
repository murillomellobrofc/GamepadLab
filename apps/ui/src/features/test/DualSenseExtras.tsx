import { useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type { ControllerInfo, TriggerEffect } from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { Field, Segmented, Slider } from '@/app/controls';
import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import ui from '@/styles/ui.module.css';
import css from './DualSenseExtras.module.css';

type Kind = TriggerEffect['type'];

const DEFAULTS: Record<Kind, TriggerEffect> = {
  off: { type: 'off' },
  feedback: { type: 'feedback', position: 2, strength: 6 },
  weapon: { type: 'weapon', start: 3, end: 6, strength: 8 },
  vibration: { type: 'vibration', position: 1, amplitude: 6, frequency: 30 },
};

function EffectEditor({
  label,
  effect,
  onChange,
}: {
  label: string;
  effect: TriggerEffect;
  onChange: (e: TriggerEffect) => void;
}) {
  const t = useT();
  return (
    <fieldset className={css.fieldset}>
      <legend>{label}</legend>
      <Segmented<Kind>
        label={label}
        value={effect.type}
        onChange={(k) => onChange(DEFAULTS[k])}
        options={(['off', 'feedback', 'weapon', 'vibration'] as Kind[]).map((k) => ({
          value: k,
          label: t(`test.dualsense.effects.${k}`),
        }))}
      />
      {effect.type === 'feedback' && (
        <>
          <Slider
            label={t('test.dualsense.position')}
            value={effect.position}
            min={0}
            max={9}
            step={1}
            format={String}
            onChange={(position) => onChange({ ...effect, position })}
          />
          <Slider
            label={t('test.dualsense.strength')}
            value={effect.strength}
            min={1}
            max={8}
            step={1}
            format={String}
            onChange={(strength) => onChange({ ...effect, strength })}
          />
        </>
      )}
      {effect.type === 'weapon' && (
        <>
          <Slider
            label={t('test.dualsense.start')}
            value={effect.start}
            min={2}
            max={7}
            step={1}
            format={String}
            onChange={(start) =>
              onChange({ ...effect, start, end: Math.max(effect.end, start + 1) })
            }
          />
          <Slider
            label={t('test.dualsense.end')}
            value={effect.end}
            min={effect.start + 1}
            max={8}
            step={1}
            format={String}
            onChange={(end) => onChange({ ...effect, end })}
          />
          <Slider
            label={t('test.dualsense.strength')}
            value={effect.strength}
            min={1}
            max={8}
            step={1}
            format={String}
            onChange={(strength) => onChange({ ...effect, strength })}
          />
        </>
      )}
      {effect.type === 'vibration' && (
        <>
          <Slider
            label={t('test.dualsense.position')}
            value={effect.position}
            min={0}
            max={9}
            step={1}
            format={String}
            onChange={(position) => onChange({ ...effect, position })}
          />
          <Slider
            label={t('test.dualsense.amplitude')}
            value={effect.amplitude}
            min={1}
            max={8}
            step={1}
            format={String}
            onChange={(amplitude) => onChange({ ...effect, amplitude })}
          />
          <Slider
            label={t('test.dualsense.frequency')}
            value={effect.frequency}
            min={1}
            max={255}
            step={1}
            format={(v) => `${v} Hz`}
            onChange={(frequency) => onChange({ ...effect, frequency })}
          />
        </>
      )}
    </fieldset>
  );
}

export function DualSenseExtras({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const [left, setLeft] = useState<TriggerEffect>(DEFAULTS.feedback);
  const [right, setRight] = useState<TriggerEffect>(DEFAULTS.weapon);
  const [color, setColor] = useState('#4bf0b5');
  const [leds, setLeds] = useState(0b00100);
  const run = async (fn: () => Promise<void>) => {
    try {
      await fn();
      toast({ tone: 'ok', title: t('test.vibration.sent') });
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  };
  const rgb = [1, 3, 5].map((i) => parseInt(color.slice(i, i + 2), 16));

  return (
    <section className={ui.panel} aria-labelledby="ds-title">
      <div className={ui.panelHead}>
        <h2 id="ds-title" className={ui.panelTitle}>
          {t('test.dualsense.title')}
        </h2>
      </div>
      <div className={css.body}>
        {c.capabilities.adaptiveTriggers && (
          <>
            <EffectEditor label={t('test.dualsense.left')} effect={left} onChange={setLeft} />
            <EffectEditor label={t('test.dualsense.right')} effect={right} onChange={setRight} />
            <div className={css.row}>
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                onClick={() => run(() => api.setTriggerEffects(c.id, left, right))}
              >
                {t('test.dualsense.apply')}
              </button>
              <button
                type="button"
                className={ui.btn}
                onClick={() =>
                  run(() => api.setTriggerEffects(c.id, { type: 'off' }, { type: 'off' }))
                }
              >
                {t('test.dualsense.reset')}
              </button>
            </div>
            <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
              {t('test.dualsense.hint')}
            </p>
          </>
        )}
        {c.capabilities.lightbar && (
          <div className={css.lights}>
            <Field label={t('test.dualsense.lightbar')}>
              <input
                type="color"
                value={color}
                onChange={(e) => setColor(e.target.value)}
                className={css.color}
                aria-label={t('test.dualsense.lightbar')}
              />
            </Field>
            {c.capabilities.playerLeds && (
              <Field label={t('test.dualsense.playerLeds')}>
                <div className={css.leds}>
                  {[0, 1, 2, 3, 4].map((i) => (
                    <button
                      key={i}
                      type="button"
                      aria-pressed={!!(leds & (1 << i))}
                      aria-label={`LED ${i + 1}`}
                      onClick={() => setLeds((m) => m ^ (1 << i))}
                    />
                  ))}
                </div>
              </Field>
            )}
            <button
              type="button"
              className={ui.btn}
              onClick={() =>
                run(async () => {
                  await api.setLightbar(c.id, rgb[0], rgb[1], rgb[2]);
                  if (c.capabilities.playerLeds) await api.setPlayerLeds(c.id, leds);
                })
              }
            >
              {t('test.dualsense.applyLight')}
            </button>
          </div>
        )}
      </div>
    </section>
  );
}
