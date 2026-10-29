import { useEffect, useState } from 'react';
import type { ControllerInfo } from '@/api/types';
import { Segmented } from '@/app/controls';
import { useT } from '@/i18n';
import { useActiveProfile, useProfiles } from '@/state/profiles';
import ui from '@/styles/ui.module.css';
import { ControllerHeader } from '../common/ControllerHeader';
import { Adjustments } from './Adjustments';
import css from './CalibrationView.module.css';
import { FirmwarePanel } from './FirmwarePanel';
import { ProfilesPanel } from './ProfilesPanel';
import { Wizard } from './Wizard';

type Sub = 'wizard' | 'adjust' | 'profiles' | 'firmware';

function ProfileSwitcher() {
  const t = useT();
  const doc = useProfiles((s) => s.doc);
  const save = useProfiles((s) => s.save);
  const active = useActiveProfile();
  const reload = useProfiles((s) => s.reload);
  if (!doc) return null;
  return (
    <div className={css.switcher}>
      <label className={ui.fieldLabel} htmlFor="active-profile">
        {t('calibration.activeProfile')}
      </label>
      <select
        id="active-profile"
        className={ui.select}
        value={active?.id ?? ''}
        onChange={async (e) => {
          const { api } = await import('@/api/client');
          await useProfiles.getState().flush();
          await api.profilesSetActive(doc.deviceKey, e.target.value);
          await reload();
        }}
      >
        {doc.profiles.map((p) => (
          <option key={p.id} value={p.id}>
            {p.name}
          </option>
        ))}
      </select>
      <span className={css.save} aria-live="polite">
        {save === 'saving'
          ? t('calibration.adjust.saving')
          : save === 'saved'
            ? `✓ ${t('calibration.adjust.saved')}`
            : ''}
      </span>
    </div>
  );
}

export function CalibrationView({ c }: { c: ControllerInfo }) {
  const t = useT();
  const [sub, setSub] = useState<Sub>('wizard');
  const load = useProfiles((s) => s.load);
  const flush = useProfiles((s) => s.flush);
  useEffect(() => {
    void load(c);
    return () => void flush();
  }, [c, load, flush]);
  return (
    <>
      <ControllerHeader c={c} />
      <div className={css.bar}>
        <Segmented<Sub>
          label={t('calibration.tabs.label')}
          value={sub}
          onChange={setSub}
          options={[
            { value: 'wizard', label: t('calibration.tabs.wizard') },
            { value: 'adjust', label: t('calibration.tabs.adjust') },
            { value: 'profiles', label: t('calibration.tabs.profiles') },
            { value: 'firmware', label: t('calibration.firmware.tab') },
          ]}
        />
        <ProfileSwitcher />
      </div>
      {sub === 'wizard' && <Wizard c={c} />}
      {sub === 'adjust' && <Adjustments c={c} />}
      {sub === 'profiles' && <ProfilesPanel c={c} />}
      {sub === 'firmware' && <FirmwarePanel c={c} />}
    </>
  );
}
