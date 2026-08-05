#!/usr/bin/env python3
"""Independent validator for source-only Phase 0 manifests and queue freezing."""

from __future__ import annotations

import argparse
from collections import defaultdict
import csv
import hashlib
import json
from pathlib import Path
import re
import shlex
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
LLVM_ROOT = Path("/usr/lib/llvm-19/bin")
ARCHES = ("x86_64", "aarch64")
CONFIG_EVIDENCE = {
    "x86_64": "rewrite/configs/x86_64/frozen.config",
    "aarch64": "rewrite/configs/aarch64/frozen.config",
}
ENTITY_KINDS = {"function", "function_macro", "type", "static", "global", "export"}
LIFETIME_KINDS = {"function", "function_macro", "type", "static", "global"}


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
    text = "".join(unconditional_lines)
    categories: set[str] = set()
    if re.search(r"(?m)^\s*#\s*define\s+[A-Za-z_]", text):
        categories.add("operative_macro")
    if has_conditional:
        categories.add("conditional")
    if re.search(r"(?m)^static\s+", text):
        categories.add("static")
    if re.search(r"\b(?:struct|union|enum)\s+[A-Za-z_]\w*\s*\{", text) or re.search(
        r"(?ms)^\s*typedef\b.{0,4096}?;", text
    ):
        categories.add("type")
    function_pattern = re.compile(
        r"(?ms)^\s*(?!if\b|for\b|while\b|switch\b)"
        r"(?:[A-Za-z_][A-Za-z0-9_]*[\s\*]+)+"
        r"[A-Za-z_][A-Za-z0-9_]*\s*\([^;{}]{0,8192}\)\s*"
        r"(?:[A-Za-z_][A-Za-z0-9_]*(?:\([^;{}]*\))?\s*)*\{"
    )
    if function_pattern.search(text) or re.search(
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--artifacts", type=Path, default=Path("rewrite"))
    parser.add_argument("--stage", choices=("pre-queue", "frozen"), default="frozen")
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

    scope = required_fields(
        checks,
        "scope_schema",
        artifacts / "SCOPE.tsv",
        {"id", "linux_path", "destination_path", "class", "architectures", "kconfig_evidence", "kbuild_target", "dependencies"},
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
        {"scope_id", "linux_path", "architectures", "record_kind", "symbol_name", "source_line", "selection_expression", "config_evidence", "linkage", "declaration", "evidence", "status"},
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
    check(checks, "file_map_unique", len(fmap_keys) == len(fmap), f"rows={len(fmap)} keys={len(fmap_keys)}")
    check(
        checks,
        "scope_in_file_map",
        set(scope_by_path) <= {row.get("source_path", "") for row in fmap},
        sorted(set(scope_by_path) - {row.get("source_path", "") for row in fmap})[:10],
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
        for key in (item for item in fmap_keys if item[0] == arch):
            expected = inventory_map.get(key)
            actual = next((row for row in fmap if (row.get("architecture"), row.get("source_path"), row.get("object_path")) == key), None)
            if expected is None or actual is None:
                inventory_errors.append(f"{key}:missing")
            elif any(expected.get(field) != actual.get(field) for field in ("module_or_builtin", "kbuild_owner", "disposition_evidence")):
                inventory_errors.append(f"{key}:contradictory")
    check(checks, "object_inventory_matches_file_map", not inventory_errors, inventory_errors[:20])

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
        if (
            not row.get("source_line", "").isdigit()
            or not row.get("selection_expression")
            or row.get("config_evidence") != expected_config
            or not row.get("evidence")
            or row.get("status") != "COMPLETE"
            or row.get("record_kind") not in {"function", "function_macro", "type", "static", "global", "export", "operative_macro", "conditional"}
            or (row.get("record_kind") == "conditional" and "selected=" not in row.get("evidence", ""))
        ):
            malformed_symbol_rows.append((key, row.get("record_kind"), row.get("symbol_name")))
    check(checks, "semantic_no_file_placeholders", not placeholder_rows, placeholder_rows[:20])
    check(checks, "symbols_mechanical_fields", not malformed_symbol_rows, malformed_symbol_rows[:20])

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
            if not records or missing:
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
    required_manifests = {"SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv", "LIFETIMES.tsv", "DRIVER_ABI.tsv"}
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

    check(
        checks,
        "src_empty_at_first_init",
        not any(path.is_file() for path in (root / "src").rglob("*")),
        root / "src",
    )

    if args.stage == "pre-queue":
        queue_absent = not (artifacts / "TRANSLATION_TASKS.tsv").exists() and not (artifacts / "TRANSLATION_TASKS.sha256").exists()
        check(checks, "queue_absent_before_init", queue_absent, artifacts)
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
        check(
            checks,
            "scope_schema_identity",
            identity.get("scope_schema_version", {}).get("value") == "source-symbol-phase0-v3",
            identity.get("scope_schema_version", {}).get("value"),
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
                and not row.get("work_started_at")
                and not row.get("lease_owner")
                and not row.get("pipeline_id")
                for row in task_rows
            ),
            "all rows TODO and unleased",
        )
        check(
            checks,
            "driver_exclusion",
            not any(scope_by_id.get(row.get("id", ""), {}).get("class") == "LINUX_DRIVER_OBJECT" for row in task_rows),
            "driver rows must not be queued",
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

    report = {"ok": all(item["ok"] for item in checks.values()), "stage": args.stage, "checks": checks}
    if not args.no_write_report:
        (artifacts / "PHASE0_VALIDATION.json").write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        with (artifacts / "PHASE0_VALIDATION.tsv").open("w", encoding="utf-8") as handle:
            handle.write("check\tstatus\tdetail\n")
            for name, item in checks.items():
                handle.write(
                    f"{name}\t{'PASS' if item['ok'] else 'FAIL'}\t{item['detail'].replace(chr(9), ' ')}\n"
                )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
