#!/bin/sh
set -eu

identity=3943AF130A015B36824B7FBC86AA672E7B25F3C4
binary=$1
shift
stem=${binary##*/}
suffix=${stem##*-}
# Cargo test hashes change independently of the program's stable signing identity.
case "$suffix" in
    ????????????????)
        case "$suffix" in
            *[!0-9a-fA-F]*) ;;
            *) stem=${stem%-*} ;;
        esac
        ;;
esac
identifier=com.bboxy450.$stem

# A matching signature needs neither private-key lookup nor re-signing.
requirement="=identifier \"$identifier\" and certificate leaf = H\"$identity\""
if /usr/bin/codesign --verify --strict --test-requirement "$requirement" "$binary" >/dev/null 2>&1; then
    exec "$binary" "$@"
fi

identities=$(/usr/bin/security find-identity -v -p codesigning 2>/dev/null || true)
case "$identities" in
    *"$identity"*) ;;
    *)
        printf 'sign-run: development signing identity unavailable; executing without signing\n' >&2
        exec "$binary" "$@"
        ;;
esac

/usr/bin/codesign --force --sign "$identity" --identifier "$identifier" --timestamp=none "$binary"
exec "$binary" "$@"
