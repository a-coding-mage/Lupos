#!/usr/bin/env python3
"""Render tracking plots for the translation-quality audit in file_verdicts.tsv.

Usage: python3 plot_file_verdicts.py [path/to/file_verdicts.tsv] [outdir]
Writes one PNG per metric into <outdir> (default: translation_plots/).
"""

import csv
import sys
from collections import Counter, defaultdict
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import PathPatch, Rectangle, FancyBboxPatch
from matplotlib.path import Path as MplPath

# ---------------------------------------------------------------- palette ----
SURFACE = "#fcfcfb"
PAGE = "#f9f9f7"
INK = "#0b0b0b"
INK2 = "#52514e"
MUTED = "#898781"
GRID = "#e1e0d9"
BASELINE = "#c3c2b7"
BORDER = (11 / 255, 11 / 255, 11 / 255, 0.10)

BLUE = {100: "#cde2fb", 150: "#b7d3f6", 200: "#9ec5f4", 250: "#86b6ef",
        300: "#6da7ec", 350: "#5598e7", 400: "#3987e5", 450: "#2a78d6",
        500: "#256abf", 550: "#1c5cab", 600: "#184f95", 650: "#104281",
        700: "#0d366b"}
SERIES1 = BLUE[450]

STATUS_GOOD = "#0ca30c"
STATUS_CRITICAL = "#d03b3b"

DPI = 150

plt.rcParams.update({
    "font.family": "sans-serif",
    "font.sans-serif": ["DejaVu Sans"],
    "figure.facecolor": SURFACE,
    "axes.facecolor": SURFACE,
    "savefig.facecolor": SURFACE,
    "text.color": INK,
    "axes.edgecolor": BASELINE,
    "axes.labelcolor": MUTED,
    "xtick.color": MUTED,
    "ytick.color": MUTED,
    "axes.grid": False,
    "svg.fonttype": "none",
})

# ------------------------------------------------------------- vocabulary ----
# Verdict quality scale (dark = more complete) + non-translation buckets.
VERDICT_ORDER = ["FULL", "PARTIAL", "SHELL", "CONSTS", "STUB"]
VERDICT_COLOR = {"FULL": BLUE[650], "PARTIAL": BLUE[550], "SHELL": BLUE[450],
                 "CONSTS": BLUE[350], "STUB": BLUE[250],
                 "INDEX": BASELINE, "NOSRC": MUTED, "DIVERGENT": MUTED}
NON_TRANSLATION = {"INDEX", "NOSRC"}  # coverage % is meaningless for these

PATTERN_LABEL = {
    "P1": "P1 consts only", "P2": "P2 data-shape mirror",
    "P3": "P3 pure-logic extraction", "P4": "P4 faithful translation",
    "P5": "P5 idiomatic win", "P6": "P6 C-ism carryover",
    "P7": "P7 source-text tests", "P8": "P8 errno-stub policy",
    "P9": "P9 divergence risk", "P10": "P10 stale reference",
    "P11": "P11 silent subset", "P12": "P12 unclassified",
}
PATTERN_GROUP = {"P3": "substance", "P4": "substance", "P5": "substance",
                 "P1": "structure", "P2": "structure",
                 "P6": "risk", "P7": "risk", "P8": "risk", "P9": "risk",
                 "P10": "risk", "P11": "risk", "P12": "risk"}
GROUP_COLOR = {"substance": STATUS_GOOD, "structure": MUTED,
               "risk": STATUS_CRITICAL}
GROUP_LABEL = {"substance": "translation substance",
               "structure": "structure only", "risk": "quality risk"}

RISK_EMPTY = {"", "none", "n/a"}


# ------------------------------------------------------------------ marks ----
def px2data(ax, px):
    """Convert a pixel length to data units on both axes."""
    inv = ax.transData.inverted()
    (x0, y0), (x1, y1) = inv.transform([(0, 0), (px, px)])
    return abs(x1 - x0), abs(y1 - y0)


def rounded_hbar(ax, y, w, h, color, x0=0.0, round_end=True, zorder=3):
    """Horizontal bar: 4px-rounded data end, square at the baseline."""
    rx, ry = px2data(ax, 6)
    ry = min(ry, h / 2)
    y0, y1 = y - h / 2, y + h / 2
    if not round_end or w - x0 <= 2 * rx:
        ax.add_patch(Rectangle((x0, y0), w - x0, h, facecolor=color,
                               edgecolor="none", zorder=zorder))
        return
    verts = [(x0, y0), (w - rx, y0), (w, y0), (w, y0 + ry), (w, y1 - ry),
             (w, y1), (w - rx, y1), (x0, y1), (x0, y0)]
    codes = [MplPath.MOVETO, MplPath.LINETO, MplPath.CURVE3, MplPath.CURVE3,
             MplPath.LINETO, MplPath.CURVE3, MplPath.CURVE3, MplPath.LINETO,
             MplPath.CLOSEPOLY]
    ax.add_patch(PathPatch(MplPath(verts, codes), facecolor=color,
                           edgecolor="none", zorder=zorder))


def rounded_vbar(ax, x, h, w, color, zorder=3):
    """Column: 4px-rounded cap, square at the baseline."""
    rx, ry = px2data(ax, 6)
    rx = min(rx, w / 2)
    xl, xr = x - w / 2, x + w / 2
    if h <= 2 * ry:
        ax.add_patch(Rectangle((xl, 0), w, h, facecolor=color,
                               edgecolor="none", zorder=zorder))
        return
    verts = [(xl, 0), (xl, h - ry), (xl, h), (xl + rx, h), (xr - rx, h),
             (xr, h), (xr, h - ry), (xr, 0), (xl, 0)]
    codes = [MplPath.MOVETO, MplPath.LINETO, MplPath.CURVE3, MplPath.CURVE3,
             MplPath.LINETO, MplPath.CURVE3, MplPath.CURVE3, MplPath.LINETO,
             MplPath.CLOSEPOLY]
    ax.add_patch(PathPatch(MplPath(verts, codes), facecolor=color,
                           edgecolor="none", zorder=zorder))


def style_axes(ax, xgrid=False, ygrid=False):
    for side in ("top", "right"):
        ax.spines[side].set_visible(False)
    for side in ("left", "bottom"):
        ax.spines[side].set_color(BASELINE)
        ax.spines[side].set_linewidth(0.8)
    ax.tick_params(length=0, labelsize=9)
    if xgrid:
        ax.xaxis.grid(True, color=GRID, linewidth=0.7, zorder=0)
    if ygrid:
        ax.yaxis.grid(True, color=GRID, linewidth=0.7, zorder=0)


def titles(fig, title, subtitle):
    fig.text(0.06, 0.965, title, fontsize=13, fontweight="bold", color=INK,
             ha="left", va="top")
    fig.text(0.06, 0.925, subtitle, fontsize=9.5, color=INK2, ha="left",
             va="top")


def compact(n):
    if n >= 1_000_000:
        return f"{n / 1_000_000:.1f}M"
    if n >= 10_000:
        return f"{n / 1000:.0f}K"
    return f"{n:,}"


# ------------------------------------------------------------------- data ----
def load(tsv):
    with open(tsv, newline="") as f:
        rows = list(csv.DictReader(f, delimiter="\t"))
    for r in rows:
        r["rust_lines"] = int(r["rust_lines"])
        r["cover_pct"] = int(r["cover_pct"])
        # the audit merges the 3 COMPLETE verdicts into FULL
        if r["verdict"] == "COMPLETE":
            r["verdict"] = "FULL"
        parts = r["rust_file"].split("/")
        r["subsystem"] = parts[1] if len(parts) > 2 else "other"
    return rows


def fold_subsystems(rows, min_files=20):
    counts = Counter(r["subsystem"] for r in rows)
    keep = {s for s, n in counts.items() if n >= min_files}
    for r in rows:
        if r["subsystem"] not in keep:
            r["subsystem"] = "other"


# ---------------------------------------------------------------- figures ----
def fig_kpis(rows, out):
    translated = [r for r in rows if r["verdict"] not in NON_TRANSLATION]
    lines_total = sum(r["rust_lines"] for r in rows)
    wcov = (sum(r["cover_pct"] * r["rust_lines"] for r in translated)
            / sum(r["rust_lines"] for r in translated))
    mcov = sum(r["cover_pct"] for r in translated) / len(translated)
    full = sum(1 for r in rows if r["verdict"] == "FULL")
    over = sum(1 for r in rows if r["claim_check"] == "over")
    risky = sum(1 for r in rows if r["risk"].strip().lower() not in RISK_EMPTY)

    tiles = [
        ("Files audited", f"{len(rows):,}", f"{compact(lines_total)} lines of Rust"),
        ("C coverage, line-weighted", f"{wcov:.0f}%",
         f"unweighted mean {mcov:.0f}% · {len(translated):,} translation files"),
        ("FULL verdicts", f"{full:,}", f"{full / len(rows):.0%} of all files"),
        ("Header overclaims", f"{over:,}", f"{over / len(rows):.0%} claim more than they deliver"),
        ("Files with risk notes", f"{risky:,}", "reviewer-flagged risks"),
        ("Coverage ≥80% / ≤20%",
         f"{sum(1 for r in translated if r['cover_pct'] >= 80):,} / "
         f"{sum(1 for r in translated if r['cover_pct'] <= 20):,}",
         "translation files at each extreme"),
    ]

    fig = plt.figure(figsize=(10.5, 3.6), dpi=DPI)
    fig.set_facecolor(PAGE)
    fig.text(0.045, 0.94, "Linux→Rust translation audit — overview",
             fontsize=13, fontweight="bold", color=INK, va="top")
    cols, rows_n = 3, 2
    for i, (label, value, sub) in enumerate(tiles):
        cx = 0.045 + (i % cols) * 0.31
        cy = 0.47 - (i // cols) * 0.38
        fig.patches.append(FancyBboxPatch(
            (cx, cy), 0.285, 0.33, transform=fig.transFigure,
            boxstyle="round,pad=0.008,rounding_size=0.012",
            facecolor=SURFACE, edgecolor=BORDER, linewidth=1))
        fig.text(cx + 0.015, cy + 0.28, label, fontsize=9.5, color=INK2, va="top")
        fig.text(cx + 0.015, cy + 0.19, value, fontsize=19, fontweight="bold",
                 color=INK, va="top")
        fig.text(cx + 0.015, cy + 0.045, sub, fontsize=8.5, color=MUTED, va="top")
    fig.savefig(out / "01_overview_kpis.png", bbox_inches=None)
    plt.close(fig)
    return wcov


def fig_verdicts(rows, out):
    counts = Counter(r["verdict"] for r in rows)
    order = VERDICT_ORDER + ["DIVERGENT", "INDEX", "NOSRC"]
    labels = {"INDEX": "INDEX (aggregators)", "NOSRC": "NOSRC (missing C source)"}
    cats = [v for v in order if counts.get(v)]
    vals = [counts[v] for v in cats]
    total = len(rows)

    fig, ax = plt.subplots(figsize=(9, 4.4), dpi=DPI)
    fig.subplots_adjust(left=0.24, right=0.95, top=0.82, bottom=0.1)
    ax.set_xlim(0, max(vals) * 1.18)
    ax.set_ylim(-0.6, len(cats) - 0.4)
    style_axes(ax, xgrid=True)
    ax.set_axisbelow(True)
    for i, (v, n) in enumerate(zip(cats, vals)):
        rounded_hbar(ax, i, n, 0.58, VERDICT_COLOR[v])
        ax.text(n + max(vals) * 0.015, i, f"{n:,}  ({n / total:.0%})",
                va="center", fontsize=9, color=INK2)
    ax.set_yticks(range(len(cats)))
    ax.set_yticklabels([labels.get(v, v) for v in cats], fontsize=9.5, color=INK)
    ax.invert_yaxis()
    ax.xaxis.set_major_formatter(lambda x, _: f"{x:,.0f}")
    titles(fig, "Verdicts across the tree",
           f"How much of each C counterpart is really implemented · {total:,} files · darker = more complete")
    fig.savefig(out / "02_verdict_distribution.png")
    plt.close(fig)


def fig_coverage_hist(rows, out):
    translated = [r for r in rows if r["verdict"] not in NON_TRANSLATION]
    covs = [r["cover_pct"] for r in translated]
    bins = list(range(0, 101, 10))
    counts = [0] * 10
    for c in covs:
        counts[min(c // 10, 9)] += 1
    mean = sum(covs) / len(covs)

    fig, ax = plt.subplots(figsize=(9, 4.2), dpi=DPI)
    fig.subplots_adjust(left=0.09, right=0.95, top=0.8, bottom=0.14)
    ax.set_xlim(-2, 102)
    ax.set_ylim(0, max(counts) * 1.18)
    style_axes(ax, ygrid=True)
    ax.set_axisbelow(True)
    for i, n in enumerate(counts):
        rounded_vbar(ax, bins[i] + 5, n, 6.2, SERIES1)
    ax.axvline(mean, color=INK2, linewidth=1, zorder=4)
    ax.text(mean + 1.5, max(counts) * 1.08, f"mean {mean:.0f}%",
            fontsize=8.5, color=INK2)
    ax.set_xticks(bins)
    ax.set_xticklabels([f"{b}%" for b in bins])
    ax.set_ylabel("files", fontsize=9)
    titles(fig, "Estimated coverage of the C counterpart",
           f"% of C functions with real Rust logic · {len(covs):,} translation files (INDEX/NOSRC excluded)")
    fig.savefig(out / "03_coverage_distribution.png")
    plt.close(fig)


def fig_coverage_by_subsystem(rows, out):
    translated = [r for r in rows if r["verdict"] not in NON_TRANSLATION]
    lines = defaultdict(int)
    weighted = defaultdict(int)
    for r in translated:
        lines[r["subsystem"]] += r["rust_lines"]
        weighted[r["subsystem"]] += r["cover_pct"] * r["rust_lines"]
    subs = sorted(lines, key=lambda s: weighted[s] / lines[s], reverse=True)
    vals = [weighted[s] / lines[s] for s in subs]

    fig, ax = plt.subplots(figsize=(9, 0.42 * len(subs) + 1.9), dpi=DPI)
    fig.subplots_adjust(left=0.2, right=0.95, top=1 - 1.05 / fig.get_figheight(),
                        bottom=0.45 / fig.get_figheight())
    ax.set_xlim(0, 100)
    ax.set_ylim(-0.6, len(subs) - 0.4)
    style_axes(ax, xgrid=True)
    ax.set_axisbelow(True)
    for i, (s, v) in enumerate(zip(subs, vals)):
        rounded_hbar(ax, i, v, 0.58, SERIES1)
        ax.text(v + 1.2, i, f"{v:.0f}%", va="center", fontsize=9, color=INK2)
    ax.set_yticks(range(len(subs)))
    ax.set_yticklabels(subs, fontsize=9.5, color=INK)
    ax.invert_yaxis()
    ax.set_xticks(range(0, 101, 20))
    ax.set_xticklabels([f"{v}%" for v in range(0, 101, 20)])
    titles(fig, "Coverage by subsystem, line-weighted",
           "Rust-line-weighted % of C functions implemented · translation files only")
    fig.savefig(out / "04_coverage_by_subsystem.png")
    plt.close(fig)


def fig_verdict_mix(rows, out):
    segs = VERDICT_ORDER + ["other"]
    seg_color = dict(VERDICT_COLOR, other=BASELINE)
    by_sub = defaultdict(Counter)
    for r in rows:
        v = r["verdict"] if r["verdict"] in VERDICT_ORDER else "other"
        by_sub[r["subsystem"]][v] += 1
    subs = sorted(by_sub, key=lambda s: by_sub[s]["FULL"] / sum(by_sub[s].values()),
                  reverse=True)

    fig, ax = plt.subplots(figsize=(9.5, 0.42 * len(subs) + 2.3), dpi=DPI)
    fig.subplots_adjust(left=0.2, right=0.95, top=1 - 1.0 / fig.get_figheight(),
                        bottom=0.85 / fig.get_figheight())
    ax.set_xlim(0, 100)
    ax.set_ylim(-0.6, len(subs) - 0.4)
    style_axes(ax)
    for i, s in enumerate(subs):
        total = sum(by_sub[s].values())
        x = 0.0
        for seg in segs:
            share = 100 * by_sub[s][seg] / total
            if share == 0:
                continue
            ax.add_patch(Rectangle((x, i - 0.29), share, 0.58,
                                   facecolor=seg_color[seg], edgecolor=SURFACE,
                                   linewidth=1.4, zorder=3))
            if seg == "FULL" and share >= 9:
                ax.text(x + share / 2, i, f"{share:.0f}%", ha="center",
                        va="center", fontsize=8, color="white", zorder=4)
            x += share
    ax.set_yticks(range(len(subs)))
    ax.set_yticklabels(subs, fontsize=9.5, color=INK)
    ax.invert_yaxis()
    ax.set_xticks(range(0, 101, 20))
    ax.set_xticklabels([f"{v}%" for v in range(0, 101, 20)])
    handles = [Rectangle((0, 0), 1, 1, facecolor=seg_color[s]) for s in segs]
    names = segs[:-1] + ["INDEX/NOSRC/other"]
    ax.legend(handles, names, loc="upper center", bbox_to_anchor=(0.5, -0.1),
              ncol=len(segs), frameon=False, fontsize=8.5,
              handlelength=1.1, handleheight=1.1, labelcolor=INK2)
    titles(fig, "Verdict mix by subsystem",
           "Share of files per verdict · sorted by FULL share · darker = more complete")
    fig.savefig(out / "05_verdict_mix_by_subsystem.png")
    plt.close(fig)


def fig_claim_honesty(rows, out):
    segs = [("ok", STATUS_GOOD, "header ok"),
            ("under", MUTED, "underclaim"),
            ("unknown", BASELINE, "unknown"),
            ("over", STATUS_CRITICAL, "overclaim")]
    by_sub = defaultdict(Counter)
    for r in rows:
        c = r["claim_check"] if r["claim_check"] in {"ok", "under", "over"} else "unknown"
        by_sub[r["subsystem"]][c] += 1
    subs = sorted(by_sub, key=lambda s: by_sub[s]["over"] / sum(by_sub[s].values()),
                  reverse=True)

    fig, ax = plt.subplots(figsize=(9.5, 0.42 * len(subs) + 2.3), dpi=DPI)
    fig.subplots_adjust(left=0.2, right=0.95, top=1 - 1.0 / fig.get_figheight(),
                        bottom=0.85 / fig.get_figheight())
    ax.set_xlim(0, 100)
    ax.set_ylim(-0.6, len(subs) - 0.4)
    style_axes(ax)
    for i, s in enumerate(subs):
        total = sum(by_sub[s].values())
        x = 0.0
        for key, color, _ in segs:
            share = 100 * by_sub[s][key] / total
            if share == 0:
                continue
            ax.add_patch(Rectangle((x, i - 0.29), share, 0.58, facecolor=color,
                                   edgecolor=SURFACE, linewidth=1.4, zorder=3))
            if key in {"ok", "over"} and share >= 9:
                ax.text(x + share / 2, i, f"{share:.0f}%", ha="center",
                        va="center", fontsize=8, color="white", zorder=4)
            x += share
    ax.set_yticks(range(len(subs)))
    ax.set_yticklabels(subs, fontsize=9.5, color=INK)
    ax.invert_yaxis()
    ax.set_xticks(range(0, 101, 20))
    ax.set_xticklabels([f"{v}%" for v in range(0, 101, 20)])
    handles = [Rectangle((0, 0), 1, 1, facecolor=c) for _, c, _ in segs]
    ax.legend(handles, [lbl for _, _, lbl in segs], loc="upper center",
              bbox_to_anchor=(0.5, -0.1), ncol=4, frameon=False, fontsize=8.5,
              handlelength=1.1, handleheight=1.1, labelcolor=INK2)
    titles(fig, "Header honesty by subsystem",
           "Does the linux-parity header match the verdict? · sorted by overclaim share")
    fig.savefig(out / "06_claim_honesty_by_subsystem.png")
    plt.close(fig)


def fig_claim_vs_verdict(rows, out):
    claims = ["complete", "partial", "stub", "NONE"]
    verdicts = ["FULL", "PARTIAL", "SHELL", "CONSTS", "STUB", "INDEX",
                "NOSRC", "DIVERGENT"]
    grid = {(c, v): 0 for c in claims for v in verdicts}
    for r in rows:
        key = (r["parity_claim"], r["verdict"])
        if key in grid:
            grid[key] += 1
    vmax = max(grid.values())
    ramp = [BLUE[k] for k in (100, 150, 200, 250, 300, 350, 400, 450, 500,
                              550, 600, 650, 700)]

    fig, ax = plt.subplots(figsize=(9.5, 4.2), dpi=DPI)
    fig.subplots_adjust(left=0.13, right=0.96, top=0.78, bottom=0.16)
    ax.set_xlim(0, len(verdicts))
    ax.set_ylim(0, len(claims))
    ax.set_frame_on(False)
    ax.tick_params(length=0, labelsize=9)
    overclaim_zone = {("complete", v) for v in ("SHELL", "CONSTS", "STUB", "NOSRC")}
    for yi, c in enumerate(claims):
        for xi, v in enumerate(verdicts):
            n = grid[(c, v)]
            frac = (n / vmax) ** 0.5
            color = SURFACE if n == 0 else ramp[min(int(frac * len(ramp)), len(ramp) - 1)]
            ax.add_patch(Rectangle((xi, len(claims) - 1 - yi), 1, 1,
                                   facecolor=color, edgecolor=SURFACE,
                                   linewidth=1.4, zorder=2))
            if n:
                ax.text(xi + 0.5, len(claims) - 1 - yi + 0.5, f"{n:,}",
                        ha="center", va="center", fontsize=8.5,
                        color="white" if frac > 0.55 else INK, zorder=4)
            if (c, v) in overclaim_zone:
                ax.add_patch(Rectangle((xi + 0.06, len(claims) - 1 - yi + 0.06),
                                       0.88, 0.88, facecolor="none",
                                       edgecolor=STATUS_CRITICAL, linewidth=1.4,
                                       zorder=5))
    ax.set_xticks([i + 0.5 for i in range(len(verdicts))])
    ax.set_xticklabels(verdicts, fontsize=8.5, color=INK)
    ax.set_yticks([i + 0.5 for i in range(len(claims))])
    ax.set_yticklabels(reversed(claims), fontsize=9, color=INK)
    ax.set_ylabel("header claims", fontsize=9)
    ax.set_xlabel("audit verdict", fontsize=9)
    titles(fig, "What headers claim vs what the audit found",
           "File counts · red outline = claims “complete” but the file is a shell, consts, stub, or its C source is missing")
    fig.savefig(out / "07_claim_vs_verdict.png")
    plt.close(fig)


def fig_patterns(rows, out):
    counts = Counter()
    for r in rows:
        for p in r["patterns"].split(","):
            p = p.strip()
            if p in PATTERN_LABEL:
                counts[p] += 1
    order = sorted(counts, key=lambda p: ({"substance": 0, "structure": 1,
                                           "risk": 2}[PATTERN_GROUP[p]],
                                          -counts[p]))
    vals = [counts[p] for p in order]

    fig, ax = plt.subplots(figsize=(9.5, 0.36 * len(order) + 2.3), dpi=DPI)
    fig.subplots_adjust(left=0.26, right=0.95, top=1 - 1.0 / fig.get_figheight(),
                        bottom=0.85 / fig.get_figheight())
    ax.set_xlim(0, max(vals) * 1.14)
    ax.set_ylim(-0.6, len(order) - 0.4)
    style_axes(ax, xgrid=True)
    ax.set_axisbelow(True)
    for i, p in enumerate(order):
        rounded_hbar(ax, i, counts[p], 0.58, GROUP_COLOR[PATTERN_GROUP[p]])
        ax.text(counts[p] + max(vals) * 0.012, i, f"{counts[p]:,}",
                va="center", fontsize=8.5, color=INK2)
    ax.set_yticks(range(len(order)))
    ax.set_yticklabels([PATTERN_LABEL[p] for p in order], fontsize=9, color=INK)
    ax.invert_yaxis()
    ax.xaxis.set_major_formatter(lambda x, _: f"{x:,.0f}")
    handles = [Rectangle((0, 0), 1, 1, facecolor=GROUP_COLOR[g])
               for g in ("substance", "structure", "risk")]
    ax.legend(handles, [GROUP_LABEL[g] for g in ("substance", "structure", "risk")],
              loc="upper center", bbox_to_anchor=(0.5, -0.12), ncol=3,
              frameon=False, fontsize=8.5, handlelength=1.1, handleheight=1.1,
              labelcolor=INK2)
    titles(fig, "Translation patterns",
           "Tags per file (a file can carry several) · P7 = tests that only grep vendored C source")
    fig.savefig(out / "08_pattern_frequency.png")
    plt.close(fig)


def fig_lines_by_coverage(rows, out):
    translated = [r for r in rows if r["verdict"] not in NON_TRANSLATION]
    buckets = [(0, 20), (21, 40), (41, 60), (61, 80), (81, 100)]
    ramp = [BLUE[250], BLUE[350], BLUE[450], BLUE[550], BLUE[650]]
    totals = [0] * 5
    for r in translated:
        for i, (lo, hi) in enumerate(buckets):
            if lo <= r["cover_pct"] <= hi:
                totals[i] += r["rust_lines"]
                break

    grand = sum(totals)

    fig, ax = plt.subplots(figsize=(8.5, 4.6), dpi=DPI)
    fig.subplots_adjust(left=0.1, right=0.95, top=0.78, bottom=0.2)
    ax.set_xlim(-0.6, 4.6)
    ax.set_ylim(0, max(totals) * 1.34)
    style_axes(ax, ygrid=True)
    ax.set_axisbelow(True)
    for i, t in enumerate(totals):
        rounded_vbar(ax, i, t, 0.42, ramp[i])
        ax.text(i, t + max(totals) * 0.03,
                f"{compact(t)} · {t / grand:.0%}", ha="center",
                fontsize=9, color=INK2)

    # bracket over the two low-coverage buckets: the remaining translation mass
    low = totals[0] + totals[1]
    y_br = max(totals[0], totals[1]) + max(totals) * 0.13
    tick = max(totals) * 0.025
    ax.plot([-0.25, -0.25, 1.25, 1.25], [y_br - tick, y_br, y_br, y_br - tick],
            color=MUTED, linewidth=1, zorder=4)
    ax.text(0.5, y_br + tick * 1.2,
            f"{compact(low)} lines still ≤40% covered\n"
            f"({low / grand:.0%} of the translated tree)",
            ha="center", va="bottom", fontsize=8.5, color=INK2)

    ax.set_xticks(range(5))
    ax.set_xticklabels(["0–20%", "21–40%", "41–60%", "61–80%", "81–100%"],
                       fontsize=9, color=INK)
    from matplotlib.transforms import blended_transform_factory
    hint_tf = blended_transform_factory(ax.transData, ax.transAxes)
    ax.text(0, -0.115, "← barely translated", transform=hint_tf,
            ha="center", fontsize=8, color=MUTED)
    ax.text(4, -0.115, "fully translated →", transform=hint_tf,
            ha="center", fontsize=8, color=MUTED)
    ax.set_xlabel("share of the C counterpart's functions implemented in Rust",
                  fontsize=9, labelpad=18)
    ax.set_ylabel("lines of Rust", fontsize=9)
    ax.yaxis.set_major_formatter(lambda y, _: compact(int(y)))
    titles(fig, "Where the Rust code mass sits",
           "Each bar = total Rust lines in files at that coverage level · label = lines · share of translated-file lines\n"
           "translation files only (INDEX aggregators and NOSRC excluded)")
    fig.savefig(out / "09_lines_by_coverage.png")
    plt.close(fig)


# ------------------------------------------------------------------- main ----
def main():
    tsv = sys.argv[1] if len(sys.argv) > 1 else "file_verdicts.tsv"
    out = Path(sys.argv[2] if len(sys.argv) > 2 else "translation_plots")
    out.mkdir(parents=True, exist_ok=True)
    rows = load(tsv)
    fold_subsystems(rows)

    wcov = fig_kpis(rows, out)
    fig_verdicts(rows, out)
    fig_coverage_hist(rows, out)
    fig_coverage_by_subsystem(rows, out)
    fig_verdict_mix(rows, out)
    fig_claim_honesty(rows, out)
    fig_claim_vs_verdict(rows, out)
    fig_patterns(rows, out)
    fig_lines_by_coverage(rows, out)

    print(f"{len(rows)} rows → {out}/ · line-weighted coverage {wcov:.1f}%")
    for p in sorted(out.glob("*.png")):
        print(" ", p)


if __name__ == "__main__":
    main()
