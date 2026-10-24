#!/bin/sh
# One global sequence for repository records (decision 48). Allocation races
# are rejected by the corpus gate; this prints a candidate, not a reservation.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
numbers=$(find .bboxy -type f -name '*.md' -exec awk '
    FNR == 1 {
        sub(/^\357\273\277/, "")
        sub(/\r$/, "")
        front = ($0 == "---")
        next
    }
    front {
        sub(/\r$/, "")
        if ($0 == "---") { front = 0; next }
        if ($0 ~ /^number:/) {
            value = $0
            sub(/^number:[[:space:]]*/, "", value)
            sub(/[[:space:]]*#.*/, "", value)
            sub(/[[:space:]]*$/, "", value)
            if (value !~ /^[1-9][0-9]*$/) {
                print FILENAME ": invalid record number" > "/dev/stderr"
                exit 1
            }
            print value
        }
    }
' {} +)
printf '%s\n' "$numbers" | awk 'max < $1 { max = $1 } END { printf "%.0f\n", max + 1 }'
