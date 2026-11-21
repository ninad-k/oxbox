/**
 * The tool catalogue: the single source of truth for what appears in the launcher.
 *
 * Adding a tool (see AGENTS.md for the full checklist):
 *   1. Add or update its entry below. Keep `id` stable forever (kebab-case).
 *   2. Set `status: 'ready'` and `load` once the Svelte view exists in `src/lib/tools/`.
 */
import type { Component } from 'svelte';
import type { LucideIcon } from '@lucide/svelte';
import Activity from '@lucide/svelte/icons/activity';
import Archive from '@lucide/svelte/icons/archive';
import BrushCleaning from '@lucide/svelte/icons/brush-cleaning';
import Camera from '@lucide/svelte/icons/camera';
import CaseSensitive from '@lucide/svelte/icons/case-sensitive';
import ClipboardList from '@lucide/svelte/icons/clipboard-list';
import Clock from '@lucide/svelte/icons/clock';
import Coffee from '@lucide/svelte/icons/coffee';
import Copy from '@lucide/svelte/icons/copy';
import Cpu from '@lucide/svelte/icons/cpu';
import EyeOff from '@lucide/svelte/icons/eye-off';
import FileCheck from '@lucide/svelte/icons/file-check';
import FileLock from '@lucide/svelte/icons/file-lock';
import FolderOpen from '@lucide/svelte/icons/folder-open';
import Globe from '@lucide/svelte/icons/globe';
import Hourglass from '@lucide/svelte/icons/hourglass';
import Image from '@lucide/svelte/icons/image';
import Lock from '@lucide/svelte/icons/lock';
import Network from '@lucide/svelte/icons/network';
import QrCode from '@lucide/svelte/icons/qr-code';
import Ruler from '@lucide/svelte/icons/ruler';
import Shield from '@lucide/svelte/icons/shield';
import Sparkles from '@lucide/svelte/icons/sparkles';
import Sun from '@lucide/svelte/icons/sun';
import Timer from '@lucide/svelte/icons/timer';
import {roadmapTools} from './roadmap-registry';

export type Os = 'windows' | 'macos' | 'linux';
export const ALL_OS: Os[] = ['windows', 'macos', 'linux'];

export type CategoryId =
  | 'productivity'
  | 'time'
  | 'files'
  | 'capture'
  | 'media'
  | 'system'
  | 'privacy';

export interface Category {
  id: CategoryId;
  label: string;
  icon: LucideIcon;
  /** CSS colour token used for the category's icon chips (defined in app.css). */
  tone: string;
}

export interface ToolDef {
  /** Stable kebab-case id. Also used for favourites and URLs; never rename. */
  id: string;
  name: string;
  /** One line, sentence case, no trailing period. Shown on the tile. */
  description: string;
  category: CategoryId;
  icon: LucideIcon;
  /** Extra search terms. */
  keywords: string[];
  /** OSes the tool supports. The UI shows a notice on others. */
  platforms: Os[];
  /** Roadmap phase that delivers this tool (see tasks/plan.md). */
  phase: number;
  status: 'ready' | 'planned';
  /** Lazy-loads the tool's view. Required when status is 'ready'. */
  load?: () => Promise<{ default: Component }>;
}

export const categories: Category[] = [
  { id: 'productivity', label: 'Productivity', icon: Sparkles, tone: 'var(--tone-blue)' },
  { id: 'time', label: 'Time', icon: Clock, tone: 'var(--tone-teal)' },
  { id: 'files', label: 'Files', icon: FolderOpen, tone: 'var(--tone-amber)' },
  { id: 'capture', label: 'Screen Capture', icon: Camera, tone: 'var(--tone-rose)' },
  { id: 'media', label: 'Images & Media', icon: Image, tone: 'var(--tone-violet)' },
  { id: 'system', label: 'System', icon: Cpu, tone: 'var(--tone-green)' },
  { id: 'privacy', label: 'Privacy & Security', icon: Shield, tone: 'var(--tone-red)' },
];

const baseTools: ToolDef[] = [
  // Productivity
  {
    id: 'transform-text',
    name: 'Transform Text',
    description: 'Switch text between UPPER, lower, Title, camelCase, snake_case and more',
    category: 'productivity',
    icon: CaseSensitive,
    keywords: ['case', 'uppercase', 'lowercase', 'camel', 'snake', 'kebab', 'title'],
    platforms: ALL_OS,
    phase: 0,
    status: 'ready',
    load: () => import('./tools/TransformText.svelte'),
  },
  {
    id: 'clipboard-history',
    name: 'Clipboard History',
    description: 'Keep recent copied text and images and paste any of them again',
    category: 'productivity',
    icon: ClipboardList,
    keywords: ['copy', 'paste', 'clipboard'],
    platforms: ALL_OS,
    phase: 4,
    status: 'ready',
    load: () => import('./tools/ClipboardHistory.svelte'),
  },
  {
    id: 'unit-converter',
    name: 'Unit Converter',
    description: 'Convert length, mass, temperature, data size and time',
    category: 'productivity',
    icon: Ruler,
    keywords: ['convert', 'units', 'metric', 'imperial', 'celsius', 'fahrenheit', 'bytes'],
    platforms: ALL_OS,
    phase: 1,
    status: 'ready',
    load: () => import('./tools/UnitConverter.svelte'),
  },

  // Time
  {
    id: 'break-time',
    name: 'Break Time',
    description: 'Get gentle reminders to step away from the screen',
    category: 'time',
    icon: Coffee,
    keywords: ['reminder', 'pomodoro', 'rest', 'health'],
    platforms: ALL_OS,
    phase: 3,
    status: 'ready',
    load: () => import('./tools/BreakTime.svelte'),
  },
  {
    id: 'timer-stopwatch',
    name: 'Timer & Stopwatch',
    description: 'Count down to zero or measure elapsed time with laps',
    category: 'time',
    icon: Timer,
    keywords: ['countdown', 'stopwatch', 'lap', 'alarm'],
    platforms: ALL_OS,
    phase: 1,
    status: 'ready',
    load: () => import('./tools/TimerStopwatch.svelte'),
  },
  {
    id: 'world-time',
    name: 'World Time',
    description: 'See the current time in cities around the world',
    category: 'time',
    icon: Globe,
    keywords: ['timezone', 'clock', 'city', 'utc'],
    platforms: ALL_OS,
    phase: 1,
    status: 'ready',
    load: () => import('./tools/WorldTime.svelte'),
  },

  // Files
  {
    id: 'verify-checksum',
    name: 'Verify Checksum',
    description: 'Hash a file with SHA-256, SHA-1, MD5 or BLAKE3 and compare it',
    category: 'files',
    icon: FileCheck,
    keywords: ['hash', 'sha256', 'md5', 'blake3', 'integrity', 'verify'],
    platforms: ALL_OS,
    phase: 0,
    status: 'ready',
    load: () => import('./tools/VerifyChecksum.svelte'),
  },
  {
    id: 'find-duplicates',
    name: 'Find Duplicates',
    description: 'Find identical files, even with different names, and free up space',
    category: 'files',
    icon: Copy,
    keywords: ['duplicate', 'same', 'space', 'cleanup'],
    platforms: ALL_OS,
    phase: 2,
    status: 'ready',
    load: () => import('./tools/FindDuplicates.svelte'),
  },
  {
    id: 'archive',
    name: 'Archive & Unarchive',
    description: 'Compress files to zip or tar.gz and extract zip, tar and 7z',
    category: 'files',
    icon: Archive,
    keywords: ['zip', 'unzip', 'compress', 'extract', '7z', 'tar'],
    platforms: ALL_OS,
    phase: 2,
    status: 'ready',
    load: () => import('./tools/Archive.svelte'),
  },
  {
    id: 'clean-drive',
    name: 'Clean Drive',
    description: 'Find temporary and cache files and move what you choose to the trash',
    category: 'files',
    icon: BrushCleaning,
    keywords: ['junk', 'cache', 'temp', 'disk', 'space'],
    platforms: ALL_OS,
    phase: 2,
    status: 'ready',
    load: () => import('./tools/CleanDrive.svelte'),
  },

  // Screen capture
  {
    id: 'screenshot',
    name: 'Screenshot',
    description: 'Capture the whole screen, a window or an area to file and clipboard',
    category: 'capture',
    icon: Camera,
    keywords: ['capture', 'screen', 'window', 'area', 'snip'],
    platforms: ALL_OS,
    phase: 4,
    status: 'ready',
    load: () => import('./tools/Screenshot.svelte'),
  },

  // Images & media
  {
    id: 'resize-images',
    name: 'Resize Images',
    description: 'Resize one or many images and convert between formats',
    category: 'media',
    icon: Image,
    keywords: ['resize', 'scale', 'png', 'jpg', 'webp', 'convert', 'batch'],
    platforms: ALL_OS,
    phase: 4,
    status: 'ready',
    load: () => import('./tools/ResizeImages.svelte'),
  },
  {
    id: 'qr-code',
    name: 'QR & Barcode',
    description: 'Turn text or a link into a QR code or barcode image',
    category: 'media',
    icon: QrCode,
    keywords: ['qr', 'barcode', 'code', 'generate'],
    platforms: ALL_OS,
    phase: 1,
    status: 'ready',
    load: () => import('./tools/QrCode.svelte'),
  },

  // System
  {
    id: 'system-monitor',
    name: 'System Monitor',
    description: 'Watch CPU, memory, disk and the busiest processes in one view',
    category: 'system',
    icon: Activity,
    keywords: ['cpu', 'ram', 'memory', 'disk', 'process', 'performance'],
    platforms: ALL_OS,
    phase: 3,
    status: 'ready',
    load: () => import('./tools/SystemMonitor.svelte'),
  },
  {
    id: 'network-usage',
    name: 'Network Usage',
    description: 'See live upload and download speed and totals',
    category: 'system',
    icon: Network,
    keywords: ['bandwidth', 'speed', 'upload', 'download', 'traffic'],
    platforms: ALL_OS,
    phase: 3,
    status: 'ready',
    load: () => import('./tools/NetworkUsage.svelte'),
  },
  {
    id: 'do-not-sleep',
    name: 'Do Not Sleep',
    description: 'Keep the computer and display awake for as long as you need',
    category: 'system',
    icon: Sun,
    keywords: ['awake', 'caffeine', 'sleep', 'display', 'power'],
    platforms: ALL_OS,
    phase: 3,
    status: 'ready',
    load: () => import('./tools/DoNotSleep.svelte'),
  },
  {
    id: 'sleep-timer',
    name: 'Sleep Timer',
    description: 'Put the computer to sleep or shut it down after a countdown',
    category: 'system',
    icon: Hourglass,
    keywords: ['shutdown', 'sleep', 'timer', 'power off'],
    platforms: ALL_OS,
    phase: 3,
    status: 'ready',
    load: () => import('./tools/SleepTimer.svelte'),
  },

  // Privacy & security
  {
    id: 'lock-screen',
    name: 'Lock Screen',
    description: 'Lock the computer instantly',
    category: 'privacy',
    icon: Lock,
    keywords: ['lock', 'secure', 'away'],
    platforms: ALL_OS,
    phase: 3,
    status: 'ready',
    load: () => import('./tools/LockScreen.svelte'),
  },
  {
    id: 'hide-desktop',
    name: 'Hide Desktop',
    description: 'Hide desktop icons for presentations and screen sharing',
    category: 'privacy',
    icon: EyeOff,
    keywords: ['icons', 'presentation', 'clean', 'desktop'],
    platforms: ['windows', 'macos'],
    phase: 4,
    status: 'ready',
    load: () => import('./tools/HideDesktop.svelte'),
  },
  {
    id: 'encrypt-files',
    name: 'Encrypt Files',
    description: 'Protect files with a passphrase using modern age encryption',
    category: 'privacy',
    icon: FileLock,
    keywords: ['encrypt', 'decrypt', 'password', 'secure', 'age'],
    platforms: ALL_OS,
    phase: 4,
    status: 'ready',
    load: () => import('./tools/EncryptFiles.svelte'),
  },
];

export const categoryById = new Map(categories.map((c) => [c.id, c]));
export const tools:ToolDef[]=[...baseTools,...roadmapTools];
export const toolById = new Map(tools.map((t) => [t.id, t]));
