import { defaultProfile } from '@/lib/calibration';
import type { CalibrationProfile, DeviceProfiles, Profile } from '../types';

/** In-memory profile store for the browser mock (same rules as the Rust store). */
export class MockProfiles {
  docs = new Map<string, DeviceProfiles>();
  seq = 1;

  private now() {
    return new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  }

  private fresh(key: string, name: string): DeviceProfiles {
    const p = this.make('Padrão', defaultProfile());
    return {
      schemaVersion: 2,
      deviceKey: key,
      deviceName: name,
      activeProfileId: p.id,
      profiles: [p],
    };
  }

  private make(name: string, calibration: CalibrationProfile): Profile {
    const now = this.now();
    return {
      id: `m${this.seq++}`,
      name,
      createdAt: now,
      updatedAt: now,
      calibration: structuredClone(calibration),
    };
  }

  private unique(doc: DeviceProfiles, base: string) {
    const b = base.trim().slice(0, 60) || 'Perfil';
    if (!doc.profiles.some((p) => p.name === b)) return b;
    for (let n = 2; ; n++)
      if (!doc.profiles.some((p) => p.name === `${b} (${n})`)) return `${b} (${n})`;
  }

  load(key: string, name = ''): DeviceProfiles {
    let doc = this.docs.get(key);
    if (!doc) {
      doc = this.fresh(key, name);
      this.docs.set(key, doc);
    }
    return structuredClone(doc);
  }

  private doc(key: string) {
    this.load(key);
    return this.docs.get(key)!;
  }

  private find(doc: DeviceProfiles, id: string) {
    const p = doc.profiles.find((x) => x.id === id);
    if (!p) throw { code: 'notFound', message: `profile ${id} not found` };
    return p;
  }

  active(key: string): CalibrationProfile | null {
    const d = this.doc(key);
    return d.profiles.find((p) => p.id === d.activeProfileId)?.calibration ?? null;
  }

  create(key: string, name: string, cal: CalibrationProfile | null) {
    const d = this.doc(key);
    const p = this.make(this.unique(d, name), cal ?? defaultProfile());
    d.profiles.push(p);
    d.activeProfileId = p.id;
    return structuredClone(p);
  }

  update(key: string, profile: Profile) {
    const p = this.find(this.doc(key), profile.id);
    p.calibration = structuredClone(profile.calibration);
    p.name = profile.name;
    p.updatedAt = this.now();
    return structuredClone(p);
  }

  rename(key: string, id: string, name: string) {
    const d = this.doc(key);
    this.find(d, id).name = this.unique(d, name);
  }

  duplicate(key: string, id: string) {
    const d = this.doc(key);
    const src = this.find(d, id);
    const p = this.make(this.unique(d, `${src.name} (cópia)`), src.calibration);
    d.profiles.push(p);
    return structuredClone(p);
  }

  delete(key: string, id: string) {
    const d = this.doc(key);
    if (d.profiles.length <= 1)
      throw { code: 'invalidInput', message: 'cannot delete the only profile' };
    this.find(d, id);
    d.profiles = d.profiles.filter((p) => p.id !== id);
    if (d.activeProfileId === id) d.activeProfileId = d.profiles[0].id;
  }

  setActive(key: string, id: string) {
    const d = this.doc(key);
    this.find(d, id);
    d.activeProfileId = id;
  }

  reset(key: string, id: string) {
    const p = this.find(this.doc(key), id);
    p.calibration = defaultProfile();
    p.updatedAt = this.now();
    return structuredClone(p);
  }

  export(key: string, id: string) {
    const d = this.doc(key);
    const profile = this.find(d, id);
    return JSON.stringify(
      {
        format: 'gamepadlab-profile',
        schemaVersion: 2,
        exportedAt: this.now(),
        appVersion: '0.1.0',
        deviceKey: key,
        deviceName: d.deviceName,
        profile,
      },
      null,
      2,
    );
  }

  import(key: string, json: string) {
    let v: { format?: string; profile?: Profile };
    try {
      v = JSON.parse(json);
    } catch {
      throw { code: 'invalidInput', message: 'not JSON' };
    }
    if (v.format !== 'gamepadlab-profile' || !v.profile?.calibration)
      throw { code: 'invalidInput', message: 'this is not a GamepadLab profile' };
    const d = this.doc(key);
    const p = this.make(this.unique(d, v.profile.name ?? 'Importado'), {
      ...defaultProfile(),
      ...v.profile.calibration,
    });
    d.profiles.push(p);
    return structuredClone(p);
  }
}
