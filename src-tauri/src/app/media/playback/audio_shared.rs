use ffmpeg_next::channel_layout::ChannelLayout;

pub(crate) fn fallback_channel_layout(decoder: &ffmpeg_next::decoder::Audio) -> ChannelLayout {
    if decoder.channel_layout().is_empty() {
        ChannelLayout::default(decoder.channels().into())
    } else {
        decoder.channel_layout()
    }
}
