//! Cancellation and progress shared by long-running tools.

use crate::{CoreError, CoreResult};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn check(&self) -> CoreResult<()> {
        if self.0.load(Ordering::Relaxed) {
            Err(CoreError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: u64,
    pub total: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_a_clone_stops_the_worker() {
        let token = CancelToken::default();
        let worker = token.clone();
        assert!(worker.check().is_ok());
        token.cancel();
        assert_eq!(worker.check().unwrap_err().code(), "cancelled");
    }
}
