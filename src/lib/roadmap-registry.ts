/** Roadmap tools promoted to implementation by the user on 2026-11-21. */
import type { ToolDef } from './registry';
import Activity from '@lucide/svelte/icons/activity';
import Camera from '@lucide/svelte/icons/camera';
import Clock from '@lucide/svelte/icons/clock';
import Cpu from '@lucide/svelte/icons/cpu';
import EyeOff from '@lucide/svelte/icons/eye-off';
import Globe from '@lucide/svelte/icons/globe';
import Image from '@lucide/svelte/icons/image';
import Network from '@lucide/svelte/icons/network';
import Shield from '@lucide/svelte/icons/shield';
import Sparkles from '@lucide/svelte/icons/sparkles';
const all: ToolDef['platforms']=['windows','macos','linux'];
type Entry=Omit<ToolDef,'phase'|'status'>;
const entries:Entry[]=[
 {id:'window-on-top',name:'Window on Top',description:'Keep a chosen window above other windows',category:'productivity',icon:Sparkles,keywords:['window','top','pin','always'],platforms:['windows'],load:()=>import('./tools/WindowOnTop.svelte')},
 {id:'window-manager',name:'Window Manager',description:'Place windows in halves, thirds or quarters of a display',category:'productivity',icon:Sparkles,keywords:['snap','window','layout','third','half'],platforms:['windows'],load:()=>import('./tools/WindowManager.svelte')},
 {id:'show-desktop',name:'Show Desktop',description:'Minimize application windows to reveal the desktop',category:'productivity',icon:EyeOff,keywords:['desktop','minimize','windows'],platforms:['windows'],load:()=>import('./tools/ShowDesktop.svelte')},
 {id:'close-apps',name:'Close Apps',description:'Review open windows and request that selected apps close',category:'system',icon:Cpu,keywords:['close','apps','windows'],platforms:['windows'],load:()=>import('./tools/CloseApps.svelte')},
 {id:'focus-window',name:'Focus on Window',description:'Dim the space around a chosen window',category:'productivity',icon:EyeOff,keywords:['focus','window','dim'],platforms:['windows'],load:()=>import('./tools/FocusWindow.svelte')},
 {id:'switch-resolution',name:'Switch Resolution',description:'Try a display mode with an automatic rollback countdown',category:'system',icon:Cpu,keywords:['resolution','monitor','display','scale'],platforms:['windows'],load:()=>import('./tools/SwitchResolution.svelte')},
 {id:'eject-volumes',name:'Eject Volumes',description:'Review removable volumes and request a safe eject',category:'system',icon:Cpu,keywords:['eject','usb','volume','drive'],platforms:['windows'],load:()=>import('./tools/EjectVolumes.svelte')},
 {id:'energy-saver',name:'Energy Saver',description:'Choose an available operating-system power plan',category:'system',icon:Activity,keywords:['battery','power','energy','saver'],platforms:['windows'],load:()=>import('./tools/EnergySaver.svelte')},
 {id:'hidden-files',name:'Hidden Files',description:'Show or hide hidden files in the system file manager',category:'system',icon:EyeOff,keywords:['hidden','files','explorer','finder'],platforms:['windows','macos'],load:()=>import('./tools/HiddenFiles.svelte')},
 {id:'cpu-temperature',name:'CPU Temperature',description:'Read temperature sensors exposed by the operating system',category:'system',icon:Activity,keywords:['temperature','sensor','cpu','heat'],platforms:all,load:()=>import('./tools/CpuTemperature.svelte')},
 {id:'launch-app-sets',name:'Launch App Sets',description:'Save and open a reviewed group of local apps or files',category:'productivity',icon:Sparkles,keywords:['launch','apps','sets','workspace'],platforms:all,load:()=>import('./tools/LaunchAppSets.svelte')},
 {id:'uninstall-apps',name:'Uninstall Apps',description:'Review registered uninstallers and separately inspect leftovers',category:'system',icon:Cpu,keywords:['uninstall','apps','remove','leftovers'],platforms:['windows'],load:()=>import('./tools/UninstallApps.svelte')},
 {id:'presentation-mode',name:'Presentation Mode',description:'Coordinate keep-awake and desktop visibility for a presentation',category:'productivity',icon:Sparkles,keywords:['presentation','awake','desktop','external','display'],platforms:all,load:()=>import('./tools/PresentationMode.svelte')},
 {id:'do-not-disturb',name:'Do Not Disturb',description:'Open the supported system controls for notification interruptions',category:'system',icon:Shield,keywords:['focus','notifications','disturb'],platforms:['windows','macos'],load:()=>import('./tools/DoNotDisturb.svelte')},
 {id:'airplane-mode',name:'Airplane Mode',description:'Open Windows controls for wireless radios and airplane mode',category:'system',icon:Network,keywords:['airplane','wireless','radio','wifi'],platforms:['windows'],load:()=>import('./tools/AirplaneMode.svelte')},
 {id:'alarm',name:'Alarm',description:'Schedule local reminders with time-zone and recurrence controls',category:'time',icon:Clock,keywords:['alarm','schedule','reminder','daily','weekday'],platforms:all,load:()=>import('./tools/Alarm.svelte')},
 {id:'date-countdown',name:'Date Countdown',description:'Count down to dated events in explicit time zones',category:'time',icon:Clock,keywords:['date','countdown','event','deadline'],platforms:all,load:()=>import('./tools/DateCountdown.svelte')},
 {id:'mute-microphone',name:'Mute Microphone',description:'Inspect and toggle the default input-device mute state',category:'media',icon:Image,keywords:['microphone','mute','audio','input'],platforms:['windows','linux'],load:()=>import('./tools/MuteMicrophone.svelte')},
 {id:'record-audio',name:'Record Audio',description:'Record an explicitly selected audio input to a new local file',category:'media',icon:Image,keywords:['record','audio','microphone','wav'],platforms:all,load:()=>import('./tools/RecordAudio.svelte')},
 {id:'record-screen',name:'Record Screen',description:'Record a screen, area or window with explicit start and stop',category:'capture',icon:Camera,keywords:['record','screen','area','window','video'],platforms:['windows','linux'],load:()=>import('./tools/RecordScreen.svelte')},
 {id:'make-gif',name:'Make GIF',description:'Create a local animated GIF from a video segment',category:'media',icon:Image,keywords:['gif','animation','video'],platforms:all,load:()=>import('./tools/MakeGif.svelte')},
 {id:'convert-audio-video',name:'Convert Audio / Video',description:'Convert local media with reviewed offline format presets',category:'media',icon:Image,keywords:['convert','audio','video','mp4','mp3','wav','webm'],platforms:all,load:()=>import('./tools/ConvertMedia.svelte')},
 {id:'take-photo',name:'Take Photo',description:'Capture a photo from an explicitly selected camera',category:'capture',icon:Camera,keywords:['photo','camera','webcam'],platforms:all,load:()=>import('./tools/TakePhoto.svelte')},
 {id:'take-video',name:'Take Video',description:'Record a selected camera with visible controls and a duration limit',category:'capture',icon:Camera,keywords:['video','camera','webcam','record'],platforms:all,load:()=>import('./tools/TakeVideo.svelte')},
 {id:'recognize-text',name:'Recognize Text',description:'Extract text from a local image using an available offline OCR engine',category:'productivity',icon:Sparkles,keywords:['ocr','recognize','text','image'],platforms:all,load:()=>import('./tools/RecognizeText.svelte')},
 {id:'barcode-reader',name:'Barcode Reader',description:'Read QR codes and supported barcodes from a local image',category:'media',icon:Image,keywords:['barcode','reader','qr','scan'],platforms:all,load:()=>import('./tools/BarcodeReader.svelte')},
 {id:'measure-color',name:'Measure Color',description:'Sample an image or screen pixel as HEX, RGB, HSL and CMYK',category:'media',icon:Image,keywords:['color','colour','picker','hex','rgb','hsl','cmyk'],platforms:all,load:()=>import('./tools/MeasureColor.svelte')},
 {id:'currency-converter',name:'Currency Converter',description:'Convert dated reference rates with offline import and opt-in ECB refresh',category:'productivity',icon:Globe,keywords:['currency','exchange','rates','ecb','money','convert'],platforms:all,load:()=>import('./tools/CurrencyConverter.svelte')},
];
export const roadmapTools:ToolDef[]=entries.map(tool=>({...tool,phase:5,status:'ready'}));
