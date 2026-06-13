use std::ffi::OsStr;

/// Decode raw metadata bytes from FFmpeg tags. Tags are often UTF-8, but legacy
/// Chinese libraries frequently store GBK/GB18030 without an explicit charset.
pub fn decode_bytes_to_text(bytes: &[u8]) -> String {
    let trimmed = trim_metadata_bytes(bytes);
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(value) = std::str::from_utf8(trimmed) {
        return value.to_string();
    }

    for encoding in [
        encoding_rs::GB18030,
        encoding_rs::GBK,
        encoding_rs::BIG5,
        encoding_rs::SHIFT_JIS,
        encoding_rs::EUC_KR,
        encoding_rs::WINDOWS_1252,
    ] {
        let (decoded, _, had_errors) = encoding.decode(trimmed);
        if !had_errors && !decoded.contains('\u{FFFD}') {
            return decoded.into_owned();
        }
    }

    String::from_utf8_lossy(trimmed).into_owned()
}

/// Decode a filesystem path component. Unix paths may use locale byte encodings;
/// Windows paths are usually UTF-16 internally and convert cleanly via lossy UTF-8.
pub fn decode_os_str(value: &OsStr) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes = value.as_bytes();
        if bytes.is_empty() {
            return String::new();
        }
        if let Ok(value) = std::str::from_utf8(bytes) {
            return value.to_string();
        }
        let decoded = decode_bytes_to_text(bytes);
        if !decoded.is_empty() && !decoded.contains('\u{FFFD}') {
            return decoded;
        }
    }

    value.to_string_lossy().into_owned()
}

fn trim_metadata_bytes(bytes: &[u8]) -> &[u8] {
    bytes.trim_ascii()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn decodes_gbk_metadata_bytes() {
        // "青花瓷" in GBK
        let bytes = [0xC7, 0xE0, 0xBB, 0xA8, 0xB4, 0xC9];
        assert_eq!(decode_bytes_to_text(&bytes), "青花瓷");
    }

    #[test]
    fn keeps_valid_utf8_metadata() {
        assert_eq!(decode_bytes_to_text("周杰伦".as_bytes()), "周杰伦");
    }

    #[test]
    fn decodes_gbk_path_component_on_unix() {
        if !cfg!(unix) {
            return;
        }
        use std::os::unix::ffi::OsStrExt;

        let decoded = decode_os_str(OsStr::from_bytes(b"\xD6\xDC\xBD\xDC\xC2\xD7 - \xC7\xE0\xBB\xA8\xB4\xC9"));
        assert_eq!(decoded, "周杰伦 - 青花瓷");
    }
}
