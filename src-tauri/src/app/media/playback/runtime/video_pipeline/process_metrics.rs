use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

const PROCESS_METRICS_SAMPLE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
struct ProcessMetricsSnapshot {
    cpu_percent: f32,
    memory_mb: f64,
}

pub(crate) struct ProcessMetricsSampler {
    system: System,
    pid: Pid,
    last_sampled_at: Option<std::time::Instant>,
    last_snapshot: Option<ProcessMetricsSnapshot>,
}

impl ProcessMetricsSampler {
    pub(crate) fn new() -> Self {
        let refresh = RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing());
        let mut sampler = Self {
            system: System::new_with_specifics(refresh),
            pid: Pid::from_u32(std::process::id()),
            last_sampled_at: None,
            last_snapshot: None,
        };
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        sampler.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sampler.pid]),
            true,
            refresh,
        );
        std::thread::sleep(Duration::from_millis(120));
        sampler.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[sampler.pid]),
            true,
            refresh,
        );
        sampler
    }

    pub(super) fn sample(&mut self) -> Option<(f32, f64)> {
        let now = std::time::Instant::now();
        if let (Some(last_sampled_at), Some(snapshot)) = (self.last_sampled_at, self.last_snapshot)
        {
            if now.saturating_duration_since(last_sampled_at) < PROCESS_METRICS_SAMPLE_INTERVAL {
                return Some((snapshot.cpu_percent, snapshot.memory_mb));
            }
        }
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[self.pid]),
            true,
            refresh,
        );
        let process = self.system.process(self.pid)?;
        let snapshot = ProcessMetricsSnapshot {
            cpu_percent: process.cpu_usage(),
            memory_mb: (process.memory() as f64) / (1024.0 * 1024.0),
        };
        self.last_sampled_at = Some(now);
        self.last_snapshot = Some(snapshot);
        Some((snapshot.cpu_percent, snapshot.memory_mb))
    }
}
