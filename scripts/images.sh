#!/bin/sh
# Images invariant (.bboxy/invariants/images-resolve.md, decision #52): the face
# in .bboxy/images/bboxy.png is the identity file byte for byte, and every image
# wikilink ![[name.ext]] under .bboxy names a file in .bboxy/images/.
set -eu
cd "$(dirname "$0")/.."
FACE=bc5cc774612f29a452967a5a4fa993292092a59b58dfd03dfb7ddb6b3286c370
bad=0
actual=$(shasum -a 256 .bboxy/images/bboxy.png | cut -c1-64)
[ "$actual" = "$FACE" ] || { echo "images: .bboxy/images/bboxy.png is not the face ($actual)"; bad=1; }
for ref in $(grep -rhoE '!\[\[[^]|#]+' .bboxy --include='*.md' | sed 's/^!\[\[//' | grep -iE '\.(png|jpe?g|gif|svg|webp)$' | sort -u); do
    [ -f ".bboxy/images/$ref" ] || { echo "images: ![[${ref}]] has no file in .bboxy/images/"; bad=1; }
done
# Every tracked app icon is generated from the face: a .source-sha256 sidecar equal to FACE.
for icon in $(git ls-files | grep -iE '(\.icns$|\.iconset/|/AppIcon[^/]*$)' | sed 's#/\.iconset/.*#.iconset#' | sort -u); do
    side="$icon.source-sha256"
    if [ ! -f "$side" ] || [ "$(tr -d '[:space:]' < "$side")" != "$FACE" ]; then echo "images: $icon is not generated from the face (missing or wrong $side)"; bad=1; fi
done
echo "images: $([ "$bad" -eq 0 ] && echo held || echo VIOLATED)"
exit "$bad"
