//! Everything that talks to ffmpeg/ffprobe: locating or downloading the binaries,
//! probing inputs, building argument lists and running conversions.

pub mod args;
pub mod download;
#[cfg(test)]
mod integration_tests;
pub mod locate;
pub mod probe;
pub mod run;

use std::path::{Path, PathBuf};

use iced::futures::{SinkExt, Stream};

use crate::paths;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Downloaded by the app into its data directory.
    Managed { label: String },
    /// Found on `PATH`.
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub source: Source,
}

impl FfmpegPaths {
    pub fn describe(&self) -> String {
        match &self.source {
            Source::Managed { label } => format!("Ready (managed ffmpeg {label})"),
            Source::System => "Ready (system ffmpeg)".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum FfmpegEvent {
    Checking,
    Downloading { done: u64, total: Option<u64> },
    Extracting,
    Ready(FfmpegPaths),
    Failed(String),
}

/// Locates ffmpeg (managed copy, then `PATH`) or downloads it, reporting progress as events.
/// With `force_download` the lookup is skipped and a fresh copy is always installed.
pub fn ensure(force_download: bool) -> impl Stream<Item = FfmpegEvent> {
    iced::stream::channel(32, async move |mut out| {
        let _ = out.send(FfmpegEvent::Checking).await;

        if !force_download {
            let found =
                tokio::task::spawn_blocking(|| locate::find_managed().or_else(locate::find_system))
                    .await;
            match found {
                Ok(Some(paths)) => {
                    let _ = out.send(FfmpegEvent::Ready(paths)).await;
                    return;
                }
                Ok(None) => {}
                Err(e) => {
                    let _ = out
                        .send(FfmpegEvent::Failed(format!("internal error: {e}")))
                        .await;
                    return;
                }
            }
        }

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<FfmpegEvent>();
        let worker = tokio::task::spawn_blocking(move || {
            let dir = paths::ffmpeg_dir()?;
            let progress_tx = tx.clone();
            download::install(
                &dir,
                move |done, total| {
                    let _ = progress_tx.send(FfmpegEvent::Downloading { done, total });
                },
                move || {
                    let _ = tx.send(FfmpegEvent::Extracting);
                },
            )
        });

        while let Some(event) = rx.recv().await {
            if out.send(event).await.is_err() {
                return;
            }
        }

        let event = match worker.await {
            Ok(Ok(paths)) => FfmpegEvent::Ready(paths),
            Ok(Err(e)) => FfmpegEvent::Failed(format!("{e:#}")),
            Err(e) => FfmpegEvent::Failed(format!("internal error: {e}")),
        };
        let _ = out.send(event).await;
    })
}

/// A `std::process::Command` that never opens a console window on Windows.
pub fn std_command(program: &Path) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// A `tokio::process::Command` that never opens a console window on Windows.
pub fn tokio_command(program: &Path) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut cmd = tokio::process::Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}
