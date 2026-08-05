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

ARCH_CONFIG_NAMES = {"x86_64": "x86_64", "aarch64": "aarch64"}
IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
DEFINE_RE = re.compile(r"^\s*#\s*define\s+([A-Za-z_][A-Za-z0-9_]*)(\s*\([^\n]*?\))?\s*(.*)$", re.S)
UNDEF_RE = re.compile(r"^\s*#\s*undef\s+([A-Za-z_][A-Za-z0-9_]*)\b")
CONDITIONAL_RE = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)$", re.S)
EXPORT_RE = re.compile(r"\bEXPORT_SYMBOL(_GPL)?\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")
TYPE_TAG_RE = re.compile(r"\b(struct|union|enum)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{")
TYPEDEF_NAME_RE = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*(?:\[[^;]*\])?\s*;$")
FUNCTION_MACRO_RE = re.compile(
    r"\b((?:COMPAT_)?SYSCALL_DEFINE\d+|DEFINE_[A-Z0-9_]*SHOW_ATTRIBUTE|"
    r"BPF_CALL_\d+|TRACE_EVENT)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)"
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_tsv(path: Path, fields: list[str], rows: Iterable[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows({field: row.get(field, "") for field in fields} for row in rows)


def normalize_path(value: str) -> str:
    return os.path.normpath(value).replace(os.sep, "/")


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


def built_in_members(build: Path) -> dict[str, set[str]]:
    """Read thin-archive membership from Kbuild's retained ``.built-in.a.cmd`` files."""
    owners: dict[str, set[str]] = defaultdict(set)
    pattern = re.compile(r"printf\s+([\"'])([^\"']*%s[^\"']*)\1\s+(.*?)\s*\|\s*xargs", re.S)
    for command_file in sorted(build.rglob(".built-in.a.cmd")):
        archive = normalize_path((command_file.parent / "built-in.a").relative_to(build).as_posix())
        content = command_file.read_text(errors="replace").replace("\\\n", " ")
        for match in pattern.finditer(content):
            template, arguments = match.group(2), match.group(3)
            prefix, suffix = template.split("%s", 1)
            try:
                tokens = shlex.split(arguments)
            except ValueError as exc:
                raise ValueError(f"cannot parse {command_file}: {exc}") from exc
            for token in tokens:
                if token.endswith((".o", ".a")):
                    owners[normalize_path(prefix + token + suffix)].add(archive)
    return owners


def kbuild_ownership(build: Path) -> dict[str, tuple[str, str, str]]:
    """Resolve each object to disposition, owning Kbuild target, and evidence."""
    modules = module_targets(build)
    composites = composite_members(build)
    archives = built_in_members(build)
    cache: dict[str, tuple[str, str, str]] = {}

    def resolve(object_path: str, trail: tuple[str, ...] = ()) -> tuple[str, str, str]:
        object_path = normalize_path(object_path)
        if object_path in cache:
            return cache[object_path]
        if object_path in trail:
            raise ValueError(f"cyclic Kbuild ownership: {' -> '.join((*trail, object_path))}")
        if object_path in modules and object_path in archives:
            raise ValueError(f"object is both module and built-in: {object_path}")
        if object_path in archives and object_path in composites:
            raise ValueError(f"object is both a built-in member and a composite component: {object_path}")
        if object_path in modules:
            result = ("module", object_path, "modules.order")
        elif object_path in archives:
            choices = sorted(archives[object_path])
            if len(choices) != 1:
                raise ValueError(f"ambiguous built-in ownership for {object_path}: {choices}")
            result = ("built-in", choices[0], f"{Path(choices[0]).parent.as_posix()}/.built-in.a.cmd")
        elif object_path in composites:
            choices = sorted(composites[object_path])
            resolved = [resolve(choice, (*trail, object_path)) for choice in choices]
            unique = {(mode, owner) for mode, owner, _ in resolved}
            if len(unique) != 1:
                raise ValueError(f"contradictory composite ownership for {object_path}: {resolved}")
            mode, owner, parent_evidence = resolved[0]
            composite_evidence = f"{Path(choices[0]).with_suffix('.mod').as_posix()};{parent_evidence}"
            result = (mode, owner, composite_evidence)
        else:
            result = ("metadata", object_path, "compile_commands.json;ownership-unresolved")
        cache[object_path] = result
        return result

    candidates = set(modules) | set(composites) | set(archives)
    for candidate in sorted(candidates):
        resolve(candidate)
    return cache


def source_class(path: str, kind: str, owners: Iterable[str]) -> str:
    if kind != "linux":
        return "BUILD_METADATA"
    suffix = Path(path).suffix
    lowered = suffix.lower()
    test_markers = ("/kunit/", "/selftests/", "/testing/", "/tests/", "/test/", "test_", "_test.")
    if any(marker in f"/{path}" for marker in test_markers) or path.startswith("tools/testing/"):
        return "ORACLE_ONLY"
    owner_paths = tuple(owners)
    if path.startswith(("drivers/", "sound/")) or any(owner.startswith(("drivers/", "sound/")) for owner in owner_paths):
        return "LINUX_DRIVER_OBJECT"
    if lowered in {".s", ".asm"} and path.startswith("arch/"):
        return "LINUX_ARCH_ASM"
    if lowered in {".s", ".asm"}:
        return "LINUX_DRIVER_OBJECT"
    if lowered not in {".c", ".h", ".cc", ".cpp"}:
        return "BUILD_METADATA"
    return "RUST_TRANSLATE"


def destination(path: str, source_classification: str) -> str:
    return "src/" + str(Path(path).with_suffix(".rs")) if source_classification == "RUST_TRANSLATE" else ""


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


def safe_expression_value(expression: str, macros: dict[str, object]) -> bool:
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
                    value = safe_expression_value(expression, macros)
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
                value = safe_expression_value(argument, macros)
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
    if "=" in compact or compact.startswith(("struct ", "union ", "enum ", "typedef ")):
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


def source_entities(text: str, active_lines: set[int], selection: dict[int, str]) -> list[dict[str, str]]:
    lines = text.splitlines(keepends=True)
    selected_text = "".join(
        line if number in active_lines and not re.match(r"^\s*#", line) else "\n" if line.endswith("\n") else ""
        for number, line in enumerate(lines, 1)
    )
    masked = mask_c(selected_text)
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
            elif compact and "(" not in compact and not compact.startswith(("struct ", "union ", "enum ")):
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
    unique: dict[tuple[str, str, str], dict[str, str]] = {}
    for row in entities:
        unique[(row["record_kind"], row["symbol_name"], row["source_line"])] = row
    return sorted(unique.values(), key=lambda row: (int(row["source_line"]), row["record_kind"], row["symbol_name"]))


def semantic_records(
    scope_id: str,
    linux_path: str,
    arch: str,
    source: Path,
    config_path: Path,
    compile_command: str,
) -> tuple[list[dict[str, str]], list[dict[str, str]], list[dict[str, str]]]:
    text = source.read_text(errors="replace")
    active, selection, conditions, macros = selected_lines(text, arch, config_path, compile_command)
    entities = source_entities(text, active, selection)
    base_evidence = f"vendor/linux/{linux_path}"
    config_evidence = f"rewrite/configs/{ARCH_CONFIG_NAMES[arch]}/frozen.config"
    symbols: list[dict[str, str]] = []
    abi: list[dict[str, str]] = []
    lifetimes: list[dict[str, str]] = []
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
        content = command_file.read_text(errors="replace").replace("\\\n", " ")
        dependency_match = re.search(r"\bdeps_[^:]+:=\s*(.*?)(?:\n\s*[^\s].*?:=|$)", content, re.S)
        if dependency_match:
            for dependency in dependency_match.group(1).split():
                if dependency not in {"\\"}:
                    include_rows.append({"architecture": arch, "depfile": rel, "dependency": dependency})
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

    source_rows: list[dict[str, str]] = []
    symbols: list[dict[str, str]] = []
    abi: list[dict[str, str]] = []
    lifetimes: list[dict[str, str]] = []
    driver_abi: list[dict[str, str]] = []
    for index, source_path in enumerate(sorted(by_source), 1):
        items = by_source[source_path]
        arches = sorted({item["architecture"] for item in items})
        architecture = "common" if len(arches) == 2 else arches[0]
        resolved_items = []
        for item in items:
            mode, owner, evidence = ownership_by_arch[item["architecture"]].get(
                item["object_path"], ("metadata", item["object_path"], "compile_commands.json;ownership-unresolved")
            )
            resolved_items.append((item, mode, owner, evidence))
        classification = source_class(source_path, items[0]["source_kind"], (owner for _, _, owner, _ in resolved_items))
        scope_id = f"S{index:06d}"
        kbuild_targets = sorted(
            f"{item['architecture']}:{item['object_path']}:{mode}:{owner}" for item, mode, owner, _ in resolved_items
        )
        kconfig_evidence = sorted(
            f"config:{item['architecture']}=rewrite/configs/{ARCH_CONFIG_NAMES[item['architecture']]}/frozen.config;"
            f"disposition={mode};owner={owner};command=metadata/{item['architecture']}/compile_commands.json"
            for item, mode, owner, _ in resolved_items
        )
        source_rows.append({
            "id": scope_id,
            "linux_path": source_path,
            "destination_path": destination(source_path, classification),
            "class": classification,
            "architectures": architecture,
            "kconfig_evidence": ";".join(kconfig_evidence),
            "kbuild_target": ";".join(kbuild_targets),
            "cluster": source_path.split("/", 1)[0],
            "weight": str(weight(source_path, linux)),
            "risk": risk(source_path),
            "dependencies": "",
            "recommended_implementer": "luna",
            "source_kind": items[0]["source_kind"],
            "metadata_status": "COMPLETE" if all(mode != "metadata" for _, mode, _, _ in resolved_items) else "PENDING_REVIEW",
            "metadata_evidence": "rewrite/metadata/manifest.tsv",
            "semantic_status": "PENDING_REVIEW" if classification in {"RUST_TRANSLATE", "LINUX_DRIVER_OBJECT"} else "NOT_APPLICABLE",
        })
        if classification == "RUST_TRANSLATE":
            for item, _, _, _ in resolved_items:
                source = linux / source_path
                if not source.is_file():
                    raise ValueError(f"RUST_TRANSLATE source does not exist: {source_path}")
                symbol_rows, abi_rows, lifetime_rows = semantic_records(
                    scope_id, source_path, item["architecture"], source, configs[item["architecture"]], item["compile_command"]
                )
                symbols.extend(symbol_rows)
                abi.extend(abi_rows)
                lifetimes.extend(lifetime_rows)
        elif classification == "LINUX_DRIVER_OBJECT":
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
    write_tsv(args.out / "FILE_MAP.tsv", FILE_MAP_FIELDS, file_rows)
    write_tsv(args.out / "SYMBOLS.tsv", SYMBOL_FIELDS, symbols)
    write_tsv(args.out / "ABI.tsv", ABI_FIELDS, abi)
    write_tsv(args.out / "LIFETIMES.tsv", LIFETIME_FIELDS, lifetimes)
    write_tsv(args.out / "DRIVER_ABI.tsv", DRIVER_ABI_FIELDS, driver_abi)

    summary = {
        "linux_commit": Path("vendor/linux.SHA").read_text().strip(),
        "sources_total": len(source_rows),
        "rust_translate": sum(row["class"] == "RUST_TRANSLATE" for row in source_rows),
        "symbols": len(symbols),
        "abi_records": len(abi),
        "lifetime_records": len(lifetimes),
        "by_class": {},
    }
    for row in source_rows:
        summary["by_class"][row["class"]] = summary["by_class"].get(row["class"], 0) + 1
    (args.out / "metadata" / "summary.json").write_text(
        json.dumps(summary, sort_keys=True, indent=2) + "\n", encoding="utf-8"
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
