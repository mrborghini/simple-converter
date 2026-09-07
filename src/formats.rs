//! Registry of output formats the app can produce.

use std::fmt;

use crate::ffmpeg::probe::ProbeInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Video,
    Audio,
    Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputFormat {
    pub ext: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    /// Encoder used when the video codec is left on "Auto".
    pub default_vcodec: Option<&'static str>,
    /// Encoder used when the audio codec is left on "Auto".
    pub default_acodec: Option<&'static str>,
    /// ffprobe `codec_name`s this container can carry with `-c:v copy` (`*` = anything).
    pub copy_vcodecs: &'static [&'static str],
    /// ffprobe `codec_name`s this container can carry with `-c:a copy` (`*` = anything).
    pub copy_acodecs: &'static [&'static str],
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label)
    }
}

const fn video(
    ext: &'static str,
    label: &'static str,
    vcodec: &'static str,
    acodec: &'static str,
    copy_v: &'static [&'static str],
    copy_a: &'static [&'static str],
) -> OutputFormat {
    OutputFormat {
        ext,
        label,
        kind: Kind::Video,
        default_vcodec: Some(vcodec),
        default_acodec: Some(acodec),
        copy_vcodecs: copy_v,
        copy_acodecs: copy_a,
    }
}

const fn audio(
    ext: &'static str,
    label: &'static str,
    acodec: &'static str,
    copy_a: &'static [&'static str],
) -> OutputFormat {
    OutputFormat {
        ext,
        label,
        kind: Kind::Audio,
        default_vcodec: None,
        default_acodec: Some(acodec),
        copy_vcodecs: &[],
        copy_acodecs: copy_a,
    }
}

const fn image(ext: &'static str, label: &'static str, vcodec: &'static str) -> OutputFormat {
    OutputFormat {
        ext,
        label,
        kind: Kind::Image,
        default_vcodec: Some(vcodec),
        default_acodec: None,
        copy_vcodecs: &[],
        copy_acodecs: &[],
    }
}

pub const MP4: OutputFormat = video(
    "mp4",
    "MP4 video",
    "libx264",
    "aac",
    &["h264", "hevc", "av1", "mpeg4", "vp9"],
    &["aac", "mp3", "ac3", "eac3", "alac"],
);
pub const MKV: OutputFormat = video("mkv", "MKV video", "libx264", "aac", &["*"], &["*"]);
pub const WEBM: OutputFormat = video(
    "webm",
    "WebM video",
    "libvpx-vp9",
    "libopus",
    &["vp8", "vp9", "av1"],
    &["opus", "vorbis"],
);
pub const MOV: OutputFormat = video(
    "mov",
    "MOV video",
    "libx264",
    "aac",
    &["h264", "hevc", "prores", "mpeg4", "mjpeg", "dnxhd"],
    &["aac", "mp3", "alac", "pcm_s16le", "pcm_s24le", "ac3"],
);
pub const AVI: OutputFormat = video(
    "avi",
    "AVI video",
    "mpeg4",
    "libmp3lame",
    &["mpeg4", "msmpeg4v3", "h264", "mjpeg"],
    &["mp3", "ac3", "pcm_s16le"],
);
pub const GIF: OutputFormat = video("gif", "GIF animation", "gif", "none", &[], &[]);

pub const MP3: OutputFormat = audio("mp3", "MP3 audio", "libmp3lame", &["mp3"]);
pub const M4A: OutputFormat = audio("m4a", "M4A audio (AAC)", "aac", &["aac", "alac"]);
pub const FLAC: OutputFormat = audio("flac", "FLAC audio", "flac", &["flac"]);
pub const WAV: OutputFormat = audio(
    "wav",
    "WAV audio",
    "pcm_s16le",
    &["pcm_s16le", "pcm_s24le", "pcm_s32le", "pcm_f32le", "pcm_u8"],
);
pub const OGG: OutputFormat = audio(
    "ogg",
    "OGG audio (Vorbis)",
    "libvorbis",
    &["vorbis", "opus", "flac"],
);
pub const OPUS: OutputFormat = audio("opus", "Opus audio", "libopus", &["opus"]);

pub const PNG: OutputFormat = image("png", "PNG image", "png");
pub const JPG: OutputFormat = image("jpg", "JPEG image", "mjpeg");
pub const WEBP: OutputFormat = image("webp", "WebP image", "libwebp");

pub const ALL: &[OutputFormat] = &[
    MP4, MKV, WEBM, MOV, AVI, GIF, MP3, M4A, FLAC, WAV, OGG, OPUS, PNG, JPG, WEBP,
];

pub fn by_ext(ext: &str) -> Option<OutputFormat> {
    let ext = ext.to_ascii_lowercase();
    ALL.iter().copied().find(|f| f.ext == ext)
}

/// Formats offered in the output list for the current input and "Audio only" choice.
pub fn options(audio_only: bool, is_image: bool) -> Vec<OutputFormat> {
    ALL.iter()
        .copied()
        .filter(|f| {
            if is_image {
                f.kind == Kind::Image
            } else if audio_only {
                f.kind == Kind::Audio
            } else {
                true
            }
        })
        .collect()
}

/// Whether `format` makes sense for the probed input.
pub fn fits(format: OutputFormat, probe: &ProbeInfo) -> bool {
    if probe.is_image {
        format.kind == Kind::Image
    } else if !probe.has_video {
        format.kind == Kind::Audio
    } else {
        true
    }
}

/// A sensible default format for the probed input.
pub fn default_for(probe: &ProbeInfo) -> OutputFormat {
    if probe.is_image {
        PNG
    } else if !probe.has_video {
        MP3
    } else {
        MP4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_unique_and_lowercase() {
        let mut seen = std::collections::HashSet::new();
        for f in ALL {
            assert!(seen.insert(f.ext), "duplicate extension {}", f.ext);
            assert_eq!(f.ext, f.ext.to_ascii_lowercase());
        }
    }

    #[test]
    fn options_filter_by_kind() {
        assert!(options(true, false).iter().all(|f| f.kind == Kind::Audio));
        assert!(options(false, true).iter().all(|f| f.kind == Kind::Image));
        assert_eq!(options(false, false).len(), ALL.len());
    }

    #[test]
    fn lookup_is_case_insensitive() {
        assert_eq!(by_ext("MP4"), Some(MP4));
        assert_eq!(by_ext("nope"), None);
    }
}
