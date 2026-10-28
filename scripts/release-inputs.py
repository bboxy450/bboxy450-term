"""Keep packaged build inputs byte-identical to their repository source."""
# Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import argparse
from pathlib import Path
import sys


def check(root, synchronize=False):
    errors = []
    count = 0
    for folder in sorted((root / "crates").glob("*/release-inputs")):
        for copy in sorted(folder.rglob("*")):
            if copy.is_dir():
                continue
            source = root / copy.relative_to(folder)
            count += 1
            if copy.is_symlink() or not source.is_file():
                errors.append(f"{copy.relative_to(root)} differs from {source.relative_to(root)}")
            elif copy.read_bytes() != source.read_bytes():
                if synchronize:
                    copy.write_bytes(source.read_bytes())
                else:
                    errors.append(f"{copy.relative_to(root)} differs from {source.relative_to(root)}")
    for error in errors:
        print(f"release-inputs: {error}", file=sys.stderr)
    if not errors:
        print(f"release-inputs: held ({count} identical copies)")
    return bool(errors)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--sync", action="store_true", help="refresh existing packaged copies from canonical inputs")
    args = parser.parse_args()
    raise SystemExit(check(args.root.resolve(), args.sync))
