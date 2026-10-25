#!/bin/sh
# Invariant #283 (e2e-runs-unattended): the night runs with nobody present. For every group of
# e2e-acceptance §1 the app's own runner runs with stdin closed and a bound, and held is all
# three, whatever the verdicts (red promises are the checklist's business, not this check's):
#   1. it ends by verdict: every group exits 0 or 1, never 2 (refused), never killed at the bound;
#   2. every promise is written: no unwritten record, one e2e record per §1 promise;
#   3. no operator identity: every record's run_by is test-account:, its signer a device, and
#      every app the night started reported custody_backend file and windows 0.
# Usage: sh scripts/e2e-unattended.sh   BBT names the app binary; E2E_GROUP_BOUND seconds
# per group (default 1200). Nothing is built.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
bin=${BBT:-target/debug/bboxy450-term}
[ -x "$bin" ] || { echo "e2e-unattended: no app binary at $bin; the night builds nothing (#282)" >&2; exit 2; }
bound=${E2E_GROUP_BOUND:-1200}
root=$(mktemp -d "${TMPDIR:-/tmp}/e2eu.XXXXXX")
chmod 700 "$root"
# A night stopped at the bound leaves no app behind: the runner's first line names its
# private root, and every app under it that opened ctl is stopped by its own endpoint.
stop_apps() {
    cat "$root"/groups/*.jsonl 2>/dev/null | python3 -I -c '
import json, sys
for line in sys.stdin:
    try:
        row = json.loads(line)
    except ValueError:
        continue
    if row.get("kind") == "night" and row.get("root"):
        print(row["root"])' | while IFS= read -r night; do
        find "$night" -name gui.endpoint 2>/dev/null | while IFS= read -r endpoint; do
            "$bin" ctl --run-dir "$(dirname "$endpoint")" app stop >/dev/null 2>&1 || true
        done
    done
}
trap 'stop_apps; rm -rf "${root:?}"' EXIT
mkdir "$root/records" "$root/groups"
spec=.bboxy/specs/e2e-acceptance.md
sed -n '/^## 1\./,/^## [^1]/s/^### //p' "$spec" > "$root/groups.txt"
[ -s "$root/groups.txt" ] || { echo "e2e-unattended: no groups in $spec §1" >&2; exit 2; }
n=0
while IFS= read -r group; do
    n=$((n + 1))
    out="$root/groups/$n.jsonl"
    printf '%s\n' "$group" > "$root/groups/$n.name"
    # The bound is the only exit a hung night has: SIGALRM (exit 142) means it waited.
    code=0
    perl -e 'alarm shift; exec @ARGV or die' "$bound" \
        "$bin" e2e night --group "$group" --scripts scripts/e2e --records "$root/records" \
        </dev/null >"$out" 2>"$root/groups/$n.err" || code=$?
    case $code in
        0 | 1) ;;
        142) echo "e2e-unattended: VIOLATED (ends by verdict): group '$group' still running after ${bound}s" >&2; exit 1 ;;
        *) echo "e2e-unattended: VIOLATED (ends by verdict): group '$group' exit $code: $(tail -3 "$root/groups/$n.err")" >&2; exit 1 ;;
    esac
done < "$root/groups.txt"
python3 -I - "$spec" "$root" <<'EOF'
import json, os, re, sys
spec, root = sys.argv[1], sys.argv[2]
text = open(spec).read()
section = text.split("\n## 1.", 1)[1].split("\n## ", 1)[0]
promises, group = {}, None
for line in section.splitlines():
    if line.startswith("### "):
        group = line[4:].strip()
    elif (m := re.match(r"- \*\*([A-Z][0-9]+)\*\*", line)) and group:
        promises[m.group(1)] = group
bad = []
groups_dir = os.path.join(root, "groups")
seen = set()
for name in sorted(f for f in os.listdir(groups_dir) if f.endswith(".jsonl")):
    rows = [json.loads(l) for l in open(os.path.join(groups_dir, name)) if l.startswith("{")]
    label = open(os.path.join(groups_dir, name[:-6] + ".name")).read().strip()
    summary = [r for r in rows if r.get("kind") == "summary"]
    if len(summary) != 1:
        bad.append(f"{label}: {len(summary)} summary lines")
    elif summary[0].get("unwritten"):
        bad.append(f"{label}: unwritten records {summary[0]['unwritten']}")
    for row in rows:
        if row.get("kind") == "e2e":
            seen.add(row["function"])
        if row.get("kind") == "app":
            if row.get("custody_backend") != "file" or row.get("windows") != 0:
                bad.append(f"{label}: app {row.get('role')} {row.get('function')} ran custody "
                           f"{row.get('custody_backend')} with {row.get('windows')} windows")
            if not str(row.get("issued_by", "")).startswith("test-account:"):
                bad.append(f"{label}: app {row.get('function')} issued_by {row.get('issued_by')}")
missing = sorted(set(promises) - seen)
if missing:
    bad.append(f"promises the runner never reported: {missing}")
records = os.path.join(root, "records", ".bboxy", "e2e")
for function in sorted(promises):
    path = os.path.join(records, f"{function}.md")
    if not os.path.isfile(path):
        bad.append(f"{function}: no e2e record written")
        continue
    front = open(path).read().split("\n---", 1)[0]
    run_by = re.search(r"^run_by: *'?([^'\n]*)", front, re.M)
    if not run_by or not run_by.group(1).startswith("test-account:"):
        bad.append(f"{function}: run_by {run_by.group(1) if run_by else None}")
    if not re.search(r"^  signer: device:", front, re.M):
        bad.append(f"{function}: record not signed by a device")
if bad:
    print("e2e-unattended: VIOLATED", file=sys.stderr)
    for line in bad:
        print("  " + line, file=sys.stderr)
    sys.exit(1)
groups = len([f for f in os.listdir(groups_dir) if f.endswith(".jsonl")])
print(f"e2e-unattended: held ({groups} groups, {len(promises)} promises written, every record "
      f"test-account signed, every app file custody, no window)")
EOF
