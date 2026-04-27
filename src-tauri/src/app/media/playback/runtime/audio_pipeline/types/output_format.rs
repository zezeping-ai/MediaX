use ffmpeg_next::format;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AudioOutputSampleFormat {
    F32Packed,
    I16Packed,
}

impl AudioOutputSampleFormat {
    pub fn ffmpeg_sample_format(self) -> format::Sample {
        match self {
            Self::F32Packed => format::Sample::F32(format::sample::Type::Packed),
            Self::I16Packed => format::Sample::I16(format::sample::Type::Packed),
        }
    }

    pub fn debug_label(self) -> &'static str {
        match self {
            Self::F32Packed => "f32-packed",
            Self::I16Packed => "i16-packed",
        }
    }
}
