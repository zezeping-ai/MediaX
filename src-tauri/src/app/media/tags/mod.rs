mod cover;
mod write;

pub use cover::{
    cover_art_to_base64, read_audio_cover_art, read_image_file_for_cover, AudioCoverArtData,
    CoverArtChange,
};
pub use write::{supports_tag_writing, write_audio_tags, AudioTagWriteInput};
