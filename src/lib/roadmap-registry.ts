/** Legacy Svelte adapters only. Metadata lives in catalog/tools.json. */
import type { LucideIcon } from '@lucide/svelte';
import type { ToolViewLoader } from './registry';
import Pin from '@lucide/svelte/icons/pin';
import PanelsTopLeft from '@lucide/svelte/icons/panels-top-left';
import Monitor from '@lucide/svelte/icons/monitor';
import AppWindow from '@lucide/svelte/icons/app-window';
import Focus from '@lucide/svelte/icons/focus';
import Scan from '@lucide/svelte/icons/scan';
import Unplug from '@lucide/svelte/icons/unplug';
import Battery from '@lucide/svelte/icons/battery';
import FolderOpen from '@lucide/svelte/icons/folder-open';
import Thermometer from '@lucide/svelte/icons/thermometer';
import PackageMinus from '@lucide/svelte/icons/package-minus';
import Presentation from '@lucide/svelte/icons/presentation';
import BellOff from '@lucide/svelte/icons/bell-off';
import Plane from '@lucide/svelte/icons/plane';
import Bell from '@lucide/svelte/icons/bell';
import CalendarDays from '@lucide/svelte/icons/calendar-days';
import MicOff from '@lucide/svelte/icons/mic-off';
import Mic from '@lucide/svelte/icons/mic';
import ScreenShare from '@lucide/svelte/icons/screen-share';
import Film from '@lucide/svelte/icons/film';
import FileVideo from '@lucide/svelte/icons/file-video';
import Camera from '@lucide/svelte/icons/camera';
import Video from '@lucide/svelte/icons/video';
import ScanText from '@lucide/svelte/icons/scan-text';
import ScanLine from '@lucide/svelte/icons/scan-line';
import Pipette from '@lucide/svelte/icons/pipette';
import Coins from '@lucide/svelte/icons/coins';
import Sparkles from '@lucide/svelte/icons/sparkles';
import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';

export const roadmapIcons: Record<string, LucideIcon> = {
  'pin': Pin,
  'panels-top-left': PanelsTopLeft,
  'monitor': Monitor,
  'app-window': AppWindow,
  'focus': Focus,
  'scan': Scan,
  'unplug': Unplug,
  'battery': Battery,
  'folder-open': FolderOpen,
  'thermometer': Thermometer,
  'package-minus': PackageMinus,
  'presentation': Presentation,
  'bell-off': BellOff,
  'plane': Plane,
  'bell': Bell,
  'calendar-days': CalendarDays,
  'mic-off': MicOff,
  'mic': Mic,
  'screen-share': ScreenShare,
  'film': Film,
  'file-video': FileVideo,
  'camera': Camera,
  'video': Video,
  'scan-text': ScanText,
  'scan-line': ScanLine,
  'pipette': Pipette,
  'coins': Coins,
  'sparkles': Sparkles,
  'mouse-pointer-2': MousePointer2,
};

export const roadmapViews: Record<string, ToolViewLoader> = {
  'mouse-activity': ()=>import('./tools/MouseActivity.svelte'),
  'window-on-top': ()=>import('./tools/WindowOnTop.svelte'),
  'window-manager': ()=>import('./tools/WindowManager.svelte'),
  'show-desktop': ()=>import('./tools/ShowDesktop.svelte'),
  'close-apps': ()=>import('./tools/CloseApps.svelte'),
  'focus-window': ()=>import('./tools/FocusWindow.svelte'),
  'switch-resolution': ()=>import('./tools/SwitchResolution.svelte'),
  'eject-volumes': ()=>import('./tools/EjectVolumes.svelte'),
  'energy-saver': ()=>import('./tools/EnergySaver.svelte'),
  'hidden-files': ()=>import('./tools/HiddenFiles.svelte'),
  'cpu-temperature': ()=>import('./tools/CpuTemperature.svelte'),
  'launch-app-sets': ()=>import('./tools/LaunchAppSets.svelte'),
  'uninstall-apps': ()=>import('./tools/UninstallApps.svelte'),
  'presentation-mode': ()=>import('./tools/PresentationMode.svelte'),
  'do-not-disturb': ()=>import('./tools/DoNotDisturb.svelte'),
  'airplane-mode': ()=>import('./tools/AirplaneMode.svelte'),
  'alarm': ()=>import('./tools/Alarm.svelte'),
  'date-countdown': ()=>import('./tools/DateCountdown.svelte'),
  'mute-microphone': ()=>import('./tools/MuteMicrophone.svelte'),
  'record-audio': ()=>import('./tools/RecordAudio.svelte'),
  'record-screen': ()=>import('./tools/RecordScreen.svelte'),
  'make-gif': ()=>import('./tools/MakeGif.svelte'),
  'convert-audio-video': ()=>import('./tools/ConvertMedia.svelte'),
  'take-photo': ()=>import('./tools/TakePhoto.svelte'),
  'take-video': ()=>import('./tools/TakeVideo.svelte'),
  'recognize-text': ()=>import('./tools/RecognizeText.svelte'),
  'barcode-reader': ()=>import('./tools/BarcodeReader.svelte'),
  'measure-color': ()=>import('./tools/MeasureColor.svelte'),
  'currency-converter': ()=>import('./tools/CurrencyConverter.svelte'),
};
