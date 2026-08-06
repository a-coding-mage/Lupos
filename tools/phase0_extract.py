#!/usr/bin/env python3
"""Extract mechanically provable, source-only Phase 0 manifests.

This tool deliberately does not create ``TRANSLATION_TASKS.tsv``.  The queue is
created only by ``tools/rewrite_queue.py init`` after these manifests have been
independently reviewed and validated.
"""

from __future__ import annotations

import argparse
import ast
from collections import defaultdict
import csv
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import tarfile
from typing import Iterable


SCOPE_FIELDS = [
    "id", "linux_path", "destination_path", "class", "architectures",
    "kconfig_evidence", "kbuild_target", "cluster", "weight", "risk",
    "dependencies", "recommended_implementer", "source_kind",
    "metadata_status", "metadata_evidence", "semantic_status",
]
FILE_MAP_FIELDS = [
    "source_path", "object_path", "architecture", "module_or_builtin",
    "kbuild_owner", "disposition_evidence", "compile_input", "compile_command",
    "metadata_evidence",
]
SYMBOL_FIELDS = [
    "scope_id", "linux_path", "architectures", "record_kind", "symbol_name",
    "source_line", "selection_expression", "config_evidence", "linkage",
    "declaration", "evidence", "status",
]
ABI_FIELDS = [
    "scope_id", "linux_path", "architectures", "record_kind", "symbol_name",
    "source_line", "abi_item", "linkage", "export_kind", "declaration",
    "layout", "alignment", "calling_convention", "config_evidence", "evidence",
    "status",
]
LIFETIME_FIELDS = [
    "scope_id", "linux_path", "architectures", "record_kind", "symbol_name",
    "source_line", "lifetime_item", "storage_duration", "ownership",
    "lifetime_contract", "locking_rcu_refcount", "config_evidence", "evidence",
    "status",
]
DRIVER_ABI_FIELDS = [
    "scope_id", "linux_path", "architectures", "object_path", "kbuild_owner",
    "module_or_builtin", "record_kind", "abi_item", "evidence", "status",
]
HEADER_CLOSURE_FIELDS = [
    "architecture", "header_path", "header_kind", "class",
    "consumer_count", "rust_consumer_count", "consumer_classes", "evidence",
]
HEADER_INCLUDE_EDGE_FIELDS = [
    "architecture", "including_header", "including_kind", "included_header",
    "included_kind", "relationship", "directive", "consumer_source",
    "consumer_object", "evidence",
]
HEADER_CONTEXT_EDGE_FIELDS = [
    "architecture", "header_path", "provider_header", "relationship",
    "consumer_source", "consumer_object", "header_position",
    "provider_position", "provided_identifiers", "provider_origin", "evidence",
]
HEADER_COMPONENT_FIELDS = [
    "component_id", "member_path", "member_order", "component_size", "tail_path",
]
TASK_DEPENDENCY_FIELDS = [
    "task_id", "linux_path", "dependency_task_id", "dependency_linux_path",
    "reason", "evidence",
]
ORACLE_CLASSIFICATION_FIELDS = [
    "linux_path", "source_kind", "reason", "evidence",
]

ARCH_CONFIG_NAMES = {"x86_64": "x86_64", "aarch64": "aarch64"}
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
DEFINE_RE = re.compile(r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)(\s*\([^\n]*?\))?\s*(.*)$", re.S)
UNDEF_RE = re.compile(r"^\s*#\s*undef\s+([A-Za-z_][A-Za-z0-9_]*)\b")
CONDITIONAL_RE = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)$", re.S)
EXPORT_RE = re.compile(r"\bEXPORT_SYMBOL(_GPL)?\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")
TYPE_TAG_RE = re.compile(r"\b(struct|union|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
TYPE_BODY_RE = re.compile(r"\b(struct|union|enum)(?:\s+([A-Za-z_][A-Za-z0-9_]*))?\s*\{")
TYPEDEF_NAME_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^;]*\])?\s*;$")
TYPEDEF_RE = re.compile(r"\btypedef\b.*?;", re.S)
TYPEDEF_FUNCTION_POINTER_RE = re.compile(r"\(\s*\*\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")
TYPEDEF_FUNCTION_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*\(")
STATIC_DEFINE_MACRO_RE = re.compile(
    r"\bDEFINE_(?:RAW_)?SPINLOCK\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)"
)
INCLUDED_C_RE = re.compile(r'^\s*#\s*include\s*"([^"\n]+\.c)"', re.M)
INCLUDE_RE = re.compile(r'^\s*#\s*include\s*([<"])([^>"\n]+)[>"]', re.M)
FUNCTION_MACRO_RE = re.compile(
    r"\b((?:COMPAT_)?SYSCALL_DEFINE\d+|DEFINE_[A-Z0-9_]*SHOW_ATTRIBUTE|"
    r"BPF_CALL_\d+|TRACE_EVENT)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)"
)
PREDICATE_KINDS = {
    "__has_attribute", "__has_builtin", "__has_feature", "__has_extension",
    "__has_c_attribute", "__has_declspec_attribute", "__has_warning",
}
PREDICATE_FIELDS = {
    "predicate_id", "predicate_kind", "argument", "architecture", "result",
    "status", "source_locations", "linux_commit", "config_sha256",
    "toolchain_sha256",
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def predicate_value_map(predicate_root: Path, linux_commit: str) -> tuple[dict[str, dict[tuple[str, str], int]], dict[str, str]]:
    """Load only independently validated compiler builtin results.

    Compiler predicates affect mechanical source selection, so the lexical
    selected-line extractor must consume the frozen probe evidence rather than
    pretending the builtin is an ordinary undefined macro.
    """

    inventory_path = predicate_root / "COMPILER_PREDICATES.tsv"
    fingerprint_path = predicate_root / "COMPILER_PREDICATES.sha256"
    validation_path = predicate_root / "VALIDATION.tsv"
    report_path = predicate_root / "validation-report.md"
    if not all(path.is_file() for path in (inventory_path, fingerprint_path, validation_path, report_path)):
        raise ValueError(f"missing compiler predicate evidence under {predicate_root}")
    with inventory_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if reader.fieldnames is None or not PREDICATE_FIELDS <= set(reader.fieldnames):
            raise ValueError(f"invalid compiler predicate inventory schema: {inventory_path}")
        inventory = list(reader)
    with validation_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if reader.fieldnames is None or not {"predicate_id", "validation_status"} <= set(reader.fieldnames):
            raise ValueError(f"invalid compiler predicate validation schema: {validation_path}")
        validation = {row["predicate_id"]: row.get("validation_status", "") for row in reader}
    fingerprint = {}
    for line in fingerprint_path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("\t")
        if separator:
            fingerprint[key] = value
    if fingerprint.get("sha256") != sha256(inventory_path) or fingerprint.get("rows") != str(len(inventory)):
        raise ValueError("compiler predicate inventory fingerprint mismatch")
    if fingerprint.get("linux_commit") != linux_commit:
        raise ValueError("compiler predicate inventory Linux commit mismatch")
    values: dict[str, dict[tuple[str, str], int]] = defaultdict(dict)
    for row in inventory:
        arch = row.get("architecture", "")
        key = (row.get("predicate_kind", ""), re.sub(r"\s+", " ", row.get("argument", "").strip()))
        if arch not in {"x86_64", "aarch64"} or key[0] not in PREDICATE_KINDS:
            raise ValueError(f"invalid compiler predicate row: {row.get('predicate_id', '')}")
        if row.get("linux_commit") != linux_commit or row.get("status") != "PROVEN" or row.get("result") not in {"0", "1"}:
            raise ValueError(f"unproven compiler predicate row: {row.get('predicate_id', '')}")
        if validation.get(row.get("predicate_id", "")) != "PASS":
            raise ValueError(f"compiler predicate did not pass independent validation: {row.get('predicate_id', '')}")
        if key in values[arch]:
            raise ValueError(f"duplicate compiler predicate for {arch}: {key}")
        values[arch][key] = int(row["result"])
    if not values["x86_64"] or not values["aarch64"]:
        raise ValueError("compiler predicate inventory lacks one approved architecture")
    if "- Result: PASS" not in report_path.read_text(encoding="utf-8"):
        raise ValueError("compiler predicate validation report is not PASS")
    binding = {
        "compiler_predicates_sha256": sha256(inventory_path),
        "compiler_predicates_validation_sha256": sha256(validation_path),
        "compiler_predicates_schema_version": "compiler-predicates-v1",
        "compiler_predicates_count": str(len(inventory)),
        "compiler_predicates_x86_64_count": str(len(values["x86_64"])),
        "compiler_predicates_aarch64_count": str(len(values["aarch64"])),
        "compiler_predicates_validation_status": "PASS",
    }
    return values, binding


def write_tsv(path: Path, fields: list[str], rows: Iterable[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows({field: row.get(field, "") for field in fields} for row in rows)


def normalize_path(value: str) -> str:
    return os.path.normpath(value).replace(os.sep, "/")


def make_assignment(text: str, variable: str) -> str | None:
    """Read one retained Kbuild simple assignment and its continuations."""

    lines = text.splitlines()
    pattern = re.compile(rf"^{re.escape(variable)}\s*:=\s*(.*)$")
    for index, line in enumerate(lines):
        match = pattern.match(line)
        if match is None:
            continue
        value = match.group(1)
        pieces: list[str] = []
        while True:
            continuation = value.rstrip().endswith("\\")
            pieces.append(value.rstrip()[:-1] if continuation else value)
            if not continuation:
                return " ".join(pieces).strip()
            index += 1
            if index >= len(lines):
                raise ValueError(f"unterminated {variable} assignment")
            value = lines[index].strip()
    return None


def command_evidence_path(build: Path, object_path: str) -> Path:
    relative = Path(normalize_path(object_path))
    return build / relative.parent / f".{relative.name}.cmd"


def canonical_dependency(
    value: str, linux: Path, build: Path, arch: str,
) -> tuple[str, str] | None:
    """Return a selected dependency as (manifest path, origin kind)."""

    candidate = Path(value)
    if not candidate.is_absolute():
        candidate = build / candidate
    candidate = Path(os.path.abspath(os.path.normpath(candidate)))
    linux_abs = Path(os.path.abspath(linux))
    build_abs = Path(os.path.abspath(build))
    try:
        return candidate.relative_to(linux_abs).as_posix(), "linux"
    except ValueError:
        pass
    try:
        generated = candidate.relative_to(build_abs).as_posix()
    except ValueError:
        return None
    return f"generated/{arch}/{generated}", "generated"


def dependency_headers(
    build: Path, linux: Path, arch: str, object_path: str,
) -> tuple[list[tuple[str, str]], str]:
    """Read the compiler-emitted transitive ``.h`` closure for one object."""

    command_file = command_evidence_path(build, object_path)
    if not command_file.is_file():
        raise ValueError(f"missing retained Kbuild command evidence for {arch}:{object_path}")
    content = command_file.read_text(encoding="utf-8", errors="strict")
    dependency_variables = re.findall(r"^(deps_[^\s:]+)\s*:=", content, flags=re.M)
    matching_variables = [
        variable for variable in dependency_variables
        if normalize_path(variable[len("deps_"):]) == normalize_path(object_path)
    ]
    if len(matching_variables) != 1:
        raise ValueError(f"missing dependency assignment for {arch}:{object_path} in {command_file}")
    assignment = make_assignment(content, matching_variables[0])
    if assignment is None:
        raise ValueError(f"cannot parse {matching_variables[0]} in {command_file}")
    result: list[tuple[str, str]] = []
    seen: set[str] = set()
    for token in assignment.split():
        token = token.rstrip("\\")
        if not token.endswith(".h"):
            continue
        normalized = canonical_dependency(token, linux, build, arch)
        if normalized is None or normalized[0] in seen:
            continue
        seen.add(normalized[0])
        result.append(normalized)
    return result, command_file.relative_to(build).as_posix()


def include_search_directories(command: str, directory: str) -> list[Path]:
    """Recover ordered quote/angle include roots from a frozen compile command."""

    tokens = shlex.split(command)
    base = Path(directory)
    result: list[Path] = []
    index = 1
    while index < len(tokens):
        token = tokens[index]
        value = ""
        if token in {"-I", "-iquote", "-isystem"} and index + 1 < len(tokens):
            value = tokens[index + 1]
            index += 2
        elif token.startswith("-I") and token != "-I":
            value = token[2:]
            index += 1
        elif token.startswith("-iquote") and token != "-iquote":
            value = token[len("-iquote"):]
            index += 1
        elif token.startswith("-isystem") and token != "-isystem":
            value = token[len("-isystem"):]
            index += 1
        else:
            index += 1
        if value:
            path = Path(value)
            result.append(path if path.is_absolute() else base / path)
    return result


def selected_header_file(header_path: str, linux: Path, build: Path, arch: str) -> Path:
    """Resolve one canonical selected-header path without consulting PATH."""

    generated_prefix = f"generated/{arch}/"
    if header_path.startswith(generated_prefix):
        return build / header_path[len(generated_prefix):]
    if header_path.startswith("generated/"):
        raise ValueError(f"generated header architecture mismatch: {arch}:{header_path}")
    return linux / header_path


def resolve_include(
    including_header: str,
    delimiter: str,
    include_value: str,
    context: dict[str, str],
    linux: Path,
    build: Path,
    arch: str,
) -> tuple[str, str] | None:
    candidates: list[Path] = []
    if delimiter == '"':
        candidates.append(
            selected_header_file(including_header, linux, build, arch).parent / include_value
        )
    candidates.extend(
        directory / include_value
        for directory in include_search_directories(context["compile_command"], context["directory"])
    )
    for candidate in candidates:
        if candidate.is_file():
            normalized = canonical_dependency(str(candidate), linux, build, arch)
            if normalized is not None:
                return normalized
    return None


def project_rust_header_dependencies(
    graph: dict[str, set[str]], rust_headers: set[str],
) -> dict[str, set[str]]:
    """Project direct includes through non-Rust/generated wrapper headers.

    A generated ``asm/*.h`` wrapper is build metadata rather than a queue task,
    but the first translated header below that wrapper remains a mechanical
    prerequisite.  Traversal stops at the first Rust task on each path so the
    task graph stays minimal and auditable.
    """

    result: dict[str, set[str]] = {path: set() for path in rust_headers}
    for source in sorted(rust_headers):
        pending = list(graph.get(source, set()))
        seen: set[str] = set()
        while pending:
            dependency = pending.pop()
            if dependency == source or dependency in seen:
                continue
            seen.add(dependency)
            if dependency in rust_headers:
                result[source].add(dependency)
                continue
            pending.extend(graph.get(dependency, set()) - seen)
    return result


def rel_linux(value: str, directory: Path, linux: Path, build: Path, arch: str) -> tuple[str, str]:
    candidate = Path(value)
    candidate = (directory / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()
    try:
        return candidate.relative_to(linux.resolve()).as_posix(), "linux"
    except ValueError:
        try:
            generated = candidate.relative_to(build.resolve()).as_posix()
        except ValueError:
            generated = candidate.name
        return f"generated/{arch}/{generated}", "generated"


def compile_entries(build: Path, linux: Path, arch: str) -> list[dict[str, str]]:
    database = build / "compile_commands.json"
    entries = json.loads(database.read_text(encoding="utf-8"))
    result: list[dict[str, str]] = []
    for entry in entries:
        directory = Path(entry["directory"]).resolve()
        command = entry.get("command") or shlex.join(entry["arguments"])
        tokens = shlex.split(command)
        output = ""
        for index, token in enumerate(tokens[:-1]):
            if token == "-o":
                output = tokens[index + 1]
        source_path, source_kind = rel_linux(entry.get("file", ""), directory, linux, build, arch)
        result.append({
            "source_path": source_path,
            "source_kind": source_kind,
            "object_path": normalize_path(output) if output else "",
            "architecture": arch,
            "compile_input": entry.get("file", ""),
            "compile_command": command,
            "directory": str(directory),
        })
    return result


def module_targets(build: Path) -> set[str]:
    order = build / "modules.order"
    if not order.exists():
        return set()
    result = set()
    for line in order.read_text(errors="replace").splitlines():
        target = normalize_path(line.strip())
        if not target:
            continue
        if target.endswith(".ko"):
            target = str(Path(target).with_suffix(".o"))
        result.add(target)
    return result


def composite_members(build: Path) -> dict[str, set[str]]:
    """Return component object -> composite Kbuild object owners from ``*.mod``."""
    owners: dict[str, set[str]] = defaultdict(set)
    for mod_file in sorted(build.rglob("*.mod")):
        owner = normalize_path(mod_file.relative_to(build).with_suffix(".o").as_posix())
        for token in mod_file.read_text(errors="replace").split():
            if token.endswith(".o"):
                owners[normalize_path(token)].add(owner)
    return owners


def archive_members(build: Path) -> tuple[dict[str, set[str]], dict[str, str]]:
    """Read all retained Kbuild archive membership records.

    Kbuild uses printf/xargs for built-in archives and direct ``llvm-ar`` calls
    for static architecture libraries and ``built-in-fixup.a``. Both forms are
    required to resolve a selected object all the way to ``vmlinux.a``.
    """

    owners: dict[str, set[str]] = defaultdict(set)
    evidence: dict[str, str] = {}
    printf_pattern = re.compile(r"printf\s+([\"'])([^\"']*%s[^\"']*)\1\s+(.*?)\s*\|\s*xargs", re.S)
    direct_pattern = re.compile(r"(?:^|;)\s*\S*llvm-ar\s+(\S+)\s+(\S+\.a)\s+([^;]+)", re.S)
    cat_pattern = re.compile(r"(?:^|;|:=)\s*cat\s+(\S+\.a)\s*>\s*(\S+\.a)")
    for command_file in sorted(build.rglob(".*.a.cmd")):
        content = command_file.read_text(errors="replace").replace("\\\n", " ")
        command_evidence = command_file.relative_to(build).as_posix()
        for match in printf_pattern.finditer(content):
            template, arguments = match.group(2), match.group(3)
            prefix, suffix = template.split("%s", 1)
            output_match = re.search(r"llvm-ar\s+\S+\s+(\S+\.a)", content[match.end():])
            if output_match is None:
                raise ValueError(f"cannot locate printf archive output in {command_file}")
            archive = normalize_path(output_match.group(1))
            evidence[archive] = command_evidence
            try:
                tokens = shlex.split(arguments)
            except ValueError as exc:
                raise ValueError(f"cannot parse {command_file}: {exc}") from exc
            for token in tokens:
                if token.endswith((".o", ".a")):
                    owners[normalize_path((prefix + token + suffix).strip())].add(archive)
        for match in direct_pattern.finditer(content):
            flags, archive_value, members = match.groups()
            if "c" not in flags:
                continue
            archive = normalize_path(archive_value)
            evidence[archive] = command_evidence
            for token in shlex.split(members, comments=False, posix=True):
                if token.endswith((".o", ".a")):
                    owners[normalize_path(token)].add(archive)
        for match in cat_pattern.finditer(content):
            member, archive = (normalize_path(value) for value in match.groups())
            owners[member].add(archive)
            evidence[archive] = command_evidence
    return owners, evidence


def kbuild_ownership(build: Path) -> dict[str, tuple[str, str, str]]:
    """Resolve each object to disposition, owning Kbuild target, and evidence."""
    modules = module_targets(build)
    composites = composite_members(build)
    archives, archive_evidence = archive_members(build)
    cache: dict[str, tuple[str, str, str]] = {}

    def resolve(object_path: str, trail: tuple[str, ...] = ()) -> tuple[str, str, str]:
        object_path = normalize_path(object_path)
        if object_path in cache:
            return cache[object_path]
        if object_path in trail:
            raise ValueError(f"cyclic Kbuild ownership: {' -> '.join((*trail, object_path))}")
        if object_path == "vmlinux.a":
            result = ("built-in", object_path, ".vmlinux.a.cmd")
        elif object_path in modules and object_path in archives:
            raise ValueError(f"object is both module and built-in: {object_path}")
        elif object_path in modules:
            result = ("module", object_path, "modules.order")
        elif object_path in composites:
            choices = sorted(composites[object_path])
            resolved = [resolve(choice, (*trail, object_path)) for choice in choices]
            unique = {(mode, owner) for mode, owner, _ in resolved}
            if len(unique) != 1:
                raise ValueError(f"contradictory composite ownership for {object_path}: {resolved}")
            mode, owner, parent_evidence = resolved[0]
            composite_evidence = f"{Path(choices[0]).with_suffix('.mod').as_posix()};{parent_evidence}"
            result = (mode, owner, composite_evidence)
        elif object_path in archives:
            choices = sorted(archives[object_path])
            resolved = [resolve(choice, (*trail, object_path)) for choice in choices]
            unique = {(mode, owner) for mode, owner, _ in resolved}
            if len(unique) != 1:
                raise ValueError(f"contradictory archive ownership for {object_path}: {resolved}")
            mode, owner, parent_evidence = resolved[0]
            result = (mode, owner, f"{archive_evidence[choices[0]]};{parent_evidence}")
        else:
            result = ("metadata", object_path, "compile_commands.json;ownership-unresolved")
        cache[object_path] = result
        return result

    candidates = set(modules) | set(composites) | set(archives)
    for candidate in sorted(candidates):
        resolve(candidate)
    return cache


def header_only_compilation_unit(path: str, linux_root: Path) -> bool:
    """Whether a selected C input emits no code beyond header inclusion.

    A direct quoted C include remains implementation-bearing because its body
    is part of this translation unit.  A C file containing only ordinary
    header inclusion/comments (such as ``lib/debug_info.c``) exists solely for
    generated debug metadata and is classified as build metadata instead of a
    zero-symbol Rust task.
    """

    source = linux_root / path
    if source.suffix != ".c" or not source.is_file():
        return False
    text = source.read_text(errors="replace")
    if INCLUDED_C_RE.search(text):
        return False
    masked = mask_c(text)
    lines = masked.splitlines()
    in_directive = False
    for line in lines:
        stripped = line.strip()
        if in_directive:
            in_directive = line.rstrip().endswith("\\")
            continue
        if stripped.startswith("#"):
            in_directive = line.rstrip().endswith("\\")
            continue
        if stripped:
            return False
    return True


def oracle_path_rule(path: str) -> tuple[str, bool]:
    """Return the mechanical oracle rule and whether it overrides ownership.

    Explicit KUnit and test-directory structure is authoritative even below a
    driver directory.  Generic test/selftest basename tokens are intentionally
    weaker: driver-owned diagnostic implementations remain Linux objects.
    Substrings such as ``testmgr``, ``memtest``, ``testmode``, and
    ``cabletest`` do not form test tokens.
    """

    parts = Path(path).parts
    lowered_parts = tuple(part.lower() for part in parts)
    if path.startswith("tools/testing/"):
        return "kselftest_path", True
    if path.startswith(("include/kunit/", "lib/kunit/")) or "kunit" in lowered_parts:
        return "kunit_framework_or_suite_path", True
    for component in ("selftests", "tests", "testing", "test"):
        if component in lowered_parts[:-1]:
            return f"in_tree_test_directory:{component}", True
    stem = Path(path).stem.lower()
    tokens = tuple(token for token in re.split(r"[-_.]+", stem) if token)
    if "kunit" in tokens:
        return "kunit_named_source", True
    if "selftest" in tokens or "selftests" in tokens:
        return "selftest_named_source", False
    if "test" in tokens:
        return "test_named_source", False
    return "", False


def driver_owned_source(path: str, owners: Iterable[str]) -> bool:
    owner_paths = tuple(owners)
    return path.startswith(("drivers/", "sound/")) or any(
        owner.startswith(("drivers/", "sound/")) for owner in owner_paths
    )


def source_class(
    path: str, kind: str, owners: Iterable[str], linux_root: Path,
) -> tuple[str, str]:
    if kind != "linux":
        return "BUILD_METADATA", ""
    owner_paths = tuple(owners)
    oracle_reason, ownership_override = oracle_path_rule(path)
    if oracle_reason and ownership_override:
        return "ORACLE_ONLY", oracle_reason
    if driver_owned_source(path, owner_paths):
        return "LINUX_DRIVER_OBJECT", ""
    if oracle_reason:
        return "ORACLE_ONLY", oracle_reason
    suffix = Path(path).suffix
    lowered = suffix.lower()
    if lowered in {".s", ".asm"} and path.startswith("arch/"):
        return "LINUX_ARCH_ASM", ""
    if lowered in {".s", ".asm"}:
        return "LINUX_DRIVER_OBJECT", ""
    if lowered not in {".c", ".h", ".cc", ".cpp"}:
        return "BUILD_METADATA", ""
    if header_only_compilation_unit(path, linux_root):
        return "BUILD_METADATA", ""
    return "RUST_TRANSLATE", ""


def destination(path: str, source_classification: str) -> str:
    return "src/" + str(Path(path).with_suffix(".rs")) if source_classification == "RUST_TRANSLATE" else ""


def header_class(
    path: str, kind: str, consumer_classes: set[str],
) -> tuple[str, str]:
    """Classify a selected header from the Kbuild ownership of its consumers."""

    if kind != "linux":
        return "BUILD_METADATA", ""
    oracle_reason, ownership_override = oracle_path_rule(path)
    if oracle_reason and ownership_override:
        return "ORACLE_ONLY", oracle_reason
    if oracle_reason and (
        "RUST_TRANSLATE" in consumer_classes or "ORACLE_ONLY" in consumer_classes
    ):
        return "ORACLE_ONLY", oracle_reason
    if consumer_classes == {"ORACLE_ONLY"}:
        return "ORACLE_ONLY", "oracle_consumer_closure"
    if "RUST_TRANSLATE" in consumer_classes:
        return "RUST_TRANSLATE", ""
    return "REFERENCE_ONLY", ""


def assign_destinations(rows: list[dict[str, str]]) -> None:
    """Assign unique path-preserving Rust destinations.

    A C file and header can share a basename.  Rust cannot place both at the
    same path, so only colliding non-C inputs receive an extension-qualified
    suffix; every mapping remains deterministic and recorded in SCOPE.tsv.
    """

    rust_rows = [row for row in rows if row["class"] == "RUST_TRANSLATE"]
    preferred = {".c": 0, ".cc": 1, ".cpp": 2, ".h": 3}
    by_candidate: dict[str, list[dict[str, str]]] = defaultdict(list)
    for row in rust_rows:
        by_candidate[destination(row["linux_path"], row["class"])].append(row)
    occupied: set[str] = set()
    for candidate in sorted(by_candidate):
        group = sorted(
            by_candidate[candidate],
            key=lambda row: (preferred.get(Path(row["linux_path"]).suffix.lower(), 9), row["linux_path"]),
        )
        for index, row in enumerate(group):
            selected = candidate
            if index:
                path = Path(candidate)
                suffix = Path(row["linux_path"]).suffix.lower().lstrip(".") or "source"
                selected = (path.parent / f"{path.stem}_{suffix}.rs").as_posix()
            if selected in occupied:
                path = Path(selected)
                discriminator = hashlib.sha256(row["linux_path"].encode()).hexdigest()[:8]
                selected = (path.parent / f"{path.stem}_{discriminator}.rs").as_posix()
            if selected in occupied:
                raise ValueError(f"cannot assign unique destination for {row['linux_path']}")
            row["destination_path"] = selected
            occupied.add(selected)


def strongly_connected_components(graph: dict[str, set[str]]) -> list[list[str]]:
    """Return deterministic SCCs without depending on Python recursion depth."""

    reverse: dict[str, set[str]] = {node: set() for node in graph}
    for node, dependencies in graph.items():
        for dependency in dependencies:
            reverse[dependency].add(node)
    visited: set[str] = set()
    finish_order: list[str] = []
    for root in sorted(graph):
        if root in visited:
            continue
        visited.add(root)
        stack: list[tuple[str, bool]] = [(root, False)]
        while stack:
            node, expanded = stack.pop()
            if expanded:
                finish_order.append(node)
                continue
            stack.append((node, True))
            for dependency in sorted(graph[node], reverse=True):
                if dependency not in visited:
                    visited.add(dependency)
                    stack.append((dependency, False))
    components: list[list[str]] = []
    assigned: set[str] = set()
    for root in reversed(finish_order):
        if root in assigned:
            continue
        component: list[str] = []
        assigned.add(root)
        pending = [root]
        while pending:
            node = pending.pop()
            component.append(node)
            for dependency in sorted(reverse[node], reverse=True):
                if dependency not in assigned:
                    assigned.add(dependency)
                    pending.append(dependency)
        components.append(sorted(component))
    return sorted(components, key=lambda component: component[0])


def reachable(graph: dict[str, set[str]], start: str) -> set[str]:
    """Return graph reachability without assigning semantic meaning to edges."""

    result: set[str] = set()
    pending = list(graph.get(start, set()))
    while pending:
        node = pending.pop()
        if node in result:
            continue
        result.add(node)
        pending.extend(graph.get(node, set()) - result)
    return result


def header_reference_identifiers(text: str, definition_names: set[str]) -> set[str]:
    """Return lexical references that can be tied to a selected header definition.

    This deliberately does not claim C name resolution.  It limits the provider
    graph to identifiers that Phase 0 independently inventoried as definitions,
    masks comments/literals/include operands, and excludes member selectors.
    """

    def preserve_newlines(match: re.Match[str]) -> str:
        return "\n" * match.group(0).count("\n")

    masked = re.sub(r"/\*.*?\*/", preserve_newlines, text, flags=re.S)
    masked = re.sub(r"//[^\n]*", "", masked)
    masked = re.sub(r'"(?:\\.|[^"\\])*"', '""', masked)
    masked = re.sub(r"'(?:\\.|[^'\\])*'", "''", masked)
    masked = re.sub(r"^\s*#\s*include\b[^\n]*", "", masked, flags=re.M)
    result: set[str] = set()
    for match in IDENTIFIER.finditer(masked):
        name = match.group(0)
        prefix = masked[max(0, match.start() - 8):match.start()]
        if re.search(r"(?:\.|->)\s*$", prefix):
            continue
        tag = re.search(r"\b(struct|union|enum)\s*$", prefix)
        candidates = (
            [f"{tag.group(1)}:{name}"]
            if tag else [f"identifier:{name}", f"macro:{name}"]
        )
        result.update(candidate for candidate in candidates if candidate in definition_names)
    return result


def forced_include_headers(
    command: str, directory: str, linux: Path, build: Path, arch: str,
) -> set[str]:
    """Resolve the retained command's explicit ``-include`` inputs."""

    tokens = shlex.split(command)
    base = Path(directory)
    result: set[str] = set()
    index = 1
    while index < len(tokens):
        token = tokens[index]
        value = ""
        if token == "-include" and index + 1 < len(tokens):
            value = tokens[index + 1]
            index += 2
        elif token.startswith("-include") and token != "-include":
            value = token[len("-include"):]
            index += 1
        else:
            index += 1
        if not value:
            continue
        candidate = Path(value)
        candidate = candidate if candidate.is_absolute() else base / candidate
        normalized = canonical_dependency(str(candidate), linux, build, arch)
        if normalized is not None:
            result.add(normalized[0])
    return result


def weight(path: str, linux: Path) -> float:
    try:
        lines = (linux / path).read_text(errors="replace").count("\n")
    except OSError:
        lines = 100
    return round(max(1.0, lines / 10.0), 1)


def risk(path: str) -> str:
    if path.startswith(("kernel/", "mm/", "arch/")):
        return "high"
    if path.startswith(("fs/", "net/", "block/", "security/")):
        return "medium"
    return "low"


def parse_config(path: Path, arch: str) -> dict[str, object]:
    macros: dict[str, object] = {
        "__KERNEL__": 1,
        "__SIZEOF_LONG__": 8,
        "__LP64__": 1,
        "__x86_64__": 1 if arch == "x86_64" else 0,
        "__aarch64__": 1 if arch == "aarch64" else 0,
        # Pinned `include/vdso/time64.h` supplies these unconditionally to
        # kernel translation units; they are material to selected #if branches.
        "MSEC_PER_SEC": 1000,
        "USEC_PER_MSEC": 1000,
        "USEC_PER_SEC": 1_000_000,
        "NSEC_PER_USEC": 1000,
        "NSEC_PER_MSEC": 1_000_000,
        "NSEC_PER_SEC": 1_000_000_000,
        # Pinned `include/uapi/asm-generic/param.h` defines USER_HZ as 100.
        "USER_HZ": 100,
    }
    for raw in path.read_text(encoding="utf-8").splitlines():
        unset = re.match(r"# (CONFIG_[A-Za-z0-9_]+) is not set$", raw)
        if unset:
            macros.pop(unset.group(1), None)
            macros.pop(unset.group(1) + "_MODULE", None)
            continue
        match = re.match(r"(CONFIG_[A-Za-z0-9_]+)=(.*)$", raw)
        if not match:
            continue
        name, value = match.groups()
        if value == "y":
            macros[name] = 1
        elif value == "m":
            macros[name + "_MODULE"] = 1
        elif value.startswith('"'):
            macros[name] = value
        else:
            try:
                macros[name] = int(re.sub(r"[uUlL]+$", "", value), 0)
            except ValueError:
                macros[name] = value
    # `include/asm-generic/param.h` defines the kernel HZ value from the
    # mechanically frozen CONFIG_HZ setting for both approved architectures.
    if isinstance(macros.get("CONFIG_HZ"), int):
        macros["HZ"] = macros["CONFIG_HZ"]
    return macros


def compile_defines(command: str) -> dict[str, object]:
    result: dict[str, object] = {}
    tokens = shlex.split(command)
    index = 0
    while index < len(tokens):
        token = tokens[index]
        definition = ""
        if token == "-D" and index + 1 < len(tokens):
            index += 1
            definition = tokens[index]
        elif token.startswith("-D"):
            definition = token[2:]
        if definition:
            name, separator, value = definition.partition("=")
            if IDENTIFIER.fullmatch(name):
                if not separator:
                    result[name] = 1
                else:
                    try:
                        result[name] = int(re.sub(r"[uUlL]+$", "", value), 0)
                    except ValueError:
                        result[name] = value
        index += 1
    return result


def substitute_compiler_predicates(
    expression: str, arch: str, predicates: dict[str, dict[tuple[str, str], int]]
) -> str:
    """Replace direct compiler builtin calls using frozen probe evidence."""

    pattern = re.compile(
        r"\b(" + "|".join(re.escape(kind) for kind in sorted(PREDICATE_KINDS)) + r")\s*\(\s*([^()]+?)\s*\)"
    )

    def replace(match: re.Match[str]) -> str:
        key = (match.group(1), re.sub(r"\s+", " ", match.group(2).strip()))
        try:
            return str(predicates[arch][key])
        except KeyError as exc:
            raise ValueError(f"missing frozen compiler predicate for {arch}: {key[0]}({key[1]})") from exc

    substituted = pattern.sub(replace, expression)
    if any(re.search(rf"\b{re.escape(kind)}\s*\(", substituted) for kind in PREDICATE_KINDS):
        raise ValueError(f"unsupported nested compiler predicate expression: {expression!r}")
    return substituted


def safe_expression_value(
    expression: str, macros: dict[str, object], arch: str, predicates: dict[str, dict[tuple[str, str], int]]
) -> bool:
    expression = substitute_compiler_predicates(expression, arch, predicates)
    expression = re.sub(
        r"\bdefined\s*(?:\(\s*([A-Za-z_]\w*)\s*\)|([A-Za-z_]\w*))",
        lambda match: "1" if (match.group(1) or match.group(2)) in macros else "0",
        expression,
    )

    def enabled(match: re.Match[str]) -> str:
        function, name = match.groups()
        builtin = name in macros
        module = name + "_MODULE" in macros
        value = builtin or module if function == "IS_ENABLED" else builtin if function == "IS_BUILTIN" else module
        return "1" if value else "0"

    expression = re.sub(r"\b(IS_ENABLED|IS_BUILTIN|IS_MODULE)\s*\(\s*([A-Za-z_]\w*)\s*\)", enabled, expression)
    expression = re.sub(r"\b([0-9]+|0[xX][0-9A-Fa-f]+)[uUlL]+\b", r"\1", expression)
    expression = IDENTIFIER.sub(
        lambda match: str(macros.get(match.group(0), 0)) if isinstance(macros.get(match.group(0), 0), int) else "0",
        expression,
    )
    expression = expression.replace("&&", " and ").replace("||", " or ")
    expression = re.sub(r"!(?!=)", " not ", expression).strip()
    if "?" in expression:
        raise ValueError("ternary preprocessor expressions are not supported")
    tree = ast.parse(expression or "0", mode="eval")
    allowed = (
        ast.Expression, ast.BoolOp, ast.BinOp, ast.UnaryOp, ast.Compare, ast.Constant,
        ast.And, ast.Or, ast.Not, ast.Invert, ast.UAdd, ast.USub, ast.Add, ast.Sub,
        ast.Mult, ast.Div, ast.FloorDiv, ast.Mod, ast.LShift, ast.RShift, ast.BitAnd,
        ast.BitOr, ast.BitXor, ast.Eq, ast.NotEq, ast.Lt, ast.LtE, ast.Gt, ast.GtE,
    )
    if any(not isinstance(node, allowed) for node in ast.walk(tree)):
        raise ValueError(f"unsupported preprocessor expression: {expression!r}")
    return bool(eval(compile(tree, "<phase0-condition>", "eval"), {"__builtins__": {}}, {}))


def logical_directives(lines: list[str]) -> dict[int, tuple[str, int]]:
    result: dict[int, tuple[str, int]] = {}
    index = 0
    while index < len(lines):
        if not re.match(r"^\s*#", lines[index]):
            index += 1
            continue
        start = index
        parts = [lines[index].rstrip("\n")]
        while parts[-1].rstrip().endswith("\\") and index + 1 < len(lines):
            parts[-1] = parts[-1].rstrip()[:-1]
            index += 1
            parts.append(lines[index].rstrip("\n"))
        result[start + 1] = (" ".join(parts), index + 1)
        index += 1
    return result


def selected_lines(
    text: str,
    arch: str,
    config_path: Path,
    compile_command: str,
    predicates: dict[str, dict[tuple[str, str], int]],
) -> tuple[set[int], dict[int, str], list[dict[str, str]], list[dict[str, str]]]:
    lines = text.splitlines(keepends=True)
    directives = logical_directives(lines)
    macros = parse_config(config_path, arch)
    macros.update(compile_defines(compile_command))
    active_lines: set[int] = set()
    selection: dict[int, str] = {}
    conditional_rows: list[dict[str, str]] = []
    macro_rows: list[dict[str, str]] = []
    stack: list[dict[str, object]] = []
    active = True
    line_number = 1
    while line_number <= len(lines):
        directive = directives.get(line_number)
        if directive is None:
            if active:
                active_lines.add(line_number)
                expressions = [str(frame["selected_expression"]) for frame in stack if frame["this_active"]]
                selection[line_number] = " && ".join(expressions) if expressions else "1"
            line_number += 1
            continue
        logical, end_line = directive
        conditional = CONDITIONAL_RE.match(logical)
        if conditional:
            kind = conditional.group(1)
            # Directives commonly carry explanatory C comments (for example,
            # ``#ifndef CONFIG_ZISOFS /* No flag ... */``).  They are not part
            # of the preprocessor condition and must not reach the restricted
            # expression evaluator below.
            argument = re.sub(r"/\\*.*?\\*/", "", conditional.group(2), flags=re.S)
            argument = argument.split("//", 1)[0].strip()
            if kind in {"if", "ifdef", "ifndef"}:
                parent_active = active
                if kind in {"ifdef", "ifndef"}:
                    identifier = IDENTIFIER.match(argument)
                    if identifier is None:
                        raise ValueError(
                            f"{arch}:{config_path}:{line_number}: invalid #{kind} operand {argument!r}"
                        )
                    argument = identifier.group(0)
                expression = argument if kind == "if" else f"defined({argument})"
                if kind == "ifndef":
                    expression = f"!defined({argument})"
                try:
                    value = safe_expression_value(expression, macros, arch, predicates)
                except (SyntaxError, ValueError, ZeroDivisionError) as exc:
                    raise ValueError(f"{arch}:{config_path}:{line_number}: {logical}: {exc}") from exc
                this_active = parent_active and value
                stack.append({
                    "parent_active": parent_active,
                    "any_taken": this_active,
                    "this_active": this_active,
                    "selected_expression": expression,
                })
                active = this_active
                selected = this_active
            elif kind == "elif":
                if not stack:
                    raise ValueError(f"unmatched #elif at line {line_number}")
                frame = stack[-1]
                value = safe_expression_value(argument, macros, arch, predicates)
                this_active = bool(frame["parent_active"]) and not bool(frame["any_taken"]) and value
                frame["this_active"] = this_active
                frame["any_taken"] = bool(frame["any_taken"]) or this_active
                frame["selected_expression"] = argument
                active = this_active
                expression = argument
                selected = this_active
            elif kind == "else":
                if not stack:
                    raise ValueError(f"unmatched #else at line {line_number}")
                frame = stack[-1]
                this_active = bool(frame["parent_active"]) and not bool(frame["any_taken"])
                frame["this_active"] = this_active
                frame["any_taken"] = True
                frame["selected_expression"] = "else"
                active = this_active
                expression = "else"
                selected = this_active
            else:
                if not stack:
                    raise ValueError(f"unmatched #endif at line {line_number}")
                closed = stack.pop()
                expression = str(closed["selected_expression"])
                selected = bool(closed["this_active"])
                active = bool(stack[-1]["this_active"]) if stack else True
            conditional_rows.append({
                "record_kind": "conditional",
                "symbol_name": f"{kind}@{line_number}",
                "source_line": str(line_number),
                "selection_expression": expression,
                "linkage": "NOT_APPLICABLE",
                "declaration": logical.strip(),
                "status": "COMPLETE",
                "selected": "YES" if selected else "NO",
            })
        else:
            define = DEFINE_RE.match(logical)
            if define and active:
                # Directive lines are excluded from normal entity parsing, but
                # keeping their active location lets the macro-template
                # inventory distinguish a selected definition from one hidden
                # behind an inactive configuration branch.
                active_lines.add(line_number)
                selection[line_number] = " && ".join(
                    str(frame["selected_expression"]) for frame in stack
                ) or "1"
                name, parameters, value = define.groups()
                macro_rows.append({
                    "record_kind": "operative_macro",
                    "symbol_name": name,
                    "source_line": str(line_number),
                    "selection_expression": " && ".join(str(frame["selected_expression"]) for frame in stack) or "1",
                    "linkage": "NOT_APPLICABLE",
                    "declaration": logical.strip(),
                    "status": "COMPLETE",
                })
                if not parameters:
                    stripped = value.strip()
                    try:
                        macros[name] = int(re.sub(r"[uUlL]+$", "", stripped), 0) if stripped else 1
                    except ValueError:
                        macros[name] = stripped or 1
            undef = UNDEF_RE.match(logical)
            if undef and active:
                macros.pop(undef.group(1), None)
        line_number = end_line + 1
    if stack:
        raise ValueError(f"unterminated preprocessor conditional in {config_path}")
    return active_lines, selection, conditional_rows, macro_rows


def mask_c(text: str) -> str:
    """Mask comments and literals while preserving character and line offsets."""
    output = list(text)
    index = 0
    state = "code"
    while index < len(text):
        char = text[index]
        nxt = text[index + 1] if index + 1 < len(text) else ""
        if state == "code" and char == "/" and nxt == "/":
            output[index] = output[index + 1] = " "
            index += 2
            state = "line_comment"
            continue
        if state == "code" and char == "/" and nxt == "*":
            output[index] = output[index + 1] = " "
            index += 2
            state = "block_comment"
            continue
        if state == "code" and char in {'"', "'"}:
            output[index] = " "
            state = "string" if char == '"' else "char"
            index += 1
            continue
        if state == "line_comment":
            if char == "\n":
                state = "code"
            else:
                output[index] = " "
            index += 1
            continue
        if state == "block_comment":
            if char == "*" and nxt == "/":
                output[index] = output[index + 1] = " "
                index += 2
                state = "code"
            else:
                if char != "\n":
                    output[index] = " "
                index += 1
            continue
        if state in {"string", "char"}:
            if char == "\\" and nxt:
                output[index] = " "
                if nxt != "\n":
                    output[index + 1] = " "
                index += 2
                continue
            closing = '"' if state == "string" else "'"
            if char == closing:
                output[index] = " "
                state = "code"
            elif char != "\n":
                output[index] = " "
            index += 1
            continue
        index += 1
    return "".join(output)


def normalize_declaration(value: str) -> str:
    return re.sub(r"\s+", " ", value).strip()


def function_identity(prefix: str) -> tuple[str, str] | None:
    compact = normalize_declaration(prefix)
    macro = FUNCTION_MACRO_RE.search(compact)
    if macro and "=" not in compact[:macro.start()]:
        return f"{macro.group(1)}:{macro.group(2)}", "function_macro"
    # `struct foo {` is a declaration, but `struct foo *function(...) {`
    # is a normal function definition.  Do not discard the latter merely
    # because the return type begins with a tag keyword.
    if "=" in compact or (
        compact.startswith(("struct ", "union ", "enum ", "typedef ")) and "(" not in compact
    ):
        return None
    ignored = {"if", "for", "while", "switch", "sizeof", "typeof", "__attribute__", "__section"}
    depth = 0
    for index, char in enumerate(compact):
        if char == "(" and depth == 0:
            before = compact[:index].rstrip()
            match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)$", before)
            if match and match.group(1) not in ignored:
                return match.group(1), "function"
            depth += 1
        elif char == "(":
            depth += 1
        elif char == ")" and depth:
            depth -= 1
    return None


def variable_identity(declaration: str) -> str | None:
    compact = normalize_declaration(declaration)
    pointer = re.search(r"\(\s*\*\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)", compact)
    if pointer:
        return pointer.group(1)
    before_initializer = compact.split("=", 1)[0]
    before_initializer = re.sub(r"\[[^\]]*\]\s*$", "", before_initializer).rstrip(" ;")
    identifiers = IDENTIFIER.findall(before_initializer)
    return identifiers[-1] if identifiers else None


def typedef_identity(declaration: str) -> str | None:
    """Return the alias declared by a complete ``typedef`` statement.

    Function and function-pointer typedefs do not put their alias at the end
    of the declaration, so they need their own mechanical forms before the
    normal terminal-name case.
    """

    masked = normalize_declaration(declaration)
    function_pointer = TYPEDEF_FUNCTION_POINTER_RE.search(masked)
    if function_pointer:
        return function_pointer.group(1)
    function = TYPEDEF_FUNCTION_RE.search(masked)
    if function:
        return function.group(1)
    terminal = TYPEDEF_NAME_RE.search(masked)
    return terminal.group(1) if terminal else None


def translation_source_units(
    source: Path, linux_root: Path, generated_root: Path
) -> list[tuple[Path, str, str]]:
    """Return a selected translation unit plus its direct quoted ``.c`` inputs.

    Linux uses C-file inclusion for a few configuration-specific compilation
    units.  Such an include is part of the exact selected translation unit;
    treating the wrapper as an empty source would omit all of its operative
    symbols.  Only quoted local ``.c`` inclusions reachable from the selected
    input are followed, recursively, and every target must remain inside the
    pinned Linux tree.
    """

    units: list[tuple[Path, str, str]] = []
    seen: set[Path] = set()

    def source_label(path: Path) -> str:
        if path.is_relative_to(linux_root.resolve()):
            return f"vendor/linux/{path.relative_to(linux_root.resolve()).as_posix()}"
        return f"generated/{path.relative_to(generated_root.resolve()).as_posix()}"

    def visit(path: Path) -> None:
        resolved = path.resolve()
        if resolved in seen:
            return
        inside_linux = resolved.is_relative_to(linux_root.resolve())
        inside_generated = resolved.is_relative_to(generated_root.resolve())
        if not resolved.is_file() or not (inside_linux or inside_generated):
            raise ValueError(f"selected C include is absent from frozen source/generated inputs: {path}")
        seen.add(resolved)
        text = resolved.read_text(errors="replace")
        units.append((resolved, text, source_label(resolved)))
        for include in INCLUDED_C_RE.findall(text):
            target = (resolved.parent / include).resolve()
            if not target.is_file() and resolved.is_relative_to(linux_root.resolve()):
                target = (generated_root / resolved.relative_to(linux_root.resolve()).parent / include).resolve()
            visit(target)

    visit(source)
    return units


def macro_template_entities(
    text: str, active_lines: set[int], selection: dict[int, str]
) -> list[dict[str, str]]:
    """Mechanically materialize selected C macro templates that define symbols.

    Several Linux implementation units define functions or global dispatch
    records in a multi-line ``#define`` and instantiate that template with a
    small, literal argument list.  The compiler sees the instantiated symbols,
    so Phase 0 records them directly instead of treating the file as a
    macro-only shell.  This recognises only templates whose bodies visibly
    declare a static function or a struct global and only simple literal
    invocation arguments; unsupported forms remain visible to the extractor
    as absent symbols rather than being guessed.
    """

    function_template = re.compile(
        r"\bstatic\b\s+(?:[A-Za-z_][A-Za-z0-9_]*\s+|\*+\s*){1,8}"
        r"([A-Za-z_][A-Za-z0-9_]*(?:\s*##\s*[A-Za-z_][A-Za-z0-9_]*)*)\s*\("
    )
    global_template = re.compile(
        r"\b(?:static\s+)?(?:const\s+)?struct\s+[A-Za-z_][A-Za-z0-9_]*\s+"
        r"((?:[A-Za-z_][A-Za-z0-9_]*|\s*##\s*)+)\s*=\s*\{"
    )
    lines = text.splitlines(keepends=True)
    directives = logical_directives(lines)
    hidden = list(text)
    templates: dict[str, tuple[list[str], list[str], list[str]]] = {}
    for start_line, (logical, end_line) in directives.items():
        if start_line not in active_lines or "##" not in logical:
            continue
        # Do not reuse DEFINE_RE here: its permissive, multiline body is
        # appropriate for normal macro inventory but can backtrack heavily on
        # exceptionally large generated macro definitions.  Only the compact
        # macro header is needed to identify a token-pasting symbol template.
        header = re.match(
            r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)\n]*)\)",
            logical,
        )
        if header is None:
            continue
        name, parameter_text = header.groups()
        body = logical[header.end():]
        # A template without token pasting cannot manufacture a distinct
        # symbol name per invocation.  It is already represented by its
        # operative macro record, so only retain the mechanically necessary
        # token-pasting form here.
        if not parameter_text or ("static" not in body and "struct" not in body):
            continue
        params = IDENTIFIER.findall(parameter_text)
        function_names = function_template.findall(body)
        global_names = global_template.findall(body)
        if not params or not (function_names or global_names):
            continue
        templates[name] = (params, function_names, global_names)
        start_offset = sum(len(line) for line in lines[: start_line - 1])
        end_offset = sum(len(line) for line in lines[:end_line])
        for index in range(start_offset, end_offset):
            if hidden[index] != "\n":
                hidden[index] = " "
    invocation_text = "".join(hidden)
    entities: list[dict[str, str]] = []

    def instantiate(template: str, params: list[str], arguments: list[str]) -> str | None:
        candidate = template
        for parameter, argument in zip(params, arguments):
            candidate = re.sub(rf"\b{re.escape(parameter)}\b", argument.strip(), candidate)
        candidate = re.sub(r"\s*##\s*", "", candidate)
        candidate = re.sub(r"\s+", "", candidate)
        return candidate if IDENTIFIER.fullmatch(candidate) else None

    # Index possible macro calls once.  Re-running one full-source regex for
    # each #define is quadratic in macro-heavy implementation files such as
    # zstd, while this pass remains linear in the selected source text.
    invocation_pattern = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(([^()]+)\)\s*;")
    for match in invocation_pattern.finditer(invocation_text):
        name = match.group(1)
        template_info = templates.get(name)
        if template_info is None:
            continue
        params, function_names, global_names = template_info
        line = invocation_text.count("\n", 0, match.start()) + 1
        if line not in active_lines:
            continue
        arguments = [item.strip() for item in match.group(2).split(",")]
        if len(arguments) != len(params) or any(not IDENTIFIER.fullmatch(item) and not re.fullmatch(r"[0-9]+", item) for item in arguments):
            continue
        declaration = normalize_declaration(text[match.start():match.end()])
        for template in function_names:
            symbol = instantiate(template, params, arguments)
            if symbol:
                for kind in ("function", "static"):
                    entities.append({
                        "record_kind": kind,
                        "symbol_name": symbol,
                        "source_line": str(line),
                        "selection_expression": selection.get(line, "1"),
                        "linkage": "internal",
                        "declaration": declaration,
                        "status": "COMPLETE",
                    })
        for template in global_names:
            symbol = instantiate(template, params, arguments)
            if symbol:
                entities.append({
                    "record_kind": "global",
                    "symbol_name": symbol,
                    "source_line": str(line),
                    "selection_expression": selection.get(line, "1"),
                    "linkage": "external",
                    "declaration": declaration,
                    "status": "COMPLETE",
                })
    return entities


def source_entities(text: str, active_lines: set[int], selection: dict[int, str]) -> list[dict[str, str]]:
    lines = text.splitlines(keepends=True)
    fully_masked = mask_c(text).splitlines(keepends=True)
    if len(fully_masked) != len(lines):
        raise ValueError("C masker did not preserve source line count")
    selected_text = "".join(
        line if number in active_lines and not re.match(r"^\s*#", line)
        else "".join("\n" if character == "\n" else " " for character in line)
        for number, line in enumerate(lines, 1)
    )
    # Mask the complete C stream before blanking selected-out directives.  A
    # directive can start a multi-line comment; masking after deleting that
    # first line would leave an orphan apostrophe or quote in the following
    # comment text and can hide every later declaration.
    masked = "".join(
        line if number in active_lines and not re.match(r"^\s*#", raw)
        else "".join("\n" if character == "\n" else " " for character in raw)
        for number, (raw, line) in enumerate(zip(lines, fully_masked), 1)
    )
    entities: list[dict[str, str]] = []
    depth = 0
    segment_start = 0
    function_depth: int | None = None
    index = 0
    while index < len(masked):
        char = masked[index]
        if char == "{" and depth == 0:
            prefix = masked[segment_start:index]
            identity = function_identity(prefix)
            if identity:
                name, kind = identity
                name_offset = masked.rfind(name.split(":")[-1], segment_start, index)
                line = masked.count("\n", 0, name_offset if name_offset >= 0 else segment_start) + 1
                declaration = normalize_declaration(selected_text[segment_start:index])
                linkage = "internal" if re.search(r"\bstatic\b", declaration) else "external"
                entities.append({
                    "record_kind": kind,
                    "symbol_name": name,
                    "source_line": str(line),
                    "selection_expression": selection.get(line, "1"),
                    "linkage": linkage,
                    "declaration": declaration,
                    "status": "COMPLETE",
                })
                if linkage == "internal":
                    entities.append({
                        "record_kind": "static",
                        "symbol_name": name,
                        "source_line": str(line),
                        "selection_expression": selection.get(line, "1"),
                        "linkage": linkage,
                        "declaration": declaration,
                        "status": "COMPLETE",
                    })
                function_depth = 1
            depth = 1
        elif char == "{" and depth > 0:
            depth += 1
            if function_depth is not None:
                function_depth += 1
        elif char == "}" and depth > 0:
            depth -= 1
            if function_depth is not None:
                function_depth -= 1
                if function_depth == 0:
                    function_depth = None
                    segment_start = index + 1
        elif char == ";" and depth == 0 and function_depth is None:
            declaration = selected_text[segment_start:index + 1]
            compact = normalize_declaration(masked[segment_start:index + 1])
            first_content = re.search(r"\S", masked[segment_start:index + 1])
            content_offset = segment_start + first_content.start() if first_content else segment_start
            line = masked.count("\n", 0, content_offset) + 1
            for type_kind, type_name in TYPE_TAG_RE.findall(compact):
                entities.append({
                    "record_kind": "type",
                    "symbol_name": f"{type_kind} {type_name}",
                    "source_line": str(line),
                    "selection_expression": selection.get(line, "1"),
                    "linkage": "NOT_APPLICABLE",
                    "declaration": normalize_declaration(declaration),
                    "status": "COMPLETE",
                })
            if re.search(r"\btypedef\b", compact):
                alias = TYPEDEF_NAME_RE.search(compact)
                if alias:
                    entities.append({
                        "record_kind": "type",
                        "symbol_name": alias.group(1),
                        "source_line": str(line),
                        "selection_expression": selection.get(line, "1"),
                        "linkage": "NOT_APPLICABLE",
                        "declaration": normalize_declaration(declaration),
                        "status": "COMPLETE",
                    })
            elif re.search(r"\bstatic\b", compact):
                name = variable_identity(compact)
                if name:
                    entities.append({
                        "record_kind": "static",
                        "symbol_name": name,
                        "source_line": str(line),
                        "selection_expression": selection.get(line, "1"),
                        "linkage": "internal",
                        "declaration": normalize_declaration(declaration),
                        "status": "COMPLETE",
                    })
            elif compact and (
                "=" in compact or "(" not in compact
            ) and not compact.startswith(("struct ", "union ", "enum ")):
                name = variable_identity(compact)
                if name:
                    entities.append({
                        "record_kind": "global",
                        "symbol_name": name,
                        "source_line": str(line),
                        "selection_expression": selection.get(line, "1"),
                        "linkage": "external",
                        "declaration": normalize_declaration(declaration),
                        "status": "COMPLETE",
                    })
            segment_start = index + 1
        index += 1
    for match in EXPORT_RE.finditer(masked):
        line = masked.count("\n", 0, match.start()) + 1
        entities.append({
            "record_kind": "export",
            "symbol_name": match.group(2),
            "source_line": str(line),
            "selection_expression": selection.get(line, "1"),
            "linkage": "external",
            "declaration": normalize_declaration(selected_text[match.start():match.end()]),
            "export_kind": "GPL" if match.group(1) else "NON_GPL",
            "status": "COMPLETE",
        })
    # A declaration scanner based on semicolon boundaries can miss a local
    # anonymous aggregate (for example `struct __packed { ... } value;`).
    # Record each actual selected type body directly, retaining its source line
    # and using a location-qualified name only when the C type is anonymous.
    for match in TYPE_BODY_RE.finditer(masked):
        line = masked.count("\n", 0, match.start()) + 1
        type_kind, type_name = match.groups()
        name = f"{type_kind} {type_name}" if type_name else f"anonymous_{type_kind}@{line}"
        line_text = selected_text.splitlines()[line - 1] if line <= len(selected_text.splitlines()) else ""
        entities.append({
            "record_kind": "type",
            "symbol_name": name,
            "source_line": str(line),
            "selection_expression": selection.get(line, "1"),
            "linkage": "NOT_APPLICABLE",
            "declaration": normalize_declaration(line_text),
            "status": "COMPLETE",
        })
    # A source-only phase must retain typedefs even when their declaration is
    # inside a selected function body.  The brace-depth parser above quite
    # deliberately skips function bodies while recognizing globals, whereas a
    # typedef's declared alias remains an ABI-relevant type fact regardless of
    # its lexical nesting.
    for match in TYPEDEF_RE.finditer(masked):
        line = masked.count("\n", 0, match.start()) + 1
        declaration = selected_text[match.start():match.end()]
        name = typedef_identity(declaration)
        if name:
            entities.append({
                "record_kind": "type",
                "symbol_name": name,
                "source_line": str(line),
                "selection_expression": selection.get(line, "1"),
                "linkage": "NOT_APPLICABLE",
                "declaration": normalize_declaration(declaration),
                "status": "COMPLETE",
            })
    # The lock-definition macros below expand to a selected static object.
    # Preserve that mechanically visible object rather than treating a
    # macro-only implementation file as empty.
    for match in STATIC_DEFINE_MACRO_RE.finditer(masked):
        line = masked.count("\n", 0, match.start()) + 1
        entities.append({
            "record_kind": "static",
            "symbol_name": match.group(1),
            "source_line": str(line),
            "selection_expression": selection.get(line, "1"),
            "linkage": "internal",
            "declaration": normalize_declaration(selected_text[match.start():match.end()]),
            "status": "COMPLETE",
        })
    if "##" in text and "#define" in text and ("static" in text or "struct" in text):
        entities.extend(macro_template_entities(text, active_lines, selection))
    unique: dict[tuple[str, str, str], dict[str, str]] = {}
    for row in entities:
        unique[(row["record_kind"], row["symbol_name"], row["source_line"])] = row
    return sorted(unique.values(), key=lambda row: (int(row["source_line"]), row["record_kind"], row["symbol_name"]))


def semantic_records(
    scope_id: str,
    linux_path: str,
    arch: str,
    source: Path,
    linux_root: Path,
    generated_root: Path,
    config_path: Path,
    compile_command: str,
    predicates: dict[str, dict[tuple[str, str], int]],
) -> tuple[list[dict[str, str]], list[dict[str, str]], list[dict[str, str]]]:
    config_evidence = f"rewrite/configs/{ARCH_CONFIG_NAMES[arch]}/frozen.config"
    symbols: list[dict[str, str]] = []
    abi: list[dict[str, str]] = []
    lifetimes: list[dict[str, str]] = []
    for unit, text, unit_label in translation_source_units(source, linux_root, generated_root):
        active, selection, conditions, macros = selected_lines(
            text, arch, config_path, compile_command, predicates
        )
        entities = source_entities(text, active, selection)
        base_evidence = unit_label
        if unit_label != f"vendor/linux/{linux_path}":
            base_evidence += f"; included_by=vendor/linux/{linux_path}"
        for record in [*conditions, *macros, *entities]:
            line = record["source_line"]
            selected = record.pop("selected", "YES")
            symbol = {
                "scope_id": scope_id,
                "linux_path": linux_path,
                "architectures": arch,
                **record,
                "config_evidence": config_evidence,
                "evidence": f"{base_evidence}:{line};{config_evidence};selected={selected}",
            }
            symbols.append(symbol)
            kind = record["record_kind"]
            if kind in {"function", "function_macro", "static", "global", "type", "export"}:
                known_layout = "NOT_APPLICABLE" if kind in {"function", "function_macro", "export"} else "PENDING_REVIEW"
                known_alignment = "NOT_APPLICABLE" if kind in {"function", "function_macro", "export"} else "PENDING_REVIEW"
                export_kind = record.get("export_kind", "NOT_EXPORTED" if kind != "export" else "PENDING_REVIEW")
                pending_abi = "PENDING_REVIEW" in {known_layout, known_alignment, export_kind}
                abi.append({
                    "scope_id": scope_id,
                    "linux_path": linux_path,
                    "architectures": arch,
                    "record_kind": kind,
                    "symbol_name": record["symbol_name"],
                    "source_line": line,
                    "abi_item": f"{kind}:{record['symbol_name']}",
                    "linkage": record.get("linkage", "NOT_APPLICABLE"),
                    "export_kind": export_kind,
                    "declaration": record.get("declaration", ""),
                    "layout": known_layout,
                    "alignment": known_alignment,
                    "calling_convention": "C_SOURCE" if kind in {"function", "function_macro", "export"} else "NOT_APPLICABLE",
                    "config_evidence": config_evidence,
                    "evidence": f"{base_evidence}:{line};{config_evidence}",
                    "status": "PENDING_REVIEW" if pending_abi else "COMPLETE",
                })
            if kind in {"function", "function_macro", "static", "global", "type"}:
                storage = "static" if kind in {"function", "function_macro", "static", "global"} else "NOT_APPLICABLE"
                lifetimes.append({
                    "scope_id": scope_id,
                    "linux_path": linux_path,
                    "architectures": arch,
                    "record_kind": kind,
                    "symbol_name": record["symbol_name"],
                    "source_line": line,
                    "lifetime_item": f"{kind}:{record['symbol_name']}",
                    "storage_duration": storage,
                    "ownership": "PENDING_REVIEW",
                    "lifetime_contract": "PENDING_REVIEW",
                    "locking_rcu_refcount": "PENDING_REVIEW",
                    "config_evidence": config_evidence,
                    "evidence": f"{base_evidence}:{line};{config_evidence}",
                    "status": "PENDING_REVIEW",
                })
    return symbols, abi, lifetimes


def contextual_header_semantic_records(
    scope_id: str,
    linux_path: str,
    arch: str,
    source: Path,
) -> tuple[list[dict[str, str]], list[dict[str, str]], list[dict[str, str]]]:
    """Inventory lexical header entities without guessing inclusion context.

    Kbuild proves that the header is in a selected compiler dependency closure,
    but a header's active branches can depend on macros established earlier by
    each including translation unit.  Phase 0 therefore records all lexical
    entities and directives and leaves contextual selection/semantic contracts
    for the per-file implement/review/apply pipeline.
    """

    text = source.read_text(errors="replace")
    lines = text.splitlines(keepends=True)
    directives = logical_directives(lines)
    active_lines = set(range(1, len(lines) + 1))
    selection = {line: "PENDING_REVIEW" for line in active_lines}
    directive_rows: list[dict[str, str]] = []
    macro_rows: list[dict[str, str]] = []
    for line, (logical, end_line) in directives.items():
        # Continuation lines are macro syntax rather than standalone C.
        active_lines.difference_update(range(line + 1, end_line + 1))
        conditional = CONDITIONAL_RE.match(logical)
        if conditional:
            kind = conditional.group(1)
            expression = re.sub(r"/\*.*?\*/", "", conditional.group(2), flags=re.S)
            expression = expression.split("//", 1)[0].strip() or kind
            directive_rows.append({
                "record_kind": "conditional",
                "symbol_name": f"{kind}@{line}",
                "source_line": str(line),
                "selection_expression": expression,
                "linkage": "NOT_APPLICABLE",
                "declaration": logical.strip(),
                "status": "PENDING_REVIEW",
                "selected": "PENDING_REVIEW",
            })
        define = DEFINE_RE.match(logical)
        if define:
            macro_rows.append({
                "record_kind": "operative_macro",
                "symbol_name": define.group(1),
                "source_line": str(line),
                "selection_expression": "PENDING_REVIEW",
                "linkage": "NOT_APPLICABLE",
                "declaration": logical.strip(),
                "status": "PENDING_REVIEW",
                "selected": "PENDING_REVIEW",
            })
    entities = source_entities(text, active_lines, selection)
    config_evidence = f"rewrite/configs/{ARCH_CONFIG_NAMES[arch]}/frozen.config"
    symbols: list[dict[str, str]] = []
    abi: list[dict[str, str]] = []
    lifetimes: list[dict[str, str]] = []
    for record in [*directive_rows, *macro_rows, *entities]:
        record = dict(record)
        line = record["source_line"]
        selected = record.pop("selected", "PENDING_REVIEW")
        record["status"] = "PENDING_REVIEW"
        symbols.append({
            "scope_id": scope_id,
            "linux_path": linux_path,
            "architectures": arch,
            **record,
            "config_evidence": config_evidence,
            "evidence": (
                f"vendor/linux/{linux_path}:{line};{config_evidence};"
                f"header_closure=rewrite/metadata/header_closure.tsv;selected={selected}"
            ),
        })
        kind = record["record_kind"]
        if kind in {"function", "function_macro", "static", "global", "type", "export"}:
            abi.append({
                "scope_id": scope_id,
                "linux_path": linux_path,
                "architectures": arch,
                "record_kind": kind,
                "symbol_name": record["symbol_name"],
                "source_line": line,
                "abi_item": f"{kind}:{record['symbol_name']}",
                "linkage": record.get("linkage", "NOT_APPLICABLE"),
                "export_kind": record.get("export_kind", "PENDING_REVIEW"),
                "declaration": record.get("declaration", ""),
                "layout": "NOT_APPLICABLE" if kind in {"function", "function_macro", "export"} else "PENDING_REVIEW",
                "alignment": "NOT_APPLICABLE" if kind in {"function", "function_macro", "export"} else "PENDING_REVIEW",
                "calling_convention": "C_SOURCE" if kind in {"function", "function_macro", "export"} else "NOT_APPLICABLE",
                "config_evidence": config_evidence,
                "evidence": f"vendor/linux/{linux_path}:{line};header-context=PENDING_REVIEW",
                "status": "PENDING_REVIEW",
            })
        if kind in {"function", "function_macro", "static", "global", "type"}:
            lifetimes.append({
                "scope_id": scope_id,
                "linux_path": linux_path,
                "architectures": arch,
                "record_kind": kind,
                "symbol_name": record["symbol_name"],
                "source_line": line,
                "lifetime_item": f"{kind}:{record['symbol_name']}",
                "storage_duration": "static" if kind != "type" else "NOT_APPLICABLE",
                "ownership": "PENDING_REVIEW",
                "lifetime_contract": "PENDING_REVIEW",
                "locking_rcu_refcount": "PENDING_REVIEW",
                "config_evidence": config_evidence,
                "evidence": f"vendor/linux/{linux_path}:{line};header-context=PENDING_REVIEW",
                "status": "PENDING_REVIEW",
            })
    return symbols, abi, lifetimes


def copy_metadata(
    build: Path,
    target: Path,
    arch: str,
    entries: list[dict[str, str]],
    ownership: dict[str, tuple[str, str, str]],
) -> None:
    target.mkdir(parents=True, exist_ok=True)
    for name in (
        "compile_commands.json", "modules.order", "modules.builtin",
        "modules.builtin.modinfo", "System.map", "vmlinux.symvers", ".config",
    ):
        source = build / name
        if source.exists():
            shutil.copy2(source, target / ("generated.config" if name == ".config" else name))
    log = build / "build.log"
    if log.exists():
        shutil.copy2(log, target / "build.log")
    cmd_rows: list[dict[str, str]] = []
    dep_rows: list[dict[str, str]] = []
    include_rows: list[dict[str, str]] = []
    for command_file in sorted(build.rglob("*.cmd")):
        rel = command_file.relative_to(build).as_posix()
        cmd_rows.append({"architecture": arch, "path": rel, "sha256": sha256(command_file)})
        content = command_file.read_text(errors="replace")
        for variable in re.findall(r"^(deps_[^\s:]+)\s*:=", content, flags=re.M):
            assignment = make_assignment(content, variable)
            if assignment is None:
                raise ValueError(f"cannot parse {variable} from {command_file}")
            include_rows.extend(
                {"architecture": arch, "depfile": rel, "dependency": dependency}
                for dependency in assignment.split()
                if dependency != "\\"
            )
    for dep_file in sorted(build.rglob("*.d")):
        rel = dep_file.relative_to(build).as_posix()
        dep_rows.append({"architecture": arch, "path": rel, "sha256": sha256(dep_file)})
        content = dep_file.read_text(errors="replace").replace("\\\n", " ")
        include_rows.extend(
            {"architecture": arch, "depfile": rel, "dependency": dependency}
            for dependency in content.split()[1:]
        )
    object_rows = []
    for item in entries:
        mode, owner, evidence = ownership.get(
            item["object_path"], ("metadata", item["object_path"], "compile_commands.json;ownership-unresolved")
        )
        object_rows.append({
            "architecture": arch,
            "source_path": item["source_path"],
            "object_path": item["object_path"],
            "module_or_builtin": mode,
            "kbuild_owner": owner,
            "disposition_evidence": evidence,
        })
    write_tsv(target / "cmd_inventory.tsv", ["architecture", "path", "sha256"], cmd_rows)
    write_tsv(target / "depfile_inventory.tsv", ["architecture", "path", "sha256"], dep_rows)
    write_tsv(target / "include_dependencies.tsv", ["architecture", "depfile", "dependency"], include_rows)
    write_tsv(
        target / "object_inventory.tsv",
        ["architecture", "source_path", "object_path", "module_or_builtin", "kbuild_owner", "disposition_evidence"],
        object_rows,
    )
    artifacts = [
        {"architecture": arch, "path": path.relative_to(build).as_posix(), "sha256": sha256(path)}
        for path in sorted(build.rglob("*")) if path.is_file()
    ]
    write_tsv(target / "all_artifacts.tsv", ["architecture", "path", "sha256"], artifacts)
    for directory_name in ("include/generated", f"arch/{'x86' if arch == 'x86_64' else 'arm64'}/include/generated"):
        directory = build / directory_name
        if directory.exists():
            archive = target / ("generated-headers-" + directory_name.replace("/", "-") + ".tar")
            with tarfile.open(archive, "w") as handle:
                handle.add(directory, arcname=directory_name)
    generated_files = [
        path for path in sorted(build.rglob("*"))
        if path.is_file() and ("generated" in path.parts or path.name in {"autoconf.h", "rustc_cfg"})
        and path.suffix in {".c", ".S", ".h", ".inc"}
    ]
    if generated_files:
        with tarfile.open(target / "generated-sources.tar", "w") as handle:
            for path in generated_files:
                handle.add(path, arcname=path.relative_to(build).as_posix())


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--linux", type=Path, default=Path("vendor/linux"))
    parser.add_argument("--x86-build", type=Path, required=True)
    parser.add_argument("--arm-build", type=Path, required=True)
    parser.add_argument("--x86-config", type=Path, default=Path("rewrite/configs/x86_64/frozen.config"))
    parser.add_argument("--arm-config", type=Path, default=Path("rewrite/configs/aarch64/frozen.config"))
    parser.add_argument("--out", type=Path, default=Path("rewrite"))
    parser.add_argument(
        "--compiler-predicates", type=Path, default=Path("rewrite/compiler-predicates"),
        help="independently validated frozen compiler predicate evidence",
    )
    args = parser.parse_args()
    authoritative_outputs = (
        "SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv", "LIFETIMES.tsv",
        "DRIVER_ABI.tsv", "TRANSLATION_TASKS.tsv", "TRANSLATION_TASKS.sha256",
    )
    existing = [str(args.out / name) for name in authoritative_outputs if (args.out / name).exists()]
    if existing:
        raise SystemExit(
            "refusing to overwrite an existing Phase 0/queue artifact; extract into a clean "
            f"staging directory or archive the invalidated run first: {', '.join(existing)}"
        )
    linux = args.linux.resolve()
    linux_commit = Path("vendor/linux.SHA").read_text().strip()
    predicate_root = args.compiler_predicates.resolve()
    predicates, predicate_binding = predicate_value_map(predicate_root, linux_commit)
    configs = {"x86_64": args.x86_config.resolve(), "aarch64": args.arm_config.resolve()}
    builds = {"x86_64": args.x86_build.resolve(), "aarch64": args.arm_build.resolve()}
    all_entries: list[dict[str, str]] = []
    by_source: dict[str, list[dict[str, str]]] = defaultdict(list)
    ownership_by_arch: dict[str, dict[str, tuple[str, str, str]]] = {}
    for arch, build in builds.items():
        ownership_by_arch[arch] = kbuild_ownership(build)
        entries = compile_entries(build, linux, arch)
        all_entries.extend(entries)
        for item in entries:
            by_source[item["source_path"]].append(item)
        copy_metadata(build, args.out / "metadata" / arch, arch, entries, ownership_by_arch[arch])

    resolved_by_source: dict[str, list[tuple[dict[str, str], str, str, str]]] = defaultdict(list)
    direct_class_by_path: dict[str, str] = {}
    oracle_reason_by_path: dict[str, str] = {}
    for source_path, items in by_source.items():
        resolved = []
        for item in items:
            mode, owner, evidence = ownership_by_arch[item["architecture"]].get(
                item["object_path"], ("metadata", item["object_path"], "compile_commands.json;ownership-unresolved")
            )
            resolved.append((item, mode, owner, evidence))
        resolved_by_source[source_path] = resolved
        classification, oracle_reason = source_class(
            source_path, items[0]["source_kind"], (owner for _, _, owner, _ in resolved), linux
        )
        direct_class_by_path[source_path] = classification
        if oracle_reason:
            oracle_reason_by_path[source_path] = oracle_reason

    consumer_contexts: dict[tuple[str, str, str], dict[str, str]] = {}
    consumer_headers: dict[tuple[str, str, str], tuple[tuple[str, str], ...]] = {}
    header_consumers: dict[tuple[str, str], set[tuple[str, str, str]]] = defaultdict(set)
    header_consumer_keys_by_path: dict[str, set[tuple[str, str, str]]] = defaultdict(set)
    header_kinds: dict[str, str] = {}
    header_command_evidence: dict[tuple[str, str, str], str] = {}
    for item in sorted(all_entries, key=lambda row: (row["architecture"], row["source_path"], row["object_path"])):
        key = (item["architecture"], item["source_path"], item["object_path"])
        if key in consumer_contexts:
            raise ValueError(f"duplicate compiler context: {key}")
        headers, command_evidence = dependency_headers(
            builds[item["architecture"]], linux, item["architecture"], item["object_path"]
        )
        consumer_contexts[key] = item
        consumer_headers[key] = tuple(headers)
        header_command_evidence[key] = command_evidence
        for header_path, kind in headers:
            prior_kind = header_kinds.setdefault(header_path, kind)
            if prior_kind != kind:
                raise ValueError(f"contradictory header origin for {header_path}: {prior_kind}/{kind}")
            header_consumers[(item["architecture"], header_path)].add(key)
            header_consumer_keys_by_path[header_path].add(key)

    all_header_paths = sorted({path for _, path in header_consumers})
    header_class_by_path: dict[str, str] = {}
    for header_path in all_header_paths:
        consumer_classes = {
            direct_class_by_path[key[1]] for key in header_consumer_keys_by_path[header_path]
        }
        classification, oracle_reason = header_class(
            header_path, header_kinds[header_path], consumer_classes
        )
        header_class_by_path[header_path] = classification
        if oracle_reason:
            oracle_reason_by_path[header_path] = oracle_reason

    overlap = set(by_source) & set(all_header_paths)
    if overlap:
        raise ValueError(f"selected header also appears as a direct compile input: {sorted(overlap)[:10]}")
    all_scope_paths = sorted([*by_source, *all_header_paths])
    scope_id_by_path = {path: f"S{index:06d}" for index, path in enumerate(all_scope_paths, 1)}

    source_rows: list[dict[str, str]] = []
    symbols: list[dict[str, str]] = []
    abi: list[dict[str, str]] = []
    lifetimes: list[dict[str, str]] = []
    driver_abi: list[dict[str, str]] = []
    scope_rows_by_path: dict[str, dict[str, str]] = {}
    for source_path in all_scope_paths:
        scope_id = scope_id_by_path[source_path]
        if source_path in by_source:
            items = by_source[source_path]
            resolved_items = resolved_by_source[source_path]
            classification = direct_class_by_path[source_path]
            arches = sorted({item["architecture"] for item in items})
            kbuild_targets = sorted(
                f"{item['architecture']}:{item['object_path']}:{mode}:{owner}"
                for item, mode, owner, _ in resolved_items
            )
            kconfig_evidence = sorted(
                f"config:{item['architecture']}=rewrite/configs/{ARCH_CONFIG_NAMES[item['architecture']]}/frozen.config;"
                f"disposition={mode};owner={owner};command=metadata/{item['architecture']}/compile_commands.json"
                for item, mode, owner, _ in resolved_items
            )
            source_kind = items[0]["source_kind"]
            metadata_evidence = "rewrite/metadata/manifest.tsv"
            metadata_status = "COMPLETE" if all(mode != "metadata" for _, mode, _, _ in resolved_items) else "PENDING_REVIEW"
        else:
            classification = header_class_by_path[source_path]
            header_keys = sorted(header_consumer_keys_by_path[source_path])
            arches = sorted({key[0] for key in header_keys})
            representatives = []
            for arch in arches:
                choices = [key for key in header_keys if key[0] == arch]
                choices.sort(key=lambda key: (direct_class_by_path[key[1]] != "RUST_TRANSLATE", key[1], key[2]))
                key = choices[0]
                item = consumer_contexts[key]
                mode, owner, evidence = ownership_by_arch[arch].get(
                    item["object_path"], ("metadata", item["object_path"], "compile_commands.json;ownership-unresolved")
                )
                representatives.append((item, mode, owner, evidence, len(choices)))
            kbuild_targets = sorted(
                f"{item['architecture']}:header-via={item['object_path']}:{mode}:{owner}:consumers={count}"
                for item, mode, owner, _, count in representatives
            )
            kconfig_evidence = sorted(
                f"config:{item['architecture']}=rewrite/configs/{ARCH_CONFIG_NAMES[item['architecture']]}/frozen.config;"
                f"disposition={mode};owner={owner};header_closure=metadata/header_closure.tsv;consumers={count}"
                for item, mode, owner, _, count in representatives
            )
            source_kind = "header" if header_kinds[source_path] == "linux" else "generated_header"
            metadata_evidence = "rewrite/metadata/header_closure.tsv"
            metadata_status = "COMPLETE"
        architecture = "common" if set(arches) == {"x86_64", "aarch64"} else arches[0]
        row = {
            "id": scope_id,
            "linux_path": source_path,
            "destination_path": "",
            "class": classification,
            "architectures": architecture,
            "kconfig_evidence": ";".join(kconfig_evidence),
            "kbuild_target": ";".join(kbuild_targets),
            "cluster": source_path.split("/", 1)[0],
            "weight": str(weight(source_path, linux)),
            "risk": risk(source_path),
            "dependencies": "",
            "recommended_implementer": "luna",
            "source_kind": source_kind,
            "metadata_status": metadata_status,
            "metadata_evidence": metadata_evidence,
            "semantic_status": "PENDING_REVIEW" if classification in {"RUST_TRANSLATE", "LINUX_DRIVER_OBJECT"} else "NOT_APPLICABLE",
        }
        source_rows.append(row)
        scope_rows_by_path[source_path] = row
        if classification == "RUST_TRANSLATE" and source_kind == "linux":
            resolved_items = resolved_by_source[source_path]
            for item, _, _, _ in resolved_items:
                source = linux / source_path
                if not source.is_file():
                    raise ValueError(f"RUST_TRANSLATE source does not exist: {source_path}")
                symbol_rows, abi_rows, lifetime_rows = semantic_records(
                    scope_id, source_path, item["architecture"], source,
                    linux, builds[item["architecture"]], configs[item["architecture"]],
                    item["compile_command"], predicates,
                )
                symbols.extend(symbol_rows)
                abi.extend(abi_rows)
                lifetimes.extend(lifetime_rows)
        elif classification == "RUST_TRANSLATE" and source_kind == "header":
            source = linux / source_path
            for arch in arches:
                symbol_rows, abi_rows, lifetime_rows = contextual_header_semantic_records(
                    scope_id, source_path, arch, source
                )
                symbols.extend(symbol_rows)
                abi.extend(abi_rows)
                lifetimes.extend(lifetime_rows)
        elif classification == "LINUX_DRIVER_OBJECT":
            resolved_items = resolved_by_source[source_path]
            for item, mode, owner, evidence in resolved_items:
                driver_abi.append({
                    "scope_id": scope_id,
                    "linux_path": source_path,
                    "architectures": item["architecture"],
                    "object_path": item["object_path"],
                    "kbuild_owner": owner,
                    "module_or_builtin": mode,
                    "record_kind": "driver_object_contract",
                    "abi_item": f"object={item['object_path']};owner={owner};disposition={mode};core_contracts=PENDING_REVIEW",
                    "evidence": f"rewrite/metadata/{item['architecture']}/{evidence}",
                    "status": "PENDING_REVIEW",
                })

    assign_destinations(source_rows)

    header_closure_rows: list[dict[str, str]] = []
    for arch, header_path in sorted(header_consumers):
        keys = header_consumers[(arch, header_path)]
        classes = sorted({direct_class_by_path[key[1]] for key in keys})
        evidence_keys = sorted(keys)
        first = evidence_keys[0]
        last = evidence_keys[-1]
        header_closure_rows.append({
            "architecture": arch,
            "header_path": header_path,
            "header_kind": header_kinds[header_path],
            "class": header_class_by_path[header_path],
            "consumer_count": str(len(keys)),
            "rust_consumer_count": str(sum(direct_class_by_path[key[1]] == "RUST_TRANSLATE" for key in keys)),
            "consumer_classes": ",".join(classes),
            "evidence": (
                f"rewrite/kbuild/{arch}/{header_command_evidence[first]};first={first[1]}:{first[2]};"
                f"last=rewrite/kbuild/{arch}/{header_command_evidence[last]}:{last[1]}:{last[2]}"
            ),
        })

    rust_header_paths = {
        path for path, classification in header_class_by_path.items() if classification == "RUST_TRANSLATE"
    }
    all_header_graph: dict[str, set[str]] = {path: set() for path in all_header_paths}
    all_header_graph_by_arch: dict[str, dict[str, set[str]]] = {
        arch: {path: set() for path in all_header_paths} for arch in builds
    }
    header_edge_rows: list[dict[str, str]] = []
    seen_header_edges: set[tuple[str, str, str]] = set()
    for arch, including_header in sorted(header_consumers):
        contexts = sorted(
            key for key in header_consumers[(arch, including_header)]
            if direct_class_by_path[key[1]] == "RUST_TRANSLATE"
        )
        if not contexts:
            continue
        header_file = selected_header_file(
            including_header, linux, builds[arch], arch
        )
        if not header_file.is_file():
            raise ValueError(f"selected header is unavailable: {arch}:{including_header}")
        text_value = header_file.read_text(errors="replace")
        representative = contexts[0]
        context = consumer_contexts[representative]
        for match in INCLUDE_RE.finditer(text_value):
            delimiter, include_value = match.groups()
            resolved = resolve_include(
                including_header, delimiter, include_value, context,
                linux, builds[arch], arch,
            )
            if resolved is None:
                continue
            included_header, kind = resolved
            if included_header not in all_header_graph:
                continue
            shared = sorted(
                header_consumers[(arch, including_header)]
                & header_consumers.get((arch, included_header), set())
                & set(contexts)
            )
            if not shared:
                continue
            if header_kinds[included_header] != kind:
                raise ValueError(
                    f"resolved header kind mismatch: {arch}:{included_header}:"
                    f"{kind}/{header_kinds[included_header]}"
                )
            edge_key = (arch, including_header, included_header)
            if edge_key in seen_header_edges:
                continue
            seen_header_edges.add(edge_key)
            all_header_graph[including_header].add(included_header)
            all_header_graph_by_arch[arch][including_header].add(included_header)
            witness = shared[0]
            line = text_value.count("\n", 0, match.start()) + 1
            evidence_root = (
                f"rewrite/kbuild/{arch}/{including_header[len(f'generated/{arch}/'):]}"
                if including_header.startswith(f"generated/{arch}/")
                else f"vendor/linux/{including_header}"
            )
            header_edge_rows.append({
                "architecture": arch,
                "including_header": including_header,
                "including_kind": header_kinds[including_header],
                "included_header": included_header,
                "included_kind": kind,
                "relationship": "literal_include",
                "directive": f"{include_value}@{line}",
                "consumer_source": witness[1],
                "consumer_object": witness[2],
                "evidence": (
                    f"{evidence_root}:{line};"
                    f"rewrite/kbuild/{arch}/{header_command_evidence[witness]}"
                ),
            })

    direct_header_graph = project_rust_header_dependencies(
        all_header_graph, rust_header_paths
    )
    direct_header_graph_by_arch = {
        arch: project_rust_header_dependencies(graph, rust_header_paths)
        for arch, graph in all_header_graph_by_arch.items()
    }
    direct_reachability_by_arch = {
        arch: {
            path: reachable(graph, path) for path in sorted(rust_header_paths)
        }
        for arch, graph in direct_header_graph_by_arch.items()
    }
    definitions_by_header: dict[str, set[str]] = {
        path: set() for path in rust_header_paths
    }
    for row in symbols:
        path = row["linux_path"]
        if path not in definitions_by_header:
            continue
        name = row["symbol_name"]
        kind = row["record_kind"]
        if IDENTIFIER.fullmatch(name) and kind == "operative_macro":
            definitions_by_header[path].add(f"macro:{name}")
        elif IDENTIFIER.fullmatch(name) and kind in {"type", "function", "function_macro"}:
            definitions_by_header[path].add(f"identifier:{name}")
        else:
            tag = re.fullmatch(r"(?:struct|union|enum)\s+([A-Za-z_][A-Za-z0-9_]*)", name)
            if tag:
                definitions_by_header[path].add(
                    f"{name.split(None, 1)[0]}:{tag.group(1)}"
                )
    definition_names = set().union(*definitions_by_header.values())
    unresolved_references_by_arch: dict[str, dict[str, set[str]]] = {
        arch: {} for arch in builds
    }
    for path in sorted(rust_header_paths):
        references = header_reference_identifiers(
            (linux / path).read_text(errors="replace"), definition_names,
        ) - definitions_by_header[path]
        for arch in builds:
            directly_available = set(definitions_by_header[path])
            for dependency in direct_reachability_by_arch[arch][path]:
                directly_available.update(definitions_by_header[dependency])
            unresolved_references_by_arch[arch][path] = references - directly_available
    header_graph = {
        path: set(dependencies) for path, dependencies in direct_header_graph.items()
    }
    header_context_records: dict[
        tuple[str, str, str], tuple[dict[str, str], set[str]]
    ] = {}
    context_pairs: set[tuple[str, str]] = set()
    for key in sorted(consumer_contexts):
        arch, consumer_source, consumer_object = key
        if direct_class_by_path[consumer_source] != "RUST_TRANSLATE":
            continue
        context = consumer_contexts[key]
        forced_roots = forced_include_headers(
            context["compile_command"], context["directory"], linux, builds[arch], arch,
        )
        forced_provider_paths: set[str] = set()
        for root in forced_roots:
            if root in rust_header_paths:
                forced_provider_paths.add(root)
                forced_provider_paths.update(direct_reachability_by_arch[arch][root])
        last_definition: dict[str, tuple[str, int]] = {}
        for position, (header_path, _) in enumerate(consumer_headers[key], 1):
            if header_path not in rust_header_paths:
                continue
            candidates: dict[str, tuple[int, set[str]]] = {}
            for name in unresolved_references_by_arch[arch][header_path]:
                prior = last_definition.get(name)
                if prior is None:
                    continue
                provider, provider_position = prior
                stored_position, names = candidates.setdefault(
                    provider, (provider_position, set())
                )
                names.add(name)
                candidates[provider] = (min(stored_position, provider_position), names)
            # If another candidate provider's direct closure already supplies
            # this one, retain the outer provider root.  This preserves the
            # mechanically sufficient prerequisite without manufacturing an
            # edge to every nested definition header in the flattened depfile.
            reduced_candidates = {
                provider: value for provider, value in candidates.items()
                if not any(
                    provider != other
                    and provider in direct_reachability_by_arch[arch][other]
                    and other not in direct_reachability_by_arch[arch][provider]
                    for other in candidates
                )
            }
            for provider, (provider_position, names) in sorted(reduced_candidates.items()):
                header_graph[header_path].add(provider)
                context_pairs.add((header_path, provider))
                edge_key = (arch, header_path, provider)
                record = {
                    "architecture": arch,
                    "header_path": header_path,
                    "provider_header": provider,
                    "relationship": "lexical_identifier_provider",
                    "consumer_source": consumer_source,
                    "consumer_object": consumer_object,
                    "header_position": str(position),
                    "provider_position": str(provider_position),
                    "provided_identifiers": "",
                    "provider_origin": (
                        "forced_include_closure"
                        if provider in forced_provider_paths else "dependency_order"
                    ),
                    "evidence": (
                        f"rewrite/kbuild/{arch}/{header_command_evidence[key]};"
                        f"ordered_dependency_positions={provider_position}<{position}"
                    ),
                }
                if edge_key not in header_context_records:
                    header_context_records[edge_key] = (record, set(names))
                else:
                    header_context_records[edge_key][1].update(names)
            for name in definitions_by_header[header_path]:
                last_definition[name] = (header_path, position)

    header_context_rows: list[dict[str, str]] = []
    for edge_key in sorted(header_context_records):
        record, names = header_context_records[edge_key]
        record["provided_identifiers"] = ",".join(sorted(names))
        header_context_rows.append(record)

    components = strongly_connected_components(header_graph)
    component_by_path: dict[str, int] = {}
    component_tail: dict[int, str] = {}
    component_rows: list[dict[str, str]] = []
    for component_index, members in enumerate(components, 1):
        tail = members[-1]
        component_tail[component_index] = tail
        for order, member in enumerate(members, 1):
            component_by_path[member] = component_index
            component_rows.append({
                "component_id": f"HC{component_index:06d}",
                "member_path": member,
                "member_order": str(order),
                "component_size": str(len(members)),
                "tail_path": tail,
            })
    component_dependencies: dict[int, dict[int, set[str]]] = defaultdict(
        lambda: defaultdict(set)
    )
    for member, dependencies in header_graph.items():
        source_component = component_by_path[member]
        for dependency_path in dependencies:
            dependency_component = component_by_path[dependency_path]
            if dependency_component != source_component:
                relation = (
                    "context" if (member, dependency_path) in context_pairs
                    else "include"
                )
                component_dependencies[source_component][dependency_component].add(relation)

    dependency_rows: list[dict[str, str]] = []
    dependency_paths_by_source: dict[str, set[str]] = defaultdict(set)

    def add_dependency(source_path: str, dependency_path: str, reason: str, evidence: str) -> None:
        if source_path == dependency_path or dependency_path in dependency_paths_by_source[source_path]:
            return
        dependency_paths_by_source[source_path].add(dependency_path)
        dependency_rows.append({
            "task_id": scope_id_by_path[source_path],
            "linux_path": source_path,
            "dependency_task_id": scope_id_by_path[dependency_path],
            "dependency_linux_path": dependency_path,
            "reason": reason,
            "evidence": evidence,
        })

    for component_index, members in enumerate(components, 1):
        first = members[0]
        for dependency_component, relations in sorted(
            component_dependencies[component_index].items()
        ):
            if relations == {"include"}:
                reason = "header_include_component"
                evidence = "rewrite/metadata/header_include_edges.tsv"
            elif relations == {"context"}:
                reason = "header_context_component"
                evidence = "rewrite/metadata/header_context_edges.tsv"
            else:
                reason = "header_provider_component"
                evidence = (
                    "rewrite/metadata/header_include_edges.tsv;"
                    "rewrite/metadata/header_context_edges.tsv"
                )
            add_dependency(
                first, component_tail[dependency_component], reason, evidence,
            )
        for previous, member in zip(members, members[1:]):
            add_dependency(
                member, previous, "header_scc_order",
                f"rewrite/metadata/header_components.tsv;component=HC{component_index:06d}",
            )
    for source_path, classification in sorted(direct_class_by_path.items()):
        if classification != "RUST_TRANSLATE":
            continue
        components_needed = {
            component_by_path[header_path]
            for item, _, _, _ in resolved_by_source[source_path]
            for header_path, _ in consumer_headers[(item["architecture"], item["source_path"], item["object_path"])]
            if header_path in rust_header_paths
        }
        for component_index in sorted(components_needed):
            add_dependency(
                source_path, component_tail[component_index], "source_header_closure",
                f"rewrite/metadata/header_closure.tsv;component=HC{component_index:06d}",
            )
    for source_path, dependencies in dependency_paths_by_source.items():
        scope_rows_by_path[source_path]["dependencies"] = ";".join(
            sorted(scope_id_by_path[path] for path in dependencies)
        )

    oracle_rows = []
    for row in source_rows:
        if row["class"] != "ORACLE_ONLY":
            continue
        reason = oracle_reason_by_path.get(row["linux_path"], "")
        if not reason:
            raise ValueError(
                f"ORACLE_ONLY row lacks mechanical classification reason: {row['linux_path']}"
            )
        oracle_rows.append({
            "linux_path": row["linux_path"],
            "source_kind": row["source_kind"],
            "reason": reason,
            "evidence": (
                f"vendor/linux/{row['linux_path']};"
                f"{row['metadata_evidence']};classification_rule={reason}"
            ),
        })

    write_tsv(args.out / "metadata" / "header_closure.tsv", HEADER_CLOSURE_FIELDS, header_closure_rows)
    write_tsv(args.out / "metadata" / "header_include_edges.tsv", HEADER_INCLUDE_EDGE_FIELDS, header_edge_rows)
    write_tsv(
        args.out / "metadata" / "header_context_edges.tsv",
        HEADER_CONTEXT_EDGE_FIELDS,
        header_context_rows,
    )
    write_tsv(args.out / "metadata" / "header_components.tsv", HEADER_COMPONENT_FIELDS, component_rows)
    write_tsv(args.out / "metadata" / "task_dependencies.tsv", TASK_DEPENDENCY_FIELDS, dependency_rows)
    write_tsv(
        args.out / "metadata" / "oracle_classification.tsv",
        ORACLE_CLASSIFICATION_FIELDS,
        oracle_rows,
    )
    write_tsv(args.out / "SCOPE.tsv", SCOPE_FIELDS, source_rows)
    file_rows = []
    for item in sorted(all_entries, key=lambda row: (row["architecture"], row["source_path"], row["object_path"], row["compile_command"])):
        mode, owner, evidence = ownership_by_arch[item["architecture"]].get(
            item["object_path"], ("metadata", item["object_path"], "compile_commands.json;ownership-unresolved")
        )
        file_rows.append({
            **item,
            "module_or_builtin": mode,
            "kbuild_owner": owner,
            "disposition_evidence": evidence,
            "metadata_evidence": "rewrite/metadata/manifest.tsv",
        })
    for arch, header_path in sorted(header_consumers):
        keys = sorted(
            header_consumers[(arch, header_path)],
            key=lambda key: (direct_class_by_path[key[1]] != "RUST_TRANSLATE", key[1], key[2]),
        )
        representative = keys[0]
        item = consumer_contexts[representative]
        mode, owner, evidence = ownership_by_arch[arch].get(
            item["object_path"], ("metadata", item["object_path"], "compile_commands.json;ownership-unresolved")
        )
        file_rows.append({
            **item,
            "source_path": header_path,
            "module_or_builtin": mode,
            "kbuild_owner": owner,
            "disposition_evidence": f"header-closure:{header_command_evidence[representative]};{evidence}",
            "metadata_evidence": "rewrite/metadata/header_closure.tsv",
        })
    write_tsv(args.out / "FILE_MAP.tsv", FILE_MAP_FIELDS, file_rows)
    write_tsv(args.out / "SYMBOLS.tsv", SYMBOL_FIELDS, symbols)
    write_tsv(args.out / "ABI.tsv", ABI_FIELDS, abi)
    write_tsv(args.out / "LIFETIMES.tsv", LIFETIME_FIELDS, lifetimes)
    write_tsv(args.out / "DRIVER_ABI.tsv", DRIVER_ABI_FIELDS, driver_abi)

    summary = {
        "linux_commit": Path("vendor/linux.SHA").read_text().strip(),
        "sources_total": len(source_rows),
        "compiled_sources_total": len(by_source),
        "selected_headers_total": len(all_header_paths),
        "rust_headers": len(rust_header_paths),
        "header_include_edges": len(header_edge_rows),
        "header_context_edges": len(header_context_rows),
        "header_components": len(components),
        "task_dependency_edges": len(dependency_rows),
        "oracle_only": len(oracle_rows),
        "oracle_reasons": {},
        "rust_translate": sum(row["class"] == "RUST_TRANSLATE" for row in source_rows),
        "symbols": len(symbols),
        "abi_records": len(abi),
        "lifetime_records": len(lifetimes),
        "by_class": {},
    }
    for row in source_rows:
        summary["by_class"][row["class"]] = summary["by_class"].get(row["class"], 0) + 1
    for row in oracle_rows:
        summary["oracle_reasons"][row["reason"]] = (
            summary["oracle_reasons"].get(row["reason"], 0) + 1
        )
    (args.out / "metadata" / "summary.json").write_text(
        json.dumps(summary, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )
    write_tsv(
        args.out / "metadata" / "compiler-predicates-binding.tsv",
        ["key", "value"],
        [{"key": key, "value": value} for key, value in sorted(predicate_binding.items())],
    )
    authoritative_manifest_rows = [
        {"path": name, "sha256": sha256(args.out / name)}
        for name in ("SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv", "LIFETIMES.tsv", "DRIVER_ABI.tsv")
    ]
    write_tsv(
        args.out / "metadata" / "authoritative_manifests.tsv",
        ["path", "sha256"],
        authoritative_manifest_rows,
    )
    manifest_rows = []
    for path in sorted((args.out / "metadata").rglob("*")):
        if path.is_file() and path != args.out / "metadata" / "manifest.tsv":
            manifest_rows.append({"path": path.relative_to(args.out).as_posix(), "sha256": sha256(path)})
    write_tsv(args.out / "metadata" / "manifest.tsv", ["path", "sha256"], manifest_rows)


if __name__ == "__main__":
    main()
