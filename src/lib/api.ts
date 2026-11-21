/**
 * Typed bridge to the Rust commands in `src-tauri/src/commands.rs`.
 * Every tool talks to Rust through this file. Never call `invoke` directly from a view.
 */
import { invoke, isTauri } from '@tauri-apps/api/core';
import type { Os } from './registry';

/** Mirrors `ToolError` in `src-tauri/src/error.rs`. */
export class ToolError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = 'ToolError';
  }
}

export const inDesktopApp = (): boolean => isTauri();

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!inDesktopApp()) {
    throw new ToolError('not_desktop', 'Open Oxbox as the desktop app to use this tool.');
  }
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    const e = err as Partial<{ code: string; message: string }>;
    throw new ToolError(e?.code ?? 'internal', e?.message ?? String(err));
  }
}

export interface AppInfo {
  version: string;
  os: Os | 'other';
}

/** Mirrors `TextCase` in `crates/oxbox-core/src/tools/text_transform.rs`. */
export type TextCase =
  | 'upper'
  | 'lower'
  | 'title'
  | 'sentence'
  | 'camel'
  | 'pascal'
  | 'snake'
  | 'kebab'
  | 'constant';

/** Mirrors `Algorithm` in `crates/oxbox-core/src/tools/checksum.rs`. */
export type HashAlgorithm = 'sha256' | 'sha1' | 'md5' | 'blake3';

export const api = {
  appInfo: () => call<AppInfo>('app_info'),
  transformText: (text: string, textCase: TextCase) =>
    call<string>('transform_text', { text, case: textCase }),
  checksumFile: (path: string, algorithm: HashAlgorithm) =>
    call<string>('checksum_file', { path, algorithm }),
};

/** Best-effort OS guess for the browser preview, where `app_info` is unavailable. */
export function guessOs(): Os | 'other' {
  const ua = globalThis.navigator?.userAgent ?? '';
  if (/Windows/i.test(ua)) return 'windows';
  if (/Mac OS X|Macintosh/i.test(ua)) return 'macos';
  if (/Linux/i.test(ua)) return 'linux';
  return 'other';
}
