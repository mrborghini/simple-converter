# Simple Converter

A small desktop app that converts video, audio and image files with [ffmpeg](https://ffmpeg.org),
without you having to install ffmpeg or remember any of its flags.

- Pick a file, pick an output type, press **Convert**.
- **Audio only** checkbox to pull the sound out of a video.
- **Advanced options** for codec, quality (CRF or bitrate), resolution, frame rate, audio bitrate,
  sample rate and channels, plus one-click templates such as *Podcast MP3* or *Web MP4*.
- ffmpeg is fetched automatically on first start if it is not already on your system.

Single static binary, written in Rust with the [iced](https://iced.rs) GUI toolkit. No runtime
dependencies to install on Windows; on Linux only the usual desktop libraries.

## Download

Grab the latest build from the [releases page](https://github.com/mrborghini/simple-converter/releases).

| Platform | File | Notes |
|---|---|---|
| Windows x86_64 | `simple-converter-<version>-windows-x86_64-setup.exe` | Per-user installer, no admin rights needed. |
| Windows x86_64 | `simple-converter-<version>-windows-x86_64-portable.exe` | Single file, run from anywhere. |
| Linux x86_64 | `simple-converter-<version>-linux-x86_64.AppImage` | `chmod +x` and run. |
| Linux x86_64 | `simple-converter-<version>-linux-x86_64.deb` | `sudo apt install ./simple-converter-*.deb` |

The Windows builds are not code-signed, so SmartScreen shows a warning the first time
("More info" then "Run anyway"). The Linux builds are made on Ubuntu 24.04 and need glibc 2.39 or
newer (Ubuntu 24.04, Fedora 40, Debian 13 and anything more recent).

## How ffmpeg is found

On every start the app looks for ffmpeg in this order:

1. A copy it downloaded earlier, stored in its data directory
   (`~/.local/share/simple-converter/ffmpeg` on Linux,
   `%APPDATA%\mrborghini\simple-converter\data\ffmpeg` on Windows).
2. `ffmpeg` and `ffprobe` on your `PATH`.
3. Otherwise it downloads a static GPL build (branch n9.0) from
   [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds), checks the SHA-256 against the
   published checksum file, and installs only `ffmpeg` and `ffprobe`.

**Re-download ffmpeg** in the advanced panel forces a fresh managed copy, for example when a
distro-provided ffmpeg lacks an encoder you need.

## Using it

1. **Choose file...** (or drop a file on the window). The app shows what it found: duration,
   resolution and codecs.
2. Pick an **Output type**. Video, audio and image formats are listed; tick **Audio only** to keep
   just the sound. Inputs without video are audio only automatically.
3. Adjust **Save to** if you like, or use **Save as...**. Picking a name with a different known
   extension switches the output type.
4. **Convert**. A progress bar and speed indicator appear; **Cancel** stops ffmpeg and removes
   the partial file. **Show in folder** opens the result in your file manager.

### Advanced options

- **Template** applies a preset to everything below: podcast/music MP3, voice AAC, lossless FLAC,
  Opus, WAV, web MP4, small 720p MP4, WebM VP9, GIF, or a straight remux to MKV.
- **Video**: codec (auto, copy, H.264, H.265, VP9, AV1), CRF or target bitrate, resolution,
  frame rate and encoder preset. Copying a stream into a container that cannot hold it falls
  back to encoding, and the panel tells you so.
- **Audio**: codec, bitrate, sample rate, channels, or strip the audio entirely.
- **Extra ffmpeg args** are appended verbatim (quotes are honoured).
- **Command preview** shows the exact ffmpeg command line, with a **Copy** button.

### Known limitations

- Drag-and-drop does not work under Wayland (an iced limitation); use the file button instead.
- File dialogs on Linux use the desktop portal and fall back to zenity.
- Everything runs through the ffmpeg CLI, so exotic inputs behave exactly as they do in ffmpeg.

## Building from source

Requires a stable Rust toolchain. On Linux, install `libxkbcommon-dev` and `pkg-config` first.

```sh
cargo run                 # development build
cargo test                # unit tests (argument builder, probe parsing, paths)
cargo build --release     # target/release/simple-converter[.exe]
```

Packaging locally:

```sh
cargo install cargo-deb
cargo deb --no-build      # target/debian/*.deb
bash packaging/appimage.sh  # out/*.AppImage (downloads appimagetool into target/ if needed)
```

The Windows installer is built with Inno Setup: `ISCC.exe /DAppVersion=1.2.3 packaging\installer.iss`.

## Releases

Every push to `main` runs the **Release** workflow:

1. `scripts/release-version.sh` reads the commits since the last `v*` tag and picks the bump:
   `feat!:` or a `BREAKING CHANGE:` footer bumps major, `feat:` bumps minor, anything else patch.
   With no tag yet, the version in `Cargo.toml` is released as-is.
2. The version is written to `Cargo.toml` and `Cargo.lock`, committed as `chore(release): vX.Y.Z`,
   tagged and pushed.
3. Windows and Linux builds run the tests, build the binaries and package them.
4. A **draft** release with all four artifacts and a `SHA256SUMS.txt` is created. Review it on
   GitHub and press *Publish* when you are happy.

The workflow can also be started by hand from the Actions tab with an explicit bump. If `main`
is protected against direct pushes, allow the GitHub Actions bot to push or supply a PAT.
Commits use the [Conventional Commits](https://www.conventionalcommits.org) format.

## License

MIT, see [LICENSE](LICENSE). ffmpeg itself is downloaded separately and is licensed under the GPL.
