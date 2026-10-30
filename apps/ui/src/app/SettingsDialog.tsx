import { useState } from 'react';
import { api, errorMessage } from '@/api/client';
import { LOCALE_NAMES, useT, type Locale } from '@/i18n';
import { copyText } from '@/lib/clipboard';
import { checkForUpdate, type UpdateStatus } from '@/lib/updater';
import { useApp } from '@/state/app';
import { useSettings, type LogLevel } from '@/state/settings';
import ui from '@/styles/ui.module.css';
import { Field, Section, Segmented, Slider, Switch } from './controls';
import { Dialog } from './Dialog';
import { Icon } from './icons';

export function SettingsDialog() {
  const t = useT();
  const open = useApp((s) => s.dialog === 'settings');
  const openDialog = useApp((s) => s.openDialog);
  const toast = useApp((s) => s.toast);
  const info = useApp((s) => s.appInfo);
  const s = useSettings();
  const [update, setUpdate] = useState<UpdateStatus | null>(null);
  const [checking, setChecking] = useState(false);
  // undefined = not installing, null = downloading with unknown size, 0…1 = progress
  const [progress, setProgress] = useState<number | null | undefined>(undefined);

  const copyDiagnostics = async () => {
    try {
      const text = await api.diagnostics();
      await copyText(text);
      toast({ tone: 'ok', title: t('app.toast.copied') });
    } catch (e) {
      toast({ tone: 'bad', title: t('app.toast.error'), body: errorMessage(e) });
    }
  };

  return (
    <Dialog open={open} title={t('settings.title')} onClose={() => openDialog(null)}>
      <Section title={t('settings.appearance')}>
        <Field label={t('settings.theme')}>
          <Segmented
            label={t('settings.theme')}
            value={s.theme}
            onChange={(theme) => s.set({ theme })}
            options={[
              { value: 'dark', label: t('settings.themeDark') },
              { value: 'light', label: t('settings.themeLight') },
              { value: 'auto', label: t('settings.themeAuto') },
            ]}
          />
        </Field>
        <Switch
          label={t('settings.highContrast')}
          checked={s.highContrast}
          onChange={(highContrast) => s.set({ highContrast })}
        />
        <Slider
          label={t('settings.textSize')}
          value={s.textScale}
          min={0.9}
          max={1.5}
          step={0.05}
          format={(v) => `${Math.round(v * 100)}%`}
          onChange={(textScale) => s.set({ textScale })}
        />
        <Field label={t('settings.motion')}>
          <Segmented
            label={t('settings.motion')}
            value={s.motion}
            onChange={(motion) => s.set({ motion })}
            options={[
              { value: 'system', label: t('settings.motionSystem') },
              { value: 'reduced', label: t('settings.motionReduced') },
              { value: 'full', label: t('settings.motionFull') },
            ]}
          />
        </Field>
        <Field label={t('settings.language')}>
          <select
            className={ui.select}
            value={s.locale}
            aria-label={t('settings.language')}
            onChange={(e) => s.set({ locale: e.target.value as Locale })}
          >
            {Object.entries(LOCALE_NAMES).map(([value, name]) => (
              <option key={value} value={value}>
                {name}
              </option>
            ))}
          </select>
        </Field>
      </Section>

      <Section title={t('settings.input')}>
        <Field label={t('settings.uiRate')}>
          <Segmented
            label={t('settings.uiRate')}
            value={s.uiRate}
            onChange={(uiRate) => {
              s.set({ uiRate });
              void api.setUiRate(uiRate).catch(() => {});
            }}
            options={[
              { value: 60, label: '60 Hz' },
              { value: 120, label: '120 Hz' },
            ]}
          />
          <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)' }}>
            {t('settings.uiRateHint')}
          </p>
        </Field>
        <Switch
          label={t('settings.gamepadNav')}
          hint={t('settings.gamepadNavHint')}
          checked={s.gamepadNav}
          onChange={(gamepadNav) => s.set({ gamepadNav })}
        />
      </Section>

      <Section title={t('settings.diagnostics')}>
        <Field label={t('settings.logLevel')}>
          <select
            className={ui.select}
            value={s.logLevel}
            aria-label={t('settings.logLevel')}
            onChange={(e) => {
              const logLevel = e.target.value as LogLevel;
              s.set({ logLevel });
              void api.setLogLevel(logLevel).catch(() => {});
            }}
          >
            {(['error', 'warn', 'info', 'debug', 'trace'] as const).map((l) => (
              <option key={l} value={l}>
                {l}
              </option>
            ))}
          </select>
        </Field>
        <div>
          <button type="button" className={ui.btn} onClick={copyDiagnostics}>
            <Icon name="copy" size={16} /> {t('settings.copyDiagnostics')}
          </button>
          <p className={ui.faint} style={{ fontSize: 'var(--fs-xs)', marginTop: 6 }}>
            {t('settings.copyDiagnosticsHint')}
          </p>
        </div>
      </Section>

      <Section title={t('settings.updates')}>
        <Switch
          label={t('settings.autoUpdate')}
          checked={s.autoUpdate}
          onChange={(autoUpdate) => s.set({ autoUpdate })}
        />
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
          <button
            type="button"
            className={ui.btn}
            disabled={checking}
            onClick={async () => {
              setChecking(true);
              setUpdate(await checkForUpdate(!!info?.updaterConfigured));
              setChecking(false);
            }}
          >
            <Icon name="refresh" size={16} /> {t('settings.checkNow')}
          </button>
          {update?.state === 'disabled' && (
            <span className={ui.faint}>
              {t(
                info?.portable
                  ? 'settings.updaterPortable'
                  : info?.packageManaged
                    ? 'settings.updaterPackaged'
                    : 'settings.updaterOff',
              )}
            </span>
          )}
          {update?.state === 'none' && <span className={ui.muted}>{t('settings.upToDate')}</span>}
          {update?.state === 'error' && (
            <span style={{ color: 'var(--bad)' }} title={update.message}>
              {update.reason === 'other'
                ? t('settings.updateFailed', { message: update.message })
                : t(`settings.update_${update.reason}`)}
            </span>
          )}
          {update?.state === 'available' && (
            <>
              <span>{t('settings.updateAvailable', { version: update.version })}</span>
              {progress === undefined ? (
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  onClick={async () => {
                    setProgress(null);
                    try {
                      await update.install(setProgress);
                    } catch (e) {
                      setProgress(undefined);
                      setUpdate({
                        state: 'error',
                        reason: 'other',
                        message: e instanceof Error ? e.message : String(e),
                      });
                    }
                  }}
                >
                  {t('settings.installUpdate')}
                </button>
              ) : (
                <span className={ui.muted} role="status">
                  {t('settings.downloading', {
                    pct: progress === null ? '' : `${Math.round(progress * 100)}%`,
                  })}
                </span>
              )}
            </>
          )}
        </div>
      </Section>

      <Section title={t('settings.keyboard')}>
        <p className={ui.muted} style={{ fontSize: 'var(--fs-sm)' }}>
          {t('settings.shortcuts')}
        </p>
      </Section>
    </Dialog>
  );
}
