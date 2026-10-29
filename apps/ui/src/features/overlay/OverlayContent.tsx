import type { ControllerInfo } from '@/api/types';
import type { OverlaySettings } from '@/state/settings';
import { ControllerDiagram } from '../test/ControllerDiagram';
import { StickScope } from '../test/StickScope';
import { TriggerBar } from '../test/TriggerBar';
import css from './Overlay.module.css';

const CHROMA: Record<OverlaySettings['background'], string> = {
  transparent: 'transparent',
  green: '#00b140',
  blue: '#0047bb',
  magenta: '#ff00ff',
};

/** What the stream sees: the pad, optionally sticks and triggers. */
export function OverlayContent({ c, s }: { c: ControllerInfo; s: OverlaySettings }) {
  return (
    <div
      className={css.content}
      style={{ background: CHROMA[s.background], ['--ov-scale' as string]: s.scale }}
    >
      <div className={css.scaled}>
        <div className={css.pad}>
          <ControllerDiagram controller={c} forceCalibrated={s.calibrated} />
        </div>
        {(s.showSticks || s.showTriggers) && (
          <div className={css.row}>
            {s.showTriggers && <TriggerBar id={c.id} side="left" label="L" />}
            {s.showSticks && <StickScope id={c.id} side="left" size={92} showTrail={s.showTrail} />}
            {s.showSticks && (
              <StickScope id={c.id} side="right" size={92} showTrail={s.showTrail} />
            )}
            {s.showTriggers && <TriggerBar id={c.id} side="right" label="R" />}
          </div>
        )}
      </div>
    </div>
  );
}
