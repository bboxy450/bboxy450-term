#!/bin/sh
# Explicit app packaging; cargo run and cargo test keep their ordinary runner.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
if [ "$(uname -s)" != Darwin ]; then
    echo 'bundle-macos: macOS is required' >&2
    exit 2
fi
identity=3943AF130A015B36824B7FBC86AA672E7B25F3C4
if ! security find-identity -v -p codesigning | grep -F "$identity" >/dev/null; then
    echo "bundle-macos: approved Apple Development identity $identity is unavailable" >&2
    exit 2
fi
profile=${1:-debug}
case "$profile" in
    debug) cargo build --locked -p bboxy450-term ;;
    release) cargo build --locked -p bboxy450-term --release ;;
    *) echo 'usage: sh scripts/bundle-macos.sh [debug|release] [output.app]' >&2; exit 2 ;;
esac
target_root=${CARGO_TARGET_DIR:-target}
bundle=${2:-"$target_root/$profile/bboxy450-term.app"}
case "$bundle" in
    *.app) ;;
    *) echo 'bundle-macos: output must end in .app' >&2; exit 2 ;;
esac
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp packaging/macos/Info.plist "$bundle/Contents/Info.plist"
cp "$target_root/$profile/bboxy450-term" "$bundle/Contents/MacOS/bboxy450-term"
cp NOTICE LICENSE-MIT LICENSE-APACHE "$bundle/Contents/Resources/"
cp packaging/macos/bboxy.icns "$bundle/Contents/Resources/"
plutil -lint "$bundle/Contents/Info.plist"
codesign --force --sign "$identity" --options runtime --timestamp=none "$bundle"
codesign --verify --strict --verbose=2 "$bundle"
printf '%s\n' "$bundle"
