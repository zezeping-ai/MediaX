use super::StreamRuntimeState;
use std::sync::atomic::Ordering;

pub(super) fn next_restart_epoch(state: &StreamRuntimeState) -> u64 {
    state
        .restart_epoch
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1)
}

pub(super) fn is_restart_epoch_current(state: &StreamRuntimeState, epoch: u64) -> bool {
    state.restart_epoch.load(Ordering::Relaxed) == epoch
}

pub(super) fn advance_generation(state: &StreamRuntimeState) -> u32 {
    state
        .generation
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1)
}

pub(super) fn current_generation(state: &StreamRuntimeState) -> u32 {
    state.generation.load(Ordering::Relaxed)
}
