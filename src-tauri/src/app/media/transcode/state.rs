use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscodeJobKind {
    Video,
    Audio,
    ImageLossless,
    ImageLossy,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscodeJobStatus {
    Queued,
    Running,
    Success,
    Skipped,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscodeJob {
    pub id: u64,
    pub kind: TranscodeJobKind,
    pub source_path: String,
    pub output_path: String,
    pub status: TranscodeJobStatus,
    pub progress_percent: f64,
    pub started_at_ms: Option<u64>,
    pub finished_at_ms: Option<u64>,
    pub error_message: Option<String>,
    pub input_size_bytes: Option<u64>,
    pub output_size_bytes: Option<u64>,
    pub estimated_output_size_bytes: Option<u64>,
    pub quality: Option<u8>,
    pub format: Option<String>,
    pub resolution: Option<String>,
    pub playback_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscodeQueueSnapshot {
    pub jobs: Vec<TranscodeJob>,
    pub running_jobs: usize,
    pub queued_jobs: usize,
}

#[derive(Default)]
pub struct TranscodeState {
    next_id: AtomicU64,
    inner: Arc<Mutex<TranscodeStateInner>>,
}

#[derive(Default)]
pub struct TranscodeStateInner {
    pub jobs: HashMap<u64, TranscodeJob>,
    pub order: Vec<u64>,
    pub pending: VecDeque<u64>,
    pub running: Vec<u64>,
    pub canceled: Vec<u64>,
}

impl TranscodeState {
    pub const MAX_CONCURRENT_JOBS: usize = 3;

    pub fn next_job_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn with_inner<T>(&self, f: impl FnOnce(&mut TranscodeStateInner) -> T) -> T {
        let mut guard = self.inner.lock().expect("transcode state poisoned");
        f(&mut guard)
    }

    pub fn snapshot(&self) -> TranscodeQueueSnapshot {
        self.with_inner(|inner| {
            let mut jobs = Vec::with_capacity(inner.order.len());
            for id in &inner.order {
                if let Some(job) = inner.jobs.get(id) {
                    jobs.push(job.clone());
                }
            }
            TranscodeQueueSnapshot {
                running_jobs: inner.running.len(),
                queued_jobs: inner.pending.len(),
                jobs,
            }
        })
    }
}
