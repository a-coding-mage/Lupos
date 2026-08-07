#!/usr/bin/env python3
"""Transactional per-task closure for Phase 0 semantic PENDING_REVIEW fields.

The mechanically generated Phase 0 manifests are immutable inputs.  This tool
derives stable field keys from those frozen bytes and records reviewed Phase 1
decisions in an append-only prepare/commit ledger.  It never rewrites a base
manifest.  Mutating commands share rewrite_queue.py's OS lock and branch gate.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from typing import Iterable, Mapping


EXPECTED_BRANCH = "feat/bun-like-rewrite-test"
SCHEMA_VERSION = "semantic-closure-v1"
KEY_SCHEMA_VERSION = "semantic-field-key-v1"
BASE_SCHEMA_VERSION = "semantic-base-v1"
LEDGER_SCHEMA_VERSION = "semantic-ledger-v1"
PROPOSAL_SCHEMA_VERSION = "semantic-proposal-v1"
REVIEW_SCHEMA_VERSION = "semantic-review-attestation-v1"
DISPOSITION_SCHEMA_VERSION = "semantic-dispositions-v1"
COMMIT_SCHEMA_VERSION = "semantic-commit-v1"

DEFAULT_QUEUE = Path("rewrite/TRANSLATION_TASKS.tsv")
DEFAULT_FINGERPRINT = Path("rewrite/TRANSLATION_TASKS.sha256")
DEFAULT_IDENTITY = Path("rewrite/PHASE0_IDENTITY.tsv")
DEFAULT_EVENTS = Path("rewrite/events.jsonl")
DEFAULT_LOGS = Path("rewrite/logs/tasks")
DEFAULT_ROOT = Path("rewrite/semantic-closure")
DEFAULT_SCHEMA = DEFAULT_ROOT / "SCHEMA.tsv"
DEFAULT_BASE = DEFAULT_ROOT / "BASE.tsv"
DEFAULT_LEDGER = DEFAULT_ROOT / "LEDGER.jsonl"

BASE_MANIFESTS = ("SCOPE.tsv", "SYMBOLS.tsv", "ABI.tsv", "LIFETIMES.tsv")
PENDING_FIELDS = {
    "SCOPE.tsv": ("semantic_status",),
    "SYMBOLS.tsv": ("selection_expression", "mechanical_value", "status"),
    "ABI.tsv": (
        "abi_item", "linkage", "export_kind", "declaration", "layout",
        "alignment", "calling_convention", "status",
    ),
    "LIFETIMES.tsv": (
        "lifetime_item", "storage_duration", "ownership", "lifetime_contract",
        "locking_rcu_refcount", "status",
    ),
}

SCHEMA_FIELDS = ("key", "value", "status", "evidence")
BASE_FIELDS = ("key", "value", "status", "evidence")
PROPOSAL_FIELDS = (
    "schema_version", "record_key", "task_id", "attempt", "pipeline_id",
    "manifest", "base_row", "field", "architecture", "linux_path",
    "record_kind", "symbol_name", "source_line", "old_value", "final_value",
    "decision_status", "source_citations", "linux_sha", "candidate_sha256",
    "implementation_sha256", "phase0_identity_sha256", "queue_fingerprint",
    "base_manifest_sha256",
)
PROPOSAL_SEAL_FIELDS = (
    "schema_version", "sha256", "task_id", "attempt", "pipeline_id",
    "records", "queue_fingerprint", "phase0_identity_sha256", "sealed_at",
)
REVIEW_FIELDS = (
    "schema_version", "task_id", "attempt", "pipeline_id", "slot",
    "proposal_sha256", "report_path", "report_sha256", "review_status",
    "reviewer", "model", "reasoning_effort", "reviewed_at", "finding_id",
    "record_keys",
)
DISPOSITION_FIELDS = (
    "finding_id", "source_slot", "record_keys", "disposition",
    "source_citations", "detail",
)

SEMANTIC_EVIDENCE_FILES = (
    "semantic-closure-proposal.tsv",
    "semantic-closure-proposal.sha256",
    "semantic-closure-parity-review.tsv",
    "semantic-closure-rust-review.tsv",
    "semantic-closure-final.tsv",
    "semantic-closure-dispositions.tsv",
    "semantic-closure-commit.json",
)

HEX64 = re.compile(r"[0-9a-f]{64}")
TASK_ID = re.compile(r"S[0-9]{6}")
FINDING_ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,127}")


def die(message: str, code: int = 2) -> "NoReturn":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(code)


def now_utc() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


def ensure_branch() -> None:
    try:
        branch = subprocess.check_output(
            ["git", "branch", "--show-current"], text=True, stderr=subprocess.STDOUT
        ).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        die(f"cannot verify Git branch: {exc}")
    if branch != EXPECTED_BRANCH:
        die(f"semantic closure mutation requires branch {EXPECTED_BRANCH!r}; found {branch!r}")


def ensure_root() -> None:
    try:
        root = Path(subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True,
            stderr=subprocess.STDOUT,
        ).strip()).resolve()
    except (OSError, subprocess.CalledProcessError) as exc:
        die(f"cannot locate repository root: {exc}")
    if Path.cwd().resolve() != root:
        die(f"run semantic_closure.py from repository root {root}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_tsv(path: Path, fields: tuple[str, ...] | None = None) -> list[dict[str, str]]:
    if not path.is_file() or path.is_symlink():
        die(f"missing non-symlink TSV: {path}")
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        actual = tuple(reader.fieldnames or ())
        if fields is not None and actual != fields:
            die(f"unexpected schema in {path}; expected {fields}, found {actual}")
        return [dict(row) for row in reader]


def key_values(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("\t")
        if separator:
            result[key] = value
    return result


def atomic_write(path: Path, content: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        fsync_directory(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


def atomic_write_tsv(path: Path, fields: tuple[str, ...], records: Iterable[Mapping[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent, text=True)
    temporary = Path(name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n", extrasaction="raise")
            writer.writeheader()
            writer.writerows({field: str(row.get(field, "")) for field in fields} for row in records)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        fsync_directory(path.parent)
    finally:
        temporary.unlink(missing_ok=True)


def fsync_directory(path: Path) -> None:
    descriptor = os.open(path, os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def append_jsonl(path: Path, record: Mapping[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n"
    with path.open("a", encoding="utf-8") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


class QueueLock:
    def __init__(self, queue: Path) -> None:
        self.path = queue.parent / ".translation_tasks.lock"
        self.handle = None

    def __enter__(self) -> "QueueLock":
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.handle = self.path.open("a+", encoding="utf-8")
        fcntl.flock(self.handle.fileno(), fcntl.LOCK_EX)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        assert self.handle is not None
        fcntl.flock(self.handle.fileno(), fcntl.LOCK_UN)
        self.handle.close()


def schema_rows() -> list[dict[str, str]]:
    values = {
        "schema_version": SCHEMA_VERSION,
        "key_schema_version": KEY_SCHEMA_VERSION,
        "base_schema_version": BASE_SCHEMA_VERSION,
        "ledger_schema_version": LEDGER_SCHEMA_VERSION,
        "proposal_schema_version": PROPOSAL_SCHEMA_VERSION,
        "review_schema_version": REVIEW_SCHEMA_VERSION,
        "disposition_schema_version": DISPOSITION_SCHEMA_VERSION,
        "commit_schema_version": COMMIT_SCHEMA_VERSION,
        "base_manifests": ";".join(BASE_MANIFESTS),
        "key_algorithm": (
            "SC1- + sha256(canonical-json(key_schema_version,manifest,"
            "base_manifest_sha256,base_row,field,task_id))"
        ),
        "task_ownership": "SCOPE.id=queue.id; semantic manifest scope_id=queue.id",
        "pending_rule": "one required closure record for every task-owned allowed field exactly equal to PENDING_REVIEW",
        "effective_rule": "a field is closed only by a PREPARE record followed by matching event and COMMIT in the current queue generation",
        "mutable_layer": "LEDGER.jsonl contents are append-only and excluded from Phase 0 identity hashes",
        "review_isolation": "each review command reads only the sealed proposal and its own fixed report",
        "proposal_fields": ";".join(PROPOSAL_FIELDS),
        "proposal_seal_fields": ";".join(PROPOSAL_SEAL_FIELDS),
        "review_fields": ";".join(REVIEW_FIELDS),
        "disposition_fields": ";".join(DISPOSITION_FIELDS),
        "semantic_evidence_files": ";".join(SEMANTIC_EVIDENCE_FILES),
    }
    return [
        {"key": key, "value": values[key], "status": "FROZEN", "evidence": "tools/semantic_closure.py"}
        for key in sorted(values)
    ]


def record_key(manifest: str, manifest_sha: str, base_row: int, field: str, task_id: str) -> str:
    canonical = {
        "key_schema_version": KEY_SCHEMA_VERSION,
        "manifest": manifest,
        "base_manifest_sha256": manifest_sha,
        "base_row": base_row,
        "field": field,
        "task_id": task_id,
    }
    encoded = json.dumps(canonical, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return "SC1-" + hashlib.sha256(encoded).hexdigest()


def scope_task_ids(rewrite: Path) -> tuple[set[str], dict[str, dict[str, str]]]:
    scope = read_tsv(rewrite / "SCOPE.tsv")
    selected = {row.get("id", ""): row for row in scope if row.get("class") == "RUST_TRANSLATE"}
    if not selected or any(not TASK_ID.fullmatch(task_id) for task_id in selected):
        die("SCOPE.tsv has an invalid RUST_TRANSLATE task inventory")
    return set(selected), selected


def expected_closure_records(rewrite: Path, task_id: str | None = None) -> list[dict[str, str]]:
    task_ids, _ = scope_task_ids(rewrite)
    if task_id is not None and task_id not in task_ids:
        die(f"unknown RUST_TRANSLATE task: {task_id}")
    result: list[dict[str, str]] = []
    for manifest in BASE_MANIFESTS:
        path = rewrite / manifest
        manifest_sha = sha256_file(path)
        records = read_tsv(path)
        allowed = set(PENDING_FIELDS[manifest])
        for base_row, row in enumerate(records, 2):
            owner = row.get("id", "") if manifest == "SCOPE.tsv" else row.get("scope_id", "")
            pending = {field for field, value in row.items() if value == "PENDING_REVIEW"}
            if owner in task_ids and pending - allowed:
                die(f"{manifest}:{base_row} has PENDING_REVIEW in non-closure fields: {sorted(pending - allowed)}")
            if owner not in task_ids or (task_id is not None and owner != task_id):
                continue
            for field in PENDING_FIELDS[manifest]:
                if row.get(field, "") != "PENDING_REVIEW":
                    continue
                result.append({
                    "record_key": record_key(manifest, manifest_sha, base_row, field, owner),
                    "task_id": owner,
                    "manifest": manifest,
                    "base_row": str(base_row),
                    "field": field,
                    "architecture": row.get("architectures", ""),
                    "linux_path": row.get("linux_path", ""),
                    "record_kind": row.get("record_kind", "scope"),
                    "symbol_name": row.get("symbol_name", owner),
                    "source_line": row.get("source_line", "0"),
                    "old_value": "PENDING_REVIEW",
                    "source_citations": (
                        f"vendor/linux/{row.get('linux_path', '')};"
                        f"{row.get('evidence', '') or row.get('metadata_evidence', '')}"
                    ),
                    "base_manifest_sha256": manifest_sha,
                })
    result.sort(key=lambda row: (BASE_MANIFESTS.index(row["manifest"]), int(row["base_row"]), row["field"]))
    return result


def base_rows(rewrite: Path, schema_sha: str) -> list[dict[str, str]]:
    expected = expected_closure_records(rewrite)
    manifest_rows: dict[str, int] = {}
    manifest_pending: dict[str, int] = {}
    hashes: dict[str, str] = {}
    for manifest in BASE_MANIFESTS:
        hashes[manifest] = sha256_file(rewrite / manifest)
        manifest_rows[manifest] = len(read_tsv(rewrite / manifest))
        manifest_pending[manifest] = sum(row["manifest"] == manifest for row in expected)
    keyset = hashlib.sha256()
    for record in expected:
        keyset.update((record["record_key"] + "\n").encode("ascii"))
    values = {
        "base_schema_version": BASE_SCHEMA_VERSION,
        "semantic_schema_sha256": schema_sha,
        "task_count": str(len(scope_task_ids(rewrite)[0])),
        "pending_field_count": str(len(expected)),
        "task_keyset_sha256": keyset.hexdigest(),
    }
    for manifest in BASE_MANIFESTS:
        prefix = manifest.removesuffix(".tsv").lower()
        values[f"{prefix}_sha256"] = hashes[manifest]
        values[f"{prefix}_rows"] = str(manifest_rows[manifest])
        values[f"{prefix}_pending_fields"] = str(manifest_pending[manifest])
    return [
        {"key": key, "value": values[key], "status": "FROZEN", "evidence": "mechanical base manifest scan"}
        for key in sorted(values)
    ]


def refresh_manifest_indexes(rewrite: Path) -> None:
    names = (
        "SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv",
        "LIFETIMES.tsv", "DRIVER_ABI.tsv", "PORTING.md",
        "BRANDING_ALLOWLIST.tsv", "semantic-closure/SCHEMA.tsv",
        "semantic-closure/BASE.tsv",
    )
    authoritative = [
        {"path": name, "sha256": sha256_file(rewrite / name)} for name in names
    ]
    atomic_write_tsv(rewrite / "metadata/authoritative_manifests.tsv", ("path", "sha256"), authoritative)
    manifest_path = rewrite / "metadata/manifest.tsv"
    metadata = [
        {"path": path.relative_to(rewrite).as_posix(), "sha256": sha256_file(path)}
        for path in sorted((rewrite / "metadata").rglob("*"))
        if path.is_file() and path != manifest_path
    ]
    atomic_write_tsv(manifest_path, ("path", "sha256"), metadata)


def initialize_phase0(rewrite: Path, *, phase_gate_reopen: bool) -> dict[str, str]:
    ensure_branch()
    schema_path = rewrite / "semantic-closure/SCHEMA.tsv"
    base_path = rewrite / "semantic-closure/BASE.tsv"
    ledger_path = rewrite / "semantic-closure/LEDGER.jsonl"
    if not phase_gate_reopen and any(path.exists() for path in (schema_path, base_path, ledger_path)):
        die("semantic closure Phase 0 artifacts already exist")
    schema_content_rows = schema_rows()
    atomic_write_tsv(schema_path, SCHEMA_FIELDS, schema_content_rows)
    schema_sha = sha256_file(schema_path)
    atomic_write_tsv(base_path, BASE_FIELDS, base_rows(rewrite, schema_sha))
    if ledger_path.exists() and ledger_path.stat().st_size:
        # Historical entries remain append-only.  A new fingerprint opens a
        # distinct clean generation during queue freeze.
        validate_ledger(ledger_path)
    elif not ledger_path.exists():
        atomic_write(ledger_path, b"")
    refresh_manifest_indexes(rewrite)
    base_values = {row["key"]: row["value"] for row in read_tsv(base_path, BASE_FIELDS)}
    return {
        "schema_sha256": sha256_file(schema_path),
        "base_sha256": sha256_file(base_path),
        "pending_fields": base_values["pending_field_count"],
    }


def validate_phase0_artifacts(rewrite: Path) -> dict[str, str]:
    schema_path = rewrite / "semantic-closure/SCHEMA.tsv"
    base_path = rewrite / "semantic-closure/BASE.tsv"
    schema = {row["key"]: row["value"] for row in read_tsv(schema_path, SCHEMA_FIELDS)}
    if schema != {row["key"]: row["value"] for row in schema_rows()}:
        die("semantic closure schema differs from tools/semantic_closure.py")
    expected = {row["key"]: row["value"] for row in base_rows(rewrite, sha256_file(schema_path))}
    actual = {row["key"]: row["value"] for row in read_tsv(base_path, BASE_FIELDS)}
    if actual != expected:
        changed = sorted(key for key in set(actual) | set(expected) if actual.get(key) != expected.get(key))
        die(f"semantic closure base binding mismatch: {changed[:20]}")
    validate_ledger(rewrite / "semantic-closure/LEDGER.jsonl")
    return actual


def validate_ledger(path: Path) -> list[dict[str, object]]:
    if not path.is_file() or path.is_symlink():
        die(f"missing append-only semantic closure ledger: {path}")
    records: list[dict[str, object]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as exc:
            die(f"malformed semantic ledger at {path}:{line_number}: {exc}")
        if not isinstance(record, dict) or record.get("schema_version") != LEDGER_SCHEMA_VERSION:
            die(f"invalid semantic ledger record at {path}:{line_number}")
        records.append(record)
    return records


def fingerprint_value(path: Path) -> str:
    value = key_values(path).get("sha256", "")
    if not HEX64.fullmatch(value):
        die(f"invalid queue fingerprint: {path}")
    return value


def identity_sha(path: Path) -> str:
    value = sha256_file(path)
    if not HEX64.fullmatch(value):
        die(f"invalid identity digest: {path}")
    return value


def validate_expected_hashes(args: argparse.Namespace, rewrite: Path) -> dict[str, str]:
    actual = {
        "scope": sha256_file(rewrite / "SCOPE.tsv"),
        "symbols": sha256_file(rewrite / "SYMBOLS.tsv"),
        "abi": sha256_file(rewrite / "ABI.tsv"),
        "lifetimes": sha256_file(rewrite / "LIFETIMES.tsv"),
        "identity": identity_sha(Path(args.identity)),
        "fingerprint": fingerprint_value(Path(args.fingerprint)),
    }
    for name, value in actual.items():
        expected = getattr(args, f"expected_{name}_sha256", "") if name != "fingerprint" else args.expected_queue_fingerprint
        if expected != value:
            die(f"expected {name} hash mismatch: expected {expected!r}, actual {value}")
    validate_phase0_artifacts(rewrite)
    return actual


def queue_row(queue: Path, task_id: str) -> dict[str, str]:
    matches = [row for row in read_tsv(queue) if row.get("id") == task_id]
    if len(matches) != 1:
        die(f"expected one queue row for {task_id}, found {len(matches)}")
    return matches[0]


def require_task_context(row: Mapping[str, str], *, attempt: int, pipeline: str, statuses: set[str]) -> None:
    if row.get("status") not in statuses:
        die(f"task {row.get('id')} must be in {sorted(statuses)}, found {row.get('status')}")
    if int(row.get("attempt", "0") or 0) != attempt:
        die(f"task {row.get('id')} attempt mismatch")
    if row.get("pipeline_id") != pipeline:
        die(f"task {row.get('id')} pipeline mismatch")
    if not row.get("lease_owner") or not row.get("lease_expires_at"):
        die(f"task {row.get('id')} has no active lease")


def fixed_task_paths(logs: Path, task_id: str) -> dict[str, Path]:
    root = logs / task_id
    return {
        "proposal": root / "semantic-closure-proposal.tsv",
        "proposal_hash": root / "semantic-closure-proposal.sha256",
        "review_1": root / "semantic-closure-parity-review.tsv",
        "review_2": root / "semantic-closure-rust-review.tsv",
        "final": root / "semantic-closure-final.tsv",
        "dispositions": root / "semantic-closure-dispositions.tsv",
        "commit": root / "semantic-closure-commit.json",
        "implementation": root / "implementation.md",
        "candidate": root / "candidate.diff",
        "parity_report": root / "parity-review.md",
        "rust_report": root / "rust-review.md",
        "resolution": root / "resolution.md",
    }


def ensure_regular_nonempty(path: Path) -> None:
    if path.is_symlink() or not path.is_file() or path.stat().st_size == 0:
        die(f"required evidence is missing, empty, or not a regular file: {path}")


def proposal_metadata(
    args: argparse.Namespace, row: Mapping[str, str], paths: Mapping[str, Path], hashes: Mapping[str, str]
) -> dict[str, str]:
    linux_sha = Path(args.linux_sha_file).read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"[0-9a-f]{40}", linux_sha):
        die("invalid vendor/linux.SHA")
    return {
        "schema_version": PROPOSAL_SCHEMA_VERSION,
        "task_id": row["id"],
        "attempt": str(args.attempt),
        "pipeline_id": args.pipeline,
        "linux_sha": linux_sha,
        "candidate_sha256": sha256_file(paths["candidate"]),
        "implementation_sha256": sha256_file(paths["implementation"]),
        "phase0_identity_sha256": hashes["identity"],
        "queue_fingerprint": hashes["fingerprint"],
    }


def cmd_scaffold(args: argparse.Namespace) -> None:
    ensure_branch()
    queue = Path(args.queue)
    with QueueLock(queue):
        hashes = validate_expected_hashes(args, Path(args.rewrite))
        row = queue_row(queue, args.task)
        require_task_context(row, attempt=args.attempt, pipeline=args.pipeline, statuses={"IN_PROGRESS"})
        paths = fixed_task_paths(Path(args.logs_root), args.task)
        ensure_regular_nonempty(paths["implementation"])
        ensure_regular_nonempty(paths["candidate"])
        if paths["proposal"].exists() or paths["proposal_hash"].exists():
            die("refusing to replace an existing semantic closure proposal")
        metadata = proposal_metadata(args, row, paths, hashes)
        records = []
        for record in expected_closure_records(Path(args.rewrite), args.task):
            records.append({
                **metadata,
                **record,
                "final_value": "",
                "decision_status": "",
            })
        if not records:
            die(f"task {args.task} has no semantic closure records")
        atomic_write_tsv(paths["proposal"], PROPOSAL_FIELDS, records)
    print(json.dumps({"task_id": args.task, "records": len(records), "proposal": str(paths["proposal"])}, sort_keys=True))


def validate_proposal(
    path: Path,
    *, rewrite: Path,
    task_id: str,
    attempt: int,
    pipeline: str,
    identity_hash: str,
    fingerprint: str,
    candidate_hash: str,
    implementation_hash: str,
    require_final: bool,
) -> list[dict[str, str]]:
    records = read_tsv(path, PROPOSAL_FIELDS)
    expected = expected_closure_records(rewrite, task_id)
    if len(records) != len(expected):
        die(f"semantic proposal row count mismatch for {task_id}: {len(records)}/{len(expected)}")
    expected_by_key = {row["record_key"]: row for row in expected}
    if len(expected_by_key) != len(expected):
        die(f"internal duplicate semantic record key for {task_id}")
    if [row.get("record_key", "") for row in records] != [row["record_key"] for row in expected]:
        die(f"semantic proposal key order/set mismatch for {task_id}")
    repeated = {
        "schema_version": PROPOSAL_SCHEMA_VERSION,
        "task_id": task_id,
        "attempt": str(attempt),
        "pipeline_id": pipeline,
        "phase0_identity_sha256": identity_hash,
        "queue_fingerprint": fingerprint,
        "candidate_sha256": candidate_hash,
        "implementation_sha256": implementation_hash,
    }
    for row in records:
        expected_row = expected_by_key[row["record_key"]]
        for field, value in repeated.items():
            if row.get(field) != value:
                die(f"proposal metadata mismatch for {row['record_key']}:{field}")
        for field in (
            "manifest", "base_row", "field", "architecture", "linux_path",
            "record_kind", "symbol_name", "source_line", "old_value",
            "base_manifest_sha256",
        ):
            if row.get(field) != expected_row.get(field):
                die(f"proposal mechanical field mismatch for {row['record_key']}:{field}")
        if not row.get("source_citations", "").strip():
            die(f"proposal lacks pinned source citations for {row['record_key']}")
        if row.get("linux_sha") != Path("vendor/linux.SHA").read_text(encoding="utf-8").strip():
            die(f"proposal Linux SHA mismatch for {row['record_key']}")
        if require_final:
            decision = row.get("decision_status", "")
            final = row.get("final_value", "")
            if decision not in {"COMPLETE", "NOT_APPLICABLE"}:
                die(f"proposal has unresolved decision status for {row['record_key']}")
            if not final.strip() or final == "PENDING_REVIEW" or "\t" in final or "\n" in final or "\r" in final:
                die(f"proposal has invalid final value for {row['record_key']}")
            if decision == "NOT_APPLICABLE" and final != "NOT_APPLICABLE":
                die(f"NOT_APPLICABLE decision must use exact final value for {row['record_key']}")
            if row["field"] == "status" or (
                row["manifest"] == "SCOPE.tsv" and row["field"] == "semantic_status"
            ):
                if final != decision:
                    die(f"status field final value must equal decision status for {row['record_key']}")
    return records


def write_proposal_seal(
    proposal: Path, seal: Path, records: list[dict[str, str]]
) -> None:
    """Atomically seal the exact validated proposal bytes."""

    digest = sha256_file(proposal)
    first = records[0]
    content = (
        f"schema_version\t{PROPOSAL_SCHEMA_VERSION}\n"
        f"sha256\t{digest}\n"
        f"task_id\t{first['task_id']}\n"
        f"attempt\t{first['attempt']}\n"
        f"pipeline_id\t{first['pipeline_id']}\n"
        f"records\t{len(records)}\n"
        f"queue_fingerprint\t{first['queue_fingerprint']}\n"
        f"phase0_identity_sha256\t{first['phase0_identity_sha256']}\n"
        f"sealed_at\t{now_utc()}\n"
    )
    atomic_write(seal, content.encode("utf-8"))


def read_proposal_seal(proposal: Path, seal: Path) -> dict[str, str]:
    """Read a canonical seal and verify that it binds the current proposal bytes."""

    ensure_regular_nonempty(proposal)
    ensure_regular_nonempty(seal)
    values: dict[str, str] = {}
    order: list[str] = []
    for line_number, line in enumerate(
        seal.read_text(encoding="utf-8").splitlines(), 1
    ):
        key, separator, value = line.partition("\t")
        if not separator or key in values:
            die(f"malformed proposal seal at {seal}:{line_number}")
        order.append(key)
        values[key] = value
    if tuple(order) != PROPOSAL_SEAL_FIELDS:
        die(f"non-canonical proposal seal fields in {seal}")
    if not HEX64.fullmatch(values["sha256"]):
        die(f"invalid proposal digest in {seal}")
    if values["sha256"] != sha256_file(proposal):
        die(f"proposal seal digest mismatch for {proposal}")
    if not values["sealed_at"]:
        die(f"proposal seal lacks sealed_at in {seal}")
    return values


def seal_validated_proposal(
    proposal: Path,
    seal: Path,
    *,
    rewrite: Path,
    task_id: str,
    attempt: int,
    pipeline: str,
    identity_hash: str,
    fingerprint: str,
    candidate_hash: str,
    implementation_hash: str,
) -> list[dict[str, str]]:
    """Validate a complete proposal before atomically creating its seal."""

    if seal.exists() or seal.is_symlink():
        die("semantic proposal is already sealed")
    records = validate_proposal(
        proposal,
        rewrite=rewrite,
        task_id=task_id,
        attempt=attempt,
        pipeline=pipeline,
        identity_hash=identity_hash,
        fingerprint=fingerprint,
        candidate_hash=candidate_hash,
        implementation_hash=implementation_hash,
        require_final=True,
    )
    write_proposal_seal(proposal, seal, records)
    return records


def cmd_seal_proposal(args: argparse.Namespace) -> None:
    ensure_branch()
    queue = Path(args.queue)
    with QueueLock(queue):
        hashes = validate_expected_hashes(args, Path(args.rewrite))
        row = queue_row(queue, args.task)
        require_task_context(row, attempt=args.attempt, pipeline=args.pipeline, statuses={"IN_PROGRESS"})
        paths = fixed_task_paths(Path(args.logs_root), args.task)
        ensure_regular_nonempty(paths["implementation"])
        ensure_regular_nonempty(paths["candidate"])
        records = seal_validated_proposal(
            paths["proposal"], paths["proposal_hash"],
            rewrite=Path(args.rewrite), task_id=args.task,
            attempt=args.attempt, pipeline=args.pipeline,
            identity_hash=hashes["identity"], fingerprint=hashes["fingerprint"],
            candidate_hash=sha256_file(paths["candidate"]),
            implementation_hash=sha256_file(paths["implementation"]),
        )
    print(json.dumps({"task_id": args.task, "proposal_sha256": sha256_file(paths["proposal"]), "records": len(records)}, sort_keys=True))


def validate_sealed_proposal(
    row: Mapping[str, str], logs: Path, rewrite: Path, identity: Path, fingerprint: Path
) -> tuple[list[dict[str, str]], str]:
    paths = fixed_task_paths(logs, row["id"])
    for name in ("proposal", "proposal_hash", "implementation", "candidate"):
        ensure_regular_nonempty(paths[name])
    seal = read_proposal_seal(paths["proposal"], paths["proposal_hash"])
    proposal_hash = sha256_file(paths["proposal"])
    identity_hash = sha256_file(identity)
    queue_fingerprint = fingerprint_value(fingerprint)
    if (
        seal.get("schema_version") != PROPOSAL_SCHEMA_VERSION
        or seal.get("sha256") != proposal_hash
        or seal.get("task_id") != row["id"]
        or seal.get("attempt") != (row.get("attempt", "0") or "0")
        or seal.get("pipeline_id") != row.get("pipeline_id", "")
        or seal.get("queue_fingerprint") != queue_fingerprint
        or seal.get("phase0_identity_sha256") != identity_hash
    ):
        die(f"sealed semantic proposal metadata mismatch for {row['id']}")
    records = validate_proposal(
        paths["proposal"], rewrite=rewrite, task_id=row["id"],
        attempt=int(row.get("attempt", "0") or 0), pipeline=row.get("pipeline_id", ""),
        identity_hash=identity_hash, fingerprint=queue_fingerprint,
        candidate_hash=sha256_file(paths["candidate"]),
        implementation_hash=sha256_file(paths["implementation"]), require_final=True,
    )
    if seal.get("records") != str(len(records)):
        die(f"sealed semantic proposal record count mismatch for {row['id']}")
    return records, proposal_hash


def parse_findings(values: list[str], valid_keys: set[str]) -> list[tuple[str, str]]:
    result: list[tuple[str, str]] = []
    seen: set[str] = set()
    for value in values:
        finding, separator, keys_text = value.partition(":")
        if not FINDING_ID.fullmatch(finding) or finding == "NOT_APPLICABLE" or finding in seen:
            die(f"invalid or duplicate finding identifier: {finding!r}")
        keys = [key for key in keys_text.split(",") if key] if separator else []
        unknown = sorted(set(keys) - valid_keys)
        if unknown:
            die(f"finding {finding} references unknown semantic keys: {unknown[:10]}")
        if len(keys) != len(set(keys)):
            die(f"finding {finding} repeats semantic keys")
        result.append((finding, ",".join(keys)))
        seen.add(finding)
    return result


def cmd_review(args: argparse.Namespace) -> None:
    ensure_branch()
    queue = Path(args.queue)
    with QueueLock(queue):
        hashes = validate_expected_hashes(args, Path(args.rewrite))
        row = queue_row(queue, args.task)
        require_task_context(row, attempt=args.attempt, pipeline=args.pipeline, statuses={"REVIEWING"})
        paths = fixed_task_paths(Path(args.logs_root), args.task)
        records, proposal_hash = validate_sealed_proposal(
            row, Path(args.logs_root), Path(args.rewrite), Path(args.identity), Path(args.fingerprint)
        )
        report_key = "parity_report" if args.slot == 1 else "rust_report"
        output_key = f"review_{args.slot}"
        ensure_regular_nonempty(paths[report_key])
        if paths[output_key].exists():
            die(f"semantic review slot {args.slot} is already sealed")
        findings = parse_findings(args.finding, {record["record_key"] for record in records})
        if args.review_status == "APPROVE" and findings:
            die("APPROVE review cannot carry findings")
        if args.review_status == "FINDINGS" and not findings:
            die("FINDINGS review requires at least one --finding")
        reviewed_at = now_utc()
        common = {
            "schema_version": REVIEW_SCHEMA_VERSION,
            "task_id": args.task,
            "attempt": str(args.attempt),
            "pipeline_id": args.pipeline,
            "slot": str(args.slot),
            "proposal_sha256": proposal_hash,
            "report_path": paths[report_key].as_posix(),
            "report_sha256": sha256_file(paths[report_key]),
            "review_status": args.review_status,
            "reviewer": args.reviewer,
            "model": args.model,
            "reasoning_effort": args.effort,
            "reviewed_at": reviewed_at,
        }
        rows_out = [
            {**common, "finding_id": finding, "record_keys": keys}
            for finding, keys in findings
        ] or [{**common, "finding_id": "NOT_APPLICABLE", "record_keys": ""}]
        atomic_write_tsv(paths[output_key], REVIEW_FIELDS, rows_out)
    print(json.dumps({"task_id": args.task, "slot": args.slot, "proposal_sha256": proposal_hash, "findings": len(findings)}, sort_keys=True))


def validate_review_attestation(
    row: Mapping[str, str], slot: int, logs: Path, rewrite: Path, identity: Path, fingerprint: Path
) -> tuple[list[dict[str, str]], dict[tuple[int, str], set[str]]]:
    paths = fixed_task_paths(logs, row["id"])
    proposal, proposal_hash = validate_sealed_proposal(row, logs, rewrite, identity, fingerprint)
    path = paths[f"review_{slot}"]
    records = read_tsv(path, REVIEW_FIELDS)
    if not records:
        die(f"empty semantic review attestation for {row['id']} slot {slot}")
    report = paths["parity_report" if slot == 1 else "rust_report"]
    ensure_regular_nonempty(report)
    valid_keys = {record["record_key"] for record in proposal}
    findings: dict[tuple[int, str], set[str]] = {}
    for record in records:
        repeated = {
            "schema_version": REVIEW_SCHEMA_VERSION,
            "task_id": row["id"],
            "attempt": row.get("attempt", "0") or "0",
            "pipeline_id": row.get("pipeline_id", ""),
            "slot": str(slot),
            "proposal_sha256": proposal_hash,
            "report_path": report.as_posix(),
            "report_sha256": sha256_file(report),
        }
        if any(record.get(field) != value for field, value in repeated.items()):
            die(f"semantic review attestation binding mismatch for {row['id']} slot {slot}")
        if record.get("review_status") not in {"APPROVE", "FINDINGS"}:
            die(f"invalid semantic review status for {row['id']} slot {slot}")
        if not record.get("reviewer") or not record.get("model") or record.get("reasoning_effort") not in {"high", "xhigh", "max"}:
            die(f"invalid semantic reviewer attribution for {row['id']} slot {slot}")
        finding = record.get("finding_id", "")
        keys = {key for key in record.get("record_keys", "").split(",") if key}
        if keys - valid_keys:
            die(f"semantic review references unknown keys for {row['id']} slot {slot}")
        if finding == "NOT_APPLICABLE":
            if len(records) != 1 or record.get("review_status") != "APPROVE" or keys:
                die(f"malformed no-findings semantic review for {row['id']} slot {slot}")
        else:
            if not FINDING_ID.fullmatch(finding) or record.get("review_status") != "FINDINGS":
                die(f"malformed semantic finding for {row['id']} slot {slot}")
            key = (slot, finding)
            if key in findings:
                die(f"duplicate semantic finding {key}")
            findings[key] = keys
    return records, findings


def cmd_prepare_final(args: argparse.Namespace) -> None:
    ensure_branch()
    queue = Path(args.queue)
    with QueueLock(queue):
        validate_expected_hashes(args, Path(args.rewrite))
        row = queue_row(queue, args.task)
        require_task_context(row, attempt=args.attempt, pipeline=args.pipeline, statuses={"APPLYING"})
        paths = fixed_task_paths(Path(args.logs_root), args.task)
        proposal, _ = validate_sealed_proposal(row, Path(args.logs_root), Path(args.rewrite), Path(args.identity), Path(args.fingerprint))
        validate_review_attestation(row, 1, Path(args.logs_root), Path(args.rewrite), Path(args.identity), Path(args.fingerprint))
        validate_review_attestation(row, 2, Path(args.logs_root), Path(args.rewrite), Path(args.identity), Path(args.fingerprint))
        if paths["final"].exists() or paths["dispositions"].exists() or paths["commit"].exists():
            die("refusing to replace existing semantic applier evidence")
        atomic_write_tsv(paths["final"], PROPOSAL_FIELDS, proposal)
        atomic_write_tsv(paths["dispositions"], DISPOSITION_FIELDS, [])
    print(json.dumps({"task_id": args.task, "records": len(proposal), "final": str(paths["final"])}, sort_keys=True))


def validate_final_and_dispositions(
    row: Mapping[str, str], paths: Mapping[str, Path], rewrite: Path, identity: Path, fingerprint: Path
) -> tuple[list[dict[str, str]], list[dict[str, str]], dict[str, str]]:
    proposal, proposal_hash = validate_sealed_proposal(row, paths["proposal"].parents[1], rewrite, identity, fingerprint)
    review_hashes: dict[str, str] = {}
    findings: dict[tuple[int, str], set[str]] = {}
    for slot in (1, 2):
        _, found = validate_review_attestation(row, slot, paths["proposal"].parents[1], rewrite, identity, fingerprint)
        findings.update(found)
        review_hashes[f"review_{slot}_sha256"] = sha256_file(paths[f"review_{slot}"])
    final = validate_proposal(
        paths["final"], rewrite=rewrite, task_id=row["id"],
        attempt=int(row.get("attempt", "0") or 0), pipeline=row.get("pipeline_id", ""),
        identity_hash=sha256_file(identity), fingerprint=fingerprint_value(fingerprint),
        candidate_hash=sha256_file(paths["candidate"]),
        implementation_hash=sha256_file(paths["implementation"]), require_final=True,
    )
    proposal_by_key = {record["record_key"]: record for record in proposal}
    final_by_key = {record["record_key"]: record for record in final}
    mutable = {"final_value", "decision_status", "source_citations"}
    changed_keys: set[str] = set()
    for key, final_record in final_by_key.items():
        original = proposal_by_key[key]
        if any(final_record[field] != original[field] for field in mutable):
            changed_keys.add(key)
        for field in PROPOSAL_FIELDS:
            if field not in mutable and final_record[field] != original[field]:
                die(f"applier final changed frozen proposal field {field} for {key}")
    dispositions = read_tsv(paths["dispositions"], DISPOSITION_FIELDS)
    by_finding: dict[tuple[int, str], dict[str, str]] = {}
    changed_authorized: set[str] = set()
    for disposition in dispositions:
        try:
            slot = int(disposition.get("source_slot", ""))
        except ValueError:
            die("semantic disposition has non-integer source_slot")
        key = (slot, disposition.get("finding_id", ""))
        if key in by_finding or key not in findings:
            die(f"semantic disposition has duplicate/unknown finding: {key}")
        keys = {item for item in disposition.get("record_keys", "").split(",") if item}
        if keys != findings[key]:
            die(f"semantic disposition record-key set differs from finding {key}")
        status = disposition.get("disposition", "")
        if status not in {"RESOLVED_CHANGED", "RESOLVED_NO_CHANGE", "DISPROVED"}:
            die(f"semantic finding {key} has unresolved disposition {status!r}")
        if not disposition.get("source_citations", "").strip() or not disposition.get("detail", "").strip():
            die(f"semantic disposition lacks source-backed detail for {key}")
        if status == "RESOLVED_CHANGED":
            if not keys or not keys <= changed_keys:
                die(f"RESOLVED_CHANGED finding {key} does not match changed final records")
            changed_authorized.update(keys)
        elif keys & changed_keys:
            die(f"unchanged/disproved finding {key} cannot authorize final changes")
        by_finding[key] = disposition
    if set(by_finding) != set(findings):
        die(f"semantic dispositions do not cover all findings: missing={sorted(set(findings)-set(by_finding))[:20]}")
    if changed_keys != changed_authorized:
        die(f"applier final contains changes not authorized by accepted findings: {sorted(changed_keys-changed_authorized)[:20]}")
    return final, dispositions, {"proposal_sha256": proposal_hash, **review_hashes}


def ledger_state(ledger: Path, fingerprint: str) -> tuple[list[dict[str, object]], dict[str, dict[str, object]], set[str]]:
    records = validate_ledger(ledger)
    prepares: dict[str, dict[str, object]] = {}
    commits: set[str] = set()
    for record in records:
        if record.get("queue_fingerprint") != fingerprint:
            continue
        kind = record.get("record_type")
        transaction = str(record.get("transaction_id", ""))
        if kind == "PREPARE":
            if transaction in prepares:
                die(f"duplicate semantic ledger PREPARE: {transaction}")
            prepares[transaction] = record
        elif kind == "COMMIT":
            if transaction not in prepares or transaction in commits:
                die(f"orphan/duplicate semantic ledger COMMIT: {transaction}")
            if record.get("prepare_sha256") != sha256_bytes(
                json.dumps(prepares[transaction], sort_keys=True, separators=(",", ":")).encode("utf-8")
            ):
                die(f"semantic ledger COMMIT/PREPARE hash mismatch: {transaction}")
            commits.add(transaction)
        elif kind == "GENERATION_OPEN":
            continue
        else:
            die(f"unknown semantic ledger record type: {kind!r}")
    return records, prepares, commits


def event_records(path: Path) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line:
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as exc:
            die(f"malformed event log at {path}:{line_number}: {exc}")
        if not isinstance(record, dict):
            die(f"non-object event at {path}:{line_number}")
        records.append(record)
    return records


def cmd_commit(args: argparse.Namespace) -> None:
    ensure_branch()
    queue = Path(args.queue)
    events = Path(args.events)
    ledger = Path(args.ledger)
    with QueueLock(queue):
        hashes = validate_expected_hashes(args, Path(args.rewrite))
        row = queue_row(queue, args.task)
        require_task_context(row, attempt=args.attempt, pipeline=args.pipeline, statuses={"APPLYING"})
        paths = fixed_task_paths(Path(args.logs_root), args.task)
        for name in ("final", "dispositions", "resolution", "candidate", "implementation", "parity_report", "rust_report"):
            ensure_regular_nonempty(paths[name])
        final, dispositions, evidence_hashes = validate_final_and_dispositions(
            row, paths, Path(args.rewrite), Path(args.identity), Path(args.fingerprint)
        )
        destination = Path(row["path"])
        ensure_regular_nonempty(destination)
        transaction_input = {
            "schema_version": COMMIT_SCHEMA_VERSION,
            "task_id": args.task,
            "attempt": args.attempt,
            "pipeline_id": args.pipeline,
            "queue_fingerprint": hashes["fingerprint"],
            "phase0_identity_sha256": hashes["identity"],
            "base_hashes": {name: hashes[name] for name in ("scope", "symbols", "abi", "lifetimes")},
            "proposal_sha256": evidence_hashes["proposal_sha256"],
            "final_sha256": sha256_file(paths["final"]),
            "review_1_sha256": evidence_hashes["review_1_sha256"],
            "review_2_sha256": evidence_hashes["review_2_sha256"],
            "parity_report_sha256": sha256_file(paths["parity_report"]),
            "rust_report_sha256": sha256_file(paths["rust_report"]),
            "resolution_sha256": sha256_file(paths["resolution"]),
            "dispositions_sha256": sha256_file(paths["dispositions"]),
            "candidate_sha256": sha256_file(paths["candidate"]),
            "implementation_sha256": sha256_file(paths["implementation"]),
            "destination_sha256": sha256_file(destination),
            "record_count": len(final),
            "finding_count": len(dispositions),
        }
        transaction_id = "SCTX-" + sha256_bytes(
            json.dumps(transaction_input, sort_keys=True, separators=(",", ":")).encode("utf-8")
        )
        _, prepares, commits = ledger_state(ledger, hashes["fingerprint"])
        prior_for_task = [
            transaction for transaction, prepare in prepares.items()
            if prepare.get("task_id") == args.task and int(prepare.get("attempt", 0)) == args.attempt
        ]
        if prior_for_task and prior_for_task != [transaction_id]:
            die(f"task attempt already has a different semantic transaction: {prior_for_task}")
        prepare = {
            "schema_version": LEDGER_SCHEMA_VERSION,
            "record_type": "PREPARE",
            "transaction_id": transaction_id,
            "prepared_at": now_utc(),
            **transaction_input,
            "actor": args.actor,
            "model": args.model,
            "reasoning_effort": args.effort,
            "records": [
                {
                    "record_key": record["record_key"],
                    "manifest": record["manifest"],
                    "base_row": int(record["base_row"]),
                    "field": record["field"],
                    "old_value": record["old_value"],
                    "final_value": record["final_value"],
                    "decision_status": record["decision_status"],
                    "architecture": record["architecture"],
                    "source_citations": record["source_citations"],
                }
                for record in final
            ],
        }
        prepare_hash = sha256_bytes(json.dumps(prepare, sort_keys=True, separators=(",", ":")).encode("utf-8"))
        matching_events = [
            record for record in event_records(events)
            if record.get("event") == "semantic_closure_committed"
            and record.get("task_id") == args.task
            and record.get("attempt") == args.attempt
            and f"transaction_id={transaction_id};" in str(record.get("detail", ""))
        ]
        if transaction_id not in prepares:
            append_jsonl(ledger, prepare)
        elif prepares[transaction_id].get("final_sha256") != transaction_input["final_sha256"]:
            die("existing semantic PREPARE does not match current final evidence")
        if not matching_events:
            event = {
                "ts": now_utc(), "phase": "translation", "task_id": args.task,
                "path": row["path"], "pipeline_id": args.pipeline, "role": args.actor,
                "event": "semantic_closure_committed", "from_status": "APPLYING",
                "to_status": "APPLYING", "model": args.model,
                "reasoning_effort": args.effort, "attempt": args.attempt,
                "detail": (
                    f"transaction_id={transaction_id}; prepare_sha256={prepare_hash}; "
                    f"queue_fingerprint={hashes['fingerprint']}; records={len(final)}; "
                    f"findings={len(dispositions)}; proposal_sha256={evidence_hashes['proposal_sha256']}; "
                    f"final_sha256={transaction_input['final_sha256']}; "
                    f"parity_report_sha256={transaction_input['parity_report_sha256']}; "
                    f"rust_report_sha256={transaction_input['rust_report_sha256']}; "
                    f"resolution_sha256={transaction_input['resolution_sha256']}"
                ),
            }
            append_jsonl(events, event)
        elif len(matching_events) != 1:
            die(f"duplicate semantic closure commit events for {transaction_id}")
        if transaction_id not in commits:
            append_jsonl(ledger, {
                "schema_version": LEDGER_SCHEMA_VERSION,
                "record_type": "COMMIT",
                "transaction_id": transaction_id,
                "queue_fingerprint": hashes["fingerprint"],
                "prepare_sha256": prepare_hash,
                "committed_at": now_utc(),
            })
        receipt = {
            **transaction_input,
            "transaction_id": transaction_id,
            "prepare_sha256": prepare_hash,
            "committed_at": now_utc(),
            "ledger": ledger.as_posix(),
            "events": events.as_posix(),
        }
        atomic_write(paths["commit"], (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    print(json.dumps({"task_id": args.task, "transaction_id": transaction_id, "records": len(final), "findings": len(dispositions)}, sort_keys=True))


def initialize_generation(
    queue_rows: list[dict[str, str]], fingerprint: Path, identity: Path,
    ledger: Path, events: Path,
) -> None:
    """Open one clean semantic generation while the caller holds QueueLock."""

    fingerprint_digest = fingerprint_value(fingerprint)
    identity_digest = sha256_file(identity)
    records, prepares, commits = ledger_state(ledger, fingerprint_digest)
    opens = [record for record in records if record.get("record_type") == "GENERATION_OPEN" and record.get("queue_fingerprint") == fingerprint_digest]
    if opens:
        die(f"semantic closure generation already opened for {fingerprint_digest}")
    if prepares or commits:
        die(f"new semantic closure generation is not clean: {fingerprint_digest}")
    if any(row.get("status") != "TODO" or (row.get("attempt", "0") or "0") != "0" for row in queue_rows):
        die("semantic closure generation can open only for an all-TODO attempt-zero queue")
    record = {
        "schema_version": LEDGER_SCHEMA_VERSION,
        "record_type": "GENERATION_OPEN",
        "transaction_id": "",
        "queue_fingerprint": fingerprint_digest,
        "phase0_identity_sha256": identity_digest,
        "opened_at": now_utc(),
        "tasks": len(queue_rows),
    }
    append_jsonl(ledger, record)
    append_jsonl(events, {
        "ts": now_utc(), "phase": "phase0", "task_id": "", "path": "",
        "pipeline_id": "", "role": "queue_tool", "event": "semantic_closure_generation_opened",
        "from_status": "", "to_status": "", "model": "none",
        "reasoning_effort": "none", "attempt": 0,
        "detail": f"queue_fingerprint={fingerprint_digest}; identity_sha256={identity_digest}; tasks={len(queue_rows)}; ledger={ledger}",
    })


def validate_generation_initial_state(
    queue_rows: list[dict[str, str]], fingerprint: Path, ledger: Path
) -> dict[str, object]:
    fingerprint_digest = fingerprint_value(fingerprint)
    records, prepares, commits = ledger_state(ledger, fingerprint_digest)
    opens = [record for record in records if record.get("record_type") == "GENERATION_OPEN" and record.get("queue_fingerprint") == fingerprint_digest]
    todo_zero = all(row.get("status") == "TODO" and (row.get("attempt", "0") or "0") == "0" for row in queue_rows)
    ok = len(opens) == 1 and not prepares and not commits and todo_zero
    return {"ok": ok, "opens": len(opens), "prepares": len(prepares), "commits": len(commits), "todo_zero": todo_zero}


def committed_closure(
    row: Mapping[str, str], logs: Path, rewrite: Path, identity: Path,
    fingerprint: Path, ledger: Path, events: Path,
) -> dict[str, object]:
    paths = fixed_task_paths(logs, row["id"])
    ensure_regular_nonempty(paths["commit"])
    try:
        receipt = json.loads(paths["commit"].read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        die(f"malformed semantic commit receipt for {row['id']}: {exc}")
    if not isinstance(receipt, dict):
        die(f"semantic commit receipt is not an object for {row['id']}")
    final, dispositions, evidence_hashes = validate_final_and_dispositions(row, paths, rewrite, identity, fingerprint)
    fingerprint_digest = fingerprint_value(fingerprint)
    _, prepares, commits = ledger_state(ledger, fingerprint_digest)
    transaction = str(receipt.get("transaction_id", ""))
    prepare = prepares.get(transaction)
    if prepare is None or transaction not in commits:
        die(f"task {row['id']} has no committed semantic ledger transaction")
    if (
        prepare.get("task_id") != row["id"]
        or int(prepare.get("attempt", 0)) != int(row.get("attempt", "0") or 0)
        or prepare.get("pipeline_id") != row.get("pipeline_id", "")
        or prepare.get("phase0_identity_sha256") != sha256_file(identity)
        or prepare.get("proposal_sha256") != evidence_hashes["proposal_sha256"]
        or prepare.get("final_sha256") != sha256_file(paths["final"])
        or prepare.get("resolution_sha256") != sha256_file(paths["resolution"])
        or prepare.get("dispositions_sha256") != sha256_file(paths["dispositions"])
        or prepare.get("record_count") != len(final)
        or prepare.get("finding_count") != len(dispositions)
    ):
        die(f"semantic ledger/evidence binding mismatch for {row['id']}")
    event_matches = [
        record for record in event_records(events)
        if record.get("event") == "semantic_closure_committed"
        and record.get("task_id") == row["id"]
        and record.get("attempt") == int(row.get("attempt", "0") or 0)
        and f"transaction_id={transaction};" in str(record.get("detail", ""))
    ]
    if len(event_matches) != 1:
        die(f"semantic closure transaction has {len(event_matches)} matching events for {row['id']}")
    required = expected_closure_records(rewrite, row["id"])
    final_keys = {record["record_key"] for record in final}
    if final_keys != {record["record_key"] for record in required}:
        die(f"semantic closure leaves a task-owned key-set mismatch for {row['id']}")
    pending = [record["record_key"] for record in final if record["final_value"] == "PENDING_REVIEW" or record["decision_status"] not in {"COMPLETE", "NOT_APPLICABLE"}]
    if pending:
        die(f"semantic closure leaves PENDING effective fields for {row['id']}: {pending[:20]}")
    return {"transaction_id": transaction, "records": len(final), "findings": len(dispositions)}


# Queue-tool integration entry points.  Imports remain one-way: this module does
# not import rewrite_queue.py, so rewrite_queue.py can lazily import these.
def require_sealed_proposal_for_queue(row: Mapping[str, str], logs: Path, rewrite: Path, identity: Path, fingerprint: Path) -> None:
    validate_sealed_proposal(row, logs, rewrite, identity, fingerprint)


def require_review_attestation_for_queue(row: Mapping[str, str], slot: int, logs: Path, rewrite: Path, identity: Path, fingerprint: Path) -> None:
    validate_review_attestation(row, slot, logs, rewrite, identity, fingerprint)


def require_committed_closure_for_queue(row: Mapping[str, str], logs: Path, rewrite: Path, identity: Path, fingerprint: Path, ledger: Path, events: Path) -> None:
    committed_closure(row, logs, rewrite, identity, fingerprint, ledger, events)


def cmd_verify_task(args: argparse.Namespace) -> None:
    row = queue_row(Path(args.queue), args.task)
    result = committed_closure(
        row, Path(args.logs_root), Path(args.rewrite), Path(args.identity),
        Path(args.fingerprint), Path(args.ledger), Path(args.events),
    )
    print(json.dumps({"task_id": args.task, **result}, sort_keys=True))


def add_paths(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--rewrite", default="rewrite")
    parser.add_argument("--queue", default=str(DEFAULT_QUEUE))
    parser.add_argument("--fingerprint", default=str(DEFAULT_FINGERPRINT))
    parser.add_argument("--identity", default=str(DEFAULT_IDENTITY))
    parser.add_argument("--events", default=str(DEFAULT_EVENTS))
    parser.add_argument("--logs-root", default=str(DEFAULT_LOGS))
    parser.add_argument("--ledger", default=str(DEFAULT_LEDGER))
    parser.add_argument("--linux-sha-file", default="vendor/linux.SHA")


def add_context(parser: argparse.ArgumentParser) -> None:
    add_paths(parser)
    parser.add_argument("--task", required=True)
    parser.add_argument("--attempt", type=int, required=True)
    parser.add_argument("--pipeline", choices=("P01", "P02"), required=True)
    parser.add_argument("--expected-identity-sha256", required=True)
    parser.add_argument("--expected-queue-fingerprint", required=True)
    parser.add_argument("--expected-scope-sha256", required=True)
    parser.add_argument("--expected-symbols-sha256", required=True)
    parser.add_argument("--expected-abi-sha256", required=True)
    parser.add_argument("--expected-lifetimes-sha256", required=True)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)

    init = commands.add_parser("init-phase0", help="freeze semantic schema/base bindings without changing mechanical manifests")
    init.add_argument("--rewrite", default="rewrite")
    init.add_argument("--phase-gate-reopen", action="store_true")
    init.set_defaults(func=lambda args: print(json.dumps(initialize_phase0(Path(args.rewrite), phase_gate_reopen=args.phase_gate_reopen), sort_keys=True)))

    scaffold = commands.add_parser("scaffold", help="create one task's complete field-level proposal template")
    add_context(scaffold)
    scaffold.set_defaults(func=cmd_scaffold)

    seal = commands.add_parser("seal-proposal", help="validate and seal an implementation-stage closure proposal")
    add_context(seal)
    seal.set_defaults(func=cmd_seal_proposal)

    review = commands.add_parser("review", help="bind one isolated reviewer report to the sealed proposal")
    add_context(review)
    review.add_argument("--slot", type=int, choices=(1, 2), required=True)
    review.add_argument("--review-status", choices=("APPROVE", "FINDINGS"), required=True)
    review.add_argument("--finding", action="append", default=[], metavar="ID:KEY,KEY")
    review.add_argument("--reviewer", required=True)
    review.add_argument("--model", required=True)
    review.add_argument("--effort", choices=("high", "xhigh", "max"), required=True)
    review.set_defaults(func=cmd_review)

    final = commands.add_parser("prepare-final", help="create applier final/disposition evidence from the reviewed proposal")
    add_context(final)
    final.set_defaults(func=cmd_prepare_final)

    commit = commands.add_parser("commit", help="atomically commit the applier's effective semantic decisions")
    add_context(commit)
    commit.add_argument("--actor", default="applier")
    commit.add_argument("--model", required=True)
    commit.add_argument("--effort", choices=("high", "xhigh", "max"), required=True)
    commit.set_defaults(func=cmd_commit)

    verify = commands.add_parser("verify-task", help="validate one current-attempt committed closure")
    add_paths(verify)
    verify.add_argument("--task", required=True)
    verify.set_defaults(func=cmd_verify_task)
    return parser


def main() -> None:
    ensure_root()
    args = build_parser().parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
