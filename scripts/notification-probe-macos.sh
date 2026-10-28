#!/bin/sh
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Build only. Oracle schedules the operator's native permission/click session.
# Origin: bboxy450-term. Retain credit and upstream terms on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
make check-licenses
if [ "$(uname -s)" != Darwin ]; then echo 'notification probe requires macOS' >&2; exit 2; fi
identity=3943AF130A015B36824B7FBC86AA672E7B25F3C4
if ! security find-identity -v -p codesigning | grep -F "$identity" >/dev/null; then
    echo "notification-probe: approved Apple Development identity $identity is unavailable" >&2
    exit 2
fi
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
cp LICENSE NOTICE ATTRIBUTION.md LICENSE-MIT LICENSE-APACHE "$bundle/Contents/Resources/"
cp packaging/macos/bboxy.icns "$bundle/Contents/Resources/"
plutil -lint "$bundle/Contents/Info.plist"
codesign --force --sign "$identity" --options runtime --timestamp=none "$bundle"
codesign --verify --strict --verbose=2 "$bundle"
printf '%s\n' "$bundle"
