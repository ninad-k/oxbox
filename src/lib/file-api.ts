/** Typed file-tool IPC. Re-exported by the public api facade. */
import { call } from './ipc';
import type { HashAlgorithm } from './api';

export interface DuplicateFile { path:string; modifiedMs:number }
export interface DuplicateGroup { size:number; files:DuplicateFile[] }
export interface DuplicateResult { groups:DuplicateGroup[]; wastedBytes:number; scannedFiles:number }
export interface CleanCategory { id:string; name:string; path:string; bytes:number; files:number; skipped:number; readOnly:boolean }
export type ArchiveFormat = 'zip'|'tar-gz';
export const fileApi = {
  checksumFileJob: (jobId:string,path:string,algorithm:HashAlgorithm)=>call<string>('checksum_file_job',{jobId,path,algorithm}),
  findDuplicates: (jobId:string,roots:string[],includeHidden:boolean)=>call<DuplicateResult>('find_duplicates',{jobId,roots,includeHidden}),
  trashFiles: (paths:string[],confirmed:boolean)=>call<number>('trash_files',{paths,confirmed}),
  createArchive: (jobId:string,sources:string[],output:string,format:ArchiveFormat)=>call<string>('create_archive',{jobId,sources,output,format}),
  extractArchive: (jobId:string,input:string,output:string,maxBytes:number)=>call<string>('extract_archive',{jobId,input,output,maxBytes}),
  scanCleanDrive: (jobId:string)=>call<CleanCategory[]>('scan_clean_drive',{jobId}),
  cleanDrive: (jobId:string,ids:string[],maxFiles:number,maxBytes:number,confirmed:boolean)=>call<number>('clean_drive',{jobId,ids,maxFiles,maxBytes,confirmed}),
};

export function formatBytes(bytes:number):string {
  if (!Number.isFinite(bytes) || bytes<=0) return '0 B';
  const units=['B','KiB','MiB','GiB','TiB'];
  const exponent=Math.min(Math.floor(Math.log(bytes)/Math.log(1024)),units.length-1);
  return `${(bytes/1024**exponent).toFixed(exponent===0?0:1)} ${units[exponent]}`;
}
