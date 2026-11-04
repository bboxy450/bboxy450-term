#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Frame formal subcommands for the presubmit's existing two-clock observer."""
import json
import subprocess
import sys


def main():
    step, separator, *command = sys.argv[1:]
    if separator != "--" or not command:
        raise ValueError("formal-stage expects STEP -- COMMAND")
    def emit(event):
        print("formal-phase: " + json.dumps({"event": event, "step": step}), flush=True)
    emit("start")
    try:
        return subprocess.run(command).returncode
    finally:
        emit("end")


if __name__ == "__main__":
    raise SystemExit(main())
