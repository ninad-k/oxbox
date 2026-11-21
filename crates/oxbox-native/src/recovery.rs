//! Independent native tray recovery dispatch. It does not require egui to poll.
use std::{
    collections::VecDeque,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};
const CAPACITY: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    StopMouse,
    CancelSleep,
    StopAwake,
    StopPresentation,
    StopRecording,
    ToggleAwake,
    Lock,
    ToggleDesktop,
    Quit,
    QuitMedia,
}
impl Action {
    fn critical(self) -> bool {
        matches!(
            self,
            Self::StopMouse
                | Self::CancelSleep
                | Self::StopAwake
                | Self::StopPresentation
                | Self::StopRecording
                | Self::Quit
                | Self::QuitMedia
        )
    }
    fn quit(self) -> bool {
        matches!(self, Self::Quit | Self::QuitMedia)
    }
}
pub enum Failure {
    Cancelled,
    Error(String),
}
#[derive(Clone)]
struct Request {
    action: Action,
    generation: u64,
}
#[derive(Default)]
struct Pending {
    items: VecDeque<Request>,
    closed: bool,
}
#[derive(Default)]
struct Lane {
    state: Mutex<Pending>,
    wake: Condvar,
}
impl Lane {
    fn push(&self, request: Request) -> Result<(), String> {
        let mut pending = self
            .state
            .lock()
            .map_err(|_| "Tray recovery queue is unavailable.")?;
        if pending.closed {
            return Err("Oxbox is closing.".into());
        }
        if request.action.quit() {
            pending.items.clear();
            pending.closed = true;
        } else if request.action.critical() {
            // There are only five distinct Stop/Cancel actions, all coalesced.
            // Thus a full critical queue cannot occur with the 16-entry bound.
            pending.items.retain(|old| {
                old.action != request.action
                    && !(request.action == Action::StopAwake && old.action == Action::ToggleAwake)
            });
            if pending.items.len() >= CAPACITY
                && let Some(index) = pending.items.iter().position(|old| !old.action.critical())
            {
                pending.items.remove(index);
            }
        }
        if pending.items.len() >= CAPACITY {
            return Err(
                "The tray action queue is busy. Retry after the current action completes.".into(),
            );
        }
        pending.items.push_back(request);
        self.wake.notify_one();
        Ok(())
    }
    fn next(&self) -> Option<Request> {
        let mut pending = self.state.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if let Some(request) = pending.items.pop_front() {
                return Some(request);
            }
            if pending.closed {
                return None;
            }
            pending = self.wake.wait(pending).unwrap_or_else(|p| p.into_inner());
        }
    }
}
struct Reports {
    errors: Mutex<VecDeque<String>>,
    wake: Box<dyn Fn() + Send + Sync>,
    notify: Box<dyn Fn(String) + Send + Sync>,
}
impl Reports {
    fn error(&self, message: String) {
        let mut errors = self.errors.lock().unwrap_or_else(|p| p.into_inner());
        if errors.len() == CAPACITY {
            errors.pop_front();
        }
        errors.push_back(message.clone());
        drop(errors);
        // Reporting failures must not kill the recovery executor either.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.notify)(message)));
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.wake)()));
    }
}
#[derive(Clone)]
pub struct Handle {
    system: Arc<Lane>,
    media: Arc<Lane>,
    quitting: Arc<AtomicBool>,
    prepare: Arc<dyn Fn(Action) -> u64 + Send + Sync>,
    reports: Arc<Reports>,
}
impl Handle {
    pub fn request(&self, action: Action) -> Result<(), String> {
        if action == Action::QuitMedia {
            return Err("Media Quit is an internal recovery action.".into());
        }
        if action == Action::Quit {
            if self.quitting.swap(true, Ordering::AcqRel) {
                return Ok(());
            }
        } else if self.quitting.load(Ordering::Acquire) {
            return Err("Oxbox is closing.".into());
        }
        // Immediate cancellation/admission closure happens in the native callback,
        // before waiting for a worker or egui's next frame.
        let generation = (self.prepare)(action);
        let request = Request { action, generation };
        let result = if action == Action::StopRecording {
            self.media.push(request)
        } else {
            self.system.push(request)
        };
        let result = if action == Action::Quit {
            result.and(self.media.push(Request {
                action: Action::QuitMedia,
                generation,
            }))
        } else {
            result
        };
        if let Err(error) = &result {
            self.reports.error(error.clone());
        }
        result
    }
}
pub struct RecoveryWorker {
    handle: Handle,
    system: Option<JoinHandle<()>>,
    media: Option<JoinHandle<()>>,
}
impl RecoveryWorker {
    pub fn new(
        prepare: impl Fn(Action) -> u64 + Send + Sync + 'static,
        execute: impl Fn(Action, u64) -> Result<(), Failure> + Send + Sync + 'static,
        wake: impl Fn() + Send + Sync + 'static,
        notify: impl Fn(String) + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let handle = Handle {
            system: Arc::new(Lane::default()),
            media: Arc::new(Lane::default()),
            quitting: Arc::new(AtomicBool::new(false)),
            prepare: Arc::new(prepare),
            reports: Arc::new(Reports {
                errors: Mutex::new(VecDeque::new()),
                wake: Box::new(wake),
                notify: Box::new(notify),
            }),
        };
        let execute = Arc::new(execute);
        let spawn = |name: &str, lane: Arc<Lane>| {
            let execute = execute.clone();
            let reports = handle.reports.clone();
            let quitting = handle.quitting.clone();
            std::thread::Builder::new().name(name.into()).spawn(move || {
                while let Some(request) = lane.next() {
                    if !request.action.quit() && quitting.load(Ordering::Acquire) { continue; }
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(request.action, request.generation)));
                    match outcome {
                        Ok(Ok(()) | Err(Failure::Cancelled)) => {}
                        Ok(Err(Failure::Error(message))) => reports.error(message),
                        Err(_) => reports.error("A tray recovery callback panicked. Later recovery commands are still available.".into()),
                    }
                    (reports.wake)();
                    if request.action.quit() { break; }
                }
            })
        };
        let system = spawn("oxbox-recovery-system", handle.system.clone())?;
        let media = match spawn("oxbox-recovery-media", handle.media.clone()) {
            Ok(worker) => worker,
            Err(error) => {
                let _ = handle.request(Action::Quit);
                let _ = system.join();
                return Err(error);
            }
        };
        Ok(Self {
            handle,
            system: Some(system),
            media: Some(media),
        })
    }
    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }
    pub fn drain_errors(&self) -> Vec<String> {
        self.handle
            .reports
            .errors
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .drain(..)
            .collect()
    }
    pub fn shutdown(&mut self) {
        let _ = self.handle.request(Action::Quit);
        if let Some(worker) = self.system.take() {
            let _ = worker.join();
        }
        if let Some(worker) = self.media.take() {
            let _ = worker.join();
        }
    }
}
impl Drop for RecoveryWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    };
    use std::time::Duration;
    #[test]
    fn media_quit_does_not_wait_for_blocked_system_cleanup() {
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let held = gate.clone();
        let (done, ready) = mpsc::channel();
        let mut worker = RecoveryWorker::new(
            |_| 1,
            move |action, _| {
                if action == Action::Quit {
                    let mut open = held.0.lock().unwrap();
                    while !*open {
                        open = held.1.wait(open).unwrap();
                    }
                }
                if action == Action::QuitMedia {
                    done.send(()).unwrap();
                }
                Ok(())
            },
            || {},
            |_| {},
        )
        .unwrap();
        worker.handle().request(Action::Quit).unwrap();
        let result = ready.recv_timeout(Duration::from_secs(2));
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        worker.shutdown();
        assert!(
            result.is_ok(),
            "media cleanup waited behind system state restoration"
        );
    }
    #[test]
    fn accepted_consecutive_toggles_are_serialized_and_not_collapsed() {
        let active = Arc::new(AtomicU64::new(1));
        let state = active.clone();
        let (done, ready) = mpsc::channel();
        let mut worker = RecoveryWorker::new(
            |_| 1,
            move |action, _| {
                if action == Action::ToggleAwake {
                    state.fetch_xor(1, Ordering::SeqCst);
                    done.send(()).unwrap();
                }
                Ok(())
            },
            || {},
            |_| {},
        )
        .unwrap();
        let handle = worker.handle();
        handle.request(Action::ToggleAwake).unwrap();
        handle.request(Action::ToggleAwake).unwrap();
        let first = ready.recv_timeout(Duration::from_secs(2));
        let second = ready.recv_timeout(Duration::from_secs(2));
        worker.shutdown();
        assert!(first.is_ok() && second.is_ok());
        assert_eq!(active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn blocked_media_and_unpolled_ui_do_not_delay_system_cancellation() {
        let epoch = Arc::new(AtomicU64::new(0));
        let prepared = epoch.clone();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let held = gate.clone();
        let (started, media) = mpsc::channel();
        let (done, system) = mpsc::channel();
        let mut worker = RecoveryWorker::new(
            move |_| prepared.fetch_add(1, Ordering::SeqCst) + 1,
            move |action, _| {
                if action == Action::StopRecording {
                    started.send(()).unwrap();
                    let (lock, wake) = &*held;
                    let mut ready = lock.lock().unwrap();
                    while !*ready {
                        ready = wake.wait(ready).unwrap();
                    }
                } else if action == Action::CancelSleep {
                    done.send(()).unwrap();
                }
                Ok(())
            },
            || {},
            |_| {},
        )
        .unwrap();
        let handle = worker.handle();
        handle.request(Action::StopRecording).unwrap();
        media.recv_timeout(Duration::from_secs(2)).unwrap();
        handle.request(Action::CancelSleep).unwrap();
        let result = system.recv_timeout(Duration::from_secs(2));
        let prepared_before_ack = epoch.load(Ordering::SeqCst);
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        worker.shutdown();
        assert!(
            result.is_ok(),
            "system recovery waited for recorder finalization/UI polling"
        );
        assert_eq!(prepared_before_ack, 2);
    }
    #[test]
    fn callback_panics_and_errors_are_reported_and_later_recovery_still_runs() {
        let reported = Arc::new(Mutex::new(vec![]));
        let messages = reported.clone();
        let (done, ready) = mpsc::channel();
        let mut worker = RecoveryWorker::new(
            |_| 1,
            move |action, _| match action {
                Action::Lock => panic!("fake backend panic"),
                Action::ToggleDesktop => Err(Failure::Error("fake desktop failure".into())),
                Action::StopAwake => {
                    done.send(()).unwrap();
                    Ok(())
                }
                _ => Ok(()),
            },
            || {},
            move |message| messages.lock().unwrap().push(message),
        )
        .unwrap();
        let handle = worker.handle();
        handle.request(Action::Lock).unwrap();
        handle.request(Action::ToggleDesktop).unwrap();
        handle.request(Action::StopAwake).unwrap();
        let result = ready.recv_timeout(Duration::from_secs(2));
        worker.shutdown();
        assert!(result.is_ok());
        assert_eq!(reported.lock().unwrap().len(), 2);
        assert_eq!(worker.drain_errors().len(), 2);
    }
    #[test]
    fn a_full_normal_queue_coalesces_and_keeps_critical_recovery() {
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let held = gate.clone();
        let (started, ready) = mpsc::channel();
        let mut worker = RecoveryWorker::new(
            |_| 1,
            move |action, _| {
                if action == Action::Lock {
                    started.send(()).unwrap();
                    let mut open = held.0.lock().unwrap();
                    while !*open {
                        open = held.1.wait(open).unwrap();
                    }
                }
                Ok(())
            },
            || {},
            |_| {},
        )
        .unwrap();
        let handle = worker.handle();
        handle.request(Action::Lock).unwrap();
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        for _ in 0..64 {
            let _ = handle.request(Action::ToggleDesktop);
        }
        handle.request(Action::StopAwake).unwrap();
        handle.request(Action::StopAwake).unwrap();
        handle.request(Action::CancelSleep).unwrap();
        let pending = handle.system.state.lock().unwrap().items.clone();
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        worker.shutdown();
        assert!(pending.len() <= CAPACITY);
        assert_eq!(
            pending
                .iter()
                .filter(|r| r.action == Action::StopAwake)
                .count(),
            1
        );
        assert_eq!(
            pending
                .iter()
                .filter(|r| r.action == Action::CancelSleep)
                .count(),
            1
        );
    }
    #[test]
    fn quit_preparation_is_immediate_and_shutdown_joins_both_lanes() {
        let closing = Arc::new(AtomicU64::new(0));
        let prepared = closing.clone();
        let cleaned = Arc::new(AtomicU64::new(0));
        let cleanup = cleaned.clone();
        let mut worker = RecoveryWorker::new(
            move |action| {
                if action == Action::Quit {
                    prepared.fetch_add(1, Ordering::SeqCst);
                }
                1
            },
            move |action, _| {
                if action == Action::Quit {
                    cleanup.fetch_add(1, Ordering::SeqCst);
                }
                Ok(())
            },
            || {},
            |_| {},
        )
        .unwrap();
        let handle = worker.handle();
        handle.request(Action::Quit).unwrap();
        assert_eq!(closing.load(Ordering::SeqCst), 1);
        assert!(handle.request(Action::ToggleAwake).is_err());
        worker.shutdown();
        assert_eq!(closing.load(Ordering::SeqCst), 1);
        assert_eq!(cleaned.load(Ordering::SeqCst), 1);
    }
}
