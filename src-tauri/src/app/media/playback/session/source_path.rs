use crate::app::media::error::{MediaError, MediaResult};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const SCAN_MAX_DEPTH: usize = 3;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "mov", "avi", "webm", "flv", "m4v", "wmv", "mpeg", "mpg", "ts", "m2ts", "mts",
    "mxf", "rm", "rmvb", "3gp", "3g2", "ogv", "asf", "vob", "f4v", "divx",
];

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "aac", "m4a", "ogg", "opus", "wma", "aif", "aiff", "ape", "alac", "amr",
    "ac3", "dts", "mp2", "mka",
];

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum MediaFileKind {
    Audio = 0,
    Video = 1,
}

struct MediaCandidate {
    kind: MediaFileKind,
    size_bytes: u64,
    path: PathBuf,
}

/// Normalize a playable source: remote URLs pass through; local paths are canonicalized.
pub fn normalize_playable_source(path: String) -> MediaResult<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(MediaError::invalid_input("媒体路径为空"));
    }
    if is_remote_source(trimmed) {
        return Ok(trimmed.to_string());
    }
    normalize_local_source_path(path)
}

/// Normalize paths from dialogs, drag-drop, or CLI into a canonical local path for FFmpeg.
pub fn normalize_local_source_path(path: String) -> MediaResult<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(MediaError::invalid_input("媒体路径为空"));
    }
    let path_buf = parse_local_path(trimmed)?;
    if !path_buf.exists() {
        return Err(MediaError::invalid_input(format!(
            "媒体路径不存在: {}",
            path_buf.display()
        )));
    }
    let resolved = resolve_playable_path(&path_buf)?;
    Ok(resolved
        .canonicalize()
        .unwrap_or(resolved)
        .to_string_lossy()
        .to_string())
}

/// Normalize a local image path for cover-art import (not playable media validation).
pub fn normalize_local_image_path(path: String) -> MediaResult<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(MediaError::invalid_input("图片路径为空"));
    }
    let path_buf = parse_local_path(trimmed)?;
    if !path_buf.is_file() {
        return Err(MediaError::invalid_input(format!(
            "图片路径不存在或不是文件: {}",
            path_buf.display()
        )));
    }
    if !is_supported_image_path(&path_buf) {
        return Err(MediaError::invalid_input(format!(
            "不支持的图片格式: {}",
            path_buf.display()
        )));
    }
    Ok(path_buf
        .canonicalize()
        .unwrap_or(path_buf)
        .to_string_lossy()
        .to_string())
}

fn is_supported_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.as_str()))
}

fn resolve_playable_path(path: &Path) -> MediaResult<PathBuf> {
    if path.is_file() {
        if media_kind_for_path(path).is_some() {
            return Ok(path.to_path_buf());
        }
        return Err(MediaError::invalid_input(format!(
            "不支持的媒体文件类型: {}",
            path.display()
        )));
    }
    if path.is_dir() {
        return resolve_media_inside_directory(path);
    }
    Err(MediaError::invalid_input(format!(
        "媒体路径既不是文件也不是文件夹: {}",
        path.display()
    )))
}

/// Some release folders are named like `movie.mp4` but are directories; pick the main media file inside.
fn resolve_media_inside_directory(dir: &Path) -> MediaResult<PathBuf> {
    let Some(best) = find_best_media_candidate(dir) else {
        return Err(MediaError::invalid_input(format!(
            "所选路径是文件夹，且其中没有可播放的媒体文件: {}",
            dir.display()
        )));
    };
    Ok(best.path)
}

fn find_best_media_candidate(root: &Path) -> Option<MediaCandidate> {
    let mut best: Option<MediaCandidate> = None;
    for entry in WalkDir::new(root)
        .min_depth(1)
        .max_depth(SCAN_MAX_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(kind) = media_kind_for_path(path) else {
            continue;
        };
        let size_bytes = path.metadata().ok().map(|meta| meta.len()).unwrap_or(0);
        let candidate = MediaCandidate {
            kind,
            size_bytes,
            path: path.to_path_buf(),
        };
        best = Some(select_preferred(best, candidate));
    }
    best
}

fn select_preferred(current: Option<MediaCandidate>, next: MediaCandidate) -> MediaCandidate {
    let Some(current) = current else {
        return next;
    };
    if next.kind > current.kind {
        return next;
    }
    if next.kind < current.kind {
        return current;
    }
    if next.size_bytes > current.size_bytes {
        return next;
    }
    current
}

fn media_kind_for_path(path: &Path) -> Option<MediaFileKind> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        return Some(MediaFileKind::Video);
    }
    if AUDIO_EXTENSIONS.contains(&extension.as_str()) {
        return Some(MediaFileKind::Audio);
    }
    None
}

fn is_remote_source(source: &str) -> bool {
    matches!(
        source.split(':').next().map(|value| value.to_ascii_lowercase()),
        Some(scheme)
            if matches!(
                scheme.as_str(),
                "http" | "https" | "rtsp" | "rtmp" | "mms"
            )
    )
}

fn parse_local_path(raw: &str) -> MediaResult<PathBuf> {
    if let Some(file_url) = raw.strip_prefix("file://") {
        return decode_file_url_path(file_url).map_err(MediaError::invalid_input);
    }
    Ok(PathBuf::from(raw))
}

fn decode_file_url_path(file_url: &str) -> Result<PathBuf, String> {
    let without_host = file_url.strip_prefix("//").unwrap_or(file_url);
    let decoded = urlencoding_decode(without_host);
    let path = Path::new(&decoded);
    if path.as_os_str().is_empty() {
        return Err("file URL does not contain a path".to_string());
    }
    Ok(path.to_path_buf())
}

fn urlencoding_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                16,
            ) {
                out.push(byte as char);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn accepts_local_jpeg_for_cover_import() {
        let root = std::env::temp_dir().join(format!(
            "mediax-cover-image-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let image = root.join("cover.jpeg");
        fs::write(&image, b"fake-jpeg").expect("jpeg");

        let resolved = normalize_local_image_path(image.to_string_lossy().to_string())
            .expect("normalize jpeg");
        assert!(resolved.ends_with("cover.jpeg"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_non_image_for_cover_import() {
        let root = std::env::temp_dir().join(format!(
            "mediax-cover-reject-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let audio = root.join("track.flac");
        fs::write(&audio, b"fake-flac").expect("flac");

        let err = normalize_local_image_path(audio.to_string_lossy().to_string())
            .expect_err("reject flac");
        assert!(err.to_string().contains("不支持的图片格式"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remote_url_passes_through_unchanged() {
        let url = "https://example.com/media/demo.mp4";
        let resolved = normalize_playable_source(url.to_string()).expect("remote url");
        assert_eq!(resolved, url);
    }

    #[test]
    fn resolves_video_inside_fake_mp4_folder() {
        let root = std::env::temp_dir().join(format!(
            "mediax-source-path-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let bundle = root.join("movie.mp4");
        fs::create_dir_all(&bundle).expect("bundle dir");
        let real_video = bundle.join("feature.mp4");
        fs::write(&real_video, b"fake-video").expect("video file");
        fs::write(bundle.join("readme.txt"), b"nope").expect("readme");

        let resolved = resolve_playable_path(&bundle).expect("resolve bundle");
        assert_eq!(resolved, real_video);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn prefers_largest_video_in_folder() {
        let root = std::env::temp_dir().join(format!(
            "mediax-source-pick-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("root");
        let small = root.join("clip.mp4");
        let large = root.join("main.mkv");
        let mut small_file = fs::File::create(&small).expect("small");
        let mut large_file = fs::File::create(&large).expect("large");
        small_file.write_all(&[0u8; 32]).expect("small bytes");
        large_file.write_all(&[0u8; 4096]).expect("large bytes");

        let resolved = resolve_playable_path(&root).expect("resolve folder");
        assert_eq!(resolved, large);

        let _ = fs::remove_dir_all(root);
    }
}
