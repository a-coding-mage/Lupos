#!/usr/bin/env python3
"""Independently reconstruct and replay frozen compiler predicate probes."""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
BRANCH = "feat/bun-like-rewrite-test"
KINDS = {
    "__has_attribute", "__has_builtin", "__has_feature", "__has_extension",
    "__has_c_attribute", "__has_declspec_attribute", "__has_warning",
}
INVENTORY_FIELDS = [
    "predicate_id", "predicate_kind", "argument", "architecture",
    "target_triple", "result", "source_locations", "linux_commit",
    "config_sha256", "toolchain_sha256", "compiler_requested_path",
    "compiler_resolved_path", "compiler_sha256", "compiler_version",
    "original_command_source", "original_command_sha256", "probe_path",
    "probe_sha256", "probe_command_path", "probe_command_sha256",
    "stdout_path", "stdout_sha256", "stderr_path", "stderr_sha256",
    "exit_status", "started_at", "completed_at", "status",
]
VALIDATION_FIELDS = [
    "predicate_id", "validation_status", "checks_passed", "checks_failed",
    "detail", "validated_at",
]
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


def write_atomic(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if temporary.exists():
            temporary.unlink()


def tsv_bytes(fields: list[str], rows: list[dict[str, str]]) -> bytes:
    import io

    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=fields, delimiter="\t", lineterminator="\n")
    writer.writeheader()
    for row in rows:
        writer.writerow({field: row.get(field, "") for field in fields})
    return output.getvalue().encode()


def command_argv(entry: dict[str, object]) -> list[str]:
    arguments = entry.get("arguments")
    if isinstance(arguments, list):
        return [str(value) for value in arguments]
    command = entry.get("command")
    if not isinstance(command, str) or not command:
        raise ValueError("compile entry has no command or arguments")
    return shlex.split(command)


def resolve_source(entry: dict[str, object]) -> Path:
    directory = Path(str(entry["directory"]))
    source = Path(str(entry["file"]))
    return (directory / source).resolve() if not source.is_absolute() else source.resolve()


def reconstruct_command(
    argv: list[str], source: Path, directory: Path, compiler: str
) -> list[str]:
    if not argv or argv[0] != compiler:
        raise ValueError(f"compiler mismatch in original command: {argv[0] if argv else '(empty)'}")
    if any(token.startswith("@") for token in argv[1:]):
        raise ValueError("response-file compile command cannot be independently reconstructed")
    language = "assembler-with-cpp" if source.suffix == ".S" else "c++" if source.suffix in {".cc", ".cpp", ".cxx"} else "c"
    output = [argv[0]]
    removed = 0
    index = 1
    while index < len(argv):
        token = argv[index]
        if token == "-c" or token in DEPENDENCY_FLAGS:
            index += 1
            continue
        if token in {"-o", "-x", *DEPENDENCY_VALUE_FLAGS}:
            if index + 1 >= len(argv):
                raise ValueError(f"missing value after {token}")
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
            candidate = (directory / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()
            if candidate == source:
                removed += 1
                index += 1
                continue
        output.append(token)
        index += 1
    if removed != 1:
        raise ValueError(f"expected one source input, removed {removed}")
    output.extend(["-E", "-P", "-x", language, "-"])
    return output


def normalize_path(value: str) -> str:
    return os.path.normpath(value).replace(os.sep, "/")


def make_assignment(text: str, variable: str) -> str:
    """Independently parse one Kbuild make assignment and continuations."""

    lines = text.splitlines()
    pattern = re.compile(rf"^{re.escape(variable)}\s*:=\s*(.*)$")
    for index, line in enumerate(lines):
        match = pattern.match(line)
        if not match:
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
    raise ValueError(f"missing {variable} assignment")


def parse_cmd_evidence(path: Path, object_path: str) -> tuple[list[str], Path]:
    """Independently obtain the compiler command and source from Kbuild .cmd."""

    text = path.read_text(encoding="utf-8", errors="strict")
    saved_names = re.findall(r"^savedcmd_([^\s]+)\s*:=", text, flags=re.MULTILINE)
    matching = [name for name in saved_names if normalize_path(name) == normalize_path(object_path)]
    if len(matching) != 1:
        raise ValueError(f"expected one savedcmd for {object_path} in {path}, found {matching}")
    object_name = matching[0]
    argv = shlex.split(make_assignment(text, f"savedcmd_{object_name}"), comments=False, posix=True)
    if ";" in argv:
        argv = argv[:argv.index(";")]
    if not argv:
        raise ValueError(f"empty compiler stage in {path}")
    source_values = shlex.split(make_assignment(text, f"source_{object_name}"), comments=False, posix=True)
    if len(source_values) != 1:
        raise ValueError(f"expected one source in {path}, found {len(source_values)}")
    source = Path(source_values[0])
    return argv, source


def cmd_evidence_path(build: Path, object_path: str) -> Path:
    relative = Path(normalize_path(object_path))
    return build / relative.parent / f".{relative.name}.cmd"


def reconstruct_environment(rows: list[dict[str, str]], architecture: str) -> dict[str, str]:
    frozen: dict[str, str] = {}
    for selected_architecture in ("common", architecture):
        for row in rows:
            if row.get("architecture") != selected_architecture:
                continue
            key = row["key"]
            if key in frozen:
                raise ValueError(f"duplicate environment key for {selected_architecture}: {key}")
            if row.get("status") == "NEUTRALIZED" or row["value"] == "__UNSET__":
                continue
            if row.get("status") != "FROZEN":
                raise ValueError(f"environment key is not FROZEN or NEUTRALIZED: {selected_architecture}:{key}")
            frozen[key] = "" if row["value"] == "(empty)" else row["value"]
    missing = sorted({"PATH", "LLVM", "LLVM_IAS"} - set(frozen))
    if missing:
        raise ValueError(f"ENVIRONMENT.tsv missing {missing}")
    return {
        "PATH": frozen["PATH"],
        "LLVM": frozen["LLVM"],
        "LLVM_IAS": frozen["LLVM_IAS"],
        **{key: value for key, value in frozen.items() if key not in {"PATH", "LLVM", "LLVM_IAS"}},
        "LC_ALL": "C",
        "LANG": "C",
        "TZ": "UTC",
    }


def target_triple(argv: list[str]) -> str:
    value = ""
    index = 1
    while index < len(argv):
        if argv[index] == "--target" and index + 1 < len(argv):
            value = argv[index + 1]
            index += 2
            continue
        if argv[index].startswith("--target="):
            value = argv[index].split("=", 1)[1]
        index += 1
    return value


def probe_source(predicate_id: str, kind: str, argument: str) -> bytes:
    return (
        f"#if {kind}({argument})\n"
        f"LUPOS_COMPILER_PREDICATE_{predicate_id}_1\n"
        f"#else\n"
        f"LUPOS_COMPILER_PREDICATE_{predicate_id}_0\n"
        f"#endif\n"
    ).encode()


def parse_result(predicate_id: str, stdout: bytes) -> str | None:
    text = stdout.decode("utf-8", errors="replace")
    values = re.findall(rf"(?m)^LUPOS_COMPILER_PREDICATE_{re.escape(predicate_id)}_([01])\s*$", text)
    return values[0] if len(values) == 1 else None


def evidence_file(root: Path, output: Path, recorded: str, required_parent: Path) -> Path:
    candidate = Path(recorded)
    candidate = (root / candidate).resolve() if not candidate.is_absolute() else candidate.resolve()
    if candidate != required_parent and required_parent not in candidate.parents:
        raise ValueError(f"evidence path escapes {required_parent}: {recorded}")
    if output != required_parent and output not in candidate.parents:
        raise ValueError(f"evidence path is outside predicate output: {recorded}")
    return candidate


def original_entry(root: Path, source: str) -> tuple[dict[str, object], Path, int]:
    match = re.fullmatch(r"(.+)#entry=([0-9]+)", source)
    if not match:
        raise ValueError(f"invalid original_command_source: {source}")
    path = Path(match.group(1))
    path = (root / path).resolve() if not path.is_absolute() else path.resolve()
    index = int(match.group(2))
    database = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(database, list) or index >= len(database) or not isinstance(database[index], dict):
        raise ValueError(f"invalid compile database entry: {source}")
    return database[index], path, index


def fingerprint_values(path: Path) -> dict[str, str]:
    result = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("\t")
        if separator:
            result[key] = value
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--rewrite-root", type=Path, default=Path("rewrite"))
    parser.add_argument("--input", type=Path, default=Path("rewrite/compiler-predicates"))
    parser.add_argument("--timeout-seconds", type=int, default=60)
    parser.add_argument("--execute", action="store_true", help="required acknowledgement that probes will be replayed")
    args = parser.parse_args()
    if not args.execute:
        raise SystemExit("refusing to replay compiler probes without --execute")
    root = args.root.resolve()
    rewrite = args.rewrite_root if args.rewrite_root.is_absolute() else root / args.rewrite_root
    evidence_root = args.input if args.input.is_absolute() else root / args.input
    branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=root, text=True).strip()
    if branch != BRANCH:
        raise SystemExit(f"required branch {BRANCH!r}; current branch is {branch!r}")

    inventory_path = evidence_root / "COMPILER_PREDICATES.tsv"
    fields, inventory = read_tsv(inventory_path)
    if fields != INVENTORY_FIELDS:
        raise SystemExit(f"inventory header mismatch: {fields}")
    fingerprint = fingerprint_values(evidence_root / "COMPILER_PREDICATES.sha256")
    global_errors = []
    if fingerprint.get("sha256") != sha256_file(inventory_path):
        global_errors.append("inventory fingerprint mismatch")
    if fingerprint.get("rows") != str(len(inventory)):
        global_errors.append("inventory row count mismatch")
    if not inventory:
        global_errors.append("inventory is empty")
    linux_commit = (root / "vendor/linux.SHA").read_text().strip()
    actual_linux_commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root / "vendor/linux", text=True
    ).strip()
    if actual_linux_commit != linux_commit:
        global_errors.append("vendor/linux revision differs from vendor/linux.SHA")
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=root / "vendor/linux", text=True):
        global_errors.append("vendor/linux has local changes")
    toolchain_sha = (rewrite / "toolchain/TOOLCHAIN.sha256").read_text().split()[0]
    capture_tool = root / "tools/compiler_predicates.py"
    if fingerprint.get("generator_sha256") != sha256_file(capture_tool):
        global_errors.append("capture tool hash differs from predicate fingerprint")
    toolchain_fields, toolchain = read_tsv(rewrite / "toolchain/TOOLCHAIN.tsv")
    del toolchain_fields
    compiler_rows = [row for row in toolchain if row.get("tool_name") == "clang"]
    if len(compiler_rows) != 1:
        raise SystemExit(f"expected one clang toolchain row, found {len(compiler_rows)}")
    compiler = compiler_rows[0]
    _, environment_rows = read_tsv(rewrite / "toolchain/ENVIRONMENT.tsv")
    file_map_fields, file_map_rows = read_tsv(rewrite / "FILE_MAP.tsv")
    required_file_map = {"source_path", "object_path", "architecture", "compile_input", "compile_command"}
    if not required_file_map <= set(file_map_fields):
        raise SystemExit(f"FILE_MAP.tsv missing {sorted(required_file_map - set(file_map_fields))}")
    file_map_contexts: dict[tuple[str, str, str], dict[str, str]] = {}
    for item in file_map_rows:
        key = (
            item.get("architecture", ""),
            item.get("source_path", ""),
            normalize_path(item.get("object_path", "")),
        )
        if key in file_map_contexts:
            raise SystemExit(f"duplicate FILE_MAP compiler context: {key}")
        file_map_contexts[key] = item

    validation_rows: list[dict[str, str]] = []
    seen_ids: set[str] = set()
    for row in inventory:
        predicate_id = row["predicate_id"]
        passed: list[str] = []
        failed: list[str] = []

        def verify(name: str, condition: bool, detail: str = "") -> None:
            (passed if condition else failed).append(name if condition or not detail else f"{name}({detail})")

        verify("unique_id", predicate_id not in seen_ids)
        seen_ids.add(predicate_id)
        verify("kind", row["predicate_kind"] in KINDS)
        verify("result", row["result"] in {"0", "1"})
        verify("status", row["status"] in {"PROVEN", "BLOCKED", "NOT_APPLICABLE"})
        verify("proven_exit", row["status"] != "PROVEN" or row["exit_status"] == "0")
        verify("blocked_exit", row["status"] != "BLOCKED" or row["exit_status"] != "0")
        verify("source_locations", bool(row["source_locations"]))
        timestamp_pattern = r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3}Z"
        verify("started_at", re.fullmatch(timestamp_pattern, row["started_at"]) is not None)
        verify("completed_at", re.fullmatch(timestamp_pattern, row["completed_at"]) is not None)
        verify("timestamp_order", row["started_at"] <= row["completed_at"])
        verify("linux_commit", row["linux_commit"] == linux_commit)
        verify("fingerprint_linux", fingerprint.get("linux_commit") == linux_commit)
        verify("toolchain", row["toolchain_sha256"] == toolchain_sha)
        verify("fingerprint_toolchain", fingerprint.get("toolchain_sha256") == toolchain_sha)
        arch = row["architecture"]
        config_path = rewrite / "configs" / arch / "frozen.config"
        verify("architecture", arch in {"x86_64", "aarch64"})
        verify("config", config_path.is_file() and row["config_sha256"] == sha256_file(config_path))
        environment = reconstruct_environment(environment_rows, arch)
        requested = Path(compiler["requested_path"])
        compiler_ok = requested.is_file() and sha256_file(requested) == compiler["sha256"]
        verify("compiler_file", compiler_ok)
        verify("compiler_requested", row["compiler_requested_path"] == compiler["requested_path"])
        verify("compiler_resolved", row["compiler_resolved_path"] == compiler["resolved_path"] and requested.resolve() == Path(compiler["resolved_path"]).resolve())
        verify("compiler_sha256", row["compiler_sha256"] == compiler["sha256"])
        verify("compiler_version", row["compiler_version"] == compiler["version"])

        try:
            probe_path = evidence_file(root, evidence_root, row["probe_path"], evidence_root / "probes")
            command_path = evidence_file(root, evidence_root, row["probe_command_path"], evidence_root / "commands")
            stdout_path = evidence_file(root, evidence_root, row["stdout_path"], evidence_root / "stdout")
            stderr_path = evidence_file(root, evidence_root, row["stderr_path"], evidence_root / "stderr")
            command_bytes = command_path.read_bytes()
            verify("command_hash", sha256_bytes(command_bytes) == row["probe_command_sha256"])
            command_record = json.loads(command_bytes)
            source_path = str(command_record.get("source_path", ""))
            object_path = str(command_record.get("object_path", ""))
            map_match = file_map_contexts.get((arch, source_path, normalize_path(object_path)))
            if map_match is None:
                raise ValueError(f"missing FILE_MAP context for {arch}:{source_path}:{object_path}")
            build = (rewrite / "kbuild" / arch).resolve()
            cmd_source_record = Path(row["original_command_source"])
            cmd_source = (root / cmd_source_record).resolve() if not cmd_source_record.is_absolute() else cmd_source_record.resolve()
            expected_cmd_source = cmd_evidence_path(build, object_path).resolve()
            verify("original_command_source", cmd_source == expected_cmd_source and cmd_source.is_file())
            verify("original_command_hash", sha256_file(cmd_source) == row["original_command_sha256"])
            original_argv, original_source = parse_cmd_evidence(cmd_source, object_path)
            original_source = (build / original_source).resolve() if not original_source.is_absolute() else original_source.resolve()
            mapped_source = Path(map_match["compile_input"])
            mapped_source = (build / mapped_source).resolve() if not mapped_source.is_absolute() else mapped_source.resolve()
            verify("source_mapping", original_source == mapped_source)
            verify("file_map_command", shlex.split(map_match["compile_command"], comments=False, posix=True) == original_argv)
            expected_argv = reconstruct_command(original_argv, original_source, build, compiler["requested_path"])
            expected_target = target_triple(original_argv)
            verify("target_triple", expected_target == row["target_triple"] and bool(expected_target))
            expected_probe = probe_source(predicate_id, row["predicate_kind"], row["argument"])
            recorded_probe = probe_path.read_bytes()
            verify("probe_content", recorded_probe == expected_probe)
            verify("probe_hash", sha256_bytes(recorded_probe) == row["probe_sha256"])
            database_entry, database_path, database_index = original_entry(
                root, str(command_record.get("compile_database_source", ""))
            )
            expected_database = (rewrite / "metadata" / arch / "compile_commands.json").resolve()
            verify("compile_database_source", database_path == expected_database)
            verify("compile_database_hash", sha256_bytes(canonical_json(database_entry)) == command_record.get("compile_database_entry_sha256"))
            verify("compile_database_command", command_argv(database_entry) == original_argv)
            verify("compile_database_source_mapping", resolve_source(database_entry) == original_source)
            expected_command_record = {
                "schema_version": "compiler-predicate-command-v1",
                "predicate_id": predicate_id,
                "cwd": str(build),
                "argv": expected_argv,
                "environment": environment,
                "stdin_path": row["probe_path"],
                "original_command_source": row["original_command_source"],
                "original_command_sha256": row["original_command_sha256"],
                "source_path": source_path,
                "object_path": object_path,
                "compile_database_source": command_record.get("compile_database_source", ""),
                "compile_database_entry_sha256": command_record.get("compile_database_entry_sha256", ""),
            }
            verify("command_reconstruction", command_record == expected_command_record)
            recorded_stdout = stdout_path.read_bytes()
            recorded_stderr = stderr_path.read_bytes()
            verify("stdout_hash", sha256_bytes(recorded_stdout) == row["stdout_sha256"])
            verify("stderr_hash", sha256_bytes(recorded_stderr) == row["stderr_sha256"])
            verify("recorded_result", parse_result(predicate_id, recorded_stdout) == row["result"])
            if row["status"] == "PROVEN":
                completed = subprocess.run(
                    expected_argv,
                    cwd=build,
                    env=environment,
                    input=expected_probe,
                    capture_output=True,
                    timeout=args.timeout_seconds,
                    check=False,
                )
                actual_result = parse_result(predicate_id, completed.stdout)
                verify("replay_exit", str(completed.returncode) == row["exit_status"])
                verify("replay_result", actual_result == row["result"])
                verify("replay_stdout", completed.stdout == recorded_stdout)
                verify("replay_stderr", completed.stderr == recorded_stderr)
            else:
                passed.append("replay_not_required_for_non_proven")
            del database_path, database_index
        except Exception as exc:
            failed.append(f"reconstruction({type(exc).__name__}: {exc})")

        validation_rows.append({
            "predicate_id": predicate_id,
            "validation_status": "PASS" if not failed else "FAIL",
            "checks_passed": ";".join(passed),
            "checks_failed": ";".join(failed),
            "detail": "independent context reconstruction and preprocess replay" if not failed else "validation mismatch",
            "validated_at": utc_now(),
        })

    overall_ok = not global_errors and all(row["validation_status"] == "PASS" for row in validation_rows)
    validation_bytes = tsv_bytes(VALIDATION_FIELDS, validation_rows)
    write_atomic(evidence_root / "VALIDATION.tsv", validation_bytes)
    failures = [row for row in validation_rows if row["validation_status"] == "FAIL"]
    report_lines = [
        "# Compiler predicate validation",
        "",
        f"- Result: {'PASS' if overall_ok else 'FAIL'}",
        f"- Inventory rows: {len(inventory)}",
        f"- Passed rows: {len(inventory) - len(failures)}",
        f"- Failed rows: {len(failures)}",
        f"- Inventory SHA-256: `{sha256_file(inventory_path)}`",
        f"- Validated at: `{utc_now()}`",
        "",
    ]
    if global_errors:
        report_lines.extend(["## Global failures", "", *[f"- {value}" for value in global_errors], ""])
    if failures:
        report_lines.extend(["## Predicate failures", ""])
        report_lines.extend(f"- `{row['predicate_id']}`: {row['checks_failed']}" for row in failures)
        report_lines.append("")
    write_atomic(evidence_root / "validation-report.md", ("\n".join(report_lines) + "\n").encode())
    print(json.dumps({"ok": overall_ok, "rows": len(inventory), "failures": len(failures)}, sort_keys=True))
    return 0 if overall_ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
