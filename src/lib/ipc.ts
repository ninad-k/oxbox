/** Shared desktop transport. Tool views use typed API wrappers. */
import { invoke, isTauri } from '@tauri-apps/api/core';

export class ToolError extends Error {
  constructor(public readonly code: string, message: string) {
    super(message);
    this.name = 'ToolError';
  }
}

export const inDesktopApp = (): boolean => isTauri();

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!inDesktopApp()) throw new ToolError('not_desktop', 'Open Oxbox as the desktop app to use this tool.');
  try { return await invoke<T>(command, args); }
  catch (err) {
    const e = err as Partial<{code: string; message: string}>;
    throw new ToolError(e?.code ?? 'internal', e?.message ?? String(err));
  }
}
