#!/usr/bin/env python3
"""Atomic TSV-backed task queue for the Lupos source-translation phase.

The queue is intentionally simple and inspectable. Mutating commands take one
OS-level lock, validate the frozen immutable fields, atomically replace the TSV,
and append an event while the lock is held.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
from typing import Iterable, Mapping

EXPECTED_BRANCH = "feat/bun-like-rewrite-test"
DEFAULT_QUEUE = Path("rewrite/TRANSLATION_TASKS.tsv")
DEFAULT_FINGERPRINT = Path("rewrite/TRANSLATION_TASKS.sha256")
DEFAULT_EVENTS = Path("rewrite/events.jsonl")
DEFAULT_LOGS_ROOT = Path("rewrite/logs/tasks")
DEFAULT_LINUX_SHA_FILE = Path("vendor/linux.SHA")
DEFAULT_LINUX_ROOT = Path("vendor/linux")

FIELDS = [
    "id",
    "path",
    "created_at",
    "work_started_at",
    "done_at",
    "status",
    "linux_path",
    "architectures",
    "cluster",
    "weight",
    "risk",
    "dependencies",
    "recommended_implementer",
    "pipeline_id",
    "attempt",
    "lease_owner",
    "lease_expires_at",
    "implement_done_at",
    "review_started_at",
    "review_1_done_at",
    "review_2_done_at",
    "apply_started_at",
    "updated_at",
    "resume_status",
    "last_error",
]

IMMUTABLE_FIELDS = [
    "id",
    "path",
    "created_at",
    "linux_path",
    "architectures",
    "cluster",
    "weight",
    "risk",
    "dependencies",
    "recommended_implementer",
]

ACTIVE_STATUSES = {"IN_PROGRESS", "IMPLEMENTED", "REVIEWING", "APPLYING"}
ALLOWED_PIPELINES = {"P01", "P02"}
ALL_STATUSES = {
    "TODO",
    "IN_PROGRESS",
    "IMPLEMENTED",
    "REVIEWING",
    "APPLYING",
    "DONE",
    "BLOCKED",
    "PAUSED",
}

TRANSITIONS = {
    "TODO": {"IN_PROGRESS", "BLOCKED"},
    "IN_PROGRESS": {"IMPLEMENTED", "BLOCKED", "PAUSED"},
    "IMPLEMENTED": {"REVIEWING", "BLOCKED", "PAUSED"},
    "REVIEWING": {"APPLYING", "BLOCKED", "PAUSED"},
    "APPLYING": {"DONE", "BLOCKED", "PAUSED"},
    "BLOCKED": {"TODO"},
    "PAUSED": {"TODO", "BLOCKED", "IN_PROGRESS", "IMPLEMENTED", "REVIEWING", "APPLYING"},
    "DONE": set(),
}

EVIDENCE_FILES = [
    "implementation.md",
    "candidate.diff",
    "parity-review.md",
    "rust-review.md",
    "resolution.md",
]

SCOPE_REQUIRED_FIELDS = {
    "id",
    "linux_path",
    "destination_path",
    "class",
    "architectures",
    "cluster",
    "weight",
    "risk",
    "dependencies",
    "metadata_status",
    "metadata_evidence",
    "semantic_status",
}

SCOPE_CLASSES = {
    "RUST_TRANSLATE",
    "LINUX_ARCH_ASM",
    "LINUX_DRIVER_OBJECT",
    "ORACLE_ONLY",
    "BUILD_METADATA",
    "REFERENCE_ONLY",
    "OUT_OF_SCOPE",
}


def now_utc() -> str:
    return (
        dt.datetime.now(dt.timezone.utc)
        .isoformat(timespec="milliseconds")
        .replace("+00:00", "Z")
    )


def parse_utc(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def die(message: str, code: int = 2) -> "NoReturn":
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(code)


def ensure_branch(skip: bool) -> None:
    if skip:
        return
    try:
        current = subprocess.check_output(
            ["git", "branch", "--show-current"], text=True, stderr=subprocess.STDOUT
        ).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        die(f"cannot verify Git branch: {exc}")
    if current != EXPECTED_BRANCH:
        die(f"queue mutation requires branch {EXPECTED_BRANCH!r}; current branch is {current!r}")


def ensure_repository_root() -> None:
    try:
        root = Path(
            subprocess.check_output(
                ["git", "rev-parse", "--show-toplevel"],
                text=True,
                stderr=subprocess.STDOUT,
            ).strip()
        ).resolve()
    except (OSError, subprocess.CalledProcessError) as exc:
        die(f"cannot locate repository root: {exc}")
    current = Path.cwd().resolve()
    if current != root:
        die(f"run rewrite_queue.py from the repository root {root}; current directory is {current}")


def lock_path_for(queue: Path) -> Path:
    return queue.parent / ".translation_tasks.lock"


class QueueLock:
    def __init__(self, queue: Path) -> None:
        self.path = lock_path_for(queue)
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


def read_tsv(path: Path, expected_fields: list[str] | None = None) -> list[dict[str, str]]:
    if not path.exists():
        die(f"missing file: {path}")
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        if reader.fieldnames is None:
            die(f"missing TSV header: {path}")
        if expected_fields is not None and reader.fieldnames != expected_fields:
            die(
                f"unexpected header in {path}; expected exactly {expected_fields}, "
                f"found {reader.fieldnames}"
            )
        rows = [dict(row) for row in reader]
    return rows


def atomic_write_tsv(path: Path, rows: Iterable[Mapping[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent, text=True)
    temp_path = Path(temp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=FIELDS,
                delimiter="\t",
                lineterminator="\n",
                extrasaction="raise",
            )
            writer.writeheader()
            for row in rows:
                writer.writerow({field: str(row.get(field, "")) for field in FIELDS})
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_path, path)
        fsync_directory(path.parent)
    finally:
        if temp_path.exists():
            temp_path.unlink()


def append_events(path: Path, payloads: Iterable[Mapping[str, object]]) -> None:
    """Append one or more complete JSONL records with a single fsync.

    Queue mutations already hold the queue lock, so event order matches mutation
    order. Batching matters during initialization, where thousands of
    `task_created` records would otherwise require thousands of fsync calls.
    """

    records = [
        json.dumps(payload, sort_keys=True, separators=(",", ":")) + "\n"
        for payload in payloads
    ]
    if not records:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write("".join(records))
        handle.flush()
        os.fsync(handle.fileno())


def append_event(path: Path, payload: Mapping[str, object]) -> None:
    append_events(path, [payload])


def immutable_digest(rows: list[dict[str, str]]) -> str:
    canonical = [
        {field: row.get(field, "") for field in IMMUTABLE_FIELDS}
        for row in rows
    ]
    data = json.dumps(canonical, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def fsync_directory(path: Path) -> None:
    dir_fd = os.open(path, os.O_DIRECTORY)
    try:
        os.fsync(dir_fd)
    finally:
        os.close(dir_fd)


def read_pinned_linux_sha(path: Path) -> str:
    if not path.is_file():
        die(f"missing pinned Linux revision file: {path}")
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        candidate = line.split()[0].lower()
        if len(candidate) != 40 or any(character not in "0123456789abcdef" for character in candidate):
            die(
                f"{path} must contain one exact 40-character Git commit SHA; "
                f"found {candidate!r}"
            )
        return candidate
    die(f"{path} contains no pinned Linux commit SHA")


def verify_linux_checkout(linux_root: Path, sha_file: Path) -> str:
    expected = read_pinned_linux_sha(sha_file)
    if not linux_root.is_dir():
        die(f"missing pinned Linux source root: {linux_root}")
    try:
        actual = subprocess.check_output(
            ["git", "-C", str(linux_root), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.STDOUT,
        ).strip().lower()
        dirty = subprocess.check_output(
            ["git", "-C", str(linux_root), "status", "--porcelain", "--untracked-files=no"],
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        die(f"cannot verify pinned Linux checkout {linux_root}: {exc}")
    if actual != expected:
        die(
            f"pinned Linux checkout mismatch: {linux_root} is {actual}, "
            f"but {sha_file} requires {expected}"
        )
    if dirty:
        die(
            f"pinned Linux checkout has tracked modifications; restore the oracle before work:\n{dirty}"
        )
    return expected


def write_fingerprint(path: Path, rows: list[dict[str, str]], linux_sha: str) -> str:
    digest = immutable_digest(rows)
    path.parent.mkdir(parents=True, exist_ok=True)
    content = (
        f"sha256\t{digest}\n"
        f"tasks\t{len(rows)}\n"
        f"linux_sha\t{linux_sha}\n"
        f"created_at\t{now_utc()}\n"
    )
    fd, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent, text=True)
    temp_path = Path(temp_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_path, path)
        fsync_directory(path.parent)
    finally:
        if temp_path.exists():
            temp_path.unlink()
    return digest


def read_fingerprint(path: Path) -> tuple[str, int, str]:
    if not path.exists():
        die(f"missing queue fingerprint: {path}; run freeze first")
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        parts = line.split("\t", 1)
        if len(parts) != 2:
            die(f"malformed fingerprint line in {path}: {line!r}")
        values[parts[0]] = parts[1]
    try:
        linux_sha = values["linux_sha"].lower()
        if len(linux_sha) != 40 or any(
            character not in "0123456789abcdef" for character in linux_sha
        ):
            raise ValueError(f"invalid linux_sha {linux_sha!r}")
        return values["sha256"], int(values["tasks"]), linux_sha
    except (KeyError, ValueError) as exc:
        die(f"malformed fingerprint file {path}: {exc}")


def verify_fingerprint(rows: list[dict[str, str]], path: Path, sha_file: Path) -> None:
    expected_digest, expected_count, expected_linux_sha = read_fingerprint(path)
    current_linux_sha = read_pinned_linux_sha(sha_file)
    if current_linux_sha != expected_linux_sha:
        die(
            "pinned Linux SHA changed after queue freeze; stop all pipelines and reopen "
            f"the scope gate (expected {expected_linux_sha}, found {current_linux_sha})"
        )
    actual_digest = immutable_digest(rows)
    if len(rows) != expected_count:
        die(f"queue task count changed: expected {expected_count}, found {len(rows)}")
    if actual_digest != expected_digest:
        die(
            "queue immutable fields changed; stop all pipelines and reopen the scope gate "
            f"(expected {expected_digest}, found {actual_digest})"
        )


def validate_architectures(value: str, *, context: str) -> None:
    allowed = {"common", "x86_64", "aarch64"}
    values = [item.strip() for item in value.split(",") if item.strip()]
    if not values:
        die(f"{context} has no architecture selection")
    if len(values) != len(set(values)):
        die(f"{context} repeats an architecture: {value!r}")
    unknown = sorted(set(values) - allowed)
    if unknown:
        die(f"{context} has invalid architectures: {', '.join(unknown)}")


def validate_relative_posix_path(value: str, *, context: str) -> PurePosixPath:
    path = PurePosixPath(value)
    if not value or path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        die(f"{context} must be a normalized relative POSIX path: {value!r}")
    return path


def validate_pipeline_id(value: str, *, context: str, allow_empty: bool = False) -> None:
    if not value and allow_empty:
        return
    if value not in ALLOWED_PIPELINES:
        die(
            f"{context} must be one of {', '.join(sorted(ALLOWED_PIPELINES))}; "
            f"found {value!r}"
        )


def validate_dependencies(rows: list[dict[str, str]]) -> None:
    ids = {row["id"] for row in rows}
    graph: dict[str, list[str]] = {}
    for row in rows:
        deps = parse_dependencies(row)
        if len(deps) != len(set(deps)):
            die(f"task {row['id']} repeats a dependency")
        unknown = sorted(set(deps) - ids)
        if unknown:
            die(f"task {row['id']} has unknown dependencies: {', '.join(unknown)}")
        if row["id"] in deps:
            die(f"task {row['id']} depends on itself")
        graph[row["id"]] = deps

    state: dict[str, int] = {task_id: 0 for task_id in graph}
    stack: list[str] = []

    def visit(task_id: str) -> None:
        if state[task_id] == 2:
            return
        if state[task_id] == 1:
            try:
                index = stack.index(task_id)
            except ValueError:
                index = 0
            cycle = stack[index:] + [task_id]
            die("dependency cycle: " + " -> ".join(cycle))
        state[task_id] = 1
        stack.append(task_id)
        for dependency in graph[task_id]:
            visit(dependency)
        stack.pop()
        state[task_id] = 2

    for task_id in sorted(graph):
        visit(task_id)


def validate_rows(rows: list[dict[str, str]]) -> None:
    ids: set[str] = set()
    paths: set[str] = set()
    reserved_pipelines: dict[str, str] = {}
    timestamp_fields = [
        "created_at",
        "work_started_at",
        "done_at",
        "lease_expires_at",
        "implement_done_at",
        "review_started_at",
        "review_1_done_at",
        "review_2_done_at",
        "apply_started_at",
        "updated_at",
    ]

    for index, row in enumerate(rows, start=2):
        task_id = row["id"]
        path = row["path"]
        status = row["status"]
        if not task_id or task_id in ids:
            die(f"duplicate or empty task id at TSV line {index}: {task_id!r}")
        if not path or path in paths:
            die(f"duplicate or empty destination path at TSV line {index}: {path!r}")
        destination = validate_relative_posix_path(path, context=f"task {task_id} destination")
        if destination.parts[0] != "src" or destination.suffix != ".rs":
            die(f"task {task_id} destination must be a Rust file under src/: {path!r}")
        validate_relative_posix_path(row["linux_path"], context=f"task {task_id} Linux path")
        validate_architectures(row["architectures"], context=f"task {task_id}")
        if status not in ALL_STATUSES:
            die(f"invalid status at TSV line {index}: {status!r}")
        if row["risk"] not in {"low", "medium", "high"}:
            die(f"invalid risk at TSV line {index}: {row['risk']!r}")
        if row["recommended_implementer"] not in {"luna", "spark"}:
            die(
                f"invalid recommended implementer at TSV line {index}: "
                f"{row['recommended_implementer']!r}"
            )
        if row["recommended_implementer"] == "spark" and row["risk"] != "low":
            die(
                f"task {task_id} may recommend Spark only when risk=low; "
                f"found risk={row['risk']!r}"
            )
        try:
            weight = float(row["weight"])
        except ValueError:
            die(f"invalid weight at TSV line {index}: {row['weight']!r}")
        if not math.isfinite(weight) or weight <= 0:
            die(f"weight must be finite and positive at TSV line {index}: {row['weight']!r}")
        try:
            attempt = int(row["attempt"] or 0)
        except ValueError:
            die(f"invalid attempt at TSV line {index}: {row['attempt']!r}")
        if attempt < 0:
            die(f"attempt cannot be negative at TSV line {index}")
        for field in timestamp_fields:
            value = row[field]
            if not value:
                continue
            try:
                parse_utc(value)
            except ValueError as exc:
                die(f"invalid timestamp {field} at TSV line {index}: {value!r} ({exc})")
        if not row["created_at"] or not row["updated_at"]:
            die(f"task {task_id} is missing created_at or updated_at")

        if row["pipeline_id"]:
            validate_pipeline_id(
                row["pipeline_id"], context=f"task {task_id} pipeline_id"
            )

        if status in ACTIVE_STATUSES:
            if not row["pipeline_id"] or not row["lease_owner"] or not row["lease_expires_at"]:
                die(f"active task {task_id} is missing pipeline or lease data")
            if not row["work_started_at"]:
                die(f"active task {task_id} has no work_started_at")
        elif row["lease_owner"] or row["lease_expires_at"]:
            die(f"non-active task {task_id} retains an active lease")

        if status == "PAUSED":
            if row["resume_status"] not in ACTIVE_STATUSES:
                die(f"paused task {task_id} has invalid resume_status {row['resume_status']!r}")
            if not row["pipeline_id"]:
                die(f"paused task {task_id} has no reserved pipeline_id")
        elif row["resume_status"]:
            die(f"non-paused task {task_id} has resume_status {row['resume_status']!r}")

        if status in ACTIVE_STATUSES or status == "PAUSED":
            previous = reserved_pipelines.get(row["pipeline_id"] or "")
            if previous:
                die(
                    f"pipeline {row['pipeline_id']} owns multiple active/paused tasks: "
                    f"{previous}, {task_id}"
                )
            reserved_pipelines[row["pipeline_id"]] = task_id

        effective_status = row["resume_status"] if status == "PAUSED" else status
        if effective_status in {"IMPLEMENTED", "REVIEWING", "APPLYING", "DONE"} and not row[
            "implement_done_at"
        ]:
            die(f"task {task_id} is {status} ({effective_status}) without implement_done_at")
        if effective_status in {"REVIEWING", "APPLYING", "DONE"} and not row[
            "review_started_at"
        ]:
            die(f"task {task_id} is {status} ({effective_status}) without review_started_at")
        if effective_status in {"APPLYING", "DONE"}:
            if not row["review_1_done_at"] or not row["review_2_done_at"]:
                die(f"task {task_id} is {status} ({effective_status}) without both review timestamps")
            if not row["apply_started_at"]:
                die(f"task {task_id} is {status} ({effective_status}) without apply_started_at")
        if status == "DONE":
            if not row["done_at"]:
                die(f"DONE task {task_id} has no done_at")
        elif row["done_at"]:
            die(f"non-DONE task {task_id} has done_at")

        ids.add(task_id)
        paths.add(path)

    validate_dependencies(rows)


def task_by_id(rows: list[dict[str, str]], task_id: str) -> dict[str, str]:
    matches = [row for row in rows if row["id"] == task_id]
    if len(matches) != 1:
        die(f"expected exactly one task {task_id!r}, found {len(matches)}")
    return matches[0]


def event_payload(
    row: Mapping[str, str] | None,
    *,
    phase: str = "translation",
    event: str,
    role: str,
    model: str,
    effort: str,
    detail: str,
    from_status: str = "",
    to_status: str = "",
    pipeline_id: str = "",
) -> dict[str, object]:
    if not role.strip() or not model.strip() or not effort.strip():
        die(
            f"event {event!r} requires non-empty role, model, and reasoning effort "
            "for auditability"
        )
    if effort not in {"none", "minimal", "low", "medium", "high", "xhigh", "max"}:
        die(f"event {event!r} has invalid reasoning effort {effort!r}")
    return {
        "ts": now_utc(),
        "phase": phase,
        "task_id": row.get("id", "") if row else "",
        "path": row.get("path", "") if row else "",
        "pipeline_id": pipeline_id or (row.get("pipeline_id", "") if row else ""),
        "role": role,
        "event": event,
        "from_status": from_status,
        "to_status": to_status,
        "model": model,
        "reasoning_effort": effort,
        "attempt": int(row.get("attempt", "0") or 0) if row else 0,
        "detail": detail,
    }


def cmd_invalidate(args: argparse.Namespace) -> None:
    """Record a Phase 0 invalidation without changing task state."""
    ensure_branch(args.skip_branch_check)
    queue, fingerprint, events, _ = common_paths(args)
    with QueueLock(queue):
        rows = read_tsv(queue)
        validate_rows(rows)
        verify_fingerprint(rows, fingerprint, Path(args.linux_sha_file))
        # A failed Phase 0 may already have produced a provisional queue and
        # even locally paused/blocked rows before the scope gate exposes its
        # defect.  It is still safe to record an invalidation provided no
        # source pipeline reached a terminal acceptance state and no lease is
        # active.  The command deliberately preserves the TSV unchanged; the
        # caller must archive it and create a brand-new queue from regenerated
        # Phase 0 scope rather than rewriting its rows.
        disallowed = [
            row for row in rows
            if row["status"] not in {"TODO", "BLOCKED", "PAUSED"}
        ]
        if disallowed:
            details = ", ".join(
                f"{row['id']}={row['status']}" for row in disallowed[:10]
            )
            die(
                "queue invalidation requires no active, IMPLEMENTED, REVIEWING, "
                f"APPLYING, or DONE rows; found {details}"
            )
        append_event(
            events,
            event_payload(
                None,
                phase="phase0",
                event="queue_invalidated",
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=f"archive={args.archive}; {args.reason}",
            ),
        )
    print(json.dumps({"event": "queue_invalidated", "archive": args.archive}))


def parse_dependencies(row: Mapping[str, str]) -> list[str]:
    return [value.strip() for value in row.get("dependencies", "").split(";") if value.strip()]


def ready_tasks(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    status_by_id = {row["id"]: row["status"] for row in rows}
    ready: list[dict[str, str]] = []
    for row in rows:
        if row["status"] != "TODO":
            continue
        dependencies = parse_dependencies(row)
        unknown = [dep for dep in dependencies if dep not in status_by_id]
        if unknown:
            die(f"task {row['id']} has unknown dependencies: {', '.join(unknown)}")
        if all(status_by_id[dep] == "DONE" for dep in dependencies):
            ready.append(row)
    risk_order = {"high": 0, "medium": 1, "low": 2}
    ready.sort(
        key=lambda row: (
            -float(row.get("weight", "0") or 0),
            risk_order.get(row.get("risk", "medium"), 1),
            row["id"],
        )
    )
    return ready


def update_status(row: dict[str, str], new_status: str, timestamp: str) -> tuple[str, str]:
    old_status = row["status"]
    if new_status not in TRANSITIONS.get(old_status, set()):
        die(f"invalid transition for {row['id']}: {old_status} -> {new_status}")
    row["status"] = new_status
    row["updated_at"] = timestamp
    return old_status, new_status


def common_paths(args: argparse.Namespace) -> tuple[Path, Path, Path, Path]:
    return Path(args.queue), Path(args.fingerprint), Path(args.events), Path(args.logs_root)


def ensure_clean_initial_artifacts(
    queue: Path, fingerprint: Path, events: Path, logs_root: Path
) -> None:
    if queue.exists() and queue.stat().st_size > 0:
        die(f"refusing to overwrite existing queue: {queue}")
    if fingerprint.exists():
        die(f"refusing to initialize while fingerprint exists: {fingerprint}")
    if events.exists() and events.stat().st_size > 0:
        die(f"refusing to mix a new queue with existing events: {events}")
    if logs_root.exists():
        stale = [
            path
            for path in logs_root.iterdir()
            if path.name != ".gitkeep"
        ]
        if stale:
            die(
                f"refusing to initialize with existing task evidence under {logs_root}: "
                + ", ".join(str(path) for path in stale[:5])
            )


def validate_scope_translation_row(source: Mapping[str, str], linux_root: Path) -> None:
    task_id = source.get("id", "")
    linux_value = source.get("linux_path", "")
    destination_value = source.get("destination_path", "")
    linux_path = validate_relative_posix_path(
        linux_value, context=f"scope task {task_id} Linux path"
    )
    destination = validate_relative_posix_path(
        destination_value, context=f"scope task {task_id} destination"
    )
    if destination.parts[0] != "src" or destination.suffix != ".rs":
        die(
            f"scope task {task_id} destination must be a Rust file under src/: "
            f"{destination_value!r}"
        )
    source_file = linux_root.joinpath(*linux_path.parts)
    if not source_file.is_file():
        die(f"scope task {task_id} cites missing pinned Linux file: {source_file}")
    linux_root_resolved = linux_root.resolve()
    source_resolved = source_file.resolve()
    if not source_resolved.is_relative_to(linux_root_resolved):
        die(
            f"scope task {task_id} resolves outside the pinned Linux tree: "
            f"{source_file} -> {source_resolved}"
        )
    destination_file = Path(*destination.parts)
    if destination_file.exists() or destination_file.is_symlink():
        die(
            f"fresh rewrite destination already exists for task {task_id}: "
            f"{destination_file}; remove historical translation contamination before init"
        )
    validate_architectures(
        source.get("architectures", ""), context=f"scope task {task_id}"
    )


def validate_scope_mechanical_row(source: Mapping[str, str], scope: Path) -> None:
    row_id = source.get("id", "")
    source_class = source.get("class", "")
    if source_class not in SCOPE_CLASSES:
        die(f"scope row {row_id} has invalid class {source_class!r}")
    if source.get("metadata_status", "") != "COMPLETE":
        die(
            f"scope row {row_id} is not mechanically complete: "
            f"metadata_status={source.get('metadata_status', '')!r}"
        )
    if not source.get("metadata_evidence", "").strip():
        die(f"scope row {row_id} has no metadata_evidence")
    semantic_status = source.get("semantic_status", "")
    if semantic_status not in {"PENDING_REVIEW", "COMPLETE", "NOT_APPLICABLE"}:
        die(
            f"scope row {row_id} has invalid semantic_status "
            f"{semantic_status!r}; use PENDING_REVIEW until reviewed"
        )
    try:
        validate_relative_posix_path(
            source.get("linux_path", ""), context=f"scope row {row_id} Linux path"
        )
    except SystemExit:
        # Generated Linux inputs may be represented only by metadata evidence;
        # RUST_TRANSLATE rows are checked strictly below.
        if source_class == "RUST_TRANSLATE":
            raise


def cmd_init(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    queue, fingerprint, events, logs_root = common_paths(args)
    scope = Path(args.scope)
    linux_root = Path(args.linux_root)
    linux_sha_file = Path(args.linux_sha_file)
    with QueueLock(queue):
        ensure_clean_initial_artifacts(queue, fingerprint, events, logs_root)
        if not scope.is_file():
            die(f"missing scope file: {scope}")
        linux_sha = verify_linux_checkout(linux_root, linux_sha_file)
        with scope.open("r", encoding="utf-8", newline="") as handle:
            reader = csv.DictReader(handle, delimiter="\t")
            if reader.fieldnames is None:
                die(f"scope file has no header: {scope}")
            missing = sorted(SCOPE_REQUIRED_FIELDS - set(reader.fieldnames))
            if missing:
                die(f"scope file is missing required columns: {', '.join(missing)}")
            scope_rows = [dict(row) for row in reader]
        if not scope_rows:
            die(f"scope contains no rows: {scope}")
        for source in scope_rows:
            validate_scope_mechanical_row(source, scope)
        selected = [row for row in scope_rows if row["class"] == "RUST_TRANSLATE"]
        if not selected:
            die("scope contains no RUST_TRANSLATE rows")
        created = now_utc()
        rows: list[dict[str, str]] = []
        for source in selected:
            validate_scope_translation_row(source, linux_root)
            risk = source.get("risk", "medium") or "medium"
            if risk not in {"low", "medium", "high"}:
                die(f"invalid risk for scope row {source['id']}: {risk!r}")
            recommended = source.get("recommended_implementer", "")
            if not recommended:
                recommended = "luna"
            if recommended not in {"luna", "spark"}:
                die(
                    f"invalid recommended_implementer for scope row {source['id']}: "
                    f"{recommended!r}"
                )
            row = {field: "" for field in FIELDS}
            row.update(
                {
                    "id": source["id"],
                    "path": source["destination_path"],
                    "created_at": created,
                    "status": "TODO",
                    "linux_path": source["linux_path"],
                    "architectures": source["architectures"],
                    "cluster": source["cluster"],
                    "weight": source["weight"],
                    "risk": risk,
                    "dependencies": source["dependencies"],
                    "recommended_implementer": recommended,
                    "attempt": "0",
                    "updated_at": created,
                }
            )
            rows.append(row)
        rows.sort(key=lambda row: row["id"])
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        initialization_events = [
            event_payload(
                None,
                event="queue_initialized",
                role="queue_tool",
                model="none",
                effort="none",
                detail=f"created {len(rows)} tasks from {scope}; linux_sha={linux_sha}",
            )
        ]
        initialization_events.extend(
            event_payload(
                row,
                event="task_created",
                role="queue_tool",
                model="none",
                effort="none",
                detail="inventoried before Phase 1",
                to_status="TODO",
            )
            for row in rows
        )
        append_events(events, initialization_events)
    print(json.dumps({"queue": str(queue), "tasks": len(rows), "created_at": created}))


def cmd_freeze(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    queue, fingerprint, events, _ = common_paths(args)
    with QueueLock(queue):
        rows = read_tsv(queue, FIELDS)
        validate_rows(rows)
        if any(row["status"] != "TODO" for row in rows):
            die("queue can be frozen only before any task leaves TODO")
        linux_sha = verify_linux_checkout(Path(args.linux_root), Path(args.linux_sha_file))
        if fingerprint.exists():
            verify_fingerprint(rows, fingerprint, Path(args.linux_sha_file))
            digest, _, _ = read_fingerprint(fingerprint)
            print(
                json.dumps(
                    {
                        "tasks": len(rows),
                        "sha256": digest,
                        "fingerprint": str(fingerprint),
                        "already_frozen": True,
                    }
                )
            )
            return
        digest = write_fingerprint(fingerprint, rows, linux_sha)
        append_event(
            events,
            event_payload(
                None,
                event="queue_frozen",
                role="queue_tool",
                model="none",
                effort="none",
                detail=f"tasks={len(rows)} sha256={digest} linux_sha={linux_sha}",
            ),
        )
    print(json.dumps({"tasks": len(rows), "sha256": digest, "fingerprint": str(fingerprint)}))


def cmd_verify(args: argparse.Namespace) -> None:
    queue, fingerprint, _, logs_root = common_paths(args)
    with QueueLock(queue):
        verify_linux_checkout(Path(args.linux_root), Path(args.linux_sha_file))
        rows = read_tsv(queue, FIELDS)
        validate_rows(rows)
        verify_fingerprint(rows, fingerprint, Path(args.linux_sha_file))
        for row in rows:
            if row["status"] == "DONE":
                require_done_evidence(row, logs_root, Path(args.linux_sha_file))
    print(json.dumps({"ok": True, "tasks": len(rows), "sha256": immutable_digest(rows)}))


def cmd_claim(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    if not args.pipeline.strip() or not args.worker.strip():
        die("--pipeline and --worker must be non-empty")
    validate_pipeline_id(args.pipeline, context="claim --pipeline")
    if args.lease_minutes <= 0:
        die("--lease-minutes must be positive")
    queue, fingerprint, events, logs_root = common_paths(args)
    with QueueLock(queue):
        verify_linux_checkout(Path(args.linux_root), Path(args.linux_sha_file))
        rows = read_tsv(queue, FIELDS)
        validate_rows(rows)
        verify_fingerprint(rows, fingerprint, Path(args.linux_sha_file))
        reserved = [
            row
            for row in rows
            if row["pipeline_id"] == args.pipeline
            and (row["status"] in ACTIVE_STATUSES or row["status"] == "PAUSED")
        ]
        if reserved:
            die(
                f"pipeline {args.pipeline} already owns active/paused task "
                f"{reserved[0]['id']} ({reserved[0]['status']})"
            )
        ready = ready_tasks(rows)
        if args.risk:
            ready = [row for row in ready if row["risk"] in set(args.risk)]
        if not ready:
            print(json.dumps({"claimed": False, "reason": "no ready tasks"}))
            return
        row = ready[0]
        timestamp = now_utc()
        old_status, new_status = update_status(row, "IN_PROGRESS", timestamp)
        if not row["work_started_at"]:
            row["work_started_at"] = timestamp
        row["pipeline_id"] = args.pipeline
        row["lease_owner"] = args.worker
        row["attempt"] = str(int(row["attempt"] or 0) + 1)
        expiry = dt.datetime.now(dt.timezone.utc) + dt.timedelta(minutes=args.lease_minutes)
        row["lease_expires_at"] = expiry.isoformat(timespec="milliseconds").replace("+00:00", "Z")
        row["last_error"] = ""
        task_dir = logs_root / row["id"]
        task_dir.mkdir(parents=True, exist_ok=True)
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        append_event(
            events,
            event_payload(
                row,
                event="claimed",
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=f"worker={args.worker}; lease_minutes={args.lease_minutes}",
                from_status=old_status,
                to_status=new_status,
                pipeline_id=args.pipeline,
            ),
        )
    print(json.dumps({"claimed": True, **row}, sort_keys=True))


def load_mutable(args: argparse.Namespace) -> tuple[Path, Path, Path, Path, list[dict[str, str]], dict[str, str]]:
    queue, fingerprint, events, logs_root = common_paths(args)
    rows = read_tsv(queue, FIELDS)
    validate_rows(rows)
    verify_fingerprint(rows, fingerprint, Path(args.linux_sha_file))
    row = task_by_id(rows, args.id)
    return queue, fingerprint, events, logs_root, rows, row


def assert_owner(row: Mapping[str, str], pipeline: str | None) -> None:
    if pipeline:
        validate_pipeline_id(pipeline, context=f"task {row['id']} --pipeline")
    if row.get("status") in ACTIVE_STATUSES and not pipeline:
        die(f"--pipeline is required for active task {row['id']}")
    if pipeline and row.get("pipeline_id") and row.get("pipeline_id") != pipeline:
        die(
            f"task {row['id']} is owned by pipeline {row.get('pipeline_id')!r}, "
            f"not {pipeline!r}"
        )


def mutate_simple(
    args: argparse.Namespace,
    *,
    to_status: str,
    event: str,
    timestamp_field: str | None = None,
    prerequisite=None,
    clear_lease: bool = False,
) -> None:
    ensure_branch(args.skip_branch_check)
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, logs_root, rows, row = load_mutable(args)
        assert_owner(row, getattr(args, "pipeline", None))
        if prerequisite:
            prerequisite(row, logs_root)
        timestamp = now_utc()
        old_status, new_status = update_status(row, to_status, timestamp)
        if timestamp_field:
            row[timestamp_field] = timestamp
        if clear_lease:
            row["lease_owner"] = ""
            row["lease_expires_at"] = ""
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        append_event(
            events,
            event_payload(
                row,
                event=event,
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=args.message,
                from_status=old_status,
                to_status=new_status,
            ),
        )
    print(json.dumps({"id": row["id"], "status": row["status"], "updated_at": row["updated_at"]}))


def require_evidence_files(
    row: Mapping[str, str], logs_root: Path, names: Iterable[str], *, stage: str
) -> None:
    task_dir = logs_root / row["id"]
    missing = [
        str(task_dir / name)
        for name in names
        if not (task_dir / name).is_file() or (task_dir / name).stat().st_size == 0
    ]
    if missing:
        die(f"cannot complete {stage}; missing or empty evidence files: " + ", ".join(missing))


def require_destination_translation(row: Mapping[str, str], sha_file: Path) -> None:
    destination = Path(row["path"])
    if destination.is_symlink():
        die(f"task {row['id']} destination must not be a symlink: {destination}")
    if not destination.is_file() or destination.stat().st_size == 0:
        die(f"task {row['id']} has no non-empty translated destination file: {destination}")

    repository_root = Path.cwd().resolve()
    destination_resolved = destination.resolve()
    if not destination_resolved.is_relative_to(repository_root):
        die(
            f"task {row['id']} destination resolves outside the repository: "
            f"{destination} -> {destination_resolved}"
        )

    try:
        text = destination.read_text(encoding="utf-8")
    except UnicodeDecodeError as exc:
        die(f"task {row['id']} destination is not UTF-8 Rust source: {exc}")

    linux_sha = read_pinned_linux_sha(sha_file)
    required_headers = {
        f"//! linux-source: {row['linux_path']}",
        f"//! linux-revision: {linux_sha}",
        f"//! architectures: {row['architectures']}",
        f"//! rewrite-task: {row['id']}",
    }
    header_lines = {line.strip() for line in text.splitlines()[:64]}
    missing_headers = sorted(required_headers - header_lines)
    if missing_headers:
        die(
            f"task {row['id']} destination is missing immutable provenance headers: "
            + ", ".join(missing_headers)
        )

    prohibited_patterns = {
        r"\btodo\s*!\s*\(": "todo! placeholder",
        r"\bunimplemented\s*!\s*\(": "unimplemented! placeholder",
        r"#\s*\[\s*test\s*\]": "project-authored Rust test",
        r"#\s*\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]": "cfg(test) code",
    }
    for pattern, description in prohibited_patterns.items():
        if re.search(pattern, text):
            die(f"task {row['id']} destination contains forbidden {description}: {destination}")


def require_implementation_evidence(
    row: Mapping[str, str], logs_root: Path, sha_file: Path
) -> None:
    require_evidence_files(
        row, logs_root, ["implementation.md", "candidate.diff"], stage="implementation"
    )
    require_destination_translation(row, sha_file)


def cmd_mark_implemented(args: argparse.Namespace) -> None:
    mutate_simple(
        args,
        to_status="IMPLEMENTED",
        event="implementation_done",
        timestamp_field="implement_done_at",
        prerequisite=lambda row, logs_root: require_implementation_evidence(
            row, logs_root, Path(args.linux_sha_file)
        ),
    )


def cmd_start_review(args: argparse.Namespace) -> None:
    mutate_simple(
        args,
        to_status="REVIEWING",
        event="review_started",
        timestamp_field="review_started_at",
    )


def cmd_mark_review(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, logs_root, rows, row = load_mutable(args)
        assert_owner(row, getattr(args, "pipeline", None))
        if row["status"] != "REVIEWING":
            die(f"task {row['id']} must be REVIEWING, found {row['status']}")
        field = "review_1_done_at" if args.slot == 1 else "review_2_done_at"
        report = "parity-review.md" if args.slot == 1 else "rust-review.md"
        if row[field]:
            die(f"review slot {args.slot} already completed for task {row['id']}")
        require_evidence_files(
            row, logs_root, [report], stage=f"review slot {args.slot}"
        )
        timestamp = now_utc()
        row[field] = timestamp
        row["updated_at"] = timestamp
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        review_role = args.role or (
            "parity_reviewer" if args.slot == 1 else "rust_reviewer"
        )
        append_event(
            events,
            event_payload(
                row,
                event=f"review_{args.slot}_done",
                role=review_role,
                model=args.model,
                effort=args.effort,
                detail=args.message,
                from_status="REVIEWING",
                to_status="REVIEWING",
            ),
        )
    print(json.dumps({"id": row["id"], "slot": args.slot, "completed_at": row[field]}))


def require_reviews(row: Mapping[str, str], _logs_root: Path | None = None) -> None:
    if not row.get("review_1_done_at") or not row.get("review_2_done_at"):
        die(f"task {row['id']} cannot enter APPLYING until both reviews are done")


def cmd_start_apply(args: argparse.Namespace) -> None:
    mutate_simple(
        args,
        to_status="APPLYING",
        event="apply_started",
        timestamp_field="apply_started_at",
        prerequisite=require_reviews,
    )


def require_done_evidence(row: Mapping[str, str], logs_root: Path, sha_file: Path) -> None:
    require_evidence_files(row, logs_root, EVIDENCE_FILES, stage="DONE")
    require_reviews(row)
    require_destination_translation(row, sha_file)


def cmd_done(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, logs_root, rows, row = load_mutable(args)
        assert_owner(row, getattr(args, "pipeline", None))
        if row["status"] != "APPLYING":
            die(f"task {row['id']} must be APPLYING, found {row['status']}")
        require_done_evidence(row, logs_root, Path(args.linux_sha_file))
        timestamp = now_utc()
        old_status, new_status = update_status(row, "DONE", timestamp)
        row["done_at"] = timestamp
        row["lease_owner"] = ""
        row["lease_expires_at"] = ""
        row["resume_status"] = ""
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        append_event(
            events,
            event_payload(
                row,
                event="done",
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=args.message or "source translation pipeline complete; not compiled or tested",
                from_status=old_status,
                to_status=new_status,
            ),
        )
    print(json.dumps({"id": row["id"], "status": "DONE", "done_at": timestamp}))


def cmd_terminal(args: argparse.Namespace, status: str, event: str) -> None:
    ensure_branch(args.skip_branch_check)
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, _, rows, row = load_mutable(args)
        assert_owner(row, getattr(args, "pipeline", None))
        if status not in TRANSITIONS.get(row["status"], set()):
            die(f"invalid transition for {row['id']}: {row['status']} -> {status}")
        timestamp = now_utc()
        old_status, new_status = update_status(row, status, timestamp)
        row["last_error"] = args.reason
        row["resume_status"] = old_status if status == "PAUSED" else ""
        row["lease_owner"] = ""
        row["lease_expires_at"] = ""
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        append_event(
            events,
            event_payload(
                row,
                event=event,
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=args.reason,
                from_status=old_status,
                to_status=new_status,
            ),
        )
    print(
        json.dumps(
            {
                "id": row["id"],
                "status": status,
                "resume_status": row["resume_status"],
                "reason": args.reason,
            }
        )
    )


def cmd_block(args: argparse.Namespace) -> None:
    cmd_terminal(args, "BLOCKED", "blocked")


def cmd_pause(args: argparse.Namespace) -> None:
    cmd_terminal(args, "PAUSED", "paused")


def cmd_resume(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    if not args.pipeline.strip() or not args.worker.strip():
        die("--pipeline and --worker must be non-empty")
    validate_pipeline_id(args.pipeline, context="resume --pipeline")
    if args.lease_minutes <= 0:
        die("--lease-minutes must be positive")
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, _, rows, row = load_mutable(args)
        if row["status"] != "PAUSED":
            die(f"task {row['id']} can be resumed only from PAUSED")
        target = row["resume_status"]
        if target not in ACTIVE_STATUSES:
            die(f"task {row['id']} has invalid resume_status {target!r}")
        conflicts = [
            item
            for item in rows
            if item["id"] != row["id"]
            and item["pipeline_id"] == args.pipeline
            and (item["status"] in ACTIVE_STATUSES or item["status"] == "PAUSED")
        ]
        if conflicts:
            die(
                f"pipeline {args.pipeline} already owns active/paused task "
                f"{conflicts[0]['id']} ({conflicts[0]['status']})"
            )
        timestamp = now_utc()
        old_status, new_status = update_status(row, target, timestamp)
        row["pipeline_id"] = args.pipeline
        row["lease_owner"] = args.worker
        expiry = dt.datetime.now(dt.timezone.utc) + dt.timedelta(minutes=args.lease_minutes)
        row["lease_expires_at"] = expiry.isoformat(timespec="milliseconds").replace(
            "+00:00", "Z"
        )
        row["resume_status"] = ""
        row["last_error"] = ""
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        append_event(
            events,
            event_payload(
                row,
                event="resumed",
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=f"worker={args.worker}; lease_minutes={args.lease_minutes}",
                from_status=old_status,
                to_status=new_status,
                pipeline_id=args.pipeline,
            ),
        )
    print(
        json.dumps(
            {
                "id": row["id"],
                "status": row["status"],
                "pipeline_id": row["pipeline_id"],
                "lease_expires_at": row["lease_expires_at"],
            }
        )
    )


def archive_evidence(logs_root: Path, row: Mapping[str, str], timestamp: str) -> str:
    task_dir = logs_root / row["id"]
    existing = [task_dir / name for name in EVIDENCE_FILES if (task_dir / name).exists()]
    if not existing:
        return ""
    stamp = timestamp.replace("-", "").replace(":", "").replace(".", "")
    archive_dir = task_dir / "attempts" / f"attempt-{int(row.get('attempt', '0') or 0):03d}-{stamp}"
    archive_dir.mkdir(parents=True, exist_ok=False)
    for source in existing:
        os.replace(source, archive_dir / source.name)
    return str(archive_dir)


def cmd_requeue(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, logs_root, rows, row = load_mutable(args)
        if row["status"] not in {"BLOCKED", "PAUSED"}:
            die(f"task {row['id']} can be requeued only from BLOCKED/PAUSED")
        timestamp = now_utc()
        archive = archive_evidence(logs_root, row, timestamp)
        old_status, new_status = update_status(row, "TODO", timestamp)
        row["pipeline_id"] = ""
        row["lease_owner"] = ""
        row["lease_expires_at"] = ""
        row["implement_done_at"] = ""
        row["review_started_at"] = ""
        row["review_1_done_at"] = ""
        row["review_2_done_at"] = ""
        row["apply_started_at"] = ""
        row["done_at"] = ""
        row["resume_status"] = ""
        row["last_error"] = ""
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        detail_parts = [part for part in [args.message, f"archived={archive}" if archive else ""] if part]
        append_event(
            events,
            event_payload(
                row,
                event="requeued",
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail="; ".join(detail_parts),
                from_status=old_status,
                to_status=new_status,
            ),
        )
    print(json.dumps({"id": row["id"], "status": "TODO", "archive": archive}))


def cmd_heartbeat(args: argparse.Namespace) -> None:
    ensure_branch(args.skip_branch_check)
    if args.lease_minutes <= 0:
        die("--lease-minutes must be positive")
    queue = Path(args.queue)
    with QueueLock(queue):
        queue, _, events, _, rows, row = load_mutable(args)
        assert_owner(row, args.pipeline)
        if row["status"] not in ACTIVE_STATUSES:
            die(f"cannot heartbeat non-active task {row['id']} ({row['status']})")
        timestamp = now_utc()
        expiry = dt.datetime.now(dt.timezone.utc) + dt.timedelta(minutes=args.lease_minutes)
        row["lease_expires_at"] = expiry.isoformat(timespec="milliseconds").replace("+00:00", "Z")
        row["updated_at"] = timestamp
        validate_rows(rows)
        atomic_write_tsv(queue, rows)
        append_event(
            events,
            event_payload(
                row,
                event="heartbeat",
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=f"lease_minutes={args.lease_minutes}",
                from_status=row["status"],
                to_status=row["status"],
            ),
        )
    print(json.dumps({"id": row["id"], "lease_expires_at": row["lease_expires_at"]}))


def cmd_stats(args: argparse.Namespace) -> None:
    queue = Path(args.queue)
    if not queue.exists():
        payload = {"queue": str(queue), "exists": False}
        print(json.dumps(payload, indent=2 if not args.json else None, sort_keys=True))
        return
    with QueueLock(queue):
        rows = read_tsv(queue, FIELDS)
        validate_rows(rows)
        fingerprint = Path(args.fingerprint)
        fingerprint_ok = False
        if fingerprint.exists():
            verify_fingerprint(rows, fingerprint, Path(args.linux_sha_file))
            fingerprint_ok = True
        counts = {status: 0 for status in sorted(ALL_STATUSES)}
        weights = {status: 0.0 for status in sorted(ALL_STATUSES)}
        for row in rows:
            counts[row["status"]] += 1
            weights[row["status"]] += float(row["weight"] or 0)
        total_weight = sum(weights.values())
        done_weight = weights["DONE"]
        active = [
            {
                "id": row["id"],
                "path": row["path"],
                "status": row["status"],
                "pipeline_id": row["pipeline_id"],
                "lease_owner": row["lease_owner"],
                "lease_expires_at": row["lease_expires_at"],
            }
            for row in rows
            if row["status"] in ACTIVE_STATUSES
        ]
        payload = {
            "queue": str(queue),
            "exists": True,
            "tasks": len(rows),
            "counts": counts,
            "done_percent": round((counts["DONE"] / len(rows) * 100) if rows else 0, 2),
            "total_weight": round(total_weight, 3),
            "done_weight": round(done_weight, 3),
            "done_weight_percent": round((done_weight / total_weight * 100) if total_weight else 0, 2),
            "active": active,
            "fingerprint_verified": fingerprint_ok,
        }
    if args.json:
        print(json.dumps(payload, sort_keys=True))
    else:
        print(f"queue: {payload['queue']}")
        print(f"tasks: {payload['tasks']}  done: {counts['DONE']} ({payload['done_percent']}%)")
        print(
            f"weight: {payload['done_weight']}/{payload['total_weight']} "
            f"({payload['done_weight_percent']}%)"
        )
        print("status counts:")
        for status in ["TODO", "IN_PROGRESS", "IMPLEMENTED", "REVIEWING", "APPLYING", "DONE", "BLOCKED", "PAUSED"]:
            print(f"  {status:12} {counts[status]}")
        if active:
            print("active:")
            for item in active:
                print(
                    f"  {item['pipeline_id'] or '-':4} {item['status']:12} "
                    f"{item['id']} {item['path']}"
                )


def cmd_stale(args: argparse.Namespace) -> None:
    queue = Path(args.queue)
    with QueueLock(queue):
        rows = read_tsv(queue, FIELDS)
        validate_rows(rows)
        verify_fingerprint(rows, Path(args.fingerprint), Path(args.linux_sha_file))
        now = dt.datetime.now(dt.timezone.utc)
        stale = []
        for row in rows:
            if row["status"] not in ACTIVE_STATUSES or not row["lease_expires_at"]:
                continue
            try:
                expiry = parse_utc(row["lease_expires_at"])
            except ValueError:
                stale.append({"id": row["id"], "reason": "invalid lease timestamp", **row})
                continue
            if expiry < now:
                stale.append(
                    {
                        "id": row["id"],
                        "path": row["path"],
                        "status": row["status"],
                        "pipeline_id": row["pipeline_id"],
                        "lease_owner": row["lease_owner"],
                        "lease_expires_at": row["lease_expires_at"],
                    }
                )
    print(json.dumps({"stale": stale, "count": len(stale)}, indent=2))


def add_common_files(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--queue", default=str(DEFAULT_QUEUE))
    parser.add_argument("--fingerprint", default=str(DEFAULT_FINGERPRINT))
    parser.add_argument("--events", default=str(DEFAULT_EVENTS))
    parser.add_argument("--logs-root", default=str(DEFAULT_LOGS_ROOT))
    parser.add_argument("--linux-sha-file", default=str(DEFAULT_LINUX_SHA_FILE))
    parser.add_argument("--linux-root", default=str(DEFAULT_LINUX_ROOT))
    parser.add_argument("--skip-branch-check", action="store_true", help=argparse.SUPPRESS)


def add_actor_args(
    parser: argparse.ArgumentParser,
    *,
    default_role: str,
    default_model: str,
    default_effort: str,
    pipeline_required: bool = True,
) -> None:
    parser.add_argument("--pipeline", required=pipeline_required, default="")
    parser.add_argument("--role", default=default_role)
    parser.add_argument("--model", default=default_model)
    parser.add_argument("--effort", default=default_effort)
    parser.add_argument("--message", default="")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    init = sub.add_parser("init", help="Create all RUST_TRANSLATE task rows from frozen SCOPE.tsv")
    add_common_files(init)
    init.add_argument("--scope", default="rewrite/SCOPE.tsv")
    init.set_defaults(func=cmd_init)

    freeze = sub.add_parser("freeze", help="Fingerprint immutable queue fields before Phase 1")
    add_common_files(freeze)
    freeze.set_defaults(func=cmd_freeze)

    invalidate = sub.add_parser(
        "invalidate",
        help="Record invalidation of an untouched provisional Phase 0 queue",
    )
    add_common_files(invalidate)
    invalidate.add_argument("--archive", required=True)
    invalidate.add_argument("--reason", required=True)
    invalidate.add_argument("--role", default="scope_architect")
    invalidate.add_argument("--model", default="gpt-5.6-sol")
    invalidate.add_argument("--effort", default="xhigh")
    invalidate.set_defaults(func=cmd_invalidate)

    verify = sub.add_parser("verify", help="Validate schema, identities, and immutable-field fingerprint")
    add_common_files(verify)
    verify.set_defaults(func=cmd_verify)

    claim = sub.add_parser("claim", help="Atomically claim one ready task for a pipeline")
    add_common_files(claim)
    claim.add_argument("--pipeline", required=True)
    claim.add_argument("--worker", required=True)
    claim.add_argument("--lease-minutes", type=int, default=90)
    claim.add_argument("--risk", action="append", choices=["low", "medium", "high"])
    claim.add_argument("--role", default="pipeline_coordinator")
    claim.add_argument("--model", default="gpt-5.6-terra")
    claim.add_argument("--effort", default="medium")
    claim.set_defaults(func=cmd_claim)

    implemented = sub.add_parser("mark-implemented", help="Mark candidate implementation complete")
    add_common_files(implemented)
    implemented.add_argument("--id", required=True)
    add_actor_args(
        implemented,
        default_role="implementer",
        default_model="gpt-5.6-luna",
        default_effort="medium",
    )
    implemented.set_defaults(func=cmd_mark_implemented)

    start_review = sub.add_parser("start-review", help="Open the two-review stage")
    add_common_files(start_review)
    start_review.add_argument("--id", required=True)
    add_actor_args(
        start_review,
        default_role="pipeline_coordinator",
        default_model="gpt-5.6-terra",
        default_effort="medium",
    )
    start_review.set_defaults(func=cmd_start_review)

    review = sub.add_parser("mark-review", help="Record one independent reviewer completion")
    add_common_files(review)
    review.add_argument("--id", required=True)
    review.add_argument("--slot", type=int, choices=[1, 2], required=True)
    add_actor_args(
        review,
        default_role="",
        default_model="gpt-5.6-terra",
        default_effort="high",
    )
    review.set_defaults(func=cmd_mark_review)

    apply = sub.add_parser("start-apply", help="Open applier stage after both reviews")
    add_common_files(apply)
    apply.add_argument("--id", required=True)
    add_actor_args(
        apply,
        default_role="applier",
        default_model="gpt-5.6-terra",
        default_effort="high",
    )
    apply.set_defaults(func=cmd_start_apply)

    done = sub.add_parser("done", help="Atomically close a task after evidence and application")
    add_common_files(done)
    done.add_argument("--id", required=True)
    add_actor_args(
        done,
        default_role="applier",
        default_model="gpt-5.6-terra",
        default_effort="high",
    )
    done.set_defaults(func=cmd_done)

    block = sub.add_parser("block", help="Stop a task because exact translation cannot proceed")
    add_common_files(block)
    block.add_argument("--id", required=True)
    block.add_argument("--reason", required=True)
    add_actor_args(
        block,
        default_role="pipeline_coordinator",
        default_model="gpt-5.6-terra",
        default_effort="medium",
        pipeline_required=False,
    )
    block.set_defaults(func=cmd_block)

    pause = sub.add_parser("pause", help="Preserve an active task across quota/interruption")
    add_common_files(pause)
    pause.add_argument("--id", required=True)
    pause.add_argument("--reason", required=True)
    add_actor_args(
        pause,
        default_role="pipeline_coordinator",
        default_model="gpt-5.6-terra",
        default_effort="medium",
    )
    pause.set_defaults(func=cmd_pause)

    resume = sub.add_parser("resume", help="Resume a PAUSED task at its exact saved stage")
    add_common_files(resume)
    resume.add_argument("--id", required=True)
    resume.add_argument("--pipeline", required=True)
    resume.add_argument("--worker", required=True)
    resume.add_argument("--lease-minutes", type=int, default=90)
    resume.add_argument("--role", default="pipeline_coordinator")
    resume.add_argument("--model", default="gpt-5.6-terra")
    resume.add_argument("--effort", default="medium")
    resume.set_defaults(func=cmd_resume)

    requeue = sub.add_parser("requeue", help="Restart a resolved BLOCKED/PAUSED task from TODO")
    add_common_files(requeue)
    requeue.add_argument("--id", required=True)
    add_actor_args(
        requeue,
        default_role="pipeline_coordinator",
        default_model="gpt-5.6-terra",
        default_effort="medium",
        pipeline_required=False,
    )
    requeue.set_defaults(func=cmd_requeue)

    heartbeat = sub.add_parser("heartbeat", help="Renew an active task lease")
    add_common_files(heartbeat)
    heartbeat.add_argument("--id", required=True)
    heartbeat.add_argument("--pipeline", required=True)
    heartbeat.add_argument("--lease-minutes", type=int, default=90)
    heartbeat.add_argument("--role", default="pipeline_coordinator")
    heartbeat.add_argument("--model", default="gpt-5.6-terra")
    heartbeat.add_argument("--effort", default="medium")
    heartbeat.set_defaults(func=cmd_heartbeat)

    stats = sub.add_parser("stats", help="Print queue status, active tasks, and weighted completion")
    add_common_files(stats)
    stats.add_argument("--json", action="store_true")
    stats.set_defaults(func=cmd_stats)

    stale = sub.add_parser("stale", help="List expired active leases without reassigning them")
    add_common_files(stale)
    stale.set_defaults(func=cmd_stale)

    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()
    ensure_repository_root()
    args.func(args)


if __name__ == "__main__":
    main()
