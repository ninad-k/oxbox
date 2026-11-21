//! Managed cancellation registry and throttled progress events.
use crate::error::ToolError;
use oxbox_core::{
    CoreError, CoreResult,
    jobs::{CancelToken, Progress},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct Registry {
    active: HashMap<String, CancelToken>,
    early: HashSet<String>,
    early_order: VecDeque<String>,
    stopping: bool,
}

impl Registry {
    fn register(&mut self, job_id: &str) -> CoreResult<CancelToken> {
        if self.stopping {
            return Err(CoreError::Cancelled);
        }
        if self.active.contains_key(job_id) || self.active.len() >= 8 {
            return Err(CoreError::InvalidInput(
                "job identifier is already active or too many jobs are running".into(),
            ));
        }
        let cancel = CancelToken::default();
        if self.early.remove(job_id) {
            cancel.cancel();
        }
        self.active.insert(job_id.to_owned(), cancel.clone());
        Ok(cancel)
    }
}

#[derive(Default)]
pub struct JobState(Mutex<Registry>);

impl JobState {
    pub fn cancel_all(&self) {
        // A failed worker must not prevent shutdown from closing registration.
        let mut registry = match self.0.lock() {
            Ok(registry) => registry,
            Err(poisoned) => poisoned.into_inner(),
        };
        registry.stopping = true;
        for token in registry.active.values() {
            token.cancel();
        }
    }
}

fn validate_id(id: &str) -> CoreResult<()> {
    if id.is_empty()
        || id.len() > 80
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(CoreError::InvalidInput("invalid job identifier".into()));
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_job(job_id: String, state: State<'_, JobState>) -> Result<(), ToolError> {
    validate_id(&job_id)?;
    let mut registry = state
        .inner()
        .0
        .lock()
        .map_err(|_| ToolError::internal("job registry is unavailable".into()))?;
    if let Some(token) = registry.active.get(&job_id) {
        token.cancel();
    } else if registry.early.insert(job_id.clone()) {
        registry.early_order.push_back(job_id);
        while registry.early_order.len() > 256 {
            if let Some(old) = registry.early_order.pop_front() {
                registry.early.remove(&old);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JobProgress {
    job_id: String,
    #[serde(flatten)]
    progress: Progress,
}

pub async fn run_job<T, F>(
    app: tauri::AppHandle,
    job_id: String,
    tool_id: &'static str,
    work: F,
) -> Result<T, ToolError>
where
    T: Send + 'static,
    F: FnOnce(CancelToken, &mut dyn FnMut(Progress)) -> CoreResult<T> + Send + 'static,
{
    validate_id(&job_id)?;
    let cancel = {
        let state = app.state::<JobState>();
        let mut registry = state
            .inner()
            .0
            .lock()
            .map_err(|_| ToolError::internal("job registry is unavailable".into()))?;
        registry.register(&job_id)?
    };
    let worker_app = app.clone();
    let worker_id = job_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut last = None;
        work(cancel, &mut |progress| {
            let now = Instant::now();
            if last.is_none_or(|t: Instant| now.duration_since(t) >= Duration::from_millis(100)) {
                let _ = worker_app.emit(
                    &format!("tool://{tool_id}/progress"),
                    JobProgress {
                        job_id: worker_id.clone(),
                        progress,
                    },
                );
                last = Some(now);
            }
        })
    })
    .await;
    if let Ok(mut registry) = app.state::<JobState>().inner().0.lock() {
        registry.active.remove(&job_id);
    }
    result
        .map_err(|e| ToolError::internal(e.to_string()))?
        .map_err(ToolError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_cancels_active_jobs_and_refuses_late_registration() {
        let state = JobState::default();
        let active = state.0.lock().unwrap().register("active").unwrap();
        state.cancel_all();
        assert!(matches!(active.check(), Err(CoreError::Cancelled)));
        assert!(matches!(
            state.0.lock().unwrap().register("late"),
            Err(CoreError::Cancelled)
        ));
    }
}
