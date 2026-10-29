/**
 * Complementary filter: integrates the gyroscope and pulls pitch/roll
 * towards the gravity direction from the accelerometer. Yaw is gyro only
 * (no magnetometer on game controllers), so it drifts slowly; "tare" resets.
 */
export interface Orientation {
  pitch: number; // degrees
  roll: number;
  yaw: number;
}

export class ComplementaryFilter {
  o: Orientation = { pitch: 0, roll: 0, yaw: 0 };
  bias: [number, number, number] = [0, 0, 0];
  private lastUs: number | null = null;
  private alpha: number;

  constructor(alpha = 0.98) {
    this.alpha = alpha;
  }

  /** gyro in °/s (pitch, yaw, roll as reported), accel in g. */
  update(
    gyro: [number, number, number],
    accel: [number, number, number],
    tUs: number,
  ): Orientation {
    if (this.lastUs === null || tUs <= this.lastUs || tUs - this.lastUs > 200_000) {
      this.lastUs = tUs;
      return this.o;
    }
    const dt = (tUs - this.lastUs) / 1e6;
    this.lastUs = tUs;
    const gx = gyro[0] - this.bias[0];
    const gy = gyro[1] - this.bias[1];
    const gz = gyro[2] - this.bias[2];
    const [ax, ay, az] = accel;
    const accPitch = (Math.atan2(az, Math.hypot(ax, ay)) * 180) / Math.PI;
    const accRoll = (Math.atan2(ax, -ay) * 180) / Math.PI;
    const usable = Math.abs(Math.hypot(ax, ay, az) - 1) < 0.35;
    const a = usable ? this.alpha : 1;
    this.o = {
      pitch: a * (this.o.pitch + gx * dt) + (1 - a) * accPitch,
      roll: a * (this.o.roll + gz * dt) + (1 - a) * accRoll,
      yaw: this.o.yaw + gy * dt,
    };
    return this.o;
  }

  /** Zero the orientation and learn the current gyro reading as bias. */
  tare(gyro: [number, number, number] | null) {
    this.o = { pitch: 0, roll: 0, yaw: 0 };
    if (gyro) this.bias = [...gyro];
  }
}
