//! Downloading a static ffmpeg build from BtbN/FFmpeg-Builds and installing it.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow, bail};
use sha2::{Digest, Sha256};

use super::locate::{self, EXE, Manifest};
use super::{FfmpegPaths, Source};

const RELEASE_URL: &str = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";
const CHECKSUMS: &str = "checksums.sha256";
/// Human readable label of the ffmpeg branch that is installed.
pub const LABEL: &str = "n9.0";
#[cfg(windows)]
pub const ASSET: &str = "ffmpeg-n9.0-latest-win64-gpl-9.0.zip";
#[cfg(not(windows))]
pub const ASSET: &str = "ffmpeg-n9.0-latest-linux64-gpl-9.0.tar.xz";

const PROGRESS_STEP: u64 = 256 * 1024;

/// Downloads, verifies and installs ffmpeg + ffprobe into `dir/bin`.
/// `on_progress(done, total)` is called while downloading; `on_extract` once extraction starts.
pub fn install(
    dir: &Path,
    on_progress: impl Fn(u64, Option<u64>),
    on_extract: impl FnOnce(),
) -> anyhow::Result<FfmpegPaths> {
    let tmp = dir.join("tmp");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
    let result = install_inner(dir, &tmp, on_progress, on_extract);
    let _ = fs::remove_dir_all(&tmp);
    result
}

fn install_inner(
    dir: &Path,
    tmp: &Path,
    on_progress: impl Fn(u64, Option<u64>),
    on_extract: impl FnOnce(),
) -> anyhow::Result<FfmpegPaths> {
    let agent = agent();
    let expected = fetch_expected_sha256(&agent).context("fetching checksum list")?;

    let archive = tmp.join(ASSET);
    let actual = download_file(
        &agent,
        &format!("{RELEASE_URL}/{ASSET}"),
        &archive,
        on_progress,
    )
    .context("downloading ffmpeg")?;
    if actual != expected {
        bail!("checksum mismatch for {ASSET}: expected {expected}, got {actual}");
    }

    on_extract();
    let staged = tmp.join("bin");
    fs::create_dir_all(&staged)?;
    extract(&archive, &staged).context("extracting ffmpeg")?;
    let _ = fs::remove_file(&archive);

    let ffmpeg = staged.join(format!("ffmpeg{EXE}"));
    let ffprobe = staged.join(format!("ffprobe{EXE}"));
    if !(ffmpeg.is_file() && ffprobe.is_file()) {
        bail!("the archive did not contain bin/ffmpeg and bin/ffprobe");
    }
    make_executable(&ffmpeg)?;
    make_executable(&ffprobe)?;

    let bin = locate::bin_dir(dir);
    if bin.exists() {
        fs::remove_dir_all(&bin).context("removing the previous ffmpeg")?;
    }
    fs::rename(&staged, &bin).context("moving ffmpeg into place")?;
    locate::write_manifest(
        dir,
        &Manifest {
            asset: ASSET.to_string(),
            sha256: actual,
            label: LABEL.to_string(),
            installed_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        },
    )?;

    Ok(FfmpegPaths {
        ffmpeg: locate::ffmpeg_path(dir),
        ffprobe: locate::ffprobe_path(dir),
        source: Source::Managed {
            label: LABEL.to_string(),
        },
    })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(30)))
        .timeout_recv_response(Some(Duration::from_secs(60)))
        .timeout_recv_body(Some(Duration::from_secs(60 * 30)))
        .user_agent("simple-converter")
        .build()
        .new_agent()
}

fn fetch_expected_sha256(agent: &ureq::Agent) -> anyhow::Result<String> {
    let mut response = agent.get(format!("{RELEASE_URL}/{CHECKSUMS}")).call()?;
    let text = response.body_mut().read_to_string()?;
    parse_checksum(&text, ASSET).ok_or_else(|| anyhow!("{CHECKSUMS} has no entry for {ASSET}"))
}

/// Finds the lowercase hex digest for `asset` in a `sha256sum`-style listing.
pub fn parse_checksum(text: &str, asset: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == asset && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

/// Streams `url` into `dest`, returning the SHA-256 of the written bytes.
fn download_file(
    agent: &ureq::Agent,
    url: &str,
    dest: &Path,
    on_progress: impl Fn(u64, Option<u64>),
) -> anyhow::Result<String> {
    let mut response = agent.get(url).call()?;
    let total = response.body().content_length();
    let mut reader = response.body_mut().as_reader();
    let mut file = BufWriter::new(File::create(dest)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    let mut last_report = 0u64;
    on_progress(0, total);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if done - last_report >= PROGRESS_STEP {
            last_report = done;
            on_progress(done, total);
        }
    }
    file.flush()?;
    on_progress(done, total);
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Maps an archive entry path to the file name it should get in `bin/`, if we want it.
fn wanted(entry_path: &str) -> Option<String> {
    let mut parts = entry_path.rsplit(['/', '\\']);
    let file = parts.next()?;
    let parent = parts.next()?;
    if parent != "bin" {
        return None;
    }
    let ffmpeg = format!("ffmpeg{EXE}");
    let ffprobe = format!("ffprobe{EXE}");
    (file == ffmpeg || file == ffprobe).then(|| file.to_string())
}

fn extract(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    if ASSET.ends_with(".zip") {
        extract_zip(archive, dest)
    } else {
        extract_tar_xz(archive, dest)
    }
}

fn extract_zip(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let Some(target) = wanted(entry.name()) else {
            continue;
        };
        let mut out = File::create(dest.join(target))?;
        io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn extract_tar_xz(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    let tar_path = archive.with_extension("");
    {
        let mut input = BufReader::new(File::open(archive)?);
        let mut output = BufWriter::new(File::create(&tar_path)?);
        lzma_rs::xz_decompress(&mut input, &mut output)
            .map_err(|e| anyhow!("xz decompression failed: {e:?}"))?;
        output.flush()?;
    }
    let mut tar = tar::Archive::new(BufReader::new(File::open(&tar_path)?));
    for entry in tar.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.to_string_lossy().into_owned();
        let Some(target) = wanted(&path) else {
            continue;
        };
        let mut out = File::create(dest.join(target))?;
        io::copy(&mut entry, &mut out)?;
    }
    let _ = fs::remove_file(&tar_path);
    Ok(())
}

fn make_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("marking {} executable", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_checksum_lines() {
        let hash = "a".repeat(64);
        let text = format!("{hash}  {ASSET}\n{}  other.zip\n", "b".repeat(64));
        assert_eq!(parse_checksum(&text, ASSET), Some(hash));
        assert_eq!(parse_checksum(&text, "missing.zip"), None);
    }

    #[test]
    fn accepts_binary_mode_marker_and_uppercase() {
        let hash = "AB".repeat(32);
        let text = format!("{hash} *{ASSET}\n");
        assert_eq!(
            parse_checksum(&text, ASSET),
            Some(hash.to_ascii_lowercase())
        );
    }

    #[test]
    fn selects_only_binaries_in_bin() {
        let ffmpeg = format!("ffmpeg{EXE}");
        assert_eq!(
            wanted(&format!("ffmpeg-n9.0-latest/bin/{ffmpeg}")),
            Some(ffmpeg.clone())
        );
        assert_eq!(
            wanted(&format!("ffmpeg-n9.0-latest\\bin\\{ffmpeg}")),
            Some(ffmpeg.clone())
        );
        assert_eq!(wanted("ffmpeg-n9.0-latest/bin/ffplay"), None);
        assert_eq!(wanted(&format!("ffmpeg-n9.0-latest/doc/{ffmpeg}")), None);
        assert_eq!(wanted(&ffmpeg), None);
    }
}
