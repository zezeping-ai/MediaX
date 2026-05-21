use super::StreamRuntimeState;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const ORPHAN_DRAIN_SLICE_MS: u64 = 200;

impl StreamRuntimeState {
    pub(in crate::app::media) fn register_orphan(&self, handle: JoinHandle<()>) {
        let mut orphans = self
            .orphan_threads
            .lock()
            .expect("orphan decode thread registry poisoned");
        orphans.retain(|entry| !entry.is_finished());
        orphans.push(handle);
    }
}

/// Wait for decode threads that outlived a join timeout before starting a new demux loop.
pub(super) fn wait_orphans_drained(
    state: &StreamRuntimeState,
    total_timeout_ms: u64,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_millis(total_timeout_ms);
    loop {
        let mut orphans = state
            .orphan_threads
            .lock()
            .expect("orphan decode thread registry poisoned");
        orphans.retain(|entry| !entry.is_finished());
        if orphans.is_empty() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            let remaining = orphans.len();
            drop(orphans);
            return Err(format!(
                "orphan decode threads still running after {total_timeout_ms}ms (remaining={remaining})"
            ));
        }
        let handle = orphans.pop().expect("orphan list checked non-empty");
        drop(orphans);
        match try_join_for(handle, ORPHAN_DRAIN_SLICE_MS) {
            Ok(()) => {}
            Err(pending) => state.register_orphan(pending),
        }
        std::thread::sleep(Duration::from_millis(ORPHAN_DRAIN_SLICE_MS));
    }
}

fn try_join_for(handle: JoinHandle<()>, timeout_ms: u64) -> Result<(), JoinHandle<()>> {
    use std::sync::mpsc;
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let join_worker = std::thread::spawn(move || {
        let _ = handle.join();
        let _ = done_tx.send(());
    });
    if done_rx
        .recv_timeout(Duration::from_millis(timeout_ms))
        .is_ok()
    {
        let _ = join_worker.join();
        return Ok(());
    }
    Err(join_worker)
}
