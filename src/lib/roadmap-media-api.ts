/** Typed, local-only Phase 5 media and vision bridge. */
import { call } from './ipc';
export interface MediaProvider { name:string; path:string|null; version:string|null; error:string|null }
export interface RecordingStatus { active:boolean; id:string; kind:string; output:string|null; startedAt:number; error:string|null }
export type ScreenSource={kind:'desktop'}|{kind:'area';x:number;y:number;width:number;height:number}|{kind:'window';title:string};
export interface ScreenRequest {source:ScreenSource;fps:number;seconds:number}
export type MediaFormat='mp4'|'webm'|'mp3'|'wav'|'flac';
export interface GifOptions{startSeconds:number;seconds:number;fps:number;width:number}
export interface BarcodeResult{text:string;format:string}
export interface ColorValue{hex:string;rgb:[number,number,number];hsl:[number,number,number];cmyk:[number,number,number,number]}
export const roadmapMediaApi={
  mediaProviders:()=>call<MediaProvider[]>('roadmap_media_providers'),
  microphoneStatus:()=>call<boolean>('roadmap_microphone_status'),
  microphoneSet:(muted:boolean)=>call<boolean>('roadmap_microphone_set',{muted}),
  recordingStatus:()=>call<RecordingStatus>('roadmap_recording_status'),
  recordingStop:(id:string)=>call<RecordingStatus>('roadmap_recording_stop',{id}),
  recordAudioStart:(device:string,seconds:number,output:string)=>call<RecordingStatus>('roadmap_record_audio_start',{device,seconds,output,consent:true}),
  recordScreenStart:(request:ScreenRequest,output:string)=>call<RecordingStatus>('roadmap_record_screen_start',{request,output,consent:true}),
  takeVideoStart:(device:string,seconds:number,output:string)=>call<RecordingStatus>('roadmap_take_video_start',{device,seconds,output,consent:true}),
  convertMedia:(jobId:string,input:string,output:string,format:MediaFormat)=>call<string>('roadmap_convert_media',{jobId,input,output,format}),
  makeGif:(jobId:string,input:string,output:string,options:GifOptions)=>call<string>('roadmap_make_gif',{jobId,input,output,options}),
  takePhoto:(jobId:string,device:string,output:string)=>call<string>('roadmap_take_photo',{jobId,device,output,consent:true}),
  recognizeText:(jobId:string,path:string,provider:'native'|'tesseract',language:string)=>call<string>('roadmap_recognize_text',{jobId,path,provider,language}),
  readBarcode:(path:string)=>call<BarcodeResult[]>('roadmap_read_barcode',{path}),
  colorSample:(id:number,x:number,y:number)=>call<ColorValue>('roadmap_color_sample',{id,x,y}),
  colorCursor:()=>call<ColorValue>('roadmap_color_cursor'),
  colorTargets:()=>call<{kind:'monitor';id:number;name:string;width:number;height:number}[]>('roadmap_color_targets'),
};
