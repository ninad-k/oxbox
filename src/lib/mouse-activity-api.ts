/** Explicit-session idle-aware pointer activity. No command starts implicitly. */
import { call } from './ipc';

export interface MouseActivityConfig { idleSeconds: number; intervalSeconds: number; durationMinutes: number | null }
export type MouseActivityPhase = 'stopped' | 'waiting' | 'paused' | 'simulating';
export interface MouseActivityStatus { active: boolean; phase: MouseActivityPhase; idleSeconds: number; movements: number; remainingSeconds: number | null; error: string | null }
export const mouseActivityApi = {
  start: async (config: MouseActivityConfig) => {
    const request = await call<string>('mouse_activity_reserve', { action: 'start' });
    return call<MouseActivityStatus>('mouse_activity_start', { config, request });
  },
  stop: async () => {
    const request = await call<string>('mouse_activity_reserve', { action: 'stop' });
    return call<MouseActivityStatus>('mouse_activity_stop', { request });
  },
  status: () => call<MouseActivityStatus>('mouse_activity_status'),
};
export function activityLabel(status: MouseActivityStatus): string {
  if (!status.active) return 'Mouse Activity is stopped.';
  if (status.phase === 'paused') return 'Paused by input. Waiting for the full idle delay before moving again.';
  return status.phase === 'simulating' ? 'Simulating while idle. Real input suspends further movements.' : 'Waiting for idle. Real keyboard or mouse input restarts the idle delay.';
}
