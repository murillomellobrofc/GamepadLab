import { useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type { ControllerInfo, Profile } from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { ConfirmDialog } from '@/app/ConfirmDialog';
import { Icon } from '@/app/icons';
import { formatDateTime, useT } from '@/i18n';
import { openText, saveText, stamp } from '@/lib/files';
import { useApp } from '@/state/app';
import { useProfiles } from '@/state/profiles';
import ui from '@/styles/ui.module.css';
import css from './ProfilesPanel.module.css';

const JSON_FILTER = [{ name: 'GamepadLab profile', extensions: ['json'] }];

export function ProfilesPanel({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const doc = useProfiles((s) => s.doc);
  const reload = useProfiles((s) => s.reload);
  const flush = useProfiles((s) => s.flush);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [confirm, setConfirm] = useState<{ kind: 'delete' | 'reset'; p: Profile } | null>(null);
  if (!doc) return <p className={ui.muted}>{t('app.common.loading')}</p>;
  const key = doc.deviceKey;

  const run = async (fn: () => Promise<unknown>, ok?: string) => {
    try {
      await flush();
      await fn();
      await reload();
      if (ok) toast({ tone: 'ok', title: ok });
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  };

  const fmt = (iso: string) => formatDateTime(iso);

  return (
    <section className={ui.panel} aria-labelledby="profiles-title">
      <div className={ui.panelHead}>
        <h2 id="profiles-title" className={ui.panelTitle}>
          {t('calibration.profiles.title', { device: c.name })}
        </h2>
        <span className={ui.spacer} />
        <button
          type="button"
          className={`${ui.btn} ${ui.small} ${ui.primary}`}
          onClick={() =>
            run(() => api.profilesCreate(key, t('calibration.profiles.newName'), null))
          }
          data-testid="profile-new"
        >
          <Icon name="plus" size={14} /> {t('calibration.profiles.new')}
        </button>
        <button
          type="button"
          className={`${ui.btn} ${ui.small}`}
          onClick={() =>
            run(async () => {
              const json = await openText(JSON_FILTER);
              if (json == null) return;
              const p = await api.profilesImport(key, json);
              toast({ tone: 'ok', title: t('calibration.profiles.imported', { name: p.name }) });
            })
          }
        >
          <Icon name="upload" size={14} /> {t('calibration.profiles.import')}
        </button>
      </div>
      <p className={css.hint}>
        {t('calibration.profiles.hint')} <span className="mono">{key}</span>
      </p>
      <ul className={css.list} role="list">
        {doc.profiles.map((p) => {
          const active = p.id === doc.activeProfileId;
          return (
            <li
              key={p.id}
              className={css.item}
              data-active={active || undefined}
              data-testid="profile-item"
            >
              <div className={css.main}>
                {renaming === p.id ? (
                  <form
                    className={css.rename}
                    onSubmit={(e) => {
                      e.preventDefault();
                      setRenaming(null);
                      void run(() => api.profilesRename(key, p.id, name));
                    }}
                  >
                    <input
                      className={ui.input}
                      value={name}
                      autoFocus
                      maxLength={60}
                      aria-label={t('calibration.profiles.renamePrompt')}
                      onChange={(e) => setName(e.target.value)}
                      onKeyDown={(e) => e.key === 'Escape' && setRenaming(null)}
                    />
                    <button type="submit" className={`${ui.btn} ${ui.small}`}>
                      {t('calibration.profiles.save')}
                    </button>
                  </form>
                ) : (
                  <p className={css.name}>
                    {p.name}{' '}
                    {active && (
                      <span className={`${ui.chip} ${ui.chipOk}`}>
                        {t('calibration.profiles.active')}
                      </span>
                    )}
                  </p>
                )}
                <p className={css.meta}>
                  {t('calibration.profiles.updated', { date: fmt(p.updatedAt) })}
                  {p.calibration.measuredAt &&
                    ` · ${t('calibration.profiles.measured', { date: fmt(p.calibration.measuredAt) })}`}
                </p>
              </div>
              <div className={css.actions}>
                {!active && (
                  <button
                    type="button"
                    className={`${ui.btn} ${ui.small} ${ui.primary}`}
                    onClick={() => run(() => api.profilesSetActive(key, p.id))}
                  >
                    {t('calibration.profiles.activate')}
                  </button>
                )}
                <button
                  type="button"
                  className={`${ui.btn} ${ui.small}`}
                  onClick={() => {
                    setRenaming(p.id);
                    setName(p.name);
                  }}
                >
                  {t('calibration.profiles.rename')}
                </button>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.small}`}
                  onClick={() => run(() => api.profilesDuplicate(key, p.id))}
                >
                  <Icon name="copy" size={14} /> {t('calibration.profiles.duplicate')}
                </button>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.small}`}
                  onClick={() =>
                    run(async () => {
                      const json = await api.profilesExport(key, p.id);
                      const safe = p.name.replace(/[^\w-]+/g, '_');
                      if (await saveText(`gamepadlab-${safe}-${stamp()}.json`, json, JSON_FILTER))
                        toast({ tone: 'ok', title: t('calibration.profiles.exported') });
                    })
                  }
                >
                  <Icon name="download" size={14} /> {t('calibration.profiles.export')}
                </button>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.small}`}
                  onClick={() => setConfirm({ kind: 'reset', p })}
                >
                  <Icon name="refresh" size={14} /> {t('calibration.profiles.reset')}
                </button>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.small} ${ui.danger}`}
                  disabled={doc.profiles.length <= 1}
                  onClick={() => setConfirm({ kind: 'delete', p })}
                >
                  <Icon name="trash" size={14} /> {t('calibration.profiles.delete')}
                </button>
              </div>
            </li>
          );
        })}
      </ul>
      <ConfirmDialog
        open={!!confirm}
        danger
        title={
          confirm?.kind === 'delete'
            ? t('calibration.profiles.delete')
            : t('calibration.profiles.reset')
        }
        body={
          confirm
            ? t(
                confirm.kind === 'delete'
                  ? 'calibration.profiles.confirmDelete'
                  : 'calibration.profiles.confirmReset',
                {
                  name: confirm.p.name,
                },
              )
            : ''
        }
        onCancel={() => setConfirm(null)}
        onConfirm={() => {
          const cnf = confirm;
          setConfirm(null);
          if (!cnf) return;
          void run(() =>
            cnf.kind === 'delete'
              ? api.profilesDelete(key, cnf.p.id)
              : api.profilesReset(key, cnf.p.id),
          );
        }}
      />
    </section>
  );
}
