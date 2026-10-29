import { describe, expect, it } from 'vitest';
import { downsample } from './history';

describe('downsample', () => {
  it('keeps short lists and thins long ones evenly', () => {
    expect(downsample([1, 2, 3], 5)).toEqual([1, 2, 3]);
    const d = downsample(
      Array.from({ length: 1000 }, (_, i) => i),
      10,
    );
    expect(d).toHaveLength(10);
    expect(d[0]).toBe(0);
    expect(d[9]).toBe(900);
  });
});
