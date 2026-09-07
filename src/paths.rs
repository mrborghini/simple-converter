//! Filesystem locations used by the app.

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use directories::ProjectDirs;

/// Directory that holds the managed ffmpeg install (`<data dir>/ffmpeg`).
pub fn ffmpeg_dir() -> anyhow::Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "mrborghini", "simple-converter")
        .ok_or_else(|| anyhow!("could not determine the application data directory"))?;
    Ok(dirs.data_dir().join("ffmpeg"))
}

/// Default output path: same folder and stem as the input with the new extension.
/// A ` (n)` suffix is added when that file already exists (or is the input itself).
pub fn default_output(input: &Path, ext: &str) -> PathBuf {
    let dir = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());

    let candidate = dir.join(format!("{stem}.{ext}"));
    if candidate != input && !candidate.exists() {
        return candidate;
    }
    let mut n = 1u32;
    loop {
        let candidate = dir.join(format!("{stem} ({n}).{ext}"));
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "simple-converter-test-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn uses_new_extension_next_to_input() {
        let dir = temp_dir("ext");
        let input = dir.join("clip.mkv");
        std::fs::write(&input, b"x").unwrap();
        assert_eq!(default_output(&input, "mp4"), dir.join("clip.mp4"));
    }

    #[test]
    fn avoids_overwriting_existing_or_input() {
        let dir = temp_dir("collide");
        let input = dir.join("song.mp3");
        std::fs::write(&input, b"x").unwrap();
        assert_eq!(default_output(&input, "mp3"), dir.join("song (1).mp3"));
        std::fs::write(dir.join("song (1).mp3"), b"x").unwrap();
        assert_eq!(default_output(&input, "mp3"), dir.join("song (2).mp3"));
    }
}
