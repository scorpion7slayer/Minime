#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_bundle="$project_dir/dist/Minime.app"
temporary_root="$(mktemp -d)"
iconset_dir="$temporary_root/Minime.iconset"
staged_bundle="$temporary_root/Minime.app"

cleanup() {
  rm -rf "$temporary_root"
}
trap cleanup EXIT

cd "$project_dir"
cargo build --release

mkdir -p "$staged_bundle/Contents/MacOS" "$staged_bundle/Contents/Resources"
cp "$project_dir/target/release/minime" "$staged_bundle/Contents/MacOS/minime"
cp "$project_dir/packaging/macos/Info.plist" "$staged_bundle/Contents/Info.plist"
cp "$project_dir/assets/minime.svg" "$staged_bundle/Contents/Resources/minime.svg"
chmod +x "$staged_bundle/Contents/MacOS/minime"

mkdir -p "$iconset_dir"
for icon_size in 16 32 128 256 512; do
  double_size=$((icon_size * 2))
  sips -s format png -z "$icon_size" "$icon_size" \
    "$project_dir/assets/minime.svg" \
    --out "$iconset_dir/icon_${icon_size}x${icon_size}.png" >/dev/null
  sips -s format png -z "$double_size" "$double_size" \
    "$project_dir/assets/minime.svg" \
    --out "$iconset_dir/icon_${icon_size}x${icon_size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset_dir" -o "$staged_bundle/Contents/Resources/MinimeAppIcon.icns"
codesign --force --deep --sign - "$staged_bundle"

mkdir -p "$project_dir/dist"
rm -rf "$app_bundle"
mv "$staged_bundle" "$app_bundle"
touch "$app_bundle"

echo "Created $app_bundle"
