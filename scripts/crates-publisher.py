#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""Publish only with the operator's approved identity bound to this exact token.

crates.io /api/v1/me rejects API-token authentication (cookie-only). The local
receipt must come from an actual operator confirmation, never an agent's guess.
It contains a token fingerprint, never the token, and stays outside Git.
"""
import argparse
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import tomllib
import urllib.error
import urllib.request

USER_AGENT = "bboxy450-term release (https://github.com/bboxy450/bboxy450-term)"


def verify_identity(token, receipt):
    if not token or receipt.get("publisher") != "bboxy450" or receipt.get("source") != "operator-confirmed":
        raise ValueError("approved crates.io publisher identity is not confirmed")
    if receipt.get("token_sha256") != hashlib.sha256(token.encode()).hexdigest():
        raise ValueError("Cargo token changed; the publisher identity binding is stale")


def publication_order(data, names=None):
    members = set(data["workspace_members"])
    packages = {p["name"]: p for p in data["packages"]
                if p["id"] in members and p.get("publish") != []}
    selected = set(names) if names else set(packages)
    if not selected <= packages.keys():
        raise ValueError("selection contains a non-publishable workspace package")
    ordered, visiting, visited = [], set(), set()

    def visit(name):
        if name in visiting:
            raise ValueError("workspace publication dependency cycle: " + name)
        if name in visited:
            return
        visiting.add(name)
        # Metadata uses actual package names, including renamed dependencies,
        # and includes dev/build/platform edges needed by cargo package.
        for dep in packages[name]["dependencies"]:
            if dep["name"] in packages:
                visit(dep["name"])
        visiting.remove(name)
        visited.add(name)
        if name in selected:
            ordered.append(packages[name])

    for name in sorted(selected):
        visit(name)
    return ordered


def retry_delay(headers, message, now=None):
    now = time.time() if now is None else now
    value = headers.get("Retry-After", "")
    match = re.search(r"Retry-After:\s*([^\r\n]+)", message, re.I)
    value = value or (match.group(1).strip() if match else "")
    if value:
        try:
            return max(1, math.ceil(float(value)))
        except ValueError:
            try:
                return max(1, math.ceil(parsedate_to_datetime(value).timestamp() - now))
            except (ValueError, TypeError):
                pass
    match = re.search(r"(?:try again after|retry after)\s+(\d{4}-\d\d-\d\d[T ]\d\d:\d\d:\d\d(?:Z|\+00:00| UTC)?)", message, re.I)
    if match:
        date = datetime.fromisoformat(match.group(1).replace(" UTC", "+00:00"))
        return max(1, math.ceil(date.replace(tzinfo=timezone.utc).timestamp() - now))
    match = re.search(r"(?:retry|try again)\s+(?:in|after)\s+(\d+)\s*(seconds?|minutes?|hours?|s\b)", message, re.I)
    if match:
        unit = match.group(2).lower()
        return max(1, int(match.group(1)) * (3600 if unit.startswith("h") else 60 if unit.startswith("m") else 1))
    return 60


def run_retry(argv, *, env=None):
    while True:
        result = subprocess.run(argv, env=env, text=True, capture_output=True)
        output = result.stdout + result.stderr
        # Cargo normally redacts credentials; do not depend on that for errors.
        for key in ("CARGO_REGISTRY_TOKEN", "CARGO_REGISTRIES_CRATES_IO_TOKEN"):
            if env and env.get(key):
                output = output.replace(env[key], "[withheld]")
        print(output, end="", flush=True)
        if not result.returncode:
            return
        if not re.search(r"\b429\b|too many requests", output, re.I):
            raise subprocess.CalledProcessError(result.returncode, argv)
        delay = retry_delay({}, output)
        print(f"crates publisher: rate limited; retrying in {delay}s", flush=True)
        time.sleep(delay)


def registry_read(url, *, missing_ok=False):
    while True:
        try:
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code == 404 and missing_ok:
                return None
            if error.code != 429:
                raise
            delay = retry_delay(error.headers, error.read().decode(errors="replace"))
            print(f"crates publisher: HTTP 429; retrying in {delay}s", flush=True)
            time.sleep(delay)


def version_exists(package):
    data = registry_read(f"https://crates.io/api/v1/crates/{package['name']}/{package['version']}", missing_ok=True)
    if data is None:
        return False
    version = json.loads(data)["version"]
    if version["yanked"]:
        raise ValueError(f"{package['name']} {package['version']} already exists but is yanked")
    return True


def wait_index(package):
    name = package["name"].lower()
    path = ("1/" + name if len(name) == 1 else "2/" + name if len(name) == 2
            else "3/" + name[0] + "/" + name if len(name) == 3 else name[:2] + "/" + name[2:4] + "/" + name)
    while True:
        data = registry_read("https://index.crates.io/" + path, missing_ok=True)
        if data and any(row["vers"] == package["version"] and not row["yanked"]
                        for row in map(json.loads, data.splitlines())):
            print(f"crates publisher: indexed {name} {package['version']}", flush=True)
            return
        print(f"crates publisher: waiting for index: {name} {package['version']}", flush=True)
        time.sleep(10)


def verify_artifact(path):
    subprocess.run(["gpg", "--verify", str(path) + ".asc", str(path)], check=True)
    return hashlib.sha256(path.read_bytes()).hexdigest()


def publish_packages(packages, target, env):
    target.mkdir(parents=True, exist_ok=True)
    rows = []
    for package in packages:
        name, version = package["name"], package["version"]
        row = {"name": name, "version": version}
        if version_exists(package):
            print(f"crates publisher: skip existing {name} {version}", flush=True)
            row["status"] = "already-published"
        else:
            run_retry(["make", "package-crate", f"CRATE={name}", f"TARGET_DIR={target}"], env=env)
            artifact = target / "package" / f"{name}-{version}.crate"
            digest = verify_artifact(artifact)
            run_retry(["cargo", "publish", "--locked", "--registry", "crates-io", "-p", name,
                       "--target-dir", str(target)], env=env)
            if verify_artifact(artifact) != digest:
                raise ValueError(f"cargo publish changed the signed artifact: {name}")
            row.update(status="published", sha256=digest, artifact=str(artifact))
        # Even a skipped dependency must be available to Cargo before advancing.
        wait_index(package)
        rows.append(row)
        (target / "crates-publication.json").write_text(json.dumps(rows, indent=2) + "\n")
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--target", type=Path, required=True)
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--package", action="append", help="Workspace package name; repeatable")
    selection.add_argument("--workspace", action="store_true", help="All publishable workspace packages")
    args = parser.parse_args()
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    credentials = cargo_home / "credentials.toml"
    token = os.environ.get("CARGO_REGISTRIES_CRATES_IO_TOKEN") or os.environ.get("CARGO_REGISTRY_TOKEN")
    if not token and credentials.is_file():
        config = tomllib.loads(credentials.read_text())
        token = (config.get("registries", {}).get("crates-io", {}).get("token")
                 or config.get("registry", {}).get("token"))
    verify_identity(token, json.loads(args.receipt.read_text()))
    print("crates publisher: bboxy450; exact-token operator binding verified", flush=True)
    env = {**os.environ, "CARGO_REGISTRY_TOKEN": token, "CARGO_REGISTRIES_CRATES_IO_TOKEN": token}
    data = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"], text=True))
    publish_packages(publication_order(data, args.package), args.target.resolve(), env)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"crates publication refused: {error}", file=sys.stderr)
        sys.exit(1)
