#!/usr/bin/env python3
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
"""GitKB data_source adapter. Build the single executable through Make.

Uses (does not copy) GitKB's source-aware CLI contracts, studied in
https://github.com/gitkb/gitkb-core : crates/gitkb-cli/src/commands/{show,workspace}.rs
and crates/gitkb-core/src/kb/{types,ops_impl}.rs. Native storage remains GitKB's.
"""
import concurrent.futures
import hashlib
import json
import mimetypes
import os
from pathlib import Path
import re
import secrets
import select
import shlex
import stat
import subprocess
import sys
import tempfile
import threading
import time
import uuid

# Embedded by scripts/build-gitkb-plugin.py; no adjacent runtime files.
BUILD = {}  # @build
SCHEMA = {}  # @schema
SKILLS = []  # @skills
SKILL_URI = "gitkb://skills/gitkb/SKILL.md"
METHODS = {"resolve": ("ResolveRequest", "RecordRef"), "fetch": ("FetchRequest", "Record"),
           "search": ("SearchRequest", "SearchPage"), "list": ("ListRequest", "RefPage"),
           "links": ("FetchRequest", "Links"), "graph": ("GraphRequest", "GraphPage"),
           "write": ("WriteRequest", "WriteResult"), "changes_since": ("ChangesRequest", "Changes"),
           "commit": ("CommitRequest", "CommitResult")}
MAX_FRAME = 8 * 1024 * 1024
MAX_RESULT = 256 * 1024
MAX_BACKEND = 32 * 1024 * 1024
MAX_PAGE = 128
ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"
NATIVE_TO_PORTABLE = {"draft": "open", "backlog": "open", "active": "running", "completed": "done"}
PORTABLE_TO_NATIVE = {"open": "draft", "claimed": "active", "running": "active",
                      "reported": "active", "accepted": "completed", "rejected": "active",
                      "done": "completed"}


class Refusal(Exception):
    def __init__(self, code, message, **fields):
        self.value = dict(code=code, message=message, **fields)
        super().__init__(code + ": " + message)


def encoded(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)


def digest(value):
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def portable(native):
    try:
        n = uuid.UUID(native).int
    except (ValueError, TypeError, AttributeError):
        raise Refusal("invalid_native_id", "GitKB must return a canonical UUID")
    return "".join(ALPHABET[(n >> (5 * i)) & 31] for i in reversed(range(26)))


def native(identifier):
    try:
        if len(identifier) == 26 and identifier[0] <= "7":
            n = 0
            for char in identifier:
                n = n * 32 + ALPHABET.index(char)
            return str(uuid.UUID(int=n))
        parsed = uuid.UUID(identifier)
        if str(parsed) == identifier and parsed.int:
            return str(parsed)
    except (ValueError, TypeError, AttributeError):
        pass
    raise Refusal("invalid_id", "expected a portable ULID or a native canonical UUID")


def slug_path(value):
    if not isinstance(value, str) or len(value) > 4096:
        raise Refusal("invalid_path", "expected a bounded relative Markdown path or slug")
    if value.endswith(".md"):
        value = value[:-3]
    if not value or any(p in ("", ".", "..") or p.startswith(".") for p in value.split("/")):
        raise Refusal("path_outside_root", "empty, hidden, absolute and parent paths are refused")
    if not re.fullmatch(r"[a-z0-9_][a-z0-9_.-]*(?:/[a-z0-9_][a-z0-9_.-]*)*", value):
        raise Refusal("invalid_path", "GitKB slugs use lowercase ASCII letters, digits, dots, hyphens and underscores")
    return value


def number(fm, slug):
    value = fm.get("number")
    if value is not None:
        if not isinstance(value, int) or isinstance(value, bool) or not 0 < value < 2**64:
            raise Refusal("invalid_number", "native number must be a positive u64")
        return value
    match = re.search(r"-([1-9][0-9]*)$", slug)
    return int(match[1]) if match and int(match[1]) < 2**64 else None


def schema(name):
    definitions = {}
    pending = [name]

    def references(value):
        if isinstance(value, dict):
            ref = value.get("$ref", "")
            if ref.startswith("#/$defs/"):
                pending.append(ref[len("#/$defs/"):])
            for child in value.values():
                references(child)
        elif isinstance(value, list):
            for child in value:
                references(child)

    while pending:
        key = pending.pop()
        if key not in definitions:
            definitions[key] = SCHEMA["$defs"][key]
            references(definitions[key])
    return {"$schema": SCHEMA["$schema"], "$defs": definitions, "$ref": "#/$defs/" + name}


def shape():
    return {"name": "GitKB", "description": "Source-aware local GitKB documents and task storage",
            "identity": {"provider": "gitkb", "executable": "bboxy450-term-data_source-gitkb",
                         "backend": BUILD}, "version": "1.0.0", "shape_version": "1.0",
            "systems": [{"contract": "data_source", "contract_version": "1.0",
                         "commands": [{"name": m, "args": schema(a), "result": schema(b),
                                       "timeout_ms": 15000 if m == "commit" else 2000} for m, (a, b) in METHODS.items()]}],
            "hooks": [], "health_command": "health", "signed_writer": True, "fixture_bootstrap": True,
            "concurrent": False, "trace": False, "skills": SKILLS}


def skill_resource(uri):
    # Exact embedded URI only: no decoding, cwd lookup or filesystem fallback.
    if uri != SKILL_URI or len(SKILLS) != 1:
        raise Refusal("unknown_resource", "only the embedded GitKB skill URI is available")
    return {"uri": SKILL_URI, "mimeType": "text/markdown", "text": SKILLS[0]["content"]}


class Adapter:
    def __init__(self, params, cancelled):
        self.cancelled = cancelled
        self.deadline = time.monotonic() + 1.9
        root = params.get("root")
        if not isinstance(root, str) or not Path(root).is_absolute():
            raise Refusal("invalid_knowledge_root", "initialize.root must be an absolute project, .kb or .bboxy directory")
        supplied = Path(root)
        if supplied.is_symlink():
            raise Refusal("invalid_knowledge_root", "symlinked roots are refused")
        try:
            self.root = supplied.resolve(strict=True)
        except OSError:
            raise Refusal("invalid_knowledge_root", "initialize.root must name an existing directory")
        if self.root.name in (".kb", ".bboxy"):
            self.root = self.root.parent
        self.cwd = Path.cwd().resolve()
        self.env = {k: v for k, v in os.environ.items()
                    if (k in ("PATH", "HOME", "LANG", "TMPDIR", "TERM") or k.startswith("LC_"))
                    and not any(word in k.upper() for word in ("TOKEN", "KEY", "SECRET", "BBOXY_", "ICQ_"))}
        self.env["GITKB_ROOT"] = str(self.root)
        self.backend = BUILD["path"]
        self.verify_backend()
        self.fixture_mode = params.get("fixture") == {"version": 1, "kind": "data_source_conformance"}
        if self.fixture_mode:
            temporary = Path(tempfile.gettempdir()).resolve()
            if (self.root.name != "checker-knowledge" or not self.root.is_relative_to(temporary)
                    or not (self.root / ".bboxy/p0-fixture.md").is_file()):
                raise Refusal("fixture_scope_refused", "bootstrap requires the host's disposable checker-knowledge root")
            # Approved test-only setup budget: native init measured 6.077 s.
            # Production initialize and every data call retain the 2 s cap.
            self.deadline = time.monotonic() + 14.5
            if not (self.root / ".kb").exists():
                previous = self.cwd
                try:
                    # `git-kb init` explicitly uses cwd (not GITKB_ROOT).
                    # This exception is confined to the disposable checker.
                    self.cwd = self.root
                    self.cli("init", "--name", "Disposable bboxy plugin conformance", allow_text=True)
                finally:
                    self.cwd = previous
        if not (self.root / ".kb").is_dir() or (self.root / ".kb").is_symlink():
            raise Refusal("invalid_knowledge_root", "the exact project must own a real .kb directory; no ancestor discovery")
        identity = self.cli("workspace", "resolve", "--json")
        if Path(identity["kbRoot"]).resolve() != self.root:
            raise Refusal("root_mismatch", "GitKB resolved a different knowledge root")
        self.workspace = identity["workspace"]
        self.workspace_path = Path(identity["workspacePath"])
        if not self.workspace_path.is_relative_to(self.root / ".kb" / "workspaces"):
            raise Refusal("workspace_outside_root", "GitKB workspace escaped the initialized root")
        self.env["GITKB_WORKSPACE"] = self.workspace
        self.cursors = {}
        if self.fixture_mode:
            self.bootstrap_fixture()

    def bootstrap_fixture(self):
        rows, _ = self.catalog()
        if "p0-fixture" in rows:
            return
        path = self.root / ".bboxy/p0-fixture.md"
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > MAX_RESULT:
            raise Refusal("unsafe_fixture", "fixture must be one confined regular Markdown seed")
        raw = path.read_text()
        match = re.search(r"(?m)^id: *[\"']?([0-9A-HJKMNP-TV-Z]{26})[\"']? *$", raw)
        if not match:
            raise Refusal("invalid_fixture", "expected canonical host conformance seed")
        raw = raw[:match.start(1)] + native(match[1]) + raw[match.end(1):]
        raw = raw.replace("---\n", "---\nslug: p0-fixture\n", 1)
        self.cli("workspace", "write", "p0-fixture", "--stdin", "--create-if-missing",
                 "--origin-id", "bboxy-conformance-seed", "--json", stdin=raw)

    def check_budget(self):
        if self.cancelled.is_set():
            raise Refusal("cancelled", "request cancelled")
        if time.monotonic() >= self.deadline:
            raise Refusal("deadline_exceeded", "GitKB call exhausted its declared budget")

    def verify_backend(self):
        # A root connection can outlive a CLI upgrade. Check every invocation,
        # within the original call budget, without accepting a new pin silently.
        checksum = hashlib.sha256()
        self.check_budget()
        with Path(self.backend).open("rb") as backend:
            while True:
                self.check_budget()
                chunk = backend.read(1024 * 1024)
                if not chunk:
                    break
                checksum.update(chunk)
        self.check_budget()
        if checksum.hexdigest() != BUILD["sha256"]:
            raise Refusal("backend_changed", "the installed GitKB changed; rebuild and retrust this executable")

    def cli(self, *args, stdin=None, allow_text=False):
        self.verify_backend()
        # No shell, inherited GitKB root/workspace, repository cwd or credentials.
        with tempfile.TemporaryFile() as inp, tempfile.TemporaryFile() as out_file, tempfile.TemporaryFile() as err_file:
            if stdin is not None:
                inp.write(stdin.encode())
            inp.seek(0)
            self.check_budget()
            with subprocess.Popen([self.backend, "--no-pager", "--color", "never", *args],
                                  cwd=self.cwd, env=self.env, stdin=inp,
                                  stdout=out_file, stderr=err_file) as process:
                while process.poll() is None:
                    failure = None
                    if self.cancelled.is_set():
                        failure = "cancelled"
                    elif time.monotonic() >= self.deadline:
                        failure = "deadline_exceeded"
                    elif max(os.fstat(out_file.fileno()).st_size, os.fstat(err_file.fileno()).st_size) > MAX_BACKEND:
                        failure = "backend_response_too_large"
                    if failure:
                        process.kill()
                        process.wait()
                        operation = " ".join(args[:2]) if args[0] == "workspace" else args[0]
                        hint = "; a write may have applied, inspect its source before retrying" if operation in ("workspace write", "commit", "checkout", "init") else ""
                        raise Refusal(failure, operation + " backend stopped" + hint)
                    try:
                        process.wait(timeout=min(0.02, max(0.001, self.deadline - time.monotonic())))
                    except subprocess.TimeoutExpired:
                        pass
            if max(os.fstat(out_file.fileno()).st_size, os.fstat(err_file.fileno()).st_size) > MAX_BACKEND:
                raise Refusal("backend_response_too_large", "GitKB output exceeded 32 MiB")
            out_file.seek(0)
            out = out_file.read(MAX_BACKEND + 1)
            err_file.seek(0)
            err = err_file.read(MAX_BACKEND + 1)
        try:
            value = json.loads(out)
        except (ValueError, UnicodeError):
            value = {} if allow_text and process.returncode == 0 else None
        if process.returncode:
            if not isinstance(value, dict):
                try:
                    value = json.loads(err)
                except (ValueError, UnicodeError):
                    value = None
            code = value.get("type", "backend_failed") if isinstance(value, dict) else "backend_failed"
            code = {"source_generation_mismatch": "etag_mismatch", "document_not_found": "not_found"}.get(code, code)
            if isinstance(value, dict) and "Author is required" in value.get("message", ""):
                code = "author_unconfigured"
            # Do not echo CLI stderr, which may include private document content.
            hint = "; configure native GitKB user.name and user.email" if code == "author_unconfigured" else ""
            raise Refusal(code, "GitKB refused " + args[0] + " (exit " + str(process.returncode) + ")" + hint,
                          backend_exit=process.returncode)
        if value is None:
            raise Refusal("backend_protocol_error", "GitKB did not return JSON")
        return value

    def workspace_stats(self):
        result = {}
        if self.workspace_path.is_symlink():
            raise Refusal("workspace_outside_root", "symlinked workspace refused")
        for directory, dirs, files in os.walk(self.workspace_path, followlinks=False):
            for name in dirs + files:
                if time.monotonic() >= self.deadline:
                    raise Refusal("deadline_exceeded", "workspace inventory exceeded the method budget")
                path = Path(directory) / name
                st = path.lstat()
                if stat.S_ISLNK(st.st_mode):
                    raise Refusal("unsafe_workspace_path", "symlinked workspace entry refused")
                if name.endswith(".md"):
                    result[str(path.relative_to(self.workspace_path))[:-3]] = [st.st_size, st.st_mtime_ns, st.st_ctime_ns, st.st_ino]
                if len(result) > 20000:
                    raise Refusal("workspace_too_large", "workspace exceeds 20000 files")
        return result

    def catalog(self, *flags):
        rows = self.cli("list", "--json", "--sort", "title", "--asc", *flags)
        if not isinstance(rows, list):
            raise Refusal("backend_protocol_error", "GitKB list is not an array")
        stats = self.workspace_stats()
        entries = {row["slug"]: dict(row, workspace_stat=stats.get(row["slug"])) for row in rows}
        for row in entries.values():
            if row["workspace_stat"] is not None:
                # GitKB 0.2.19 reparses workspace modified_at as read time.
                # Exact file stats fence that source; committed timestamps stay.
                row.pop("modified_at", None)
        return entries, digest(encoded(entries))

    def read(self, slug, requested=None):
        slug = slug_path(slug)
        if requested is None and slug in getattr(self, "call_records", {}):
            return self.call_records[slug]
        workspace_generation = None
        # Explicit workspace reads are never allowed to fall back to committed.
        result = self.cli("show", "--source", requested or "workspace", "--raw-content", "--json", "--", slug)
        if not result.get("exists"):
            if requested == "workspace" or result.get("participates"):
                raise Refusal("not_found", "the selected workspace source is deleted or absent")
            workspace_generation = result["generation"]
            result = self.cli("show", "--source", "committed", "--raw-content", "--json", "--", slug)
        if not result.get("exists") or not result.get("document"):
            raise Refusal("not_found", "no document in the selected GitKB source")
        doc = result["document"]
        raw = result.get("rawContent")
        if not isinstance(raw, str):
            raise Refusal("raw_content_unavailable", "GitKB omitted source bytes")
        # GitKB declares these projections graph-derived, never source YAML.
        editable = {k: v for k, v in doc["frontmatter"].items() if k not in ("references", "blocks", "children")}
        fm = dict(editable, id=portable(doc["id"]), slug=doc["slug"],
                  title=doc["title"], type=doc["doc_type"])
        native_status = fm.get("status")
        if fm["type"] == "task":
            fm["status"] = fm.pop("bboxy_status", NATIVE_TO_PORTABLE.get(native_status, native_status or "open"))
        # GitKB's content excludes the YAML framing newline. The exact native
        # bytes remain available below, without pretending the framing is body.
        body = doc["content"]
        source = result["documentSource"]
        content_digest = digest(encoded([str(self.root), source, raw]))
        record = {"id": portable(doc["id"]), "external_id": doc["id"], "number": number(fm, slug),
                  "slug": slug, "path": slug + ".md", "digest": content_digest,
                  "frontmatter": fm, "body": body, "etag": result["generation"], "resources": [],
                  "gitkb": {"source": source, "native_status": native_status,
                            "committed_revision": result.get("committedRevision"),
                            "version": doc["version"], "created_at": doc["created_at"],
                            "modified_at": doc["modified_at"], "raw_content": raw}}
        if "bboxy_signed_record" in fm:
            record["signed_record"] = fm.pop("bboxy_signed_record")
        if workspace_generation:
            record["gitkb"]["workspace_generation"] = workspace_generation
        if record["number"] is not None:
            fm["number"] = record["number"]
        if requested is None and hasattr(self, "call_records"):
            self.call_records[slug] = record
        return record

    @staticmethod
    def ref(record):
        return {key: record[key] for key in ("id", "external_id", "number", "slug", "path", "digest")}

    def check_ref(self, reference):
        if not isinstance(reference, dict):
            raise Refusal("invalid_ref", "ref must be an object")
        record = self.read(reference["slug"])
        if any(record.get(k) != reference.get(k) for k in ("id", "slug", "path", "digest")):
            raise Refusal("stale_ref", "record or source changed; resolve again")
        if reference.get("external_id") not in (None, record["external_id"]):
            raise Refusal("stale_ref", "native identity changed")
        return record

    def resources(self, record):
        for target in re.findall(r"!\[\[([^\]|]+)(?:\|[^\]]*)?\]\]", record["body"]):
            parts = Path(target).parts
            if not parts or Path(target).is_absolute() or any(p in ("..", ".") for p in parts):
                raise Refusal("resource_outside_data_dir", "image path escapes the GitKB root")
            candidates = [self.root / target, self.root / ".kb" / target]
            path = next((p for p in candidates if p.exists() or p.is_symlink()), None)
            if path is None:
                raise Refusal("resource_not_found", "GitKB image link is missing")
            for parent in (path, *path.parents):
                if parent == self.root:
                    break
                if parent.is_symlink():
                    raise Refusal("resource_outside_data_dir", "symlinked image path refused")
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
            with os.fdopen(fd, "rb") as source:
                info = os.fstat(source.fileno())
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > 64 * 1024 * 1024:
                    raise Refusal("unsafe_resource", "image must be a regular unlinked file, at most 64 MiB")
                data = source.read(64 * 1024 * 1024 + 1)
            if len(data) > 64 * 1024 * 1024:
                raise Refusal("resource_too_large", "image exceeds 64 MiB")
            directory = self.cwd / "resources"
            directory.mkdir(exist_ok=True, mode=0o700)
            if directory.is_symlink():
                raise Refusal("resource_outside_data_dir", "private resource directory is a symlink")
            dest = directory / hashlib.sha256(data).hexdigest()
            try:
                output = os.open(dest, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
                with os.fdopen(output, "wb") as file:
                    file.write(data)
            except FileExistsError:
                fd = os.open(dest, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
                with os.fdopen(fd, "rb") as file:
                    if file.read(len(data) + 1) != data:
                        raise Refusal("resource_changed", "private cached image changed")
            record["resources"].append({"type": "resource_link", "uri": dest.as_uri(), "name": Path(target).name,
                                         "mimeType": mimetypes.guess_type(target)[0] or "application/octet-stream",
                                         "size": len(data)})
        return record

    def resolve(self, args):
        locator = args["locator"]
        if not isinstance(locator, str) or not locator or len(locator) > 4096:
            raise Refusal("invalid_locator", "locator must be a bounded nonempty string")
        if locator.startswith("#"):
            if not re.fullmatch(r"#[1-9][0-9]*", locator):
                raise Refusal("invalid_number", "expected # followed by a positive number")
            target = int(locator[1:])
            rows, _ = self.catalog()
            candidates = {s for s in rows if number({}, s) == target}
            # Explicit native numbers take precedence within each record.
            for row in self.cli("list", "--where", "number = " + str(target), "--json"):
                candidates.add(row["slug"])
            records = [self.read(s) for s in sorted(candidates)]
            records = [r for r in records if r["number"] == target]
        else:
            try:
                identifier = native(locator)
            except Refusal:
                identifier = None
            if identifier:
                rows, _ = self.catalog()
                records = [self.read(s) for s, r in rows.items() if r["id"] == identifier]
            else:
                slug = slug_path(locator)
                try:
                    records = [self.read(slug)]
                except Refusal as error:
                    if error.value["code"] != "not_found":
                        raise
                    rows, _ = self.catalog()
                    records = [self.read(s) for s in rows if s.rsplit("/", 1)[-1] == slug]
        if not records:
            raise Refusal("not_found", "no document matches this root-scoped locator")
        if len(records) != 1:
            raise Refusal("ambiguous_locator", "use the full GitKB slug or native ID",
                          candidates=[self.ref(r) for r in records])
        return self.ref(records[0])

    def token(self, value):
        token = secrets.token_urlsafe(24)
        self.cursors[token] = value
        if len(self.cursors) > 128:
            del self.cursors[next(iter(self.cursors))]
        return token

    def position(self, args, key, snapshot):
        limit = args.get("limit") if args.get("limit") is not None else 32
        if not isinstance(limit, int) or isinstance(limit, bool) or not 1 <= limit <= MAX_PAGE:
            raise Refusal("invalid_limit", "limit must be from 1 through 128")
        cursor = args.get("cursor")
        if not cursor:
            return 0, min(limit, 2)
        old = self.cursors.get(cursor)
        if not old or old.get("key") != key or old.get("snapshot") != snapshot:
            raise Refusal("stale_cursor", "cursor is expired, belongs to another query/root, or the source changed")
        return old["offset"], min(limit, 2)

    def page_records(self, slugs, args, key, snapshot):
        offset, limit = self.position(args, key, snapshot)
        result = [self.read(s) for s in slugs[offset:offset + limit]]
        next_offset = offset + len(result)
        token = self.token(dict(key=key, snapshot=snapshot, offset=next_offset)) if next_offset < len(slugs) else None
        return result, token

    @staticmethod
    def matches_filter(record, filters):
        # Native --where can include nonmatching workspace overlays (0.2.19).
        # Check the fetched source within the existing per-page read budget.
        return all(not filters.get(field) or record["frontmatter"].get(field) == filters[field]
                   for field in ("type", "status", "assignee", "swarm"))

    def listing(self, args, search=False):
        filters = dict(args.get("filter") or {})
        words = []
        if search:
            for word in shlex.split(args["query"]):
                name, sep, value = word.partition(":")
                if sep and name in ("type", "status", "since"):
                    filters[name] = value
                else:
                    words.append(word)
        flags = []
        for field in ("type", "since"):
            if filters.get(field):
                flags.extend(["--" + field, filters[field]])
        for field in ("assignee", "swarm"):
            if filters.get(field):
                flags.extend(["--where", field + " = " + filters[field]])
        if filters.get("status"):
            status = filters["status"]
            native_statuses = {status, *[n for n, p in NATIVE_TO_PORTABLE.items() if p == status]}
            native_statuses.add(PORTABLE_TO_NATIVE.get(status, status))
            rows, _ = self.catalog(*flags, "--status", ",".join(sorted(native_statuses)))
            namespaced, _ = self.catalog(*flags, "--where", "bboxy_status = " + status)
            rows.update(namespaced)
            snapshot = digest(encoded(rows))
        else:
            rows, snapshot = self.catalog(*flags)
        selected = set(rows)
        if words:
            return self.search_page(rows, selected, words, filters, args, snapshot)
        slugs = sorted(selected)
        key = encoded(["search" if search else "list", filters, words])
        offset, limit = self.position(args, key, snapshot)
        items = []
        examined = 0
        while offset < len(slugs) and len(items) < limit and examined < 2:
            record = self.read(slugs[offset])
            offset += 1
            examined += 1
            if words and rows[record["slug"]].get("workspace_stat"):
                haystack = (encoded(record["frontmatter"]) + " " + record["body"]).casefold()
                if not all(word.casefold() in haystack for word in words):
                    continue
            if not self.matches_filter(record, filters):
                continue
            if search:
                items.append({"ref": self.ref(record), "title": record["frontmatter"]["title"],
                              "type": record["frontmatter"]["type"], "number": record["number"],
                              "snippet": record["body"][:240], "score": 1.0})
            else:
                items.append(self.ref(record))
        next_cursor = self.token(dict(key=key, snapshot=snapshot, offset=offset)) if offset < len(slugs) else None
        return {"items": items, "next": next_cursor}

    def search_page(self, rows, selected, words, filters, args, snapshot):
        key = encoded(["search", filters, words])
        offset, limit = self.position(args, key, snapshot)
        state = self.cursors.get(args.get("cursor"), {})
        phase = state.get("phase", "native")
        seen = set(state.get("seen", []))
        if phase == "native":
            flags = ["--type", filters["type"]] if filters.get("type") else []
            result = self.cli("search", " ".join(words), "--all-terms", "--limit", str(limit), "--offset", str(offset), *flags, "--json")
            hits = result["results"]
            next_offset = result["pagination"].get("next_offset")
            following = {"phase": "native", "offset": next_offset} if next_offset is not None else {"phase": "workspace", "offset": 0}
            if not hits:
                phase, offset = "workspace", 0
        if phase == "workspace":
            candidates = sorted(s for s in selected if rows[s].get("workspace_stat") and s not in seen)
            hits = [{"slug": s} for s in candidates[offset:offset + limit]]
            next_offset = offset + len(hits)
            following = {"phase": "workspace", "offset": next_offset} if next_offset < len(candidates) else None
        items = []
        for hit in hits:
            slug = hit["slug"]
            if rows.get(slug, {}).get("workspace_stat") and phase == "native":
                seen.add(slug)
            if slug not in selected:
                continue
            record = self.read(slug)
            if not self.matches_filter(record, filters):
                continue
            # Native FTS does not overlay workspace-only/dirty source content.
            if rows[slug].get("workspace_stat"):
                text = (encoded(record["frontmatter"]) + " " + record["body"]).casefold()
                if not all(word.casefold() in text for word in words):
                    continue
            items.append({"ref": self.ref(record), "title": record["frontmatter"]["title"],
                          "type": record["frontmatter"]["type"], "number": record["number"],
                          "snippet": hit.get("snippet", record["body"][:240]), "score": float(hit.get("score", 1))})
        token = self.token(dict(following, key=key, snapshot=snapshot, seen=sorted(seen))) if following else None
        return {"items": items, "next": token}

    def edges(self, slugs, depth=1):
        if not slugs:
            return [], []
        result = self.cli("graph", "--json", "--depth", str(depth), "--", *slugs)
        edges = [{"from": portable(e["from"]), "to": portable(e["to"]),
                  "rel": e.get("rel_type", "references"), "weight": float(e.get("weight", 1.0))}
                 for e in result.get("edges", [])]
        return edges, result.get("nodes", [])

    def links(self, args):
        record = self.check_ref(args["ref"])
        edges, _ = self.edges([record["slug"]])
        return {"outgoing": [e for e in edges if e["from"] == record["id"]],
                "incoming": [e for e in edges if e["to"] == record["id"]]}

    def graph(self, args):
        scope = args["scope"]
        if scope.get("graph") not in ("markdown", "task", "artifact"):
            raise Refusal("unsupported_graph", "execution, channel and messaging graphs belong to the host")
        rows, snapshot = self.catalog()
        selected = set(rows)
        if scope["graph"] != "markdown":
            selected = {s for s in selected if rows[s]["type"] == scope["graph"]}
        if scope["kind"] == "neighborhood":
            depth = scope["depth"]
            if not isinstance(depth, int) or not 0 <= depth <= 8:
                raise Refusal("invalid_depth", "neighborhood depth must be 0 through 8")
            record = self.check_ref(scope["ref"])
            _, nodes = self.edges([record["slug"]], depth)
            selected &= {n["slug"] for n in nodes}
        elif scope["kind"] == "type":
            selected = {s for s in selected if rows[s]["type"] == scope["type"]}
        elif scope["kind"] == "swarm":
            selected &= {r["slug"] for r in self.cli("list", "--where", "swarm = " + scope["swarm"], "--json")}
        elif scope["kind"] != "all":
            raise Refusal("invalid_scope", "unknown graph scope")
        records, token = self.page_records(sorted(selected), args, encoded(["graph", scope]), snapshot)
        if scope["kind"] == "swarm":
            records = [record for record in records if self.matches_filter(record, {"swarm": scope["swarm"]})]
        edges, _ = self.edges([r["slug"] for r in records])
        ids = {r["id"] for r in records}
        outgoing = [e for e in edges if e["from"] in ids]
        return {"nodes": [{"id": r["id"], "ref": self.ref(r), "title": r["frontmatter"]["title"],
                           "type": r["frontmatter"]["type"], "frontmatter": r["frontmatter"]} for r in records],
                "edges": outgoing[:512], "next": token, "edges_truncated": len(outgoing) > 512}

    def write(self, args):
        signed = args.get("signed_record")
        if not signed and self.fixture_mode:
            signed = "disposable unsigned conformance fixture"
        if not isinstance(signed, str) or not signed:
            raise Refusal("signed_writer_unavailable", "write requires a host-admitted signed_record; use the host knowledge writer")
        requested_fm = dict(args["frontmatter"])
        fm = dict(requested_fm)
        if not isinstance(args["body"], str) or not fm.get("title") or not fm.get("type"):
            raise Refusal("invalid_record", "title, type and Markdown body are required")
        reference = args.get("ref")
        create_workspace = False
        if reference:
            if args.get("path") is not None:
                raise Refusal("invalid_write", "updates use ref with no path")
            old = self.check_ref(reference)
            if args.get("etag") != old["etag"]:
                raise Refusal("etag_mismatch", "update requires the current source etag")
            slug = old["slug"]
            if fm.get("id", old["id"]) not in (old["id"], old["external_id"]):
                raise Refusal("immutable_id", "record identity cannot change")
            if old["gitkb"]["source"]["kind"] == "committed":
                # Native CAS binds the absent workspace generation to the
                # current committed revision. Materialize+write in one native
                # transaction, rather than checkout/read/write CLI sequences.
                generation = old["gitkb"]["workspace_generation"]
                create_workspace = True
            else:
                generation = old["etag"]
            fm["id"] = old["external_id"]
        else:
            if not isinstance(args.get("path"), str) or not args["path"].endswith(".md"):
                raise Refusal("path_required", "create requires a relative .md path")
            slug = slug_path(args["path"])
            rows, _ = self.catalog()
            if slug in rows:
                raise Refusal("path_exists", "a GitKB document already occupies this slug")
            generation = None
            if fm.get("id"):
                fm["id"] = native(fm["id"])
        if fm.get("slug", slug) != slug:
            raise Refusal("slug_mismatch", "frontmatter.slug must match the addressed path")
        fm["slug"] = slug
        if fm["type"] == "task":
            status = fm.get("status", "open")
            fm["bboxy_status"] = status
            fm["status"] = PORTABLE_TO_NATIVE.get(status, status)
        fm["bboxy_signed_record"] = signed
        # Flow-style JSON values are YAML 1.2. Plain top-level keys also let
        # GitKB's exact ID/slug guard see the supplied identity before parsing.
        header = "\n".join((k if re.fullmatch(r"[a-zA-Z_$][a-zA-Z0-9_$-]*", k) else json.dumps(k)) +
                           ": " + json.dumps(v, ensure_ascii=False, allow_nan=False) for k, v in fm.items())
        raw = "---\n" + header + "\n---\n" + args["body"]
        flags = ["--if-generation", generation] if generation else ["--create-if-missing"]
        if create_workspace:
            flags.append("--create-if-missing")
        self.cli("workspace", "write", slug, "--stdin", "--origin-id", "bboxy-" + secrets.token_hex(12),
                 *flags, "--json", stdin=raw)
        self.call_records.pop(slug, None)
        record = self.read(slug, "workspace")
        lossy = record["body"] != args["body"] or any(record["frontmatter"].get(k) != v for k, v in requested_fm.items())
        return {"record": record, "lossy": lossy, "signed_record": signed}

    def changes(self, args):
        rows, snapshot = self.catalog()
        manifest = {slug: {"id": row["id"], "state": digest(encoded(row))} for slug, row in rows.items()}
        cursor = args["cursor"]
        _, limit = self.position({"limit": args.get("limit")}, "", "")
        if cursor in ("", "0"):
            return {"changed": [], "next": self.token({"rows": manifest, "snapshot": snapshot, "changes": True})}
        old = self.cursors.get(cursor)
        if not old or not old.get("changes"):
            raise Refusal("stale_cursor", "unknown change cursor; start a new baseline with cursor 0")
        pending = old.get("pending")
        if pending is not None:
            if old["snapshot"] != snapshot:
                raise Refusal("stale_cursor", "source changed while paging changes; resync the root")
        else:
            pending = sorted(s for s in set(manifest) | set(old["rows"]) if manifest.get(s) != old["rows"].get(s))
        changed = []
        for slug in pending[:limit]:
            if slug in rows:
                changed.append(self.ref(self.read(slug)))
            else:
                row = old["rows"][slug]
                changed.append({"id": portable(row["id"]), "external_id": row["id"], "slug": slug,
                                "path": slug + ".md", "number": number({}, slug),
                                "digest": digest("deleted:" + row["id"])})
        following = {"rows": manifest, "snapshot": snapshot, "changes": True}
        if len(pending) > limit:
            following["pending"] = pending[limit:]
            following["rows"] = old["rows"]
        return {"changed": changed, "next": self.token(following)}

    def commit(self, args):
        refs, message = args.get("refs"), args.get("message")
        if not isinstance(refs, list) or not 1 <= len(refs) <= 32:
            raise Refusal("invalid_commit", "commit requires from 1 through 32 explicit references")
        if not isinstance(message, str) or not message.strip() or len(message) > 4096:
            raise Refusal("invalid_commit", "commit requires a nonempty message of at most 4096 characters")
        records = [self.check_ref(ref) for ref in refs]
        slugs = [record["slug"] for record in records]
        if len(slugs) != len(set(slugs)):
            raise Refusal("invalid_commit", "duplicate document references")
        result = self.cli("commit", "--json", "-m", message, "--", *slugs)
        if any(change["slug"] not in slugs for change in result["changes"]):
            raise Refusal("unexpected_commit_scope", "GitKB returned an unexpected document; inspect the native commit")
        self.call_records = {}
        return {"commit": result["commit_id"], "committed": refs}

    def dispatch(self, method, args, cancelled):
        self.cancelled = cancelled
        self.call_records = {}
        self.deadline = time.monotonic() + (14.5 if method == "commit" else 1.9)
        if any(k in args for k in ("root", "author", "signer", "device", "account")):
            raise Refusal("host_field_in_arguments", "root and authority belong to initialize and the host")
        if method == "resolve":
            return self.resolve(args)
        if method == "fetch":
            return self.resources(self.check_ref(args["ref"]))
        if method in ("list", "search"):
            return self.listing(args, method == "search")
        if method == "links":
            return self.links(args)
        if method == "graph":
            return self.graph(args)
        if method == "write":
            return self.write(args)
        if method == "changes_since":
            return self.changes(args)
        if method == "commit":
            return self.commit(args)
        raise Refusal("unknown_method", method)


def serve():
    adapter = None
    output_lock = threading.Lock()
    pending = {}

    def send(value):
        line = encoded(value)
        with output_lock:
            sys.stdout.write(line + "\n")
            sys.stdout.flush()

    def process(request, cancelled):
        nonlocal adapter
        request_id = request["id"]
        try:
            method, params = request.get("method"), request.get("params") or {}
            if method == "initialize":
                if adapter is not None:
                    raise Refusal("already_initialized", "one process binds one root")
                adapter = Adapter(params, cancelled)
                value = {"protocolVersion": "2025-06-18", "serverInfo": {"name": "GitKB", "version": "1.0.0"},
                         "capabilities": {"tools": {}, "resources": {}, "experimental": {"bboxy450": {
                             "shape_version": "1.0", "contracts": {"data_source": "1.0"},
                             "signed_writer": "1.0", "concurrent": False, "trace": False}}}}
            elif method == "ping":
                value = {}
            elif method in ("resources/list", "resources/read"):
                if adapter is None:
                    raise Refusal("not_initialized", "initialize.root must bind this process first")
                if method == "resources/list":
                    value = {"resources": [{"uri": SKILL_URI, "name": SKILLS[0]["name"],
                                            "description": SKILLS[0]["description"], "mimeType": "text/markdown"}]}
                else:
                    value = {"contents": [skill_resource(params.get("uri"))]}
            elif method == "tools/list":
                value = {"tools": [{"name": "data_source_" + m, "description": "GitKB " + m,
                                    "inputSchema": schema(a), "outputSchema": schema(b)} for m, (a, b) in METHODS.items()]}
            elif method == "tools/call":
                if adapter is None:
                    raise Refusal("not_initialized", "initialize.root must bind this process first")
                name = params.get("name", "")
                if name == "health":
                    result = {"healthy": True, "root": str(adapter.root), "workspace": adapter.workspace, "backend": BUILD}
                elif name.startswith("data_source_"):
                    result = adapter.dispatch(name[len("data_source_"):], params.get("arguments") or {}, cancelled)
                else:
                    raise Refusal("unknown_method", "unknown tool")
                if len(encoded(result).encode()) > MAX_RESULT:
                    raise Refusal("response_too_large", "serialized result exceeds 256 KiB; use a smaller page")
                value = {"content": [{"type": "text", "text": encoded(result)}], "structuredContent": result, "isError": False}
            else:
                send({"jsonrpc": "2.0", "id": request_id, "error": {"code": -32601, "message": "method not found"}})
                return
            send({"jsonrpc": "2.0", "id": request_id, "result": value})
        except Refusal as error:
            if request.get("method") == "tools/call":
                send({"jsonrpc": "2.0", "id": request_id, "result": {
                    "content": [{"type": "text", "text": str(error)}], "structuredContent": error.value, "isError": True}})
            else:
                code = -32002 if error.value["code"] == "unknown_resource" else -32602
                send({"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": str(error), "data": error.value}})
        except (KeyError, TypeError, ValueError, OSError) as error:
            if request.get("method") == "tools/call":
                send({"jsonrpc": "2.0", "id": request_id, "result": {"isError": True, "content": [],
                     "structuredContent": {"code": "invalid_arguments", "message": "invalid request or unavailable local dependency", "kind": type(error).__name__}}})
            else:
                send({"jsonrpc": "2.0", "id": request_id, "error": {"code": -32602, "message": "invalid request or unavailable local dependency", "data": {"kind": type(error).__name__}}})
        finally:
            pending.pop(request_id, None)

    with concurrent.futures.ThreadPoolExecutor(max_workers=1) as worker:
        while True:
            line = sys.stdin.buffer.readline(MAX_FRAME + 1)
            if not line:
                break
            if len(line) > MAX_FRAME:
                send({"jsonrpc": "2.0", "id": None, "error": {"code": -32600, "message": "frame_too_large"}})
                break
            try:
                request = json.loads(line)
                if not isinstance(request, dict) or request.get("jsonrpc") != "2.0":
                    raise ValueError()
            except (ValueError, UnicodeError):
                send({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "parse error"}})
                continue
            if request.get("method") == "notifications/cancelled":
                event = pending.get((request.get("params") or {}).get("requestId"))
                if event:
                    event.set()
                continue
            if "id" not in request:
                continue
            if len(pending) >= 8 or request["id"] in pending:
                send({"jsonrpc": "2.0", "id": request["id"], "error": {"code": -32000, "message": "busy_or_duplicate_id"}})
                continue
            event = threading.Event()
            pending[request["id"]] = event
            worker.submit(process, request, event)


if __name__ == "__main__":
    # The host starts stdio with no flags. A bare terminal/closed stdin is the
    # human shape courtesy; an open pipe waits for the first MCP request.
    bare_shape = not sys.argv[1:] and (sys.stdin.isatty() or
                 (select.select([sys.stdin], [], [], 0)[0] and not sys.stdin.buffer.peek(1)))
    if bare_shape or sys.argv[1:] == ["--shape"]:
        print(encoded(shape()))
    elif not sys.argv[1:] or sys.argv[1:] in (["--serve"], ["serve"], ["--stdio"]):
        serve()
    else:
        print("use --shape or --serve", file=sys.stderr)
        sys.exit(2)
