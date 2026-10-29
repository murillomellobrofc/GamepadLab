import { useEffect, useState } from 'react';
import { api, events } from '@/api/client';
import type { ControllerInfo } from '@/api/types';
import { getTransport, isTauri } from '@/api/transport';
import { useDocumentSettings } from '@/app/useDocumentSettings';
import { useT } from '@/i18n';
import { live } from '@/state/live';
import { useSettings, type OverlaySettings } from '@/state/settings';
import css from './Overlay.module.css';
import { OverlayContent } from './OverlayContent';

/** Root of overlay.html: the frameless always-on-top stream window. */
export function OverlayApp() {
  const t = useT();
  useDocumentSettings();
  const stored = useSettings((s) => s.overlay);
  const [settings, setSettings] = useState<OverlaySettings>(stored);
  const [controllers, setControllers] = useState<ControllerInfo[]>([]);
  const [targetId, setTargetId] = useState<string | null>(null);

  useEffect(() => {
    document.documentElement.style.background = 'transparent';
    document.body.style.background = 'transparent';
    const unsubs: (() => void)[] = [];
    void (async () => {
      const tr = await getTransport();
      unsubs.push(await events.frame((f) => live.push(f)));
      unsubs.push(
        await events.connection(({ controller, connected }) =>
          setControllers((cur) =>
            connected
              ? [...cur.filter((c) => c.id !== controller.id), controller]
              : cur.filter((c) => c.id !== controller.id),
          ),
        ),
      );
      unsubs.push(
        await events.overlaySettings((cfg) => {
          setSettings(cfg.settings);
          setTargetId(cfg.controllerId);
        }),
      );
      if (tr.kind === 'mock') await api.setDemo(true);
      setControllers(await api.listControllers());
      if (isTauri()) {
        const { invoke } = await import('@tauri-apps/api/core');
        const cfg = await invoke<{ settings: OverlaySettings; controllerId: string | null } | null>(
          'overlay_state',
        );
        if (cfg) {
          setSettings(cfg.settings);
          setTargetId(cfg.controllerId);
        }
      }
    })();
    return () => unsubs.forEach((u) => u());
  }, []);

  const c = controllers.find((x) => x.id === targetId) ?? controllers[0] ?? null;
  const close = async () => {
    if (isTauri()) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } else window.close();
  };

  return (
    <div className={css.window}>
      <div className={css.handle} data-tauri-drag-region>
        <span data-tauri-drag-region>{t('overlay.drag')}</span>
        <button type="button" onClick={close} aria-label={t('overlay.close')}>
          ×
        </button>
      </div>
      {c ? (
        <OverlayContent c={c} s={settings} />
      ) : (
        <div className={css.content}>
          <p className={css.empty}>{t('overlay.noController')}</p>
        </div>
      )}
    </div>
  );
}
