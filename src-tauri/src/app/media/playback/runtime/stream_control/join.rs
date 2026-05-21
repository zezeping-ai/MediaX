use crate::app::media::state::StreamRuntimeState;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const DECODE_JOIN_TIMEOUT_MS: u64 = 1200;

pub(super) fn join_decode_thread_with_timeout(
    stream: &StreamRuntimeState,
    handle: thread::JoinHandle<()>,
) -> Result<(), String> {
    let timeout = Duration::from_millis(DECODE_JOIN_TIMEOUT_MS);
    let timeout_ms = timeout.as_millis();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let join_worker = thread::spawn(move || {
        let _ = handle.join();
        let _ = done_tx.send(());
    });
    if done_rx.recv_timeout(timeout).is_ok() {
        let _ = join_worker.join();
        return Ok(());
    }
    stream.register_orphan(join_worker);
    Err(format!("decode thread join timeout after {timeout_ms}ms"))
}
