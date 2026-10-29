import { useEffect, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import { isTauri } from '@/api/transport';
import { errorToast } from '@/app/bootstrap';
import { Field, Segmented, Slider, Switch } from '@/app/controls';
import { Icon } from '@/app/icons';
import { useT } from '@/i18n';
import { useApp } from '@/state/app';
import { useSettings, type ChromaKey } from '@/state/settings';
import ui from '@/styles/ui.module.css';
import css from './Overlay.module.css';
import { OverlayContent } from './OverlayContent';

export function OverlayView() {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const controllers = useApp((s) => s.controllers);
  const selectedId = useApp((s) => s.selectedId);
  const s = useSettings((st) => st.overlay);
  const setOverlay = useSettings((st) => st.setOverlay);
  const [targetId, setTargetId] = useState<string | null>(selectedId);
  const [open, setOpen] = useState(false);
  const target = controllers.find((c) => c.id === targetId) ?? controllers[0] ?? null;

  useEffect(() => {
    if (open) void api.overlayUpdate(s, target?.id ?? null).catch(() => {});
  }, [s, target?.id, open]);

  const openOverlay = async () => {
    try {
      if (isTauri()) await api.overlayOpen(s, target?.id ?? null);
      else window.open('/overlay.html', 'gamepadlab-overlay', 'width=480,height=340');
      setOpen(true);
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  };

  return (
    <section aria-labelledby="overlay-title">
      <h1 id="overlay-title" className={ui.panelTitle} style={{ marginBottom: 8 }}>
        {t('overlay.title')}
      </h1>
      <p className={ui.muted} style={{ marginBottom: 16, maxWidth: '70ch' }}>
        {t('overlay.intro')}
      </p>
      <div className={css.layout}>
        <section className={`${ui.panel} ${css.form}`}>
          <Field label={t('overlay.controller')}>
            <select
              className={ui.select}
              value={target?.id ?? ''}
              onChange={(e) => setTargetId(e.target.value)}
              aria-label={t('overlay.controller')}
            >
              {controllers.length === 0 && <option value="">{t('overlay.noController')}</option>}
              {controllers.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t('overlay.background')}>
            <Segmented<ChromaKey>
              label={t('overlay.background')}
              value={s.background}
              onChange={(background) => setOverlay({ background })}
              options={(['transparent', 'green', 'blue', 'magenta'] as ChromaKey[]).map((b) => ({
                value: b,
                label: t(`overlay.backgrounds.${b}`),
              }))}
            />
          </Field>
          <Slider
            label={t('overlay.scale')}
            value={s.scale}
            min={0.5}
            max={2}
            step={0.05}
            format={(v) => `${Math.round(v * 100)}%`}
            onChange={(scale) => setOverlay({ scale })}
          />
          <Switch
            label={t('overlay.showSticks')}
            checked={s.showSticks}
            onChange={(showSticks) => setOverlay({ showSticks })}
          />
          <Switch
            label={t('overlay.showTrail')}
            checked={s.showTrail}
            onChange={(showTrail) => setOverlay({ showTrail })}
            disabled={!s.showSticks}
          />
          <Switch
            label={t('overlay.showTriggers')}
            checked={s.showTriggers}
            onChange={(showTriggers) => setOverlay({ showTriggers })}
          />
          <Switch
            label={t('overlay.calibrated')}
            checked={s.calibrated}
            onChange={(calibrated) => setOverlay({ calibrated })}
          />
          <Switch
            label={t('overlay.alwaysOnTop')}
            checked={s.alwaysOnTop}
            onChange={(alwaysOnTop) => setOverlay({ alwaysOnTop })}
          />
          <div className={css.actions}>
            <button
              type="button"
              className={`${ui.btn} ${ui.primary}`}
              onClick={openOverlay}
              data-testid="overlay-open"
            >
              <Icon name="cast" size={16} /> {t('overlay.open')}
            </button>
            <button
              type="button"
              className={ui.btn}
              onClick={async () => {
                await api.overlayClose().catch(() => {});
                setOpen(false);
              }}
            >
              {t('overlay.close')}
            </button>
          </div>
          <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
            {t('overlay.obsTip')} {!isTauri() && t('overlay.browserNote')}
          </p>
        </section>
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t('overlay.preview')}</h2>
          </div>
          <div className={css.previewBox}>
            {target ? (
              <OverlayContent c={target} s={s} />
            ) : (
              <p className={ui.faint}>{t('overlay.noController')}</p>
            )}
          </div>
        </section>
      </div>
    </section>
  );
}
