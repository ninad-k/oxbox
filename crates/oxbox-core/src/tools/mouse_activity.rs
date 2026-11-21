//! Tool: **Mouse Activity** (`mouse-activity`). Explicit, idle-aware, bounded pointer motion.
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
#[cfg(any(windows, test))]
use std::{sync::mpsc, thread::JoinHandle, time::Duration};

#[cfg(windows)]
#[path = "../platform/mouse_input.rs"]
mod native;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub idle_seconds: u64,
    pub interval_seconds: u64,
    pub duration_minutes: Option<u64>,
}
impl Config {
    pub fn validate(self) -> CoreResult<()> {
        if !(30..=3600).contains(&self.idle_seconds)
            || !(5..=300).contains(&self.interval_seconds)
            || self
                .duration_minutes
                .is_some_and(|minutes| !(1..=1440).contains(&minutes))
        {
            return Err(CoreError::InvalidInput("Choose an idle delay of 30–3600 seconds, an interval of 5–300 seconds, and an optional duration of 1–1440 minutes.".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    #[default]
    Stopped,
    Waiting,
    Paused,
    Simulating,
}
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub active: bool,
    pub phase: Phase,
    pub idle_seconds: u64,
    pub movements: u64,
    pub remaining_seconds: Option<u64>,
    pub error: Option<String>,
}

#[cfg(any(windows, test))]
struct Engine {
    config: Config,
    started: u64,
    last_genuine: u64,
    last_move: Option<u64>,
    movements: u64,
    has_input: bool,
    right: bool,
}
#[cfg(any(windows, test))]
impl Engine {
    fn new(config: Config, now: u64) -> CoreResult<Self> {
        config.validate()?;
        Ok(Self {
            config,
            started: now,
            last_genuine: now,
            last_move: None,
            movements: 0,
            has_input: false,
            right: true,
        })
    }
    fn input(&mut self, now: u64, synthetic: bool) {
        if !synthetic && now > self.last_genuine {
            self.last_genuine = now;
            self.has_input = true;
            self.right = true;
        }
    }
    fn expired(&self, now: u64) -> bool {
        self.config
            .duration_minutes
            .is_some_and(|minutes| now.saturating_sub(self.started) >= minutes * 60 * 1000)
    }
    fn due(&self, now: u64) -> bool {
        !self.expired(now)
            && now.saturating_sub(self.last_genuine) >= self.config.idle_seconds * 1000
            && self
                .last_move
                .is_none_or(|time| now.saturating_sub(time) >= self.config.interval_seconds * 1000)
    }
    fn moved(&mut self, now: u64) {
        self.last_move = Some(now);
        self.movements = self.movements.saturating_add(1);
        self.right = !self.right;
    }
    fn status(&self, now: u64) -> Status {
        let active = !self.expired(now);
        let idle = now.saturating_sub(self.last_genuine) / 1000;
        Status {
            active,
            phase: if !active {
                Phase::Stopped
            } else if idle >= self.config.idle_seconds {
                Phase::Simulating
            } else if self.has_input {
                Phase::Paused
            } else {
                Phase::Waiting
            },
            idle_seconds: idle,
            movements: self.movements,
            remaining_seconds: self.config.duration_minutes.map(|minutes| {
                (minutes * 60 * 1000)
                    .saturating_sub(now.saturating_sub(self.started))
                    .div_ceil(1000)
            }),
            error: None,
        }
    }
}

#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Point {
    x: i32,
    y: i32,
}
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy)]
struct Bounds {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}
#[cfg(any(windows, test))]
fn step(point: Point, bounds: Bounds, right: bool) -> CoreResult<Point> {
    let (x, y) = (i64::from(point.x), i64::from(point.y));
    let (left, top) = (i64::from(bounds.left), i64::from(bounds.top));
    let width = i64::from(bounds.width);
    let height = i64::from(bounds.height);
    if !(2..=65536).contains(&width)
        || !(1..=65536).contains(&height)
        || x < left
        || x >= left + width
        || y < top
        || y >= top + height
    {
        return Err(CoreError::InvalidInput(
            "Mouse Activity cannot safely address this desktop geometry.".into(),
        ));
    }
    let next = if right {
        if x + 1 < left + width { x + 1 } else { x - 1 }
    } else if x > left {
        x - 1
    } else {
        x + 1
    };
    Ok(Point {
        x: i32::try_from(next).map_err(|_| {
            CoreError::InvalidInput("Mouse coordinates exceed the supported range.".into())
        })?,
        y: point.y,
    })
}

#[cfg(any(windows, test))]
#[derive(Clone, Copy)]
struct InputState {
    now: u64,
    last_genuine: u64,
}
#[cfg(any(windows, test))]
trait Backend {
    fn poll(&mut self) -> CoreResult<InputState>;
    /// Recheck input immediately before moving; false means activity superseded the decision.
    fn move_one(
        &mut self,
        last_genuine: u64,
        right: bool,
        stop: &AtomicBool,
        deadline: Option<u64>,
    ) -> CoreResult<bool>;
}
#[cfg(any(windows, test))]
struct Worker {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

pub struct Controller {
    status: Arc<Mutex<Status>>,
    #[cfg(any(windows, test))]
    worker: Option<Worker>,
    stopping: bool,
}
impl Default for Controller {
    fn default() -> Self {
        Self {
            status: Arc::new(Mutex::new(Status::default())),
            #[cfg(any(windows, test))]
            worker: None,
            stopping: false,
        }
    }
}
fn unavailable() -> CoreError {
    CoreError::InvalidInput("Mouse Activity worker is unavailable. Restart Oxbox.".into())
}
impl Controller {
    pub fn start(&mut self, config: Config) -> CoreResult<Status> {
        self.start_signalled(config, Arc::new(AtomicBool::new(false)))
    }
    fn start_signalled(&mut self, config: Config, stop: Arc<AtomicBool>) -> CoreResult<Status> {
        config.validate()?;
        if self.stopping || stop.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        #[cfg(windows)]
        {
            self.start_with_stop(config, native::NativeInput::new, stop)
        }
        #[cfg(not(windows))]
        {
            Err(CoreError::Unsupported(
                crate::platform::Os::current().display_name(),
            ))
        }
    }
    pub fn status(&self) -> CoreResult<Status> {
        self.status
            .lock()
            .map(|value| value.clone())
            .map_err(|_| unavailable())
    }
    pub fn stop(&mut self) -> CoreResult<Status> {
        #[cfg(any(windows, test))]
        if let Some(worker) = self.worker.take() {
            worker.stop.store(true, Ordering::Release);
            worker.thread.thread().unpark();
            worker.thread.join().map_err(|_| unavailable())?;
        }
        let mut status = self.status.lock().map_err(|_| unavailable())?;
        *status = Status::default();
        Ok(status.clone())
    }
    pub fn shutdown(&mut self) {
        self.stopping = true;
        let _ = self.stop();
    }
    #[cfg(test)]
    fn start_with<B: Backend + 'static>(
        &mut self,
        config: Config,
        factory: impl FnOnce() -> CoreResult<B> + Send + 'static,
    ) -> CoreResult<Status> {
        self.start_with_stop(config, factory, Arc::new(AtomicBool::new(false)))
    }
    #[cfg(any(windows, test))]
    fn start_with_stop<B: Backend + 'static>(
        &mut self,
        config: Config,
        factory: impl FnOnce() -> CoreResult<B> + Send + 'static,
        stop: Arc<AtomicBool>,
    ) -> CoreResult<Status> {
        config.validate()?;
        if self.stopping || stop.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        self.stop()?;
        let worker_stop = stop.clone();
        let status = self.status.clone();
        let (ready, reply) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("oxbox-mouse-activity".into())
            .spawn(move || {
                let result: CoreResult<()> = (|| {
                    if worker_stop.load(Ordering::Acquire) {
                        return Err(CoreError::Cancelled);
                    }
                    let mut backend = factory()?;
                    if worker_stop.load(Ordering::Acquire) {
                        return Err(CoreError::Cancelled);
                    }
                    let initial = backend.poll()?;
                    let mut engine = Engine::new(config, initial.now)?;
                    let initial_status = engine.status(initial.now);
                    if let Ok(mut value) = status.lock() {
                        *value = initial_status.clone();
                    }
                    let _ = ready.send(Ok(initial_status));
                    while !worker_stop.load(Ordering::Acquire) {
                        let input = backend.poll()?;
                        engine.input(input.last_genuine, false);
                        if engine.expired(input.now) {
                            if let Ok(mut value) = status.lock() {
                                *value = engine.status(input.now);
                            }
                            return Ok(());
                        }
                        if engine.due(input.now)
                            && !worker_stop.load(Ordering::Acquire)
                            && backend.move_one(
                                input.last_genuine,
                                engine.right,
                                &worker_stop,
                                config.duration_minutes.map(|minutes| {
                                    engine.started.saturating_add(minutes * 60 * 1000)
                                }),
                            )?
                        {
                            engine.moved(input.now);
                        }
                        if let Ok(mut value) = status.lock() {
                            *value = engine.status(input.now);
                        }
                        std::thread::park_timeout(Duration::from_millis(25));
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    let message = error.to_string();
                    let _ = ready.send(Err(error));
                    if let Ok(mut value) = status.lock() {
                        value.active = false;
                        value.phase = Phase::Stopped;
                        value.error = Some(message);
                    }
                } else if worker_stop.load(Ordering::Acquire)
                    && let Ok(mut value) = status.lock()
                {
                    value.active = false;
                    value.phase = Phase::Stopped;
                }
            })?;
        self.worker = Some(Worker { stop, thread });
        match reply.recv().map_err(|_| unavailable())? {
            Ok(status) => Ok(status),
            Err(error) => {
                let _ = self.stop();
                Err(error)
            }
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Start,
    Stop,
}
impl Action {
    fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
        }
    }
}
/// An action-bound, single-use reservation. Its wire representation is a string
/// so JavaScript cannot round the internal generation counter.
#[derive(Debug, Clone, Copy)]
pub struct Request {
    id: u64,
    action: Action,
}
impl Serialize for Request {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("ma1:{}:{}", self.action.name(), self.id))
    }
}
impl<'de> Deserialize<'de> for Request {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Token;
        impl serde::de::Visitor<'_> for Token {
            type Value = Request;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an issued Mouse Activity request token")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Request, E> {
                let invalid = || E::custom("Invalid Mouse Activity request token.");
                if value.len() > 40 {
                    return Err(invalid());
                }
                let mut fields = value.split(':');
                if fields.next() != Some("ma1") {
                    return Err(invalid());
                }
                let action = match fields.next() {
                    Some("start") => Action::Start,
                    Some("stop") => Action::Stop,
                    _ => return Err(invalid()),
                };
                let digits = fields.next().ok_or_else(invalid)?;
                if digits.is_empty()
                    || digits.starts_with('0')
                    || !digits.bytes().all(|value| value.is_ascii_digit())
                    || fields.next().is_some()
                {
                    return Err(invalid());
                }
                let id = digits.parse().map_err(|_| invalid())?;
                Ok(Request { id, action })
            }
        }
        deserializer.deserialize_str(Token)
    }
}
#[derive(Default)]
struct Session {
    controller: Controller,
    applied: u64,
}
/// Orders command intentions independently of blocking-pool scheduling.
#[derive(Default)]
pub struct Service {
    closed: AtomicBool,
    requested: AtomicU64,
    issued: Mutex<BTreeMap<u64, Action>>,
    session: Mutex<Session>,
    active_stop: Mutex<Option<Arc<AtomicBool>>>,
}
impl Service {
    /// Signal recovery without waiting for the managed session or its worker.
    pub fn request_stop(&self) -> CoreResult<Request> {
        let request = self.reserve(Action::Stop);
        if request.is_err() {
            self.closed.store(true, Ordering::Release);
        }
        self.signal_active();
        request
    }
    /// Close admission and signal recovery; normal shutdown still joins ownership.
    pub fn request_shutdown(&self) {
        self.closed.store(true, Ordering::Release);
        self.signal_active();
        let mut issued = self
            .issued
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        self.requested.store(
            self.requested.load(Ordering::SeqCst).saturating_add(1),
            Ordering::SeqCst,
        );
        issued.clear();
    }
    fn signal_active(&self) {
        let active = self
            .active_stop
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if let Some(stop) = &*active {
            stop.store(true, Ordering::Release);
        }
    }
    pub fn reserve(&self, action: Action) -> CoreResult<Request> {
        let mut issued = self.issued.lock().map_err(|_| unavailable())?;
        if action == Action::Start && self.closed.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        let id = self
            .requested
            .load(Ordering::SeqCst)
            .checked_add(1)
            .ok_or_else(unavailable)?;
        self.requested.store(id, Ordering::SeqCst);
        issued.insert(id, action);
        while issued.len() > 64 {
            issued.pop_first();
        }
        Ok(Request { id, action })
    }
    fn claim(&self, request: Request, action: Action) -> CoreResult<()> {
        let mut issued = self.issued.lock().map_err(|_| unavailable())?;
        if request.action != action || issued.get(&request.id) != Some(&action) {
            return Err(CoreError::InvalidInput("Mouse Activity request expired, was already used, or has the wrong action. Try again.".into()));
        }
        issued.remove(&request.id);
        Ok(())
    }
    fn apply_start(
        &self,
        request: Request,
        start: impl FnOnce(&mut Controller, Arc<AtomicBool>) -> CoreResult<Status>,
    ) -> CoreResult<Status> {
        if self.closed.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        self.claim(request, Action::Start)?;
        let mut session = self.session.lock().map_err(|_| unavailable())?;
        if self.closed.load(Ordering::Acquire)
            || self.requested.load(Ordering::SeqCst) != request.id
        {
            return Err(CoreError::Cancelled);
        }
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active_stop.lock().map_err(|_| unavailable())?;
            // A replacement Start already owns ending its predecessor. Signal
            // it before publishing the new flag so recovery cannot miss either.
            if let Some(previous) = active.replace(stop.clone()) {
                previous.store(true, Ordering::Release);
            }
        }
        // Recovery can arrive before or during publication. Never reset this
        // flag: the exact published Arc belongs to the eventual native worker.
        if self.closed.load(Ordering::Acquire)
            || self.requested.load(Ordering::SeqCst) != request.id
        {
            stop.store(true, Ordering::Release);
            session.controller.stop()?;
            return Err(CoreError::Cancelled);
        }
        let result = start(&mut session.controller, stop);
        if result.is_ok() {
            session.applied = request.id;
        }
        // Stop may arrive while native observation is being initialized. Release
        // any just-created worker before returning a stale Start response.
        if self.closed.load(Ordering::Acquire)
            || self.requested.load(Ordering::SeqCst) != request.id
        {
            session.controller.stop()?;
            return Err(CoreError::Cancelled);
        }
        result
    }
    pub fn start(&self, request: Request, config: Config) -> CoreResult<Status> {
        config.validate()?;
        self.apply_start(request, |controller, stop| {
            controller.start_signalled(config, stop)
        })
    }
    pub fn stop(&self, request: Request) -> CoreResult<Status> {
        self.claim(request, Action::Stop)?;
        let mut session = self.session.lock().map_err(|_| unavailable())?;
        // A delayed Stop must not terminate a newer, explicitly requested session.
        if session.applied > request.id {
            return session.controller.status();
        }
        let status = session.controller.stop()?;
        session.applied = request.id;
        Ok(status)
    }
    pub fn status(&self) -> CoreResult<Status> {
        self.session
            .lock()
            .map_err(|_| unavailable())?
            .controller
            .status()
    }
    pub fn shutdown(&self) {
        self.request_shutdown();
        self.session
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .controller
            .shutdown();
    }
    #[cfg(test)]
    fn start_with<B: Backend + 'static>(
        &self,
        request: Request,
        config: Config,
        factory: impl FnOnce() -> CoreResult<B> + Send + 'static,
    ) -> CoreResult<Status> {
        config.validate()?;
        self.apply_start(request, |controller, stop| {
            controller.start_with_stop(config, factory, stop)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Config {
        Config {
            idle_seconds: 30,
            interval_seconds: 5,
            duration_minutes: Some(1),
        }
    }
    #[test]
    fn fast_stop_ends_an_active_worker_while_the_session_mutex_is_held() {
        let service = Service::default();
        let (backend, now, _, moves, dropped) = fake(false);
        let start = service.reserve(Action::Start).unwrap();
        service
            .start_with(start, config(), move || Ok(backend))
            .unwrap();
        let held = service.session.lock().unwrap();
        let stop = service.request_stop().unwrap();
        now.store(30_000, Ordering::Release);
        let ended_before_release = dropped.recv_timeout(Duration::from_secs(2)).is_ok();
        drop(held);
        service.stop(stop).unwrap();
        assert!(
            ended_before_release,
            "fast Stop waited for the session mutex"
        );
        assert!(
            moves.try_recv().is_err(),
            "worker moved after the stop signal"
        );
    }
    #[test]
    fn fast_shutdown_refuses_a_previously_reserved_start_and_new_admission() {
        let service = Service::default();
        let start = service.reserve(Action::Start).unwrap();
        service.request_shutdown();
        let result = service.start_with(start, config(), move || Ok(fake(false).0));
        let late_admission = service.reserve(Action::Start);
        service.shutdown();
        assert!(matches!(result, Err(CoreError::Cancelled)));
        assert!(matches!(late_admission, Err(CoreError::Cancelled)));
    }
    #[test]
    fn fast_stop_signals_the_same_flag_while_start_factory_is_blocked() {
        let service = Arc::new(Service::default());
        let start = service.reserve(Action::Start).unwrap();
        let (entered, ready) = mpsc::sync_channel(1);
        let (release, wait) = mpsc::sync_channel(1);
        let (backend, _, _, moves, dropped) = fake(false);
        let pending = service.clone();
        let worker = std::thread::spawn(move || {
            pending.start_with(start, config(), move || {
                entered.send(()).unwrap();
                wait.recv().unwrap();
                Ok(backend)
            })
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let published = service.active_stop.lock().unwrap().clone().unwrap();
        let stop = service.request_stop().unwrap();
        let signalled_during_setup = published.load(Ordering::Acquire);
        release.send(()).unwrap();
        let result = worker.join().unwrap();
        service.stop(stop).unwrap();
        assert!(signalled_during_setup);
        assert!(matches!(result, Err(CoreError::Cancelled)));
        dropped.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(moves.try_recv().is_err());
        assert!(
            published.load(Ordering::Acquire),
            "a later worker reset the old Stop signal"
        );
    }
    #[test]
    fn validates_ranges_before_starting() {
        assert!(config().validate().is_ok());
        for config in [
            Config {
                idle_seconds: 0,
                ..config()
            },
            Config {
                interval_seconds: 0,
                ..config()
            },
            Config {
                duration_minutes: Some(0),
                ..config()
            },
            Config {
                duration_minutes: Some(1441),
                ..config()
            },
        ] {
            assert!(config.validate().is_err());
        }
    }
    #[test]
    fn waits_full_idle_and_ignores_synthetic_events() {
        let mut engine = Engine::new(
            Config {
                duration_minutes: None,
                ..config()
            },
            1000,
        )
        .unwrap();
        assert!(!engine.due(30_999));
        assert!(engine.due(31_000));
        engine.moved(31_000);
        engine.input(32_000, true);
        assert!(!engine.due(35_999));
        assert!(engine.due(36_000));
        engine.input(36_000, false);
        assert!(!engine.due(65_999));
        assert!(engine.due(66_000));
    }
    #[test]
    fn expiry_prevents_motion_even_if_idle() {
        let engine = Engine::new(config(), 1000).unwrap();
        assert!(engine.due(60_999));
        assert!(!engine.due(61_000));
        assert!(!engine.status(61_000).active);
    }
    #[test]
    fn bounded_step_never_restores_an_old_pointer_or_crosses_bounds() {
        let bounds = Bounds {
            left: -1920,
            top: 0,
            width: 3840,
            height: 1080,
        };
        assert_eq!(
            step(Point { x: 1919, y: 100 }, bounds, true).unwrap(),
            Point { x: 1918, y: 100 }
        );
        assert_eq!(
            step(Point { x: -1920, y: 100 }, bounds, false).unwrap(),
            Point { x: -1919, y: 100 }
        );
        assert!(step(Point { x: 0, y: 0 }, Bounds { width: 0, ..bounds }, true).is_err());
        assert!(step(Point { x: i32::MAX, y: 0 }, bounds, true).is_err());
    }
    struct Fake {
        now: Arc<std::sync::atomic::AtomicU64>,
        last: Arc<std::sync::atomic::AtomicU64>,
        moves: mpsc::Sender<()>,
        dropped: mpsc::Sender<()>,
        fail: bool,
    }
    impl Backend for Fake {
        fn poll(&mut self) -> CoreResult<InputState> {
            Ok(InputState {
                now: self.now.load(Ordering::Acquire),
                last_genuine: self.last.load(Ordering::Acquire),
            })
        }
        fn move_one(
            &mut self,
            last_genuine: u64,
            _: bool,
            stop: &AtomicBool,
            deadline: Option<u64>,
        ) -> CoreResult<bool> {
            if stop.load(Ordering::Acquire)
                || self.last.load(Ordering::Acquire) != last_genuine
                || deadline.is_some_and(|deadline| self.now.load(Ordering::Acquire) >= deadline)
            {
                return Ok(false);
            }
            if self.fail {
                return Err(CoreError::InvalidInput("fake injection refusal".into()));
            }
            let _ = self.moves.send(());
            Ok(true)
        }
    }
    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = self.dropped.send(());
        }
    }
    fn fake(
        fail: bool,
    ) -> (
        Fake,
        Arc<std::sync::atomic::AtomicU64>,
        Arc<std::sync::atomic::AtomicU64>,
        mpsc::Receiver<()>,
        mpsc::Receiver<()>,
    ) {
        let now = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let last = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let (moves, received) = mpsc::channel();
        let (dropped, drop_received) = mpsc::channel();
        (
            Fake {
                now: now.clone(),
                last: last.clone(),
                moves,
                dropped,
                fail,
            },
            now,
            last,
            received,
            drop_received,
        )
    }
    #[test]
    fn fake_input_suspends_worker_and_stop_waits_for_backend_drop() {
        let (backend, now, last, moves, dropped) = fake(false);
        let mut controller = Controller::default();
        assert!(!controller.status().unwrap().active);
        controller
            .start_with(
                Config {
                    duration_minutes: None,
                    ..config()
                },
                move || Ok(backend),
            )
            .unwrap();
        now.store(30_000, Ordering::Release);
        moves.recv_timeout(Duration::from_secs(2)).unwrap();
        last.store(30_001, Ordering::Release);
        now.store(30_001, Ordering::Release);
        std::thread::sleep(Duration::from_millis(75));
        assert_eq!(controller.status().unwrap().phase, Phase::Paused);
        assert!(moves.try_recv().is_err());
        now.store(60_001, Ordering::Release);
        moves.recv_timeout(Duration::from_secs(2)).unwrap();
        controller.stop().unwrap();
        dropped.recv_timeout(Duration::from_secs(1)).unwrap();
        now.store(120_001, Ordering::Release);
        assert!(moves.try_recv().is_err());
        assert!(!controller.status().unwrap().active);
    }
    #[test]
    fn fake_injection_failure_and_duration_expiry_end_the_session() {
        for fail in [false, true] {
            let (backend, now, _, moves, dropped) = fake(fail);
            let mut controller = Controller::default();
            controller
                .start_with(config(), move || Ok(backend))
                .unwrap();
            now.store(if fail { 30_000 } else { 60_000 }, Ordering::Release);
            dropped.recv_timeout(Duration::from_secs(2)).unwrap();
            controller.worker.take().unwrap().thread.join().unwrap();
            let status = controller.status().unwrap();
            assert!(!status.active);
            assert_eq!(status.error.is_some(), fail);
            assert!(moves.try_recv().is_err());
            controller.stop().unwrap();
        }
    }
    #[test]
    fn shutdown_refuses_queued_start_and_factory_failure_is_stopped() {
        let mut controller = Controller::default();
        controller.shutdown();
        let invoked = Arc::new(AtomicBool::new(false));
        let called = invoked.clone();
        assert!(
            controller
                .start_with(config(), move || {
                    called.store(true, Ordering::Release);
                    Ok(fake(false).0)
                })
                .is_err()
        );
        assert!(!invoked.load(Ordering::Acquire));
        let mut controller = Controller::default();
        assert!(
            controller
                .start_with(config(), || -> CoreResult<Fake> {
                    Err(CoreError::InvalidInput("fake observation refusal".into()))
                })
                .is_err()
        );
        assert!(!controller.status().unwrap().active);
        controller.stop().unwrap();
    }
    #[test]
    fn final_backend_check_observes_duration_expiry_before_injection() {
        let (mut backend, now, _, moves, _) = fake(false);
        now.store(60_000, Ordering::Release);
        assert!(
            !backend
                .move_one(0, true, &AtomicBool::new(false), Some(60_000))
                .unwrap()
        );
        assert!(moves.try_recv().is_err());
    }
    #[test]
    fn starting_after_the_last_observed_input_does_not_suppress_all_motion() {
        let (backend, now, last, moves, _) = fake(false);
        now.store(1000, Ordering::Release);
        last.store(900, Ordering::Release);
        let mut controller = Controller::default();
        controller
            .start_with(config(), move || Ok(backend))
            .unwrap();
        now.store(31_000, Ordering::Release);
        moves.recv_timeout(Duration::from_secs(1)).unwrap();
        controller.stop().unwrap();
    }
    #[test]
    fn acknowledged_stop_invalidates_an_earlier_queued_start() {
        let service = Service::default();
        let start = service.reserve(Action::Start).unwrap();
        let stop = service.reserve(Action::Stop).unwrap();
        assert!(!service.stop(stop).unwrap().active);
        let called = Arc::new(AtomicBool::new(false));
        let invoked = called.clone();
        assert!(
            service
                .start_with(start, config(), move || {
                    invoked.store(true, Ordering::Release);
                    Ok(fake(false).0)
                })
                .is_err()
        );
        assert!(!called.load(Ordering::Acquire));
        assert!(!service.status().unwrap().active);
    }
    #[test]
    fn an_older_queued_stop_cannot_stop_a_newer_explicit_start() {
        let service = Service::default();
        let old_stop = service.reserve(Action::Stop).unwrap();
        let new_start = service.reserve(Action::Start).unwrap();
        let (backend, _, _, _, dropped) = fake(false);
        assert!(
            service
                .start_with(new_start, config(), move || Ok(backend))
                .unwrap()
                .active
        );
        assert!(service.stop(old_stop).unwrap().active);
        assert!(dropped.try_recv().is_err());
        let stop = service.reserve(Action::Stop).unwrap();
        assert!(!service.stop(stop).unwrap().active);
        dropped.recv_timeout(Duration::from_secs(1)).unwrap();
    }
    #[test]
    fn stop_during_initialization_cancels_start_before_it_can_outlive_acknowledgement() {
        let service = Arc::new(Service::default());
        let start = service.reserve(Action::Start).unwrap();
        let (entered, entry) = mpsc::sync_channel(0);
        let (release, released) = mpsc::sync_channel(0);
        let (backend, _, _, _, dropped) = fake(false);
        let worker_service = service.clone();
        let starting = std::thread::spawn(move || {
            worker_service.start_with(start, config(), move || {
                entered.send(()).unwrap();
                released.recv().unwrap();
                Ok(backend)
            })
        });
        entry.recv_timeout(Duration::from_secs(1)).unwrap();
        let stop = service.reserve(Action::Stop).unwrap();
        release.send(()).unwrap();
        assert!(starting.join().unwrap().is_err());
        assert!(!service.stop(stop).unwrap().active);
        dropped.recv_timeout(Duration::from_secs(1)).unwrap();
        service.shutdown();
        assert!(matches!(
            service.reserve(Action::Start),
            Err(CoreError::Cancelled)
        ));
    }
    #[test]
    fn reservation_precedes_first_poll_and_tokens_remain_exact_strings() {
        let service = Service::default();
        let start = service.reserve(Action::Start).unwrap();
        let encoded = serde_json::to_string(&start).unwrap();
        assert!(encoded.starts_with('"'));
        assert_eq!(
            serde_json::to_string(&Request {
                id: u64::MAX,
                action: Action::Stop
            })
            .unwrap(),
            "\"ma1:stop:18446744073709551615\""
        );
        let delayed: Request = serde_json::from_str(&encoded).unwrap();
        let stop = service.reserve(Action::Stop).unwrap();
        assert!(!service.stop(stop).unwrap().active);
        assert!(
            service
                .start_with(delayed, config(), || Ok(fake(false).0))
                .is_err()
        );
    }
    #[test]
    fn unknown_future_replayed_and_wrong_action_tokens_cannot_mutate_sessions() {
        let service = Service::default();
        let start = service.reserve(Action::Start).unwrap();
        let future: Request = serde_json::from_str("\"ma1:stop:18446744073709551615\"").unwrap();
        assert!(service.stop(future).is_err());
        assert!(service.stop(start).is_err());
        let (backend, _, _, _, dropped) = fake(false);
        assert!(
            service
                .start_with(start, config(), move || Ok(backend))
                .unwrap()
                .active
        );
        let unknown: Request = serde_json::from_str("\"ma1:stop:900\"").unwrap();
        assert!(service.stop(unknown).is_err());
        assert!(service.status().unwrap().active);
        let stop = service.reserve(Action::Stop).unwrap();
        assert!(!service.stop(stop).unwrap().active);
        dropped.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(service.stop(stop).is_err());
        for encoded in [
            "0",
            "\"ma1:stop:0\"",
            "\"ma1:launch:1\"",
            "\"ma1:stop:+1\"",
            "\"ma1:stop:01\"",
            "\"ma1:stop:18446744073709551616\"",
        ] {
            assert!(serde_json::from_str::<Request>(encoded).is_err());
        }
    }
    #[test]
    fn expired_unclaimed_reservations_are_bounded_and_do_not_poison_new_starts() {
        let service = Service::default();
        let expired = service.reserve(Action::Start).unwrap();
        for _ in 0..100 {
            service.reserve(Action::Stop).unwrap();
        }
        assert_eq!(service.issued.lock().unwrap().len(), 64);
        assert!(
            service
                .start_with(expired, config(), || Ok(fake(false).0))
                .is_err()
        );
        let start = service.reserve(Action::Start).unwrap();
        assert!(
            service
                .start_with(start, config(), || Ok(fake(false).0))
                .unwrap()
                .active
        );
        service.shutdown();
        assert!(!service.status().unwrap().active);
    }
}
