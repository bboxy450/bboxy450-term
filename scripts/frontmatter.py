# Invariant check (.bboxy/invariants/frontmatter-parses.md): every .bboxy Markdown
# frontmatter is well-formed YAML for the subset work-items.md uses. Standard
# library only; the bbt-doc corpus test replaces this when it lands.
import glob, re, sys
errors = []
key = re.compile(r"^( *)([A-Za-z_][A-Za-z0-9_]*):(?: (.*))?$")
item = re.compile(r"^( +)- (.*)$")
for path in sorted(glob.glob(".bboxy/**/*.md", recursive=True)):
    lines = open(path, encoding="utf-8").read().split("\n")
    if not lines or lines[0] != "---":
        continue
    try:
        end = lines.index("---", 1)
    except ValueError:
        errors.append(f"{path}:1 frontmatter never closed"); continue
    keys = set()
    for n, line in enumerate(lines[1:end], start=2):
        if not line.strip():
            continue
        m = key.match(line) or item.match(line)
        if not m:
            errors.append(f"{path}:{n} not `key: value` or `- item`: {line[:60]}"); continue
        if m.re is key and not m.group(1):
            keys.add(m.group(2))
        value = (m.group(3) if m.re is key else m.group(2)) or ""
        v = value.strip()
        if v and v[0] not in "\"'[{" and (": " in v or " #" in v or v.endswith(":")):
            errors.append(f"{path}:{n} unquoted value with ': ' or ' #': {v[:60]}")
    for required in ("id", "type"):
        if required not in keys:
            errors.append(f"{path}: frontmatter missing `{required}:`")
for e in errors:
    print(e)
print(f"frontmatter: {len(errors)} errors")
sys.exit(1 if errors else 0)
