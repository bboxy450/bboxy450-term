"""Keep packaged build inputs byte-identical to their repository source."""
from pathlib import Path
import sys


def check(root):
    errors = []
    count = 0
    for folder in sorted((root / "crates").glob("*/release-inputs")):
        for copy in sorted(folder.rglob("*")):
            if copy.is_dir():
                continue
            source = root / copy.relative_to(folder)
            count += 1
            if copy.is_symlink() or not source.is_file() or copy.read_bytes() != source.read_bytes():
                errors.append(f"{copy.relative_to(root)} differs from {source.relative_to(root)}")
    for error in errors:
        print(f"release-inputs: {error}", file=sys.stderr)
    if not errors:
        print(f"release-inputs: held ({count} identical copies)")
    return bool(errors)


if __name__ == "__main__":
    root = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
    raise SystemExit(check(root))
