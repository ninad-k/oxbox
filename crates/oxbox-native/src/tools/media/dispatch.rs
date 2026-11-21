//! Recording admission survives modal UI stalls and orders Stop before queued Start.
use oxbox_core::{CoreError, CoreResult, jobs::CancelToken};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
struct State {
    gate: Mutex<()>,
    generation: AtomicU64,
}
#[derive(Clone, Default)]
pub(super) struct RecordingDispatch {
    inner: Arc<State>,
}
impl RecordingDispatch {
    /// Snapshot at the initiating UI click, before its confirmation modal opens.
    pub fn reserve_start(&self) -> u64 {
        self.inner.generation.load(Ordering::Acquire)
    }
    /// Fast callback-entry barrier; never waits for a provider or UI job.
    pub fn invalidate_pending(&self) -> u64 {
        self.inner
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1)
    }
    pub fn start<T>(
        &self,
        ticket: u64,
        cancel: &CancelToken,
        quit: &CancelToken,
        work: impl FnOnce() -> CoreResult<T>,
    ) -> CoreResult<T> {
        let _gate = self
            .inner
            .gate
            .lock()
            .map_err(|_| CoreError::InvalidInput("recording state is unavailable".into()))?;
        cancel.check()?;
        quit.check()?;
        if ticket != self.inner.generation.load(Ordering::Acquire) {
            return Err(CoreError::Cancelled);
        }
        work()
    }
    pub fn stop<T>(&self, work: impl FnOnce() -> CoreResult<T>) -> CoreResult<T> {
        let _gate = self
            .inner
            .gate
            .lock()
            .map_err(|_| CoreError::InvalidInput("recording state is unavailable".into()))?;
        work()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxbox_core::jobs::CancelToken;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::time::Duration;

    #[test]
    fn stop_during_confirmation_rejects_the_eventual_yes_without_a_provider() {
        let dispatch = RecordingDispatch::default();
        let ticket = dispatch.reserve_start();
        dispatch.invalidate_pending();
        let called = AtomicBool::new(false);
        let result = dispatch.start(
            ticket,
            &CancelToken::default(),
            &CancelToken::default(),
            || {
                called.store(true, Ordering::Release);
                Ok(())
            },
        );
        assert_eq!(result.unwrap_err().code(), "cancelled");
        assert!(!called.load(Ordering::Acquire));
    }

    #[test]
    fn stop_callback_invalidates_a_start_already_waiting_for_the_gate() {
        let dispatch = RecordingDispatch::default();
        let ticket = dispatch.reserve_start();
        let held = dispatch.inner.gate.lock().unwrap();
        let worker = dispatch.clone();
        let called = Arc::new(AtomicBool::new(false));
        let flag = called.clone();
        let (queued, waiting) = mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            queued.send(()).unwrap();
            worker.start(
                ticket,
                &CancelToken::default(),
                &CancelToken::default(),
                || {
                    flag.store(true, Ordering::Release);
                    Ok(())
                },
            )
        });
        waiting.recv_timeout(Duration::from_secs(5)).unwrap();
        dispatch.invalidate_pending();
        drop(held);
        assert_eq!(thread.join().unwrap().unwrap_err().code(), "cancelled");
        assert!(!called.load(Ordering::Acquire));
    }

    #[test]
    fn recovery_stop_executes_without_another_ui_tick() {
        let dispatch = RecordingDispatch::default();
        let ticket = dispatch.reserve_start();
        dispatch.invalidate_pending();
        let worker = dispatch.clone();
        let active = Arc::new(AtomicBool::new(true));
        let state = active.clone();
        let (stopped, completion) = mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            worker.stop(|| {
                state.store(false, Ordering::Release);
                stopped.send(()).unwrap();
                Ok(())
            })
        });
        // The UI is deliberately not pumped while the recovery worker acts.
        completion.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!active.load(Ordering::Acquire));
        thread.join().unwrap().unwrap();
        assert_eq!(
            dispatch
                .start(
                    ticket,
                    &CancelToken::default(),
                    &CancelToken::default(),
                    || Ok(())
                )
                .unwrap_err()
                .code(),
            "cancelled"
        );
        let fresh = dispatch.reserve_start();
        assert!(
            dispatch
                .start(
                    fresh,
                    &CancelToken::default(),
                    &CancelToken::default(),
                    || Ok(())
                )
                .is_ok()
        );
    }
}
