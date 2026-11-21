import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { call, inDesktopApp } from './ipc';
import { Clock } from './time';
export class Clocks {
  timer = new Clock(5*60000);
  stopwatch = new Clock();
  now = $state(0);
  revision = $state(0);
  laps = $state<number[]>([]);
  notice = $state('');
  error = $state('');
  busy = $state(false);
  private ticks?: ReturnType<typeof setInterval>;
  private releaseBackgroundLock?: () => void;
  private backgroundLockPending = false;
  private nativeNotification = false;
  private notificationGeneration = 0;
  private lifecycle = 0;
  private alarmWrites: Promise<void> = Promise.resolve();
  private queueNotification(milliseconds: (() => number) | null): Promise<void> {
    if (!inDesktopApp()) return Promise.resolve();
    const write = this.alarmWrites.catch(() => {}).then(() => call<void>('timer_notification_set', { milliseconds: milliseconds ? Math.max(0, Math.ceil(milliseconds())) : null }));
    this.alarmWrites = write;
    return write;
  }
  private notifyDesktop() {
    if (inDesktopApp()) void isPermissionGranted().then(granted => { if (granted) sendNotification({ title: 'Oxbox timer', body: 'Your countdown is complete.' }); }).catch(e => { this.error = String(e); });
  }
  private keepTimersRunning() {
    const active = this.timer.running || this.stopwatch.running;
    if (!active) { this.releaseBackgroundLock?.(); this.releaseBackgroundLock = undefined; return; }
    if (this.releaseBackgroundLock || this.backgroundLockPending || !globalThis.navigator?.locks) return;
    this.backgroundLockPending = true;
    // A pending WebLock discourages hidden Chromium WebViews from being suspended.
    void navigator.locks.request('oxbox-running-clock', async () => {
      this.backgroundLockPending = false;
      if (!this.ticks || (!this.timer.running && !this.stopwatch.running)) return;
      await new Promise<void>(resolve => { this.releaseBackgroundLock = resolve; });
    }).catch(() => { this.backgroundLockPending = false; });
  }
  startTicks() {
    if (this.ticks) return;
    const lifecycle = ++this.lifecycle;
    const generation = ++this.notificationGeneration;
    this.now=performance.now();
    this.nativeNotification = inDesktopApp() && this.timer.running;
    void this.queueNotification(this.timer.running ? () => this.timer.remaining(performance.now()) : null).catch(e => { if(generation===this.notificationGeneration){this.nativeNotification = false; this.error = String(e);} });
    this.ticks=setInterval(() => {
      this.now=performance.now();
      if (this.timer.finish(this.now)) {
        this.revision++; this.notice='Timer finished.';
        if (!this.nativeNotification) this.notifyDesktop();
      }
      this.keepTimersRunning();
    },100);
    return () => {if(lifecycle!==this.lifecycle)return;this.lifecycle++;this.notificationGeneration++;this.nativeNotification=false;clearInterval(this.ticks);this.ticks=undefined;this.releaseBackgroundLock?.();this.releaseBackgroundLock=undefined;void this.queueNotification(null).catch(() => {});};
  }
  async toggleTimer() {
    if (this.busy) return;
    const lifecycle = this.lifecycle;
    const generation = ++this.notificationGeneration;
    this.busy=true;
    this.error='';
    try {
      if (this.timer.running) {
        this.timer.pause(performance.now());
        this.nativeNotification = false;
        await this.queueNotification(null);
      }
      else {
      if (inDesktopApp()) {
        try {if (!await isPermissionGranted() && await requestPermission()!=='granted') this.notice='Notifications are disabled. The countdown will still finish in Oxbox.';}
        catch(e){this.error=String(e);}
      }
        if(lifecycle!==this.lifecycle||!this.ticks)return;
        if(this.timer.completed)this.timer.reset();
        this.timer.start(performance.now());
        this.nativeNotification = inDesktopApp();
        try { await this.queueNotification(() => this.timer.remaining(performance.now())); }
        catch (e) { if(generation===this.notificationGeneration){this.nativeNotification = false; this.error = `Background notification could not be armed: ${String(e)}`; if (this.timer.completed) this.notifyDesktop();} }
      }
      this.revision++;
      this.keepTimersRunning();
    } catch (e) { this.error=String(e); }
    finally {
      this.busy=false;
    }
  }
  async resetTimer(duration = this.timer.duration) {
    if (this.busy) return;
    this.busy = true;
    this.notificationGeneration++;
    this.timer.reset(); this.timer.duration = duration;
    this.nativeNotification = false; this.notice = ''; this.revision++;
    this.keepTimersRunning();
    try { await this.queueNotification(null); this.error = ''; }
    catch (e) { this.error = `Could not cancel the background notification: ${String(e)}`; }
    finally { this.busy = false; }
  }
}
export const clocks=new Clocks();
