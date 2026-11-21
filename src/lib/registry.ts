/** Legacy Svelte adapters for the shared native/website tool catalogue. */
import type { Component } from 'svelte';
import type { LucideIcon } from '@lucide/svelte';
import canonical from '../../catalog/tools.json';
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
import { roadmapIcons, roadmapViews } from './roadmap-registry';

export type Os = 'windows' | 'macos' | 'linux';
export const ALL_OS: Os[] = ['windows', 'macos', 'linux'];
export type CategoryId = 'productivity' | 'time' | 'files' | 'capture' | 'media' | 'system' | 'privacy';
export interface Category { id: CategoryId; label: string; icon: LucideIcon; tone: string }
export type ToolViewLoader = () => Promise<{ default: Component }>;
export interface ToolDef {
  /** Stable catalogue ID; never rename because favorites and URLs use it. */
  id: string;
  name: string;
  description: string;
  category: CategoryId;
  icon: LucideIcon;
  keywords: string[];
  platforms: Os[];
  phase: number;
  status: 'ready' | 'planned';
  load?: ToolViewLoader;
}

export const baseIcons: Record<string, LucideIcon> = {
  'activity': Activity,
  'archive': Archive,
  'brush-cleaning': BrushCleaning,
  'camera': Camera,
  'case-sensitive': CaseSensitive,
  'clipboard-list': ClipboardList,
  'clock': Clock,
  'coffee': Coffee,
  'copy': Copy,
  'cpu': Cpu,
  'eye-off': EyeOff,
  'file-check': FileCheck,
  'file-lock': FileLock,
  'folder-open': FolderOpen,
  'globe': Globe,
  'hourglass': Hourglass,
  'image': Image,
  'lock': Lock,
  'network': Network,
  'qr-code': QrCode,
  'ruler': Ruler,
  'shield': Shield,
  'sparkles': Sparkles,
  'sun': Sun,
  'timer': Timer,
};

export const baseViews: Record<string, ToolViewLoader> = {
  'transform-text': () => import('./tools/TransformText.svelte'),
  'clipboard-history': () => import('./tools/ClipboardHistory.svelte'),
  'unit-converter': () => import('./tools/UnitConverter.svelte'),
  'break-time': () => import('./tools/BreakTime.svelte'),
  'timer-stopwatch': () => import('./tools/TimerStopwatch.svelte'),
  'world-time': () => import('./tools/WorldTime.svelte'),
  'verify-checksum': () => import('./tools/VerifyChecksum.svelte'),
  'find-duplicates': () => import('./tools/FindDuplicates.svelte'),
  'archive': () => import('./tools/Archive.svelte'),
  'clean-drive': () => import('./tools/CleanDrive.svelte'),
  'screenshot': () => import('./tools/Screenshot.svelte'),
  'resize-images': () => import('./tools/ResizeImages.svelte'),
  'qr-code': () => import('./tools/QrCode.svelte'),
  'system-monitor': () => import('./tools/SystemMonitor.svelte'),
  'network-usage': () => import('./tools/NetworkUsage.svelte'),
  'do-not-sleep': () => import('./tools/DoNotSleep.svelte'),
  'sleep-timer': () => import('./tools/SleepTimer.svelte'),
  'lock-screen': () => import('./tools/LockScreen.svelte'),
  'hide-desktop': () => import('./tools/HideDesktop.svelte'),
  'encrypt-files': () => import('./tools/EncryptFiles.svelte'),
};

// Presentation tokens belong to the legacy UI, rather than catalogue metadata.
const categoryTones: Record<CategoryId, string> = {
  productivity: 'var(--tone-blue)', time: 'var(--tone-teal)', files: 'var(--tone-amber)',
  capture: 'var(--tone-rose)', media: 'var(--tone-violet)', system: 'var(--tone-green)', privacy: 'var(--tone-red)',
};
const icons = { ...baseIcons, ...roadmapIcons };
function iconFor(name: string): LucideIcon {
  const icon = icons[name];
  if (!icon) throw new Error('Missing legacy icon adapter: ' + name);
  return icon;
}
export const categories: Category[] = canonical.categories.map(category => {
  const id = category.id as CategoryId;
  const tone = categoryTones[id];
  if (!tone) throw new Error('Missing legacy category presentation: ' + id);
  return { id, label: category.label, icon: iconFor(category.icon), tone };
});
export const categoryById = new Map(categories.map(category => [category.id, category]));
const views = { ...baseViews, ...roadmapViews };
const ids = new Set(canonical.tools.map(tool => tool.id));
if (ids.size !== canonical.tools.length) throw new Error('Duplicate canonical tool ID');
for (const id of Object.keys(views)) if (!ids.has(id)) throw new Error('Orphan legacy view adapter: ' + id);
export const tools: ToolDef[] = canonical.tools.map(tool => {
  const category = tool.category as CategoryId;
  if (!categoryById.has(category)) throw new Error('Unknown canonical category: ' + category);
  if (tool.status !== 'ready' && tool.status !== 'planned') throw new Error('Unknown canonical status: ' + tool.id);
  const platforms = tool.platforms.map(os => {
    if (!ALL_OS.includes(os as Os)) throw new Error('Unknown canonical platform: ' + os);
    return os as Os;
  });
  if (!platforms.length) throw new Error('Missing canonical platform: ' + tool.id);
  const load = views[tool.id];
  if (tool.status === 'ready' && !load) throw new Error('Missing ready legacy view adapter: ' + tool.id);
  return { id: tool.id, name: tool.name, description: tool.description, category,
    icon: iconFor(tool.icon), keywords: [...tool.keywords], platforms, phase: tool.phase, status: tool.status, load };
});
export const toolById = new Map(tools.map(tool => [tool.id, tool]));
