import type { ControllerInfo } from '@/api/types';
import { useT, type TKey } from '@/i18n';
import { hex4 } from '@/lib/format';
import ui from '@/styles/ui.module.css';
import css from './ControllerHeader.module.css';
import { Icon } from '@/app/icons';

const CAPS: [keyof ControllerInfo['capabilities'], TKey][] = [
  ['rumble', 'test.caps.rumble'],
  ['adaptiveTriggers', 'test.caps.adaptiveTriggers'],
  ['gyro', 'test.caps.gyro'],
  ['touchpad', 'test.caps.touchpad'],
  ['lightbar', 'test.caps.lightbar'],
  ['battery', 'test.caps.battery'],
];

export function ControllerHeader({ c }: { c: ControllerInfo }) {
  const t = useT();
  const bat = c.battery;
  return (
    <header className={css.head}>
      <div className={css.icon}>
        <Icon name="gamepad" size={26} />
      </div>
      <div className={css.main}>
        <h1 className={css.name}>{c.name}</h1>
        <div className={css.meta}>
          <span className="mono">
            VID:PID {hex4(c.vendorId)}:{hex4(c.productId)}
          </span>
          <span>{t(`app.kind.${c.kind}`)}</span>
          <span>
            <Icon name={c.connection === 'bluetooth' ? 'bluetooth' : 'usb'} size={13} />{' '}
            {t(`app.connection.${c.connection}`)}
          </span>
          {bat?.level != null && (
            <span>
              <Icon name="battery" size={13} /> {bat.level}%
              {bat.state === 'charging' ? ` · ${t('app.sidebar.charging')}` : ''}
            </span>
          )}
          <span className={ui.faint}>via {c.backend}</span>
        </div>
      </div>
      <ul className={css.caps} aria-label={t('test.caps.title')}>
        {CAPS.map(([k, label]) => (
          <li
            key={k}
            className={`${ui.chip} ${c.capabilities[k] ? ui.chipOk : ''}`}
            data-off={!c.capabilities[k] || undefined}
          >
            {t(label)}
          </li>
        ))}
        {c.isDemo && <li className={`${ui.chip} ${ui.chipInfo}`}>{t('app.sidebar.demo')}</li>}
        {c.isVirtual && <li className={`${ui.chip} ${ui.chipWarn}`}>{t('app.sidebar.virtual')}</li>}
      </ul>
    </header>
  );
}
