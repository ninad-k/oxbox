import { listen } from '@tauri-apps/api/event';
import { call, inDesktopApp, ToolError } from './ipc';

export interface JobProgress { jobId:string; done:number; total:number; message:string }
export interface JobHandle {
  id:string;
  run<T>(invoke:(jobId:string)=>Promise<T>):Promise<T>;
  cancel():Promise<void>;
}

/** Listen before invoking so even the first progress event is observable. */
export function createJob(toolId:string,onProgress:(progress:JobProgress)=>void):JobHandle {
  const id=crypto.randomUUID();
  let used=false;
  let cancelled=false;
  return {
    id,
    async run<T>(invoke:(jobId:string)=>Promise<T>):Promise<T> {
      if (used) throw new ToolError('invalid_input','A job can only be started once.');
      used=true;
      if (!inDesktopApp()) throw new ToolError('not_desktop','Open Oxbox as the desktop app to use this tool.');
      if (cancelled) throw new ToolError('cancelled','Cancelled');
      const unlisten=await listen<JobProgress>(`tool://${toolId}/progress`,(event)=>{
        if (event.payload.jobId===id && !cancelled) onProgress(event.payload);
      });
      try {
        if (cancelled) throw new ToolError('cancelled','Cancelled');
        return await invoke(id);
      } finally {unlisten();}
    },
    async cancel() {
      cancelled=true;
      if (inDesktopApp()) await call<void>('cancel_job',{jobId:id});
    },
  };
}
