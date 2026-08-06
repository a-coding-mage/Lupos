#!/usr/bin/env python3
"""Bundle, materialize, and verify reproducible Phase 0 cache artifacts.

This utility deliberately operates only below ``rewrite/``. It never reads or
writes ``vendor/linux`` or ``src`` and does not invoke a compiler or Kbuild.
Raw Phase 0 tables are cached locally for the workflow; the committed bundles
preserve byte-exact copies in deterministic gzip archives. XZ is intentionally
not used.
"""

from __future__ import annotations

import argparse
import csv
import gzip
import hashlib
import os
from pathlib import Path, PurePosixPath
import tarfile
import tempfile
from typing import Iterable


SCHEMA_VERSION = "phase0-materialized-bundles-v2-gzip"
BUNDLE_DIRNAME = "phase0-bundles"
MAX_PART_BYTES = 90 * 1024 * 1024
BUNDLE_LAYOUT: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("manifests.tar.gz", (
        "SCOPE.tsv", "FILE_MAP.tsv", "SYMBOLS.tsv", "ABI.tsv",
        "LIFETIMES.tsv", "DRIVER_ABI.tsv",
    )),
    ("metadata-x86_64.tar.gz", ("metadata/x86_64",)),
    ("metadata-aarch64.tar.gz", ("metadata/aarch64",)),
    ("metadata-shared.tar.gz", (
        "metadata/authoritative_manifests.tsv",
        "metadata/compiler-predicates-binding.tsv",
        "metadata/header_closure.tsv",
        "metadata/header_components.tsv",
        "metadata/header_context_edges.tsv",
        "metadata/header_include_edges.tsv",
        "metadata/manifest.tsv",
        "metadata/oracle_classification.tsv",
        "metadata/summary.json",
        "metadata/task_dependencies.tsv",
    )),
)
BUNDLES_FIELDS = ("schema_version", "archive_group", "bundle", "part", "sha256", "bytes", "member_count")
MEMBER_FIELDS = ("archive_group", "path", "sha256", "bytes")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def safe_rel(value: str) -> Path:
    pure = PurePosixPath(value)
    if pure.is_absolute() or ".." in pure.parts or value in {"", "."}:
        raise ValueError(f"unsafe Phase 0 relative path: {value!r}")
    return Path(*pure.parts)


def rewrite_root(value: str) -> Path:
    root = Path(value).resolve()
    if root.name != "rewrite" or not root.is_dir():
        raise SystemExit(f"--rewrite must resolve to an existing rewrite directory, got {root}")
    return root


def bundled_files(root: Path, roots: Iterable[str]) -> list[Path]:
    files: list[Path] = []
    for item in roots:
        source = root / safe_rel(item)
        if not source.exists():
            raise SystemExit(f"required materialized Phase 0 path is missing: {source}")
        if source.is_symlink():
            raise SystemExit(f"refusing symlinked Phase 0 path: {source}")
        if source.is_dir():
            for candidate in sorted(source.rglob("*")):
                if candidate.is_symlink():
                    raise SystemExit(f"refusing symlinked Phase 0 path: {candidate}")
                if candidate.is_file():
                    files.append(candidate)
        elif source.is_file():
            files.append(source)
        else:
            raise SystemExit(f"required Phase 0 path is not regular: {source}")
    return sorted(set(files), key=lambda path: path.relative_to(root).as_posix())


def fsync_file(path: Path) -> None:
    with path.open("rb") as handle:
        os.fsync(handle.fileno())


def write_archive_parts(group: str, root: Path, files: list[Path], bundle_dir: Path) -> list[dict[str, str]]:
    bundle_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(prefix=f".{group}.", suffix=".tmp", dir=bundle_dir, delete=False) as temp:
        compressed = Path(temp.name)
    try:
        with compressed.open("wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, compresslevel=9, mtime=0) as zipped:
                with tarfile.open(fileobj=zipped, mode="w|") as output:
                    for source in files:
                        relative = source.relative_to(root).as_posix()
                        info = output.gettarinfo(str(source), arcname=relative)
                        info.uid = info.gid = 0
                        info.uname = info.gname = ""
                        info.mtime = 0
                        info.mode = 0o644
                        with source.open("rb") as payload:
                            output.addfile(info, payload)
            raw.flush()
            os.fsync(raw.fileno())
        for old in bundle_dir.glob(f"{group}.part*"):
            if old.is_file():
                old.unlink()
        rows: list[dict[str, str]] = []
        with compressed.open("rb") as source:
            part = 0
            while True:
                with tempfile.NamedTemporaryFile(prefix=f".{group}.part{part:03d}.", suffix=".tmp", dir=bundle_dir, delete=False) as temp_part:
                    temporary = Path(temp_part.name)
                    remaining = MAX_PART_BYTES
                    while remaining:
                        block = source.read(min(1024 * 1024, remaining))
                        if not block:
                            break
                        temp_part.write(block)
                        remaining -= len(block)
                    temp_part.flush()
                    os.fsync(temp_part.fileno())
                if temporary.stat().st_size == 0:
                    temporary.unlink(missing_ok=True)
                    break
                final = bundle_dir / f"{group}.part{part:03d}"
                os.replace(temporary, final)
                rows.append({
                    "schema_version": SCHEMA_VERSION,
                    "archive_group": group,
                    "bundle": final.name,
                    "part": str(part),
                    "sha256": sha256_file(final),
                    "bytes": str(final.stat().st_size),
                    "member_count": str(len(files)),
                })
                part += 1
        return rows
    finally:
        compressed.unlink(missing_ok=True)


def write_tsv_atomic(path: Path, fields: tuple[str, ...], rows: list[dict[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", newline="", prefix=f".{path.name}.", suffix=".tmp", dir=path.parent, delete=False) as temp:
        temporary = Path(temp.name)
        writer = csv.DictWriter(temp, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
        temp.flush()
        os.fsync(temp.fileno())
    try:
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def write_hash_atomic(path: Path, digest: str) -> None:
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", prefix=f".{path.name}.", suffix=".tmp", dir=path.parent, delete=False) as temp:
        temporary = Path(temp.name)
        temp.write(f"{digest}  BUNDLES.tsv\n")
        temp.flush()
        os.fsync(temp.fileno())
    try:
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def read_tsv(path: Path, required: tuple[str, ...]) -> list[dict[str, str]]:
    try:
        with path.open("r", encoding="utf-8", newline="") as source:
            reader = csv.DictReader(source, delimiter="\t")
            if tuple(reader.fieldnames or ()) != required:
                raise SystemExit(f"unexpected schema in {path}")
            return list(reader)
    except FileNotFoundError:
        raise SystemExit(f"missing bundle index: {path}") from None


def build(root: Path) -> None:
    bundle_dir = root / BUNDLE_DIRNAME
    bundles: list[dict[str, str]] = []
    members: list[dict[str, str]] = []
    for group, roots in BUNDLE_LAYOUT:
        files = bundled_files(root, roots)
        bundles.extend(write_archive_parts(group, root, files, bundle_dir))
        members.extend({
            "archive_group": group,
            "path": source.relative_to(root).as_posix(),
            "sha256": sha256_file(source),
            "bytes": str(source.stat().st_size),
        } for source in files)
    bundles.sort(key=lambda row: (row["archive_group"], int(row["part"])))
    members.sort(key=lambda row: (row["archive_group"], row["path"]))
    write_tsv_atomic(bundle_dir / "BUNDLES.tsv", BUNDLES_FIELDS, bundles)
    write_tsv_atomic(bundle_dir / "MEMBERS.tsv", MEMBER_FIELDS, members)
    write_hash_atomic(bundle_dir / "BUNDLES.sha256", sha256_file(bundle_dir / "BUNDLES.tsv"))
    print(f"bundled {len(members)} Phase 0 files into {len(bundles)} gzip artifact part(s) under {bundle_dir}")


def expected(root: Path) -> tuple[dict[str, list[dict[str, str]]], dict[str, dict[str, str]]]:
    bundle_dir = root / BUNDLE_DIRNAME
    bundle_rows = read_tsv(bundle_dir / "BUNDLES.tsv", BUNDLES_FIELDS)
    member_rows = read_tsv(bundle_dir / "MEMBERS.tsv", MEMBER_FIELDS)
    expected_groups = {group for group, _ in BUNDLE_LAYOUT}
    groups: dict[str, list[dict[str, str]]] = {}
    for row in bundle_rows:
        group = row["archive_group"]
        if row["schema_version"] != SCHEMA_VERSION or group not in expected_groups:
            raise SystemExit("BUNDLES.tsv has an unsupported schema or archive group")
        groups.setdefault(group, []).append(row)
    if set(groups) != expected_groups:
        raise SystemExit("BUNDLES.tsv does not match the fixed Phase 0 bundle layout")
    for group, rows in groups.items():
        rows.sort(key=lambda row: int(row["part"]))
        for expected_part, row in enumerate(rows):
            if int(row["part"]) != expected_part or row["bundle"] != f"{group}.part{expected_part:03d}":
                raise SystemExit(f"non-contiguous or malformed parts for {group}")
            archive = bundle_dir / row["bundle"]
            if not archive.is_file() or archive.stat().st_size != int(row["bytes"]) or sha256_file(archive) != row["sha256"]:
                raise SystemExit(f"bundle hash mismatch: {archive}")
    try:
        recorded = (bundle_dir / "BUNDLES.sha256").read_text(encoding="utf-8").split()[0]
    except (FileNotFoundError, IndexError):
        raise SystemExit("missing or malformed BUNDLES.sha256") from None
    if recorded != sha256_file(bundle_dir / "BUNDLES.tsv"):
        raise SystemExit("BUNDLES.tsv fingerprint mismatch")
    members = {row["path"]: row for row in member_rows}
    if len(members) != len(member_rows):
        raise SystemExit("MEMBERS.tsv has duplicate paths")
    for path, row in members.items():
        safe_rel(path)
        if row["archive_group"] not in groups:
            raise SystemExit(f"MEMBERS.tsv references unknown archive group: {path}")
    return groups, members


def stream_group(root: Path, group: str, rows: list[dict[str, str]], members: dict[str, dict[str, str]], destination: Path | None, replace: bool) -> None:
    bundle_dir = root / BUNDLE_DIRNAME
    with tempfile.NamedTemporaryFile(prefix=f".{group}.", suffix=".tmp", dir=bundle_dir, delete=False) as combined:
        temporary = Path(combined.name)
        for row in rows:
            with (bundle_dir / row["bundle"]).open("rb") as part:
                while block := part.read(1024 * 1024):
                    combined.write(block)
        combined.flush()
        os.fsync(combined.fileno())
    try:
        seen: set[str] = set()
        with tarfile.open(temporary, mode="r:gz") as archive:
            for item in archive:
                if not item.isfile():
                    raise SystemExit(f"non-regular member in {group}: {item.name}")
                safe_rel(item.name)
                row = members.get(item.name)
                if row is None or row["archive_group"] != group or item.name in seen:
                    raise SystemExit(f"unexpected member in {group}: {item.name}")
                seen.add(item.name)
                payload = archive.extractfile(item)
                if payload is None:
                    raise SystemExit(f"unreadable member in {group}: {item.name}")
                local = root / safe_rel(item.name)
                expected_hash = row["sha256"]
                expected_bytes = int(row["bytes"])
                exists_matches = local.is_file() and local.stat().st_size == expected_bytes and sha256_file(local) == expected_hash
                target: Path | None = None
                if destination is not None and (not local.exists() or replace) and not exists_matches:
                    local.parent.mkdir(parents=True, exist_ok=True)
                    with tempfile.NamedTemporaryFile("wb", prefix=f".{local.name}.", suffix=".tmp", dir=local.parent, delete=False) as output:
                        target = Path(output.name)
                        digest = hashlib.sha256()
                        count = 0
                        while block := payload.read(1024 * 1024):
                            output.write(block)
                            digest.update(block)
                            count += len(block)
                        output.flush()
                        os.fsync(output.fileno())
                else:
                    digest = hashlib.sha256()
                    count = 0
                    while block := payload.read(1024 * 1024):
                        digest.update(block)
                        count += len(block)
                if count != expected_bytes or digest.hexdigest() != expected_hash:
                    if target is not None:
                        target.unlink(missing_ok=True)
                    raise SystemExit(f"member hash mismatch: {item.name}")
                if target is not None:
                    os.replace(target, local)
        expected_paths = {path for path, row in members.items() if row["archive_group"] == group}
        if seen != expected_paths:
            raise SystemExit(f"bundle member set mismatch for {group}")
    finally:
        temporary.unlink(missing_ok=True)


def verify(root: Path, require_materialized: bool) -> None:
    groups, members = expected(root)
    for group, rows in sorted(groups.items()):
        stream_group(root, group, rows, members, destination=None, replace=False)
    if require_materialized:
        for relative, row in members.items():
            local = root / safe_rel(relative)
            if not local.is_file() or local.stat().st_size != int(row["bytes"]) or sha256_file(local) != row["sha256"]:
                raise SystemExit(f"materialized cache differs from bundle: {local}")
    print(f"verified {len(members)} Phase 0 bundle members")


def materialize(root: Path, replace: bool) -> None:
    groups, members = expected(root)
    conflicts = []
    for relative, row in members.items():
        local = root / safe_rel(relative)
        if local.exists() and (not local.is_file() or local.stat().st_size != int(row["bytes"]) or sha256_file(local) != row["sha256"]):
            conflicts.append(local)
    if conflicts and not replace:
        raise SystemExit(f"refusing to replace {len(conflicts)} conflicting materialized file(s); rerun with --replace only after preserving them")
    for group, rows in sorted(groups.items()):
        stream_group(root, group, rows, members, destination=root, replace=replace)
    verify(root, require_materialized=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("bundle", "materialize", "verify"))
    parser.add_argument("--rewrite", default="rewrite", help="path to the rewrite directory")
    parser.add_argument("--replace", action="store_true", help="allow materialize to replace conflicting local cache files")
    args = parser.parse_args()
    root = rewrite_root(args.rewrite)
    if args.replace and args.action != "materialize":
        parser.error("--replace is valid only with materialize")
    if args.action == "bundle":
        build(root)
    elif args.action == "materialize":
        materialize(root, args.replace)
    else:
        verify(root, require_materialized=False)


if __name__ == "__main__":
    main()
