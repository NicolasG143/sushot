#!/bin/sh
# Builds target/appimage/klip-x86_64.AppImage from a release build.
# Needs appimagetool on PATH and the X11 development libraries listed in README.md.
set -eu

root=$(cd "$(dirname "$0")/../.." && pwd)
appdir="$root/target/appimage/AppDir"
output="$root/target/appimage/klip-x86_64.AppImage"

cargo build --release --manifest-path "$root/Cargo.toml" --target x86_64-unknown-linux-gnu
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin"
cp "$root/target/x86_64-unknown-linux-gnu/release/klip" "$appdir/usr/bin/klip"
cp "$root/packaging/linux/klip.desktop" "$appdir/klip.desktop"
cp "$root/packaging/msix/Assets/Square150x150Logo.png" "$appdir/klip.png"
ln -s usr/bin/klip "$appdir/AppRun"

ARCH=x86_64 appimagetool "$appdir" "$output"
echo "AppImage: $output"
