//! End-to-end checks against a real ffmpeg (managed copy or system install).
//! Ignored by default because they need ffmpeg: `cargo test -- --ignored`.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use iced::futures::StreamExt;

use super::args::{self, AdvancedOptions, AudioCodec, Request, VideoCodec};
use super::run::{self, ConvertEvent, Job};
use super::{FfmpegPaths, locate, probe, std_command};
use crate::formats::{self, OutputFormat};
use crate::templates::Template;

fn ffmpeg() -> FfmpegPaths {
    locate::find_managed()
        .or_else(locate::find_system)
        .expect("these tests need ffmpeg (managed copy or on PATH)")
}

fn workdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "simple-converter-e2e-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn generate(paths: &FfmpegPaths, output: &Path, args: &[&str]) -> PathBuf {
    let status = std_command(&paths.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .arg(output)
        .stdin(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "could not generate {}", output.display());
    output.to_path_buf()
}

fn sample_video(paths: &FfmpegPaths, dir: &Path) -> PathBuf {
    generate(
        paths,
        &dir.join("sample.mp4"),
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=320x240:rate=30:duration=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ],
    )
}

fn sample_wav(paths: &FfmpegPaths, dir: &Path) -> PathBuf {
    generate(
        paths,
        &dir.join("sample.wav"),
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-c:a",
            "pcm_s16le",
        ],
    )
}

fn sample_png(paths: &FfmpegPaths, dir: &Path) -> PathBuf {
    generate(
        paths,
        &dir.join("sample.png"),
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=64x64:rate=1",
            "-frames:v",
            "1",
            "-update",
            "1",
        ],
    )
}

fn probe_sync(paths: &FfmpegPaths, file: &Path) -> probe::ProbeInfo {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(probe::run(paths.ffprobe.clone(), file.to_path_buf()))
        .unwrap_or_else(|e| panic!("probing {} failed: {e:#}", file.display()))
}

/// Runs a conversion exactly like the app does and returns every event that was emitted.
fn convert(
    paths: &FfmpegPaths,
    input: &Path,
    format: OutputFormat,
    audio_only: bool,
    adv: &AdvancedOptions,
) -> (PathBuf, Vec<ConvertEvent>, args::BuiltCommand) {
    let info = probe_sync(paths, input);
    let stem = input.file_stem().unwrap().to_string_lossy();
    let output = input.with_file_name(format!("{stem}-to-{}.{}", format.ext, format.ext));
    let built = args::build(&Request {
        input,
        output: &output,
        probe: &info,
        format,
        audio_only,
        adv,
    });
    let events = tokio::runtime::Runtime::new().unwrap().block_on(
        run::convert(Job {
            ffmpeg: paths.ffmpeg.clone(),
            args: built.args.clone(),
            output: output.clone(),
            duration_secs: info.duration_secs,
        })
        .collect::<Vec<_>>(),
    );
    (output, events, built)
}

fn expect_done(
    paths: &FfmpegPaths,
    input: &Path,
    format: OutputFormat,
    audio_only: bool,
    adv: &AdvancedOptions,
) -> (PathBuf, Vec<ConvertEvent>) {
    let (output, events, built) = convert(paths, input, format, audio_only, adv);
    match events.last() {
        Some(ConvertEvent::Done(path)) => assert_eq!(path, &output),
        other => panic!(
            "conversion to {} failed: {other:?}\ncommand: {}\nwarnings: {:?}",
            format.ext,
            args::preview("ffmpeg", &built.args),
            built.warnings
        ),
    }
    assert!(output.metadata().map(|m| m.len() > 0).unwrap_or(false));
    (output, events)
}

#[test]
#[ignore]
fn video_to_mp3_audio_only_and_progress() {
    let paths = ffmpeg();
    let dir = workdir("mp3");
    let input = sample_video(&paths, &dir);
    let (output, events) = expect_done(
        &paths,
        &input,
        formats::MP3,
        true,
        &AdvancedOptions::default(),
    );
    let info = probe_sync(&paths, &output);
    assert!(info.has_audio && !info.has_video);
    assert_eq!(info.audio_codec.as_deref(), Some("mp3"));
    assert!(
        events.iter().any(|e| matches!(
            e,
            ConvertEvent::Progress {
                percent: Some(_),
                ..
            }
        )),
        "expected progress events, got {events:?}"
    );
}

#[test]
#[ignore]
fn video_templates() {
    let paths = ffmpeg();
    let dir = workdir("templates");
    let input = sample_video(&paths, &dir);

    let (format, audio_only, adv) = Template::Small720pMp4.apply();
    let (output, _) = expect_done(&paths, &input, format, audio_only, &adv);
    let info = probe_sync(&paths, &output);
    assert_eq!(info.height, Some(720));
    assert_eq!(info.video_codec.as_deref(), Some("h264"));
    assert_eq!(info.audio_codec.as_deref(), Some("aac"));

    let (format, audio_only, adv) = Template::WebmVp9.apply();
    let (output, _) = expect_done(&paths, &input, format, audio_only, &adv);
    let info = probe_sync(&paths, &output);
    assert_eq!(info.video_codec.as_deref(), Some("vp9"));
    assert_eq!(info.audio_codec.as_deref(), Some("opus"));

    let (format, audio_only, adv) = Template::Gif480.apply();
    let (output, _) = expect_done(&paths, &input, format, audio_only, &adv);
    let info = probe_sync(&paths, &output);
    assert_eq!(info.video_codec.as_deref(), Some("gif"));
    assert_eq!(info.height, Some(480));

    let (format, audio_only, adv) = Template::RemuxCopy.apply();
    let (output, _) = expect_done(&paths, &input, format, audio_only, &adv);
    let info = probe_sync(&paths, &output);
    assert_eq!(info.video_codec.as_deref(), Some("h264"));
    assert_eq!(info.audio_codec.as_deref(), Some("aac"));

    let (format, audio_only, adv) = Template::PodcastMp3.apply();
    let (output, _) = expect_done(&paths, &input, format, audio_only, &adv);
    let info = probe_sync(&paths, &output);
    assert!(!info.has_video && info.has_audio);
}

#[test]
#[ignore]
fn images_and_audio_files() {
    let paths = ffmpeg();
    let dir = workdir("misc");

    let video = sample_video(&paths, &dir);
    let (output, _) = expect_done(
        &paths,
        &video,
        formats::PNG,
        false,
        &AdvancedOptions::default(),
    );
    let info = probe_sync(&paths, &output);
    assert!(
        info.is_image,
        "first frame should be a still image: {info:?}"
    );

    let png = sample_png(&paths, &dir);
    assert!(probe_sync(&paths, &png).is_image);
    let (output, _) = expect_done(
        &paths,
        &png,
        formats::WEBP,
        false,
        &AdvancedOptions::default(),
    );
    assert!(probe_sync(&paths, &output).is_image);
    let (output, _) = expect_done(
        &paths,
        &png,
        formats::JPG,
        false,
        &AdvancedOptions::default(),
    );
    assert_eq!(
        probe_sync(&paths, &output).video_codec.as_deref(),
        Some("mjpeg")
    );

    let wav = sample_wav(&paths, &dir);
    let info = probe_sync(&paths, &wav);
    assert!(!info.has_video && info.has_audio);
    let (output, _) = expect_done(
        &paths,
        &wav,
        formats::FLAC,
        false,
        &AdvancedOptions::default(),
    );
    assert_eq!(
        probe_sync(&paths, &output).audio_codec.as_deref(),
        Some("flac")
    );
    let adv = AdvancedOptions {
        audio_codec: AudioCodec::Opus,
        audio_bitrate_kbps: "96".into(),
        ..AdvancedOptions::default()
    };
    let (output, _) = expect_done(&paths, &wav, formats::OPUS, false, &adv);
    assert_eq!(
        probe_sync(&paths, &output).audio_codec.as_deref(),
        Some("opus")
    );
}

#[test]
#[ignore]
fn copy_fallback_and_failure_reporting() {
    let paths = ffmpeg();
    let dir = workdir("fallback");
    let input = sample_video(&paths, &dir);

    // h264/aac cannot be copied into WebM: the builder must fall back to re-encoding.
    let adv = AdvancedOptions {
        video_codec: VideoCodec::Copy,
        audio_codec: AudioCodec::Copy,
        ..AdvancedOptions::default()
    };
    let (output, events, built) = convert(&paths, &input, formats::WEBM, false, &adv);
    assert_eq!(built.warnings.len(), 2, "{:?}", built.warnings);
    assert!(matches!(events.last(), Some(ConvertEvent::Done(_))));
    assert_eq!(
        probe_sync(&paths, &output).video_codec.as_deref(),
        Some("vp9")
    );

    // A bogus extra flag must surface ffmpeg's error text.
    let adv = AdvancedOptions {
        extra_args: "-definitely-not-a-flag 1".into(),
        ..AdvancedOptions::default()
    };
    let (_, events, _) = convert(&paths, &input, formats::MP4, false, &adv);
    match events.last() {
        Some(ConvertEvent::Failed(message)) => {
            assert!(message.contains("ffmpeg exited with"), "{message}");
            assert!(message.contains("definitely-not-a-flag"), "{message}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}
