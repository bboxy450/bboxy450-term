# bboxy450-term browser bundle

Project: https://github.com/bboxy450/bboxy450-term

Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

### Bundled Chromium on macOS

Browser builds distribute the unmodified CEF framework binaries and resources
from the distribution pinned by `cef-dll-sys`, plus the distro’s Chromium and
third-party credits. The app ships `Contents/Resources/CEF-LICENSE.txt` from
[CEF e603d97](https://github.com/chromiumembedded/cef/blob/e603d97/LICENSE.txt)
under BSD-3-Clause and `Contents/Resources/Chromium-CREDITS.html` from that
distribution. The macOS helper layout was inspired by
[cef-rs](https://github.com/tauri-apps/cef-rs), whose Rust bindings are
MIT or Apache-2.0. The helper executables are built from this project’s source.
