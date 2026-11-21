/** Typed Phase 5 desktop and application APIs. No view invokes Rust directly. */
import { call } from './ipc';
export interface DesktopRect {x:number;y:number;width:number;height:number}
export interface DesktopWindow{id:number;pid:number;title:string;bounds:DesktopRect;topmost:boolean;minimized:boolean}
export type SnapPlacement='left-half'|'right-half'|'top-half'|'bottom-half'|'left-third'|'center-third'|'right-third'|'top-left'|'top-right'|'bottom-left'|'bottom-right'|'full';
export interface DisplayMode{device:string;width:number;height:number;refresh:number;bitsPerPixel:number}
export interface DesktopDisplay{device:string;name:string;current:DisplayMode;modes:DisplayMode[]}
export interface ResolutionStatus{pending:{mode:DisplayMode;deadline:number;remainingSeconds:number}|null;error:string|null;kept:boolean|null}
export interface RemovableVolume{root:string;label:string}
export interface PowerProfile{id:string;name:string;active:boolean}
export interface TemperatureSensor{label:string;temperature:number;critical:number|null}
export interface LaunchEntry{kind:'application'|'file';target:string;args:string[]}
export interface AppSet{id:string;name:string;entries:LaunchEntry[]}
export interface InstalledApp{id:string;name:string;publisher:string;version:string;command:string;installLocation:string}
export interface AppLeftover{path:string;bytes:number;files:number}
export interface FocusFrame{cutout:{x:number;y:number;width:number;height:number}|null;opacity:number}
export const roadmapSystemApi={
  windows:()=>call<DesktopWindow[]>('roadmap_windows'),
  windowTop:(id:number,pid:number,enabled:boolean)=>call<void>('roadmap_window_top',{id,pid,enabled}),
  snap:(id:number,pid:number,placement:SnapPlacement)=>call<DesktopRect>('roadmap_snap',{id,pid,placement}),
  showDesktop:(restore:boolean)=>call<number>('roadmap_show_desktop',{restore}),
  closeApps:(targets:{id:number;pid:number}[],confirmed:boolean)=>call<number>('roadmap_close_apps',{targets,confirmed}),
  displays:()=>call<DesktopDisplay[]>('roadmap_displays'),
  resolutionStatus:()=>call<ResolutionStatus>('roadmap_resolution_status'),
  resolutionApply:(mode:DisplayMode,confirmed:boolean)=>call<ResolutionStatus>('roadmap_resolution_apply',{mode,confirmed}),
  resolutionFinish:(keep:boolean)=>call<ResolutionStatus>('roadmap_resolution_finish',{keep}),
  volumes:()=>call<RemovableVolume[]>('roadmap_volumes'),
  eject:(root:string,confirmed:boolean)=>call<void>('roadmap_eject',{root,confirmed}),
  powerProfiles:()=>call<PowerProfile[]>('roadmap_power_profiles'),
  powerSet:(id:string,confirmed:boolean)=>call<void>('roadmap_power_set',{id,confirmed}),
  hiddenFiles:(show:boolean|null=null,confirmed=false)=>call<boolean>('roadmap_hidden_files',{show,confirmed}),
  cpuTemperatures:()=>call<TemperatureSensor[]>('roadmap_cpu_temperatures'),
  launchSets:()=>call<AppSet[]>('roadmap_launch_sets'),
  launchSetsSave:(sets:AppSet[])=>call<void>('roadmap_launch_sets_save',{sets}),
  launchSet:(id:string)=>call<number>('roadmap_launch_set',{id}),
  installedApps:()=>call<InstalledApp[]>('roadmap_installed_apps'),
  uninstall:(id:string,reviewedCommand:string,confirmed:boolean)=>call<number>('roadmap_uninstall',{id,reviewedCommand,confirmed}),
  leftovers:(jobId:string,name:string,publisher:string)=>call<AppLeftover[]>('roadmap_leftovers',{jobId,name,publisher}),
  focusStart:(id:number,pid:number,opacity:number)=>call<void>('roadmap_focus_start',{id,pid,opacity}),
  focusStop:()=>call<void>('roadmap_focus_stop'),
  focusStatus:()=>call<boolean>('roadmap_focus_status'),
  focusFrame:()=>call<FocusFrame>('roadmap_focus_frame'),
};
