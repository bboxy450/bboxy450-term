"""Native display-link right-edge detector and recorded real-Codex replay.

No desktop capture, GUI injection, real identity or network model. Reads only
the private app's normal-frame source/composition/drawable band copies. Raw
session bytes and pixels remain in the throwaway run, never source control.
"""
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import argparse
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile
import time


class StripDetected(AssertionError):
    def __init__(self, result):
        self.result = result
        super().__init__(("native right-edge strip", result["failures"]))


def check(directory, minimum=32):
    """Independent cell-ink oracle, then source/copy/final pixel comparisons."""
    assert not list(directory.glob("*.error")), ("native probe readback failed", directory)
    paths = sorted(directory.glob("*.json"))
    assert len(paths) >= minimum, ("incomplete native frame sample", len(paths), minimum)
    samples, failures, ink_frames, cells = [], [], 0, 0
    for path in paths:
        meta = json.loads(path.read_text())
        raw = path.with_suffix(".bin").read_bytes()
        width, height = meta["size"]
        stride = meta["stride"]
        section = stride * height
        assert len(raw) == section * 3, (path, "incomplete readback")
        assert meta["native_display_link"], (path, "not the native display-link path")
        assert meta["surface_configurations"] == 1, ("native resize reconfigured the shared queue", path, meta["surface_configurations"])
        assert meta["target"] == meta["drawable"], (path, "lagging drawable", meta)
        assert meta["source"] == meta["prepared"], (path, "stale raster size", meta)
        cw, ch = meta["cell"]
        if meta["expected"]:
            ink_frames += 1
        for col, row in meta["expected"]:
            # The app's documented unread-error HUD occupies the bottom 42
            # logical pixels. It legitimately covers terminal ink. The detector
            # samples the vertical band above that overlay, including its last
            # columns; it does not claim footer/HUD or physical scanout coverage.
            if meta["pane"][1] + (row + 1) * ch > meta["target"][1] - 44 * meta["scale"]:
                continue
            cells += 1
            ranges = []
            for stage in range(3):
                spread = 0
                for channel in range(3):
                    values = b"".join(raw[stage * section + y * stride + col * cw * 4 + channel:
                                          stage * section + y * stride + (col + 1) * cw * 4:4]
                                       for y in range(row * ch, (row + 1) * ch))
                    spread += max(values) - min(values)
                ranges.append(spread)
            for stage, spread in zip(("source", "copy", "final"), ranges):
                if spread < 60:
                    failures.append(dict(frame=meta["frame"], stage=stage,
                                         cell=[col, row], spread=spread, grid=meta["grid"]))
                    break
        # Exact comparison is diagnostic as overlay pixels may cover a pane.
        copy_differences = sum(raw[y * stride:y * stride + width * 4] !=
                               raw[section + y * stride:section + y * stride + width * 4]
                               for y in range(height))
        final_differences = 0
        for y in range(height):
            composed = raw[section + y * stride:section + y * stride + width * 4]
            presented = raw[2 * section + y * stride:2 * section + y * stride + width * 4]
            final_differences += any(composed[c::4] != presented[d::4]
                                     for c, d in ((0, 2), (1, 1), (2, 0), (3, 3)))
        samples.append(dict(frame=meta["frame"], grid=meta["grid"], scale=meta["scale"],
                            target=meta["target"], pane=meta["pane"],
                            source_current=meta["source_current"],
                            layer=meta.get("layer"), rejected_drawables=meta["rejected_drawables"],
                            layer_resizes=meta["layer_resizes"],
                            ink_cells=len(meta["expected"]), copy_differing_rows=copy_differences,
                            final_differing_rows=final_differences))
    result = dict(frames=len(paths), ink_frames=ink_frames, ink_cells=cells,
                  grids=sorted({tuple(s["grid"]) for s in samples}),
                  targets=sorted({tuple(s["target"]) for s in samples}),
                  scales=sorted({s["scale"] for s in samples}),
                  stale_sources=sum(not s["source_current"] for s in samples),
                  rejected_drawables=max(s["rejected_drawables"] for s in samples),
                  layer_resizes=max(s["layer_resizes"] for s in samples),
                  layer_mismatch_frames=sum(
                      s["layer"] is None or s["layer"]["drawable"] != s["target"] or
                      [round(v * s["layer"]["contents_scale"]) for v in s["layer"]["bounds"]] != s["target"]
                      for s in samples),
                  final_differing_rows=sum(s["final_differing_rows"] for s in samples),
                  failures=failures[:8], missing_ink_cells=len(failures), samples=samples)
    (directory.parent / "native-strip-evidence.json").write_text(json.dumps(result, indent=2) + "\n")
    assert ink_frames >= 8 and cells >= 64, ("no discriminating right-edge ink", directory,
                                           len(paths), ink_frames, cells)
    if failures:
        raise StripDetected(result)
    return result


def replay(binary, fixture, control=None, cycles=4, stress=False):
    root = Path(tempfile.mkdtemp(prefix="bbt-native-strip-", dir="/tmp")).resolve()
    print("Private native replay: " + str(root), flush=True)
    for name in ("home", "repository", "frames", "work", "tmp"):
        (root / name).mkdir()
    env = dict(HOME=str(root / "home"), XDG_CONFIG_HOME=str(root / "home/.config"),
               XDG_DATA_HOME=str(root / "home/.local/share"), XDG_CACHE_HOME=str(root / "home/.cache"),
               CODEX_HOME=str(root / "home/.codex"), CLAUDE_CONFIG_DIR=str(root / "home/.claude"),
               TMPDIR=str(root / "tmp") + "/", PATH="/usr/bin:/bin:/usr/sbin:/sbin",
               TERM="xterm-256color", LANG="en_US.UTF-8", BBOXY_RUN_DIR=str(root / "run"))
    command = [str(binary), "--ctl", "--unattended", "--test-account", str(root / "account"),
               "--run-dir", str(root / "run"), "--repository", str(root / "repository"),
               "--settings-config", str(root / "settings.toml"),
               "--native-frame-probe", str(root / "frames")]
    if control:
        command.extend(["--native-frame-control", control])
    # Replay only recorded real Codex bytes. Input consumes no user credentials.
    # One bounded chunk per 10ms means parser and resize can interleave; complete
    # sync-output batches remain authoritative. Fixture contents stay local.
    driver = root / "replay.py"
    stress_code = ("  if start + 1024 >= len(data):\n"
                   "   cols,rows=os.get_terminal_size(0);row=1+(start//16384)%(rows-1)\n"
                   "   text=f'\\x1b[?2026h\\x1b[?7l\\x1b[{row};1H\\x1b[2K'\n"
                   "   text+=f'\\x1b[{cols-15}G'+ 'W'*16\n"
                   "   text+=f'\\x1b[{cols-9}G👩‍💻中\\x1b[{cols-3}G\\x1b[KWWWW\\x1b[?7h\\x1b[?2026l'\n"
                   "   write(text.encode());time.sleep(.01)\n") if stress else ""
    driver.write_text("import os,time,tty,select\n"
                      "tty.setraw(0)\n"
                      "def write(data):\n"
                      " while data: data=data[os.write(1,data):]\n"
                      "data=open(" + repr(str(fixture)) + ",'rb').read()\n"
                      "for cycle in range(" + str(cycles) + "):\n"
                      " for start in range(0,len(data),1024):\n"
                      "  write(data[start:start+1024]);time.sleep(.01)\n"
                      "  if select.select([0],[],[],0)[0]: os.read(0,65536)\n"
                      + stress_code +
                      "open(" + repr(str(root / "replayed")) + ",'w').write(str(len(data)*" + str(cycles) + "))\n" +
                      "time.sleep(20)\n")
    python = sys.executable
    app = None

    def ctl(*args, check_exit=True):
        p = subprocess.run([str(binary), "ctl", *args], env=env, cwd=root / "work",
                           capture_output=True, text=True, timeout=40)
        assert not check_exit or p.returncode == 0, (args, p.returncode, p.stdout, p.stderr)
        return json.loads(p.stdout) if p.stdout.strip() else {}

    def until(read, accept, seconds=45):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            value = read()
            if accept(value):
                return value
            time.sleep(.1)
        raise AssertionError(("native replay timeout", root, value))

    try:
        with (root / "app.log").open("w") as log:
            app = subprocess.Popen(command, env=env, cwd=root / "work", stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=log)
        until(lambda: ctl("app", "status", check_exit=False),
              lambda r: r.get("data", {}).get("gui_ready") and r["data"].get("backend_ready"))
        seat = ctl("seat", "spawn", "zsh", "--place", "tab", "--cwd", str(root / "work"),
                   "--name", "native Codex replay")["data"]["seat"]
        ctl("ui", "focus", seat)
        ctl("seat", "type", seat, "exec " + shlex.quote(python) + " " + shlex.quote(str(driver)))
        ctl("seat", "key", seat, "enter")
        sizes = ("159x58", "91x31", "180x43", "80x24", "137x39", "159x58")
        observations = []
        for index in range(72):
            if index % 6 == 0:
                ctl("ui", "key", "cmd-equal" if index % 12 == 0 else "cmd-minus")
            response = ctl("app", "resize", sizes[index % len(sizes)])
            observations.append(response["data"])
            time.sleep(.025)
        until(lambda: (root / "replayed").exists(), bool)
        assert int((root / "replayed").read_text()) == fixture.stat().st_size * cycles
        ctl("seat", "capture", seat, str(root / "context.png"))
        until(lambda: len(list((root / "frames").glob("*.json"))), lambda n: n >= 64)
        # All complete 16-frame batches are receipts; the last partial batch is
        # explicitly outside the sample. No scanout or every-future-frame claim.
        result = check(root / "frames", minimum=64)
        result.pop("samples")
        result["resize_requests"] = len(observations)
        result["font_changes"] = 12
        result["fixture_bytes"] = fixture.stat().st_size
        result["replay_cycles"] = cycles
        result["wide_erase_stress"] = stress
        result["root"] = str(root)
        print(json.dumps(result), flush=True)
        return result
    finally:
        if app is not None and app.poll() is None:
            ctl("app", "stop", check_exit=False)
            try:
                app.wait(timeout=15)
            except subprocess.TimeoutExpired:
                app.terminate()
                app.wait(timeout=10)
        if app is not None and (root / "run/logic.owner.json").exists():
            # A native panic detaches by product design. Reattach this fixture's
            # private owners through the supported headless door, then stop
            # them explicitly; never leave a failed probe's PTYs behind.
            recovery = ctl("app", "start", "--headless", "--ctl", "--unattended",
                           "--test-account", str(root / "account"))
            (root / "cleanup-owner.json").write_text(json.dumps(recovery) + "\n")
            ctl("app", "stop")


def night(binary):
    """Fresh real local-Codex recording, native replay and causal GPU mutants."""
    root = Path(tempfile.mkdtemp(prefix="bbt-strip-night-", dir="/tmp")).resolve()
    script = Path(__file__).resolve().with_name("shared_codex.py")
    command = [sys.executable, "-I", "-B", str(script), str(binary), "--native", "--stream-check"]
    with (root / "codex.log").open("w") as log:
        run = subprocess.run(command, stdout=log, stderr=log, timeout=150)
    output = (root / "codex.log").read_text()
    assert run.returncode == 0, ("real native Codex failed", root, output[-3000:])
    match = re.search(r"Private shared-daemon fixture: (.+)", output)
    assert match, ("native Codex recording path missing", root)
    recording = Path(match[1])
    real = check(recording / "frames", minimum=64)
    real.pop("samples")
    fixture = recording / "codex-stream.bin"
    data = fixture.read_bytes()
    assert data.count(b"W") >= 8192 and data.count(b"\x1b[?2026h") >= 20, "incomplete real Codex fixture"
    summary = dict(real_codex=real, fixture_bytes=len(data),
                   synchronized_starts=data.count(b"\x1b[?2026h"),
                   synchronized_ends=data.count(b"\x1b[?2026l"),
                   line_erases=len(re.findall(rb"\x1b\[[012]?K", data)),
                   replay=replay(binary, fixture, cycles=8, stress=True), broken_controls={})
    for control in ("source", "copy", "final"):
        try:
            replay(binary, fixture, control=control)
        except StripDetected as failure:
            assert any(row["stage"] == control for row in failure.result["failures"]), failure
            summary["broken_controls"][control] = dict(
                rejected=True, frames=failure.result["frames"], failures=failure.result["failures"])
        else:
            raise AssertionError(("blank-strip broken control escaped", control))
    (root / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print("Native strip night: " + str(root), flush=True)
    print(json.dumps(summary), flush=True)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary", type=Path)
    p.add_argument("--fixture", type=Path)
    p.add_argument("--check", type=Path)
    p.add_argument("--control", choices=("source", "copy", "final"))
    p.add_argument("--cycles", type=int, default=4)
    p.add_argument("--night", action="store_true")
    p.add_argument("--stress", action="store_true")
    args = p.parse_args()
    if args.night:
        assert args.binary
        night(args.binary.resolve())
    elif args.check:
        result = check(args.check)
        result.pop("samples")
        print(json.dumps(result))
    else:
        assert args.binary and args.fixture
        replay(args.binary.resolve(), args.fixture.resolve(), args.control, args.cycles, args.stress)


if __name__ == "__main__":
    main()
