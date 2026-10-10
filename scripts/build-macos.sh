#!/bin/bash
set -euo pipefail

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-13.0}"

for architecture in aarch64 x86_64; do
    cargo build --locked --release --target "$architecture-apple-darwin" -p niz-cli -p niz-gui
done

output_dir="$project_dir/dist"
app_dir="$output_dir/NIZ Utility.app"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources/licenses"
lipo -create target/aarch64-apple-darwin/release/niz-gui target/x86_64-apple-darwin/release/niz-gui -output "$app_dir/Contents/MacOS/niz-gui"
lipo -create target/aarch64-apple-darwin/release/niz target/x86_64-apple-darwin/release/niz -output "$output_dir/niz"
chmod 755 "$app_dir/Contents/MacOS/niz-gui" "$output_dir/niz"
# Use the macOS toolchain for Mach-O stripping before signing the fat binaries.
/usr/bin/strip -S "$app_dir/Contents/MacOS/niz-gui" "$output_dir/niz"

cat > "$app_dir/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>NIZ Utility</string>
<key>CFBundleDisplayName</key><string>NIZ Utility</string>
<key>CFBundleIdentifier</key><string>dev.niz.utility</string>
<key>CFBundleExecutable</key><string>niz-gui</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>$MACOSX_DEPLOYMENT_TARGET</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
cp THIRD_PARTY_NOTICES.md "$app_dir/Contents/Resources/"
cp licenses/niz-web-MIT.txt "$app_dir/Contents/Resources/licenses/"
cp assets/fonts/OFL.txt "$app_dir/Contents/Resources/licenses/NotoSansJP-OFL.txt"
codesign --force --sign - "$app_dir"
codesign --force --sign - "$output_dir/niz"
codesign --verify --strict "$app_dir"
codesign --verify --strict "$output_dir/niz"
lipo "$app_dir/Contents/MacOS/niz-gui" -verify_arch arm64 x86_64
lipo "$output_dir/niz" -verify_arch arm64 x86_64
plutil -lint "$app_dir/Contents/Info.plist"
ditto -c -k --sequesterRsrc --keepParent "$app_dir" "$output_dir/NIZ-Utility-universal.zip"
printf 'App: %s\nCLI: %s\n' "$app_dir" "$output_dir/niz"
