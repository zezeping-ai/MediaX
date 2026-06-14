use std::io::Cursor;
use std::path::Path;

use base64::Engine as _;
use ffmpeg_next::format;
use ffmpeg_next::format::stream::Disposition;
use ffmpeg_next::media::Type;
use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::Tag;

const MAX_COVER_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct AudioCoverArtData {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverArtChange {
    None,
    Replace,
    Remove,
}

impl CoverArtChange {
    pub fn parse(value: Option<&str>) -> Self {
        match value.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("replace") => Self::Replace,
            Some("remove") => Self::Remove,
            _ => Self::None,
        }
    }
}

pub fn read_audio_cover_art(path: &str) -> Result<Option<AudioCoverArtData>, String> {
    if let Some(cover) = read_cover_from_tags(path)? {
        return Ok(Some(cover));
    }
    read_cover_from_attached_pic(path)
}

pub fn read_image_file_for_cover(path: &str) -> Result<AudioCoverArtData, String> {
    let path_buf = Path::new(path);
    if !path_buf.is_file() {
        return Err(format!("图片文件不存在: {path}"));
    }
    let bytes = std::fs::read(path_buf).map_err(|err| format!("读取图片失败: {err}"))?;
    validate_cover_bytes(&bytes)?;
    Ok(AudioCoverArtData {
        mime_type: mime_type_from_bytes(&bytes),
        bytes,
    })
}

pub fn decode_cover_art_base64(data_base64: &str, mime_type: Option<&str>) -> Result<Vec<u8>, String> {
    let trimmed = data_base64.trim();
    if trimmed.is_empty() {
        return Err("封面数据为空".to_string());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|err| format!("封面数据解码失败: {err}"))?;
    validate_cover_bytes(&bytes)?;
    if let Some(mime) = mime_type {
        let expected = mime_type_from_bytes(&bytes);
        if !mime.eq_ignore_ascii_case(&expected) && mime != "application/octet-stream" {
            return Err(format!("封面格式不匹配，期望 {expected}"));
        }
    }
    Ok(bytes)
}

pub fn apply_cover_art_change(
    tag: &mut Tag,
    change: CoverArtChange,
    cover_bytes: Option<&[u8]>,
) -> Result<(), String> {
    match change {
        CoverArtChange::None => Ok(()),
        CoverArtChange::Remove => {
            while !tag.pictures().is_empty() {
                tag.remove_picture(0);
            }
            Ok(())
        }
        CoverArtChange::Replace => {
            let bytes = cover_bytes.ok_or_else(|| "缺少封面图片数据".to_string())?;
            validate_cover_bytes(bytes)?;
            let mut picture = Picture::from_reader(&mut Cursor::new(bytes))
                .map_err(|err| format!("封面图片无效: {err}"))?;
            picture.set_pic_type(PictureType::CoverFront);
            tag.remove_picture_type(PictureType::CoverFront);
            tag.push_picture(picture);
            Ok(())
        }
    }
}

fn read_cover_from_tags(path: &str) -> Result<Option<AudioCoverArtData>, String> {
    let tagged_file = Probe::open(path)
        .map_err(|err| format!("打开文件失败: {err}"))?
        .guess_file_type()
        .map_err(|err| format!("识别文件格式失败: {err}"))?
        .read()
        .map_err(|err| format!("读取标签失败: {err}"))?;
    let Some(tag) = tagged_file.primary_tag() else {
        return Ok(None);
    };
    let picture = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first());
    let Some(picture) = picture else {
        return Ok(None);
    };
    let bytes = picture.data().to_vec();
    if bytes.is_empty() {
        return Ok(None);
    }
    validate_cover_bytes(&bytes)?;
    Ok(Some(AudioCoverArtData {
        mime_type: picture
            .mime_type()
            .map(MimeType::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| mime_type_from_bytes(&bytes)),
        bytes,
    }))
}

fn read_cover_from_attached_pic(path: &str) -> Result<Option<AudioCoverArtData>, String> {
    ffmpeg_next::init().map_err(|err| format!("ffmpeg init failed: {err}"))?;
    let input_ctx = format::input(path).map_err(|err| format!("open media failed: {err}"))?;
    let stream_index = input_ctx
        .streams()
        .find(|stream| {
            stream.parameters().medium() == Type::Video
                && stream.disposition().contains(Disposition::ATTACHED_PIC)
        })
        .map(|stream| stream.index());
    let Some(stream_index) = stream_index else {
        return Ok(None);
    };
    let stream = input_ctx
        .streams()
        .find(|value| value.index() == stream_index)
        .ok_or_else(|| "attached cover stream missing".to_string())?;
    // SAFETY: `stream` comes from the live format context and `attached_pic` is read-only.
    let packet = unsafe { &(*stream.as_ptr()).attached_pic };
    if packet.data.is_null() || packet.size <= 0 {
        return Ok(None);
    }
    // SAFETY: FFmpeg owns the packet buffer for the lifetime of the input context.
    let bytes = unsafe { std::slice::from_raw_parts(packet.data, packet.size as usize) }.to_vec();
    if bytes.is_empty() {
        return Ok(None);
    }
    validate_cover_bytes(&bytes)?;
    Ok(Some(AudioCoverArtData {
        mime_type: mime_type_from_bytes(&bytes),
        bytes,
    }))
}

fn validate_cover_bytes(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("封面图片为空".to_string());
    }
    if bytes.len() > MAX_COVER_BYTES {
        return Err(format!(
            "封面图片过大（最大 {} MB）",
            MAX_COVER_BYTES / 1024 / 1024
        ));
    }
    if image::load_from_memory(bytes).is_err() {
        return Err("封面图片格式不受支持".to_string());
    }
    Ok(())
}

fn mime_type_from_bytes(bytes: &[u8]) -> String {
    if bytes.len() >= 8 && bytes[0..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return "image/png".to_string();
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return "image/jpeg".to_string();
    }
    if bytes.len() >= 6 && (bytes[0..6] == *b"GIF87a" || bytes[0..6] == *b"GIF89a") {
        return "image/gif".to_string();
    }
    if bytes.len() >= 2 && bytes[0] == b'B' && bytes[1] == b'M' {
        return "image/bmp".to_string();
    }
    "application/octet-stream".to_string()
}

pub fn cover_art_to_base64(cover: &AudioCoverArtData) -> String {
    base64::engine::general_purpose::STANDARD.encode(&cover.bytes)
}

#[cfg(test)]
mod tests {
    use super::{CoverArtChange, decode_cover_art_base64, mime_type_from_bytes};

    #[test]
    fn parses_cover_change_values() {
        assert_eq!(CoverArtChange::parse(Some("replace")), CoverArtChange::Replace);
        assert_eq!(CoverArtChange::parse(Some("REMOVE")), CoverArtChange::Remove);
        assert_eq!(CoverArtChange::parse(Some("none")), CoverArtChange::None);
        assert_eq!(CoverArtChange::parse(None), CoverArtChange::None);
    }

    #[test]
    fn detects_png_mime_type() {
        let png_header = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(mime_type_from_bytes(&png_header), "image/png");
    }

    #[test]
    fn rejects_invalid_base64_cover() {
        assert!(decode_cover_art_base64("%%%", None).is_err());
    }
}
