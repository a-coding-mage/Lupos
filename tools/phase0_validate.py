#!/usr/bin/env python3
"""Independent, read-only validator for the frozen Phase 0 identity and queue."""

from __future__ import annotations

import csv
import hashlib
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
LLVM_ROOT = Path("/usr/lib/llvm-19/bin")


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def check(checks: dict[str, dict[str, object]], name: str, ok: bool, detail: str) -> None:
    checks[name] = {"ok": bool(ok), "detail": detail}


def main() -> int:
    checks: dict[str, dict[str, object]] = {}
    branch = subprocess.check_output(["git", "branch", "--show-current"], cwd=ROOT, text=True).strip()
    check(checks, "branch", branch == "feat/bun-like-rewrite-test", branch)
    head = subprocess.check_output(["git", "-C", "vendor/linux", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    pinned = (ROOT / "vendor/linux.SHA").read_text().strip()
    status = subprocess.check_output(["git", "-C", "vendor/linux", "status", "--short"], cwd=ROOT, text=True)
    check(checks, "linux_pin", head == pinned, f"HEAD={head}; pinned={pinned}")
    check(checks, "linux_clean", not status, status or "clean")

    tool_rows = read_tsv(ROOT / "rewrite/toolchain/TOOLCHAIN.tsv")
    required = {row["tool_name"]: row for row in tool_rows}
    tool_ok = True
    for name, row in required.items():
        path = Path(row["requested_path"])
        resolved = path.resolve() if path.exists() else Path("missing")
        valid = (
            row["status"] == "VERIFIED" and path.is_file() and path.stat().st_mode & 0o111
            and resolved.is_relative_to(LLVM_ROOT)
            and row["major_version"] == "19"
            and digest(path) == row["sha256"]
        )
        tool_ok &= valid
        check(checks, f"tool:{name}", valid, f"requested={path}; resolved={resolved}; recorded_major={row['major_version']}")
    check(checks, "toolchain_file_hash", digest(ROOT / "rewrite/toolchain/TOOLCHAIN.tsv") == (ROOT / "rewrite/toolchain/TOOLCHAIN.sha256").read_text().split()[0], "TOOLCHAIN.sha256")
    linkers = read_tsv(ROOT / "rewrite/toolchain/LINKER_INVENTORY.tsv")
    selected = [row for row in linkers if row["selected"] == "YES"]
    check(checks, "selected_linker", len(selected) == 1 and selected[0]["resolved_path"] == "/usr/lib/llvm-19/bin/lld" and not ".rustup" in selected[0]["resolved_path"], str(selected))
    check(checks, "rust_linker_rejected", not any(row["selected"] == "YES" and ("rust-lld" in row["resolved_path"] or ".rustup" in row["resolved_path"]) for row in linkers), "LINKER_INVENTORY.tsv")

    identity = {row["key"]: row for row in read_tsv(ROOT / "rewrite/PHASE0_IDENTITY.tsv")}
    for arch, config in (("x86_64", ROOT / "rewrite/configs/x86_64/frozen.config"), ("aarch64", ROOT / "rewrite/configs/aarch64/frozen.config")):
        expected = identity[f"{arch}_config_sha256"]["value"]
        check(checks, f"{arch}_config_hash", config.exists() and digest(config) == expected, f"expected={expected}; actual={digest(config) if config.exists() else 'missing'}")
        build_config = ROOT / "rewrite/kbuild" / arch / ".config"
        check(checks, f"{arch}_build_config", build_config.exists() and config.read_bytes() == build_config.read_bytes(), str(build_config))
        transition = read_tsv(ROOT / "rewrite/configs" / arch / "config-transition.tsv")
        check(checks, f"{arch}_stable_transition", any(row.get("status") == "STABLE" and row.get("before") == "0_changed_symbols" for row in transition), str(transition))
    check(checks, "identity_hash", digest(ROOT / "rewrite/PHASE0_IDENTITY.tsv") == (ROOT / "rewrite/PHASE0_IDENTITY.sha256").read_text().split()[0], "PHASE0_IDENTITY.sha256")

    scope = read_tsv(ROOT / "rewrite/SCOPE.tsv")
    fmap = read_tsv(ROOT / "rewrite/FILE_MAP.tsv")
    tasks = read_tsv(ROOT / "rewrite/TRANSLATION_TASKS.tsv")
    scope_by_path = {row["linux_path"]: row for row in scope}
    fmap_paths = {row["source_path"] for row in fmap}
    rust_scope = {row["linux_path"]: row for row in scope if row["class"] == "RUST_TRANSLATE"}
    task_by_linux = {row["linux_path"]: row for row in tasks}
    check(checks, "scope_classified", len(scope) == len(scope_by_path) and all(row["class"] for row in scope), f"rows={len(scope)}")
    check(checks, "object_source_mapping", all(row["source_path"] and row["object_path"] for row in fmap), f"rows={len(fmap)}")
    check(checks, "scope_in_file_map", set(scope_by_path) <= fmap_paths, f"missing={sorted(set(scope_by_path)-fmap_paths)[:5]}")
    check(checks, "rust_task_bijection", set(rust_scope) == set(task_by_linux) and len(tasks) == len(rust_scope), f"scope={len(rust_scope)} tasks={len(tasks)}")
    check(checks, "driver_exclusion", not any(row["class"] == "LINUX_DRIVER_OBJECT" for row in scope if row["linux_path"] in task_by_linux), "driver paths are not queued")
    check(checks, "unique_task_ids_paths", len({row["id"] for row in tasks}) == len(tasks) and len({row["path"] for row in tasks}) == len(tasks), "queue IDs and paths")
    check(checks, "all_todo", all(row["status"] == "TODO" and not row["lease_owner"] and not row["pipeline_id"] for row in tasks), "queue lifecycle fields")
    check(checks, "src_empty", not any((ROOT / "src").rglob("*")), "src")

    for arch in ("x86_64", "aarch64"):
        database = ROOT / "rewrite/metadata" / arch / "compile_commands.json"
        try:
            count = len(json.loads(database.read_text()))
        except Exception as exc:
            count = 0
            detail = repr(exc)
        else:
            detail = str(count)
        check(checks, f"{arch}_metadata_complete", count > 0 and (ROOT / "rewrite/metadata" / arch / "cmd_inventory.tsv").exists() and (ROOT / "rewrite/metadata" / arch / "depfile_inventory.tsv").exists(), detail)
        contents = "\n".join((ROOT / "rewrite/metadata" / arch / name).read_text(errors="replace") for name in ("compile_commands.json", "build.log"))
        check(checks, f"{arch}_compiler_binding", "/usr/lib/llvm-19/bin/clang" in contents and "rust-lld" not in contents and ".rustup" not in contents, "compiler/linker evidence")

    queue_verify = subprocess.run([sys.executable, "tools/rewrite_queue.py", "verify"], cwd=ROOT, text=True, capture_output=True)
    check(checks, "queue_verify", queue_verify.returncode == 0, queue_verify.stdout + queue_verify.stderr)
    fingerprint = (ROOT / "rewrite/TRANSLATION_TASKS.sha256").read_text().split()[1]
    check(checks, "identity_queue_binding", identity["queue_fingerprint"]["value"] == fingerprint, f"identity={identity['queue_fingerprint']['value']}; queue={fingerprint}")
    report = {"ok": all(item["ok"] for item in checks.values()), "checks": checks}
    (ROOT / "rewrite/PHASE0_VALIDATION.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    with (ROOT / "rewrite/PHASE0_VALIDATION.tsv").open("w", encoding="utf-8") as handle:
        handle.write("check\tstatus\tdetail\n")
        for name, item in checks.items():
            handle.write(f"{name}\t{'PASS' if item['ok'] else 'FAIL'}\t{str(item['detail']).replace(chr(9), ' ')}\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
