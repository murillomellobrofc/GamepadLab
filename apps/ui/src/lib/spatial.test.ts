import { describe, expect, it } from 'vitest';
import { pickNext } from './spatial';

const box = (x: number, y: number) => ({ x, y, w: 10, h: 10 });

describe('spatial navigation', () => {
  const grid = [box(0, 0), box(100, 0), box(0, 100), box(100, 100), box(300, 5)];
  it('moves in each direction', () => {
    expect(pickNext(grid[0], grid, 'right')).toBe(1);
    expect(pickNext(grid[0], grid, 'down')).toBe(2);
    expect(pickNext(grid[3], grid, 'up')).toBe(1);
    expect(pickNext(grid[3], grid, 'left')).toBe(2);
  });
  it('prefers aligned targets over slightly closer offset ones', () => {
    const items = [box(0, 0), box(60, 80), box(120, 0)];
    expect(pickNext(items[0], items, 'right')).toBe(2);
  });
  it('returns -1 when nothing lies that way', () => {
    expect(pickNext(grid[0], grid, 'left')).toBe(-1);
    expect(pickNext(grid[0], grid, 'up')).toBe(-1);
  });
});
