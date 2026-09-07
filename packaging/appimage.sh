#!/usr/bin/env bash
# Packages target/release/simple-converter as out/simple-converter-<version>-linux-x86_64.AppImage.
# Requires a release build. Downloads appimagetool into target/ when it is not installed.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

version=${VERSION:-$(grep -m1 -E '^version = "[^"]+"' Cargo.toml | cut -d'"' -f2)}
binary=target/release/simple-converter
if [ ! -x "$binary" ]; then
  echo "missing $binary; run 'cargo build --release' first" >&2
  exit 1
fi

appdir=AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/256x256/apps"
cp "$binary" "$appdir/usr/bin/simple-converter"
cp assets/simple-converter.desktop "$appdir/simple-converter.desktop"
cp assets/simple-converter.desktop "$appdir/usr/share/applications/simple-converter.desktop"
cp assets/icon.png "$appdir/simple-converter.png"
cp assets/icon.png "$appdir/usr/share/icons/hicolor/256x256/apps/simple-converter.png"
ln -sf simple-converter.png "$appdir/.DirIcon"
ln -sf usr/bin/simple-converter "$appdir/AppRun"

extra_flags=()
if [ -n "${APPIMAGETOOL:-}" ]; then
  tool=$APPIMAGETOOL
elif command -v appimagetool >/dev/null 2>&1; then
  tool=appimagetool
else
  tool="$root/target/appimagetool-x86_64.AppImage"
  if [ ! -x "$tool" ]; then
    echo "downloading appimagetool" >&2
    curl -fsSL -o "$tool" \
      https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
    chmod +x "$tool"
  fi
  # Works without FUSE (GitHub runners, containers).
  extra_flags=(--appimage-extract-and-run)
fi

mkdir -p out
output="out/simple-converter-$version-linux-x86_64.AppImage"
ARCH=x86_64 "$tool" "${extra_flags[@]}" "$appdir" "$output"
rm -rf "$appdir"
echo "built $output"
