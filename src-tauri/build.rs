fn main() {
    if cfg!(target_os = "windows") {
        // Prefer static FFmpeg libraries from vcpkg on Windows.
        let _ = vcpkg::Config::new().find_package("ffmpeg");
        for lib in ["avcodec", "avformat", "avutil", "swresample", "swscale"] {
            println!("cargo:rustc-link-lib=static={lib}");
        }
    } else if cfg!(target_os = "macos") {
        // Resolve FFmpeg libs from Homebrew via pkg-config on macOS.
        for lib in ["libavcodec", "libavformat", "libavutil", "libswresample", "libswscale"] {
            let _ = pkg_config::Config::new().probe(lib);
        }
    }

    tauri_build::build()
}
