#!/bin/sh
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Print the SHA-1 of a valid code-signing identity in this machine's keychain
# whose certificate OU is TEAM, Apple Development first. No default: exit 1
# when none exists.
set -eu
team=$1
certs=$(security find-certificate -a -Z -p)
security find-identity -v -p codesigning |
    awk '/"Apple Development/ {print 1, $2; next} /^ *[0-9]+\)/ {print 2, $2}' |
    sort -s -k1,1 |
    while read -r _ hash; do
        printf '%s\n' "$certs" |
            awk -v h="$hash" '/^SHA-1 hash:/ {on = ($3 == h); next} on' |
            openssl x509 -noout -subject 2>/dev/null |
            grep -Eq "OU ?= ?$team(,|/|$)" && { echo "$hash"; exit 0; }
    done | grep . || { echo "no code-signing identity for team $team in the keychain; set SIGNING_IDENTITY" >&2; exit 1; }
