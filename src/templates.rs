//! One-click presets that fill the advanced options.

use std::fmt;

use crate::ffmpeg::args::{
    AdvancedOptions, AudioCodec, Channels, Fps, Preset, QualityMode, Resolution, SampleRate,
    VideoCodec,
};
use crate::formats::{self, OutputFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    PodcastMp3,
    MusicMp3,
    VoiceAac,
    LosslessFlac,
    Opus96,
    WavPcm,
    WebMp4,
    Small720pMp4,
    WebmVp9,
    Gif480,
    RemuxCopy,
}

impl Template {
    pub const ALL: &'static [Self] = &[
        Self::PodcastMp3,
        Self::MusicMp3,
        Self::VoiceAac,
        Self::LosslessFlac,
        Self::Opus96,
        Self::WavPcm,
        Self::WebMp4,
        Self::Small720pMp4,
        Self::WebmVp9,
        Self::Gif480,
        Self::RemuxCopy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PodcastMp3 => "Audio: Podcast MP3 (128 kbps, mono)",
            Self::MusicMp3 => "Audio: Music MP3 (320 kbps)",
            Self::VoiceAac => "Audio: Voice AAC (64 kbps, mono)",
            Self::LosslessFlac => "Audio: Lossless FLAC",
            Self::Opus96 => "Audio: Opus (96 kbps)",
            Self::WavPcm => "Audio: WAV 16-bit PCM",
            Self::WebMp4 => "Video: Web MP4 (H.264 CRF 23, AAC 128k)",
            Self::Small720pMp4 => "Video: Small 720p MP4 (CRF 28, 30 fps)",
            Self::WebmVp9 => "Video: WebM VP9 (CRF 32, Opus 128k)",
            Self::Gif480 => "Video: GIF 480p, 15 fps",
            Self::RemuxCopy => "Video: Remux to MKV (copy streams)",
        }
    }

    /// Returns the output format, whether the export is audio only, and the options to apply.
    pub fn apply(self) -> (OutputFormat, bool, AdvancedOptions) {
        let mut adv = AdvancedOptions::default();
        match self {
            Self::PodcastMp3 => {
                adv.audio_codec = AudioCodec::Mp3;
                adv.audio_bitrate_kbps = "128".into();
                adv.sample_rate = SampleRate::Hz44100;
                adv.channels = Channels::Mono;
                (formats::MP3, true, adv)
            }
            Self::MusicMp3 => {
                adv.audio_codec = AudioCodec::Mp3;
                adv.audio_bitrate_kbps = "320".into();
                (formats::MP3, true, adv)
            }
            Self::VoiceAac => {
                adv.audio_codec = AudioCodec::Aac;
                adv.audio_bitrate_kbps = "64".into();
                adv.channels = Channels::Mono;
                (formats::M4A, true, adv)
            }
            Self::LosslessFlac => {
                adv.audio_codec = AudioCodec::Flac;
                (formats::FLAC, true, adv)
            }
            Self::Opus96 => {
                adv.audio_codec = AudioCodec::Opus;
                adv.audio_bitrate_kbps = "96".into();
                adv.sample_rate = SampleRate::Hz48000;
                (formats::OPUS, true, adv)
            }
            Self::WavPcm => {
                adv.audio_codec = AudioCodec::Pcm16;
                (formats::WAV, true, adv)
            }
            Self::WebMp4 => {
                adv.video_codec = VideoCodec::H264;
                adv.quality_mode = QualityMode::Crf;
                adv.crf = 23;
                adv.preset = Preset::Medium;
                adv.audio_codec = AudioCodec::Aac;
                adv.audio_bitrate_kbps = "128".into();
                (formats::MP4, false, adv)
            }
            Self::Small720pMp4 => {
                adv.video_codec = VideoCodec::H264;
                adv.quality_mode = QualityMode::Crf;
                adv.crf = 28;
                adv.resolution = Resolution::P720;
                adv.fps = Fps::F30;
                adv.preset = Preset::Medium;
                adv.audio_codec = AudioCodec::Aac;
                adv.audio_bitrate_kbps = "96".into();
                (formats::MP4, false, adv)
            }
            Self::WebmVp9 => {
                adv.video_codec = VideoCodec::Vp9;
                adv.quality_mode = QualityMode::Crf;
                adv.crf = 32;
                adv.audio_codec = AudioCodec::Opus;
                adv.audio_bitrate_kbps = "128".into();
                (formats::WEBM, false, adv)
            }
            Self::Gif480 => {
                adv.resolution = Resolution::P480;
                adv.fps = Fps::F15;
                (formats::GIF, false, adv)
            }
            Self::RemuxCopy => {
                adv.video_codec = VideoCodec::Copy;
                adv.audio_codec = AudioCodec::Copy;
                (formats::MKV, false, adv)
            }
        }
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}
