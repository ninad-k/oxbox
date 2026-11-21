/** Typed IPC for screen capture, clipboard history, image resizing and age files. */
import { call } from './ipc';

export interface CaptureTarget { kind: 'monitor' | 'window'; id: number }
export interface CaptureTargetInfo extends CaptureTarget { name: string; x: number; y: number; width: number; height: number; scaleFactor: number }
export interface CaptureResult { path: string; width: number; height: number; clipboardError: string | null }
export interface Selection { x: number; y: number; width: number; height: number }
export interface CaptureEvent { result: CaptureResult | null; error: string | null }
export interface ClipboardItem { id: number; pinned: boolean; capturedAt: number; kind: 'text' | 'image'; preview: string; bytes: number }
export interface ClipboardSnapshot { paused: boolean; bytes: number; totalItems: number; items: ClipboardItem[]; error: string | null }
export type ResizeMode = { kind: 'percent'; percent: number } | { kind: 'fit' | 'exact'; width: number; height: number };
export interface ResizeRequest { paths: string[]; outputDir: string; mode: ResizeMode; format: 'png' | 'jpeg' | 'webp'; jpegQuality: number }

export const mediaApi = {
  captureTargets: () => call<CaptureTargetInfo[]>('capture_targets'),
  captureScreenshot: (target: CaptureTarget, delay: number, outputDir: string | null) => call<CaptureResult>('capture_screenshot', { target, delay, outputDir }),
  startAreaCapture: (delay: number, outputDir: string | null) => call<void>('start_area_capture', { delay, outputDir }),
  areaCaptureFrame: () => call<ArrayBuffer>('area_capture_frame'),
  completeAreaCapture: (selection: Selection) => call<void>('complete_area_capture', { selection }),
  cancelAreaCapture: () => call<void>('cancel_area_capture'),
  openScreenshotFolder: () => call<void>('open_screenshot_folder'),
  clipboardSnapshot: (query = '') => call<ClipboardSnapshot>('clipboard_snapshot', { query }),
  clipboardSetPaused: (paused: boolean) => call<void>('clipboard_set_paused', { paused }),
  clipboardPin: (id: number, pinned: boolean) => call<void>('clipboard_pin', { id, pinned }),
  clipboardDelete: (id: number) => call<void>('clipboard_delete', { id }),
  clipboardClear: () => call<void>('clipboard_clear'),
  clipboardCopy: (id: number) => call<void>('clipboard_copy', { id }),
  resizeImages: (jobId: string, request: ResizeRequest) => call<string[]>('resize_images', { jobId, request }),
  cryptFile: (jobId: string, source: string, output: string, passphrase: string, decrypt: boolean) => call<void>('crypt_file', { jobId, source, output, passphrase, decrypt }),
};
