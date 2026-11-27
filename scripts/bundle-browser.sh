#!/bin/sh
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Inspired by cef-rs's macOS bundler, https://github.com/tauri-apps/cef-rs.
set -eu
action=$1
contents=$2/Contents
frameworks=$contents/Frameworks
framework_name='Chromium Embedded Framework.framework'
framework=$frameworks/$framework_name
helpers='Helper
Helper (Alerts)
Helper (GPU)
Helper (Plugin)
Helper (Renderer)'

case "$action" in
    copy|remove)
        rm -rf -- "$framework"
        printf '%s\n' "$helpers" | while IFS= read -r helper; do
            rm -rf -- "$frameworks/bboxy450-term $helper.app"
        done
        rm -f -- "$contents/Resources/CEF-LICENSE.txt" "$contents/Resources/Chromium-CREDITS.html" "$contents/Resources/browser-ATTRIBUTION.md"
        rmdir "$frameworks" 2>/dev/null || true
        ;;
esac

case "$action" in
    copy)
        # Read the CEF version pinned by Cargo, using the same CEF_PATH layout
        # as cef-dll-sys. Reuse the already extracted distribution.
        version=$(awk '$0 == "name = \"cef-dll-sys\"" {getline; sub(/.*\+/, ""); sub(/".*/, ""); print; exit}' Cargo.lock)
        case "$(uname -m)" in
            arm64) platform=cef_macos_aarch64 ;;
            x86_64) platform=cef_macos_x86_64 ;;
            *) echo 'unsupported macOS architecture for CEF' >&2; exit 2 ;;
        esac
        distribution=${CEF_PATH:?}/$version/$platform
        if [ ! -d "$distribution" ] && [ -f "$CEF_PATH/archive.json" ]; then
            distribution=$CEF_PATH
        fi
        source=$distribution/$framework_name
        # The minimal cef-rs archive flattens the framework. Restore the macOS
        # signing layout while preserving paths used by the CEF loader.
        if [ -d "$source/Versions" ]; then
            mkdir -p "$frameworks"
            cp -cRP "$source" "$framework"
        else
            version_dir=$framework/Versions/A
            mkdir -p "$version_dir"
            cp -cRP "$source/." "$version_dir"
            ln -s A "$framework/Versions/Current"
            for entry in 'Chromium Embedded Framework' Resources Libraries; do
                ln -s "Versions/Current/$entry" "$framework/$entry"
            done
        fi
        cp packaging/macos/CEF-LICENSE.txt packaging/macos/browser-ATTRIBUTION.md "$contents/Resources/"
        cp "$distribution/CREDITS.html" "$contents/Resources/Chromium-CREDITS.html"

        printf '%s\n' "$helpers" | while IFS= read -r helper; do
            executable="bboxy450-term $helper"
            suffix=$(printf '%s' "$helper" | tr -cd '[:alpha:]')
            dir=$frameworks/$executable.app/Contents
            mkdir -p "$dir/MacOS"
            cp -c "$contents/MacOS/bboxy450-term" "$dir/MacOS/$executable"
            plutil -create xml1 "$dir/Info.plist"
            plutil -insert CFBundleExecutable -string "$executable" "$dir/Info.plist"
            plutil -insert CFBundleIdentifier -string "com.bboxy450.term.$suffix" "$dir/Info.plist"
            plutil -insert CFBundlePackageType -string APPL "$dir/Info.plist"
            plutil -insert CFBundleVersion -string 1 "$dir/Info.plist"
            plutil -insert LSUIElement -bool true "$dir/Info.plist"
        done
        ;;
    sign)
        identity=$3
        if [ "$identity" = - ]; then
            set --
        else
            set -- --options runtime
        fi
        for library in "$framework"/Libraries/*.dylib; do
            codesign --force --sign "$identity" --timestamp=none "$library"
        done
        codesign --force --sign "$identity" --timestamp=none "$framework"
        printf '%s\n' "$helpers" | while IFS= read -r helper; do
            codesign --force --sign "$identity" "$@" --timestamp=none \
                --entitlements packaging/macos/browser-helper.entitlements \
                "$frameworks/bboxy450-term $helper.app"
        done
        ;;
    remove) ;;
    *) echo 'usage: bundle-browser.sh copy|remove|sign APP [IDENTITY]' >&2; exit 2 ;;
esac
