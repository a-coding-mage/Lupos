#!/usr/bin/env python3
"""Append Phase 1 incident lifecycle events under the canonical queue lock.

This helper intentionally never edits queue rows.  Normal lifecycle state
changes remain in rewrite_queue.py; this command provides auditable incident
events while verifying the same branch, Phase 0 identity, and frozen queue.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from rewrite_queue import (
    QueueLock,
    append_event,
    ensure_branch,
    event_payload,
    read_tsv,
    task_by_id,
    validate_rows,
    verify_fingerprint,
)


INCIDENT_EVENTS = {
    "INCIDENT_OPENED",
    "INCIDENT_REVIEW_STARTED",
    "INCIDENT_REVIEW_COMPLETED",
    "INCIDENT_RESOLVED",
}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("event", choices=sorted(INCIDENT_EVENTS))
    parser.add_argument("--incident", required=True)
    parser.add_argument("--task-id", default="")
    parser.add_argument("--detail", required=True)
    parser.add_argument("--role", default="pipeline_coordinator")
    parser.add_argument("--model", default="gpt-5.6-terra")
    parser.add_argument("--effort", default="medium")
    parser.add_argument("--queue", default="rewrite/TRANSLATION_TASKS.tsv")
    parser.add_argument("--fingerprint", default="rewrite/TRANSLATION_TASKS.sha256")
    parser.add_argument("--events", default="rewrite/events.jsonl")
    parser.add_argument("--linux-sha-file", default="vendor/linux.SHA")
    parser.add_argument("--phase0-identity", default="rewrite/PHASE0_IDENTITY.tsv")
    args = parser.parse_args()

    ensure_branch(False)
    queue = Path(args.queue)
    fingerprint = Path(args.fingerprint)
    events = Path(args.events)
    with QueueLock(queue):
        rows = read_tsv(queue)
        validate_rows(rows)
        verify_fingerprint(
            rows,
            fingerprint,
            Path(args.linux_sha_file),
            Path(args.phase0_identity),
        )
        row = task_by_id(rows, args.task_id) if args.task_id else None
        append_event(
            events,
            event_payload(
                row,
                event=args.event,
                role=args.role,
                model=args.model,
                effort=args.effort,
                detail=f"incident={args.incident}; {args.detail}",
                from_status=row["status"] if row else "",
                to_status=row["status"] if row else "",
            ),
        )
    print(json.dumps({"event": args.event, "incident": args.incident, "task_id": args.task_id}))


if __name__ == "__main__":
    main()
