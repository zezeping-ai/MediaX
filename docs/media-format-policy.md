# Media Format Policy

`MediaX` 当前的打开主链路统一走 Rust + FFmpeg：

- 打开入口使用 `ffmpeg::format::input(source)`
- 视频解码使用 `codec::context::Context::from_parameters(...).decoder().video()`
- 音频解码使用 `codec::context::Context::from_parameters(...).decoder().audio()`
- 自适应流逻辑显式识别了 `m3u8` 和 `mpd`

因此，默认播放器文件关联的扩展名应优先覆盖：

- FFmpeg 常见可直开的本地容器
- FFmpeg 常见可直开的音频容器
- 当前业务逻辑已经显式识别的自适应流清单

## 当前策略

- `tauri.conf.json > bundle.fileAssociations` 中保留“按代码路径理论可播”的集合
- 不把明显依赖额外解析器、且项目中未见对应实现的格式默认宣称为支持
- “默认播放器关联”不等于“全部格式已实测”

## 目前明确不按默认支持宣称的类别

- 需要专门播放列表解析器的格式
  例如：`pls`、`xspf`、`cue`
- 纯字幕、纯元数据、纯工程文件
- 代码中没有特殊处理、且不能仅靠 `format::input(source)` 稳定落地的外围格式

## 维护约定

- 如果后续补了真实样本和播放链路验证，可以继续把格式加入默认关联
- 如果某个扩展名被系统频繁交给 `MediaX` 但实际不能播，应优先从默认关联中移除，而不是让系统层面误导用户
