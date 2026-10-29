import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorCode, errorMessage } from '@/api/client';
import type {
  ControllerInfo,
  FirmwareOp,
  FirmwareStatus,
  SwitchCalibrationData,
  SwitchProposals,
  SwitchStickCal,
} from '@/api/types';
import { errorToast } from '@/app/bootstrap';
import { ConfirmDialog } from '@/app/ConfirmDialog';
import { Segmented } from '@/app/controls';
import { Icon } from '@/app/icons';
import { useT, type TKey } from '@/i18n';
import { useApp } from '@/state/app';
import { live } from '@/state/live';
import ui from '@/styles/ui.module.css';
import { StickScope } from '../test/StickScope';
import css from './FirmwarePanel.module.css';

const SECTORS = 32;
const RANGE_UNLOCK_S = 15;

function firmwareSupport(c: ControllerInfo): 'sony' | 'switch' | 'needsHid' | 'none' {
  if (c.isDemo) return 'sony';
  if (c.kind === 'switchPro') return 'switch';
  if (c.kind === 'ps4' || c.kind === 'ps5') return c.backend === 'hid' ? 'sony' : 'needsHid';
  return 'none';
}

function useFirmware(c: ControllerInfo) {
  const toast = useApp((s) => s.toast);
  const [status, setStatus] = useState<FirmwareStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const run = useCallback(
    async (op: FirmwareOp) => {
      setBusy(true);
      try {
        const r = await api.firmware(c.id, op);
        setStatus(r.status);
        return r;
      } catch (e) {
        toast(errorToast(errorCode(e), errorMessage(e)));
        try {
          setStatus((await api.firmware(c.id, { op: 'status' })).status);
        } catch {
          /* controller gone */
        }
        return null;
      } finally {
        setBusy(false);
      }
    },
    [c.id, toast],
  );
  useEffect(() => {
    void run({ op: 'status' });
  }, [run]);
  return { status, busy, run };
}

function StatusCard({ s, onRefresh }: { s: FirmwareStatus; onRefresh: () => void }) {
  const t = useT();
  const k = 'calibration.firmware.status';
  return (
    <section className={ui.panel} aria-labelledby="fw-status">
      <div className={ui.panelHead}>
        <h2 id="fw-status" className={ui.panelTitle}>
          {t(`${k}.title` as TKey)}
        </h2>
        <span className={ui.spacer} />
        <button type="button" className={`${ui.btn} ${ui.small}`} onClick={onRefresh}>
          <Icon name="refresh" size={14} /> {t(`${k}.refresh` as TKey)}
        </button>
      </div>
      <dl className={css.status} data-testid="fw-status">
        <dt>{t(`${k}.model` as TKey)}</dt>
        <dd>
          {
            {
              ds4: 'DualShock 4',
              ds5: 'DualSense',
              ds5Edge: 'DualSense Edge',
              switchPro: 'Switch Pro',
            }[s.model]
          }
        </dd>
        <dt>{t(`${k}.genuine` as TKey)}</dt>
        <dd>
          {s.genuine == null
            ? t(`${k}.unknown` as TKey)
            : s.genuine
              ? t(`${k}.yes` as TKey)
              : t(`${k}.no` as TKey)}
        </dd>
        {s.firmware && (
          <>
            <dt>{t(`${k}.firmware` as TKey)}</dt>
            <dd className="mono">{s.firmware}</dd>
          </>
        )}
        <dt>{t(`${k}.connection` as TKey)}</dt>
        <dd>{s.usb ? 'USB' : 'Bluetooth'}</dd>
        <dt>{t(`${k}.battery` as TKey)}</dt>
        <dd className="mono">{s.battery != null ? `${s.battery}%` : '—'}</dd>
        {s.nv !== 'notApplicable' && (
          <>
            <dt>{t(`${k}.memory` as TKey)}</dt>
            <dd>{t(`${k}.nv.${s.nv}` as TKey)}</dd>
          </>
        )}
        <dt>{t(`${k}.changes` as TKey)}</dt>
        <dd data-unsaved={s.unsavedChanges || undefined}>
          {s.unsavedChanges ? t(`${k}.unsaved` as TKey) : t(`${k}.clean` as TKey)}
        </dd>
      </dl>
      {s.blockers.length > 0 && (
        <ul className={css.blockers} role="alert">
          {s.blockers.map((b) => (
            <li key={b}>
              <Icon name="alert" size={14} /> {t(`calibration.firmware.blockers.${b}` as TKey)}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function Risks({ switchPad }: { switchPad?: boolean }) {
  const t = useT();
  const k = 'calibration.firmware.risks';
  return (
    <section className={`${ui.callout} ${ui.calloutWarn} ${css.risks}`} aria-labelledby="fw-risks">
      <Icon name="shield" />
      <div>
        <h3 id="fw-risks">{t(`${k}.title` as TKey)}</h3>
        <ul>
          <li>{t(`${k}.wear` as TKey)}</li>
          <li>{t(`${k}.battery` as TKey)}</li>
          {!switchPad && <li>{t(`${k}.noUndo` as TKey)}</li>}
          {!switchPad && <li>{t(`${k}.tempFirst` as TKey)}</li>}
        </ul>
        {!switchPad && <p className={ui.faint}>{t(`${k}.credits` as TKey)}</p>}
      </div>
    </section>
  );
}

/** Coverage (0..1) of both sticks while recording, from every frame. */
function useBothCoverage(id: string, active: boolean) {
  const [cov, setCov] = useState({ left: 0, right: 0 });
  useEffect(() => {
    if (!active) return;
    const seen = { left: new Set<number>(), right: new Set<number>() };
    setCov({ left: 0, right: 0 });
    return live.subscribe(() => {
      const e = live.get(id);
      if (!e) return;
      let changed = false;
      for (const side of ['left', 'right'] as const) {
        const p = side === 'left' ? e.snapshot.leftStick : e.snapshot.rightStick;
        if (Math.hypot(p.x, p.y) < 0.8) continue;
        let a = Math.atan2(p.y, p.x);
        if (a < 0) a += Math.PI * 2;
        const idx = Math.min(SECTORS - 1, Math.floor((a / (Math.PI * 2)) * SECTORS));
        if (!seen[side].has(idx)) {
          seen[side].add(idx);
          changed = true;
        }
      }
      if (changed) setCov({ left: seen.left.size / SECTORS, right: seen.right.size / SECTORS });
    });
  }, [id, active]);
  return cov;
}

function SonyFirmware({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const announce = useApp((s) => s.announce);
  const lockNav = useApp((s) => s.lockNav);
  const { status, busy, run } = useFirmware(c);
  const [mode, setMode] = useState<'fourStep' | 'quick'>('fourStep');
  const [corner, setCorner] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const [ack, setAck] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const phaseRef = useRef(status?.phase);
  phaseRef.current = status?.phase;
  const k = 'calibration.firmware';
  const demo = (s: Parameters<typeof api.demoScenario>[0]) =>
    c.isDemo ? api.demoScenario(s).catch(() => {}) : Promise.resolve();
  const cov = useBothCoverage(c.id, status?.phase === 'range');

  // Never leave the controller in range mode when this view goes away.
  useEffect(
    () => () => {
      if (phaseRef.current === 'range') void api.firmware(c.id, { op: 'rangeEnd' }).catch(() => {});
    },
    [c.id],
  );

  useEffect(() => {
    if (status?.phase === 'center' || status?.phase === 'range') return lockNav();
  }, [status?.phase, lockNav]);

  useEffect(() => {
    if (status?.phase !== 'range') return;
    setElapsed(0);
    const timer = window.setInterval(() => setElapsed((s) => s + 1), 1000);
    return () => window.clearInterval(timer);
  }, [status?.phase]);

  if (!status) return <p className={ui.muted}>{t('app.common.loading')}</p>;
  const blocked = status.blockers.length > 0;
  const idle = status.phase === 'idle';

  const startCenter = async () => {
    await demo('idle');
    setCorner(0);
    const r = await run({ op: 'centerBegin' });
    if (!r) return;
    if (mode === 'quick') {
      for (let i = 0; i < 5; i++) {
        await new Promise((res) => setTimeout(res, 120));
        if (!(await run({ op: 'centerSample' }))) return;
      }
      if (await run({ op: 'centerEnd' })) announce(t(`${k}.center.done` as TKey));
    } else {
      announce(t(`${k}.center.corner.0` as TKey));
    }
  };

  const nextCorner = async () => {
    if (!(await run({ op: 'centerSample' }))) return;
    if (corner < 3) {
      setCorner(corner + 1);
      announce(t(`${k}.center.corner.${corner + 1}` as TKey));
    } else if (await run({ op: 'centerEnd' })) {
      announce(t(`${k}.center.done` as TKey));
    }
  };

  const startRange = async () => {
    await demo('rotateBoth');
    await run({ op: 'rangeBegin' });
  };

  const finishRange = async () => {
    const r = await run({ op: 'rangeEnd' });
    await demo('showcase');
    if (r) announce(t(`${k}.range.done` as TKey));
  };

  const canFinishRange = elapsed >= RANGE_UNLOCK_S || (cov.left >= 0.95 && cov.right >= 0.95);
  const pct = (v: number) => `${Math.round(v * 100)}%`;

  return (
    <div className={css.layout}>
      <div className={css.column}>
        <StatusCard s={status} onRefresh={() => void run({ op: 'status' })} />
        <Risks />
      </div>
      <div className={css.column}>
        <section className={ui.panel} data-testid="fw-center">
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t(`${k}.center.title` as TKey)}</h2>
          </div>
          {status.phase === 'center' ? (
            <div className={css.step} aria-live="polite">
              <p className={css.warn}>{t(`${k}.center.running` as TKey)}</p>
              {mode === 'fourStep' && (
                <>
                  <p className={css.big}>
                    {corner + 1}/4 · {t(`${k}.center.corner.${corner}` as TKey)}
                  </p>
                  <p className={ui.muted}>{t(`${k}.center.whenCentered` as TKey)}</p>
                  <button
                    type="button"
                    className={`${ui.btn} ${ui.primary}`}
                    disabled={busy}
                    onClick={nextCorner}
                    data-testid="fw-center-next"
                  >
                    {t(`${k}.center.continue` as TKey)}
                  </button>
                </>
              )}
            </div>
          ) : (
            <div className={css.step}>
              <Segmented<'fourStep' | 'quick'>
                label={t(`${k}.center.title` as TKey)}
                value={mode}
                onChange={setMode}
                options={[
                  { value: 'fourStep', label: t(`${k}.center.fourStep` as TKey) },
                  { value: 'quick', label: t(`${k}.center.quick` as TKey) },
                ]}
              />
              {mode === 'quick' && <p className={ui.muted}>{t(`${k}.center.quickBody` as TKey)}</p>}
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                disabled={blocked || busy || !idle}
                onClick={startCenter}
                data-testid="fw-center-start"
              >
                {t(`${k}.center.start` as TKey)}
              </button>
            </div>
          )}
        </section>

        <section className={ui.panel} data-testid="fw-range">
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t(`${k}.range.title` as TKey)}</h2>
          </div>
          <div className={css.step}>
            <p>{t(`${k}.range.body` as TKey)}</p>
            {status.phase === 'range' ? (
              <>
                <p className={css.big} aria-live="polite">
                  {t(`${k}.range.coverage` as TKey, { left: pct(cov.left), right: pct(cov.right) })}
                </p>
                <button
                  type="button"
                  className={`${ui.btn} ${ui.primary}`}
                  disabled={busy || !canFinishRange}
                  onClick={finishRange}
                  data-testid="fw-range-finish"
                >
                  {canFinishRange
                    ? t(`${k}.range.finish` as TKey)
                    : t(`${k}.range.wait` as TKey, { s: RANGE_UNLOCK_S - elapsed })}
                </button>
              </>
            ) : (
              <button
                type="button"
                className={`${ui.btn} ${ui.primary}`}
                disabled={blocked || busy || !idle}
                onClick={startRange}
                data-testid="fw-range-start"
              >
                {t(`${k}.range.start` as TKey)}
              </button>
            )}
          </div>
        </section>

        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t(`${k}.check.title` as TKey)}</h2>
          </div>
          <p className={ui.muted}>{t(`${k}.check.body` as TKey)}</p>
          <div className={css.scopes}>
            <StickScope id={c.id} side="left" size={170} />
            <StickScope id={c.id} side="right" size={170} />
          </div>
        </section>

        <section className={ui.panel} data-testid="fw-decide">
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t(`${k}.decide.title` as TKey)}</h2>
          </div>
          <p>{t(`${k}.decide.body` as TKey)}</p>
          <label className={css.ack}>
            <input
              type="checkbox"
              checked={ack}
              onChange={(e) => setAck(e.target.checked)}
              data-testid="fw-ack"
            />
            {t(`${k}.decide.ack` as TKey)}
          </label>
          <div className={css.actions}>
            <button
              type="button"
              className={`${ui.btn} ${ui.danger}`}
              disabled={!status.unsavedChanges || !ack || busy || blocked || !idle}
              onClick={() => setConfirm(true)}
              data-testid="fw-save"
            >
              <Icon name="download" size={16} /> {t(`${k}.decide.save` as TKey)}
            </button>
            <button
              type="button"
              className={ui.btn}
              disabled={!status.unsavedChanges || busy}
              onClick={async () => {
                if (await run({ op: 'reboot' }))
                  toast({ tone: 'info', title: t(`${k}.decide.discarded` as TKey) });
              }}
              data-testid="fw-discard"
            >
              <Icon name="refresh" size={16} /> {t(`${k}.decide.discard` as TKey)}
            </button>
          </div>
        </section>
      </div>
      <ConfirmDialog
        open={confirm}
        danger
        title={t(`${k}.decide.confirmTitle` as TKey)}
        body={t(`${k}.decide.confirmBody` as TKey)}
        confirmLabel={t(`${k}.decide.save` as TKey)}
        onCancel={() => setConfirm(false)}
        onConfirm={async () => {
          setConfirm(false);
          if (await run({ op: 'savePermanent', acknowledgeRisk: ack })) {
            toast({ tone: 'ok', title: t(`${k}.decide.saved` as TKey) });
            setAck(false);
          }
        }}
      />
    </div>
  );
}

function CalRow({ label, cal }: { label: string; cal: SwitchStickCal | null }) {
  const t = useT();
  return (
    <tr>
      <td className={css.ui}>{label}</td>
      {cal ? (
        <>
          <td>
            {cal.centerX} / {cal.centerY}
          </td>
          <td>
            {cal.xMinBelow}/{cal.xMaxAbove} · {cal.yMinBelow}/{cal.yMaxAbove}
          </td>
        </>
      ) : (
        <td colSpan={2} className={css.ui}>
          {t('calibration.firmware.switch.noUser')}
        </td>
      )}
    </tr>
  );
}

function SwitchFirmware({ c }: { c: ControllerInfo }) {
  const t = useT();
  const toast = useApp((s) => s.toast);
  const k = 'calibration.firmware.switch';
  const { status, busy, run } = useFirmware(c);
  const [data, setData] = useState<SwitchCalibrationData | null>(null);
  const [proposals, setProposals] = useState<SwitchProposals | null>(null);
  const [measured, setMeasured] = useState<Record<string, boolean>>({});
  const [recording, setRecording] = useState<string | null>(null);
  const [ack, setAck] = useState(false);
  const [confirm, setConfirm] = useState<'write' | 'clear' | null>(null);
  const hid = c.backend === 'hid';

  const read = async () => {
    const r = await run({ op: 'switchRead' });
    if (r?.switch) setData(r.switch);
  };

  const measure = async (kind: 'rest' | 'rotation', side: 'left' | 'right') => {
    const key = `${kind}-${side}`;
    try {
      if (recording === key) {
        await api.captureStop();
        setRecording(null);
        setMeasured((m) => ({ ...m, [key]: true }));
        return;
      }
      await api.captureStart(c.id, kind, side);
      if (kind === 'rest') {
        setRecording(key);
        await new Promise((res) => setTimeout(res, 2000));
        await api.captureStop();
        setRecording(null);
        setMeasured((m) => ({ ...m, [key]: true }));
      } else setRecording(key);
    } catch (e) {
      setRecording(null);
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  };

  const propose = async () => {
    try {
      setProposals(await api.firmwareSwitchPropose(c.id));
    } catch (e) {
      toast(errorToast(errorCode(e), errorMessage(e)));
    }
  };

  const apply = async (kind: 'write' | 'clear') => {
    if (kind === 'clear') {
      const r = await run({ op: 'switchClear', acknowledgeRisk: ack });
      if (r) {
        setData(r.switch);
        toast({ tone: 'ok', title: t(`${k}.cleared` as TKey, { path: r.backupPath ?? '—' }) });
      }
      return;
    }
    if (!proposals) return;
    const cur = proposals.current;
    const left = proposals.left?.calibration ?? cur.userLeft ?? cur.factoryLeft;
    const right = proposals.right?.calibration ?? cur.userRight ?? cur.factoryRight;
    const r = await run({ op: 'switchWrite', left, right, acknowledgeRisk: ack });
    if (r) {
      setData(r.switch);
      setProposals(null);
      toast({ tone: 'ok', title: t(`${k}.written` as TKey, { path: r.backupPath ?? '—' }) });
    }
  };

  if (!hid) {
    return (
      <section className={ui.panel}>
        <p>{t(`${k}.enableHint` as TKey)}</p>
        <button
          type="button"
          className={`${ui.btn} ${ui.primary}`}
          style={{ marginTop: 12 }}
          onClick={() => void api.enableSensors(c.id)}
        >
          {t(`${k}.enable` as TKey)}
        </button>
      </section>
    );
  }

  return (
    <div className={css.layout}>
      <div className={css.column}>
        {status && <StatusCard s={status} onRefresh={() => void run({ op: 'status' })} />}
        <p className={`${ui.callout} ${ui.calloutWarn}`}>
          <Icon name="alert" /> {t(`${k}.untested` as TKey)}
        </p>
        <Risks switchPad />
      </div>
      <div className={css.column}>
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t(`${k}.read` as TKey)}</h2>
            <span className={ui.spacer} />
            <button
              type="button"
              className={`${ui.btn} ${ui.small}`}
              disabled={busy}
              onClick={read}
            >
              <Icon name="refresh" size={14} /> {t(`${k}.read` as TKey)}
            </button>
          </div>
          {data && (
            <table className={ui.table}>
              <thead>
                <tr>
                  <th />
                  <th>{t(`${k}.center` as TKey)}</th>
                  <th>{t(`${k}.ranges` as TKey)}</th>
                </tr>
              </thead>
              <tbody>
                <CalRow
                  label={`${t(`${k}.left` as TKey)} · ${t(`${k}.factory` as TKey)}`}
                  cal={data.factoryLeft}
                />
                <CalRow
                  label={`${t(`${k}.left` as TKey)} · ${t(`${k}.user` as TKey)}`}
                  cal={data.userLeft}
                />
                <CalRow
                  label={`${t(`${k}.right` as TKey)} · ${t(`${k}.factory` as TKey)}`}
                  cal={data.factoryRight}
                />
                <CalRow
                  label={`${t(`${k}.right` as TKey)} · ${t(`${k}.user` as TKey)}`}
                  cal={data.userRight}
                />
              </tbody>
            </table>
          )}
        </section>
        <section className={ui.panel}>
          <div className={ui.panelHead}>
            <h2 className={ui.panelTitle}>{t(`${k}.measureTitle` as TKey)}</h2>
          </div>
          <p className={ui.muted}>{t(`${k}.measureBody` as TKey)}</p>
          <div className={css.measure}>
            {(['left', 'right'] as const).map((side) => (
              <div key={side}>
                <p className={css.ui}>{t(`${k}.${side}` as TKey)}</p>
                {(['rest', 'rotation'] as const).map((kind) => {
                  const key = `${kind}-${side}`;
                  return (
                    <button
                      key={kind}
                      type="button"
                      className={`${ui.btn} ${ui.small}`}
                      disabled={!!recording && recording !== key}
                      onClick={() => void measure(kind, side)}
                    >
                      {recording === key
                        ? kind === 'rotation'
                          ? t('calibration.wizard.rotation.finish')
                          : '…'
                        : t(`${k}.${kind}` as TKey)}
                      {measured[key] && ` ✓`}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
          <button
            type="button"
            className={`${ui.btn} ${ui.primary}`}
            style={{ marginTop: 12 }}
            onClick={propose}
          >
            {t(`${k}.propose` as TKey)}
          </button>
          {proposals && (
            <>
              <table className={ui.table} style={{ marginTop: 12 }}>
                <tbody>
                  <CalRow
                    label={`${t(`${k}.left` as TKey)} · ${t(`${k}.proposal` as TKey)}`}
                    cal={proposals.left?.calibration ?? null}
                  />
                  <CalRow
                    label={`${t(`${k}.right` as TKey)} · ${t(`${k}.proposal` as TKey)}`}
                    cal={proposals.right?.calibration ?? null}
                  />
                </tbody>
              </table>
              {(proposals.left?.raisedToMinimum || proposals.right?.raisedToMinimum) && (
                <p className={`${ui.callout} ${ui.calloutWarn}`}>{t(`${k}.raised` as TKey)}</p>
              )}
            </>
          )}
        </section>
        <section className={ui.panel}>
          <label className={css.ack}>
            <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} />
            {t(`${k}.ack` as TKey)}
          </label>
          <div className={css.actions}>
            <button
              type="button"
              className={`${ui.btn} ${ui.danger}`}
              disabled={!ack || !proposals || busy}
              onClick={() => setConfirm('write')}
            >
              {t(`${k}.write` as TKey)}
            </button>
            <button
              type="button"
              className={ui.btn}
              disabled={!ack || busy}
              onClick={() => setConfirm('clear')}
            >
              {t(`${k}.clear` as TKey)}
            </button>
          </div>
        </section>
      </div>
      <ConfirmDialog
        open={!!confirm}
        danger
        title={confirm === 'clear' ? t(`${k}.clear` as TKey) : t(`${k}.write` as TKey)}
        body={confirm === 'clear' ? t(`${k}.clearConfirm` as TKey) : t(`${k}.writeConfirm` as TKey)}
        onCancel={() => setConfirm(null)}
        onConfirm={() => {
          const kind = confirm;
          setConfirm(null);
          if (kind) void apply(kind);
        }}
      />
    </div>
  );
}

export function FirmwarePanel({ c }: { c: ControllerInfo }) {
  const t = useT();
  const support = firmwareSupport(c);
  return (
    <div className={css.wrap}>
      <header>
        <h2 className={css.title}>{t('calibration.firmware.title')}</h2>
        <p className={ui.muted}>{t('calibration.firmware.intro')}</p>
        {c.isDemo && (
          <p className={`${ui.callout} ${ui.calloutInfo}`} style={{ marginTop: 8 }}>
            <Icon name="info" /> {t('calibration.firmware.simulated')}
          </p>
        )}
      </header>
      {support === 'sony' && <SonyFirmware c={c} />}
      {support === 'switch' && <SwitchFirmware c={c} />}
      {support === 'needsHid' && (
        <p className={`${ui.callout} ${ui.calloutWarn}`}>
          <Icon name="alert" /> {t('errors.warnings.hidrawPermission')}
        </p>
      )}
      {support === 'none' && (
        <p className={`${ui.callout} ${ui.calloutInfo}`} data-testid="fw-unsupported">
          <Icon name="info" />
          {c.kind === 'xbox'
            ? t('calibration.firmware.whyNot.xbox')
            : t('calibration.firmware.whyNot.generic')}
        </p>
      )}
    </div>
  );
}
