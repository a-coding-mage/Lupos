#!/usr/bin/env python3
"""Independent validator for source-only Phase 0 manifests and queue freezing."""

from __future__ import annotations

import argparse
import ast
from collections import defaultdict
import contextlib
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile

import semantic_closure


ROOT = Path(__file__).resolve().parents[1]
LLVM_ROOT = Path("/usr/lib/llvm-19/bin")
ARCHES = ("x86_64", "aarch64")
CONFIG_EVIDENCE = {
    "x86_64": "rewrite/configs/x86_64/frozen.config",
    "aarch64": "rewrite/configs/aarch64/frozen.config",
}
ENTITY_KINDS = {"function", "function_macro", "type", "static", "global", "export"}
LIFETIME_KINDS = {"function", "function_macro", "type", "static", "global"}
PREDICATE_FIELDS = {
    "predicate_id", "predicate_kind", "argument", "architecture", "result",
    "status", "linux_commit", "config_sha256", "toolchain_sha256",
}
HEADER_CLOSURE_FIELDS = {
    "architecture", "header_path", "header_kind", "class",
    "consumer_count", "rust_consumer_count", "consumer_classes", "evidence",
}
HEADER_INCLUDE_EDGE_FIELDS = {
    "architecture", "including_header", "including_kind", "included_header",
    "included_kind", "relationship", "directive", "consumer_source",
    "consumer_object", "evidence",
}
HEADER_CONTEXT_EDGE_FIELDS = {
    "architecture", "header_path", "provider_header", "relationship",
    "consumer_source", "consumer_object", "header_position",
    "provider_position", "provided_identifiers", "provider_origin", "evidence",
}
ORACLE_CLASSIFICATION_FIELDS = {
    "linux_path", "source_kind", "reason", "evidence",
}
CANONICAL_TASK_EVIDENCE = {
    "implementation.md", "candidate.diff", "parity-review.md",
    "rust-review.md", "resolution.md",
    *semantic_closure.SEMANTIC_EVIDENCE_FILES,
}
QUARANTINE_METADATA_FIELDS = [
    "schema_version", "superseded_fingerprint", "task_id",
    "original_status", "original_attempt", "original_resume_status",
    "provenance_state", "file_set_state", "file_name", "sha256", "bytes",
]
QUARANTINE_SCHEMA_VERSION = "task-evidence-quarantine-v2"
LEGACY_QUARANTINE_SCHEMA_VERSION = "task-evidence-quarantine-v1"
QUARANTINE_FILE_SET_STATES = {
    "QUEUE_STAGE_FILESET_MATCH",
    "QUEUE_STAGE_FILESET_MISMATCH",
    "QUEUE_STAGE_FILESET_UNSPECIFIED",
}
QUARANTINE_PROVENANCE_STATE = "UNPROVEN_MIXED_OR_UNKNOWN"
INCLUDE_RE = re.compile(r'^\s*#\s*include\s*([<"])([^>"\n]+)[>"]', re.M)
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def read_tsv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        return list(reader.fieldnames or []), list(reader)


def rows(path: Path) -> list[dict[str, str]]:
    return read_tsv(path)[1]


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def key_values(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("\t")
        if separator:
            result[key] = value
    return result


def check(checks: dict[str, dict[str, object]], name: str, ok: bool, detail: object) -> None:
    checks[name] = {"ok": bool(ok), "detail": str(detail)}


def required_fields(
    checks: dict[str, dict[str, object]],
    name: str,
    path: Path,
    required: set[str],
) -> list[dict[str, str]]:
    if not path.is_file():
        check(checks, name, False, f"missing {path}")
        return []
    fields, content = read_tsv(path)
    missing = sorted(required - set(fields))
    check(checks, name, not missing, f"rows={len(content)}; missing_fields={missing}")
    return content


def expected_arches(scope_architecture: str) -> set[str]:
    if scope_architecture == "common":
        return set(ARCHES)
    return {scope_architecture}


def normalize_path(value: str) -> str:
    return os.path.normpath(value).replace(os.sep, "/")


def make_assignment(text: str, variable: str) -> str | None:
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


def canonical_dependency(value: str, linux: Path, build: Path, arch: str) -> tuple[str, str] | None:
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
        relative = candidate.relative_to(build_abs).as_posix()
    except ValueError:
        return None
    return f"generated/{arch}/{relative}", "generated"


def dependency_headers(build: Path, linux: Path, arch: str, object_path: str) -> list[tuple[str, str]]:
    command_file = command_evidence_path(build, object_path)
    content = command_file.read_text(encoding="utf-8", errors="strict")
    variables = re.findall(r"^(deps_[^\s:]+)\s*:=", content, flags=re.M)
    matches = [
        variable for variable in variables
        if normalize_path(variable[len("deps_"):]) == normalize_path(object_path)
    ]
    if len(matches) != 1:
        raise ValueError(f"expected one dependency assignment for {arch}:{object_path}; found {matches}")
    assignment = make_assignment(content, matches[0])
    if assignment is None:
        raise ValueError(f"cannot parse {matches[0]} from {command_file}")
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
    return result


def validate_semantic_proposal_seal_contract(
    artifacts: Path,
) -> tuple[bool, dict[str, object]]:
    """Exercise the source-only proposal validation and seal boundary."""

    task_id = "S000013"
    expected = semantic_closure.expected_closure_records(artifacts, task_id)
    if not expected:
        return False, {"error": f"{task_id} has no semantic closure records"}
    metadata = {
        "schema_version": semantic_closure.PROPOSAL_SCHEMA_VERSION,
        "task_id": task_id,
        "attempt": "1",
        "pipeline_id": "P01",
        "linux_sha": (ROOT / "vendor/linux.SHA").read_text(encoding="utf-8").strip(),
        "candidate_sha256": "1" * 64,
        "implementation_sha256": "2" * 64,
        "phase0_identity_sha256": "3" * 64,
        "queue_fingerprint": "4" * 64,
    }
    records: list[dict[str, str]] = []
    for expected_record in expected:
        record = {field: "" for field in semantic_closure.PROPOSAL_FIELDS}
        record.update(metadata)
        record.update(expected_record)
        record["decision_status"] = "COMPLETE"
        record["final_value"] = (
            "COMPLETE"
            if record["field"] == "status"
            or (record["manifest"] == "SCOPE.tsv" and record["field"] == "semantic_status")
            else "SOURCE_REVIEWED_VALUE"
        )
        records.append(record)

    with tempfile.TemporaryDirectory(prefix="lupos-semantic-seal-") as directory:
        root = Path(directory)
        proposal = root / "semantic-closure-proposal.tsv"
        seal = root / "semantic-closure-proposal.sha256"
        partial = root / "partial.tsv"
        partial_seal = root / "partial.sha256"
        semantic_closure.atomic_write_tsv(
            proposal, semantic_closure.PROPOSAL_FIELDS, records
        )
        validated = semantic_closure.seal_validated_proposal(
            proposal, seal,
            rewrite=artifacts,
            task_id=task_id,
            attempt=1,
            pipeline="P01",
            identity_hash=metadata["phase0_identity_sha256"],
            fingerprint=metadata["queue_fingerprint"],
            candidate_hash=metadata["candidate_sha256"],
            implementation_hash=metadata["implementation_sha256"],
        )
        seal_values = semantic_closure.read_proposal_seal(proposal, seal)
        expected_hash = semantic_closure.sha256_file(proposal)
        complete_bound = (
            seal_values.get("sha256") == expected_hash
            and seal_values.get("records") == str(len(records))
            and len(validated) == len(records)
        )

        semantic_closure.atomic_write_tsv(
            partial, semantic_closure.PROPOSAL_FIELDS, records[:-1]
        )
        partial_rejected = False
        with contextlib.redirect_stderr(io.StringIO()):
            try:
                semantic_closure.seal_validated_proposal(
                    partial, partial_seal,
                    rewrite=artifacts,
                    task_id=task_id,
                    attempt=1,
                    pipeline="P01",
                    identity_hash=metadata["phase0_identity_sha256"],
                    fingerprint=metadata["queue_fingerprint"],
                    candidate_hash=metadata["candidate_sha256"],
                    implementation_hash=metadata["implementation_sha256"],
                )
            except SystemExit:
                partial_rejected = True
        partial_rejected = partial_rejected and not partial_seal.exists()

        original = proposal.read_bytes()
        proposal.write_bytes(original + b"\n")
        mutated_rejected = False
        with contextlib.redirect_stderr(io.StringIO()):
            try:
                semantic_closure.read_proposal_seal(proposal, seal)
            except SystemExit:
                mutated_rejected = True
        return complete_bound and partial_rejected and mutated_rejected, {
            "task_id": task_id,
            "records": len(records),
            "proposal_sha256": expected_hash,
            "complete_bound": complete_bound,
            "partial_rejected": partial_rejected,
            "mutated_rejected": mutated_rejected,
        }


def include_search_directories(command: str, directory: Path) -> list[Path]:
    """Independently recover frozen quote/angle include roots."""

    tokens = shlex.split(command)
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
            result.append(path if path.is_absolute() else directory / path)
    return result


def selected_header_file(header_path: str, linux: Path, build: Path, arch: str) -> Path:
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
    command: str,
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
        for directory in include_search_directories(command, build)
    )
    for candidate in candidates:
        if candidate.is_file():
            normalized = canonical_dependency(str(candidate), linux, build, arch)
            if normalized is not None:
                return normalized
    return None


def expected_oracle_path_rule(path: str) -> tuple[str, bool]:
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


def expected_driver_owned_source(path: str, owners: set[str]) -> bool:
    return path.startswith(("drivers/", "sound/")) or any(
        owner.startswith(("drivers/", "sound/")) for owner in owners
    )


def expected_header_classification(
    path: str, kind: str, consumer_classes: set[str],
) -> tuple[str, str]:
    if kind != "linux":
        return "BUILD_METADATA", ""
    oracle_reason, ownership_override = expected_oracle_path_rule(path)
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


def expected_header_class(path: str, kind: str, consumer_classes: set[str]) -> str:
    return expected_header_classification(path, kind, consumer_classes)[0]


def reachable(graph: dict[str, set[str]], start: str) -> set[str]:
    result: set[str] = set()
    pending = list(graph.get(start, set()))
    while pending:
        node = pending.pop()
        if node in result:
            continue
        result.add(node)
        pending.extend(graph.get(node, set()) - result)
    return result


def project_rust_header_dependencies(
    graph: dict[str, set[str]], rust_headers: set[str],
) -> dict[str, set[str]]:
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


def strongly_connected_components(graph: dict[str, set[str]]) -> list[list[str]]:
    """Independently replay deterministic SCCs without recursive traversal."""

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


def code_only(text: str) -> str:
    """Mask C comments and literals without changing line structure."""

    def preserve_newlines(match: re.Match[str]) -> str:
        return "\n" * match.group(0).count("\n")

    text = re.sub(r"/\*.*?\*/", preserve_newlines, text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r'"(?:\\.|[^"\\])*"', '""', text)
    text = re.sub(r"'(?:\\.|[^'\\])*'", "''", text)
    return text


def independent_integer_expression(expression: str, values: dict[str, int]) -> int | None:
    """Independently evaluate the small enum-expression subset Phase 0 proves."""

    candidate = re.sub(r"\b(0[xX][0-9A-Fa-f]+|[0-9]+)[uUlL]+\b", r"\1", expression)
    for name in sorted(set(IDENTIFIER.findall(candidate)), key=len, reverse=True):
        if name not in values:
            return None
        candidate = re.sub(rf"\b{re.escape(name)}\b", str(values[name]), candidate)
    try:
        tree = ast.parse(candidate.strip(), mode="eval")
    except SyntaxError:
        return None
    permitted = (
        ast.Expression, ast.BinOp, ast.UnaryOp, ast.Constant,
        ast.Invert, ast.UAdd, ast.USub, ast.Add, ast.Sub, ast.Mult,
        ast.Mod, ast.LShift, ast.RShift, ast.BitAnd, ast.BitOr, ast.BitXor,
    )
    if any(not isinstance(node, permitted) for node in ast.walk(tree)):
        return None
    try:
        value = eval(compile(tree, "<phase0-enum-validation>", "eval"), {"__builtins__": {}}, {})
    except (ArithmeticError, ValueError):
        return None
    return value if isinstance(value, int) else None


def independent_enum_constants(path: Path) -> list[tuple[str, int, str]]:
    """Recover C enumerator names, lines, and provable values from source."""

    original = path.read_text(errors="replace")
    masked = code_only(original)
    lines: list[str] = []
    in_directive = False
    for line in masked.splitlines(keepends=True):
        directive = in_directive or re.match(r"^\s*#", line) is not None
        in_directive = directive and line.rstrip().endswith("\\")
        lines.append("".join("\n" if char == "\n" else " " for char in line) if directive else line)
    text = "".join(lines)
    enum_open = re.compile(r"\benum(?:\s+[A-Za-z_][A-Za-z0-9_]*)?\s*\{")
    result: list[tuple[str, int, str]] = []
    for match in enum_open.finditer(text):
        start = match.end()
        depth = 1
        end = start
        while end < len(text) and depth:
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
            end += 1
        if depth:
            continue
        body_end = end - 1
        boundaries: list[tuple[int, int]] = []
        segment_start = start
        nesting = 0
        for cursor in range(start, body_end):
            char = text[cursor]
            if char in "([":
                nesting += 1
            elif char in ")]" and nesting:
                nesting -= 1
            elif char == "," and nesting == 0:
                boundaries.append((segment_start, cursor))
                segment_start = cursor + 1
        boundaries.append((segment_start, body_end))
        values: dict[str, int] = {}
        previous: int | None = -1
        for segment_start, segment_end in boundaries:
            segment = text[segment_start:segment_end]
            item = re.match(
                r"\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:=\s*(.*?))?\s*$",
                segment,
                flags=re.S,
            )
            if item is None:
                previous = None
                continue
            name, expression = item.groups()
            line = text.count("\n", 0, segment_start + item.start(1)) + 1
            value = (
                previous + 1 if expression is None and previous is not None
                else independent_integer_expression(expression, values) if expression is not None
                else None
            )
            if value is not None:
                values[name] = value
            previous = value
            result.append((name, line, str(value) if value is not None else "PENDING_REVIEW"))
    return result


def header_reference_identifiers(text: str, definition_names: set[str]) -> set[str]:
    masked = code_only(text)
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
    command: str, linux: Path, build: Path, arch: str,
) -> set[str]:
    tokens = shlex.split(command)
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
        candidate = candidate if candidate.is_absolute() else build / candidate
        normalized = canonical_dependency(str(candidate), linux, build, arch)
        if normalized is not None:
            result.add(normalized[0])
    return result


def source_categories(path: Path) -> set[str]:
    """Detect unconditional categories whose absence from SYMBOLS is impossible."""
    text = path.read_text(errors="replace")
    unconditional_lines = []
    conditional_depth = 0
    has_conditional = False
    for line in text.splitlines(keepends=True):
        directive = re.match(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b", line)
        if directive:
            has_conditional = True
            kind = directive.group(1)
            if kind in {"if", "ifdef", "ifndef"}:
                conditional_depth += 1
            elif kind == "endif":
                conditional_depth = max(0, conditional_depth - 1)
            unconditional_lines.append("\n" if line.endswith("\n") else "")
            continue
        unconditional_lines.append(line if conditional_depth == 0 else "\n" if line.endswith("\n") else "")
    raw_text = "".join(unconditional_lines)
    categories: set[str] = set()
    if re.search(r"(?m)^\s*#\s*define\s+[A-Za-z_]", code_only(raw_text)):
        categories.add("operative_macro")
    if has_conditional:
        categories.add("conditional")
    # Preprocessor directives are not executable declarations.  Leaving a
    # function-like macro definition in this light-weight category probe can
    # make its brace initializer look like a C function body.
    non_directive_lines: list[str] = []
    in_directive = False
    for line in raw_text.splitlines(keepends=True):
        starts_directive = re.match(r"^\s*#", line) is not None
        if in_directive or starts_directive:
            in_directive = line.rstrip().endswith("\\")
            non_directive_lines.append("\n" if line.endswith("\n") else "")
        else:
            non_directive_lines.append(line)
    text = code_only("".join(non_directive_lines))
    if re.search(r"(?m)^static\s+", text):
        categories.add("static")
    if re.search(r"\b(?:struct|union|enum)\s+[A-Za-z_]\w*\s*\{", text) or re.search(
        r"\btypedef\b[^;]*;", text, flags=re.S
    ):
        categories.add("type")
    # Avoid an unbounded DOTALL expression here: large generated initializers
    # can make it quadratic. A bounded declaration window still catches every
    # ordinary top-level definition without treating a whole source file as one
    # possible prototype.
    function_like = False
    window: list[str] = []
    window_size = 0
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        window.append(stripped)
        window_size += len(stripped)
        candidate = " ".join(window)
        if "{" in stripped:
            before_body = candidate.split("{", 1)[0]
            function_like = bool(
                "(" in before_body
                and ")" in before_body
                and "=" not in before_body
                and not re.match(r"^(?:if|for|while|switch)\b", before_body)
            )
            window.clear()
            window_size = 0
            if function_like:
                break
        elif ";" in stripped or "}" in stripped or window_size > 8192:
            window.clear()
            window_size = 0
    if function_like or re.search(
        r"(?m)^\s*(?:(?:COMPAT_)?SYSCALL_DEFINE\d+|BPF_CALL_\d+)\s*\(", text
    ):
        categories.add("function")
    return categories


def queue_matches_scope(scope_rows: list[dict[str, str]], task_rows: list[dict[str, str]]) -> tuple[bool, str]:
    selected = {row["id"]: row for row in scope_rows if row.get("class") == "RUST_TRANSLATE"}
    tasks = {row.get("id", ""): row for row in task_rows}
    if len(selected) != len(tasks) or set(selected) != set(tasks):
        return False, f"scope_ids={len(selected)} task_ids={len(tasks)} delta={sorted(set(selected) ^ set(tasks))[:10]}"
    mismatches = []
    mapping = {
        "path": "destination_path",
        "linux_path": "linux_path",
        "architectures": "architectures",
        "cluster": "cluster",
        "weight": "weight",
        "risk": "risk",
        "dependencies": "dependencies",
        "recommended_implementer": "recommended_implementer",
    }
    for task_id, source in selected.items():
        task = tasks[task_id]
        changed = [task_field for task_field, scope_field in mapping.items() if task.get(task_field, "") != source.get(scope_field, "")]
        if changed:
            mismatches.append(f"{task_id}:{','.join(changed)}")
    return not mismatches, f"scope={len(selected)} tasks={len(tasks)} mismatches={mismatches[:10]}"


def quarantine_file_set_state(
    status: str, resume_status: str, observed: set[str]
) -> str:
    if status == "PAUSED":
        status = resume_status
    implementation = {
        "implementation.md", "candidate.diff",
        "semantic-closure-proposal.tsv", "semantic-closure-proposal.sha256",
    }
    reviews = implementation | {
        "parity-review.md", "rust-review.md",
        "semantic-closure-parity-review.tsv", "semantic-closure-rust-review.tsv",
    }
    complete = set(CANONICAL_TASK_EVIDENCE)
    matches = False
    if status == "TODO":
        matches = not observed
    elif status == "IN_PROGRESS":
        matches = observed <= implementation
    elif status == "IMPLEMENTED":
        matches = observed == implementation
    elif status == "REVIEWING":
        matches = implementation <= observed <= reviews
    elif status == "APPLYING":
        matches = reviews <= observed <= complete
    elif status == "DONE":
        matches = observed == complete
    elif status == "BLOCKED":
        return "QUEUE_STAGE_FILESET_UNSPECIFIED"
    return "QUEUE_STAGE_FILESET_MATCH" if matches else "QUEUE_STAGE_FILESET_MISMATCH"


def validate_task_evidence_quarantine(
    logs_root: Path,
) -> tuple[list[str], list[str], dict[str, dict[str, object]]]:
    """Validate root isolation and every task-local invalidated generation."""

    root_files: list[str] = []
    errors: list[str] = []
    generations: dict[str, dict[str, object]] = {}
    if not logs_root.exists():
        return root_files, errors, generations
    if logs_root.is_symlink() or not logs_root.is_dir():
        return root_files, [f"invalid-logs-root:{logs_root}"], generations

    for task_dir in sorted(logs_root.iterdir(), key=lambda item: item.name):
        if task_dir.is_symlink() or not task_dir.is_dir():
            continue
        for name in sorted(CANONICAL_TASK_EVIDENCE):
            path = task_dir / name
            if path.exists() or path.is_symlink():
                root_files.append(path.relative_to(logs_root).as_posix())
        invalidated = task_dir / "invalidated-generations"
        if not invalidated.exists():
            continue
        if invalidated.is_symlink() or not invalidated.is_dir():
            errors.append(f"invalid-generation-root:{invalidated}")
            continue
        for generation in sorted(invalidated.iterdir(), key=lambda item: item.name):
            fingerprint = generation.name
            prefix = f"{task_dir.name}:{fingerprint}"
            if (
                generation.is_symlink()
                or not generation.is_dir()
                or not re.fullmatch(r"[0-9a-f]{64}", fingerprint)
            ):
                errors.append(f"invalid-generation-directory:{generation}")
                continue
            metadata = generation / "QUARANTINE.tsv"
            if not metadata.is_file() or metadata.is_symlink():
                errors.append(f"{prefix}:missing-metadata")
                continue
            fields, records = read_tsv(metadata)
            if fields != QUARANTINE_METADATA_FIELDS or not records:
                errors.append(f"{prefix}:metadata-schema-or-empty")
                continue
            names = [record.get("file_name", "") for record in records]
            observed = set(names)
            if names != sorted(names) or len(observed) != len(names):
                errors.append(f"{prefix}:nondeterministic-or-duplicate-files")
            common = records[0]
            schema_version = common.get("schema_version", "")
            if schema_version == LEGACY_QUARANTINE_SCHEMA_VERSION:
                # v1 pre-dates the semantic-closure evidence files.  Its
                # file_set_state was computed against the five original
                # evidence names, so recomputing it against today's expanded
                # set would rewrite the meaning of immutable history.
                expected_state = common.get("file_set_state", "")
                if expected_state not in QUARANTINE_FILE_SET_STATES:
                    errors.append(f"{prefix}:invalid-legacy-file-set-state")
            elif schema_version == QUARANTINE_SCHEMA_VERSION:
                expected_state = quarantine_file_set_state(
                    common.get("original_status", ""),
                    common.get("original_resume_status", ""),
                    observed,
                )
            else:
                errors.append(f"{prefix}:unsupported-schema:{schema_version}")
                continue
            repeated = {
                "schema_version": schema_version,
                "superseded_fingerprint": fingerprint,
                "task_id": task_dir.name,
                "original_status": common.get("original_status", ""),
                "original_attempt": common.get("original_attempt", ""),
                "original_resume_status": common.get("original_resume_status", ""),
                "provenance_state": QUARANTINE_PROVENANCE_STATE,
                "file_set_state": expected_state,
            }
            if not common.get("original_attempt", "").isdigit():
                errors.append(f"{prefix}:invalid-attempt")
            for record in records:
                if any(record.get(key, "") != value for key, value in repeated.items()):
                    errors.append(f"{prefix}:inconsistent-metadata:{record.get('file_name', '')}")
                    break
            payload_names = {
                item.name for item in generation.iterdir()
                if item.name != "QUARANTINE.tsv"
            }
            if payload_names != observed or not observed <= CANONICAL_TASK_EVIDENCE:
                errors.append(f"{prefix}:payload-set-mismatch")
            for record in records:
                payload = generation / record.get("file_name", "")
                try:
                    expected_bytes = int(record.get("bytes", ""))
                except ValueError:
                    errors.append(f"{prefix}:invalid-bytes:{record.get('file_name', '')}")
                    continue
                if (
                    payload.is_symlink()
                    or not payload.is_file()
                    or payload.stat().st_size != expected_bytes
                    or digest(payload) != record.get("sha256", "")
                ):
                    errors.append(f"{prefix}:payload-hash:{record.get('file_name', '')}")
            summary = generations.setdefault(
                fingerprint,
                {"tasks": set(), "files": 0, "bytes": 0, "states": defaultdict(int)},
            )
            summary["tasks"].add(task_dir.name)
            summary["files"] += len(records)
            summary["bytes"] += sum(
                int(record["bytes"])
                for record in records
                if record.get("bytes", "").isdigit()
            )
            summary["states"][expected_state] += 1
    return root_files, errors, generations


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--artifacts", type=Path, default=Path("rewrite"))
    parser.add_argument("--stage", choices=("pre-queue", "frozen"), default="frozen")
    parser.add_argument(
        "--phase-gate-reopen",
        action="store_true",
        help="validate a recorded post-translation Phase 0 gate recovery without requiring src/ to be empty",
    )
    parser.add_argument("--no-write-report", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    artifacts = args.artifacts if args.artifacts.is_absolute() else root / args.artifacts
    canonical = root / "rewrite"
    checks: dict[str, dict[str, object]] = {}

    branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip()
    check(checks, "branch", branch == "feat/bun-like-rewrite-test", branch)
    pinned = (root / "vendor/linux.SHA").read_text().strip()
    head = subprocess.check_output(["git", "-C", "vendor/linux", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    linux_status = subprocess.check_output(
        ["git", "-C", "vendor/linux", "status", "--short"], cwd=root, text=True
    )
    check(checks, "linux_pin", head == pinned, f"HEAD={head}; pinned={pinned}")
    check(checks, "linux_clean", not linux_status, linux_status or "clean")

    tool_rows = rows(canonical / "toolchain/TOOLCHAIN.tsv")
    for tool in tool_rows:
        path = Path(tool["requested_path"])
        resolved = path.resolve() if path.exists() else Path("missing")
        valid = (
            tool.get("status") == "VERIFIED"
            and path.is_file()
            and bool(path.stat().st_mode & 0o111)
            and resolved.is_relative_to(LLVM_ROOT)
            and tool.get("major_version") == "19"
            and digest(path) == tool.get("sha256")
        )
        check(
            checks,
            f"tool:{tool.get('tool_name', 'unknown')}",
            valid,
            f"requested={path}; resolved={resolved}; major={tool.get('major_version')}",
        )
    toolchain_hash_file = canonical / "toolchain/TOOLCHAIN.sha256"
    check(
        checks,
        "toolchain_file_hash",
        digest(canonical / "toolchain/TOOLCHAIN.tsv") == toolchain_hash_file.read_text().split()[0],
        toolchain_hash_file,
    )
    linkers = rows(canonical / "toolchain/LINKER_INVENTORY.tsv")
    selected_linkers = [row for row in linkers if row.get("selected") == "YES"]
    check(
        checks,
        "selected_linker",
        len(selected_linkers) == 1
        and selected_linkers[0].get("resolved_path") == "/usr/lib/llvm-19/bin/lld"
        and ".rustup" not in selected_linkers[0].get("resolved_path", ""),
        selected_linkers,
    )
    check(
        checks,
        "rust_linker_rejected",
        not any(
            row.get("selected") == "YES"
            and ("rust-lld" in row.get("resolved_path", "") or ".rustup" in row.get("resolved_path", ""))
            for row in linkers
        ),
        "LINKER_INVENTORY.tsv",
    )

    config_hashes: dict[str, str] = {}
    for arch in ARCHES:
        config = canonical / f"configs/{arch}/frozen.config"
        build_config = canonical / f"kbuild/{arch}/.config"
        transition = rows(canonical / f"configs/{arch}/config-transition.tsv")
        config_hashes[arch] = digest(config)
        check(checks, f"{arch}_build_config", build_config.is_file() and config.read_bytes() == build_config.read_bytes(), build_config)
        check(
            checks,
            f"{arch}_stable_transition",
            any(row.get("status") == "STABLE" and row.get("before") == "0_changed_symbols" for row in transition),
            transition,
        )

    predicate_root = canonical / "compiler-predicates"
    inventory_path = predicate_root / "COMPILER_PREDICATES.tsv"
    predicate_fingerprint_path = predicate_root / "COMPILER_PREDICATES.sha256"
    predicate_validation_path = predicate_root / "VALIDATION.tsv"
    predicate_report_path = predicate_root / "validation-report.md"
    predicate_rows = required_fields(checks, "compiler_predicate_schema", inventory_path, PREDICATE_FIELDS)
    predicate_fingerprint = key_values(predicate_fingerprint_path) if predicate_fingerprint_path.is_file() else {}
    check(
        checks,
        "compiler_predicate_fingerprint",
        inventory_path.is_file()
        and predicate_fingerprint.get("sha256") == digest(inventory_path)
        and predicate_fingerprint.get("rows") == str(len(predicate_rows))
        and predicate_fingerprint.get("linux_commit") == pinned
        and predicate_fingerprint.get("toolchain_sha256") == (canonical / "toolchain/TOOLCHAIN.sha256").read_text().split()[0],
        predicate_fingerprint_path,
    )
    validation_rows = required_fields(
        checks, "compiler_predicate_validation_schema", predicate_validation_path,
        {"predicate_id", "validation_status"},
    )
    validation_by_id = {row.get("predicate_id", ""): row.get("validation_status", "") for row in validation_rows}
    predicate_errors = []
    predicate_evidence_errors = []
    predicate_counts = defaultdict(int)
    predicate_keys = set()
    for row in predicate_rows:
        arch = row.get("architecture", "")
        key = (arch, row.get("predicate_kind", ""), row.get("argument", ""))
        if (
            arch not in ARCHES
            or key in predicate_keys
            or row.get("status") != "PROVEN"
            or row.get("result") not in {"0", "1"}
            or row.get("linux_commit") != pinned
            or row.get("config_sha256") != config_hashes.get(arch, "")
            or validation_by_id.get(row.get("predicate_id", "")) != "PASS"
        ):
            predicate_errors.append(row.get("predicate_id", ""))
        evidence_fields = (
            ("original_command_source", "original_command_sha256"),
            ("probe_path", "probe_sha256"),
            ("probe_command_path", "probe_command_sha256"),
            ("stdout_path", "stdout_sha256"),
            ("stderr_path", "stderr_sha256"),
        )
        evidence_bad = any(
            not (root / row.get(path_field, "")).is_file()
            or digest(root / row.get(path_field, "")) != row.get(hash_field, "")
            for path_field, hash_field in evidence_fields
        )
        compiler_path = Path(row.get("compiler_requested_path", ""))
        compiler_resolved = compiler_path.resolve() if compiler_path.exists() else Path("missing")
        if (
            evidence_bad
            or row.get("compiler_requested_path") != "/usr/lib/llvm-19/bin/clang"
            or str(compiler_resolved) != "/usr/lib/llvm-19/bin/clang"
            or not compiler_path.is_file()
            or digest(compiler_path) != row.get("compiler_sha256", "")
            or not row.get("target_triple")
            or row.get("exit_status") != "0"
        ):
            predicate_evidence_errors.append(row.get("predicate_id", ""))
        predicate_keys.add(key)
        predicate_counts[arch] += 1
    check(
        checks,
        "compiler_predicates_proven",
        bool(predicate_rows)
        and not predicate_errors
        and all(predicate_counts[arch] > 0 for arch in ARCHES)
        and "- Result: PASS" in predicate_report_path.read_text(encoding="utf-8") if predicate_report_path.is_file() else False,
        predicate_errors[:20],
    )
    check(
        checks,
        "compiler_predicate_raw_evidence",
        not predicate_evidence_errors,
        predicate_evidence_errors[:20],
    )
    predicate_binding_rows = required_fields(
        checks,
        "compiler_predicate_binding_schema",
        artifacts / "metadata/compiler-predicates-binding.tsv",
        {"key", "value"},
    )
    predicate_binding = {row.get("key", ""): row.get("value", "") for row in predicate_binding_rows}
    check(
        checks,
        "compiler_predicate_binding",
        predicate_binding.get("compiler_predicates_sha256") == digest(inventory_path)
        and predicate_binding.get("compiler_predicates_validation_sha256")
        == digest(predicate_validation_path)
        and predicate_binding.get("compiler_predicates_schema_version") == "compiler-predicates-v1"
        and predicate_binding.get("compiler_predicates_count") == str(len(predicate_rows))
        and predicate_binding.get("compiler_predicates_x86_64_count") == str(predicate_counts["x86_64"])
        and predicate_binding.get("compiler_predicates_aarch64_count") == str(predicate_counts["aarch64"])
        and predicate_binding.get("compiler_predicates_validation_status") == "PASS",
        predicate_binding,
    )

    scope = required_fields(
        checks,
        "scope_schema",
        artifacts / "SCOPE.tsv",
        {
            "id", "linux_path", "destination_path", "class", "architectures",
            "kconfig_evidence", "kbuild_target", "dependencies", "source_kind",
            "metadata_status", "metadata_evidence", "semantic_status",
        },
    )
    fmap = required_fields(
        checks,
        "file_map_schema",
        artifacts / "FILE_MAP.tsv",
        {"source_path", "object_path", "architecture", "module_or_builtin", "kbuild_owner", "disposition_evidence", "compile_command"},
    )
    symbols = required_fields(
        checks,
        "symbols_schema",
        artifacts / "SYMBOLS.tsv",
        {"scope_id", "linux_path", "architectures", "record_kind", "symbol_name", "source_line", "selection_expression", "config_evidence", "linkage", "declaration", "mechanical_value", "evidence", "status"},
    )
    abi = required_fields(
        checks,
        "abi_schema",
        artifacts / "ABI.tsv",
        {"scope_id", "linux_path", "architectures", "record_kind", "symbol_name", "source_line", "abi_item", "linkage", "export_kind", "declaration", "layout", "alignment", "calling_convention", "config_evidence", "evidence", "status"},
    )
    lifetimes = required_fields(
        checks,
        "lifetimes_schema",
        artifacts / "LIFETIMES.tsv",
        {"scope_id", "linux_path", "architectures", "record_kind", "symbol_name", "source_line", "lifetime_item", "storage_duration", "ownership", "lifetime_contract", "locking_rcu_refcount", "config_evidence", "evidence", "status"},
    )
    driver_abi = required_fields(
        checks,
        "driver_abi_schema",
        artifacts / "DRIVER_ABI.tsv",
        {"scope_id", "linux_path", "architectures", "object_path", "kbuild_owner", "module_or_builtin", "record_kind", "abi_item", "evidence", "status"},
    )
    branding_allowlist = required_fields(
        checks,
        "branding_allowlist_schema",
        artifacts / "BRANDING_ALLOWLIST.tsv",
        {"linux_name", "lupos_name", "reason", "evidence"},
    )
    semantic_schema = required_fields(
        checks,
        "semantic_closure_schema",
        artifacts / "semantic-closure/SCHEMA.tsv",
        set(semantic_closure.SCHEMA_FIELDS),
    )
    semantic_base = required_fields(
        checks,
        "semantic_closure_base_schema",
        artifacts / "semantic-closure/BASE.tsv",
        set(semantic_closure.BASE_FIELDS),
    )
    semantic_validation_error = ""
    semantic_values: dict[str, str] = {}
    try:
        semantic_values = semantic_closure.validate_phase0_artifacts(artifacts)
    except SystemExit as exc:
        semantic_validation_error = str(exc)
    check(
        checks,
        "semantic_closure_stable_keys_and_base",
        bool(semantic_schema) and bool(semantic_base) and not semantic_validation_error,
        semantic_validation_error or (
            f"tasks={semantic_values.get('task_count')}; "
            f"pending_fields={semantic_values.get('pending_field_count')}; "
            f"keyset={semantic_values.get('task_keyset_sha256')}"
        ),
    )
    seal_contract_ok = False
    seal_contract_detail: object = "not run"
    try:
        seal_contract_ok, seal_contract_detail = (
            validate_semantic_proposal_seal_contract(artifacts)
        )
    except (OSError, ValueError, SystemExit) as exc:
        seal_contract_detail = f"{type(exc).__name__}: {exc}"
    check(
        checks,
        "semantic_closure_proposal_seal_contract",
        seal_contract_ok,
        seal_contract_detail,
    )
    check(
        checks,
        "semantic_base_pending_permitted",
        semantic_values.get("pending_field_count")
        == str(sum(
            value == "PENDING_REVIEW"
            for records in (scope, symbols, abi, lifetimes)
            for row in records
            if (
                ("id" in row and row.get("class") == "RUST_TRANSLATE")
                or ("scope_id" in row)
            )
            for value in row.values()
        )),
        f"frozen_pending={semantic_values.get('pending_field_count')}",
    )
    check(
        checks,
        "branding_allowlist_complete",
        len({row.get("linux_name", "") for row in branding_allowlist})
        == len(branding_allowlist)
        and all(
            row.get("linux_name") and row.get("lupos_name")
            and row.get("reason") and row.get("evidence")
            for row in branding_allowlist
        ),
        f"rows={len(branding_allowlist)}",
    )
    check(
        checks,
        "porting_guidance_present",
        (artifacts / "PORTING.md").is_file()
        and (artifacts / "PORTING.md").stat().st_size > 0,
        artifacts / "PORTING.md",
    )
    header_closure = required_fields(
        checks,
        "header_closure_schema",
        artifacts / "metadata/header_closure.tsv",
        HEADER_CLOSURE_FIELDS,
    )
    header_include_edges = required_fields(
        checks,
        "header_include_edges_schema",
        artifacts / "metadata/header_include_edges.tsv",
        HEADER_INCLUDE_EDGE_FIELDS,
    )
    header_context_edges = required_fields(
        checks,
        "header_context_edges_schema",
        artifacts / "metadata/header_context_edges.tsv",
        HEADER_CONTEXT_EDGE_FIELDS,
    )
    header_components = required_fields(
        checks,
        "header_components_schema",
        artifacts / "metadata/header_components.tsv",
        {"component_id", "member_path", "member_order", "component_size", "tail_path"},
    )
    task_dependencies = required_fields(
        checks,
        "task_dependencies_schema",
        artifacts / "metadata/task_dependencies.tsv",
        {
            "task_id", "linux_path", "dependency_task_id", "dependency_linux_path",
            "reason", "evidence",
        },
    )
    oracle_classification = required_fields(
        checks,
        "oracle_classification_schema",
        artifacts / "metadata/oracle_classification.tsv",
        ORACLE_CLASSIFICATION_FIELDS,
    )

    scope_by_id = {row.get("id", ""): row for row in scope}
    scope_by_path = {row.get("linux_path", ""): row for row in scope}
    check(
        checks,
        "scope_classified",
        len(scope) == len(scope_by_id) == len(scope_by_path)
        and all(row.get("class") and row.get("kconfig_evidence") and row.get("kbuild_target") for row in scope),
        f"rows={len(scope)} ids={len(scope_by_id)} paths={len(scope_by_path)}",
    )
    scope_evidence_errors = []
    for row in scope:
        for arch in expected_arches(row.get("architectures", "")):
            marker = f"config:{arch}=rewrite/configs/{arch}/frozen.config;disposition="
            if marker not in row.get("kconfig_evidence", "") or f"{arch}:" not in row.get("kbuild_target", ""):
                scope_evidence_errors.append(f"{row.get('id')}:{arch}")
    check(checks, "scope_arch_config_evidence", not scope_evidence_errors, scope_evidence_errors[:20])
    fmap_keys = {(row.get("architecture", ""), row.get("source_path", ""), row.get("object_path", "")) for row in fmap}
    fmap_by_key = {
        (row.get("architecture", ""), row.get("source_path", ""), row.get("object_path", "")): row
        for row in fmap
    }
    check(checks, "file_map_unique", len(fmap_keys) == len(fmap), f"rows={len(fmap)} keys={len(fmap_keys)}")
    check(
        checks,
        "scope_in_file_map",
        set(scope_by_path) <= {row.get("source_path", "") for row in fmap},
        sorted(set(scope_by_path) - {row.get("source_path", "") for row in fmap})[:10],
    )
    owners_by_source: dict[str, set[str]] = defaultdict(set)
    for row in fmap:
        owners_by_source[row.get("source_path", "")].add(row.get("kbuild_owner", ""))
    expected_oracle_reasons: dict[str, str] = {}
    direct_oracle_errors: list[str] = []
    for source in scope:
        if source.get("source_kind") in {"header", "generated_header"}:
            continue
        path = source.get("linux_path", "")
        reason, ownership_override = expected_oracle_path_rule(path)
        driver_owned = expected_driver_owned_source(path, owners_by_source[path])
        expected_reason = reason if reason and (ownership_override or not driver_owned) else ""
        if expected_reason:
            expected_oracle_reasons[path] = expected_reason
            if source.get("class") != "ORACLE_ONLY":
                direct_oracle_errors.append(
                    f"{path}:class={source.get('class')}/ORACLE_ONLY:reason={expected_reason}"
                )
        elif source.get("class") == "ORACLE_ONLY":
            direct_oracle_errors.append(f"{path}:unexpected-oracle")
        if driver_owned and reason and not ownership_override and source.get("class") != "LINUX_DRIVER_OBJECT":
            direct_oracle_errors.append(
                f"{path}:driver-test-name-class={source.get('class')}/LINUX_DRIVER_OBJECT"
            )
    check(
        checks,
        "direct_oracle_classification_exact",
        not direct_oracle_errors,
        f"expected={len(expected_oracle_reasons)} errors={direct_oracle_errors[:20]}",
    )
    rust_destinations = [
        row.get("destination_path", "") for row in scope if row.get("class") == "RUST_TRANSLATE"
    ]
    destination_errors = [
        row.get("id", "") for row in scope
        if row.get("class") == "RUST_TRANSLATE"
        and (
            not row.get("destination_path", "").startswith("src/")
            or not row.get("destination_path", "").endswith(".rs")
        )
    ]
    check(
        checks,
        "rust_destination_unique",
        not destination_errors and len(rust_destinations) == len(set(rust_destinations)),
        f"rows={len(rust_destinations)} unique={len(set(rust_destinations))} malformed={destination_errors[:20]}",
    )

    disposition_errors = []
    fmap_by_arch_source: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    for row in fmap:
        fmap_by_arch_source[(row.get("architecture", ""), row.get("source_path", ""))].append(row)
        source = scope_by_path.get(row.get("source_path", ""))
        if not source or source.get("class") == "BUILD_METADATA":
            continue
        mode = row.get("module_or_builtin", "")
        try:
            command_tokens = shlex.split(row.get("compile_command", ""))
        except ValueError:
            command_tokens = []
            disposition_errors.append(f"{row.get('source_path')}:unparseable-command")
        is_module_compile = "-DMODULE" in command_tokens or any(token.startswith("-DMODULE=") for token in command_tokens)
        if mode not in {"module", "built-in"}:
            disposition_errors.append(f"{row.get('architecture')}:{row.get('object_path')}:unresolved={mode}")
        elif is_module_compile != (mode == "module"):
            disposition_errors.append(
                f"{row.get('architecture')}:{row.get('object_path')}:command_module={is_module_compile}:recorded={mode}"
            )
        if not row.get("kbuild_owner") or not row.get("disposition_evidence"):
            disposition_errors.append(f"{row.get('architecture')}:{row.get('object_path')}:missing-owner-evidence")
    check(checks, "kbuild_disposition_consistent", not disposition_errors, disposition_errors[:20])

    inventory_errors = []
    for arch in ARCHES:
        inventory_path = artifacts / f"metadata/{arch}/object_inventory.tsv"
        inventory = required_fields(
            checks,
            f"{arch}_object_inventory_schema",
            inventory_path,
            {"architecture", "source_path", "object_path", "module_or_builtin", "kbuild_owner", "disposition_evidence"},
        )
        inventory_map = {
            (row.get("architecture", ""), row.get("source_path", ""), row.get("object_path", "")): row
            for row in inventory
        }
        direct_keys = (
            item for item in fmap_keys
            if item[0] == arch
            and scope_by_path.get(item[1], {}).get("source_kind") not in {"header", "generated_header"}
        )
        for key in direct_keys:
            expected = inventory_map.get(key)
            actual = fmap_by_key.get(key)
            if expected is None or actual is None:
                inventory_errors.append(f"{key}:missing")
            elif any(expected.get(field) != actual.get(field) for field in ("module_or_builtin", "kbuild_owner", "disposition_evidence")):
                inventory_errors.append(f"{key}:contradictory")
    check(checks, "object_inventory_matches_file_map", not inventory_errors, inventory_errors[:20])

    # Independently replay every direct compiler dependency assignment.  This
    # is the mechanical proof that the selected transitive header closure is
    # complete; no generated closure row is accepted on trust.
    linux_root = root / "vendor/linux"
    expected_header_stats: dict[tuple[str, str], dict[str, object]] = {}
    expected_header_kinds: dict[str, str] = {}
    expected_header_arches: dict[str, set[str]] = defaultdict(set)
    expected_consumer_classes: dict[str, set[str]] = defaultdict(set)
    rust_headers_by_source: dict[str, set[str]] = defaultdict(set)
    expected_headers_by_context: dict[
        tuple[str, str, str], list[tuple[str, str]]
    ] = {}
    expected_header_consumers: dict[
        tuple[str, str], set[tuple[str, str, str]]
    ] = defaultdict(set)
    direct_context_by_key: dict[tuple[str, str, str], dict[str, str]] = {}
    header_replay_errors: list[str] = []
    direct_fmap = [
        row for row in fmap
        if scope_by_path.get(row.get("source_path", ""), {}).get("source_kind")
        not in {"header", "generated_header"}
    ]
    for item in direct_fmap:
        arch = item.get("architecture", "")
        source_path = item.get("source_path", "")
        source = scope_by_path.get(source_path, {})
        context_key = (arch, source_path, item.get("object_path", ""))
        try:
            dependencies = dependency_headers(
                canonical / f"kbuild/{arch}", linux_root, arch, item.get("object_path", "")
            )
        except Exception as exc:
            header_replay_errors.append(f"{arch}:{source_path}:{item.get('object_path')}:{type(exc).__name__}:{exc}")
            continue
        expected_headers_by_context[context_key] = dependencies
        direct_context_by_key[context_key] = item
        consumer_class = source.get("class", "")
        for header_path, kind in dependencies:
            key = (arch, header_path)
            stats = expected_header_stats.setdefault(
                key,
                {"consumer_count": 0, "rust_consumer_count": 0, "consumer_classes": set()},
            )
            stats["consumer_count"] = int(stats["consumer_count"]) + 1
            stats["rust_consumer_count"] = int(stats["rust_consumer_count"]) + int(consumer_class == "RUST_TRANSLATE")
            classes = stats["consumer_classes"]
            assert isinstance(classes, set)
            classes.add(consumer_class)
            expected_consumer_classes[header_path].add(consumer_class)
            expected_header_arches[header_path].add(arch)
            prior_kind = expected_header_kinds.setdefault(header_path, kind)
            if prior_kind != kind:
                header_replay_errors.append(f"{header_path}:origin={prior_kind}/{kind}")
            expected_header_consumers[(arch, header_path)].add(context_key)
            if consumer_class == "RUST_TRANSLATE":
                rust_headers_by_source[source_path].add(header_path)
    check(
        checks,
        "header_dependency_replay",
        not header_replay_errors,
        f"direct_contexts={len(direct_fmap)} headers={len(expected_header_kinds)} errors={header_replay_errors[:20]}",
    )

    closure_by_key: dict[tuple[str, str], dict[str, str]] = {}
    closure_duplicate_keys: list[tuple[str, str]] = []
    for row in header_closure:
        key = (row.get("architecture", ""), row.get("header_path", ""))
        if key in closure_by_key:
            closure_duplicate_keys.append(key)
        closure_by_key[key] = row
    closure_errors: list[str] = []
    for key, stats in expected_header_stats.items():
        arch, header_path = key
        actual = closure_by_key.get(key)
        expected_class = expected_header_class(
            header_path, expected_header_kinds[header_path], expected_consumer_classes[header_path]
        )
        classes = stats["consumer_classes"]
        assert isinstance(classes, set)
        expected_values = {
            "header_kind": expected_header_kinds[header_path],
            "class": expected_class,
            "consumer_count": str(stats["consumer_count"]),
            "rust_consumer_count": str(stats["rust_consumer_count"]),
            "consumer_classes": ",".join(sorted(classes)),
        }
        if actual is None:
            closure_errors.append(f"{arch}:{header_path}:missing")
        else:
            changed = [field for field, value in expected_values.items() if actual.get(field) != value]
            if changed:
                closure_errors.append(f"{arch}:{header_path}:changed={changed}")
    extra_closure = sorted(set(closure_by_key) - set(expected_header_stats))
    closure_errors.extend(f"{arch}:{path}:extra" for arch, path in extra_closure[:20])
    check(
        checks,
        "header_closure_exact",
        not closure_duplicate_keys and not closure_errors,
        f"expected={len(expected_header_stats)} actual={len(closure_by_key)} duplicates={closure_duplicate_keys[:20]} errors={closure_errors[:20]}",
    )

    expected_header_paths = set(expected_header_kinds)
    scoped_header_paths = {
        row.get("linux_path", "") for row in scope
        if row.get("source_kind") in {"header", "generated_header"}
    }
    header_scope_errors: list[str] = []
    for header_path in sorted(expected_header_paths):
        source = scope_by_path.get(header_path)
        if source is None:
            header_scope_errors.append(f"{header_path}:missing")
            continue
        expected_class = expected_header_class(
            header_path, expected_header_kinds[header_path], expected_consumer_classes[header_path]
        )
        expected_kind = "header" if expected_header_kinds[header_path] == "linux" else "generated_header"
        arches = expected_header_arches[header_path]
        expected_architecture = "common" if arches == set(ARCHES) else next(iter(arches))
        if (
            source.get("class") != expected_class
            or source.get("source_kind") != expected_kind
            or source.get("architectures") != expected_architecture
            or source.get("metadata_status") != "COMPLETE"
        ):
            header_scope_errors.append(
                f"{header_path}:class={source.get('class')}/{expected_class}:"
                f"kind={source.get('source_kind')}/{expected_kind}:arch={source.get('architectures')}/{expected_architecture}:"
                f"metadata={source.get('metadata_status')}"
            )
        if expected_kind == "header" and not (linux_root / header_path).is_file():
            header_scope_errors.append(f"{header_path}:missing-pinned-file")
    extra_scoped_headers = sorted(scoped_header_paths - expected_header_paths)
    header_scope_errors.extend(f"{path}:extra-scope-header" for path in extra_scoped_headers[:20])
    check(
        checks,
        "header_scope_complete",
        scoped_header_paths == expected_header_paths and not header_scope_errors,
        f"expected={len(expected_header_paths)} scoped={len(scoped_header_paths)} errors={header_scope_errors[:20]}",
    )

    for header_path in sorted(expected_header_paths):
        expected_class, reason = expected_header_classification(
            header_path,
            expected_header_kinds[header_path],
            expected_consumer_classes[header_path],
        )
        if expected_class == "ORACLE_ONLY":
            expected_oracle_reasons[header_path] = reason
    expected_oracle_rows = {
        path: {
            "linux_path": path,
            "source_kind": scope_by_path[path].get("source_kind", ""),
            "reason": reason,
            "evidence": (
                f"vendor/linux/{path};"
                f"{scope_by_path[path].get('metadata_evidence', '')};"
                f"classification_rule={reason}"
            ),
        }
        for path, reason in sorted(expected_oracle_reasons.items())
    }
    actual_oracle_rows: dict[str, dict[str, str]] = {}
    oracle_manifest_errors: list[str] = []
    for row in oracle_classification:
        path = row.get("linux_path", "")
        if path in actual_oracle_rows:
            oracle_manifest_errors.append(f"{path}:duplicate")
        actual_oracle_rows[path] = row
    for path in sorted(set(expected_oracle_rows) | set(actual_oracle_rows)):
        expected = expected_oracle_rows.get(path)
        actual = actual_oracle_rows.get(path)
        if expected is None:
            oracle_manifest_errors.append(f"{path}:extra")
        elif actual is None:
            oracle_manifest_errors.append(f"{path}:missing")
        else:
            changed = [
                field for field, value in expected.items()
                if actual.get(field) != value
            ]
            if changed:
                oracle_manifest_errors.append(f"{path}:changed={changed}")
    actual_oracle_scope = {
        row.get("linux_path", "") for row in scope
        if row.get("class") == "ORACLE_ONLY"
    }
    expected_oracle_paths = set(expected_oracle_rows)
    oracle_scope_errors = []
    for path in sorted(expected_oracle_paths | actual_oracle_scope):
        source = scope_by_path.get(path, {})
        if path not in expected_oracle_paths:
            oracle_scope_errors.append(f"{path}:unexpected-scope-oracle")
        elif path not in actual_oracle_scope:
            oracle_scope_errors.append(f"{path}:not-oracle")
        if source.get("destination_path"):
            oracle_scope_errors.append(
                f"{path}:forbidden-destination={source.get('destination_path')}"
            )
    check(
        checks,
        "oracle_classification_exact",
        not oracle_manifest_errors and not oracle_scope_errors,
        f"expected={len(expected_oracle_paths)} manifest_errors="
        f"{oracle_manifest_errors[:20]} scope_errors={oracle_scope_errors[:20]}",
    )
    fmap_paths = {row.get("source_path", "") for row in fmap}
    check(
        checks,
        "oracle_file_map_inventory_nonrust",
        expected_oracle_paths <= fmap_paths
        and all(scope_by_path[path].get("class") == "ORACLE_ONLY" for path in expected_oracle_paths),
        f"oracle={len(expected_oracle_paths)} missing="
        f"{sorted(expected_oracle_paths - fmap_paths)[:20]}",
    )
    oracle_scope_ids = {
        scope_by_path[path].get("id", "") for path in expected_oracle_paths
    }
    semantic_oracle_rows = [
        (name, row.get("scope_id", ""))
        for name, records in (
            ("SYMBOLS", symbols), ("ABI", abi), ("LIFETIMES", lifetimes),
            ("DRIVER_ABI", driver_abi),
        )
        for row in records
        if row.get("scope_id", "") in oracle_scope_ids
    ]
    check(
        checks,
        "oracle_excluded_from_semantic_manifests",
        not semantic_oracle_rows,
        semantic_oracle_rows[:20],
    )

    # Reconstruct literal include resolution for Linux and generated headers
    # from the retained architecture-specific compiler contexts.  Generated
    # wrappers remain graph vertices even though they are not queue tasks.
    rust_header_paths = {
        path for path in expected_header_paths
        if expected_header_class(
            path, expected_header_kinds[path], expected_consumer_classes[path]
        ) == "RUST_TRANSLATE"
    }
    all_header_graph: dict[str, set[str]] = {
        path: set() for path in expected_header_paths
    }
    all_header_graph_by_arch: dict[str, dict[str, set[str]]] = {
        arch: {path: set() for path in expected_header_paths} for arch in ARCHES
    }
    expected_include_rows: dict[tuple[str, str, str], dict[str, str]] = {}
    include_resolution_errors: list[str] = []
    for arch, including_header in sorted(expected_header_stats):
        contexts = sorted(
            key for key in expected_header_consumers[(arch, including_header)]
            if scope_by_path.get(key[1], {}).get("class") == "RUST_TRANSLATE"
        )
        if not contexts:
            continue
        build = canonical / f"kbuild/{arch}"
        try:
            header_file = selected_header_file(
                including_header, linux_root, build, arch
            )
            text_value = header_file.read_text(encoding="utf-8", errors="replace")
        except Exception as exc:
            include_resolution_errors.append(
                f"{arch}:{including_header}:{type(exc).__name__}:{exc}"
            )
            continue
        representative = contexts[0]
        context_item = direct_context_by_key[representative]
        for match in INCLUDE_RE.finditer(text_value):
            delimiter, include_value = match.groups()
            resolved = resolve_include(
                including_header,
                delimiter,
                include_value,
                context_item.get("compile_command", ""),
                linux_root,
                build,
                arch,
            )
            if resolved is None:
                continue
            included_header, included_kind = resolved
            if included_header not in expected_header_paths:
                continue
            shared = sorted(
                expected_header_consumers[(arch, including_header)]
                & expected_header_consumers.get((arch, included_header), set())
                & set(contexts)
            )
            if not shared:
                continue
            if expected_header_kinds.get(included_header) != included_kind:
                include_resolution_errors.append(
                    f"{arch}:{including_header}->{included_header}:kind="
                    f"{included_kind}/{expected_header_kinds.get(included_header)}"
                )
                continue
            all_header_graph[including_header].add(included_header)
            all_header_graph_by_arch[arch][including_header].add(included_header)
            edge_key = (arch, including_header, included_header)
            if edge_key in expected_include_rows:
                continue
            witness = shared[0]
            line = text_value.count("\n", 0, match.start()) + 1
            evidence_root = (
                f"rewrite/kbuild/{arch}/{including_header[len(f'generated/{arch}/'):]}"
                if including_header.startswith(f"generated/{arch}/")
                else f"vendor/linux/{including_header}"
            )
            expected_include_rows[edge_key] = {
                "including_kind": expected_header_kinds[including_header],
                "included_kind": included_kind,
                "relationship": "literal_include",
                "directive": f"{include_value}@{line}",
                "consumer_source": witness[1],
                "consumer_object": witness[2],
                "evidence": (
                    f"{evidence_root}:{line};"
                    f"rewrite/kbuild/{arch}/"
                    f"{command_evidence_path(build, witness[2]).relative_to(build).as_posix()}"
                ),
            }

    actual_include_rows: dict[tuple[str, str, str], dict[str, str]] = {}
    include_edge_errors: list[str] = []
    for row in header_include_edges:
        key = (
            row.get("architecture", ""),
            row.get("including_header", ""),
            row.get("included_header", ""),
        )
        if key in actual_include_rows:
            include_edge_errors.append(f"{key}:duplicate")
        actual_include_rows[key] = row
    for key in sorted(set(expected_include_rows) | set(actual_include_rows)):
        expected = expected_include_rows.get(key)
        actual = actual_include_rows.get(key)
        if expected is None:
            include_edge_errors.append(f"{key}:extra")
        elif actual is None:
            include_edge_errors.append(f"{key}:missing")
        else:
            changed = [
                field for field, value in expected.items()
                if actual.get(field) != value
            ]
            if changed:
                include_edge_errors.append(f"{key}:changed={changed}")
    check(
        checks,
        "header_literal_include_resolution_exact",
        not include_resolution_errors and not include_edge_errors,
        f"expected={len(expected_include_rows)} actual={len(actual_include_rows)} "
        f"resolver_errors={include_resolution_errors[:20]} edge_errors={include_edge_errors[:20]}",
    )

    direct_rust_header_graph = project_rust_header_dependencies(
        all_header_graph, rust_header_paths
    )
    direct_rust_header_graph_by_arch = {
        arch: project_rust_header_dependencies(graph, rust_header_paths)
        for arch, graph in all_header_graph_by_arch.items()
    }
    direct_rust_reachability_by_arch = {
        arch: {
            path: reachable(graph, path) for path in sorted(rust_header_paths)
        }
        for arch, graph in direct_rust_header_graph_by_arch.items()
    }
    definitions_by_header: dict[str, set[str]] = {
        path: set() for path in rust_header_paths
    }
    expected_enum_constants_by_header = {
        path: independent_enum_constants(linux_root / path)
        for path in sorted(rust_header_paths)
    }
    for row in symbols:
        path = row.get("linux_path", "")
        if path not in definitions_by_header:
            continue
        name = row.get("symbol_name", "")
        kind = row.get("record_kind", "")
        if IDENTIFIER.fullmatch(name) and kind == "operative_macro":
            definitions_by_header[path].add(f"macro:{name}")
        elif IDENTIFIER.fullmatch(name) and kind in {"type", "function", "function_macro"}:
            definitions_by_header[path].add(f"identifier:{name}")
        else:
            tag = re.fullmatch(
                r"(?:struct|union|enum)\s+([A-Za-z_][A-Za-z0-9_]*)", name
            )
            if tag:
                definitions_by_header[path].add(
                    f"{name.split(None, 1)[0]}:{tag.group(1)}"
                )
    for path, constants in expected_enum_constants_by_header.items():
        definitions_by_header[path].update(
            f"identifier:{name}" for name, _, _ in constants
        )
    definition_names = set().union(*definitions_by_header.values())
    unresolved_references_by_arch: dict[str, dict[str, set[str]]] = {
        arch: {} for arch in ARCHES
    }
    for path in sorted(rust_header_paths):
        references = header_reference_identifiers(
            (linux_root / path).read_text(errors="replace"), definition_names,
        ) - definitions_by_header[path]
        for arch in ARCHES:
            directly_available = set(definitions_by_header[path])
            for dependency in direct_rust_reachability_by_arch[arch][path]:
                directly_available.update(definitions_by_header[dependency])
            unresolved_references_by_arch[arch][path] = references - directly_available
    expected_context_records: dict[
        tuple[str, str, str], tuple[dict[str, str], set[str]]
    ] = {}
    expected_context_pairs: set[tuple[str, str]] = set()
    expected_header_graph = {
        path: set(dependencies)
        for path, dependencies in direct_rust_header_graph.items()
    }
    for context_key in sorted(expected_headers_by_context):
        arch, consumer_source, consumer_object = context_key
        if scope_by_path.get(consumer_source, {}).get("class") != "RUST_TRANSLATE":
            continue
        build = canonical / f"kbuild/{arch}"
        context = direct_context_by_key[context_key]
        forced_roots = forced_include_headers(
            context.get("compile_command", ""), linux_root, build, arch,
        )
        forced_provider_paths: set[str] = set()
        for forced_root in forced_roots:
            if forced_root in rust_header_paths:
                forced_provider_paths.add(forced_root)
                forced_provider_paths.update(
                    direct_rust_reachability_by_arch[arch][forced_root]
                )
        last_definition: dict[str, tuple[str, int]] = {}
        for position, (header_path, _) in enumerate(
            expected_headers_by_context[context_key], 1
        ):
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
            reduced_candidates = {
                provider: value for provider, value in candidates.items()
                if not any(
                    provider != other
                    and provider in direct_rust_reachability_by_arch[arch][other]
                    and other not in direct_rust_reachability_by_arch[arch][provider]
                    for other in candidates
                )
            }
            for provider, (provider_position, names) in sorted(reduced_candidates.items()):
                expected_header_graph[header_path].add(provider)
                expected_context_pairs.add((header_path, provider))
                edge_key = (arch, header_path, provider)
                record = {
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
                        f"rewrite/kbuild/{arch}/"
                        f"{command_evidence_path(build, consumer_object).relative_to(build).as_posix()};"
                        f"ordered_dependency_positions={provider_position}<{position}"
                    ),
                }
                if edge_key not in expected_context_records:
                    expected_context_records[edge_key] = (record, set(names))
                else:
                    expected_context_records[edge_key][1].update(names)
            for name in definitions_by_header[header_path]:
                last_definition[name] = (header_path, position)

    expected_context_rows: dict[tuple[str, str, str], dict[str, str]] = {}
    for edge_key in sorted(expected_context_records):
        record, names = expected_context_records[edge_key]
        record["provided_identifiers"] = ",".join(sorted(names))
        expected_context_rows[edge_key] = record

    actual_context_rows: dict[tuple[str, str, str], dict[str, str]] = {}
    context_edge_errors: list[str] = []
    for row in header_context_edges:
        key = (
            row.get("architecture", ""),
            row.get("header_path", ""),
            row.get("provider_header", ""),
        )
        if key in actual_context_rows:
            context_edge_errors.append(f"{key}:duplicate")
        actual_context_rows[key] = row
    for key in sorted(set(expected_context_rows) | set(actual_context_rows)):
        expected = expected_context_rows.get(key)
        actual = actual_context_rows.get(key)
        if expected is None:
            context_edge_errors.append(f"{key}:extra")
        elif actual is None:
            context_edge_errors.append(f"{key}:missing")
        else:
            changed = [
                field for field, value in expected.items()
                if actual.get(field) != value
            ]
            if changed:
                context_edge_errors.append(f"{key}:changed={changed}")
    check(
        checks,
        "header_context_provider_edges_exact",
        not context_edge_errors,
        f"expected={len(expected_context_rows)} actual={len(actual_context_rows)} "
        f"errors={context_edge_errors[:20]}",
    )
    netfilter_provider_edge = actual_context_rows.get((
        "x86_64",
        "include/uapi/linux/netfilter/xt_state.h",
        "include/uapi/linux/netfilter/nf_conntrack_common.h",
    ), {})
    netfilter_provider_identifiers = set(
        value for value in netfilter_provider_edge.get("provided_identifiers", "").split(",")
        if value
    )
    check(
        checks,
        "xt_state_conntrack_enum_provider",
        {
            "identifier:IP_CT_IS_REPLY",
            "identifier:IP_CT_NUMBER",
        } <= netfilter_provider_identifiers,
        netfilter_provider_edge,
    )

    rust_scope_ids = {
        row.get("id", "") for row in scope if row.get("class") == "RUST_TRANSLATE"
    }
    dependency_pairs_from_scope: set[tuple[str, str]] = set()
    dependency_field_errors: list[str] = []
    for source in (row for row in scope if row.get("class") == "RUST_TRANSLATE"):
        values = [value for value in source.get("dependencies", "").split(";") if value]
        if len(values) != len(set(values)):
            dependency_field_errors.append(f"{source.get('id')}:duplicate")
        for dependency_id in values:
            if dependency_id not in rust_scope_ids or dependency_id == source.get("id"):
                dependency_field_errors.append(f"{source.get('id')}:{dependency_id}:invalid")
            dependency_pairs_from_scope.add((source.get("id", ""), dependency_id))
    dependency_pairs_from_metadata: set[tuple[str, str]] = set()
    dependency_metadata_errors: list[str] = []
    graph_by_path: dict[str, set[str]] = defaultdict(set)
    for row in task_dependencies:
        pair = (row.get("task_id", ""), row.get("dependency_task_id", ""))
        if pair in dependency_pairs_from_metadata:
            dependency_metadata_errors.append(f"{pair}:duplicate")
        dependency_pairs_from_metadata.add(pair)
        source = scope_by_id.get(pair[0], {})
        dependency = scope_by_id.get(pair[1], {})
        if (
            source.get("linux_path") != row.get("linux_path")
            or dependency.get("linux_path") != row.get("dependency_linux_path")
            or source.get("class") != "RUST_TRANSLATE"
            or dependency.get("class") != "RUST_TRANSLATE"
            or row.get("reason") not in {
                "header_include_component", "header_context_component",
                "header_provider_component", "header_scc_order",
                "source_header_closure",
            }
            or not row.get("evidence")
        ):
            dependency_metadata_errors.append(f"{pair}:malformed")
        if source and dependency:
            graph_by_path[source.get("linux_path", "")].add(dependency.get("linux_path", ""))
    check(
        checks,
        "task_dependency_records",
        not dependency_field_errors
        and not dependency_metadata_errors
        and dependency_pairs_from_scope == dependency_pairs_from_metadata,
        f"scope={len(dependency_pairs_from_scope)} metadata={len(dependency_pairs_from_metadata)} "
        f"field_errors={dependency_field_errors[:20]} metadata_errors={dependency_metadata_errors[:20]} "
        f"delta={sorted(dependency_pairs_from_scope ^ dependency_pairs_from_metadata)[:20]}",
    )
    xt_state_scope = scope_by_path.get("include/uapi/linux/netfilter/xt_state.h", {})
    conntrack_scope = scope_by_path.get(
        "include/uapi/linux/netfilter/nf_conntrack_common.h", {}
    )
    required_netfilter_pair = (
        xt_state_scope.get("id", ""), conntrack_scope.get("id", "")
    )
    check(
        checks,
        "xt_state_conntrack_task_dependency",
        required_netfilter_pair == ("S016294", "S016270")
        and required_netfilter_pair in dependency_pairs_from_scope
        and required_netfilter_pair in dependency_pairs_from_metadata,
        required_netfilter_pair,
    )
    task_reachability_cache: dict[str, set[str]] = {}

    def task_reachability(path: str) -> set[str]:
        if path not in task_reachability_cache:
            task_reachability_cache[path] = reachable(graph_by_path, path)
        return task_reachability_cache[path]

    component_members: dict[str, list[tuple[int, str]]] = defaultdict(list)
    component_tail_by_id: dict[str, str] = {}
    component_by_header: dict[str, str] = {}
    component_errors: list[str] = []
    rust_header_scope = {
        row.get("linux_path", "") for row in scope
        if row.get("class") == "RUST_TRANSLATE" and row.get("source_kind") == "header"
    }
    for row in header_components:
        component_id = row.get("component_id", "")
        member = row.get("member_path", "")
        try:
            order = int(row.get("member_order", ""))
            size = int(row.get("component_size", ""))
        except ValueError:
            component_errors.append(f"{component_id}:{member}:non-integer")
            continue
        if member in component_by_header:
            component_errors.append(f"{member}:duplicate-component")
        component_by_header[member] = component_id
        component_members[component_id].append((order, member))
        tail = row.get("tail_path", "")
        prior_tail = component_tail_by_id.setdefault(component_id, tail)
        if prior_tail != tail or size <= 0:
            component_errors.append(f"{component_id}:inconsistent")
    for component_id, members in component_members.items():
        ordered = [member for _, member in sorted(members)]
        orders = [order for order, _ in sorted(members)]
        declared_sizes = {
            int(row.get("component_size", "0"))
            for row in header_components
            if row.get("component_id") == component_id
            and row.get("component_size", "").isdigit()
        }
        if (
            orders != list(range(1, len(members) + 1))
            or component_tail_by_id.get(component_id) != ordered[-1]
            or declared_sizes != {len(members)}
        ):
            component_errors.append(f"{component_id}:order-or-tail")
        for previous, member in zip(ordered, ordered[1:]):
            if previous not in graph_by_path.get(member, set()):
                component_errors.append(f"{component_id}:{member}:missing-chain-to:{previous}")
    expected_components = strongly_connected_components(expected_header_graph)
    expected_component_ids = [
        f"HC{index:06d}" for index in range(1, len(expected_components) + 1)
    ]
    actual_component_ids = sorted(component_members)
    actual_components = [
        [member for _, member in sorted(component_members[component_id])]
        for component_id in actual_component_ids
    ]
    if actual_component_ids != expected_component_ids or actual_components != expected_components:
        component_errors.append(
            f"component-replay-mismatch:expected={len(expected_components)}:"
            f"actual={len(actual_components)}"
        )
    check(
        checks,
        "header_components_complete",
        set(component_by_header) == rust_header_scope and not component_errors,
        f"rust_headers={len(rust_header_scope)} component_members={len(component_by_header)} "
        f"delta={sorted(set(component_by_header) ^ rust_header_scope)[:20]} errors={component_errors[:20]}",
    )

    include_dependency_errors: list[str] = []
    for including, dependencies in direct_rust_header_graph.items():
        for included in dependencies:
            if (
                component_by_header.get(including) != component_by_header.get(included)
                and included not in task_reachability(including)
            ):
                include_dependency_errors.append(f"{including}->{included}:not-reachable")
    check(
        checks,
        "header_include_dependency_graph",
        not include_dependency_errors,
        f"projected_edges={sum(map(len, direct_rust_header_graph.values()))} "
        f"errors={include_dependency_errors[:20]}",
    )

    context_dependency_errors: list[str] = []
    for header_path, provider in sorted(expected_context_pairs):
        if (
            component_by_header.get(header_path) != component_by_header.get(provider)
            and provider not in task_reachability(header_path)
        ):
            context_dependency_errors.append(f"{header_path}->{provider}:not-reachable")
    check(
        checks,
        "header_context_dependency_graph",
        not context_dependency_errors,
        f"provider_pairs={len(expected_context_pairs)} errors={context_dependency_errors[:20]}",
    )

    bridged_pairs = {
        (source, dependency)
        for source, dependencies in direct_rust_header_graph.items()
        for dependency in dependencies
        if dependency not in all_header_graph.get(source, set())
    }
    bridge_errors = [
        f"{source}->{dependency}:not-reachable"
        for source, dependency in sorted(bridged_pairs)
        if component_by_header.get(source) != component_by_header.get(dependency)
        and dependency not in task_reachability(source)
    ]
    check(
        checks,
        "generated_wrapper_bridge_reachability",
        bool(bridged_pairs) and not bridge_errors,
        f"bridged_pairs={len(bridged_pairs)} errors={bridge_errors[:20]}",
    )

    provider_graph_errors = []
    for header_path, providers in expected_header_graph.items():
        for provider in providers:
            if (
                component_by_header.get(header_path) != component_by_header.get(provider)
                and provider not in task_reachability(header_path)
            ):
                provider_graph_errors.append(f"{header_path}->{provider}:not-ready")
    check(
        checks,
        "header_provider_resolver_readiness",
        not provider_graph_errors,
        f"rust_headers={len(expected_header_graph)} provider_edges="
        f"{sum(map(len, expected_header_graph.values()))} errors={provider_graph_errors[:20]}",
    )

    closure_dependency_errors: list[str] = []
    for source_path, headers in rust_headers_by_source.items():
        translated = {
            header for header in headers
            if expected_header_class(
                header, expected_header_kinds[header], expected_consumer_classes[header]
            ) == "RUST_TRANSLATE"
        }
        missing = translated - task_reachability(source_path)
        if missing:
            closure_dependency_errors.append(f"{source_path}:missing={sorted(missing)[:10]}")
    check(
        checks,
        "source_header_dependency_reachability",
        not closure_dependency_errors,
        f"rust_sources={len(rust_headers_by_source)} errors={closure_dependency_errors[:20]}",
    )

    dependency_cycle_errors: list[str] = []
    visit_state: dict[str, int] = {}
    for source_path in sorted(graph_by_path):
        if visit_state.get(source_path, 0) != 0 or dependency_cycle_errors:
            continue
        visit_state[source_path] = 1
        trail = [source_path]
        stack: list[tuple[str, list[str], int]] = [
            (source_path, sorted(graph_by_path.get(source_path, set())), 0)
        ]
        while stack and not dependency_cycle_errors:
            node, dependencies, index = stack[-1]
            if index == len(dependencies):
                visit_state[node] = 2
                stack.pop()
                trail.pop()
                continue
            dependency = dependencies[index]
            stack[-1] = (node, dependencies, index + 1)
            state = visit_state.get(dependency, 0)
            if state == 2:
                continue
            if state == 1:
                dependency_cycle_errors.append(
                    " -> ".join([*trail, dependency])
                )
                continue
            visit_state[dependency] = 1
            trail.append(dependency)
            stack.append((
                dependency,
                sorted(graph_by_path.get(dependency, set())),
                0,
            ))
    check(checks, "task_dependency_acyclic", not dependency_cycle_errors, dependency_cycle_errors[:1])

    symbols_by_scope_arch: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    placeholder_rows = []
    malformed_symbol_rows = []
    for row in symbols:
        key = (row.get("scope_id", ""), row.get("architectures", ""))
        symbols_by_scope_arch[key].append(row)
        if row.get("record_kind") == "mechanical_file_record" or row.get("symbol_name") == "PENDING_REVIEW":
            placeholder_rows.append(key)
        arch = row.get("architectures", "")
        expected_config = CONFIG_EVIDENCE.get(arch, "")
        source_kind = scope_by_id.get(row.get("scope_id", ""), {}).get("source_kind", "")
        expected_status = "PENDING_REVIEW" if source_kind == "header" else "COMPLETE"
        if (
            not row.get("source_line", "").isdigit()
            or not row.get("selection_expression")
            or row.get("config_evidence") != expected_config
            or not row.get("evidence")
            or row.get("status") != expected_status
            or row.get("record_kind") not in {"function", "function_macro", "type", "static", "global", "export", "operative_macro", "conditional", "enum_constant"}
            or (row.get("record_kind") == "conditional" and "selected=" not in row.get("evidence", ""))
        ):
            malformed_symbol_rows.append((key, row.get("record_kind"), row.get("symbol_name")))
    check(checks, "semantic_no_file_placeholders", not placeholder_rows, placeholder_rows[:20])
    check(checks, "symbols_mechanical_fields", not malformed_symbol_rows, malformed_symbol_rows[:20])

    enum_inventory_errors: list[str] = []
    actual_enum_rows: dict[tuple[str, str, str, int], str] = {}
    for row in symbols:
        if row.get("record_kind") != "enum_constant":
            continue
        try:
            line = int(row.get("source_line", ""))
        except ValueError:
            enum_inventory_errors.append(f"{row.get('scope_id')}:invalid-line")
            continue
        key = (
            row.get("linux_path", ""), row.get("architectures", ""),
            row.get("symbol_name", ""), line,
        )
        if key in actual_enum_rows:
            enum_inventory_errors.append(f"{key}:duplicate")
        actual_enum_rows[key] = row.get("mechanical_value", "")
    expected_enum_rows: dict[tuple[str, str, str, int], str] = {}
    for path, constants in expected_enum_constants_by_header.items():
        scope_row = scope_by_path[path]
        for arch in expected_arches(scope_row.get("architectures", "")):
            for name, line, value in constants:
                expected_enum_rows[(path, arch, name, line)] = value
    for key in sorted(set(expected_enum_rows) | {
        item for item in actual_enum_rows if item[0] in rust_header_paths
    }):
        if expected_enum_rows.get(key) != actual_enum_rows.get(key):
            enum_inventory_errors.append(
                f"{key}:expected={expected_enum_rows.get(key)}:actual={actual_enum_rows.get(key)}"
            )
    check(
        checks,
        "header_enum_constant_inventory_exact",
        not enum_inventory_errors,
        f"expected={len(expected_enum_rows)} errors={enum_inventory_errors[:20]}",
    )
    ip_ct_number_values = {
        arch: actual_enum_rows.get((
            "include/uapi/linux/netfilter/nf_conntrack_common.h",
            arch,
            "IP_CT_NUMBER",
            27,
        ))
        for arch in ARCHES
    }
    check(
        checks,
        "ip_ct_number_mechanical_value",
        ip_ct_number_values == {"x86_64": "5", "aarch64": "5"},
        ip_ct_number_values,
    )

    coverage_errors = []
    rust_scope = [row for row in scope if row.get("class") == "RUST_TRANSLATE"]
    for source in rust_scope:
        source_path = root / "vendor/linux" / source["linux_path"]
        categories = source_categories(source_path) if source_path.is_file() else set()
        for arch in expected_arches(source.get("architectures", "")):
            records = symbols_by_scope_arch.get((source["id"], arch), [])
            kinds = {row.get("record_kind", "") for row in records}
            normalized_kinds = set(kinds)
            if kinds & {"function", "function_macro"}:
                normalized_kinds.add("function")
            missing = categories - normalized_kinds
            if missing or (not records and source.get("source_kind") != "header"):
                coverage_errors.append(f"{source['id']}:{arch}:rows={len(records)}:missing={sorted(missing)}")
    check(checks, "symbols_category_arch_coverage", not coverage_errors, coverage_errors[:20])

    abi_keys = {
        (row.get("scope_id"), row.get("architectures"), row.get("record_kind"), row.get("symbol_name"), row.get("source_line"))
        for row in abi
        if row.get("abi_item") not in {"", "PENDING_REVIEW"}
        and row.get("config_evidence") == CONFIG_EVIDENCE.get(row.get("architectures", ""), "")
        and row.get("linkage")
        and row.get("declaration")
        and row.get("layout")
        and row.get("alignment")
        and row.get("calling_convention")
        and row.get("status") in {"COMPLETE", "PENDING_REVIEW"}
    }
    lifetime_keys = {
        (row.get("scope_id"), row.get("architectures"), row.get("record_kind"), row.get("symbol_name"), row.get("source_line"))
        for row in lifetimes
        if row.get("lifetime_item") not in {"", "PENDING_REVIEW"}
        and row.get("config_evidence") == CONFIG_EVIDENCE.get(row.get("architectures", ""), "")
        and row.get("storage_duration")
        and row.get("ownership")
        and row.get("lifetime_contract")
        and row.get("locking_rcu_refcount")
        and row.get("status") in {"COMPLETE", "PENDING_REVIEW"}
    }
    expected_abi = {
        (row.get("scope_id"), row.get("architectures"), row.get("record_kind"), row.get("symbol_name"), row.get("source_line"))
        for row in symbols if row.get("record_kind") in ENTITY_KINDS
    }
    expected_lifetimes = {
        (row.get("scope_id"), row.get("architectures"), row.get("record_kind"), row.get("symbol_name"), row.get("source_line"))
        for row in symbols if row.get("record_kind") in LIFETIME_KINDS
    }
    check(checks, "abi_entity_coverage", expected_abi <= abi_keys, sorted(expected_abi - abi_keys)[:20])
    check(
        checks,
        "lifetime_entity_coverage",
        expected_lifetimes <= lifetime_keys,
        sorted(expected_lifetimes - lifetime_keys)[:20],
    )

    driver_errors = []
    driver_rows_by_object = {
        (row.get("scope_id"), row.get("architectures"), row.get("object_path")): row for row in driver_abi
    }
    for source in (row for row in scope if row.get("class") == "LINUX_DRIVER_OBJECT"):
        for arch in expected_arches(source.get("architectures", "")):
            expected_objects = fmap_by_arch_source.get((arch, source["linux_path"]), [])
            if not expected_objects:
                driver_errors.append(f"{source['id']}:{arch}:no-file-map-object")
            for expected in expected_objects:
                row = driver_rows_by_object.get((source["id"], arch, expected.get("object_path")))
                if not row or not row.get("kbuild_owner") or row.get("module_or_builtin") not in {"module", "built-in"}:
                    driver_errors.append(f"{source['id']}:{arch}:{expected.get('object_path')}:missing-mechanical-contract")
                elif row.get("abi_item") in {"", "PENDING_REVIEW"} or "object=" not in row.get("abi_item", ""):
                    driver_errors.append(f"{source['id']}:{arch}:{expected.get('object_path')}:placeholder-only")
    check(checks, "driver_abi_mechanical_coverage", not driver_errors, driver_errors[:20])

    hash_index = required_fields(
        checks,
        "authoritative_manifest_index_schema",
        artifacts / "metadata/authoritative_manifests.tsv",
        {"path", "sha256"},
    )
    indexed = {row.get("path", ""): row.get("sha256", "") for row in hash_index}
    required_manifests = {
        "SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv",
        "LIFETIMES.tsv", "DRIVER_ABI.tsv", "PORTING.md",
        "BRANDING_ALLOWLIST.tsv", "semantic-closure/SCHEMA.tsv",
        "semantic-closure/BASE.tsv",
    }
    manifest_hash_errors = [
        name for name in sorted(required_manifests)
        if not (artifacts / name).is_file() or indexed.get(name) != digest(artifacts / name)
    ]
    check(checks, "authoritative_manifest_hashes", not manifest_hash_errors, manifest_hash_errors)

    metadata_manifest = required_fields(
        checks,
        "metadata_manifest_schema",
        artifacts / "metadata/manifest.tsv",
        {"path", "sha256"},
    )
    metadata_hash_errors = []
    for row in metadata_manifest:
        path = artifacts / row.get("path", "")
        if not path.is_file() or digest(path) != row.get("sha256"):
            metadata_hash_errors.append(row.get("path", ""))
    check(checks, "metadata_manifest_hashes", not metadata_hash_errors, metadata_hash_errors[:20])

    if args.phase_gate_reopen:
        event_records: list[dict[str, object]] = []
        event_errors: list[str] = []
        try:
            for line_number, line in enumerate(
                (canonical / "events.jsonl").read_text(encoding="utf-8").splitlines(), 1
            ):
                if not line.strip():
                    continue
                record = json.loads(line)
                if not isinstance(record, dict):
                    event_errors.append(f"line={line_number}:not-object")
                    continue
                event_records.append(record)
        except (OSError, json.JSONDecodeError) as exc:
            event_errors.append(f"{type(exc).__name__}:{exc}")
        reopen_events = [
            (index, record) for index, record in enumerate(event_records)
            if record.get("phase") == "phase0"
            and record.get("event") == "queue_invalidated"
            and "mode=phase-gate-reopen" in str(record.get("detail", ""))
        ]
        current_queue_rows = (
            rows(canonical / "TRANSLATION_TASKS.tsv")
            if (canonical / "TRANSLATION_TASKS.tsv").is_file() else []
        )
        active_rows = [
            row.get("id", "") for row in current_queue_rows
            if row.get("status") in {"IN_PROGRESS", "IMPLEMENTED", "REVIEWING", "APPLYING"}
            or row.get("lease_owner") or row.get("lease_expires_at")
        ]
        archives: list[str] = []
        for _, record in reopen_events:
            match = re.search(
                r"(?:^|; )archive=(rewrite/archive/phase0-[^;]+);",
                str(record.get("detail", "")),
            )
            archives.append(match.group(1) if match else "")
        latest_index, latest_reopen = reopen_events[-1] if reopen_events else (-1, {})
        latest_archive = archives[-1] if archives else ""
        consuming = [
            (index, record) for index, record in enumerate(event_records)
            if index > latest_index
            and record.get("phase") == "phase0"
            and record.get("event") == "queue_reinitialized"
            and f"archive={latest_archive};" in str(record.get("detail", ""))
        ]
        consumed_as_expected = (
            len(consuming) == 1 if args.stage == "frozen" else not consuming
        )
        check(
            checks,
            "phase_gate_reopen_authorized",
            bool(reopen_events)
            and all(archives)
            and len(archives) == len(set(archives))
            and not event_errors
            and not active_rows
            and consumed_as_expected,
            f"events={len(reopen_events)} archives={archives[-3:]} "
            f"latest_consumers={len(consuming)} event_errors={event_errors[:5]} "
            f"active={active_rows[:10]}",
        )
        check(
            checks,
            "src_preserved_during_phase_gate_reopen",
            not any(path.is_symlink() for path in (root / "src").rglob("*")),
            root / "src",
        )
        if args.stage == "frozen":
            root_evidence, quarantine_errors, generations = (
                validate_task_evidence_quarantine(canonical / "logs/tasks")
            )
            check(
                checks,
                "task_evidence_root_isolation",
                not root_evidence,
                f"unexpected_root_files={root_evidence[:20]}",
            )
            check(
                checks,
                "task_evidence_quarantine_integrity",
                not quarantine_errors,
                quarantine_errors[:20],
            )
            latest_fingerprint = ""
            quarantine_events: list[dict[str, object]] = []
            reinitialized: dict[str, object] = {}
            if consuming:
                reinitialized_index, reinitialized = consuming[0]
                match = re.search(
                    r"old_queue_sha256=([0-9a-f]{64});",
                    str(reinitialized.get("detail", "")),
                )
                latest_fingerprint = match.group(1) if match else ""
                quarantine_events = [
                    record for index, record in enumerate(event_records)
                    if latest_index < index < reinitialized_index
                    and record.get("phase") == "phase0"
                    and record.get("event") == "task_evidence_quarantined"
                    and f"superseded_fingerprint={latest_fingerprint};"
                    in str(record.get("detail", ""))
                ]
            summary = generations.get(latest_fingerprint, {})
            quarantined_tasks = summary.get("tasks", set())
            event_tasks = {str(record.get("task_id", "")) for record in quarantine_events}
            detail_matches = all(
                f"quarantine=rewrite/logs/tasks/{record.get('task_id', '')}/"
                f"invalidated-generations/{latest_fingerprint}"
                in str(record.get("detail", ""))
                for record in quarantine_events
            )
            declared_counts = re.search(
                r"quarantined_tasks=(\d+); quarantined_files=(\d+);",
                str(reinitialized.get("detail", "")),
            )
            declared_tasks = int(declared_counts.group(1)) if declared_counts else -1
            declared_files = int(declared_counts.group(2)) if declared_counts else -1
            observed_files = int(summary.get("files", 0))
            empty_generation = (
                declared_tasks == 0
                and declared_files == 0
                and not quarantined_tasks
                and not quarantine_events
                and observed_files == 0
            )
            populated_generation = (
                bool(quarantined_tasks)
                and event_tasks == quarantined_tasks
                and len(quarantine_events) == len(quarantined_tasks)
                and declared_tasks == len(quarantined_tasks)
                and declared_files == observed_files
                and detail_matches
            )
            check(
                checks,
                "task_evidence_quarantine_events",
                bool(latest_fingerprint)
                and bool(declared_counts)
                and (empty_generation or populated_generation),
                f"fingerprint={latest_fingerprint} metadata_tasks={len(quarantined_tasks)} "
                f"event_tasks={len(event_tasks)} files={observed_files} "
                f"declared_tasks={declared_tasks} declared_files={declared_files}",
            )
            initial_semantic = semantic_closure.validate_generation_initial_state(
                current_queue_rows,
                canonical / "TRANSLATION_TASKS.sha256",
                canonical / "semantic-closure/LEDGER.jsonl",
            )
            check(
                checks,
                "semantic_closure_new_generation_initial_state",
                bool(initial_semantic.get("ok")),
                initial_semantic,
            )
    else:
        check(
            checks,
            "src_empty_at_first_init",
            not any(path.is_file() for path in (root / "src").rglob("*")),
            root / "src",
        )

    if args.stage == "pre-queue":
        queue_absent = (
            not (artifacts / "TRANSLATION_TASKS.tsv").exists()
            and not (artifacts / "TRANSLATION_TASKS.sha256").exists()
        )
        if args.phase_gate_reopen:
            # The invalidated snapshot remains byte-exact until the queue tool
            # consumes its recorded reopen authorization.
            queue_absent = (
                (artifacts / "TRANSLATION_TASKS.tsv").is_file()
                and (artifacts / "TRANSLATION_TASKS.sha256").is_file()
            )
        check(checks, "queue_absent_before_init", queue_absent, artifacts)
        ledger_records = semantic_closure.validate_ledger(
            artifacts / "semantic-closure/LEDGER.jsonl"
        )
        check(
            checks,
            "semantic_closure_prequeue_clean",
            not any(record.get("record_type") in {"PREPARE", "COMMIT"} for record in ledger_records),
            f"ledger_records={len(ledger_records)}",
        )
    else:
        identity_path = artifacts / "PHASE0_IDENTITY.tsv"
        identity_hash_path = artifacts / "PHASE0_IDENTITY.sha256"
        identity = {row["key"]: row for row in rows(identity_path)} if identity_path.is_file() else {}
        check(
            checks,
            "identity_hash",
            identity_path.is_file()
            and identity_hash_path.is_file()
            and digest(identity_path) == identity_hash_path.read_text().split()[0],
            identity_hash_path,
        )
        for arch in ARCHES:
            key = f"{arch}_config_sha256"
            check(
                checks,
                f"{arch}_config_hash",
                identity.get(key, {}).get("value") == config_hashes[arch],
                f"identity={identity.get(key, {}).get('value')} actual={config_hashes[arch]}",
            )
        extractor_identity = f"phase0_extract.py:{digest(root / 'tools/phase0_extract.py')}"
        check(
            checks,
            "extractor_identity",
            identity.get("extractor_version", {}).get("value") == extractor_identity,
            f"identity={identity.get('extractor_version', {}).get('value')} actual={extractor_identity}",
        )
        validator_identity = f"phase0_validate.py:{digest(root / 'tools/phase0_validate.py')}"
        check(
            checks,
            "validator_identity",
            identity.get("validator_version", {}).get("value") == validator_identity,
            f"identity={identity.get('validator_version', {}).get('value')} actual={validator_identity}",
        )
        queue_tool_identity = f"rewrite_queue.py:{digest(root / 'tools/rewrite_queue.py')}"
        check(
            checks,
            "queue_tool_identity",
            identity.get("queue_tool_version", {}).get("value") == queue_tool_identity,
            f"identity={identity.get('queue_tool_version', {}).get('value')} actual={queue_tool_identity}",
        )
        semantic_tool_identity = f"semantic_closure.py:{digest(root / 'tools/semantic_closure.py')}"
        check(
            checks,
            "semantic_closure_tool_identity",
            identity.get("semantic_closure_tool_version", {}).get("value")
            == semantic_tool_identity,
            f"identity={identity.get('semantic_closure_tool_version', {}).get('value')} "
            f"actual={semantic_tool_identity}",
        )
        check(
            checks,
            "scope_schema_identity",
            identity.get("scope_schema_version", {}).get("value")
            == "source-header-context-oracle-semantic-closure-phase0-v8",
            identity.get("scope_schema_version", {}).get("value"),
        )
        check(
            checks,
            "header_dependency_schema_identity",
            identity.get("header_dependency_schema_version", {}).get("value")
            == "header-provider-enumerator-graph-v3",
            identity.get("header_dependency_schema_version", {}).get("value"),
        )
        check(
            checks,
            "oracle_classification_schema_identity",
            identity.get("oracle_classification_schema_version", {}).get("value")
            == "oracle-classification-v1",
            identity.get("oracle_classification_schema_version", {}).get("value"),
        )
        semantic_identity_expected = {
            "semantic_closure_schema_version": semantic_closure.SCHEMA_VERSION,
            "semantic_closure_key_schema_version": semantic_closure.KEY_SCHEMA_VERSION,
            "semantic_closure_schema_sha256": digest(artifacts / "semantic-closure/SCHEMA.tsv"),
            "semantic_closure_base_sha256": digest(artifacts / "semantic-closure/BASE.tsv"),
            "semantic_closure_task_keyset_sha256": semantic_values.get("task_keyset_sha256", ""),
            "semantic_closure_pending_field_count": semantic_values.get("pending_field_count", ""),
            "semantic_closure_ledger_binding": "MUTABLE_CONTENT_EXCLUDED",
        }
        semantic_identity_errors = {
            key: {"identity": identity.get(key, {}).get("value"), "expected": expected}
            for key, expected in semantic_identity_expected.items()
            if identity.get(key, {}).get("value") != expected
        }
        check(
            checks,
            "semantic_closure_identity_binding",
            not semantic_identity_errors,
            semantic_identity_errors,
        )
        identity_predicate_expected = {
            "compiler_predicates_sha256": digest(predicate_root / "COMPILER_PREDICATES.tsv"),
            "compiler_predicates_validation_sha256": digest(predicate_root / "VALIDATION.tsv"),
            "compiler_predicates_schema_version": "compiler-predicates-v1",
            "compiler_predicates_count": str(len(predicate_rows)),
            "compiler_predicates_x86_64_count": str(predicate_counts["x86_64"]),
            "compiler_predicates_aarch64_count": str(predicate_counts["aarch64"]),
            "compiler_predicates_validation_status": "PASS",
        }
        predicate_identity_errors = {
            key: {"identity": identity.get(key, {}).get("value"), "expected": expected}
            for key, expected in identity_predicate_expected.items()
            if identity.get(key, {}).get("value") != expected
        }
        check(
            checks,
            "compiler_predicate_identity",
            not predicate_identity_errors,
            predicate_identity_errors,
        )
        check(
            checks,
            "authoritative_manifests_identity",
            identity.get("authoritative_manifests_sha256", {}).get("value")
            == digest(artifacts / "metadata/authoritative_manifests.tsv"),
            identity.get("authoritative_manifests_sha256", {}).get("value"),
        )
        check(
            checks,
            "metadata_manifest_identity",
            identity.get("metadata_manifest_sha256", {}).get("value")
            == digest(artifacts / "metadata/manifest.tsv"),
            identity.get("metadata_manifest_sha256", {}).get("value"),
        )
        task_rows = required_fields(
            checks,
            "queue_schema",
            artifacts / "TRANSLATION_TASKS.tsv",
            {"id", "path", "created_at", "status", "linux_path", "architectures", "cluster", "weight", "risk", "dependencies", "recommended_implementer"},
        )
        bijection_ok, bijection_detail = queue_matches_scope(scope, task_rows)
        check(checks, "rust_task_bijection", bijection_ok, bijection_detail)
        check(
            checks,
            "queue_initial_state",
            all(
                row.get("status") == "TODO"
                and row.get("attempt") == "0"
                and not row.get("work_started_at")
                and not row.get("done_at")
                and not row.get("lease_owner")
                and not row.get("lease_expires_at")
                and not row.get("pipeline_id")
                and not row.get("implement_done_at")
                and not row.get("review_started_at")
                and not row.get("review_1_done_at")
                and not row.get("review_2_done_at")
                and not row.get("apply_started_at")
                and not row.get("resume_status")
                and not row.get("last_error")
                for row in task_rows
            ),
            "all rows TODO, attempt zero, unleased, and without work-stage timestamps",
        )
        check(
            checks,
            "driver_exclusion",
            not any(scope_by_id.get(row.get("id", ""), {}).get("class") == "LINUX_DRIVER_OBJECT" for row in task_rows),
            "driver rows must not be queued",
        )
        oracle_queue_rows = [
            row.get("id", "") for row in task_rows
            if row.get("linux_path", "") in expected_oracle_paths
            or scope_by_id.get(row.get("id", ""), {}).get("class") == "ORACLE_ONLY"
        ]
        check(
            checks,
            "oracle_excluded_from_queue",
            not oracle_queue_rows,
            oracle_queue_rows[:20],
        )
        queue_verify = subprocess.run(
            [
                sys.executable,
                "tools/rewrite_queue.py",
                "verify",
                "--queue", str(artifacts / "TRANSLATION_TASKS.tsv"),
                "--fingerprint", str(artifacts / "TRANSLATION_TASKS.sha256"),
                "--events", str(artifacts / "events.jsonl"),
                "--logs-root", str(artifacts / "logs/tasks"),
                "--linux-sha-file", str(root / "vendor/linux.SHA"),
                "--linux-root", str(root / "vendor/linux"),
            ],
            cwd=root,
            text=True,
            capture_output=True,
        )
        check(checks, "queue_verify", queue_verify.returncode == 0, queue_verify.stdout + queue_verify.stderr)
        fingerprint_path = artifacts / "TRANSLATION_TASKS.sha256"
        fingerprint = ""
        fingerprint_rows: dict[str, str] = {}
        if fingerprint_path.is_file():
            fingerprint_rows = dict(
                line.split("\t", 1) for line in fingerprint_path.read_text().splitlines() if "\t" in line
            )
            fingerprint = fingerprint_rows.get("sha256", "")
        check(
            checks,
            "identity_queue_binding",
            identity.get("queue_fingerprint", {}).get("value") == fingerprint and bool(fingerprint),
            f"identity={identity.get('queue_fingerprint', {}).get('value')} queue={fingerprint}",
        )
        check(
            checks,
            "queue_phase0_identity_binding",
            fingerprint_rows.get("phase0_identity_binding_sha256", "")
            == identity.get("phase0_identity_binding_sha256", {}).get("value", ""),
            f"queue={fingerprint_rows.get('phase0_identity_binding_sha256', '')} "
            f"identity={identity.get('phase0_identity_binding_sha256', {}).get('value', '')}",
        )

    report = {"ok": all(item["ok"] for item in checks.values()), "stage": args.stage, "checks": checks}
    if not args.no_write_report:
        json_report = artifacts / "PHASE0_VALIDATION.json"
        tsv_report = artifacts / "PHASE0_VALIDATION.tsv"
        checksum_report = artifacts / "PHASE0_VALIDATION.sha256"
        json_report.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        with tsv_report.open("w", encoding="utf-8") as handle:
            handle.write("check\tstatus\tdetail\n")
            for name, item in checks.items():
                handle.write(
                    f"{name}\t{'PASS' if item['ok'] else 'FAIL'}\t{item['detail'].replace(chr(9), ' ')}\n"
                )
        checksum_report.write_text(
            f"{digest(json_report)}  {json_report.name}\n"
            f"{digest(tsv_report)}  {tsv_report.name}\n",
            encoding="utf-8",
        )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
