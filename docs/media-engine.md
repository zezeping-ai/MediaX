# MediaX 播放内核选型

## 结论
- 首选内核：`mpv`。
- 选择原因：跨平台播放能力成熟、工程接入成本低于 gstreamer，适合作为首页播放器 MVP 的核心。

## 平台依赖清单
- macOS：`libmpv`（推荐通过 Homebrew 安装 `mpv`，由其提供动态库）。
- Windows：`mpv-2.dll` 与对应 FFmpeg 运行库需随应用分发。
- Linux：`libmpv.so` 及发行版对应的 FFmpeg 运行时库。

## 当前代码策略
- 后端播放状态机已固定 `engine = "mpv"`，并通过 `media://state` 事件向前端广播。
- MVP 阶段先完成媒体命令和状态流，后续在 `playback_core` 中补齐真实 mpv 绑定。

## 后续接入建议
- Rust 侧新增 `mpv` 适配层：初始化、loadfile、pause、seek、time-pos 订阅。
- 将 `media_sync_position` 从前端上报改为内核主动推送，减少前端与后端状态偏差。
