/**
 * Typed bridge to the Rust commands in `src-tauri/src/commands.rs`.
 * Every tool talks to Rust through this file. Never call `invoke` directly from a view.
 */
import { call } from './ipc';
export { ToolError, inDesktopApp } from './ipc';
import type { Os } from './registry';
import { fileApi } from './file-api';
import { systemApi } from './system-api';
import { mediaApi } from './media-api';
import { currencyApi } from './currency-api';
import { roadmapTimeApi } from './roadmap-time-api';
import { roadmapSystemApi } from './roadmap-system-api';
import { roadmapMediaApi } from './roadmap-media-api';
import { mouseActivityApi } from './mouse-activity-api';
export * from './file-api';
export { systemApi } from './system-api';
export { mediaApi } from './media-api';

/** Mirrors `ToolError` in `src-tauri/src/error.rs`. */

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
export interface ConversionUnit {id: string; label: string; category: string}
export interface WorldClock {zone: string; date: string; time: string; abbreviation: string; offsetSeconds: number; daytime: boolean}
export type CodeKind = 'qr' | 'code128' | 'ean13';

export const api = {
  runtimePrivacyStatus: () => call<{supported:boolean;configured:boolean;blockedWebResources:number;localWebResources:number}>('runtime_privacy_status'),
  mouseActivity: mouseActivityApi,
  quitApp: () => call<void>('quit_app'),
  ...fileApi,
  ...systemApi,
  ...mediaApi,
  ...currencyApi,
  ...roadmapTimeApi,
  ...roadmapSystemApi,
  ...roadmapMediaApi,
  shortcutStatus: () => call<{shortcut: string; error: string | null}>('shortcut_status'),
  configureShortcut: (shortcut: string) => call<string>('configure_shortcut', {shortcut}),
  conversionUnits: () => call<ConversionUnit[]>('conversion_units'),
  convertUnit: (value: number, from: string, to: string) => call<number>('convert_unit', {value, from, to}),
  timeZones: () => call<string[]>('time_zones'),
  worldClocks: (zones: string[]) => call<WorldClock[]>('world_clocks', {zones}),
  generateCode: (text: string, kind: CodeKind, correction: string, size: number) => call<string>('generate_code', {text, kind, correction, size}),
  saveCode: (text: string, kind: CodeKind, correction: string, size: number, path: string, svg: boolean) => call<void>('save_code', {text, kind, correction, size, path, svg}),
  copyCode: (text: string, kind: CodeKind, correction: string, size: number) => call<void>('copy_code', {text, kind, correction, size}),
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
