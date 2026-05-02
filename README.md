# MEDIAX

## MacOS安装打不开
```bash
$ sudo xattr -rd com.apple.quarantine /Applications/sanmuyi.app
```

## 支持的打开协议

安装后的 `MediaX` 支持通过自定义协议 `mediax://` 从浏览器或其他应用唤起播放器。

推荐格式：

```text
mediax://open?url=https%3A%2F%2Fexample.com%2Fvideo.mp4
mediax://play?url=https%3A%2F%2Fexample.com%2Flive.m3u8
mediax://open?path=%2FUsers%2Fwangsen%2FDownloads%2Fdemo.mp4
mediax://open?url=file%3A%2F%2F%2FUsers%2Fwangsen%2FDownloads%2Fdemo.mp4
```

说明：

- `open` 和 `play` 当前都会直接打开并开始播放
- 支持参数：
  - `url`：远程媒体地址，或 `file:///` 本地文件 URL
  - `path`：本地文件绝对路径
- 参数值建议始终做 URL 编码
- `mediax://` 协议注册依赖正式安装包，开发态 `tauri dev` 不代表系统已注册成功

浏览器示例：

```html
<a href="mediax://open?url=https%3A%2F%2Fexample.com%2Fmovie.mp4">用 MediaX 打开视频</a>
```
