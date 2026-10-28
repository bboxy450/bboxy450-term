"""Check/synchronize crate terms and inspect Cargo's packaged file lists."""
# Origin: bboxy450-term. Respectful Slop License 1.0-Draft; upstream terms retained.
# Copyright (c) 2026 Boxy (bboxy450). Retain credit on reuse:
# https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib

POLICY = "Respectful Slop License 1.0-Draft"
COPIES = ("LICENSE", "NOTICE", "ATTRIBUTION.md")
LEGACY = ("LICENSE-MIT", "LICENSE-APACHE")
# Preserve the exact already-published and upstream texts during our policy switch.
ORDINARY = ("e3012f47cfba2c19c3ee490477faf30d45d1234759c86508a18f487ebe564c0a", "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30")
YARN = ("905373480080597d230adf07df32d04c5a3407e0ae3b0b413c15a832b61cc398", "470792be84b6ffd17172bec28c999b687288f90ae688ebc0973a8cbdff6b49bd")
CORE = ("e4c9b06fa850cb9b540a5e400e9f6394cf15efcf4098144de477d1d3dae10150", "62c7a1e35f56406896d7aa7ca52d0cc0d272ac022b5d2796e7d6905db8a3636a")
UPSTREAM = {
    "bbt-core": {
        "src/vendor/alacritty/LICENSE-APACHE": "8884289e3969c089cd556c53fa1d3f51d86b29e52a8c1d660a55ce8292986910",
        "src/vendor/vte/LICENSE-MIT": CORE[0], "src/vendor/vte/LICENSE-APACHE": CORE[1],
    },
    "bbt-render": {
        "tests/fonts/OFL.txt": "a76abf002c49097d146e86740a3105a5d00450b1592e820a1109a8c5680cd697",
        "tests/fonts/JetBrainsMono-Regular.ttf": "e6fd0d7e91550b3ed2b735d4312474362c4716edc4fc0577a0f61ed782d5aed1",
    },
    "bboxy-firehose": {
        "reference/vendor/LICENSE": "b569f629d00f2626a8100df2a1798210535621e42164dfd426a6fe5aac7b0ccd",
        "reference/vendor/unicode11-LICENSE": "e256f01188af527e4d06d21d06fbf785ae9c50d4b328bf03cbe0ba7f0aa4228f",
    },
}


def read_regular(path):
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        raise ValueError(f"{path}: required nonempty regular copy is missing")
    return path.read_bytes()


def metadata(root, synchronize):
    if synchronize:
        # First sync works before local LICENSE copies exist, without Cargo IO.
        workspace = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]
        packages = []
        for member in workspace["members"]:
            manifest = root / member / "Cargo.toml"
            package = tomllib.loads(manifest.read_text())["package"]
            packages.append({"id": package["name"], "name": package["name"],
                             "manifest_path": str(manifest), "license": package.get("license"),
                             "license_file": package.get("license-file")})
        return {"workspace_root": str(root), "workspace_members": [p["id"] for p in packages], "packages": packages}
    result = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
                            cwd=root, capture_output=True, text=True, timeout=60)
    if result.returncode:
        raise ValueError(f"cargo metadata exited {result.returncode}: {result.stderr.strip()}")
    return json.loads(result.stdout)


def packages_from(data):
    if not isinstance(data, dict) or not isinstance(data.get("workspace_root"), str):
        raise ValueError("malformed Cargo metadata: workspace_root")
    root = Path(data["workspace_root"]).resolve()
    members, packages = data.get("workspace_members"), data.get("packages")
    if (not isinstance(members, list) or not members or not all(isinstance(x, str) for x in members)
            or len(set(members)) != len(members) or not isinstance(packages, list)):
        raise ValueError("malformed Cargo metadata: workspace members/packages")
    selected, found, names = [], set(), set()
    for package in packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str):
            raise ValueError("malformed Cargo metadata: package id")
        if package["id"] not in members:
            continue
        if not all(isinstance(package.get(key), str) and package[key] for key in ("name", "manifest_path")):
            raise ValueError("malformed Cargo metadata: package name/manifest")
        if package["id"] in found or package["name"] in names:
            raise ValueError("malformed Cargo metadata: duplicate workspace package")
        found.add(package["id"])
        names.add(package["name"])
        selected.append(package)
    if found != set(members):
        raise ValueError("malformed Cargo metadata: a workspace member has no package")
    return root, selected


def licensing_map(root):
    policy, in_map = {}, False
    for line in (root / ".bboxy/specs/licensing.md").read_text(encoding="utf-8").splitlines():
        cells = [c.strip() for c in re.split(r"(?<!\\)\|", line.strip().strip("|"))]
        if cells[:3] == ["Crate", "Publishes as", "License"]:
            in_map = True
            continue
        if not in_map:
            continue
        if not line.lstrip().startswith("|"):
            break
        if len(cells) < 4:
            continue
        for name in re.findall(r"`([^`]+)`", cells[0]):
            if name in policy:
                raise ValueError(f"licensing map repeats {name}")
            policy[name] = cells[2].replace("*", "").replace("`", "")
    return policy


def check(data, synchronize=False, package_lists=False, only=None):
    root, packages = packages_from(data)
    policy, errors, directories = licensing_map(root), [], []
    sources = {name: read_regular(root / name) for name in COPIES}
    if POLICY not in sources["LICENSE"].decode("utf-8"):
        errors.append("canonical LICENSE does not identify the supplied policy")
    workspace = tomllib.loads((root / "Cargo.toml").read_text()).get("workspace", {}).get("package", {})
    if "license" in workspace or "license-file" in workspace:
        errors.append("workspace.package must not override each crate's local license-file")

    def protect(path, expected):
        try:
            if hashlib.sha256(read_regular(path)).hexdigest() != expected:
                errors.append(f"{path}: preserved prior/upstream bytes changed")
        except ValueError as error:
            errors.append(str(error))

    for name, expected in zip(LEGACY, ORDINARY):
        protect(root / name, expected)
    for package in packages:
        manifest = Path(package["manifest_path"])
        if not manifest.is_absolute():
            manifest = root / manifest
        directory = manifest.parent
        if manifest.is_symlink() or not directory.resolve().is_relative_to(root):
            errors.append(f"{package['name']}: package manifest is outside the workspace")
            continue
        declared = tomllib.loads(read_regular(manifest).decode("utf-8"))["package"]
        terms = policy.get(directory.name)
        if terms is None:
            errors.append(f"{package['name']}: missing from the licensing map")
        elif not terms.startswith(POLICY) or not all(word in terms.lower() for word in ("upstream", "prior")):
            errors.append(f"{package['name']}: licensing map must name custom policy and retained upstream/prior grants")
        license_path = package.get("license_file")
        if "license" not in package or package["license"] is not None or "license" in declared:
            errors.append(f"{package['name']}: use local license-file, not blanket SPDX license")
        if not isinstance(license_path, str):
            errors.append(f"{package['name']}: metadata is missing license_file")
        else:
            target = Path(license_path)
            target = target if target.is_absolute() else directory / target
            if target != directory / "LICENSE":
                errors.append(f"{package['name']}: license_file must point to local LICENSE")
        if declared.get("license-file") != "LICENSE":
            errors.append(f"{package['name']}: manifest must explicitly set license-file = LICENSE")
        for name in COPIES:
            path = directory / name
            if path.is_symlink():
                errors.append(f"{path}: required regular copy cannot be a symlink")
            elif not synchronize:
                try:
                    if read_regular(path) != sources[name]:
                        errors.append(f"{path}: differs from canonical {name}")
                except ValueError as error:
                    errors.append(str(error))
        legacy = CORE if directory.name == "bbt-core" else YARN if directory.name in ("bbt-yarn", "bbt-yarn-macros") else ORDINARY
        for name, expected in zip(LEGACY, legacy):
            protect(directory / name, expected)
        for name, expected in UPSTREAM.get(directory.name, {}).items():
            protect(directory / name, expected)
        directories.append((package, directory))
    if errors:
        raise ValueError("\n".join(errors))
    if synchronize:
        for _, directory in directories:
            for name, content in sources.items():
                (directory / name).write_bytes(content)
    if only and not any(p["name"] == only for p in packages):
        raise ValueError(f"unknown workspace package: {only}")
    listed = 0
    if package_lists:
        for package, directory in directories:
            if only and package["name"] != only:
                continue
            result = subprocess.run(["cargo", "package", "--locked", "--allow-dirty", "--list", "--package", package["name"]],
                                    cwd=root, capture_output=True, text=True, timeout=60)
            if result.returncode:
                raise ValueError(f"{package['name']}: cargo package --list exited {result.returncode}: {result.stderr.strip()}")
            required = set(COPIES + LEGACY)
            if directory.name != "bboxy-firehose":
                required.update(UPSTREAM.get(directory.name, {}))
            missing = required.difference(result.stdout.splitlines())
            if missing:
                raise ValueError(f"{package['name']}: package omits {', '.join(sorted(missing))}")
            if directory.name == "bboxy-firehose" and any(name.startswith("reference/") for name in result.stdout.splitlines()):
                raise ValueError("firehose package unexpectedly includes excluded reference assets")
            listed += 1
    print(f"licenses: {len(packages)} workspace crates; local policy, notices and preserved upstream bytes held"
          + (f"; {listed} package file lists checked" if package_lists else "")
          + ("; canonical copies synchronized" if synchronize else ""))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--sync", action="store_true", help="copy canonical LICENSE/NOTICE/ATTRIBUTION into each member")
    parser.add_argument("--packages", action="store_true", help="inspect cargo package --list for every member")
    parser.add_argument("--package", help="with --packages, inspect this one member's file list")
    args = parser.parse_args()
    if args.package and not args.packages:
        parser.error("--package requires --packages")
    try:
        check(metadata(args.root.resolve(), args.sync), args.sync, args.packages, args.package)
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"licenses: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
