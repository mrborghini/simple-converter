//! Turns the user's choices into an ffmpeg argument list. Pure and unit tested.

use std::fmt;
use std::path::Path;

use super::probe::ProbeInfo;
use crate::formats::{Kind, OutputFormat};

macro_rules! labelled_enum {
    ($name:ident { $($variant:ident => $label:expr),+ $(,)? }) => {
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub fn label(self) -> &'static str {
                match self { $(Self::$variant => $label),+ }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.label())
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VideoCodec {
    #[default]
    Auto,
    Copy,
    H264,
    H265,
    Vp9,
    Av1,
}

labelled_enum!(VideoCodec {
    Auto => "Auto",
    Copy => "Copy (no re-encode)",
    H264 => "H.264",
    H265 => "H.265 / HEVC",
    Vp9 => "VP9",
    Av1 => "AV1",
});

impl VideoCodec {
    fn encoder(self) -> Option<&'static str> {
        match self {
            Self::Auto | Self::Copy => None,
            Self::H264 => Some("libx264"),
            Self::H265 => Some("libx265"),
            Self::Vp9 => Some("libvpx-vp9"),
            Self::Av1 => Some("libsvtav1"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioCodec {
    #[default]
    Auto,
    Copy,
    Aac,
    Mp3,
    Opus,
    Vorbis,
    Flac,
    Pcm16,
}

labelled_enum!(AudioCodec {
    Auto => "Auto",
    Copy => "Copy (no re-encode)",
    Aac => "AAC",
    Mp3 => "MP3",
    Opus => "Opus",
    Vorbis => "Vorbis",
    Flac => "FLAC",
    Pcm16 => "PCM 16-bit",
});

impl AudioCodec {
    fn encoder(self) -> Option<&'static str> {
        match self {
            Self::Auto | Self::Copy => None,
            Self::Aac => Some("aac"),
            Self::Mp3 => Some("libmp3lame"),
            Self::Opus => Some("libopus"),
            Self::Vorbis => Some("libvorbis"),
            Self::Flac => Some("flac"),
            Self::Pcm16 => Some("pcm_s16le"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityMode {
    #[default]
    Crf,
    Bitrate,
}

labelled_enum!(QualityMode {
    Crf => "Constant quality (CRF)",
    Bitrate => "Target bitrate",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Resolution {
    #[default]
    Keep,
    P2160,
    P1080,
    P720,
    P480,
    Custom,
}

labelled_enum!(Resolution {
    Keep => "Keep resolution",
    P2160 => "2160p (4K)",
    P1080 => "1080p",
    P720 => "720p",
    P480 => "480p",
    Custom => "Custom size",
});

impl Resolution {
    fn height(self) -> Option<u32> {
        match self {
            Self::Keep | Self::Custom => None,
            Self::P2160 => Some(2160),
            Self::P1080 => Some(1080),
            Self::P720 => Some(720),
            Self::P480 => Some(480),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fps {
    #[default]
    Keep,
    F10,
    F15,
    F24,
    F30,
    F60,
}

labelled_enum!(Fps {
    Keep => "Keep frame rate",
    F10 => "10 fps",
    F15 => "15 fps",
    F24 => "24 fps",
    F30 => "30 fps",
    F60 => "60 fps",
});

impl Fps {
    fn value(self) -> Option<u32> {
        match self {
            Self::Keep => None,
            Self::F10 => Some(10),
            Self::F15 => Some(15),
            Self::F24 => Some(24),
            Self::F30 => Some(30),
            Self::F60 => Some(60),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preset {
    Ultrafast,
    Superfast,
    Veryfast,
    Faster,
    Fast,
    #[default]
    Medium,
    Slow,
    Slower,
    Veryslow,
}

labelled_enum!(Preset {
    Ultrafast => "ultrafast",
    Superfast => "superfast",
    Veryfast => "veryfast",
    Faster => "faster",
    Fast => "fast",
    Medium => "medium",
    Slow => "slow",
    Slower => "slower",
    Veryslow => "veryslow",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SampleRate {
    #[default]
    Keep,
    Hz44100,
    Hz48000,
}

labelled_enum!(SampleRate {
    Keep => "Keep sample rate",
    Hz44100 => "44.1 kHz",
    Hz48000 => "48 kHz",
});

impl SampleRate {
    fn hz(self) -> Option<u32> {
        match self {
            Self::Keep => None,
            Self::Hz44100 => Some(44_100),
            Self::Hz48000 => Some(48_000),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Channels {
    #[default]
    Keep,
    Mono,
    Stereo,
}

labelled_enum!(Channels {
    Keep => "Keep channels",
    Mono => "Mono",
    Stereo => "Stereo",
});

impl Channels {
    fn count(self) -> Option<u32> {
        match self {
            Self::Keep => None,
            Self::Mono => Some(1),
            Self::Stereo => Some(2),
        }
    }
}

/// Everything in the advanced panel.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancedOptions {
    pub video_codec: VideoCodec,
    pub quality_mode: QualityMode,
    pub crf: u8,
    pub video_bitrate_kbps: String,
    pub resolution: Resolution,
    pub custom_width: String,
    pub custom_height: String,
    pub fps: Fps,
    pub preset: Preset,
    pub audio_codec: AudioCodec,
    pub audio_bitrate_kbps: String,
    pub sample_rate: SampleRate,
    pub channels: Channels,
    pub strip_audio: bool,
    pub extra_args: String,
}

impl Default for AdvancedOptions {
    fn default() -> Self {
        Self {
            video_codec: VideoCodec::Auto,
            quality_mode: QualityMode::Crf,
            crf: 23,
            video_bitrate_kbps: "4000".to_string(),
            resolution: Resolution::Keep,
            custom_width: String::new(),
            custom_height: String::new(),
            fps: Fps::Keep,
            preset: Preset::Medium,
            audio_codec: AudioCodec::Auto,
            audio_bitrate_kbps: String::new(),
            sample_rate: SampleRate::Keep,
            channels: Channels::Keep,
            strip_audio: false,
            extra_args: String::new(),
        }
    }
}

pub struct Request<'a> {
    pub input: &'a Path,
    pub output: &'a Path,
    pub probe: &'a ProbeInfo,
    pub format: OutputFormat,
    pub audio_only: bool,
    pub adv: &'a AdvancedOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltCommand {
    pub args: Vec<String>,
    /// Choices that were adjusted or ignored, explained for the user.
    pub warnings: Vec<String>,
}

pub fn build(req: &Request) -> BuiltCommand {
    let mut args: Vec<String> = [
        "-hide_banner",
        "-nostdin",
        "-y",
        "-progress",
        "pipe:1",
        "-nostats",
        "-loglevel",
        "error",
        "-i",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push(req.input.to_string_lossy().into_owned());
    let mut warnings = Vec::new();

    let format = req.format;
    let adv = req.adv;
    let audio_only = req.audio_only || format.kind == Kind::Audio;

    if format.kind == Kind::Image {
        push_image_args(&mut args, &mut warnings, req);
    } else if format.ext == "gif" {
        push_gif_args(&mut args, &mut warnings, req);
    } else if audio_only {
        args.push("-vn".to_string());
        if adv.strip_audio {
            warnings.push("Strip audio is ignored for audio output.".to_string());
        }
        if !req.probe.has_audio {
            warnings.push("The input has no audio stream.".to_string());
        }
        push_audio_args(&mut args, &mut warnings, req);
    } else {
        push_video_args(&mut args, &mut warnings, req);
        if adv.strip_audio || !req.probe.has_audio {
            args.push("-an".to_string());
        } else {
            push_audio_args(&mut args, &mut warnings, req);
        }
    }

    if matches!(format.ext, "mp4" | "m4a" | "mov") {
        push(&mut args, ["-movflags", "+faststart"]);
    }
    args.extend(split_extra_args(&adv.extra_args));
    args.push(req.output.to_string_lossy().into_owned());

    BuiltCommand { args, warnings }
}

fn push<const N: usize>(args: &mut Vec<String>, items: [&str; N]) {
    args.extend(items.iter().map(|s| s.to_string()));
}

fn copy_allowed(codec: Option<&str>, allowed: &[&str]) -> bool {
    codec.is_some_and(|c| allowed.contains(&"*") || allowed.contains(&c))
}

fn container_accepts_video(format: OutputFormat, encoder: &str) -> bool {
    match format.ext {
        "webm" => matches!(encoder, "libvpx-vp9" | "libsvtav1"),
        "avi" => matches!(encoder, "mpeg4" | "libx264"),
        _ => true,
    }
}

fn container_accepts_audio(format: OutputFormat, encoder: &str) -> bool {
    match format.ext {
        "webm" => matches!(encoder, "libopus" | "libvorbis"),
        "mp3" => encoder == "libmp3lame",
        "m4a" => encoder == "aac",
        "flac" => encoder == "flac",
        "wav" => encoder == "pcm_s16le",
        "ogg" => matches!(encoder, "libvorbis" | "libopus" | "flac"),
        "opus" => encoder == "libopus",
        "mp4" => matches!(encoder, "aac" | "libmp3lame"),
        "mov" => matches!(encoder, "aac" | "libmp3lame" | "pcm_s16le"),
        "avi" => matches!(encoder, "libmp3lame" | "pcm_s16le" | "aac"),
        _ => true,
    }
}

fn parse_kbps(text: &str) -> Option<u32> {
    let text = text.trim().trim_end_matches(['k', 'K']);
    text.parse::<u32>().ok().filter(|v| *v > 0)
}

fn parse_dimension(text: &str) -> Option<u32> {
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|v| *v >= 2)
        .map(|v| v & !1)
}

/// `scale=...` filter for the chosen resolution, if any.
fn scale_filter(
    adv: &AdvancedOptions,
    warnings: &mut Vec<String>,
    even_width: bool,
) -> Option<String> {
    match adv.resolution {
        Resolution::Keep => None,
        Resolution::Custom => {
            match (
                parse_dimension(&adv.custom_width),
                parse_dimension(&adv.custom_height),
            ) {
                (Some(w), Some(h)) => Some(format!("scale={w}:{h}")),
                _ => {
                    warnings.push(
                        "Custom size needs a width and a height; keeping the original size."
                            .to_string(),
                    );
                    None
                }
            }
        }
        other => {
            let h = other.height().unwrap_or_default();
            let w = if even_width { "-2" } else { "-1" };
            Some(format!("scale={w}:{h}"))
        }
    }
}

fn push_video_args(args: &mut Vec<String>, warnings: &mut Vec<String>, req: &Request) {
    let adv = req.adv;
    let format = req.format;
    let default_enc = format.default_vcodec.unwrap_or("libx264");

    let encoder = match adv.video_codec {
        VideoCodec::Auto => Some(default_enc),
        VideoCodec::Copy => {
            if copy_allowed(req.probe.video_codec.as_deref(), format.copy_vcodecs) {
                None
            } else {
                warnings.push(format!(
                    "{} cannot hold {} video without re-encoding; using {default_enc} instead.",
                    format.label,
                    req.probe.video_codec.as_deref().unwrap_or("this")
                ));
                Some(default_enc)
            }
        }
        chosen => {
            let enc = chosen.encoder().unwrap_or(default_enc);
            if container_accepts_video(format, enc) {
                Some(enc)
            } else {
                warnings.push(format!(
                    "{} does not support {chosen}; using {default_enc} instead.",
                    format.label
                ));
                Some(default_enc)
            }
        }
    };

    let filter = scale_filter(adv, warnings, true);
    let fps = adv.fps.value();

    let Some(enc) = encoder else {
        push(args, ["-c:v", "copy"]);
        if filter.is_some() || fps.is_some() {
            warnings.push(
                "Copying the video stream keeps the original resolution and frame rate."
                    .to_string(),
            );
        }
        return;
    };

    push(args, ["-c:v", enc]);
    let bitrate = match adv.quality_mode {
        QualityMode::Bitrate => {
            let parsed = parse_kbps(&adv.video_bitrate_kbps);
            if parsed.is_none() {
                warnings.push("Invalid video bitrate; using constant quality instead.".to_string());
            }
            parsed
        }
        QualityMode::Crf => None,
    };
    let crf = adv.crf.to_string();
    match enc {
        "libx264" | "libx265" => {
            push(args, ["-preset", adv.preset.label()]);
            match bitrate {
                Some(b) => push(args, ["-b:v", &format!("{b}k")]),
                None => push(args, ["-crf", &crf]),
            }
            if enc == "libx264" {
                push(args, ["-pix_fmt", "yuv420p"]);
            }
        }
        "libvpx-vp9" => {
            match bitrate {
                Some(b) => push(args, ["-b:v", &format!("{b}k")]),
                None => push(args, ["-crf", &crf, "-b:v", "0"]),
            }
            push(args, ["-row-mt", "1"]);
        }
        "libsvtav1" => match bitrate {
            Some(b) => push(args, ["-b:v", &format!("{b}k")]),
            None => push(args, ["-crf", &adv.crf.min(63).to_string()]),
        },
        "mpeg4" => match bitrate {
            Some(b) => push(args, ["-b:v", &format!("{b}k")]),
            None => push(args, ["-q:v", "4"]),
        },
        _ => {}
    }
    if let Some(f) = filter {
        push(args, ["-vf", &f]);
    }
    if let Some(fps) = fps {
        push(args, ["-r", &fps.to_string()]);
    }
}

fn push_audio_args(args: &mut Vec<String>, warnings: &mut Vec<String>, req: &Request) {
    let adv = req.adv;
    let format = req.format;
    let default_enc = format.default_acodec.unwrap_or("aac");

    let encoder = match adv.audio_codec {
        AudioCodec::Auto => Some(default_enc),
        AudioCodec::Copy => {
            if copy_allowed(req.probe.audio_codec.as_deref(), format.copy_acodecs) {
                None
            } else {
                warnings.push(format!(
                    "{} cannot hold {} audio without re-encoding; using {default_enc} instead.",
                    format.label,
                    req.probe.audio_codec.as_deref().unwrap_or("this")
                ));
                Some(default_enc)
            }
        }
        chosen => {
            let enc = chosen.encoder().unwrap_or(default_enc);
            if container_accepts_audio(format, enc) {
                Some(enc)
            } else {
                warnings.push(format!(
                    "{} does not support {chosen}; using {default_enc} instead.",
                    format.label
                ));
                Some(default_enc)
            }
        }
    };

    let Some(enc) = encoder else {
        push(args, ["-c:a", "copy"]);
        if !adv.audio_bitrate_kbps.trim().is_empty()
            || adv.sample_rate != SampleRate::Keep
            || adv.channels != Channels::Keep
        {
            warnings.push(
                "Copying the audio stream ignores bitrate, sample rate and channel settings."
                    .to_string(),
            );
        }
        return;
    };

    push(args, ["-c:a", enc]);
    let lossless = matches!(enc, "flac" | "pcm_s16le");
    match parse_kbps(&adv.audio_bitrate_kbps) {
        Some(_) if lossless => {
            warnings.push(format!("Bitrate is ignored for lossless {enc} audio."));
        }
        Some(b) => push(args, ["-b:a", &format!("{b}k")]),
        None if !adv.audio_bitrate_kbps.trim().is_empty() => {
            warnings.push("Invalid audio bitrate; using the encoder default.".to_string());
        }
        None => {}
    }
    if let Some(hz) = adv.sample_rate.hz() {
        if enc == "libopus" && hz != 48_000 {
            warnings
                .push("Opus always uses 48 kHz; the sample rate setting is ignored.".to_string());
        } else {
            push(args, ["-ar", &hz.to_string()]);
        }
    }
    if let Some(c) = adv.channels.count() {
        push(args, ["-ac", &c.to_string()]);
    }
}

fn push_gif_args(args: &mut Vec<String>, warnings: &mut Vec<String>, req: &Request) {
    let adv = req.adv;
    if adv.video_codec == VideoCodec::Copy {
        warnings.push("GIF output always re-encodes the video.".to_string());
    }
    let fps = adv.fps.value().unwrap_or(15);
    let mut chain = format!("fps={fps}");
    if let Some(scale) = scale_filter(adv, warnings, false) {
        chain.push(',');
        chain.push_str(&scale);
        chain.push_str(":flags=lanczos");
    }
    chain.push_str(",split[a][b];[a]palettegen[p];[b][p]paletteuse");
    push(args, ["-vf", &chain, "-loop", "0", "-an"]);
}

fn push_image_args(args: &mut Vec<String>, warnings: &mut Vec<String>, req: &Request) {
    if req.probe.has_video && !req.probe.is_image {
        push(args, ["-frames:v", "1"]);
    }
    if let Some(scale) = scale_filter(req.adv, warnings, false) {
        push(args, ["-vf", &scale]);
    }
    match req.format.ext {
        "png" => push(args, ["-c:v", "png"]),
        "jpg" => push(args, ["-c:v", "mjpeg", "-q:v", "2"]),
        "webp" => push(args, ["-c:v", "libwebp", "-quality", "90"]),
        _ => {}
    }
    push(args, ["-update", "1", "-an"]);
}

/// Splits free-form extra arguments like a shell would (quotes and backslash escapes).
pub fn split_extra_args(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut has_token = false;
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
                has_token = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
                has_token = true;
            }
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    current.push(next);
                    has_token = true;
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if has_token {
                    out.push(std::mem::take(&mut current));
                    has_token = false;
                }
            }
            c => {
                current.push(c);
                has_token = true;
            }
        }
    }
    if has_token {
        out.push(current);
    }
    out
}

/// One-line, shell-quoted rendering of the command for display.
pub fn preview(program: &str, args: &[String]) -> String {
    std::iter::once(quote(program))
        .chain(args.iter().map(|a| quote(a)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote(arg: &str) -> String {
    let safe = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=+,@%".contains(c));
    if safe {
        return arg.to_string();
    }
    if cfg!(windows) {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats;

    fn video_probe() -> ProbeInfo {
        ProbeInfo {
            format_name: "mov,mp4".into(),
            duration_secs: Some(10.0),
            has_video: true,
            has_audio: true,
            width: Some(1920),
            height: Some(1080),
            video_codec: Some("h264".into()),
            audio_codec: Some("aac".into()),
            is_image: false,
        }
    }

    fn build_with(
        probe: &ProbeInfo,
        format: OutputFormat,
        audio_only: bool,
        adv: &AdvancedOptions,
    ) -> BuiltCommand {
        build(&Request {
            input: Path::new("/in/clip.mp4"),
            output: Path::new("/out/result.x"),
            probe,
            format,
            audio_only,
            adv,
        })
    }

    fn has_pair(args: &[String], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| w[0] == a && w[1] == b)
    }

    #[test]
    fn audio_only_mp3_from_video() {
        let cmd = build_with(
            &video_probe(),
            formats::MP3,
            true,
            &AdvancedOptions::default(),
        );
        assert!(cmd.args.contains(&"-vn".to_string()));
        assert!(has_pair(&cmd.args, "-c:a", "libmp3lame"));
        assert!(!cmd.args.contains(&"-c:v".to_string()));
        assert_eq!(cmd.args.last().unwrap(), "/out/result.x");
        assert_eq!(cmd.args[..2], ["-hide_banner", "-nostdin"]);
        assert!(cmd.warnings.is_empty());
    }

    #[test]
    fn audio_format_implies_audio_only() {
        let cmd = build_with(
            &video_probe(),
            formats::FLAC,
            false,
            &AdvancedOptions::default(),
        );
        assert!(cmd.args.contains(&"-vn".to_string()));
        assert!(has_pair(&cmd.args, "-c:a", "flac"));
    }

    #[test]
    fn default_mp4_uses_x264_crf_and_faststart() {
        let cmd = build_with(
            &video_probe(),
            formats::MP4,
            false,
            &AdvancedOptions::default(),
        );
        assert!(has_pair(&cmd.args, "-c:v", "libx264"));
        assert!(has_pair(&cmd.args, "-crf", "23"));
        assert!(has_pair(&cmd.args, "-preset", "medium"));
        assert!(has_pair(&cmd.args, "-pix_fmt", "yuv420p"));
        assert!(has_pair(&cmd.args, "-c:a", "aac"));
        assert!(has_pair(&cmd.args, "-movflags", "+faststart"));
        assert!(!cmd.args.contains(&"-b:v".to_string()));
    }

    #[test]
    fn bitrate_mode_replaces_crf() {
        let adv = AdvancedOptions {
            quality_mode: QualityMode::Bitrate,
            video_bitrate_kbps: "2500".into(),
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP4, false, &adv);
        assert!(has_pair(&cmd.args, "-b:v", "2500k"));
        assert!(!cmd.args.contains(&"-crf".to_string()));
    }

    #[test]
    fn invalid_bitrate_falls_back_to_crf_with_warning() {
        let adv = AdvancedOptions {
            quality_mode: QualityMode::Bitrate,
            video_bitrate_kbps: "fast".into(),
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP4, false, &adv);
        assert!(has_pair(&cmd.args, "-crf", "23"));
        assert_eq!(cmd.warnings.len(), 1);
    }

    #[test]
    fn scale_and_fps() {
        let adv = AdvancedOptions {
            resolution: Resolution::P720,
            fps: Fps::F30,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP4, false, &adv);
        assert!(has_pair(&cmd.args, "-vf", "scale=-2:720"));
        assert!(has_pair(&cmd.args, "-r", "30"));
    }

    #[test]
    fn custom_resolution_is_rounded_to_even() {
        let adv = AdvancedOptions {
            resolution: Resolution::Custom,
            custom_width: "641".into(),
            custom_height: "361".into(),
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP4, false, &adv);
        assert!(has_pair(&cmd.args, "-vf", "scale=640:360"));
    }

    #[test]
    fn copy_compatible_streams_into_mkv() {
        let adv = AdvancedOptions {
            video_codec: VideoCodec::Copy,
            audio_codec: AudioCodec::Copy,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MKV, false, &adv);
        assert!(has_pair(&cmd.args, "-c:v", "copy"));
        assert!(has_pair(&cmd.args, "-c:a", "copy"));
        assert!(cmd.warnings.is_empty());
    }

    #[test]
    fn copy_into_incompatible_container_falls_back() {
        let adv = AdvancedOptions {
            video_codec: VideoCodec::Copy,
            audio_codec: AudioCodec::Copy,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::WEBM, false, &adv);
        assert!(has_pair(&cmd.args, "-c:v", "libvpx-vp9"));
        assert!(has_pair(&cmd.args, "-c:a", "libopus"));
        assert_eq!(cmd.warnings.len(), 2);
    }

    #[test]
    fn explicit_codec_unsupported_by_container_falls_back() {
        let adv = AdvancedOptions {
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Mp3,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::WEBM, false, &adv);
        assert!(has_pair(&cmd.args, "-c:v", "libvpx-vp9"));
        assert!(has_pair(&cmd.args, "-c:a", "libopus"));
        assert_eq!(cmd.warnings.len(), 2);
    }

    #[test]
    fn vp9_crf_needs_zero_bitrate() {
        let adv = AdvancedOptions {
            video_codec: VideoCodec::Vp9,
            crf: 32,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::WEBM, false, &adv);
        assert!(has_pair(&cmd.args, "-crf", "32"));
        assert!(has_pair(&cmd.args, "-b:v", "0"));
    }

    #[test]
    fn audio_settings() {
        let adv = AdvancedOptions {
            audio_codec: AudioCodec::Mp3,
            audio_bitrate_kbps: "128".into(),
            sample_rate: SampleRate::Hz44100,
            channels: Channels::Mono,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP3, true, &adv);
        assert!(has_pair(&cmd.args, "-b:a", "128k"));
        assert!(has_pair(&cmd.args, "-ar", "44100"));
        assert!(has_pair(&cmd.args, "-ac", "1"));
    }

    #[test]
    fn opus_ignores_non_48k_sample_rate() {
        let adv = AdvancedOptions {
            audio_codec: AudioCodec::Opus,
            sample_rate: SampleRate::Hz44100,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::OPUS, true, &adv);
        assert!(!cmd.args.contains(&"-ar".to_string()));
        assert_eq!(cmd.warnings.len(), 1);
    }

    #[test]
    fn lossless_ignores_bitrate() {
        let adv = AdvancedOptions {
            audio_bitrate_kbps: "320".into(),
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::FLAC, true, &adv);
        assert!(!cmd.args.contains(&"-b:a".to_string()));
        assert_eq!(cmd.warnings.len(), 1);
    }

    #[test]
    fn strip_audio() {
        let adv = AdvancedOptions {
            strip_audio: true,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP4, false, &adv);
        assert!(cmd.args.contains(&"-an".to_string()));
        assert!(!cmd.args.contains(&"-c:a".to_string()));
    }

    #[test]
    fn gif_uses_palette_chain_and_no_audio() {
        let adv = AdvancedOptions {
            resolution: Resolution::P480,
            fps: Fps::F15,
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::GIF, false, &adv);
        assert!(has_pair(
            &cmd.args,
            "-vf",
            "fps=15,scale=-1:480:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse"
        ));
        assert!(cmd.args.contains(&"-an".to_string()));
        assert!(has_pair(&cmd.args, "-loop", "0"));
    }

    #[test]
    fn image_from_video_takes_first_frame() {
        let cmd = build_with(
            &video_probe(),
            formats::PNG,
            false,
            &AdvancedOptions::default(),
        );
        assert!(has_pair(&cmd.args, "-frames:v", "1"));
        assert!(has_pair(&cmd.args, "-c:v", "png"));
        assert!(has_pair(&cmd.args, "-update", "1"));
        assert!(cmd.args.contains(&"-an".to_string()));
    }

    #[test]
    fn image_to_image_has_no_frame_limit() {
        let probe = ProbeInfo {
            has_video: true,
            is_image: true,
            video_codec: Some("png".into()),
            ..ProbeInfo::default()
        };
        let cmd = build_with(&probe, formats::WEBP, false, &AdvancedOptions::default());
        assert!(!cmd.args.contains(&"-frames:v".to_string()));
        assert!(has_pair(&cmd.args, "-c:v", "libwebp"));
    }

    #[test]
    fn extra_args_go_before_output() {
        let adv = AdvancedOptions {
            extra_args: "-metadata title=\"My Clip\" -t 5".into(),
            ..AdvancedOptions::default()
        };
        let cmd = build_with(&video_probe(), formats::MP4, false, &adv);
        let n = cmd.args.len();
        assert_eq!(
            &cmd.args[n - 5..],
            ["-metadata", "title=My Clip", "-t", "5", "/out/result.x"]
        );
    }

    #[test]
    fn splits_like_a_shell() {
        assert_eq!(
            split_extra_args(r#"-a "b c" 'd e' f\ g "" "#),
            vec!["-a", "b c", "d e", "f g", ""]
        );
        assert!(split_extra_args("   ").is_empty());
    }

    #[test]
    fn preview_quotes_only_when_needed() {
        let args = vec![
            "-i".to_string(),
            "/tmp/my clip.mp4".to_string(),
            "-crf".into(),
            "23".into(),
        ];
        let line = preview("ffmpeg", &args);
        assert!(line.starts_with("ffmpeg -i "));
        assert!(line.contains("my clip.mp4"));
        assert!(line.ends_with(" -crf 23"));
        let quoted = if cfg!(windows) {
            "\"/tmp/my clip.mp4\""
        } else {
            "'/tmp/my clip.mp4'"
        };
        assert!(line.contains(quoted));
    }
}
