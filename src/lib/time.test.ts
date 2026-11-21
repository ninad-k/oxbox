import { describe, it, expect } from 'vitest';
import { Clock, formatTime } from './time';
describe('monotonic clocks', () => {
  it('accounts for hidden-window delays and pauses without drift', () => {
    const clock = new Clock(); clock.start(1000); clock.pause(61000);
    expect(clock.elapsed(90000)).toBe(60000); clock.start(100000);
    expect(clock.elapsed(101000)).toBe(61000);
  });
  it('countdown finishes once even across a large clock jump', () => {
    const clock = new Clock(60000); clock.start(0);
    expect(clock.remaining(125000)).toBe(0);
    expect(clock.finish(125000)).toBe(true); expect(clock.finish(130000)).toBe(false);
    clock.reset(); expect(clock.remaining(0)).toBe(60000);
  });
  it('formats long durations', () => {expect(formatTime(3661000)).toBe('01:01:01');});
});
