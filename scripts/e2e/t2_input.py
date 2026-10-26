"""T2 drives the app's real seat; its receiver reads only its own PTY."""
import hashlib
import json
import os
from pathlib import Path
import shlex
import sys
import termios
import tty
import time
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ui import App, pane, require


def read_exact(fd, size):
    data = bytearray()
    while len(data) < size:
        chunk = os.read(fd, size - len(data))
        require(chunk, "receiver reached EOF before its input completed")
        data.extend(chunk)
    return bytes(data)


def receiver(directory, token, typed_size):
    """No expected contents enter the receiver: output is the actual input."""
    require(os.isatty(0) and os.isatty(1), "receiver requires its seat's real PTY")
    directory = Path(directory)
    saved = termios.tcgetattr(0)
    try:
        tty.setraw(0)
        os.write(1, b"\x1b[2J\x1b[H\x1b[?2004h" +
                 f"T2_READY {token}\r\n".encode())
        typed = read_exact(0, typed_size)
        (directory / "typed.out").write_bytes(typed)
        os.write(1, b"T2_ECHO " + typed + b"\r\n" +
                 f"T2_PASTE_READY {token}\r\n".encode())
        require(read_exact(0, 6) == b"\x1b[200~", "paste lacks its opening bracket")
        received = bytearray()
        end = b"\x1b[201~"
        while end not in received:
            chunk = os.read(0, 32768)
            require(chunk, "receiver reached EOF inside bracketed paste")
            received.extend(chunk)
            require(len(received) <= 1_000_000, "paste exceeded receiver's byte bound")
        payload, trailer = received.split(end, 1)
        require(not trailer, "input followed the paste's closing bracket")
        (directory / "paste.out").write_bytes(payload)
        digest = hashlib.sha256(payload).hexdigest()
        (directory / "receiver.json").write_text(json.dumps({
            "typed_bytes": len(typed), "paste_bytes": len(payload), "sha256": digest,
            "pid": os.getpid(), "tty": os.ttyname(0),
        }, indent=2) + "\n")
        os.write(1, f"T2_DONE {token} {len(payload)}\r\n".encode())
    finally:
        os.write(1, b"\x1b[?2004l")
        termios.tcsetattr(0, termios.TCSANOW, saved)


def main():
    app = App()
    app.ctl("ui", "clock", "freeze")
    seat = app.spawn("T2 exact input")
    token = uuid.uuid4().hex
    typed = "typed-" + token[:16]
    command = ["exec", "python3", "-I", "-B", str(Path(__file__).resolve()),
               "--receiver", str(app.work), token, str(len(typed))]
    app.line(seat, shlex.join(command))
    app.wait(seat, "(?m)^T2_READY " + token + "$")
    app.ctl("seat", "type", seat, typed, label="t2-typed")
    app.wait(seat, "(?m)^T2_ECHO " + typed + "$")
    require((app.work / "typed.out").read_bytes() == typed.encode(),
            "typed input did not reach the program exactly")
    echo = app.ctl("seat", "screen", seat, label="t2-echo-screen")
    deadline = time.monotonic() + 30
    while True:
        layout = app.ctl("ui", "layout")
        rendered = pane(layout, seat).get("rendered")
        if rendered and rendered["sequence"] >= echo["sequence"]:
            break
        require(time.monotonic() < deadline, "no produced-frame source for the echo")
        time.sleep(0.01)  # watchdog polling does not define the frame bound
    assert_echo(layout, seat, echo, typed)
    (app.work / "t2-echo-frame.json").write_text(json.dumps(layout, indent=2) + "\n")
    app.wait(seat, "(?m)^T2_PASTE_READY " + token + "$")
    line = b"0123456789abcdefghijklmnopqrstuvwxyz-ABCDEFGHIJKLMNOPQRSTUVWXYZ+0123456789:.\n"
    source = (line * 1300)[:100000]
    path = app.work / "paste.src"
    path.write_bytes(source)
    app.ctl("seat", "paste", seat, path, label="t2-paste")
    app.wait(seat, f"(?m)^T2_DONE {token} 100000$")
    actual = (app.work / "paste.out").read_bytes()
    require(actual == source,
            f"paste differs: expected {len(source)} bytes, received {len(actual)}")
    print("T2: real PTY received exact typed bytes and all 100000 paste bytes; "
          "the first frame after echo ingestion drew that sequence or a later one")


def assert_echo(layout, seat, echo, typed):
    visible = pane(layout, seat)
    require(layout["focused"] == seat and visible["focused"], "input pane lost focus")
    rendered = visible.get("rendered")
    require(rendered is not None and rendered.get("lines") is not None,
            "the produced frame has no observed terminal text")
    require("T2_ECHO " + typed in rendered["lines"], "the produced frame did not draw the program's echo")
    require(rendered["view"]["generation"] == echo["generation"] and
            rendered["sequence"] >= echo["sequence"], "the produced grid predates the echo screen")
    ingested = rendered["ingested"]
    require(ingested["sequence"] >= echo["sequence"], "echo ingestion was not observed")
    require(ingested["first_frame_seq"] == ingested["after_frame_seq"] + 1,
            "echo missed the next produced frame after ingestion")
    require(ingested["view"] == rendered["view"] == ingested["first_drawn_view"] and
            ingested["first_drawn_sequence"] >= ingested["sequence"],
            "the first frame after ingestion drew an older screen")


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--receiver":
        receiver(sys.argv[2], sys.argv[3], int(sys.argv[4]))
    else:
        main()
