//! Cancellable UI jobs; workers retain ownership until joined on shutdown.
use eframe::egui;
use oxbox_core::{
    CoreResult,
    jobs::{CancelToken, Progress},
};
use std::{
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::JoinHandle,
};

struct Registration {
    lifetime: Weak<()>,
    cancel: CancelToken,
}
#[derive(Default)]
struct RegistryState {
    closed: AtomicBool,
    jobs: Mutex<Vec<Registration>>,
}
/// Shared cancellation boundary for every Task belonging to one native app.
#[derive(Clone, Default)]
pub struct Registry(Arc<RegistryState>);

pub fn register_context(ctx: &egui::Context) -> Registry {
    ctx.data_mut(|data| {
        let id = egui::Id::unique("oxbox-native-job-registry");
        if let Some(registry) = data.get_temp::<Registry>(id) {
            registry
        } else {
            let registry = Registry::default();
            data.insert_temp(id, registry.clone());
            registry
        }
    })
}
impl Registry {
    pub fn close(&self) {
        // Make queued registrations fail before waiting for their tiny mutex.
        self.0.closed.store(true, Ordering::Release);
        let registrations = {
            let mut jobs = self
                .0
                .jobs
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            std::mem::take(&mut *jobs)
        };
        for registration in registrations {
            if registration.lifetime.upgrade().is_some() {
                registration.cancel.cancel();
            }
        }
    }
    fn register(&self, cancel: CancelToken) -> Option<Arc<()>> {
        let mut jobs = self
            .0
            .jobs
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if self.0.closed.load(Ordering::Acquire) {
            cancel.cancel();
            return None;
        }
        jobs.retain(|job| job.lifetime.strong_count() > 0);
        let lifetime = Arc::new(());
        jobs.push(Registration {
            lifetime: Arc::downgrade(&lifetime),
            cancel,
        });
        Some(lifetime)
    }
}

pub struct Task<T: Send + 'static> {
    pub result: Option<T>,
    pub error: Option<String>,
    pub progress: Option<Progress>,
    receiver: Option<mpsc::Receiver<CoreResult<T>>>,
    worker: Option<JoinHandle<()>>,
    cancel: CancelToken,
    latest_progress: Arc<Mutex<Option<Progress>>>,
    discarded: bool,
}
impl<T: Send + 'static> Default for Task<T> {
    fn default() -> Self {
        Self {
            result: None,
            error: None,
            progress: None,
            receiver: None,
            worker: None,
            cancel: CancelToken::default(),
            latest_progress: Arc::new(Mutex::new(None)),
            discarded: false,
        }
    }
}
impl<T: Send + 'static> Task<T> {
    pub fn start(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce(CancelToken, &mut dyn FnMut(Progress)) -> CoreResult<T> + Send + 'static,
    ) -> bool {
        self.poll();
        if self.running() {
            return false;
        }
        self.result = None;
        self.error = None;
        self.progress = None;
        self.discarded = false;
        self.cancel = CancelToken::default();
        let Some(lifetime) = register_context(ctx).register(self.cancel.clone()) else {
            self.error = Some("Oxbox is quitting; new work was not started.".into());
            return false;
        };
        let cancel = self.cancel.clone();
        let ctx = ctx.clone();
        // Finished results have a reliable slot separate from latest progress.
        // No number of progress reports can block completion or grow a queue.
        let (sender, receiver) = mpsc::sync_channel(1);
        self.latest_progress = Arc::new(Mutex::new(None));
        let latest_progress = self.latest_progress.clone();
        match std::thread::Builder::new()
            .name("oxbox-tool-job".into())
            .spawn(move || {
                // The lease ends even if work panics; the registry holds only Weak.
                let _lifetime = lifetime;
                let repaint = ctx.clone();
                let mut last = std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(1))
                    .unwrap_or_else(std::time::Instant::now);
                let mut progress = move |value: Progress| {
                    let complete = value.done == value.total;
                    if let Ok(mut latest) = latest_progress.lock() {
                        *latest = Some(value);
                    }
                    if last.elapsed() >= std::time::Duration::from_millis(100) || complete {
                        last = std::time::Instant::now();
                        repaint.request_repaint();
                    }
                };
                let result = cancel
                    .check()
                    .and_then(|()| work(cancel.clone(), &mut progress))
                    .and_then(|value| {
                        cancel.check()?;
                        Ok(value)
                    });
                let _ = sender.send(result);
                ctx.request_repaint();
            }) {
            Ok(worker) => {
                self.receiver = Some(receiver);
                self.worker = Some(worker);
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                false
            }
        }
    }
    pub fn poll(&mut self) {
        if let Ok(mut latest) = self.latest_progress.lock()
            && let Some(value) = latest.take()
            && !self.discarded
        {
            self.progress = Some(value);
        }
        let mut finished = false;
        if let Some(receiver) = &self.receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    if !self.discarded {
                        match result {
                            Ok(value) => self.result = Some(value),
                            Err(error) => self.error = Some(error.to_string()),
                        }
                    }
                    finished = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    if !self.discarded {
                        self.error = Some(
                            "The worker stopped unexpectedly; original inputs were preserved"
                                .into(),
                        );
                    }
                    finished = true;
                }
            }
        }
        if finished {
            self.receiver = None;
            if let Some(worker) = self.worker.take()
                && worker.join().is_err()
                && !self.discarded
            {
                self.result = None;
                self.error =
                    Some("The worker stopped unexpectedly; original inputs were preserved".into());
            }
            // Joining establishes that this is the worker's final progress,
            // even if its last callback raced our first read above.
            if let Ok(mut latest) = self.latest_progress.lock()
                && let Some(value) = latest.take()
                && !self.discarded
            {
                self.progress = Some(value);
            }
        }
    }
    pub fn running(&self) -> bool {
        self.worker.is_some()
    }
    pub fn cancel(&self) {
        self.cancel.cancel();
    }
    /// Invalidate a view's work, including an already-enqueued successful result.
    /// Ownership is retained until the worker joins. Reuse starts a fresh token.
    pub fn abandon(&mut self) {
        self.cancel();
        self.discarded = true;
        self.result = None;
        self.error = None;
        self.progress = None;
        if let Ok(mut latest) = self.latest_progress.lock() {
            *latest = None;
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.show_with_announcement(ui, true);
    }
    /// Passive polling must not recreate a live announcement on every sample.
    pub fn show_quiet(&mut self, ui: &mut egui::Ui) {
        self.show_with_announcement(ui, false);
    }
    fn show_with_announcement(&mut self, ui: &mut egui::Ui, announce: bool) {
        self.poll();
        if self.running() {
            ui.horizontal(|ui| {
                ui.spinner();
                if announce {
                    crate::accessibility::status(ui, "Working…");
                } else {
                    ui.label("Working…");
                }
                if ui.button("Cancel").clicked() {
                    self.cancel();
                }
            });
            if let Some(progress) = &self.progress {
                let fraction = if progress.total > 0 {
                    progress.done as f32 / progress.total as f32
                } else {
                    0.0
                };
                ui.add(egui::ProgressBar::new(fraction).text(&progress.message));
            }
        } else if announce && self.result.is_some() {
            crate::accessibility::status(ui, "Completed");
        }
        if let Some(error) = &self.error {
            if announce {
                crate::accessibility::alert(ui, error);
            } else {
                ui.label(format!("Error: {error}"));
            }
        }
    }
    pub fn shutdown(&mut self) {
        self.abandon();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.receiver = None;
        if let Ok(mut latest) = self.latest_progress.lock() {
            *latest = None;
        }
    }
}
impl<T: Send + 'static> Drop for Task<T> {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_refresh_avoids_live_working_announcements() {
        for quiet in [false, true] {
            let context = egui::Context::default();
            context.enable_accesskit();
            let (release, wait) = mpsc::sync_channel(1);
            let mut task = Task::default();
            assert!(task.start(&context, move |_, _| {
                let _ = wait.recv_timeout(std::time::Duration::from_secs(5));
                Ok(())
            }));
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                if quiet {
                    task.show_quiet(ui);
                } else {
                    task.show(ui);
                }
            });
            output.textures_delta.clear();
            let live_working = output
                .platform_output
                .accesskit_update
                .unwrap()
                .nodes
                .iter()
                .any(|(_, node)| {
                    node.label() == Some("Working…")
                        && node.live() == Some(egui::accesskit::Live::Polite)
                });
            let _ = release.send(());
            task.shutdown();
            assert_eq!(live_working, !quiet);
        }
    }
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    fn finish<T: Send + 'static>(task: &mut Task<T>) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while task.running() && Instant::now() < deadline {
            task.poll();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!task.running(), "synthetic worker did not stop");
    }
    #[test]
    fn registry_close_cancels_a_running_job_without_a_ui_poll() {
        let ctx = egui::Context::default();
        let registry = register_context(&ctx);
        let mut task: Task<()> = Task::default();
        let observed_cancel = Arc::new(AtomicBool::new(false));
        let observed = observed_cancel.clone();
        let (entered, started) = std::sync::mpsc::sync_channel(1);
        assert!(task.start(&ctx, move |cancel, _| {
            entered.send(()).unwrap();
            while cancel.check().is_ok() {
                std::thread::sleep(Duration::from_millis(1));
            }
            observed.store(true, Ordering::Release);
            Err(oxbox_core::CoreError::Cancelled)
        }));
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        registry.close();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !observed_cancel.load(Ordering::Acquire) && Instant::now() < deadline {
            std::thread::yield_now();
        }
        let cancelled_without_poll = observed_cancel.load(Ordering::Acquire);
        task.shutdown();
        assert!(
            cancelled_without_poll,
            "blocked UI prevented global cancellation"
        );
    }
    #[test]
    fn registry_close_rejects_late_starts_and_is_shared_by_context_clones() {
        let ctx = egui::Context::default();
        let clone = ctx.clone();
        register_context(&ctx).close();
        let mut task = Task::default();
        let invoked = Arc::new(AtomicBool::new(false));
        let called = invoked.clone();
        let accepted = task.start(&clone, move |_, _| {
            called.store(true, Ordering::Release);
            Ok(())
        });
        finish(&mut task);
        assert!(!accepted, "a closed application accepted new work");
        assert!(!invoked.load(Ordering::Acquire));
    }
    #[test]
    fn registry_prunes_finished_jobs_instead_of_retaining_all_past_tokens() {
        let ctx = egui::Context::default();
        let registry = register_context(&ctx);
        let mut task = Task::default();
        for _ in 0..100 {
            assert!(task.start(&ctx, |_, _| Ok(())));
            finish(&mut task);
            assert_eq!(registry.0.jobs.lock().unwrap().len(), 1);
        }
        registry.close();
        assert!(registry.0.jobs.lock().unwrap().is_empty());
    }
    #[test]
    fn abandoned_completed_result_never_reappears_in_a_reused_view() {
        let ctx = egui::Context::default();
        let mut task = Task::default();
        assert!(task.start(&ctx, |_, _| Ok(7)));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !task.worker.as_ref().unwrap().is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        task.abandon();
        finish(&mut task);
        assert!(task.result.is_none() && task.error.is_none() && task.progress.is_none());
        assert!(task.start(&ctx, |_, _| Ok(11)));
        finish(&mut task);
        assert_eq!(task.result, Some(11));
    }
    #[test]
    fn hidden_progress_has_only_one_latest_value_and_never_blocks_completion() {
        let ctx = egui::Context::default();
        let mut task = Task::default();
        assert!(task.start(&ctx, |_, progress| {
            for done in 0..10000 {
                progress(Progress {
                    done,
                    total: 10000,
                    message: format!("step {done}"),
                });
            }
            Ok(1)
        }));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !task.worker.as_ref().unwrap().is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(
            task.latest_progress.lock().unwrap().as_ref().unwrap().done,
            9999
        );
        finish(&mut task);
        assert_eq!(task.result, Some(1));
        assert_eq!(task.progress.as_ref().unwrap().done, 9999);
    }
    #[test]
    fn shutdown_cancels_and_joins_its_owned_worker() {
        let ctx = egui::Context::default();
        let mut task: Task<()> = Task::default();
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        let (entered, started) = mpsc::sync_channel(1);
        assert!(task.start(&ctx, move |cancel, _| {
            entered.send(()).unwrap();
            while cancel.check().is_ok() {
                std::thread::sleep(Duration::from_millis(1));
            }
            flag.store(true, Ordering::Release);
            Err(oxbox_core::CoreError::Cancelled)
        }));
        // Cancellation may now correctly refuse work before invoking its closure.
        // This regression specifically exercises cleanup of an already-running job.
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        task.shutdown();
        assert!(stopped.load(Ordering::Acquire));
        assert!(!task.running());
    }
    #[test]
    fn panicking_worker_is_joined_and_reported_without_a_ui_panic() {
        let ctx = egui::Context::default();
        let mut task: Task<()> = Task::default();
        assert!(task.start(&ctx, |_, _| panic!("synthetic worker failure")));
        finish(&mut task);
        assert!(task.result.is_none());
        assert!(
            task.error
                .as_ref()
                .is_some_and(|error| error.contains("worker"))
        );
    }
}
