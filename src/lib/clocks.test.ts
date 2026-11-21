import { afterEach, beforeEach, expect, it, vi } from 'vitest';
const mocks = vi.hoisted(() => ({ call: vi.fn(), granted: vi.fn(), request: vi.fn(), send: vi.fn() }));
vi.mock('./ipc', () => ({ inDesktopApp: () => true, call: mocks.call }));
vi.mock('@tauri-apps/plugin-notification', () => ({ isPermissionGranted: mocks.granted, requestPermission: mocks.request, sendNotification: mocks.send }));
import { Clocks } from './clocks.svelte';
let stop: (() => void) | undefined;
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval', 'performance'] });
  vi.stubGlobal('navigator', {});
  mocks.call.mockReset().mockResolvedValue(undefined);
  mocks.granted.mockReset().mockResolvedValue(true);
  mocks.request.mockReset().mockResolvedValue('granted');
  mocks.send.mockReset();
});
afterEach(async () => { stop?.(); stop=undefined; await Promise.resolve(); vi.useRealTimers(); vi.unstubAllGlobals(); });

it('arms native notifications, cancels on pause/reset, and keeps monotonic elapsed time', async () => {
  const clocks = new Clocks(); clocks.timer.duration = 60000; stop = clocks.startTicks();
  await clocks.toggleTimer();
  expect(mocks.call).toHaveBeenLastCalledWith('timer_notification_set', { milliseconds: 60000 });
  await vi.advanceTimersByTimeAsync(30000);
  await clocks.toggleTimer();
  expect(mocks.call).toHaveBeenLastCalledWith('timer_notification_set', { milliseconds: null });
  expect(clocks.timer.elapsed(performance.now())).toBe(30000);
  await clocks.toggleTimer();
  expect(mocks.call).toHaveBeenLastCalledWith('timer_notification_set', { milliseconds: 30000 });
  await clocks.resetTimer(10000);
  expect(clocks.timer.running).toBe(false);
  expect(clocks.timer.remaining(performance.now())).toBe(10000);
  expect(mocks.call).toHaveBeenLastCalledWith('timer_notification_set', { milliseconds: null });
});

it('does not send a duplicate desktop notification when the UI observes native timer completion', async () => {
  const clocks = new Clocks(); clocks.timer.duration = 1000; stop = clocks.startTicks();
  await clocks.toggleTimer();
  await vi.advanceTimersByTimeAsync(2000);
  expect(clocks.timer.completed).toBe(true);
  expect(clocks.notice).toBe('Timer finished.');
  expect(mocks.send).not.toHaveBeenCalled();
});

it('ignores repeated starts while notification permission is pending', async () => {
  let resolve!: (value: string) => void;
  mocks.granted.mockResolvedValue(false);
  mocks.request.mockReturnValue(new Promise(resolveValue => { resolve = resolveValue; }));
  const clocks = new Clocks(); stop = clocks.startTicks();
  const first = clocks.toggleTimer();
  await Promise.resolve();
  expect(clocks.busy).toBe(true);
  await clocks.toggleTimer();
  expect(mocks.request).toHaveBeenCalledTimes(1);
  resolve('granted'); await first;
  expect(clocks.timer.running).toBe(true);
  expect(mocks.call.mock.calls.filter(([, args]) => args.milliseconds !== null)).toHaveLength(1);
});

it('an obsolete initial cancel failure cannot cause duplicate notifications for a later start', async () => {
  let reject!: (error:Error)=>void;
  mocks.call.mockImplementationOnce(()=>new Promise((_resolve,rejectValue)=>{reject=rejectValue;}));
  const clocks=new Clocks();clocks.timer.duration=1000;stop=clocks.startTicks();
  const start=clocks.toggleTimer();await Promise.resolve();await Promise.resolve();
  reject(new Error('initial cancel failed'));await start;
  await vi.advanceTimersByTimeAsync(2000);
  expect(clocks.timer.completed).toBe(true);expect(mocks.send).not.toHaveBeenCalled();
});

it('does not start after teardown while notification permission was pending', async () => {
  let resolve!:(value:string)=>void;mocks.granted.mockResolvedValue(false);
  mocks.request.mockReturnValue(new Promise(resolveValue=>{resolve=resolveValue;}));
  const clocks=new Clocks();stop=clocks.startTicks();const start=clocks.toggleTimer();
  await Promise.resolve();stop?.();stop=undefined;resolve('granted');await start;
  expect(clocks.timer.running).toBe(false);
  expect(mocks.call.mock.calls.filter(([,args])=>args.milliseconds!==null)).toHaveLength(0);
});
