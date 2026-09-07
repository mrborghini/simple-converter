//! Running an ffmpeg conversion and streaming its progress.

use std::path::PathBuf;
use std::process::Stdio;

use anyhow::{Context, bail};
use iced::futures::channel::mpsc::Sender;
use iced::futures::{SinkExt, Stream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use super::tokio_command;

#[derive(Debug, Clone)]
pub enum ConvertEvent {
    Progress { percent: Option<f32>, speed: String },
    Done(PathBuf),
    Failed(String),
}

pub struct Job {
    pub ffmpeg: PathBuf,
    pub args: Vec<String>,
    pub output: PathBuf,
    pub duration_secs: Option<f64>,
}

/// Runs ffmpeg. Dropping the stream (e.g. aborting the task) kills the process.
pub fn convert(job: Job) -> impl Stream<Item = ConvertEvent> {
    iced::stream::channel(64, async move |mut out| {
        let event = match run(job, &mut out).await {
            Ok(path) => ConvertEvent::Done(path),
            Err(e) => ConvertEvent::Failed(format!("{e:#}")),
        };
        let _ = out.send(event).await;
    })
}

async fn run(job: Job, out: &mut Sender<ConvertEvent>) -> anyhow::Result<PathBuf> {
    let mut child = tokio_command(&job.ffmpeg)
        .args(&job.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .context("starting ffmpeg")?;

    let stdout = child.stdout.take().context("ffmpeg stdout unavailable")?;
    let mut stderr = child.stderr.take().context("ffmpeg stderr unavailable")?;
    let stderr_task = tokio::spawn(async move {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text).await;
        text
    });

    let mut lines = BufReader::new(stdout).lines();
    let mut out_time_us: Option<u64> = None;
    let mut speed = String::new();
    while let Some(line) = lines.next_line().await? {
        if let Some(v) = line.strip_prefix("out_time_us=") {
            out_time_us = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("speed=") {
            speed = v.trim().to_string();
        } else if line.starts_with("progress=") {
            let percent = match (out_time_us, job.duration_secs) {
                (Some(t), Some(d)) if d > 0.0 => {
                    Some(((t as f64 / 1_000_000.0 / d) * 100.0).clamp(0.0, 100.0) as f32)
                }
                _ => None,
            };
            let _ = out
                .send(ConvertEvent::Progress {
                    percent,
                    speed: speed.clone(),
                })
                .await;
        }
    }

    let status = child.wait().await.context("waiting for ffmpeg")?;
    let stderr_text = stderr_task.await.unwrap_or_default();
    if status.success() {
        return Ok(job.output);
    }
    let tail: Vec<&str> = stderr_text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .rev()
        .take(12)
        .collect();
    let tail: Vec<&str> = tail.into_iter().rev().collect();
    let code = status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "a signal".to_string());
    if tail.is_empty() {
        bail!("ffmpeg exited with {code}");
    }
    bail!("ffmpeg exited with {code}:\n{}", tail.join("\n"));
}
