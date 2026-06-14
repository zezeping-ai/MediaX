use std::path::Path;

use super::cover::{apply_cover_art_change, decode_cover_art_base64, CoverArtChange};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag, TagType};

const TAG_WRITABLE_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "m4a", "ogg", "opus", "ape", "wma", "aiff", "alac", "mka", "wav",
];

#[derive(Debug, Clone)]
pub struct AudioTagWriteInput {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub lyrics_lrc: Option<String>,
    pub embed_lyrics: bool,
    pub cover_art_change: CoverArtChange,
    pub cover_art_data_base64: Option<String>,
    pub cover_art_mime_type: Option<String>,
}

pub fn supports_tag_writing(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .is_some_and(|ext| TAG_WRITABLE_EXTENSIONS.contains(&ext.as_str()))
}

pub fn write_audio_tags(path: &str, input: &AudioTagWriteInput) -> Result<(), String> {
    if !supports_tag_writing(path) {
        return Err(format!("当前文件格式不支持写入标签: {path}"));
    }
    let path_buf = Path::new(path);
    if !path_buf.is_file() {
        return Err(format!("文件不存在或不可写: {path}"));
    }

    let mut tagged_file = Probe::open(path_buf)
        .map_err(|err| format!("打开文件失败: {err}"))?
        .guess_file_type()
        .map_err(|err| format!("识别文件格式失败: {err}"))?
        .read()
        .map_err(|err| format!("读取标签失败: {err}"))?;

    let tag_type = tagged_file.primary_tag_type();
    if tagged_file.primary_tag().is_none() {
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged_file
        .primary_tag_mut()
        .ok_or_else(|| "无法创建标签".to_string())?;

    apply_optional_text(tag, input.title.as_deref(), |tag, value| tag.set_title(value), |tag| {
        tag.remove_title();
    });
    apply_optional_text(tag, input.artist.as_deref(), |tag, value| tag.set_artist(value), |tag| {
        tag.remove_artist();
    });
    apply_optional_text(tag, input.album.as_deref(), |tag, value| tag.set_album(value), |tag| {
        tag.remove_album();
    });

    if input.embed_lyrics {
        apply_lyrics_field(tag, tag_type, input.lyrics_lrc.as_deref());
    }

    if input.cover_art_change != CoverArtChange::None {
        let cover_bytes = if input.cover_art_change == CoverArtChange::Replace {
            Some(decode_cover_art_base64(
                input
                    .cover_art_data_base64
                    .as_deref()
                    .unwrap_or(""),
                input.cover_art_mime_type.as_deref(),
            )?)
        } else {
            None
        };
        apply_cover_art_change(tag, input.cover_art_change, cover_bytes.as_deref())?;
    }

    tagged_file
        .save_to_path(path_buf, WriteOptions::default())
        .map_err(|err| format!("保存标签失败: {err}"))?;
    Ok(())
}

fn apply_optional_text<F, C>(tag: &mut Tag, value: Option<&str>, setter: F, clearer: C)
where
    F: FnOnce(&mut Tag, String),
    C: FnOnce(&mut Tag),
{
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        clearer(tag);
        return;
    }
    setter(tag, trimmed.to_string());
}

fn apply_lyrics_field(tag: &mut Tag, tag_type: TagType, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        tag.remove_key(&ItemKey::Lyrics);
        if tag_type == TagType::Id3v2 {
            tag.remove_key(&ItemKey::Unknown("USLT".to_string()));
            tag.remove_key(&ItemKey::Unknown("SYLT".to_string()));
        }
        return;
    }
    tag.insert_text(ItemKey::Lyrics, trimmed.to_string());
}

#[cfg(test)]
mod tests {
    use super::{supports_tag_writing, write_audio_tags, AudioTagWriteInput};
    use crate::app::media::tags::CoverArtChange;
    use lofty::file::TaggedFileExt;
    use lofty::probe::Probe;
    use lofty::tag::ItemKey;
    use std::path::PathBuf;

    #[test]
    fn recognizes_tag_writable_extensions() {
        assert!(supports_tag_writing("/music/demo.mp3"));
        assert!(supports_tag_writing("/music/demo.flac"));
        assert!(!supports_tag_writing("/music/demo.ac3"));
    }

    #[test]
    fn writes_flac_lyrics_roundtrip() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test_embed.flac");
        if !source.is_file() {
            return;
        }
        let target =
            std::env::temp_dir().join(format!("mediax-lyrics-embed-{}.flac", std::process::id()));
        std::fs::copy(&source, &target).expect("copy test flac");

        let sample = "[00:12.34]测试歌词\n[00:18.00]第二行\n";
        write_audio_tags(
            target.to_str().expect("temp path"),
            &AudioTagWriteInput {
                title: Some("Love Is Everything".to_string()),
                artist: Some("梁静茹".to_string()),
                album: None,
                lyrics_lrc: Some(sample.to_string()),
                embed_lyrics: true,
                cover_art_change: CoverArtChange::None,
                cover_art_data_base64: None,
                cover_art_mime_type: None,
            },
        )
        .expect("write tags");

        let tagged = Probe::open(&target)
            .expect("open written file")
            .guess_file_type()
            .expect("guess type")
            .read()
            .expect("read tags");
        let lyrics = tagged
            .primary_tag()
            .and_then(|tag| tag.get(&ItemKey::Lyrics))
            .and_then(|item| item.value().text().map(str::to_string))
            .expect("embedded lyrics");
        assert!(lyrics.contains("测试歌词"));
        let _ = std::fs::remove_file(target);
    }
}
