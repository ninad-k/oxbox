//! Tool: **Do Not Sleep** (`do-not-sleep`). The OS guard lives and dies on one worker thread.
//! This matters on Windows: SetThreadExecutionState applies to its calling thread.
use crate::{CoreError, CoreResult};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub fn deadline(now: u64, minutes: Option<u64>, until: Option<u64>) -> CoreResult<Option<u64>> {
    if minutes.is_some() && until.is_some() {
        return Err(CoreError::InvalidInput(
            "Choose either a duration or an end time.".into(),
        ));
    }
    if let Some(minutes) = minutes {
        if minutes == 0 || minutes > 31 * 24 * 60 {
            return Err(CoreError::InvalidInput(
                "Choose a duration between 1 minute and 31 days.".into(),
            ));
        }
        return now
            .checked_add(minutes * 60)
            .map(Some)
            .ok_or_else(|| CoreError::InvalidInput("Duration is too large.".into()));
    }
    if let Some(until) = until
        && (until <= now || until.saturating_sub(now) > 31 * 24 * 3600)
    {
        return Err(CoreError::InvalidInput(
            "Choose an end time in the next 31 days.".into(),
        ));
    }
    Ok(until)
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AwakeStatus {
    pub active: bool,
    pub display: bool,
    pub until: Option<u64>,
}
enum Request {
    Start {
        display: bool,
        until: Option<u64>,
        duration: Option<Duration>,
        reply: mpsc::SyncSender<CoreResult<AwakeStatus>>,
    },
    Stop {
        reply: mpsc::SyncSender<()>,
    },
    Quit,
}

pub struct AwakeController {
    sender: mpsc::Sender<Request>,
    status: Arc<Mutex<AwakeStatus>>,
    worker: Option<JoinHandle<()>>,
}
fn unavailable() -> CoreError {
    CoreError::InvalidInput("The keep-awake worker is unavailable. Restart Oxbox.".into())
}
impl AwakeController {
    pub fn new() -> CoreResult<Self> {
        Self::with_factory(|display| {
            keepawake::Builder::default()
                .display(display)
                .idle(true)
                .reason("User requested Do Not Sleep")
                .app_name("Oxbox")
                .app_reverse_domain("io.github.ninad-k.oxbox")
                .create()
                .map_err(|error| {
                    CoreError::InvalidInput(format!("Could not keep the system awake: {error}"))
                })
        })
    }
    fn with_factory<G: 'static>(
        factory: impl Fn(bool) -> CoreResult<G> + Send + 'static,
    ) -> CoreResult<Self> {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(Mutex::new(AwakeStatus::default()));
        let worker_status = status.clone();
        let worker = std::thread::Builder::new()
            .name("oxbox-keep-awake".into())
            .spawn(move || {
                let mut guard = None;
                let mut expires: Option<Instant> = None;
                loop {
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Ok(Request::Start {
                            display,
                            until,
                            duration,
                            reply,
                        }) => {
                            // Release first so the next guard restores the thread's baseline on drop.
                            guard = None;
                            expires = None;
                            let result = factory(display);
                            let response = match result {
                                Ok(awake) => {
                                    guard = Some(awake);
                                    expires = duration.and_then(|d| Instant::now().checked_add(d));
                                    Ok(AwakeStatus {
                                        active: true,
                                        display,
                                        until,
                                    })
                                }
                                Err(error) => Err(error),
                            };
                            if let Ok(mut value) = worker_status.lock() {
                                *value = response.as_ref().cloned().unwrap_or_default();
                            }
                            let _ = reply.send(response);
                        }
                        Ok(Request::Stop { reply }) => {
                            guard = None;
                            expires = None;
                            if let Ok(mut value) = worker_status.lock() {
                                *value = AwakeStatus::default();
                            }
                            let _ = reply.send(());
                        }
                        Ok(Request::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    if expires.is_some_and(|time| Instant::now() >= time) {
                        guard = None;
                        expires = None;
                        if let Ok(mut value) = worker_status.lock() {
                            *value = AwakeStatus::default();
                        }
                    }
                }
                drop(guard);
                if let Ok(mut value) = worker_status.lock() {
                    *value = AwakeStatus::default();
                }
            })?;
        Ok(Self {
            sender,
            status,
            worker: Some(worker),
        })
    }
    pub fn start(
        &self,
        now: u64,
        display: bool,
        minutes: Option<u64>,
        until: Option<u64>,
    ) -> CoreResult<AwakeStatus> {
        let until = deadline(now, minutes, until)?;
        let duration = until.map(|end| Duration::from_secs(end.saturating_sub(now)));
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .send(Request::Start {
                display,
                until,
                duration,
                reply,
            })
            .map_err(|_| unavailable())?;
        response.recv().map_err(|_| unavailable())?
    }
    pub fn stop(&self) -> CoreResult<()> {
        let (reply, response) = mpsc::sync_channel(1);
        self.sender
            .send(Request::Stop { reply })
            .map_err(|_| unavailable())?;
        response.recv().map_err(|_| unavailable())
    }
    pub fn status(&self) -> CoreResult<AwakeStatus> {
        self.status
            .lock()
            .map(|s| s.clone())
            .map_err(|_| unavailable())
    }
    pub fn shutdown(&mut self) {
        let _ = self.sender.send(Request::Quit);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
impl Drop for AwakeController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_guard_lifecycle_stays_on_one_worker_and_stops_before_ack() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Event {
            Created,
            Dropped,
        }
        struct FakeGuard(Arc<Mutex<Vec<(Event, std::thread::ThreadId)>>>);
        impl Drop for FakeGuard {
            fn drop(&mut self) {
                self.0
                    .lock()
                    .unwrap()
                    .push((Event::Dropped, std::thread::current().id()));
            }
        }
        let trace = Arc::new(Mutex::new(Vec::new()));
        let worker_trace = trace.clone();
        let mut controller = AwakeController::with_factory(move |_| {
            worker_trace
                .lock()
                .unwrap()
                .push((Event::Created, std::thread::current().id()));
            Ok(FakeGuard(worker_trace.clone()))
        })
        .unwrap();
        controller.start(0, false, None, None).unwrap();
        controller.start(0, true, Some(1), None).unwrap();
        controller.stop().unwrap();
        assert!(!controller.status().unwrap().active);
        assert_eq!(
            trace
                .lock()
                .unwrap()
                .iter()
                .map(|(e, _)| *e)
                .collect::<Vec<_>>(),
            vec![
                Event::Created,
                Event::Dropped,
                Event::Created,
                Event::Dropped
            ]
        );
        controller.start(0, false, None, None).unwrap();
        controller.shutdown();
        let events = trace.lock().unwrap();
        assert_eq!(events.len(), 6);
        assert!(events.iter().all(|(_, id)| *id == events[0].1));
        assert_ne!(events[0].1, std::thread::current().id());
    }
    #[test]
    fn factory_error_does_not_report_an_active_request() {
        let controller = AwakeController::with_factory(|_| -> CoreResult<()> {
            Err(CoreError::InvalidInput("fake error".into()))
        })
        .unwrap();
        assert!(controller.start(0, false, None, None).is_err());
        assert!(!controller.status().unwrap().active);
        controller.stop().unwrap();
    }
    #[test]
    fn timed_request_releases_guard_without_an_open_view() {
        struct FakeGuard(mpsc::Sender<()>);
        impl Drop for FakeGuard {
            fn drop(&mut self) {
                let _ = self.0.send(());
            }
        }
        let (dropped, received) = mpsc::channel();
        let controller =
            AwakeController::with_factory(move |_| Ok(FakeGuard(dropped.clone()))).unwrap();
        controller.start(0, false, None, Some(1)).unwrap();
        assert!(received.recv_timeout(Duration::from_secs(3)).is_ok());
        controller.stop().unwrap();
        assert!(!controller.status().unwrap().active);
    }
    #[test]
    fn validates_deadlines_without_os_effects() {
        assert_eq!(deadline(100, Some(1), None).unwrap(), Some(160));
        assert_eq!(deadline(100, None, Some(120)).unwrap(), Some(120));
        assert_eq!(deadline(100, None, None).unwrap(), None);
        assert!(deadline(100, Some(0), None).is_err());
        assert!(deadline(100, None, Some(99)).is_err());
        assert!(deadline(100, Some(1), Some(120)).is_err());
        assert!(deadline(u64::MAX, Some(1), None).is_err());
    }
}
