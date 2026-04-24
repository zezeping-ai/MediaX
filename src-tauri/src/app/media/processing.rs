//! 媒体处理领域（转码 / 裁剪 / 拼接）的后端入口预留。
//! 当前播放器链路与处理链路严格分离，避免命令与状态耦合。
//!
//! 后续在此模块新增：
//! - transcode commands/services
//! - trim commands/services
//! - concat commands/services
//! 并通过独立事件通道上报任务进度。
