/** Elapsed time derives from a monotonic timestamp, so delayed ticks never lose time. */
export class Clock {
  startedAt: number | null = null;
  accumulated = 0;
  completed = false;
  constructor(public duration = 0) {}
  get running() { return this.startedAt !== null; }
  elapsed(now: number) {return this.accumulated + (this.startedAt === null ? 0 : Math.max(0, now - this.startedAt));}
  remaining(now: number) {return Math.max(0, this.duration - this.elapsed(now));}
  start(now: number) { if (!this.running && !this.completed) this.startedAt = now; }
  pause(now: number) {this.accumulated = this.elapsed(now); this.startedAt = null;}
  reset() {this.startedAt = null; this.accumulated = 0; this.completed = false;}
  finish(now: number) {
    if (!this.duration || !this.running || this.completed || this.remaining(now)>0) return false;
    this.accumulated = this.duration; this.startedAt = null; this.completed = true; return true;
  }
}
export function formatTime(ms: number) {
  const seconds = Math.max(0, Math.floor(ms/1000));
  return [Math.floor(seconds/3600), Math.floor(seconds/60)%60, seconds%60].map(v => String(v).padStart(2,'0')).join(':');
}
