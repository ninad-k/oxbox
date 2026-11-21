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
/** Roadmap tools promoted to implementation by the user on 2026-11-21. */
import type { ToolDef } from './registry';
const all: ToolDef['platforms']=['windows','macos','linux'];
type Entry=Omit<ToolDef,'phase'|'status'>;
const entries:Entry[]=[
 {id:'mouse-activity',name:'Mouse Activity',description:'Gently move the pointer while idle and automatically yield when you return',category:'system',icon:MousePointer2,keywords:['mouse','idle','activity','pointer','move'],platforms:['windows'],load:()=>import('./tools/MouseActivity.svelte')},
 {id:'window-on-top',name:'Window on Top',description:'Keep a chosen window above other windows',category:'productivity',icon:Pin,keywords:['window','top','pin','always'],platforms:['windows'],load:()=>import('./tools/WindowOnTop.svelte')},
 {id:'window-manager',name:'Window Manager',description:'Place windows in halves, thirds or quarters of a display',category:'productivity',icon:PanelsTopLeft,keywords:['snap','window','layout','third','half'],platforms:['windows'],load:()=>import('./tools/WindowManager.svelte')},
 {id:'show-desktop',name:'Show Desktop',description:'Minimize application windows to reveal the desktop',category:'productivity',icon:Monitor,keywords:['desktop','minimize','windows'],platforms:['windows'],load:()=>import('./tools/ShowDesktop.svelte')},
 {id:'close-apps',name:'Close Apps',description:'Review open windows and request that selected apps close',category:'system',icon:AppWindow,keywords:['close','apps','windows'],platforms:['windows'],load:()=>import('./tools/CloseApps.svelte')},
 {id:'focus-window',name:'Focus on Window',description:'Dim the space around a chosen window',category:'productivity',icon:Focus,keywords:['focus','window','dim'],platforms:['windows'],load:()=>import('./tools/FocusWindow.svelte')},
 {id:'switch-resolution',name:'Switch Resolution',description:'Try a display mode with an automatic rollback countdown',category:'system',icon:Scan,keywords:['resolution','monitor','display','scale'],platforms:['windows'],load:()=>import('./tools/SwitchResolution.svelte')},
 {id:'eject-volumes',name:'Eject Volumes',description:'Review removable volumes and request a safe eject',category:'system',icon:Unplug,keywords:['eject','usb','volume','drive'],platforms:['windows'],load:()=>import('./tools/EjectVolumes.svelte')},
 {id:'energy-saver',name:'Energy Saver',description:'Choose an available operating-system power plan',category:'system',icon:Battery,keywords:['battery','power','energy','saver'],platforms:['windows'],load:()=>import('./tools/EnergySaver.svelte')},
 {id:'hidden-files',name:'Hidden Files',description:'Show or hide hidden files in the system file manager',category:'system',icon:FolderOpen,keywords:['hidden','files','explorer','finder'],platforms:['windows','macos'],load:()=>import('./tools/HiddenFiles.svelte')},
 {id:'cpu-temperature',name:'CPU Temperature',description:'Read temperature sensors exposed by the operating system',category:'system',icon:Thermometer,keywords:['temperature','sensor','cpu','heat'],platforms:all,load:()=>import('./tools/CpuTemperature.svelte')},
 {id:'launch-app-sets',name:'Launch App Sets',description:'Save and open a reviewed group of local apps or files',category:'productivity',icon:Sparkles,keywords:['launch','apps','sets','workspace'],platforms:all,load:()=>import('./tools/LaunchAppSets.svelte')},
 {id:'uninstall-apps',name:'Uninstall Apps',description:'Review registered uninstallers and separately inspect leftovers',category:'system',icon:PackageMinus,keywords:['uninstall','apps','remove','leftovers'],platforms:['windows'],load:()=>import('./tools/UninstallApps.svelte')},
 {id:'presentation-mode',name:'Presentation Mode',description:'Coordinate keep-awake and desktop visibility for a presentation',category:'productivity',icon:Presentation,keywords:['presentation','awake','desktop','external','display'],platforms:all,load:()=>import('./tools/PresentationMode.svelte')},
 {id:'do-not-disturb',name:'Do Not Disturb',description:'Open the supported system controls for notification interruptions',category:'system',icon:BellOff,keywords:['focus','notifications','disturb'],platforms:['windows','macos'],load:()=>import('./tools/DoNotDisturb.svelte')},
 {id:'airplane-mode',name:'Airplane Mode',description:'Open Windows controls for wireless radios and airplane mode',category:'system',icon:Plane,keywords:['airplane','wireless','radio','wifi'],platforms:['windows'],load:()=>import('./tools/AirplaneMode.svelte')},
 {id:'alarm',name:'Alarm',description:'Schedule local reminders with time-zone and recurrence controls',category:'time',icon:Bell,keywords:['alarm','schedule','reminder','daily','weekday'],platforms:all,load:()=>import('./tools/Alarm.svelte')},
 {id:'date-countdown',name:'Date Countdown',description:'Count down to dated events in explicit time zones',category:'time',icon:CalendarDays,keywords:['date','countdown','event','deadline'],platforms:all,load:()=>import('./tools/DateCountdown.svelte')},
 {id:'mute-microphone',name:'Mute Microphone',description:'Inspect and toggle the default input-device mute state',category:'media',icon:MicOff,keywords:['microphone','mute','audio','input'],platforms:['windows','linux'],load:()=>import('./tools/MuteMicrophone.svelte')},
 {id:'record-audio',name:'Record Audio',description:'Record an explicitly selected audio input to a new local file',category:'media',icon:Mic,keywords:['record','audio','microphone','wav'],platforms:all,load:()=>import('./tools/RecordAudio.svelte')},
 {id:'record-screen',name:'Record Screen',description:'Record a screen, area or window with explicit start and stop',category:'capture',icon:ScreenShare,keywords:['record','screen','area','window','video'],platforms:['windows','linux'],load:()=>import('./tools/RecordScreen.svelte')},
 {id:'make-gif',name:'Make GIF',description:'Create a local animated GIF from a video segment',category:'media',icon:Film,keywords:['gif','animation','video'],platforms:all,load:()=>import('./tools/MakeGif.svelte')},
 {id:'convert-audio-video',name:'Convert Audio / Video',description:'Convert local media with reviewed offline format presets',category:'media',icon:FileVideo,keywords:['convert','audio','video','mp4','mp3','wav','webm'],platforms:all,load:()=>import('./tools/ConvertMedia.svelte')},
 {id:'take-photo',name:'Take Photo',description:'Capture a photo from an explicitly selected camera',category:'capture',icon:Camera,keywords:['photo','camera','webcam'],platforms:all,load:()=>import('./tools/TakePhoto.svelte')},
 {id:'take-video',name:'Take Video',description:'Record a selected camera with visible controls and a duration limit',category:'capture',icon:Video,keywords:['video','camera','webcam','record'],platforms:all,load:()=>import('./tools/TakeVideo.svelte')},
 {id:'recognize-text',name:'Recognize Text',description:'Extract text from a local image using an available offline OCR engine',category:'productivity',icon:ScanText,keywords:['ocr','recognize','text','image'],platforms:all,load:()=>import('./tools/RecognizeText.svelte')},
 {id:'barcode-reader',name:'Barcode Reader',description:'Read QR codes and supported barcodes from a local image',category:'media',icon:ScanLine,keywords:['barcode','reader','qr','scan'],platforms:all,load:()=>import('./tools/BarcodeReader.svelte')},
 {id:'measure-color',name:'Measure Color',description:'Sample an image or screen pixel as HEX, RGB, HSL and CMYK',category:'media',icon:Pipette,keywords:['color','colour','picker','hex','rgb','hsl','cmyk'],platforms:all,load:()=>import('./tools/MeasureColor.svelte')},
 {id:'currency-converter',name:'Currency Converter',description:'Convert dated reference rates with offline import and opt-in ECB refresh',category:'productivity',icon:Coins,keywords:['currency','exchange','rates','ecb','money','convert'],platforms:all,load:()=>import('./tools/CurrencyConverter.svelte')},
];
export const roadmapTools:ToolDef[]=entries.map(tool=>({...tool,phase:5,status:'ready'}));
