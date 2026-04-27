use std::sync::atomic::Ordering;

use crate::app::media::state::MediaState;

pub(super) fn begin_paused_seek_epoch(media: &MediaState) -> u32 {
    media.runtime.paused_seek_epoch.fetch_add(1, Ordering::Relaxed) + 1
}

pub(super) fn is_epoch_stale(media: &MediaState, epoch: u32) -> bool {
    media.runtime.paused_seek_epoch.load(Ordering::Relaxed) != epoch
}
