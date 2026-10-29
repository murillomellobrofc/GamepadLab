import { api, errorMessage } from '@/api/client';
import type { ControllerInfo, FrameEntry } from '@/api/types';
import { useT } from '@/i18n';
import { MAX_CONTROLLERS, useApp } from '@/state/app';
import { useLive } from '@/state/live';
import { hex4 } from '@/lib/format';
import ui from '@/styles/ui.module.css';
import css from './AppShell.module.css';
import { Icon } from './icons';

function isActive(e: FrameEntry): boolean {
  const s = e.snapshot;
  if (s.buttons.some((b) => b.pressed)) return true;
  if (Math.hypot(s.leftStick.x, s.leftStick.y) > 0.25) return true;
  if (Math.hypot(s.rightStick.x, s.rightStick.y) > 0.25) return true;
  return s.leftTrigger > 0.1 || s.rightTrigger > 0.1;
}

function ActivityDot({ id }: { id: string }) {
  const active = useLive(id, isActive, 20) ?? false;
  return <i data-active={active} />;
}

function PadItem({ c, current }: { c: ControllerInfo; current: boolean }) {
  const t = useT();
  const select = useApp((s) => s.select);
  const battery = c.battery;
  return (
    <li>
      <button
        type="button"
        className={css.pad}
        aria-current={current}
        aria-label={t('app.sidebar.select', { name: c.name })}
        onClick={() => select(c.id)}
        data-testid="controller-item"
      >
        <span className={css.padIcon}>
          <Icon name="gamepad" size={20} />
          <ActivityDot id={c.id} />
        </span>
        <span className={css.padName} title={c.name}>
          {c.name}
        </span>
        <span className={css.padMeta}>
          <span className="mono">
            {hex4(c.vendorId)}:{hex4(c.productId)}
          </span>
          <span>{t(`app.connection.${c.connection}`)}</span>
          {c.isDemo && <span className={`${ui.chip} ${ui.chipInfo}`}>{t('app.sidebar.demo')}</span>}
          {c.isVirtual && (
            <span className={`${ui.chip} ${ui.chipWarn}`}>{t('app.sidebar.virtual')}</span>
          )}
        </span>
        <span className={css.padMeta}>
          <span>{t(`app.kind.${c.kind}`)}</span>
          {battery && battery.level !== null && (
            <span>
              <Icon name="battery" size={12} style={{ verticalAlign: '-2px' }} />{' '}
              {t('app.sidebar.battery', { level: battery.level })}
              {battery.state === 'charging' ? ` · ${t('app.sidebar.charging')}` : ''}
            </span>
          )}
          {battery && battery.state === 'wired' && <span>{t('app.sidebar.wired')}</span>}
        </span>
      </button>
    </li>
  );
}

export function Sidebar() {
  const t = useT();
  const controllers = useApp((s) => s.controllers);
  const selectedId = useApp((s) => s.selectedId);
  const info = useApp((s) => s.appInfo);
  const openDialog = useApp((s) => s.openDialog);
  const toast = useApp((s) => s.toast);
  const demoActive = controllers.some((c) => c.isDemo);

  const toggleDemo = async () => {
    try {
      await api.setDemo(!demoActive);
    } catch (e) {
      toast({ tone: 'bad', title: t('app.toast.error'), body: errorMessage(e) });
    }
  };

  return (
    <nav className={css.sidebar} aria-label={t('app.sidebar.controllers')}>
      <div className={css.brand}>
        <img src="/favicon.svg" alt="" />
        <span className={css.brandName}>
          Gamepad<span>Lab</span>
        </span>
        <span className={css.brandVer}>v{info?.version ?? '0.1.0'}</span>
      </div>
      <div className={css.sideSection}>
        <span className={css.sideTitle}>{t('app.sidebar.controllers')}</span>
        <span className={css.sideCount}>
          {t('app.sidebar.count', { count: controllers.length, max: MAX_CONTROLLERS })}
        </span>
      </div>
      <ul className={css.padList}>
        {controllers.length === 0 && <li className={css.padEmpty}>{t('app.sidebar.none')}</li>}
        {controllers.map((c) => (
          <PadItem key={c.id} c={c} current={c.id === selectedId} />
        ))}
      </ul>
      <div className={css.sideFoot}>
        <button
          type="button"
          className={`${ui.btn} ${demoActive ? '' : ui.primary}`}
          onClick={toggleDemo}
          data-testid="demo-toggle"
          title={demoActive ? t('app.sidebar.demoStop') : t('app.sidebar.demoStart')}
        >
          <Icon name={demoActive ? 'stop' : 'play'} size={16} />
          <span className={css.hideNarrow}>
            {demoActive ? t('app.sidebar.demoStop') : t('app.sidebar.demoStart')}
          </span>
        </button>
        <div className={css.sideFootRow}>
          <button
            type="button"
            className={`${ui.btn} ${ui.ghost} ${ui.small}`}
            onClick={() => openDialog('settings')}
            title={t('app.sidebar.settings')}
          >
            <Icon name="settings" size={16} />
            <span className={css.hideNarrow}>{t('app.sidebar.settings')}</span>
          </button>
          <button
            type="button"
            className={`${ui.btn} ${ui.ghost} ${ui.small}`}
            onClick={() => openDialog('about')}
            title={t('app.sidebar.about')}
          >
            <Icon name="info" size={16} />
            <span className={css.hideNarrow}>{t('app.sidebar.about')}</span>
          </button>
        </div>
        <div className={css.privacy} title={t('app.about.privacy')}>
          <Icon name="shield" size={13} />
          <span>{t('app.noTelemetry')}</span>
        </div>
      </div>
    </nav>
  );
}
