//! Finding an existing ffmpeg: the app-managed copy or one on `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::{FfmpegPaths, Source, std_command};
use crate::paths;

pub const EXE: &str = std::env::consts::EXE_SUFFIX;

/// Written next to the managed binaries after a successful install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub asset: String,
    pub sha256: String,
    pub label: String,
    pub installed_at: u64,
}

pub fn bin_dir(dir: &Path) -> PathBuf {
    dir.join("bin")
}

pub fn ffmpeg_path(dir: &Path) -> PathBuf {
    bin_dir(dir).join(format!("ffmpeg{EXE}"))
}

pub fn ffprobe_path(dir: &Path) -> PathBuf {
    bin_dir(dir).join(format!("ffprobe{EXE}"))
}

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join("manifest.json")
}

pub fn find_managed() -> Option<FfmpegPaths> {
    let dir = paths::ffmpeg_dir().ok()?;
    let ffmpeg = ffmpeg_path(&dir);
    let ffprobe = ffprobe_path(&dir);
    if !(ffmpeg.is_file() && ffprobe.is_file() && runs(&ffmpeg)) {
        return None;
    }
    let label = read_manifest(&dir)
        .map(|m| m.label)
        .unwrap_or_else(|| "unknown".to_string());
    Some(FfmpegPaths {
        ffmpeg,
        ffprobe,
        source: Source::Managed { label },
    })
}

pub fn find_system() -> Option<FfmpegPaths> {
    let ffmpeg = which::which("ffmpeg").ok()?;
    let ffprobe = which::which("ffprobe").ok()?;
    runs(&ffmpeg).then_some(FfmpegPaths {
        ffmpeg,
        ffprobe,
        source: Source::System,
    })
}

/// `ffmpeg -version` exits successfully.
pub fn runs(ffmpeg: &Path) -> bool {
    std_command(ffmpeg)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn read_manifest(dir: &Path) -> Option<Manifest> {
    let bytes = fs::read(manifest_path(dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write_manifest(dir: &Path, manifest: &Manifest) -> anyhow::Result<()> {
    let json = serde_json::to_vec_pretty(manifest)?;
    fs::write(manifest_path(dir), json).context("writing ffmpeg manifest")
}
