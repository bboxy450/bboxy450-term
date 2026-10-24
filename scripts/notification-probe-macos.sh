#!/bin/sh
# Build only. Oracle schedules the operator's native permission/click session.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
if [ "$(uname -s)" != Darwin ]; then echo 'notification probe requires macOS' >&2; exit 2; fi
cargo build --locked -p bboxy450-term --features notification-probe --example notification_probe
target_root=${CARGO_TARGET_DIR:-target}
bundle=${1:-"$target_root/debug/bboxy450-notification-probe.app"}
case "$bundle" in *.app) ;; *) echo 'probe output must end in .app' >&2; exit 2;; esac
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp packaging/macos/Info.plist "$bundle/Contents/Info.plist"
plutil -replace CFBundleIdentifier -string com.bboxy450.term.notification-probe "$bundle/Contents/Info.plist"
plutil -replace CFBundleExecutable -string notification_probe "$bundle/Contents/Info.plist"
plutil -replace CFBundleName -string bboxy450-notification-probe "$bundle/Contents/Info.plist"
plutil -replace CFBundleDisplayName -string bboxy450-notification-probe "$bundle/Contents/Info.plist"
cp "$target_root/debug/examples/notification_probe" "$bundle/Contents/MacOS/notification_probe"
cp NOTICE LICENSE-MIT LICENSE-APACHE "$bundle/Contents/Resources/"
plutil -lint "$bundle/Contents/Info.plist"
codesign --force --sign - --timestamp=none "$bundle"
codesign --verify --strict --verbose=2 "$bundle"
printf '%s\n' "$bundle"
