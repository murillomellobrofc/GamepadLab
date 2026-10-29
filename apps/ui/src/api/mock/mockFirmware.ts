import type { FirmwareOp, FirmwareResult, FirmwareStatus } from '../types';
import type { DemoSim } from './demoSim';

/** Browser twin of the Rust demo firmware (DemoBackend::firmware). */
export class MockFirmware {
  phase: FirmwareStatus['phase'] = 'idle';
  samples = 0;
  unsaved = false;
  saved = { left: true, right: true } as { left: boolean; right: boolean };

  status(): FirmwareStatus {
    return {
      model: 'ds5',
      simulated: true,
      genuine: true,
      cloneReason: null,
      firmware: 'Demo 1.0 (simulated)',
      usb: true,
      battery: 78,
      nv: 'locked',
      phase: this.phase,
      centerSamples: this.samples,
      unsavedChanges: this.unsaved,
      blockers: [],
    };
  }

  run(op: FirmwareOp, sim: DemoSim): FirmwareResult {
    const fail = (code: string, message: string) => {
      throw { code, message };
    };
    switch (op.op) {
      case 'status':
        break;
      case 'centerBegin':
      case 'rangeBegin':
        if (this.phase !== 'idle') fail('firmwareProtocol', 'another calibration is still running');
        this.phase = op.op === 'centerBegin' ? 'center' : 'range';
        this.samples = 0;
        break;
      case 'centerSample':
        if (this.phase !== 'center') fail('firmwareProtocol', 'center calibration is not running');
        this.samples++;
        break;
      case 'centerEnd':
        if (this.phase !== 'center' || !this.samples)
          fail('firmwareProtocol', 'take at least one center sample first');
        this.phase = 'idle';
        this.unsaved = true;
        sim.calibrated.center = true;
        break;
      case 'rangeEnd':
        if (this.phase !== 'range') fail('firmwareProtocol', 'range calibration is not running');
        this.phase = 'idle';
        this.unsaved = true;
        sim.calibrated.range = true;
        break;
      case 'savePermanent':
        if (!op.acknowledgeRisk) fail('firmwareBlocked', 'the risk was not acknowledged');
        if (!this.unsaved)
          fail('firmwareBlocked', 'calibrate (temporarily) and check the result first');
        this.unsaved = false;
        sim.savedCalibration = { ...sim.calibrated };
        break;
      case 'reboot':
        this.unsaved = false;
        this.phase = 'idle';
        sim.calibrated = { ...sim.savedCalibration };
        break;
      default:
        fail('rumbleUnsupported', 'Switch operations on the demo DualSense');
    }
    return { status: this.status(), switch: null, backup: null, backupPath: null };
  }
}
