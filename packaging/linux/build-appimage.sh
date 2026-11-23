#!/bin/sh
# Builds target/appimage/sushot-x86_64.AppImage from a release build.
# Needs appimagetool on PATH and the X11 development libraries listed in README.md.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
appdir="$root/target/appimage/AppDir"
output="$root/target/appimage/sushot-x86_64.AppImage"

cargo build --release --manifest-path "$root/Cargo.toml" --target x86_64-unknown-linux-gnu
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin"
cp "$root/target/x86_64-unknown-linux-gnu/release/sushot" "$appdir/usr/bin/sushot"
cp "$root/packaging/linux/sushot.desktop" "$appdir/sushot.desktop"
cp "$root/packaging/msix/Assets/Square150x150Logo.png" "$appdir/sushot.png"
ln -s usr/bin/sushot "$appdir/AppRun"

ARCH=x86_64 appimagetool "$appdir" "$output"
echo "AppImage: $output"
