import { call } from './ipc';
export interface ControlCapability {available:boolean;automaticToggle:boolean;state:'unknown';settingsUri:string|null;message:string;actionLabel:string}
export interface PresentationOptions {display:boolean;hideDesktop:boolean;autoExternal:boolean}
export interface PresentationStatus {active:boolean;automatic:boolean;options:PresentationOptions;autoSuppressed:boolean;notificationControl:'manual';error:string|null}
export type Ambiguity='earlier'|'later';
export type Recurrence='once'|'daily'|'weekdays';
export interface AlarmInput {label:string;localDateTime:string;zone:string;recurrence:Recurrence;ambiguity:Ambiguity}
export interface Alarm {id:number;input:AlarmInput;nextAt:number;enabled:boolean;ringing:boolean;lastFired:number|null}
export interface CountdownInput {label:string;localDateTime:string;zone:string;notify:boolean;ambiguity:Ambiguity}
export interface Countdown {id:number;input:CountdownInput;target:number;notified:boolean}
export interface CountdownSnapshot extends Countdown {remainingSeconds:number}
export interface TimeSnapshot {alarms:Alarm[];countdowns:CountdownSnapshot[];error:string|null}
export const roadmapTimeApi={
  dndCapability:()=>call<ControlCapability>('dnd_capability'),
  dndOpenSettings:()=>call<void>('dnd_open_settings'),
  airplaneCapability:()=>call<ControlCapability>('airplane_capability'),
  airplaneOpenSettings:()=>call<void>('airplane_open_settings'),
  presentationStatus:()=>call<PresentationStatus>('presentation_status'),
  presentationConfigure:(options:PresentationOptions)=>call<PresentationStatus>('presentation_configure',{options}),
  presentationStart:(options:PresentationOptions)=>call<PresentationStatus>('presentation_start',{options}),
  presentationStop:()=>call<PresentationStatus>('presentation_stop'),
  roadmapTimeSnapshot:()=>call<TimeSnapshot>('roadmap_time_snapshot'),
  alarmAdd:(input:AlarmInput)=>call<number>('alarm_add',{input}),
  alarmRemove:(id:number)=>call<void>('alarm_remove',{id}),
  alarmSnooze:(id:number)=>call<void>('alarm_snooze',{id,minutes:5}),
  alarmDismiss:(id:number)=>call<void>('alarm_dismiss',{id}),
  alarmSetEnabled:(id:number,enabled:boolean)=>call<void>('alarm_set_enabled',{id,enabled}),
  countdownAdd:(input:CountdownInput)=>call<number>('countdown_add',{input}),
  countdownRemove:(id:number)=>call<void>('countdown_remove',{id}),
  roadmapTimeZones:()=>call<string[]>('time_zones'),
};
export function countdownText(seconds:number):string {
  const past=seconds<0;const absolute=Math.abs(Math.trunc(seconds));
  const days=Math.floor(absolute/86400),hours=Math.floor(absolute%86400/3600),minutes=Math.floor(absolute%3600/60),rest=absolute%60;
  return `${days}d ${hours}h ${minutes}m ${rest}s${past?' ago':' remaining'}`;
}
