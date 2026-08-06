#!/usr/bin/env python3
"""Generate the authoritative Phase 0 identity and its stable queue binding.

The queue is frozen after the source manifests but before the final identity
records the queue digest.  ``phase0_identity_binding_sha256`` is therefore a
digest of every immutable non-queue Phase 0 input.  The queue digest includes
that binding, and the final identity in turn includes the queue digest, without
introducing a circular hash.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import tempfile


IDENTITY_FIELDS = ["key", "value", "status", "evidence"]
ARCHES = ("x86_64", "aarch64")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def now_utc() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def read_tsv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if reader.fieldnames is None:
            raise ValueError(f"missing TSV header: {path}")
        return [dict(row) for row in reader]


def key_values(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("\t")
        if separator:
            result[key] = value
    return result


def checksum_value(path: Path) -> str:
    """Read either the project TSV checksum form or standard sha256sum form."""

    values = key_values(path)
    if values.get("sha256"):
        return values["sha256"]
    fields = path.read_text(encoding="utf-8").split()
    if not fields:
        raise ValueError(f"empty checksum file: {path}")
    return fields[0]


def atomic_write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent, text=True)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if temporary.exists():
            temporary.unlink()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--artifacts", type=Path, default=Path("rewrite"))
    parser.add_argument("--identity", type=Path, default=None)
    parser.add_argument("--queue-fingerprint", type=Path, default=None)
    args = parser.parse_args()

    root = args.root.resolve()
    artifacts = args.artifacts if args.artifacts.is_absolute() else root / args.artifacts
    canonical = root / "rewrite"
    identity = args.identity or (artifacts / "PHASE0_IDENTITY.tsv")
    identity = identity if identity.is_absolute() else root / identity
    identity_hash = identity.with_suffix(".sha256")

    linux_sha = (root / "vendor/linux.SHA").read_text(encoding="utf-8").strip()
    require(len(linux_sha) == 40, "vendor/linux.SHA must contain one 40-character commit")
    config_hashes = {
        arch: sha256(canonical / "configs" / arch / "frozen.config") for arch in ARCHES
    }
    toolchain_path = canonical / "toolchain/TOOLCHAIN.tsv"
    toolchain_fingerprint = checksum_value(canonical / "toolchain/TOOLCHAIN.sha256")
    require(toolchain_fingerprint == sha256(toolchain_path), "toolchain fingerprint mismatch")
    tools = {row["tool_name"]: row for row in read_tsv(toolchain_path)}
    clang = tools.get("clang", {})
    linker = tools.get("ld.lld", {})
    require(
        clang.get("requested_path") == "/usr/lib/llvm-19/bin/clang"
        and clang.get("resolved_path") == "/usr/lib/llvm-19/bin/clang"
        and clang.get("major_version") == "19",
        "frozen clang identity is unavailable",
    )
    require(
        linker.get("requested_path") == "/usr/lib/llvm-19/bin/ld.lld"
        and linker.get("resolved_path") == "/usr/lib/llvm-19/bin/lld"
        and linker.get("major_version") == "19",
        "frozen ld.lld identity is unavailable",
    )
    environment = {(row["architecture"], row["key"]): row["value"] for row in read_tsv(canonical / "toolchain/ENVIRONMENT.tsv")}

    predicate_root = canonical / "compiler-predicates"
    predicates = read_tsv(predicate_root / "COMPILER_PREDICATES.tsv")
    predicate_fingerprint = key_values(predicate_root / "COMPILER_PREDICATES.sha256")
    require(predicate_fingerprint.get("sha256") == sha256(predicate_root / "COMPILER_PREDICATES.tsv"), "compiler predicate fingerprint mismatch")
    validation = {row.get("predicate_id", ""): row.get("validation_status", "") for row in read_tsv(predicate_root / "VALIDATION.tsv")}
    counts = {arch: 0 for arch in ARCHES}
    triples: dict[str, str] = {}
    for row in predicates:
        arch = row.get("architecture", "")
        require(arch in counts, f"unexpected predicate architecture: {arch}")
        require(row.get("status") == "PROVEN" and row.get("result") in {"0", "1"}, f"unproven compiler predicate: {row.get('predicate_id')}")
        require(validation.get(row.get("predicate_id", "")) == "PASS", f"unvalidated compiler predicate: {row.get('predicate_id')}")
        require(row.get("linux_commit") == linux_sha, f"predicate Linux revision mismatch: {row.get('predicate_id')}")
        require(row.get("config_sha256") == config_hashes[arch], f"predicate configuration mismatch: {row.get('predicate_id')}")
        require(row.get("toolchain_sha256") == toolchain_fingerprint, f"predicate toolchain mismatch: {row.get('predicate_id')}")
        require(row.get("compiler_sha256") == clang.get("sha256"), f"predicate compiler mismatch: {row.get('predicate_id')}")
        counts[arch] += 1
        triples.setdefault(arch, row.get("target_triple", ""))
        require(triples[arch] == row.get("target_triple", ""), f"inconsistent predicate target triple for {arch}")
    require(all(counts.values()), "compiler predicate inventory lacks an architecture")

    binding_path = artifacts / "metadata/compiler-predicates-binding.tsv"
    binding = {row["key"]: row["value"] for row in read_tsv(binding_path)}
    require(binding.get("compiler_predicates_sha256") == sha256(predicate_root / "COMPILER_PREDICATES.tsv"), "staged predicate binding mismatch")

    manifest_paths = [
        "SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv", "LIFETIMES.tsv",
        "DRIVER_ABI.tsv", "PORTING.md", "BRANDING_ALLOWLIST.tsv",
    ]
    for name in manifest_paths:
        require((artifacts / name).is_file(), f"missing authoritative manifest: {name}")
    authoritative_manifest = artifacts / "metadata/authoritative_manifests.tsv"
    metadata_manifest = artifacts / "metadata/manifest.tsv"
    require(authoritative_manifest.is_file() and metadata_manifest.is_file(), "missing staged metadata manifests")

    values: dict[str, str] = {
        "identity_schema_version": "phase0-identity-v2",
        "linux_commit": linux_sha,
        "x86_64_config_sha256": config_hashes["x86_64"],
        "aarch64_config_sha256": config_hashes["aarch64"],
        "toolchain_sha256": toolchain_fingerprint,
        "compiler_requested_path": clang["requested_path"],
        "compiler_resolved_path": clang["resolved_path"],
        "compiler_sha256": clang["sha256"],
        "compiler_version": clang["version"],
        "linker_requested_path": linker["requested_path"],
        "linker_resolved_path": linker["resolved_path"],
        "linker_sha256": linker["sha256"],
        "linker_version": linker["version"],
        "llvm_value": environment[("common", "LLVM")],
        "llvm_ias_value": environment[("common", "LLVM_IAS")],
        "cross_compile_value": environment[("common", "CROSS_COMPILE")],
        "x86_64_arch_value": environment[("x86_64", "ARCH")],
        "aarch64_arch_value": environment[("aarch64", "ARCH")],
        "x86_64_target_triple": triples["x86_64"],
        "aarch64_target_triple": triples["aarch64"],
        "extractor_version": f"phase0_extract.py:{sha256(root / 'tools/phase0_extract.py')}",
        "validator_version": f"phase0_validate.py:{sha256(root / 'tools/phase0_validate.py')}",
        "queue_tool_version": f"rewrite_queue.py:{sha256(root / 'tools/rewrite_queue.py')}",
        "predicate_extractor_version": f"compiler_predicates.py:{sha256(root / 'tools/compiler_predicates.py')}",
        "predicate_validator_version": f"validate_compiler_predicates.py:{sha256(root / 'tools/validate_compiler_predicates.py')}",
        "scope_schema_version": "source-header-context-oracle-phase0-v7",
        "header_dependency_schema_version": "header-provider-enumerator-graph-v3",
        "oracle_classification_schema_version": "oracle-classification-v1",
        "compiler_predicates_sha256": sha256(predicate_root / "COMPILER_PREDICATES.tsv"),
        "compiler_predicates_schema_version": "compiler-predicates-v1",
        "compiler_predicates_count": str(len(predicates)),
        "compiler_predicates_x86_64_count": str(counts["x86_64"]),
        "compiler_predicates_aarch64_count": str(counts["aarch64"]),
        "compiler_predicates_validation_status": "PASS",
        "authoritative_manifests_sha256": sha256(authoritative_manifest),
        "metadata_manifest_sha256": sha256(metadata_manifest),
    }
    binding_digest = hashlib.sha256(
        json.dumps(values, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    values["phase0_identity_binding_sha256"] = binding_digest

    queue_fingerprint = ""
    if args.queue_fingerprint is not None:
        queue_path = args.queue_fingerprint if args.queue_fingerprint.is_absolute() else root / args.queue_fingerprint
        queue_values = key_values(queue_path)
        queue_fingerprint = queue_values.get("sha256", "")
        require(len(queue_fingerprint) == 64, f"invalid queue fingerprint: {queue_path}")
        require(queue_values.get("linux_sha") == linux_sha, "queue Linux binding mismatch")
        require(queue_values.get("phase0_identity_binding_sha256") == binding_digest, "queue Phase 0 identity binding mismatch")
    values["queue_fingerprint"] = queue_fingerprint
    values["identity_status"] = "RESOLVED" if queue_fingerprint else "PRE_QUEUE"
    values["created_at"] = now_utc()

    evidence = {
        "linux_commit": "vendor/linux.SHA and vendor/linux HEAD",
        "compiler_predicates_sha256": "rewrite/compiler-predicates/COMPILER_PREDICATES.tsv",
        "phase0_identity_binding_sha256": "canonical non-queue Phase 0 input digest",
        "queue_fingerprint": "rewrite/TRANSLATION_TASKS.sha256" if queue_fingerprint else "pending queue freeze",
    }
    ordered = sorted(values)
    lines = ["\t".join(IDENTITY_FIELDS)]
    for key in ordered:
        lines.append("\t".join([key, values[key], "VERIFIED", evidence.get(key, "Phase 0 frozen input")]))
    content = "\n".join(lines) + "\n"
    atomic_write(identity, content)
    atomic_write(identity_hash, f"{sha256(identity)}  {identity}\n")
    print(json.dumps({
        "identity": str(identity),
        "sha256": sha256(identity),
        "phase0_identity_binding_sha256": binding_digest,
        "queue_fingerprint": queue_fingerprint,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
