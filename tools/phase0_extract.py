#!/usr/bin/env python3
"""Extract mechanically provable Phase 0 scope and queue inputs from Linux Kbuild."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shlex
import shutil
import tarfile
from collections import defaultdict


SCOPE_FIELDS = [
    "id", "linux_path", "destination_path", "class", "architectures",
    "kconfig_evidence", "kbuild_target", "cluster", "weight", "risk",
    "dependencies", "recommended_implementer", "source_kind",
    "metadata_status", "metadata_evidence", "semantic_status",
]
FILE_MAP_FIELDS = [
    "source_path", "object_path", "architecture", "module_or_builtin",
    "compile_input", "compile_command", "metadata_evidence",
]
TASK_FIELDS = [
    "id", "path", "created_at", "work_started_at", "done_at", "status",
    "linux_path", "architectures", "cluster", "weight", "risk",
    "dependencies", "recommended_implementer", "pipeline_id", "attempt",
    "lease_owner", "lease_expires_at", "implement_done_at",
    "review_started_at", "review_1_done_at", "review_2_done_at",
    "apply_started_at", "updated_at", "resume_status", "last_error",
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_tsv(path: Path, fields: list[str], rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows({field: row.get(field, "") for field in fields} for row in rows)


def rel_linux(value: str, directory: Path, linux: Path, build: Path, arch: str) -> tuple[str, str]:
    candidate = Path(value)
    if not candidate.is_absolute():
        candidate = (directory / candidate).resolve()
    else:
        candidate = candidate.resolve()
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
        command = entry.get("command")
        if command is None:
            command = shlex.join(entry["arguments"])
        tokens = shlex.split(command)
        output = ""
        for index, token in enumerate(tokens[:-1]):
            if token == "-o":
                output = tokens[index + 1]
        source_value = entry.get("file", "")
        source_path, source_kind = rel_linux(source_value, directory, linux, build, arch)
        object_path = os.path.normpath(output) if output else ""
        result.append({
            "source_path": source_path,
            "source_kind": source_kind,
            "object_path": object_path,
            "architecture": arch,
            "compile_input": source_value,
            "compile_command": command,
            "directory": str(directory),
        })
    return result


def module_stems(build: Path) -> set[str]:
    result: set[str] = set()
    order = build / "modules.order"
    if order.exists():
        for line in order.read_text(errors="replace").splitlines():
            if line.strip():
                result.add(os.path.splitext(line.strip())[0])
    return result


def disposition(item: dict[str, str], build: Path, modules: set[str]) -> str:
    object_path = item["object_path"]
    stem = os.path.splitext(object_path)[0]
    if stem in modules:
        return "module"
    parent = Path(object_path).parent
    if (build / parent / "built-in.a").exists():
        return "built-in"
    return "metadata"


def source_class(path: str, kind: str) -> str:
    if kind != "linux":
        return "BUILD_METADATA"
    suffix = Path(path).suffix.lower()
    test_markers = ("/kunit/", "/selftests/", "/testing/", "/tests/", "/test/", "test_", "_test.")
    if any(marker in f"/{path}" for marker in test_markers) or path.startswith("tools/testing/"):
        return "ORACLE_ONLY"
    if path.startswith("drivers/") or path.startswith("sound/"):
        return "LINUX_DRIVER_OBJECT"
    if suffix in {".s", ".asm"} or (suffix == ".S" and path.startswith("arch/")):
        return "LINUX_ARCH_ASM"
    if suffix == ".S":
        return "LINUX_DRIVER_OBJECT"
    if suffix not in {".c", ".h", ".cc", ".cpp"}:
        return "BUILD_METADATA"
    return "RUST_TRANSLATE"


def destination(path: str, cls: str) -> str:
    if cls != "RUST_TRANSLATE":
        return ""
    return "src/" + str(Path(path).with_suffix(".rs"))


def weight(path: str, linux: Path) -> float:
    source = linux / path
    try:
        lines = source.read_text(errors="replace").count("\n")
    except OSError:
        lines = 100
    return round(max(1.0, lines / 10.0), 1)


def risk(path: str) -> str:
    if path.startswith(("kernel/", "mm/", "arch/")):
        return "high"
    if path.startswith(("fs/", "net/", "block/", "security/")):
        return "medium"
    return "low"


def copy_metadata(build: Path, target: Path, arch: str, entries: list[dict[str, str]]) -> None:
    target.mkdir(parents=True, exist_ok=True)
    for name in ("compile_commands.json", "modules.order", "modules.builtin",
                 "modules.builtin.modinfo", "System.map", "vmlinux.symvers", ".config"):
        source = build / name
        if source.exists():
            shutil.copy2(source, target / ("generated.config" if name == ".config" else name))
    log = build / "build.log"
    if log.exists():
        shutil.copy2(log, target / "build.log")
    cmd_rows = []
    dep_rows = []
    include_rows = []
    object_rows = []
    for command_file in sorted(build.rglob("*.cmd")):
        rel = command_file.relative_to(build).as_posix()
        cmd_rows.append({"architecture": arch, "path": rel, "sha256": sha256(command_file)})
    for dep_file in sorted(build.rglob("*.d")):
        rel = dep_file.relative_to(build).as_posix()
        dep_rows.append({"architecture": arch, "path": rel, "sha256": sha256(dep_file)})
        content = dep_file.read_text(errors="replace").replace("\\\n", " ")
        dependencies = content.split()[1:]
        include_rows.extend({"architecture": arch, "depfile": rel, "dependency": dependency} for dependency in dependencies)
    for item in entries:
        object_rows.append({"architecture": arch, "source_path": item["source_path"],
                            "object_path": item["object_path"],
                            "module_or_builtin": disposition(item, build, module_stems(build))})
    write_tsv(target / "cmd_inventory.tsv", ["architecture", "path", "sha256"], cmd_rows)
    write_tsv(target / "depfile_inventory.tsv", ["architecture", "path", "sha256"], dep_rows)
    write_tsv(target / "include_dependencies.tsv", ["architecture", "depfile", "dependency"], include_rows)
    write_tsv(target / "object_inventory.tsv", ["architecture", "source_path", "object_path", "module_or_builtin"], object_rows)
    artifacts = []
    for path in sorted(build.rglob("*")):
        if path.is_file():
            artifacts.append({"architecture": arch, "path": path.relative_to(build).as_posix(), "sha256": sha256(path)})
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
    parser.add_argument("--out", type=Path, default=Path("rewrite"))
    parser.add_argument("--created-at", required=True)
    args = parser.parse_args()
    linux = args.linux.resolve()
    builds = [("x86_64", args.x86_build.resolve()), ("aarch64", args.arm_build.resolve())]
    all_entries: list[dict[str, str]] = []
    by_source: dict[str, list[dict[str, str]]] = defaultdict(list)
    for arch, build in builds:
        entries = compile_entries(build, linux, arch)
        all_entries.extend(entries)
        for item in entries:
            by_source[item["source_path"]].append(item)
        copy_metadata(build, args.out / "metadata" / arch, arch, entries)
    source_rows = []
    task_rows = []
    ids: dict[str, str] = {}
    for index, source_path in enumerate(sorted(by_source), 1):
        items = by_source[source_path]
        arches = sorted({item["architecture"] for item in items})
        architecture = "common" if len(arches) == 2 else arches[0]
        cls = source_class(source_path, items[0]["source_kind"])
        task_id = f"S{index:06d}"
        ids[source_path] = task_id
        object_targets = sorted({f"{item['architecture']}:{item['object_path']}" for item in items})
        destination_path = destination(source_path, cls)
        source_rows.append({
            "id": task_id, "linux_path": source_path, "destination_path": destination_path,
            "class": cls, "architectures": architecture,
            "kconfig_evidence": ";".join(f"config:{item['architecture']}=rewrite/configs/{'x86_64' if item['architecture']=='x86_64' else 'aarch64'}/frozen.config" for item in items),
            "kbuild_target": ";".join(object_targets), "cluster": source_path.split("/", 1)[0],
            "weight": str(weight(source_path, linux)), "risk": risk(source_path), "dependencies": "",
            "recommended_implementer": "luna", "source_kind": items[0]["source_kind"],
            "metadata_status": "COMPLETE", "metadata_evidence": "rewrite/metadata/manifest.tsv",
            "semantic_status": "NOT_APPLICABLE" if cls != "RUST_TRANSLATE" else "PENDING_REVIEW",
        })
        if cls == "RUST_TRANSLATE":
            task_rows.append({
                "id": task_id, "path": destination_path, "created_at": args.created_at,
                "status": "TODO", "linux_path": source_path, "architectures": architecture,
                "cluster": source_path.split("/", 1)[0], "weight": str(weight(source_path, linux)),
                "risk": risk(source_path), "recommended_implementer": "luna",
            })
    write_tsv(args.out / "SCOPE.tsv", SCOPE_FIELDS, source_rows)
    file_rows = []
    for item in sorted(all_entries, key=lambda row: (row["architecture"], row["source_path"], row["object_path"], row["compile_command"])):
        file_rows.append({**item, "module_or_builtin": disposition(item, (args.x86_build if item["architecture"] == "x86_64" else args.arm_build), module_stems(args.x86_build if item["architecture"] == "x86_64" else args.arm_build)), "metadata_evidence": "rewrite/metadata/manifest.tsv"})
    write_tsv(args.out / "FILE_MAP.tsv", FILE_MAP_FIELDS, file_rows)
    task_rows.sort(key=lambda row: row["id"])
    for row in task_rows:
        row.update({field: "" for field in TASK_FIELDS if field not in row})
    write_tsv(args.out / "TRANSLATION_TASKS.tsv", TASK_FIELDS, task_rows)
    semantic_rows = []
    for row in source_rows:
        if row["class"] == "RUST_TRANSLATE":
            for filename, field in (("SYMBOLS.tsv", "symbol_name"), ("ABI.tsv", "abi_item"), ("LIFETIMES.tsv", "lifetime_item")):
                semantic_rows.append((filename, {"scope_id": row["id"], "linux_path": row["linux_path"], "architectures": row["architectures"], "record_kind": "mechanical_file_record", field: "PENDING_REVIEW", "evidence": "rewrite/metadata/manifest.tsv", "status": "PENDING_REVIEW"}))
        elif row["class"] == "LINUX_DRIVER_OBJECT":
            semantic_rows.append(("DRIVER_ABI.tsv", {"scope_id": row["id"], "linux_path": row["linux_path"], "architectures": row["architectures"], "object_path": row["kbuild_target"], "record_kind": "driver_abi_contract", "abi_item": "PENDING_REVIEW", "evidence": "rewrite/metadata/manifest.tsv", "status": "PENDING_REVIEW"}))
    for filename, fields, key in (("SYMBOLS.tsv", ["scope_id", "linux_path", "architectures", "record_kind", "symbol_name", "evidence", "status"], "symbol_name"), ("ABI.tsv", ["scope_id", "linux_path", "architectures", "record_kind", "abi_item", "evidence", "status"], "abi_item"), ("LIFETIMES.tsv", ["scope_id", "linux_path", "architectures", "record_kind", "lifetime_item", "evidence", "status"], "lifetime_item"), ("DRIVER_ABI.tsv", ["scope_id", "linux_path", "architectures", "object_path", "record_kind", "abi_item", "evidence", "status"], "abi_item")):
        rows = [row for name, row in semantic_rows if name == filename]
        write_tsv(args.out / filename, fields, rows)
    manifest_rows = []
    for path in sorted((args.out / "metadata").rglob("*")):
        if path.is_file():
            manifest_rows.append({"path": path.relative_to(args.out).as_posix(), "sha256": sha256(path)})
    write_tsv(args.out / "metadata" / "manifest.tsv", ["path", "sha256"], manifest_rows)
    summary = {"linux_commit": Path("vendor/linux.SHA").read_text().strip(), "sources_total": len(source_rows), "rust_translate": sum(row["class"] == "RUST_TRANSLATE" for row in source_rows), "by_class": {}}
    for row in source_rows:
        summary["by_class"][row["class"]] = summary["by_class"].get(row["class"], 0) + 1
    (args.out / "metadata" / "summary.json").write_text(json.dumps(summary, sort_keys=True, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
