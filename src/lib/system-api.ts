/** Typed system-tool IPC. All metrics are local; no network requests are made. */
import { call } from './ipc';
export interface ProcessInfo { pid: number; name: string; cpuPercent: number; memory: number }
export interface DiskInfo { name: string; mount: string; total: number; available: number }
export interface SystemSnapshot { cpuPercent: number; cores: number[]; memoryTotal: number; memoryUsed: number; swapTotal: number; swapUsed: number; disks: DiskInfo[]; processesCpu: ProcessInfo[]; processesMemory: ProcessInfo[] }
export interface InterfaceUsage { name: string; downloadPerSecond: number; uploadPerSecond: number; received: number; transmitted: number; virtualInterface: boolean }
export interface AwakeStatus { active: boolean; display: boolean; until: number | null }
export type PowerAction = 'sleep' | 'shutdown' | 'restart' | 'sign-out';
export interface SleepTimerStatus { action: PowerAction; deadline: number }
export interface SleepInfo { timer: SleepTimerStatus | null; error: string | null; remainingSeconds: number }
export interface BreakSettings { minutes: number; eyeRest: boolean; pauseFullscreen: boolean; enabled: boolean }
export interface BreakStatus { enabled: boolean; awaitingBreak: boolean; remainingSeconds: number; eyeRest: boolean; paused: boolean }
export interface BreakInfo { settings: BreakSettings; status: BreakStatus }
export const systemApi = {
  systemSnapshot: () => call<SystemSnapshot>('system_snapshot'),
  networkSample: () => call<InterfaceUsage[]>('network_sample'),
  awakeStart: (display: boolean, minutes: number | null, until: number | null) => call<AwakeStatus>('awake_start', { display, minutes, until }),
  awakeStop: () => call<void>('awake_stop'),
  awakeStatus: () => call<AwakeStatus>('awake_status'),
  lockScreen: () => call<void>('lock_screen'),
  sleepArm: (seconds: number, action: PowerAction, confirmed: boolean) => call<SleepTimerStatus>('sleep_arm', { seconds, action, confirmed }),
  sleepCancel: () => call<void>('sleep_cancel'),
  sleepStatus: () => call<SleepInfo>('sleep_status'),
  breakConfigure: (settings: BreakSettings) => call<BreakStatus>('break_configure', { settings }),
  breakStatus: () => call<BreakInfo>('break_status'),
  breakSnooze: () => call<void>('break_snooze'),
  breakSkip: () => call<void>('break_skip'),
  desktopSetHidden: (hidden: boolean) => call<boolean>('desktop_set_hidden', { hidden }),
  desktopStatus: () => call<boolean>('desktop_status'),
};
export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0 B';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  const index = Math.max(0, Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1));
  return `${(value / 1024 ** index).toFixed(index ? 1 : 0)} ${units[index]}`;
}
