#!/bin/sh
# Run the one-line shell checks in invariant frontmatter. README/body text is not executable.
set -eu
invariant_tools=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
invariant_root=$(CDPATH='' cd -- "$invariant_tools/.." && pwd)

frontmatter_field() {
    awk -v key="$1" '
        NR == 1 { if ($0 != "---") exit; next }
        $0 == "---" { exit }
        index($0, key ":") == 1 { sub(/^[^:]*:[[:space:]]*/, ""); print; exit }
    ' "$2"
}

run_slow=false
run_parity=false
run_cargo=true
selected_gate=''
affected_set=false
affected_crates=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        --slow) run_slow=true; shift ;;
        --parity) run_parity=true; shift ;;
        --no-cargo) run_cargo=false; shift ;;
        --affected)
            [ "$#" -ge 2 ] || { echo 'invariants: --affected requires one crate-list argument' >&2; exit 2; }
            [ "$affected_set" = false ] || { echo 'invariants: select --affected once' >&2; exit 2; }
            affected_set=true; affected_crates=$2; shift 2 ;;
        --root)
            [ "$#" -ge 2 ] || { echo 'invariants: --root requires a candidate directory' >&2; exit 2; }
            invariant_root=$2; shift 2 ;;
        --*) printf 'unknown invariant flag: %s\n' "$1" >&2; exit 2 ;;
        *)
            case "$1" in ''|*[!A-Za-z0-9_-]*) echo 'invariants: invalid gate name' >&2; exit 2 ;; esac
            [ -z "$selected_gate" ] || { echo 'invariants: select one gate' >&2; exit 2; }
            selected_gate=$1; shift ;;
    esac
done
cd -- "$invariant_root"
if [ -f "$invariant_tools/cache-env.sh" ]; then
    # shellcheck source=scripts/cache-env.sh
    . "$invariant_tools/cache-env.sh"
else
    export RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER=''
fi
if [ -n "$selected_gate" ]; then
    selected_file=".bboxy/invariants/$selected_gate.md"
    [ -f "$selected_file" ] && [ "$(frontmatter_field type "$selected_file")" = invariant ] || {
        printf 'invariants: unknown gate: %s\n' "$selected_gate" >&2; exit 2;
    }
fi

# One Cargo test graph; exact libtest filters retain each invariant's own verdict.
exec python3 -I - "$invariant_tools" "$run_slow" "$run_parity" "$run_cargo" "$selected_gate" "$affected_set" "$affected_crates" <<'PY_INVARIANTS'
import collections
import glob
import importlib.util
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tomllib

sys.dont_write_bytecode = True
tools, slow, parity, cargo_enabled, selected, affected_set, affected_arg = sys.argv[1:]
spec = importlib.util.spec_from_file_location("landing", Path(tools) / "landing.py")
landing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(landing)

# The observer is optional and never part of the invariant verdict.
ledger_helper = Path(tools) / "resource-ledger.py"
ledger = landing.ledger_module()
if ledger is not None and not os.environ.get("BBOXY_LEDGER_SAMPLER"):
    try:
        os.environ["BBOXY_LEDGER_SAMPLER"] = str(ledger.start_observer(os.getpid()))
    except Exception:
        pass


def metered(argv):
    if ledger_helper.exists():
        return [sys.executable, "-I", str(ledger_helper), "run", "integration", "--", *argv]
    return argv



def fields(path):
    lines = path.read_text().splitlines()
    result = {}
    if lines and lines[0] == "---":
        for line in lines[1:]:
            if line == "---":
                break
            if ":" in line and not line[0].isspace():
                key, value = line.split(":", 1)
                result[key] = value.strip()
    return result


def workspace_names():
    manifest = tomllib.loads(Path("Cargo.toml").read_text())
    paths = {Path("Cargo.toml")} if "package" in manifest else set()
    workspace = manifest.get("workspace", {})
    excluded = {Path(p) for pattern in workspace.get("exclude", []) for p in glob.glob(pattern)}
    for pattern in workspace.get("members", []):
        paths.update(Path(p) / "Cargo.toml" for p in glob.glob(pattern) if Path(p) not in excluded)
    return {tomllib.loads(path.read_text())["package"]["name"] for path in paths}


def packages(words):
    return {word for i, word in enumerate(words) if i and words[i - 1] in ("-p", "--package")}


def check_packages(command):
    words = shlex.split(command)
    if landing.cargo_backed(command) and "cargo" not in words:
        # The only delegated Cargo check is ui-lint.sh; inspect its actual -p inputs.
        words = shlex.split(Path("scripts/ui-lint.sh").read_text())
    return packages(words) or workspace_names()


def test_check(command):
    words = shlex.split(command)
    env = {}
    if words and words[0] == "env":
        words.pop(0)
    while words and re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*=.*", words[0]):
        key, value = words.pop(0).split("=", 1)
        env[key] = value
    if words[:2] != ["cargo", "test"]:
        return None
    words = words[2:]
    result = {"packages": [], "targets": [], "env": env, "grep": False}
    while words:
        word = words.pop(0)
        if word in ("-p", "--package", "--test", "--bin") and words:
            value = words.pop(0)
            result["packages" if word in ("-p", "--package") else "targets"].append(
                value if word in ("-p", "--package") else (word, value))
        elif word == "--lib":
            result["targets"].append(("--lib",))
        elif word == "--locked":
            pass
        else:
            result["test"] = word
            break
    if not result["packages"] or not result["targets"] or not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9:]*", result.get("test", "")):
        return None
    if words == ["--", "--exact", "2>&1", "|", "grep", "-q", "ok. 1 passed; 0 failed"]:
        result["grep"] = True
    elif words != ["--", "--exact"]:
        return None
    return result


def run_batch(checks):
    argv = ["cargo", "test", "--locked", "--no-fail-fast"]
    if os.environ.get("BBOXY_CARGO_TARGET_DIR"):
        argv += ["--target-dir", os.environ["BBOXY_CARGO_TARGET_DIR"]]
    for package in sorted({p for check in checks for p in check["packages"]}):
        argv += ["-p", package]
    for target in sorted({t for check in checks for t in check["targets"]}):
        argv += list(target)
    argv += ["--", "--exact", *dict.fromkeys(check["test"] for check in checks)]
    env = dict(os.environ)
    for check in checks:
        env.update(check["env"])
    print("invariants: Cargo test batch:", shlex.join(argv), flush=True)
    observed = collections.defaultdict(list)
    try:
        child = subprocess.Popen(metered(argv), env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    except OSError as error:
        print("invariants: Cargo test batch unavailable:", error, file=sys.stderr)
        return 127, observed
    for line in child.stdout:
        print(line, end="", flush=True)
        match = re.fullmatch(r"test (\S+) \.\.\. (ok|FAILED|ignored)\s*", re.sub(r"\x1b\[[0-9;]*m", "", line))
        if match:
            observed[match[1]].append(match[2])
    child.stdout.close()
    code = child.wait()
    return (code if code >= 0 else 128 - code), observed


# Keep explicit target forwarding for legacy/delegated checks as well as the batch.
shell_check = r'''
cargo() {
    subcommand=$1
    shift
    if [ -n "${BBOXY_CARGO_TARGET_DIR:-}" ]; then
        command cargo "$subcommand" --target-dir "$BBOXY_CARGO_TARGET_DIR" "$@"
    else
        command cargo "$subcommand" "$@"
    fi
}
eval "$1"
'''
try:
    affected = set(affected_arg.split()) if affected_set == "true" else None
    if affected:
        unknown = affected - workspace_names()
        if unknown:
            raise ValueError("unknown affected crates: " + " ".join(sorted(unknown)))
    entries = []
    for path in sorted(Path(".bboxy/invariants").glob("*.md")):
        data = fields(path)
        if data.get("type") != "invariant" or (selected and path.stem != selected):
            continue
        entry = {"name": path.stem, "command": data.get("check", ""), "skip": None, "batch": None}
        if data.get("slow") == "true" and slow == "false" and not selected and (parity == "false" or data.get("gate") != "parity"):
            entry["skip"] = "slow"
        elif landing.cargo_backed(entry["command"]):
            if cargo_enabled == "false":
                entry["skip"] = "code tier"
            elif affected is not None and not (affected & check_packages(entry["command"])):
                entry["skip"] = "unaffected crates"
            else:
                entry["batch"] = test_check(entry["command"])
        if selected and entry["skip"]:
            raise ValueError("selected gate excluded: " + entry["name"] + " (" + entry["skip"] + ")")
        entries.append(entry)
    batches = [entry["batch"] for entry in entries if entry["batch"]]
    batch_result = None
    counts = collections.Counter()
    for entry in entries:
        name, command = entry["name"], entry["command"]
        if entry["skip"]:
            reason = entry["skip"]
            print(f"invariant {name}: skipped ({reason})", flush=True)
            counts[reason] += 1
            continue
        counts["checks"] += 1
        if not command:
            print(f"invariant {name}: fail (missing check)", flush=True)
            counts["failures"] += 1
            continue
        if entry["batch"]:
            if batch_result is None:
                batch_result = run_batch(batches)
            code, observed = batch_result
            check = entry["batch"]
            results = observed[check["test"]]
            code = 0 if results == ["ok"] else (1 if check["grep"] else code or 1)
            if results != ["ok"]:
                print(f"invariants: {name} expected one passing {check['test']}; observed {results}", file=sys.stderr)
        else:
            code = subprocess.run(metered(["sh", "-c", shell_check, "invariant-check", command])).returncode
        if code < 0:
            code = 128 - code
        print(f"invariant {name}: " + ("pass" if code == 0 else f"fail (exit {code})"), flush=True)
        counts["failures"] += code != 0
    print(f"invariants: {counts['checks']} checks, {counts['failures']} failures, {counts['slow']} slow checks skipped", flush=True)
    if cargo_enabled == "false":
        print(f"invariants: {counts['code tier']} code-tier checks skipped", flush=True)
    if affected is not None:
        print(f"invariants: {counts['unaffected crates']} unaffected-crate checks skipped", flush=True)
    sys.exit(1 if counts["failures"] else 0)
except (ValueError, OSError) as error:
    print("invariants:", error, file=sys.stderr)
    sys.exit(2)
PY_INVARIANTS
