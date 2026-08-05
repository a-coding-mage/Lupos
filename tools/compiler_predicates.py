#!/usr/bin/env python3
"""Capture compiler predicate results from frozen Linux compile contexts.

The only compiler action performed by this tool is preprocessing a generated
predicate probe.  It never compiles an object and it refuses to replace an
existing evidence directory.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
import csv
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import tempfile
from typing import Iterable


ROOT = Path(__file__).resolve().parents[1]
BRANCH = "feat/bun-like-rewrite-test"
KINDS = (
    "__has_attribute",
    "__has_builtin",
    "__has_feature",
    "__has_extension",
    "__has_c_attribute",
    "__has_declspec_attribute",
    "__has_warning",
)
FIELDS = [
    "predicate_id", "predicate_kind", "argument", "architecture",
    "target_triple", "result", "source_locations", "linux_commit",
    "config_sha256", "toolchain_sha256", "compiler_requested_path",
    "compiler_resolved_path", "compiler_sha256", "compiler_version",
    "original_command_source", "original_command_sha256", "probe_path",
    "probe_sha256", "probe_command_path", "probe_command_sha256",
    "stdout_path", "stdout_sha256", "stderr_path", "stderr_sha256",
    "exit_status", "started_at", "completed_at", "status",
]
STATUS_VALUES = {"PROVEN", "BLOCKED", "NOT_APPLICABLE"}
DEPENDENCY_FLAGS = {"-M", "-MM", "-MD", "-MMD", "-MG", "-MP"}
DEPENDENCY_VALUE_FLAGS = {"-MF", "-MT", "-MQ", "-MJ", "--dependency-file"}


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def canonical_json(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def read_tsv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        return list(reader.fieldnames or []), list(reader)


def require_fields(path: Path, required: set[str]) -> list[dict[str, str]]:
    fields, rows = read_tsv(path)
    missing = sorted(required - set(fields))
    if missing:
        raise ValueError(f"{path} is missing fields: {', '.join(missing)}")
    return rows


def write_bytes(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as handle:
        handle.write(data)
        handle.flush()
        os.fsync(handle.fileno())


def write_tsv(path: Path, fields: list[str], rows: Iterable[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field, "") for field in fields})
        handle.flush()
        os.fsync(handle.fileno())


def fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def fsync_tree(root: Path) -> None:
    for directory, _, _ in os.walk(root, topdown=False):
        fsync_directory(Path(directory))


def ensure_branch(root: Path) -> None:
    branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip()
    if branch != BRANCH:
        raise SystemExit(f"required branch {BRANCH!r}; current branch is {branch!r}")


def ensure_linux_revision(root: Path, expected: str) -> None:
    linux = root / "vendor/linux"
    actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=linux, text=True).strip()
    if actual != expected:
        raise ValueError(f"vendor/linux revision differs from vendor/linux.SHA: {actual} != {expected}")
    dirty = subprocess.check_output(["git", "status", "--porcelain"], cwd=linux, text=True)
    if dirty:
        raise ValueError("vendor/linux has local changes; predicate discovery requires the exact pinned tree")


def normalize_path(value: str) -> str:
    return os.path.normpath(value).replace(os.sep, "/")


def command_argv(entry: dict[str, object]) -> list[str]:
    if isinstance(entry.get("arguments"), list):
        return [str(item) for item in entry["arguments"]]
    command = entry.get("command")
    if not isinstance(command, str) or not command:
        raise ValueError("compile command has neither arguments nor command")
    return shlex.split(command)


def command_output(argv: list[str]) -> str:
    output = ""
    index = 1
    while index < len(argv):
        token = argv[index]
        if token == "-o" and index + 1 < len(argv):
            output = argv[index + 1]
            index += 2
            continue
        if token.startswith("-o") and len(token) > 2:
            output = token[2:]
        index += 1
    return normalize_path(output)


def make_assignment(text: str, variable: str) -> str:
    lines = text.splitlines()
    pattern = re.compile(rf"^{re.escape(variable)}\s*:=\s*(.*)$")
    for index, line in enumerate(lines):
        match = pattern.match(line)
        if not match:
            continue
        parts: list[str] = []
        value = match.group(1)
        while True:
            continued = value.rstrip().endswith("\\")
            parts.append(value.rstrip()[:-1] if continued else value)
            if not continued:
                return " ".join(parts).strip()
            index += 1
            if index >= len(lines):
                raise ValueError(f"unterminated {variable} assignment")
            value = lines[index].strip()
    raise ValueError(f"missing {variable} assignment")


def cmd_evidence_path(build: Path, object_path: str) -> Path:
    object_name = Path(normalize_path(object_path))
    return build / object_name.parent / f".{object_name.name}.cmd"


def parse_cmd_evidence(path: Path, object_path: str) -> tuple[list[str], str, list[str]]:
    text = path.read_text(encoding="utf-8", errors="strict")
    saved_names = re.findall(r"^savedcmd_([^\s]+)\s*:=", text, flags=re.MULTILINE)
    matching_names = [name for name in saved_names if normalize_path(name) == normalize_path(object_path)]
    if len(matching_names) != 1:
        raise ValueError(
            f"expected one saved compiler command for {object_path} in {path}, found {matching_names}"
        )
    object_name = matching_names[0]
    command = make_assignment(text, f"savedcmd_{object_name}")
    source = make_assignment(text, f"source_{object_name}")
    dependencies = make_assignment(text, f"deps_{object_name}")
    dependency_text = re.sub(r"\$\(\s*wildcard\s+[^)]*\)", "", dependencies)
    if "$(" in dependency_text or "${" in dependency_text:
        raise ValueError(f"unhandled make expansion in dependency assignment: {path}")
    dependency_values = shlex.split(dependency_text, comments=False, posix=True)
    argv = shlex.split(command, comments=False, posix=True)
    # Kbuild may append a post-compile objtool command after a shell semicolon.
    # The retained .cmd file remains the authoritative record and is hashed in
    # full; the compiler predicate context is exactly its first compiler stage.
    if ";" in argv:
        argv = argv[:argv.index(";")]
    if not argv:
        raise ValueError(f"empty compiler stage in {path}")
    source_values = shlex.split(source, comments=False, posix=True)
    if len(source_values) != 1:
        raise ValueError(f"expected one source in {path}, found {len(source_values)}")
    return argv, source_values[0], dependency_values


def resolve_entry_source(entry: dict[str, object], directory: Path) -> Path:
    value = entry.get("file")
    if not isinstance(value, str) or not value:
        raise ValueError("compile command has no file")
    candidate = Path(value)
    return (directory / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()


def source_id(path: Path, root: Path, build: Path, arch: str) -> str:
    try:
        return path.relative_to(root / "vendor/linux").as_posix()
    except ValueError:
        try:
            return f"generated/{arch}/{path.relative_to(build).as_posix()}"
        except ValueError as exc:
            raise ValueError(f"selected source is outside pinned Linux and frozen build tree: {path}") from exc


def evidence_path(path: Path, root: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(root).as_posix()
    except ValueError:
        return str(resolved)


def compiler_environment(environment_rows: list[dict[str, str]], architecture: str) -> dict[str, str]:
    frozen: dict[str, str] = {}
    for selected_architecture in ("common", architecture):
        for row in environment_rows:
            if row.get("architecture") != selected_architecture:
                continue
            key = row["key"]
            if key in frozen:
                raise ValueError(f"duplicate frozen environment key for {selected_architecture}: {key}")
            if row.get("status") == "NEUTRALIZED" or row["value"] == "__UNSET__":
                continue
            if row.get("status") != "FROZEN":
                raise ValueError(f"environment key is not FROZEN or NEUTRALIZED: {selected_architecture}:{key}")
            frozen[key] = "" if row["value"] == "(empty)" else row["value"]
    required = {"PATH", "LLVM", "LLVM_IAS"}
    missing = sorted(required - set(frozen))
    if missing:
        raise ValueError(f"ENVIRONMENT.tsv lacks frozen values: {missing}")
    if frozen["LLVM"] != "/usr/lib/llvm-19/bin/" or frozen["LLVM_IAS"] != "1":
        raise ValueError("frozen environment does not select canonical LLVM 19 with LLVM_IAS=1")
    return {
        "PATH": frozen["PATH"],
        "LLVM": frozen["LLVM"],
        "LLVM_IAS": frozen["LLVM_IAS"],
        **{key: value for key, value in frozen.items() if key not in required},
        "LC_ALL": "C",
        "LANG": "C",
        "TZ": "UTC",
    }


def compiler_identity(toolchain_rows: list[dict[str, str]]) -> dict[str, str]:
    matches = [row for row in toolchain_rows if row.get("tool_name") == "clang"]
    if len(matches) != 1:
        raise ValueError(f"expected one clang row in TOOLCHAIN.tsv, found {len(matches)}")
    row = matches[0]
    requested = Path(row["requested_path"])
    if requested != Path("/usr/lib/llvm-19/bin/clang"):
        raise ValueError(f"non-canonical clang requested path: {requested}")
    if row.get("status") != "VERIFIED" or not requested.is_file():
        raise ValueError("canonical clang is not VERIFIED or is missing")
    if sha256_file(requested) != row.get("sha256"):
        raise ValueError("canonical clang hash differs from TOOLCHAIN.tsv")
    resolved = requested.resolve()
    recorded_resolved = Path(row["resolved_path"]).resolve()
    if resolved != recorded_resolved:
        raise ValueError(f"clang resolution changed: recorded={recorded_resolved}; actual={resolved}")
    return row


def target_triple(argv: list[str]) -> str:
    result = ""
    index = 1
    while index < len(argv):
        token = argv[index]
        if token == "--target" and index + 1 < len(argv):
            result = argv[index + 1]
            index += 2
            continue
        if token.startswith("--target="):
            result = token.split("=", 1)[1]
        index += 1
    if not result:
        raise ValueError("selected compile command has no explicit --target")
    return result


def transformed_probe_command(
    argv: list[str], source: Path, directory: Path, compiler_path: str
) -> tuple[list[str], str]:
    if not argv or argv[0] != compiler_path:
        raise ValueError(f"compile command compiler is {argv[0] if argv else '(empty)'}, expected {compiler_path}")
    if any(token.startswith("@") for token in argv[1:]):
        raise ValueError("response-file compile commands are not reconstructable without separate response evidence")
    language = "assembler-with-cpp" if source.suffix == ".S" else "c++" if source.suffix in {".cc", ".cpp", ".cxx"} else "c"
    transformed = [argv[0]]
    removed_sources = 0
    index = 1
    while index < len(argv):
        token = argv[index]
        if token == "-c" or token in DEPENDENCY_FLAGS:
            index += 1
            continue
        if token in {"-o", "-x", *DEPENDENCY_VALUE_FLAGS}:
            if index + 1 >= len(argv):
                raise ValueError(f"compile command ends after {token}")
            if token == "-x":
                language = argv[index + 1]
            index += 2
            continue
        if any(token.startswith(flag) and token != flag for flag in DEPENDENCY_VALUE_FLAGS):
            index += 1
            continue
        if token.startswith(("-Wp,-MD,", "-Wp,-MMD,", "-Wp,-MF,", "-Wp,-MT,", "-Wp,-MQ,")):
            index += 1
            continue
        if not token.startswith("-"):
            candidate = Path(token)
            resolved = (directory / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()
            if resolved == source:
                removed_sources += 1
                index += 1
                continue
        transformed.append(token)
        index += 1
    if removed_sources != 1:
        raise ValueError(f"expected to remove exactly one compile input, removed {removed_sources}")
    transformed.extend(["-E", "-P", "-x", language, "-"])
    return transformed, language


def resolve_dependency(value: str, root: Path, build: Path) -> Path | None:
    token = value.strip().rstrip("\\")
    if not token or "$" in token or token in {":=", ":"}:
        return None
    candidate = Path(token)
    candidates = [candidate] if candidate.is_absolute() else [build / candidate, root / "vendor/linux" / candidate]
    for item in candidates:
        resolved = item.resolve()
        if resolved.is_file():
            try:
                resolved.relative_to(root / "vendor/linux")
                return resolved
            except ValueError:
                try:
                    resolved.relative_to(build)
                    return resolved
                except ValueError:
                    return None
    if Path(token).suffix.lower() in {".h", ".inc", ".def"}:
        raise ValueError(f"selected header dependency is missing: {value}")
    return None


def forced_includes(argv: list[str], directory: Path, root: Path, build: Path) -> set[Path]:
    result: set[Path] = set()
    index = 1
    while index < len(argv):
        token = argv[index]
        value = ""
        if token == "-include" and index + 1 < len(argv):
            value = argv[index + 1]
            index += 2
        elif token.startswith("-include") and token != "-include":
            value = token[len("-include"):]
            index += 1
        else:
            index += 1
        if value:
            resolved = resolve_dependency(value, root, build)
            if resolved is None:
                candidate = Path(value)
                candidate = (directory / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()
                if not candidate.is_file():
                    raise ValueError(f"forced include is missing: {value}")
                resolved = candidate
            result.add(resolved)
    return result


def mask_comments(text: str) -> str:
    output = list(text)
    index = 0
    state = "code"
    while index < len(text):
        char = text[index]
        nxt = text[index + 1] if index + 1 < len(text) else ""
        if state == "code" and char == "/" and nxt == "/":
            output[index] = output[index + 1] = " "
            state = "line"
            index += 2
        elif state == "code" and char == "/" and nxt == "*":
            output[index] = output[index + 1] = " "
            state = "block"
            index += 2
        elif state == "line":
            if char == "\n":
                state = "code"
            else:
                output[index] = " "
            index += 1
        elif state == "block":
            if char == "*" and nxt == "/":
                output[index] = output[index + 1] = " "
                state = "code"
                index += 2
            else:
                if char != "\n":
                    output[index] = " "
                index += 1
        else:
            index += 1
    return "".join(output)


def logical_define(text: str, offset: int) -> tuple[set[str], bool]:
    line_start = text.rfind("\n", 0, offset) + 1
    while line_start > 0:
        prior_end = line_start - 1
        prior_start = text.rfind("\n", 0, prior_end) + 1
        if not text[prior_start:prior_end].rstrip().endswith("\\"):
            break
        line_start = prior_start
    line_end = text.find("\n", offset)
    if line_end < 0:
        line_end = len(text)
    while text[line_start:line_end].rstrip().endswith("\\") and line_end < len(text):
        next_end = text.find("\n", line_end + 1)
        line_end = len(text) if next_end < 0 else next_end
    directive = text[line_start:line_end].replace("\\\n", " ")
    match = re.match(r"\s*#\s*define\s+[A-Za-z_]\w*\s*(?:\(([^)]*)\))?", directive)
    if not match:
        return set(), False
    parameters = {value.strip() for value in (match.group(1) or "").split(",") if value.strip()}
    return parameters, True


def predicate_occurrences(path: Path, location: str) -> list[tuple[str, str, str]]:
    text = path.read_text(errors="replace")
    masked = mask_comments(text)
    pattern = re.compile(r"\b(" + "|".join(re.escape(kind) for kind in KINDS) + r")\s*\(")
    result: list[tuple[str, str, str]] = []
    for match in pattern.finditer(masked):
        open_paren = masked.find("(", match.start(), match.end())
        depth = 1
        index = open_paren + 1
        state = "code"
        while index < len(masked) and depth:
            char = masked[index]
            nxt = masked[index + 1] if index + 1 < len(masked) else ""
            if state == "code" and char in {'"', "'"}:
                state = "string" if char == '"' else "char"
            elif state in {"string", "char"} and char == "\\" and nxt:
                index += 1
            elif state == "string" and char == '"':
                state = "code"
            elif state == "char" and char == "'":
                state = "code"
            elif state == "code" and char == "(":
                depth += 1
            elif state == "code" and char == ")":
                depth -= 1
            index += 1
        if depth:
            raise ValueError(f"unterminated {match.group(1)} invocation in {location}")
        argument = text[open_paren + 1:index - 1].strip()
        if not argument:
            raise ValueError(f"empty {match.group(1)} argument in {location}")
        parameters, is_define = logical_define(text, match.start())
        if is_define and (argument in parameters or re.match(r"\s*#\s*define\s+" + re.escape(match.group(1)), text[text.rfind("\n", 0, match.start()) + 1:match.start()])):
            continue
        line = text.count("\n", 0, match.start()) + 1
        column = match.start() - text.rfind("\n", 0, match.start())
        result.append((match.group(1), re.sub(r"\s+", " ", argument), f"{location}:{line}:{column}"))
    return result


def probe_source(predicate_id: str, kind: str, argument: str) -> bytes:
    marker_one = f"LUPOS_COMPILER_PREDICATE_{predicate_id}_1"
    marker_zero = f"LUPOS_COMPILER_PREDICATE_{predicate_id}_0"
    return (
        f"#if {kind}({argument})\n{marker_one}\n#else\n{marker_zero}\n#endif\n"
    ).encode()


def parse_probe_result(predicate_id: str, stdout: bytes) -> str | None:
    text = stdout.decode("utf-8", errors="replace")
    markers = re.findall(rf"(?m)^LUPOS_COMPILER_PREDICATE_{re.escape(predicate_id)}_([01])\s*$", text)
    return markers[0] if len(markers) == 1 else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--rewrite-root", type=Path, default=Path("rewrite"))
    parser.add_argument("--out", type=Path, default=Path("rewrite/compiler-predicates"))
    parser.add_argument("--timeout-seconds", type=int, default=60)
    parser.add_argument("--execute", action="store_true", help="required acknowledgement that preprocess-only probes will run")
    args = parser.parse_args()
    if not args.execute:
        raise SystemExit("refusing to run compiler probes without --execute")
    root = args.root.resolve()
    rewrite = args.rewrite_root if args.rewrite_root.is_absolute() else root / args.rewrite_root
    output = args.out if args.out.is_absolute() else root / args.out
    ensure_branch(root)
    if output == root or output == rewrite or root not in output.parents:
        raise SystemExit(f"unsafe output directory: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    lock_path = output.parent / ".compiler-predicates.lock"
    with lock_path.open("a+b") as lock:
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        if output.exists():
            raise SystemExit(f"refusing to replace existing predicate evidence: {output}")

        scope_rows = require_fields(rewrite / "SCOPE.tsv", {"linux_path", "class", "architectures"})
        fmap_rows = require_fields(
            rewrite / "FILE_MAP.tsv", {"source_path", "object_path", "architecture", "compile_command"}
        )
        toolchain_rows = require_fields(
            rewrite / "toolchain/TOOLCHAIN.tsv",
            {"tool_name", "requested_path", "resolved_path", "sha256", "version", "status"},
        )
        environment_rows = require_fields(rewrite / "toolchain/ENVIRONMENT.tsv", {"architecture", "key", "value", "status"})
        compiler = compiler_identity(toolchain_rows)
        toolchain_sha = (rewrite / "toolchain/TOOLCHAIN.sha256").read_text().split()[0]
        linux_commit = (root / "vendor/linux.SHA").read_text().strip()
        ensure_linux_revision(root, linux_commit)
        active_sources = {row["linux_path"] for row in scope_rows if row.get("class") != "OUT_OF_SCOPE"}

        # A predicate inventory is unique by builtin spelling and architecture,
        # not by every translation unit that happens to include the same header.
        # Stream each selected .cmd closure so a complete inventory does not
        # retain millions of duplicate Path objects in memory.  The retained
        # context is a deterministic relevant Kbuild command: a direct source
        # occurrence wins over a header occurrence, then source/object/.cmd hash
        # provides a stable tie-breaker.
        occurrence_cache: dict[Path, list[tuple[str, str, str]]] = {}
        grouped: dict[tuple[str, str, str], dict[str, object]] = {}
        for arch in ("x86_64", "aarch64"):
            build = rewrite / "kbuild" / arch
            database_path = rewrite / "metadata" / arch / "compile_commands.json"
            database = json.loads(database_path.read_text(encoding="utf-8"))
            if not isinstance(database, list):
                raise ValueError(f"{database_path} is not a JSON array")
            fmap: dict[tuple[str, str], dict[str, str]] = {}
            for row in fmap_rows:
                if row.get("architecture") != arch or row.get("source_path") not in active_sources:
                    continue
                key = (normalize_path(row["source_path"]), normalize_path(row["object_path"]))
                if key in fmap:
                    raise ValueError(f"duplicate FILE_MAP row for {arch}: {key}")
                fmap[key] = row
            matched_fmap: set[tuple[str, str]] = set()
            database_index: dict[tuple[str, str], tuple[int, dict[str, object], Path, Path]] = {}
            for entry_index, entry in enumerate(database):
                if not isinstance(entry, dict):
                    raise ValueError(f"{database_path} entry {entry_index} is not an object")
                directory = Path(str(entry["directory"])).resolve()
                source = resolve_entry_source(entry, directory)
                sid = source_id(source, root, build, arch)
                object_path = command_output(command_argv(entry))
                key = (sid, object_path)
                if key in database_index:
                    raise ValueError(f"duplicate compile database context for {arch}: {(sid, object_path)}")
                database_index[key] = (entry_index, entry, directory, source)
            for key, fmap_row in sorted(fmap.items()):
                sid, object_path = key
                if key not in database_index:
                    continue
                entry_index, entry, directory, database_source = database_index[key]
                command_path = cmd_evidence_path(build, object_path)
                if not command_path.is_file():
                    raise ValueError(
                        f"selected source {sid} ({arch}, {object_path}) lacks authoritative Kbuild .cmd evidence: {command_path}"
                    )
                cmd_argv, cmd_source_value, dependency_values = parse_cmd_evidence(command_path, object_path)
                cmd_source = Path(cmd_source_value)
                cmd_source = (build / cmd_source).resolve() if not cmd_source.is_absolute() else cmd_source.resolve()
                if cmd_source != database_source or source_id(cmd_source, root, build, arch) != sid:
                    raise ValueError(f"Kbuild .cmd source disagrees with compile database for {arch}:{object_path}")
                database_argv = command_argv(entry)
                if cmd_argv != database_argv:
                    raise ValueError(f"Kbuild .cmd command disagrees with compile database for {arch}:{object_path}")
                if shlex.split(fmap_row["compile_command"], comments=False, posix=True) != cmd_argv:
                    raise ValueError(f"Kbuild .cmd command disagrees with FILE_MAP for {arch}:{object_path}")
                transformed, language = transformed_probe_command(
                    cmd_argv, cmd_source, build.resolve(), compiler["requested_path"]
                )
                target = target_triple(cmd_argv)
                files = {cmd_source}
                if not dependency_values:
                    raise ValueError(f"empty Kbuild dependency closure in {command_path}")
                for dependency in dependency_values:
                    resolved = resolve_dependency(dependency, root, build)
                    if resolved is not None:
                        files.add(resolved)
                forced = forced_includes(cmd_argv, build, root, build)
                missing_forced = sorted(forced - files)
                if missing_forced:
                    raise ValueError(f"Kbuild dependency closure omits forced includes in {command_path}: {missing_forced}")
                matched_fmap.add(key)
                context = {
                    "architecture": arch,
                    "build": build,
                    "database_path": database_path,
                    "entry_index": entry_index,
                    "entry": entry,
                    "entry_sha256": sha256_bytes(canonical_json(entry)),
                    "directory": build.resolve(),
                    "command_path": command_path.resolve(),
                    "command_sha256": sha256_file(command_path),
                    "source_id": sid,
                    "object_path": object_path,
                    "argv": transformed,
                    "language": language,
                    "target_triple": target,
                    "environment": compiler_environment(environment_rows, arch),
                    "source": cmd_source,
                }
                for input_path in sorted(files):
                    if not input_path.is_file():
                        raise ValueError(f"selected predicate input is missing: {input_path}")
                    location = evidence_path(input_path, root)
                    if input_path not in occurrence_cache:
                        occurrence_cache[input_path] = predicate_occurrences(input_path, location)
                    for kind, argument, source_location in occurrence_cache[input_path]:
                        group_key = (kind, argument, arch)
                        candidate_key = (
                            0 if input_path.resolve() == cmd_source else 1,
                            sid,
                            object_path,
                            str(context["command_sha256"]),
                        )
                        prior = grouped.get(group_key)
                        if prior is None:
                            grouped[group_key] = {
                                "context": context,
                                "context_key": candidate_key,
                                "locations": {source_location},
                            }
                        else:
                            prior_locations = prior["locations"]
                            assert isinstance(prior_locations, set)
                            prior_locations.add(source_location)
                            if candidate_key < prior["context_key"]:
                                prior["context"] = context
                                prior["context_key"] = candidate_key
            missing_contexts = sorted(set(fmap) - matched_fmap)
            if missing_contexts:
                preview = ", ".join(f"{source}->{obj}" for source, obj in missing_contexts[:5])
                raise ValueError(
                    f"{len(missing_contexts)} selected FILE_MAP rows lack compile database contexts for {arch}: {preview}"
                )

        if not grouped:
            raise ValueError("no compiler predicate invocations were discovered in selected sources or headers")

        temporary = Path(tempfile.mkdtemp(prefix=f".{output.name}.", dir=output.parent))
        try:
            for directory in ("commands", "stdout", "stderr", "probes"):
                (temporary / directory).mkdir()
            result_rows: list[dict[str, str]] = []
            used_ids: set[str] = set()
            for kind, argument, arch in sorted(grouped):
                group = grouped[(kind, argument, arch)]
                context = group["context"]
                assert isinstance(context, dict)
                identity_material = (
                    f"{kind}\0{argument}\0{context['architecture']}\0{context['command_sha256']}"
                ).encode()
                predicate_id = "CP" + sha256_bytes(identity_material)[:24]
                if predicate_id in used_ids:
                    raise ValueError(f"predicate id collision: {predicate_id}")
                used_ids.add(predicate_id)
                probe_rel = Path("probes") / f"{predicate_id}.c"
                command_rel = Path("commands") / f"{predicate_id}.json"
                stdout_rel = Path("stdout") / f"{predicate_id}.txt"
                stderr_rel = Path("stderr") / f"{predicate_id}.txt"
                probe = probe_source(predicate_id, kind, argument)
                final_probe = output / probe_rel
                command_evidence = {
                    "schema_version": "compiler-predicate-command-v1",
                    "predicate_id": predicate_id,
                    "cwd": str(context["directory"]),
                    "argv": context["argv"],
                    "environment": context["environment"],
                    "stdin_path": evidence_path(final_probe, root),
                    "original_command_source": (
                        evidence_path(Path(context["command_path"]), root)
                    ),
                    "original_command_sha256": context["command_sha256"],
                    "source_path": context["source_id"],
                    "object_path": context["object_path"],
                    "compile_database_source": (
                        f"{evidence_path(Path(context['database_path']), root)}#entry={context['entry_index']}"
                    ),
                    "compile_database_entry_sha256": context["entry_sha256"],
                }
                command_bytes = canonical_json(command_evidence)
                write_bytes(temporary / probe_rel, probe)
                write_bytes(temporary / command_rel, command_bytes)
                started = utc_now()
                completed_process = subprocess.run(
                    context["argv"],
                    cwd=context["directory"],
                    env=context["environment"],
                    input=probe,
                    capture_output=True,
                    timeout=args.timeout_seconds,
                    check=False,
                )
                completed = utc_now()
                stdout = completed_process.stdout
                stderr = completed_process.stderr
                write_bytes(temporary / stdout_rel, stdout)
                write_bytes(temporary / stderr_rel, stderr)
                result = parse_probe_result(predicate_id, stdout)
                if result not in {"0", "1"}:
                    raise ValueError(
                        f"probe {predicate_id} produced no unique result marker; exit={completed_process.returncode}"
                    )
                status = "PROVEN" if completed_process.returncode == 0 else "BLOCKED"
                if status not in STATUS_VALUES:
                    raise AssertionError(status)
                arch = str(context["architecture"])
                config_path = rewrite / "configs" / arch / "frozen.config"
                original_source = command_evidence["original_command_source"]
                result_rows.append({
                    "predicate_id": predicate_id,
                    "predicate_kind": kind,
                    "argument": argument,
                    "architecture": arch,
                    "target_triple": str(context["target_triple"]),
                    "result": result,
                    "source_locations": ";".join(sorted(group["locations"])),
                    "linux_commit": linux_commit,
                    "config_sha256": sha256_file(config_path),
                    "toolchain_sha256": toolchain_sha,
                    "compiler_requested_path": compiler["requested_path"],
                    "compiler_resolved_path": compiler["resolved_path"],
                    "compiler_sha256": compiler["sha256"],
                    "compiler_version": compiler["version"],
                    "original_command_source": original_source,
                    "original_command_sha256": str(context["command_sha256"]),
                    "probe_path": evidence_path(output / probe_rel, root),
                    "probe_sha256": sha256_bytes(probe),
                    "probe_command_path": evidence_path(output / command_rel, root),
                    "probe_command_sha256": sha256_bytes(command_bytes),
                    "stdout_path": evidence_path(output / stdout_rel, root),
                    "stdout_sha256": sha256_bytes(stdout),
                    "stderr_path": evidence_path(output / stderr_rel, root),
                    "stderr_sha256": sha256_bytes(stderr),
                    "exit_status": str(completed_process.returncode),
                    "started_at": started,
                    "completed_at": completed,
                    "status": status,
                })
            inventory = temporary / "COMPILER_PREDICATES.tsv"
            write_tsv(inventory, FIELDS, result_rows)
            fingerprint = (
                f"sha256\t{sha256_file(inventory)}\n"
                f"rows\t{len(result_rows)}\n"
                f"linux_commit\t{linux_commit}\n"
                f"toolchain_sha256\t{toolchain_sha}\n"
                f"generator_sha256\t{sha256_file(Path(__file__))}\n"
                f"created_at\t{utc_now()}\n"
            ).encode()
            write_bytes(temporary / "COMPILER_PREDICATES.sha256", fingerprint)
            fsync_tree(temporary)
            os.replace(temporary, output)
            fsync_directory(output.parent)
        except BaseException:
            if temporary.exists():
                shutil.rmtree(temporary)
            raise
    blocked = sum(row["status"] == "BLOCKED" for row in result_rows)
    print(json.dumps({"output": str(output), "predicates": len(result_rows), "blocked": blocked}, sort_keys=True))
    return 1 if blocked else 0


if __name__ == "__main__":
    raise SystemExit(main())
