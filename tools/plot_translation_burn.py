#!/usr/bin/env python3
"""Generate timestamp-based Lupos translation progress charts.

Outputs are derived from TRANSLATION_TASKS.tsv and the append-only events log.
No completion claim is inferred from line count, commits, or source headers.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
from collections import Counter, defaultdict
import statistics
from pathlib import Path
from zoneinfo import ZoneInfo


def parse_ts(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def floor_hour(value: dt.datetime) -> dt.datetime:
    return value.replace(minute=0, second=0, microsecond=0)


def read_queue(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        required = {"id", "path", "created_at", "done_at", "status", "weight"}
        if reader.fieldnames is None or not required.issubset(reader.fieldnames):
            missing = sorted(required - set(reader.fieldnames or []))
            raise SystemExit(f"queue is missing required columns: {', '.join(missing)}")
        return [dict(row) for row in reader]


def read_events(path: Path) -> list[dict[str, object]]:
    if not path.exists():
        return []
    events: list[dict[str, object]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError as exc:
                raise SystemExit(f"invalid JSON at {path}:{line_number}: {exc}") from exc
            if isinstance(event, dict):
                events.append(event)
    return events


def load_matplotlib():
    try:
        import matplotlib.dates as mdates
        import matplotlib.pyplot as plt
    except ImportError as exc:
        raise SystemExit(
            "matplotlib is required for PNG plots; install it or run inside the "
            "repository environment that provides plot_file_verdicts.py dependencies"
        ) from exc
    return plt, mdates


def write_metrics(
    path: Path,
    hours: list[dt.datetime],
    done_per_hour: Counter[dt.datetime],
    weight_per_hour: dict[dt.datetime, float],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    cumulative_files = 0
    cumulative_weight = 0.0
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(
            [
                "hour",
                "files_done_in_hour",
                "cumulative_files_done",
                "weight_done_in_hour",
                "cumulative_weight_done",
            ]
        )
        for hour in hours:
            count = done_per_hour[hour]
            weight = weight_per_hour.get(hour, 0.0)
            cumulative_files += count
            cumulative_weight += weight
            writer.writerow(
                [
                    hour.isoformat(),
                    count,
                    cumulative_files,
                    f"{weight:.3f}",
                    f"{cumulative_weight:.3f}",
                ]
            )


def chart_cumulative_files(out_path: Path, times: list[dt.datetime], total: int, tz: ZoneInfo) -> None:
    plt, mdates = load_matplotlib()
    figure = plt.figure(figsize=(12, 6))
    axis = figure.add_subplot(111)
    x = [value.astimezone(tz) for value in times]
    y = list(range(1, len(x) + 1))
    if x:
        axis.step(x, y, where="post")
    axis.set_title("Lupos translation tasks completed over time")
    axis.set_xlabel(f"Time ({tz.key})")
    axis.set_ylabel(f"Files DONE (of {total})")
    axis.set_ylim(bottom=0, top=max(total, 1))
    axis.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m-%d\n%H:%M", tz=tz))
    axis.grid(True, alpha=0.25)
    figure.tight_layout()
    figure.savefig(out_path, dpi=160)
    plt.close(figure)


def chart_hourly_throughput(
    out_path: Path, hours: list[dt.datetime], done_per_hour: Counter[dt.datetime], tz: ZoneInfo
) -> None:
    plt, mdates = load_matplotlib()
    figure = plt.figure(figsize=(12, 6))
    axis = figure.add_subplot(111)
    x = [value.astimezone(tz) for value in hours]
    y = [done_per_hour[value] for value in hours]
    if x:
        axis.bar(x, y, width=0.035)
    axis.set_title("Lupos translation tasks completed per hour")
    axis.set_xlabel(f"Hour ({tz.key})")
    axis.set_ylabel("Files marked DONE")
    axis.xaxis.set_major_formatter(mdates.DateFormatter("%m-%d\n%H:%M", tz=tz))
    axis.grid(True, axis="y", alpha=0.25)
    figure.tight_layout()
    figure.savefig(out_path, dpi=160)
    plt.close(figure)


def chart_cumulative_weight(
    out_path: Path,
    weighted_done: list[tuple[dt.datetime, float]],
    total_weight: float,
    tz: ZoneInfo,
) -> None:
    plt, mdates = load_matplotlib()
    figure = plt.figure(figsize=(12, 6))
    axis = figure.add_subplot(111)
    cumulative = 0.0
    x: list[dt.datetime] = []
    y: list[float] = []
    for timestamp, weight in weighted_done:
        cumulative += weight
        x.append(timestamp.astimezone(tz))
        y.append(cumulative)
    if x:
        axis.step(x, y, where="post")
    axis.set_title("Lupos scheduled translation weight completed over time")
    axis.set_xlabel(f"Time ({tz.key})")
    axis.set_ylabel(f"Cumulative weight DONE (of {total_weight:.1f})")
    axis.set_ylim(bottom=0, top=max(total_weight, 1.0))
    axis.xaxis.set_major_formatter(mdates.DateFormatter("%Y-%m-%d\n%H:%M", tz=tz))
    axis.grid(True, alpha=0.25)
    figure.tight_layout()
    figure.savefig(out_path, dpi=160)
    plt.close(figure)


def write_model_metrics(path: Path, events: list[dict[str, object]]) -> None:
    implementations = Counter()
    reviews = Counter()
    completions = Counter()
    for event in events:
        model = str(event.get("model", "") or "unknown")
        name = str(event.get("event", ""))
        if name == "implementation_done":
            implementations[model] += 1
        elif name in {"review_1_done", "review_2_done"}:
            reviews[model] += 1
        elif name == "done":
            completions[model] += 1
    models = sorted(set(implementations) | set(reviews) | set(completions))
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(["model", "implementation_events", "review_events", "done_events"])
        for model in models:
            writer.writerow([model, implementations[model], reviews[model], completions[model]])


def elapsed_seconds(start: str, end: str) -> float | None:
    if not start or not end:
        return None
    return max(0.0, (parse_ts(end) - parse_ts(start)).total_seconds())


def parsed_event_time(event: dict[str, object]) -> dt.datetime | None:
    value = str(event.get("ts", "") or "")
    if not value:
        return None
    try:
        return parse_ts(value)
    except ValueError:
        return None


def event_iso(event: dict[str, object] | None) -> str:
    return str(event.get("ts", "") or "") if event else ""


def first_event(events: list[dict[str, object]], name: str) -> dict[str, object] | None:
    return next((event for event in events if str(event.get("event", "")) == name), None)


def last_event(events: list[dict[str, object]], name: str) -> dict[str, object] | None:
    return next(
        (event for event in reversed(events) if str(event.get("event", "")) == name),
        None,
    )


def write_task_durations(
    task_path: Path, model_path: Path, rows: list[dict[str, str]], events: list[dict[str, object]]
) -> None:
    """Write event-derived stage timing, including retried attempts.

    The TSV row preserves the task's original work_started_at, while the event
    log records each claim/requeue attempt. Durations therefore come primarily
    from events so a retried task is not incorrectly charged from its first-ever
    claim. Current-row timestamps are used only as a recovery fallback when a
    stage event is missing.
    """

    rows_by_id = {row["id"]: row for row in rows}
    grouped: dict[tuple[str, int], list[dict[str, object]]] = defaultdict(list)
    for event in events:
        task_id = str(event.get("task_id", "") or "")
        if not task_id or task_id not in rows_by_id:
            continue
        try:
            attempt = int(event.get("attempt", 0) or 0)
        except (TypeError, ValueError):
            continue
        if attempt <= 0:
            continue
        grouped[(task_id, attempt)].append(event)

    for task_events in grouped.values():
        task_events.sort(
            key=lambda event: parsed_event_time(event)
            or dt.datetime.min.replace(tzinfo=dt.timezone.utc)
        )

    records: list[dict[str, object]] = []
    for row in rows:
        task_keys = sorted(key for key in grouped if key[0] == row["id"])
        if not task_keys:
            task_keys = [(row["id"], int(row.get("attempt", "0") or 0))]

        for task_id, attempt in task_keys:
            task_events = grouped.get((task_id, attempt), [])
            current_attempt = attempt == int(row.get("attempt", "0") or 0)

            claimed = first_event(task_events, "claimed")
            implemented = last_event(task_events, "implementation_done")
            review_started = first_event(task_events, "review_started")
            review_events = [
                event
                for event in task_events
                if str(event.get("event", "")) in {"review_1_done", "review_2_done"}
            ]
            reviews_done = review_events[-1] if review_events else None
            apply_started = first_event(task_events, "apply_started")
            done = last_event(task_events, "done")
            blocked = last_event(task_events, "blocked")
            requeued = last_event(task_events, "requeued")

            terminal_events = [
                event
                for event in (done, blocked, requeued)
                if event is not None and parsed_event_time(event) is not None
            ]
            terminal = max(terminal_events, key=parsed_event_time) if terminal_events else None
            terminal_name = str(terminal.get("event", "")) if terminal else ""
            outcome = {
                "done": "DONE",
                "blocked": "BLOCKED",
                "requeued": "REQUEUED",
            }.get(terminal_name, "")
            if not outcome:
                if task_events:
                    latest = task_events[-1]
                    outcome = str(latest.get("to_status", "") or latest.get("event", "")).upper()
                elif current_attempt:
                    outcome = row.get("status", "")
                else:
                    outcome = "UNKNOWN"

            model_event = implemented
            if model_event is None:
                model_event = next(
                    (
                        event
                        for event in reversed(task_events)
                        if str(event.get("role", "")) == "implementer"
                        and str(event.get("model", "") or "") not in {"", "none"}
                    ),
                    None,
                )
            implementer_model = (
                str(model_event.get("model", "") or "unknown")
                if model_event
                else "unknown"
            )

            claimed_at = event_iso(claimed)
            implement_done_at = event_iso(implemented)
            review_started_at = event_iso(review_started)
            reviews_done_at = event_iso(reviews_done)
            apply_started_at = event_iso(apply_started)
            terminal_at = event_iso(terminal)
            done_at = event_iso(done)

            # Fallback only for the current attempt, covering an interrupted
            # queue-write/event-append boundary without rewriting history.
            if current_attempt:
                claimed_at = claimed_at or row.get("work_started_at", "")
                implement_done_at = implement_done_at or row.get("implement_done_at", "")
                review_started_at = review_started_at or row.get("review_started_at", "")
                if not reviews_done_at:
                    review_values = [
                        value
                        for value in (
                            row.get("review_1_done_at", ""),
                            row.get("review_2_done_at", ""),
                        )
                        if value
                    ]
                    reviews_done_at = (
                        max(review_values, key=parse_ts) if review_values else ""
                    )
                apply_started_at = apply_started_at or row.get("apply_started_at", "")
                if row.get("status") == "DONE":
                    done_at = done_at or row.get("done_at", "")
                    terminal_at = terminal_at or done_at

            apply_end = done_at or (
                terminal_at if terminal_name in {"blocked", "requeued"} else ""
            )
            total_end = terminal_at
            record: dict[str, object] = {
                "id": row["id"],
                "path": row["path"],
                "queue_status": row["status"],
                "attempt": attempt,
                "outcome": outcome,
                "weight": float(row.get("weight", "0") or 0),
                "risk": row.get("risk", ""),
                "implementer_model": implementer_model,
                "pipeline_id": str(claimed.get("pipeline_id", "") or "") if claimed else (
                    row.get("pipeline_id", "") if current_attempt else ""
                ),
                "claimed_at": claimed_at,
                "implement_done_at": implement_done_at,
                "review_started_at": review_started_at,
                "reviews_done_at": reviews_done_at,
                "apply_started_at": apply_started_at,
                "terminal_at": terminal_at,
                "done_at": done_at,
                "implementation_seconds": elapsed_seconds(claimed_at, implement_done_at),
                "review_seconds": elapsed_seconds(review_started_at, reviews_done_at),
                "apply_seconds": elapsed_seconds(apply_started_at, apply_end),
                "total_seconds": elapsed_seconds(claimed_at, total_end),
                "review_reports": len(review_events),
                "blocked_events": sum(
                    str(event.get("event", "")) == "blocked" for event in task_events
                ),
                "pause_events": sum(
                    str(event.get("event", "")) == "paused" for event in task_events
                ),
                "resume_events": sum(
                    str(event.get("event", "")) == "resumed" for event in task_events
                ),
            }
            records.append(record)

    fields = [
        "id",
        "path",
        "queue_status",
        "attempt",
        "outcome",
        "weight",
        "risk",
        "implementer_model",
        "pipeline_id",
        "claimed_at",
        "implement_done_at",
        "review_started_at",
        "reviews_done_at",
        "apply_started_at",
        "terminal_at",
        "done_at",
        "implementation_seconds",
        "review_seconds",
        "apply_seconds",
        "total_seconds",
        "review_reports",
        "blocked_events",
        "pause_events",
        "resume_events",
    ]
    task_path.parent.mkdir(parents=True, exist_ok=True)
    with task_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for record in records:
            writer.writerow(
                {
                    key: (
                        f"{value:.3f}"
                        if isinstance(value, float)
                        else ""
                        if value is None
                        else value
                    )
                    for key, value in record.items()
                }
            )

    by_model: dict[str, list[dict[str, object]]] = defaultdict(list)
    for record in records:
        model = str(record["implementer_model"])
        if model != "unknown":
            by_model[model].append(record)

    with model_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(
            [
                "implementer_model",
                "attempts_attributed",
                "implemented_attempts",
                "done_tasks",
                "done_weight",
                "blocked_events",
                "requeued_attempts",
                "pause_events",
                "review_reports",
                "median_implementation_seconds",
                "mean_implementation_seconds",
                "median_total_pipeline_seconds",
            ]
        )
        for model in sorted(by_model):
            model_records = by_model[model]
            implementation_values = [
                float(record["implementation_seconds"])
                for record in model_records
                if record["implementation_seconds"] is not None
            ]
            done_records = [record for record in model_records if record["outcome"] == "DONE"]
            total_values = [
                float(record["total_seconds"])
                for record in done_records
                if record["total_seconds"] is not None
            ]
            writer.writerow(
                [
                    model,
                    len(model_records),
                    len(implementation_values),
                    len(done_records),
                    f"{sum(float(record['weight']) for record in done_records):.3f}",
                    sum(int(record["blocked_events"]) for record in model_records),
                    sum(record["outcome"] == "REQUEUED" for record in model_records),
                    sum(int(record["pause_events"]) for record in model_records),
                    sum(int(record["review_reports"]) for record in model_records),
                    f"{statistics.median(implementation_values):.3f}"
                    if implementation_values
                    else "",
                    f"{statistics.fmean(implementation_values):.3f}"
                    if implementation_values
                    else "",
                    f"{statistics.median(total_values):.3f}" if total_values else "",
                ]
            )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--queue", default="rewrite/TRANSLATION_TASKS.tsv")
    parser.add_argument("--events", default="rewrite/events.jsonl")
    parser.add_argument("--out-dir", default="rewrite/plots")
    parser.add_argument("--timezone", default="Asia/Tokyo")
    args = parser.parse_args()

    queue_path = Path(args.queue)
    events_path = Path(args.events)
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    tz = ZoneInfo(args.timezone)

    rows = read_queue(queue_path)
    events = read_events(events_path)
    total = len(rows)
    total_weight = sum(float(row.get("weight", "0") or 0) for row in rows)

    done_rows: list[tuple[dt.datetime, float, str]] = []
    for row in rows:
        if row["status"] != "DONE" or not row["done_at"]:
            continue
        done_rows.append((parse_ts(row["done_at"]), float(row["weight"] or 0), row["id"]))
    done_rows.sort(key=lambda item: (item[0], item[2]))

    done_times = [item[0] for item in done_rows]
    weighted_done = [(item[0], item[1]) for item in done_rows]
    done_per_hour: Counter[dt.datetime] = Counter(floor_hour(item[0]) for item in done_rows)
    weight_per_hour: dict[dt.datetime, float] = defaultdict(float)
    for timestamp, weight, _ in done_rows:
        weight_per_hour[floor_hour(timestamp)] += weight

    if done_rows:
        first_created_values = [parse_ts(row["created_at"]) for row in rows if row.get("created_at")]
        start = floor_hour(min(first_created_values or done_times))
        end = floor_hour(max(done_times))
        hours: list[dt.datetime] = []
        current = start
        while current <= end:
            hours.append(current)
            current += dt.timedelta(hours=1)
    else:
        hours = []

    chart_cumulative_files(out_dir / "01_tasks_done_over_time.png", done_times, total, tz)
    chart_hourly_throughput(out_dir / "02_tasks_done_per_hour.png", hours, done_per_hour, tz)
    chart_cumulative_weight(out_dir / "03_weight_done_over_time.png", weighted_done, total_weight, tz)
    write_metrics(out_dir / "translation_hourly_metrics.tsv", hours, done_per_hour, weight_per_hour)
    write_model_metrics(out_dir / "translation_model_events.tsv", events)
    write_task_durations(
        out_dir / "translation_task_durations.tsv",
        out_dir / "translation_model_performance.tsv",
        rows,
        events,
    )

    summary = {
        "tasks": total,
        "done": len(done_rows),
        "done_percent": round((len(done_rows) / total * 100) if total else 0, 2),
        "total_weight": round(total_weight, 3),
        "done_weight": round(sum(item[1] for item in done_rows), 3),
        "timezone": tz.key,
        "out_dir": str(out_dir),
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
