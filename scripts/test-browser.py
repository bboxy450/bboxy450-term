#!/usr/bin/env python3
"""Private installed Chromium, exercised through authenticated seat MCP.

Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
Adapted from the private admission fixture in crates/bboxy450-term/tests/dream_read.rs.
"""
import base64
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shlex
import signal
import struct
import subprocess
import sys
import tempfile
import time
from urllib.parse import quote


def until(probe, description, seconds=8):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if found := probe():
            return found
        time.sleep(0.025)
    raise AssertionError(description)


TOOLS = {"browser_open", "browser_cdp", "browser_screenshot", "browser_close"}
ADMISSION = ("BBOXY_SOCK", "BBOXY_SEAT_ID", "BBOXY_GENERATION", "BBOXY_CAPABILITY_FILE")
INIT = {
    "protocolVersion": "2025-11-25",
    "capabilities": {},
    "clientInfo": {"name": "private-browser-fixture", "version": "1"},
}


class PrivateApp:
    def __init__(self, bundle, root):
        self.bundle = bundle
        self.bin = bundle / "Contents/MacOS/bboxy450-term"
        self.root = root
        self.run = root / "run"
        self.work = root / "repository"
        self.home = root / "home"
        self.env = {
            "HOME": str(self.home), "ZDOTDIR": str(self.home),
            "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "TERM": "xterm-256color",
            "LANG": "en_US.UTF-8",
        }
        for path in (self.work / ".bboxy", self.home):
            path.mkdir(parents=True, mode=0o700)
        # Capture only paths to this fixture seat's own admission. The real
        # installed bridge reads its capability file directly.
        probe = (
            "import json,os,pathlib;"
            "seat=os.environ['BBOXY_SEAT_ID'];"
            f"path=pathlib.Path({str(root)!r}, seat+'.admission');"
            "temp=path.with_suffix('.tmp');"
            f"temp.write_text(json.dumps({{k:os.environ[k] for k in {ADMISSION!r}}}));"
            "temp.replace(path)"
        )
        (self.home / ".zshrc").write_text(
            f"/usr/bin/python3 -c {shlex.quote(probe)}\nexec /bin/cat\n"
        )
        subprocess.run(["/usr/bin/git", "init", "-q", self.work],
                       env=self.env, timeout=10, check=True)
        self.log = root / "app.log"
        with self.log.open("w") as log:
            self.app = subprocess.Popen(
                [self.bin, "--headless", "--ctl", "--unattended", "--test-account",
                 root / "account", "--run-dir", self.run, "--repository", self.work],
                env=self.env, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                start_new_session=True,
            )

    def ctl(self, *args, optional=False):
        out = subprocess.run([self.bin, "ctl", "--run-dir", self.run, *args],
                             env=self.env, capture_output=True, text=True, timeout=12)
        if out.returncode and optional:
            return None
        assert out.returncode == 0, (args, out.stdout[-2000:], out.stderr[-2000:])
        return json.loads(out.stdout)["data"]

    def start(self):
        def ready():
            assert self.app.poll() is None, "private headless app exited early"
            status = self.ctl("app", "status", optional=True)
            if status and status["backend_ready"] and status["gui_ready"]:
                return status
            return None

        status = until(ready, "headless private app did not start", 40)
        assert status["windows"] == 0 and status["custody_backend"] == "file", status
        self.swarm = self.ctl("ui", "layout")["swarm"]
        self.ctl("ui", "swarm-workdir", self.swarm, self.work)

    def seat(self, name, role):
        seat = self.ctl("seat", "spawn", "zsh", "--place", "tab", "--name", name,
                        "--swarm", self.swarm, "--cwd", self.work, "--role", role)["seat"]
        path = self.root / (seat + ".admission")
        until(path.exists, "private seat admission did not arrive")
        admission = json.loads(path.read_text())
        assert admission["BBOXY_SEAT_ID"] == seat
        return seat, self.env | admission

    def mcp(self, environment, method, params=None):
        frames = [
            {"id": 1, "method": "initialize", "params": INIT},
            {"method": "notifications/initialized"},
            {"id": 2, "method": method, "params": params or {}},
        ]
        wire = "".join(json.dumps({"jsonrpc": "2.0", **frame}) + "\n" for frame in frames)
        result = subprocess.run([self.bin, "mcp"], env=environment, input=wire,
                                capture_output=True, text=True, timeout=13, check=True)
        replies = {frame["id"]: frame for line in result.stdout.splitlines()
                   if "id" in (frame := json.loads(line))}
        assert "result" in replies[1] and "result" in replies[2], replies
        return replies[2]["result"]

    def tool(self, seat, name, error=None, **arguments):
        response = self.mcp(seat, "tools/call", {"name": name, "arguments": arguments})
        data = response["structuredContent"]
        if error:
            assert response["isError"] and data["error"]["code"] == error, (name, data)
        else:
            assert not response["isError"], (name, data)
        return data

    def evaluate(self, seat, expression, **arguments):
        result = self.tool(seat, "browser_cdp", method="Runtime.evaluate",
                           params={"expression": expression, "returnByValue": True}, **arguments)
        return result["result"]["result"]["value"]

    def screenshot(self, seat, full_page=False):
        def capture():
            response = self.mcp(seat, "tools/call", {
                "name": "browser_screenshot",
                "arguments": {"full_page": full_page},
            })
            data = response["structuredContent"]
            if response["isError"] and data["error"]["code"] == "provider_busy":
                return None
            assert not response["isError"], data
            return data

        artifact = until(capture, "screenshot publisher did not become available")
        return self.png(seat, artifact)

    def png(self, seat, artifact):
        assert artifact["media_type"] == "image/png" and artifact["revision"] == 1, artifact
        link = self.tool(seat, "artifact_read", id=artifact["id"])["resource"]["uri"]
        image = bytearray()
        while link:
            result = self.mcp(seat, "resources/read", {"uri": link})
            part, = result["contents"]
            assert part["mimeType"] == "image/png" and part["_meta"]["bboxy450/offset"] == len(image)
            image += base64.b64decode(part["blob"], validate=True)
            link = part["_meta"].get("bboxy450/next")
        assert len(image) == artifact["bytes"] and image[:8] == b"\x89PNG\r\n\x1a\n"
        return image

    def rows(self):
        processes = subprocess.check_output(["/bin/ps", "-axww", "-o", "pid=,pgid=,command="],
                                           text=True, timeout=3)
        owned = []
        for line in processes.splitlines():
            pid, pgid, argv = line.split(maxsplit=2)
            if argv.startswith(str(self.bundle) + "/Contents/"):
                owned.append((int(pid), int(pgid), argv))
        return owned

    def browsers(self):
        return [(pid, pgid, argv) for pid, pgid, argv in self.rows()
                if " browser-host " in argv or "/Contents/Frameworks/" in argv]

    def host(self, seat):
        pid, pgid, argv = until(
            lambda: next((row for row in self.browsers() if f"/browser-{seat}-" in row[2]), None),
            "owned browser host not observed",
        )
        assert pid == pgid
        return pid, Path(argv.split(" browser-host ", 1)[1])

    def stop(self):
        self.ctl("app", "stop", optional=True)
        self.app.wait(timeout=12)
        until(lambda: not self.browsers(), "backend stop left browser processes")
        assert not list(self.run.glob("browser-*")), "backend stop left a browser profile"

    def cleanup(self):
        if self.app.poll() is None:
            try:
                self.ctl("app", "stop", optional=True)
            except subprocess.TimeoutExpired:
                pass
        # The install path is a newly created fixture directory. Only exact
        # processes observed executing binaries from this private bundle qualify.
        for pid, _, _ in self.rows():
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        self.app.wait(timeout=10)


def page(title):
    return "data:text/html," + quote(
        f"<title>{title}</title><style>html{{background:#1b5cb5}}"
        "body{margin:0;min-height:1440px}</style>", safe=""
    )


def sample_png(app, data, height):
    png = app.root / "captured.png"
    bmp = png.with_suffix(".bmp")
    png.write_bytes(data)
    bmp.unlink(missing_ok=True)
    subprocess.run(["/usr/bin/sips", "-s", "format", "bmp", png, "--out", bmp],
                   capture_output=True, timeout=5, check=True)
    image = bmp.read_bytes()
    assert image[:2] == b"BM"
    offset, = struct.unpack_from("<I", image, 10)
    width, rows, planes, bits, compression = struct.unpack_from("<iiHHI", image, 18)
    assert (width, abs(rows)) == (1024, height), (width, rows)
    assert planes == 1 and bits in (24, 32) and compression in (0, 3)
    stride = ((width * bits + 31) // 32) * 4
    center = offset + stride * (height // 2) + (width // 2) * (bits // 8)
    assert image[center:center + 3] == bytes((181, 92, 27)), image[center:center + 3].hex()


def pending(app, pool, seat, expected_error):
    def request(expression, timeout_ms, await_promise=False):
        response = app.mcp(seat, "tools/call", {
            "name": "browser_cdp",
            "arguments": {
                "method": "Runtime.evaluate",
                "params": {"expression": expression, "awaitPromise": await_promise},
                "timeout_ms": timeout_ms,
            },
        })
        data = response["structuredContent"]
        return data["error"]["code"] if response["isError"] else None, data

    def hold():
        def acquire():
            error, data = request("new Promise(() => {})", 8000, True)
            # The probe can reach the backend first and briefly own the slot.
            if error == "browser_busy":
                return None
            assert error == expected_error, data
            return True

        until(acquire, "fixture request kept losing admission to its probe", 6)

    held = pool.submit(hold)

    def admitted():
        assert not held.done(), ("fixture request finished before the admission probe", held.result())
        return request("1", 100)[0] == "browser_busy"

    until(admitted, "fixture request did not reach the per-seat flight slot", 6)
    return held


def verify(app):
    app.start()
    first_id, first = app.seat("First browser", "executor")
    second_id, second = app.seat("Second browser", "oracle")
    listed = {tool["name"] for tool in app.mcp(first, "tools/list")["tools"]}
    assert TOOLS <= listed
    assert app.tool(first, "orient")["actor"]["role"]["class"] == "worker"
    assert app.tool(second, "orient")["actor"]["role"]["class"] == "oracle"
    assert not app.browsers(), "browser started before browser_open"
    app.tool(second, "browser_cdp", "browser_absent", method="Runtime.evaluate")

    data = app.tool(first, "browser_open", url=page("Chromium A"))
    assert data["title"] == "Chromium A" and data["url"].startswith("data:text/html,")
    assert data["http_status"] is None
    first_pid, first_profile = app.host(first_id)
    assert app.evaluate(first, "document.title") == "Chromium A"
    assert app.evaluate(first, "(globalThis.boxyPrivate = 42)") == 42

    app.tool(second, "browser_open", url=page("Chromium B"))
    second_pid, _ = app.host(second_id)
    assert first_pid != second_pid
    assert app.evaluate(second, "document.title + ':' + typeof boxyPrivate") == "Chromium B:undefined"
    assert app.evaluate(first, "document.title") == "Chromium A"
    until(lambda: any("Helper (Renderer)" in row[2] for row in app.browsers()),
          "bundled renderer helper was never observed")

    big = app.tool(first, "browser_cdp", method="Runtime.evaluate",
                   params={"expression": "'x'.repeat(550000)", "returnByValue": True})
    assert big["result"] is None and big["cut"], big.keys()

    sample_png(app, app.screenshot(first), 768)
    sample_png(app, app.screenshot(first, full_page=True), 1440)

    app.tool(first, "browser_cdp", "browser_timeout", method="Runtime.evaluate", timeout_ms=35,
             params={"expression": "new Promise(r => setTimeout(() => r('stale'), 250))",
                     "awaitPromise": True})
    later = app.tool(first, "browser_cdp", method="Runtime.evaluate",
                     params={"expression": "new Promise(r => setTimeout(() => r('current'), 550))",
                             "awaitPromise": True, "returnByValue": True})
    assert later["result"]["result"]["value"] == "current", later
    print("private MCP read two isolated pages and decoded viewport and full-page PNG pixels")

    with ThreadPoolExecutor(max_workers=1) as pool:
        crashed = pending(app, pool, first, "browser_disconnected")
        os.kill(first_pid, signal.SIGKILL)
        crashed.result(timeout=6)
        assert not first_profile.exists()
        app.tool(first, "browser_open", url=page("Chromium B"))
        restarted, restarted_profile = app.host(first_id)
        assert restarted != first_pid and restarted_profile != first_profile
        assert app.evaluate(first, "document.title + ':' + typeof boxyPrivate") == "Chromium B:undefined"
        assert app.evaluate(second, "document.title") == "Chromium B"
        assert app.tool(first, "browser_close")["closed"] is True
        assert not restarted_profile.exists()
        app.tool(first, "browser_open", url=page("Chromium A"))
        restarted, restarted_profile = app.host(first_id)
        assert app.evaluate(first, "document.title + ':' + typeof boxyPrivate") == "Chromium A:undefined"

        closing = pending(app, pool, first, "browser_closed")
        app.ctl("seat", "close", first_id)
        closing.result(timeout=5)
        assert not restarted_profile.exists()
        assert not any(row[1] == restarted for row in app.browsers())
    assert app.tool(second, "browser_close")["closed"] is True
    until(lambda: not app.browsers(), "explicit/seat close left browser hosts or helpers")
    assert not list(app.run.glob("browser-*"))
    assert app.tool(second, "browser_close")["closed"] is False

    # The backend owner must close an otherwise live browser too.
    app.tool(second, "browser_open", url=page("Backend shutdown"))
    app.host(second_id)
    app.stop()
    print("late reply fenced; killed host restarted; seat, tool and backend close reaped owned processes")


def main():
    bundle = Path(sys.argv[1]).resolve()
    assert bundle.parent.name.startswith("bboxy-browser-install."), "requires a private install"
    subprocess.run(["/usr/bin/codesign", "--verify", "--deep", "--strict", bundle], check=True)
    for name in ("CEF-LICENSE.txt", "Chromium-CREDITS.html"):
        assert (bundle / "Contents/Resources" / name).stat().st_size > 1000
    assert (bundle / "Contents/Resources/browser-ATTRIBUTION.md").is_file()
    with tempfile.TemporaryDirectory(prefix="bboxy-browser-", dir="/tmp") as root:
        app = PrivateApp(bundle, Path(root).resolve())
        try:
            verify(app)
        except BaseException:
            print(app.log.read_text(errors="replace")[-6000:], file=sys.stderr)
            raise
        finally:
            app.cleanup()


if __name__ == "__main__":
    main()
