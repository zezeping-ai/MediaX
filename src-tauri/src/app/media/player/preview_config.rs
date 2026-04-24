// 预览参数调优指南：
// - 弱网或 CPU 压力较大：优先下调 PREVIEW_MAX_BYTES、PREVIEW_TIMEOUT_MS、
//   PREVIEW_TARGET_WIDTH/HEIGHT、PREVIEW_INITIAL_QUALITY。
// - 清晰度优先：可提升 PREVIEW_TARGET_WIDTH/HEIGHT 与 PREVIEW_INITIAL_QUALITY，
//   但建议保守控制 PREVIEW_MAX_BYTES，避免悬浮预览卡顿。
// - 若过早进入降级策略：先小幅提高 PREVIEW_TIMEOUT_MS，
//   再考虑放宽 PREVIEW_FALLBACK_* 阈值。
pub(crate) const PREVIEW_MAX_BYTES: usize = 32 * 1024;
pub(crate) const PREVIEW_TIMEOUT_MS: u64 = 300;
pub(crate) const PREVIEW_RENDER_TIMEOUT_MS: u64 = 1200;

pub(crate) const PREVIEW_MIN_WIDTH: u32 = 64;
pub(crate) const PREVIEW_MAX_WIDTH: u32 = 192;
pub(crate) const PREVIEW_MIN_HEIGHT: u32 = 36;
pub(crate) const PREVIEW_MAX_HEIGHT: u32 = 108;

pub(crate) const PREVIEW_TARGET_WIDTH: u32 = 160;
pub(crate) const PREVIEW_TARGET_HEIGHT: u32 = 90;

pub(crate) const PREVIEW_INITIAL_QUALITY: u8 = 42;
pub(crate) const PREVIEW_MIN_QUALITY: u8 = 28;
pub(crate) const PREVIEW_QUALITY_STEP: u8 = 6;
pub(crate) const PREVIEW_FALLBACK_MIN_WIDTH: u32 = 96;
pub(crate) const PREVIEW_FALLBACK_MIN_HEIGHT: u32 = 54;
pub(crate) const PREVIEW_FALLBACK_SCALE_NUM: u32 = 4;
pub(crate) const PREVIEW_FALLBACK_SCALE_DEN: u32 = 5;
