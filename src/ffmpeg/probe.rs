//! Reading stream information with ffprobe.

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, bail};
use serde_json::Value;

use super::tokio_command;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProbeInfo {
    pub format_name: String,
    pub duration_secs: Option<f64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    /// A still image rather than a video (single frame, no audio).
    pub is_image: bool,
}

impl ProbeInfo {
    /// One-line description shown under the chosen file.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        parts.push(
            if self.is_image {
                "Image"
            } else if self.has_video {
                "Video"
            } else if self.has_audio {
                "Audio"
            } else {
                "Unknown"
            }
            .to_string(),
        );
        if let Some(d) = self.duration_secs {
            parts.push(format_duration(d));
        }
        if let (Some(w), Some(h)) = (self.width, self.height) {
            parts.push(format!("{w}x{h}"));
        }
        if let Some(v) = &self.video_codec {
            parts.push(format!("video: {v}"));
        }
        if let Some(a) = &self.audio_codec {
            parts.push(format!("audio: {a}"));
        }
        parts.join(" | ")
    }
}

pub fn format_duration(secs: f64) -> String {
    let total = secs.floor() as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub async fn run(ffprobe: PathBuf, input: PathBuf) -> anyhow::Result<ProbeInfo> {
    let output = tokio_command(&ffprobe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(&input)
        .stdin(Stdio::null())
        .output()
        .await
        .context("running ffprobe")?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.is_empty() {
            bail!("ffprobe could not read this file");
        }
        bail!("{err}");
    }
    let json: Value = serde_json::from_slice(&output.stdout).context("parsing ffprobe output")?;
    Ok(parse(&json))
}

pub fn parse(json: &Value) -> ProbeInfo {
    let format = &json["format"];
    let mut info = ProbeInfo {
        format_name: format["format_name"].as_str().unwrap_or("").to_string(),
        duration_secs: format["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|d| d.is_finite() && *d > 0.0),
        ..ProbeInfo::default()
    };

    let mut single_frame = false;
    for stream in json["streams"].as_array().into_iter().flatten() {
        let codec = stream["codec_name"].as_str().map(str::to_string);
        match stream["codec_type"].as_str() {
            // Cover art embedded in audio files is reported as a video stream; skip it.
            Some("video")
                if !info.has_video && stream["disposition"]["attached_pic"].as_i64() != Some(1) =>
            {
                info.has_video = true;
                info.width = stream["width"].as_u64().map(|v| v as u32);
                info.height = stream["height"].as_u64().map(|v| v as u32);
                info.video_codec = codec;
                single_frame = stream["nb_frames"].as_str() == Some("1");
            }
            Some("audio") if !info.has_audio => {
                info.has_audio = true;
                info.audio_codec = codec;
            }
            _ => {}
        }
    }

    let image_demuxer = info
        .format_name
        .split(',')
        .any(|n| n == "image2" || n.ends_with("_pipe"));
    let tiny = info.duration_secs.is_none_or(|d| d < 0.5);
    info.is_image = info.has_video && !info.has_audio && (image_demuxer || (single_frame && tiny));
    if info.is_image {
        info.duration_secs = None;
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn video_with_audio() {
        let info = parse(&json!({
            "streams": [
                {"codec_type": "video", "codec_name": "h264", "width": 1920, "height": 1080, "nb_frames": "300"},
                {"codec_type": "audio", "codec_name": "aac"}
            ],
            "format": {"format_name": "mov,mp4,m4a,3gp,3g2,mj2", "duration": "12.500000"}
        }));
        assert!(info.has_video && info.has_audio && !info.is_image);
        assert_eq!(info.duration_secs, Some(12.5));
        assert_eq!((info.width, info.height), (Some(1920), Some(1080)));
        assert_eq!(info.video_codec.as_deref(), Some("h264"));
        assert_eq!(
            info.summary(),
            "Video | 0:12 | 1920x1080 | video: h264 | audio: aac"
        );
    }

    #[test]
    fn mp3_with_cover_art_is_audio_only() {
        let info = parse(&json!({
            "streams": [
                {"codec_type": "audio", "codec_name": "mp3"},
                {"codec_type": "video", "codec_name": "mjpeg", "disposition": {"attached_pic": 1}}
            ],
            "format": {"format_name": "mp3", "duration": "200.1"}
        }));
        assert!(!info.has_video && info.has_audio && !info.is_image);
        assert_eq!(info.video_codec, None);
    }

    #[test]
    fn png_is_an_image() {
        let info = parse(&json!({
            "streams": [{"codec_type": "video", "codec_name": "png", "width": 64, "height": 64, "nb_frames": "1"}],
            "format": {"format_name": "png_pipe", "duration": "0.040000"}
        }));
        assert!(info.is_image);
        assert_eq!(info.duration_secs, None);
        assert_eq!(info.summary(), "Image | 64x64 | video: png");
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(format_duration(59.4), "0:59");
        assert_eq!(format_duration(3725.0), "1:02:05");
    }
}
