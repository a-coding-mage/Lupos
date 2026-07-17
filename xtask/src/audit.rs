//! test-origin: lupos-specific:xtask audit tooling and report regression tests
//! Linux layout and parity audits.
//!
//! Uses the checked-in layout TSVs when present. Repositories that have
//! migrated those generated files away derive the same mapping from each Rust
//! file's `linux-source` header and classify untagged files as Rust-only.
//!
//! Drift triggers a non-zero exit, mirroring the no-regression policy in
//! `CLAUDE.md` (rule 4).  Output report at `target/xtask/audit-layout.tsv`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};

use crate::{repo_root, xtask_target_dir};

pub(crate) const LAYOUT_MAP_PATH: &str = "src/docs/linux-layout-map.tsv";
pub(crate) const LAYOUT_EXCEPTIONS_PATH: &str = "src/docs/linux-layout-exceptions.tsv";
pub(crate) const AUDIT_LAYOUT_REPORT_PATH: &str = "target/xtask/audit-layout.tsv";

const LAYOUT_MAP_HEADER: &str = "lupos_path\tlinux_path\tnote";
const LAYOUT_EXCEPTIONS_HEADER: &str = "lupos_path\treason";

#[derive(Clone, Debug)]
pub(crate) struct LayoutMapRow {
    pub lupos_path: String,
    pub linux_path: String,
    pub note: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LayoutExceptionRow {
    pub lupos_path: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum AuditFinding {
    /// `lupos_path` listed in the map does not exist on disk.
    MissingLuposFile,
    /// `linux_path` listed in the map does not exist under `vendor/linux/`.
    MissingLinuxFile,
    /// `.rs` file exists under `src/` but is in neither the map nor the
    /// exceptions list.
    OrphanRsFile,
    /// Same path appears in both the map and the exceptions list.
    DuplicateMapping,
    /// `lupos_path` listed in the exceptions table does not exist on disk.
    MissingException,
}

impl AuditFinding {
    fn as_str(self) -> &'static str {
        match self {
            Self::MissingLuposFile => "missing_lupos_file",
            Self::MissingLinuxFile => "missing_linux_file",
            Self::OrphanRsFile => "orphan_rs_file",
            Self::DuplicateMapping => "duplicate_mapping",
            Self::MissingException => "missing_exception",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AuditEntry {
    pub finding: AuditFinding,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AuditReport {
    pub entries: Vec<AuditEntry>,
    pub map_rows: usize,
    pub exception_rows: usize,
    pub rs_files_scanned: usize,
}

impl AuditReport {
    pub(crate) fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out = String::from("finding\tpath\tdetail\n");
        for entry in &self.entries {
            out.push_str(entry.finding.as_str());
            out.push('\t');
            out.push_str(&entry.path);
            out.push('\t');
            out.push_str(&entry.detail);
            out.push('\n');
        }
        out
    }
}

pub(crate) fn parse_layout_map(text: &str) -> Result<Vec<LayoutMapRow>> {
    let mut lines = text.lines();
    let header = lines.next().ok_or_else(|| anyhow!("empty layout map"))?;
    if header != LAYOUT_MAP_HEADER {
        bail!(
            "invalid layout map header: expected `{}`, got `{}`",
            LAYOUT_MAP_HEADER,
            header
        );
    }
    let mut rows = Vec::new();
    for (idx, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 3 {
            bail!(
                "invalid layout map row {}: expected 3 columns, got {}",
                idx + 2,
                cols.len()
            );
        }
        rows.push(LayoutMapRow {
            lupos_path: cols[0].to_owned(),
            linux_path: cols[1].to_owned(),
            note: cols[2].to_owned(),
        });
    }
    Ok(rows)
}

pub(crate) fn parse_layout_exceptions(text: &str) -> Result<Vec<LayoutExceptionRow>> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| anyhow!("empty layout exceptions"))?;
    if header != LAYOUT_EXCEPTIONS_HEADER {
        bail!(
            "invalid layout exceptions header: expected `{}`, got `{}`",
            LAYOUT_EXCEPTIONS_HEADER,
            header
        );
    }
    let mut rows = Vec::new();
    for (idx, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() != 2 {
            bail!(
                "invalid layout exceptions row {}: expected 2 columns, got {}",
                idx + 2,
                cols.len()
            );
        }
        rows.push(LayoutExceptionRow {
            lupos_path: cols[0].to_owned(),
            reason: cols[1].to_owned(),
        });
    }
    Ok(rows)
}

/// Collect every `.rs` file under `src/` in deterministic order.  Pure helper
/// so the audit can be unit-tested against a synthetic tree without touching
/// the real repo.
pub(crate) fn collect_rs_files(repo: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let src = repo.join("src");
    if !src.is_dir() {
        bail!("repo missing src/ directory at {}", src.display());
    }
    walk_collect_rs(&src, repo, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_collect_rs(dir: &Path, repo: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries =
        fs::read_dir(dir).with_context(|| format!("failed to read directory {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("error reading entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            walk_collect_rs(&path, repo, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let rel = path
                .strip_prefix(repo)
                .with_context(|| format!("path {} not under repo", path.display()))?;
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

/// Run the layout audit using on-disk files in `repo`.  Returns a report; the
/// caller decides how to surface it.  Splitting "compute" from "side effects"
/// lets the unit tests assert findings without writing to `target/`.
pub(crate) fn run_audit(repo: &Path) -> Result<AuditReport> {
    let rs_files = collect_rs_files(repo)?;
    let (map, exceptions) = load_or_derive_layout(repo, &rs_files)?;
    Ok(audit_against_data(repo, &map, &exceptions, &rs_files))
}

fn load_or_derive_layout(
    repo: &Path,
    rs_files: &[PathBuf],
) -> Result<(Vec<LayoutMapRow>, Vec<LayoutExceptionRow>)> {
    let map_path = repo.join(LAYOUT_MAP_PATH);
    let exceptions_path = repo.join(LAYOUT_EXCEPTIONS_PATH);
    if map_path.is_file() {
        let map_text = fs::read_to_string(&map_path)
            .with_context(|| format!("failed to read {LAYOUT_MAP_PATH}"))?;
        let exceptions = if exceptions_path.is_file() {
            let exceptions_text = fs::read_to_string(&exceptions_path)
                .with_context(|| format!("failed to read {LAYOUT_EXCEPTIONS_PATH}"))?;
            parse_layout_exceptions(&exceptions_text)?
        } else {
            Vec::new()
        };
        return Ok((parse_layout_map(&map_text)?, exceptions));
    }

    let mut map = Vec::new();
    let mut exceptions = Vec::new();
    for path in rs_files {
        let lupos_path = path.to_string_lossy().replace('\\', "/");
        let text = fs::read_to_string(repo.join(path))
            .with_context(|| format!("failed to read {lupos_path}"))?;
        let sources = scan_linux_source_tags(&text);
        if sources.is_empty() {
            exceptions.push(LayoutExceptionRow {
                lupos_path,
                reason: "Rust-only source (no linux-source header)".to_owned(),
            });
        } else {
            map.extend(sources.into_iter().map(|linux_path| LayoutMapRow {
                lupos_path: lupos_path.clone(),
                linux_path,
                note: "derived-from-linux-source-header".to_owned(),
            }));
        }
    }
    Ok((map, exceptions))
}

/// Pure audit logic — no I/O.  All filesystem checks consult `repo` via
/// `path.exists()`, which is fine to keep here since the helper is small and
/// only used by `run_audit`; the unit tests prime a synthetic repo on tmpfs.
pub(crate) fn audit_against_data(
    repo: &Path,
    map: &[LayoutMapRow],
    exceptions: &[LayoutExceptionRow],
    rs_files: &[PathBuf],
) -> AuditReport {
    let mut entries = Vec::new();

    let mut mapped: BTreeSet<String> = BTreeSet::new();
    let mut mapped_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for row in map {
        if !mapped_pairs.insert((row.lupos_path.clone(), row.linux_path.clone())) {
            entries.push(AuditEntry {
                finding: AuditFinding::DuplicateMapping,
                path: row.lupos_path.clone(),
                detail: format!("duplicate map row for {}", row.linux_path),
            });
        }
        mapped.insert(row.lupos_path.clone());
    }

    let mut excepted: BTreeSet<String> = BTreeSet::new();
    for row in exceptions {
        if !excepted.insert(row.lupos_path.clone()) {
            entries.push(AuditEntry {
                finding: AuditFinding::DuplicateMapping,
                path: row.lupos_path.clone(),
                detail: "duplicate exception row".to_owned(),
            });
        }
        if mapped.contains(&row.lupos_path) {
            entries.push(AuditEntry {
                finding: AuditFinding::DuplicateMapping,
                path: row.lupos_path.clone(),
                detail: "appears in both layout map and exceptions".to_owned(),
            });
        }
        if !repo.join(&row.lupos_path).exists() {
            entries.push(AuditEntry {
                finding: AuditFinding::MissingException,
                path: row.lupos_path.clone(),
                detail: row.reason.clone(),
            });
        }
    }

    for row in map {
        if !repo.join(&row.lupos_path).exists() {
            entries.push(AuditEntry {
                finding: AuditFinding::MissingLuposFile,
                path: row.lupos_path.clone(),
                detail: format!("mapped to {}", row.linux_path),
            });
        }
        // Empty linux_path is invalid; layout-oracle rows always carry a path
        // (either a file or a directory under vendor/linux/).
        if row.linux_path.is_empty() || !repo.join(&row.linux_path).exists() {
            entries.push(AuditEntry {
                finding: AuditFinding::MissingLinuxFile,
                path: row.lupos_path.clone(),
                detail: format!("linux_path `{}` not found", row.linux_path),
            });
        }
    }

    for rel in rs_files {
        let rel_str = path_to_unix(rel);
        if mapped.contains(&rel_str) || excepted.contains(&rel_str) {
            continue;
        }
        entries.push(AuditEntry {
            finding: AuditFinding::OrphanRsFile,
            path: rel_str,
            detail: "not in layout map or exceptions".to_owned(),
        });
    }

    entries.sort_by(|a, b| (a.finding, &a.path).cmp(&(b.finding, &b.path)));

    AuditReport {
        entries,
        map_rows: map.len(),
        exception_rows: exceptions.len(),
        rs_files_scanned: rs_files.len(),
    }
}

fn path_to_unix(path: &Path) -> String {
    let mut s = String::new();
    for (i, comp) in path.components().enumerate() {
        if i > 0 {
            s.push('/');
        }
        s.push_str(&comp.as_os_str().to_string_lossy());
    }
    s
}

pub(crate) fn audit_layout_cmd() -> Result<()> {
    let repo = repo_root()?;
    let report = run_audit(&repo)?;
    write_audit_report(&report)?;

    println!(
        "audit-layout: map_rows={} exception_rows={} rs_files_scanned={}",
        report.map_rows, report.exception_rows, report.rs_files_scanned
    );
    if report.is_clean() {
        println!("audit-layout: OK");
        Ok(())
    } else {
        for entry in &report.entries {
            eprintln!(
                "audit-layout: {} {} {}",
                entry.finding.as_str(),
                entry.path,
                entry.detail
            );
        }
        bail!(
            "audit-layout: {} drift entries (see {})",
            report.entries.len(),
            AUDIT_LAYOUT_REPORT_PATH
        )
    }
}

fn write_audit_report(report: &AuditReport) -> Result<()> {
    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let path = target.join("audit-layout.tsv");
    fs::write(&path, report.to_tsv()).with_context(|| format!("failed to write {}", path.display()))
}

// ===========================================================================
// Milestone 94 A2 — audit-parity: linux-parity header tag scanner
// ===========================================================================

pub(crate) const AUDIT_PARITY_REPORT_PATH: &str = "target/xtask/audit-parity.tsv";

/// Parity state declared by a Rust file's `//! linux-parity:` doc-comment tag.
/// `Missing` means the file does not declare a tag at all — distinct from
/// `Stub`, which is an explicit declaration that the rewrite is unfinished.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ParityTag {
    Complete,
    Partial,
    Stub,
    Missing,
}

impl ParityTag {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Stub => "stub",
            Self::Missing => "missing",
        }
    }
}

/// Severity threshold for the parity audit.  CLI maps `--fail-on missing` etc.
/// onto this enum; the audit walks the population once and the caller decides
/// what to exit on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParityFailMode {
    /// Never fail; just print the histogram.
    Never,
    /// Fail if any mapped file declares `stub`.
    Stub,
    /// Fail if any mapped file declares `partial` or `stub`.
    Partial,
    /// Fail if any mapped file lacks a tag, or declares `partial` or `stub`.
    Missing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParityScope {
    All,
    CriticalFutex,
    CriticalTime,
    CriticalTask,
    CriticalFdVfs,
    CriticalRuntime,
    Video,
}

impl ParityScope {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value {
            "all" => Ok(Self::All),
            "critical-futex" => Ok(Self::CriticalFutex),
            "critical-time" => Ok(Self::CriticalTime),
            "critical-task" => Ok(Self::CriticalTask),
            "critical-fd-vfs" => Ok(Self::CriticalFdVfs),
            "critical-runtime" => Ok(Self::CriticalRuntime),
            "video" => Ok(Self::Video),
            other => bail!(
                "unknown --scope value `{other}`; expected {}",
                Self::usage()
            ),
        }
    }

    pub(crate) fn usage() -> &'static str {
        "all|critical-futex|critical-time|critical-task|critical-fd-vfs|critical-runtime|video"
    }

    fn as_arg(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::CriticalFutex => "critical-futex",
            Self::CriticalTime => "critical-time",
            Self::CriticalTask => "critical-task",
            Self::CriticalFdVfs => "critical-fd-vfs",
            Self::CriticalRuntime => "critical-runtime",
            Self::Video => "video",
        }
    }

    fn includes_path(self, path: &str) -> bool {
        match self {
            Self::All => true,
            Self::CriticalFutex => {
                path.starts_with("src/kernel/futex/")
                    || matches!(
                        path,
                        "src/kernel/sched/mod.rs"
                            | "src/kernel/sched/wait.rs"
                            | "src/kernel/sched/swait.rs"
                            | "src/kernel/sched/wait_bit.rs"
                            | "src/kernel/locking/rt_mutex.rs"
                            | "src/kernel/locking/wake_q.rs"
                            | "src/kernel/time/hrtimer.rs"
                            | "src/kernel/time/posix_clock.rs"
                            | "src/kernel/time/sleep_timeout.rs"
                            | "src/kernel/time/timekeeping.rs"
                    )
            }
            Self::CriticalTime => is_critical_time_path(path),
            Self::CriticalTask => is_critical_task_path(path),
            Self::CriticalFdVfs => is_critical_fd_vfs_path(path),
            Self::CriticalRuntime => {
                Self::CriticalFutex.includes_path(path)
                    || Self::CriticalTime.includes_path(path)
                    || Self::CriticalTask.includes_path(path)
                    || Self::CriticalFdVfs.includes_path(path)
                    || is_critical_network_path(path)
            }
            Self::Video => is_video_output_path(path),
        }
    }
}

fn is_video_output_path(path: &str) -> bool {
    path.starts_with("src/linux_driver_abi/video/")
        || path.starts_with("src/linux_driver_abi/gpu/")
        || path == "src/linux_driver_abi/mod.rs"
        || path == "src/linux_driver_abi/pci/device.rs"
        || path == "src/linux_driver_abi/pci/enumerate.rs"
        || path == "src/linux_driver_abi/virtio/mod.rs"
        || path.starts_with("src/arch/x86/video/")
        || path == "src/arch/x86/boot/compressed/misc.rs"
        || path == "src/arch/x86/boot/legacy.rs"
        || path == "src/arch/x86/boot/main.rs"
        || path == "src/arch/x86/boot/mod.rs"
        || path.starts_with("src/arch/x86/boot/video")
        || path == "src/arch/x86/boot/vesa.rs"
        || path == "src/arch/x86/entry/thunk.rs"
        || path == "src/arch/x86/kernel/alternative.rs"
        || path == "src/arch/x86/kernel/early_quirks.rs"
        || path == "src/arch/x86/kernel/probe_roms.rs"
        || path == "src/arch/x86/mm/init.rs"
        || path == "src/arch/x86/realmode/mod.rs"
        || path == "src/arch/x86/realmode/rm/mod.rs"
        || path == "src/arch/x86/realmode/rm/wakemain.rs"
        || path.starts_with("src/arch/x86/realmode/rm/video")
        || path == "src/arch/x86/xen/mod.rs"
        || path == "src/arch/x86/xen/vga.rs"
        || path == "src/arch/x86/include/uapi/asm/bootparam.rs"
        || path == "src/init/main.rs"
        || path == "src/init/rootfs.rs"
        || path == "src/kernel/console.rs"
        || path == "src/kernel/dma/mod.rs"
        || path == "src/kernel/printk/log.rs"
        || path == "src/kernel/module/loader.rs"
        || path == "src/kernel/module/relocate.rs"
        || path == "src/kernel/module/symbols.rs"
        || path == "src/fs/ops.rs"
        || path == "src/fs/proc/consoles.rs"
        || path == "src/fs/sysfs/mount.rs"
        || path == "src/io_uring/mod.rs"
        || path == "src/rust/helpers/drm.rs"
        || path == "src/rust/helpers/gpu.rs"
        || path == "src/mm/fault.rs"
        || path == "src/mm/mmap.rs"
        || path == "src/mm/mm_init.rs"
        || path == "src/mm/pgprot.rs"
        || path == "src/mm/shmem.rs"
        || path == "src/mm/vma.rs"
}

fn is_critical_time_path(path: &str) -> bool {
    path.starts_with("src/kernel/time/")
}

fn is_critical_task_path(path: &str) -> bool {
    matches!(
        path,
        "src/kernel/clone.rs"
            | "src/kernel/exec.rs"
            | "src/kernel/exit.rs"
            | "src/kernel/files.rs"
            | "src/kernel/fork.rs"
            | "src/kernel/pid.rs"
            | "src/kernel/ptrace.rs"
            | "src/kernel/sched/mod.rs"
            | "src/kernel/sched/swait.rs"
            | "src/kernel/sched/wait.rs"
            | "src/kernel/sched/wait_bit.rs"
            | "src/kernel/session.rs"
            | "src/kernel/signal.rs"
            | "src/kernel/syscalls.rs"
            | "src/kernel/task.rs"
            | "src/kernel/task_work.rs"
            | "src/kernel/wait.rs"
    )
}

fn is_critical_fd_vfs_path(path: &str) -> bool {
    matches!(
        path,
        "src/fs/anon_inode.rs"
            | "src/fs/dcache.rs"
            | "src/fs/eventfd.rs"
            | "src/fs/eventpoll.rs"
            | "src/fs/fcntl.rs"
            | "src/fs/fdtable.rs"
            | "src/fs/file.rs"
            | "src/fs/file_table.rs"
            | "src/fs/fs_struct.rs"
            | "src/fs/inode.rs"
            | "src/fs/libfs.rs"
            | "src/fs/mount.rs"
            | "src/fs/namei.rs"
            | "src/fs/namespace.rs"
            | "src/fs/openat.rs"
            | "src/fs/pipe.rs"
            | "src/fs/read_write.rs"
            | "src/fs/select.rs"
            | "src/fs/super_block.rs"
            | "src/fs/syscalls.rs"
            | "src/fs/types.rs"
    ) || path.starts_with("src/fs/proc/")
        || path.starts_with("src/fs/ramfs/")
}

fn is_critical_network_path(path: &str) -> bool {
    matches!(
        path,
        "src/net/device.rs"
            | "src/net/fib.rs"
            | "src/net/ip.rs"
            | "src/net/link.rs"
            | "src/net/mod.rs"
            | "src/net/neighbour.rs"
            | "src/net/netfilter.rs"
            | "src/net/rtnetlink.rs"
            | "src/net/skbuff.rs"
            | "src/net/socket.rs"
            | "src/net/syscalls.rs"
            | "src/net/tcp.rs"
            | "src/net/udp.rs"
    )
}

impl ParityFailMode {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value {
            "never" => Ok(Self::Never),
            "stub" => Ok(Self::Stub),
            "partial" => Ok(Self::Partial),
            "missing" => Ok(Self::Missing),
            other => {
                bail!("unknown --fail-on value `{other}`; expected never|stub|partial|missing")
            }
        }
    }

    fn includes(self, tag: ParityTag) -> bool {
        match self {
            Self::Never => false,
            Self::Stub => matches!(tag, ParityTag::Stub),
            Self::Partial => matches!(tag, ParityTag::Stub | ParityTag::Partial),
            Self::Missing => matches!(
                tag,
                ParityTag::Stub | ParityTag::Partial | ParityTag::Missing
            ),
        }
    }
}

/// Look at the first ~50 lines of a Rust file for a `//! linux-parity:` tag.
/// We deliberately scope the search so a stray match deep in test fixtures or
/// vendored comments cannot satisfy the audit.
pub(crate) fn scan_parity_tag(text: &str) -> ParityTag {
    for line in text.lines().take(50) {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix("linux-parity:") {
                match value.trim() {
                    "complete" => return ParityTag::Complete,
                    "partial" => return ParityTag::Partial,
                    "stub" => return ParityTag::Stub,
                    _ => return ParityTag::Missing,
                }
            }
        }
    }
    ParityTag::Missing
}

/// Look at the first ~50 lines of a Rust file for the mapped Linux source.
pub(crate) fn scan_linux_source_tag(text: &str) -> Option<String> {
    scan_linux_source_tags(text).into_iter().next()
}

/// Look at the first ~50 lines of a Rust file for all mapped Linux sources.
pub(crate) fn scan_linux_source_tags(text: &str) -> BTreeSet<String> {
    let mut sources = BTreeSet::new();
    for line in text.lines().take(50) {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix("linux-source:") {
                let value = value.trim();
                if !value.is_empty() {
                    sources.extend(expand_linux_source_value(value));
                }
            }
        }
    }
    sources
}

fn expand_linux_source_value(value: &str) -> BTreeSet<String> {
    let mut expanded = BTreeSet::new();
    if let (Some(open), Some(close)) = (value.find('{'), value.find('}')) {
        if open < close {
            let prefix = &value[..open];
            let suffix = &value[close + 1..];
            for alternative in value[open + 1..close].split(',') {
                expanded.insert(format!("{prefix}{}{suffix}", alternative.trim()));
            }
            return expanded;
        }
    }

    expanded.extend(
        value
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_owned),
    );
    expanded
}

// ===========================================================================
// Part D — audit-tests: test-origin provenance scanner
// ===========================================================================

pub(crate) const AUDIT_TESTS_REPORT_PATH: &str = "target/xtask/audit-tests.tsv";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TestOriginKind {
    Linux,
    LuposSpecific,
    Missing,
    Invalid,
    MissingLinuxSource,
}

impl TestOriginKind {
    fn classification(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::LuposSpecific => "lupos-specific",
            Self::Missing | Self::Invalid | Self::MissingLinuxSource => "unjustified",
        }
    }

    fn finding(self) -> &'static str {
        match self {
            Self::Linux => "ok_linux",
            Self::LuposSpecific => "ok_lupos_specific",
            Self::Missing => "missing_test_origin",
            Self::Invalid => "invalid_test_origin",
            Self::MissingLinuxSource => "missing_linux_source",
        }
    }

    fn is_unjustified(self) -> bool {
        matches!(
            self,
            Self::Missing | Self::Invalid | Self::MissingLinuxSource
        )
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TestOriginEntry {
    pub path: String,
    pub kind: TestOriginKind,
    pub origin: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TestOriginReport {
    pub entries: Vec<TestOriginEntry>,
    pub files_scanned: usize,
    pub linux: usize,
    pub lupos_specific: usize,
    pub unjustified: usize,
}

impl TestOriginReport {
    pub(crate) fn is_clean(&self) -> bool {
        self.unjustified == 0
    }

    pub(crate) fn unjustified_entries(&self) -> impl Iterator<Item = &TestOriginEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind.is_unjustified())
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out = String::from("classification\tfinding\tpath\torigin\tdetail\n");
        for entry in &self.entries {
            out.push_str(entry.kind.classification());
            out.push('\t');
            out.push_str(entry.kind.finding());
            out.push('\t');
            out.push_str(&entry.path);
            out.push('\t');
            out.push_str(&entry.origin);
            out.push('\t');
            out.push_str(&entry.detail);
            out.push('\n');
        }
        out
    }
}

pub(crate) fn scan_test_origin_tag(text: &str) -> Option<String> {
    for line in text.lines().take(80) {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            let rest = rest.trim_start();
            if let Some(value) = rest.strip_prefix("test-origin:") {
                return Some(value.trim().to_owned());
            }
        }
    }
    None
}

fn is_test_bearing_file(path: &Path, text: &str) -> bool {
    let kunit_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with("kunit.rs"));
    kunit_file || text.contains("#[test]") || text.contains("#[cfg(test)]")
}

pub(crate) fn collect_test_origin_files(repo: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for root in ["src", "xtask"] {
        let dir = repo.join(root);
        if dir.is_dir() {
            walk_collect_test_origin_files(&dir, repo, &mut out)?;
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn walk_collect_test_origin_files(dir: &Path, repo: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries =
        fs::read_dir(dir).with_context(|| format!("failed to read directory {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("error reading entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            walk_collect_test_origin_files(&path, repo, out)?;
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if is_test_bearing_file(&path, &text) {
            let rel = path
                .strip_prefix(repo)
                .with_context(|| format!("path {} not under repo", path.display()))?;
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

pub(crate) fn run_test_origin_audit(repo: &Path) -> Result<TestOriginReport> {
    let files = collect_test_origin_files(repo)?;
    audit_test_origins_for_files(repo, &files)
}

pub(crate) fn audit_test_origins_for_files(
    repo: &Path,
    files: &[PathBuf],
) -> Result<TestOriginReport> {
    let mut entries = Vec::new();
    let mut linux = 0usize;
    let mut lupos_specific = 0usize;
    let mut unjustified = 0usize;

    for rel in files {
        let path = path_to_unix(rel);
        let text = fs::read_to_string(repo.join(rel))
            .with_context(|| format!("failed to read {}", path))?;
        let Some(origin) = scan_test_origin_tag(&text) else {
            unjustified += 1;
            entries.push(TestOriginEntry {
                path,
                kind: TestOriginKind::Missing,
                origin: String::new(),
                detail: "expected `//! test-origin: linux:<vendor/linux/...>` or `//! test-origin: lupos-specific:<reason>` in the file header".to_owned(),
            });
            continue;
        };

        if let Some(source) = origin.strip_prefix("linux:") {
            let source = source.trim();
            if source.is_empty() || !source.starts_with("vendor/linux/") {
                unjustified += 1;
                entries.push(TestOriginEntry {
                    path,
                    kind: TestOriginKind::Invalid,
                    origin,
                    detail: "linux test-origin must name a vendor/linux path".to_owned(),
                });
            } else if !repo.join(source).exists() {
                unjustified += 1;
                entries.push(TestOriginEntry {
                    path,
                    kind: TestOriginKind::MissingLinuxSource,
                    origin,
                    detail: "test-origin linux source path does not exist".to_owned(),
                });
            } else {
                linux += 1;
                entries.push(TestOriginEntry {
                    path,
                    kind: TestOriginKind::Linux,
                    origin,
                    detail: "source-backed test provenance".to_owned(),
                });
            }
        } else if let Some(reason) = origin.strip_prefix("lupos-specific:") {
            if reason.trim().is_empty() {
                unjustified += 1;
                entries.push(TestOriginEntry {
                    path,
                    kind: TestOriginKind::Invalid,
                    origin,
                    detail: "lupos-specific test-origin must include a reason".to_owned(),
                });
            } else {
                lupos_specific += 1;
                entries.push(TestOriginEntry {
                    path,
                    kind: TestOriginKind::LuposSpecific,
                    origin,
                    detail: "Lupos-specific test provenance".to_owned(),
                });
            }
        } else {
            unjustified += 1;
            entries.push(TestOriginEntry {
                path,
                kind: TestOriginKind::Invalid,
                origin,
                detail: "unknown test-origin prefix".to_owned(),
            });
        }
    }

    entries.sort_by(|a, b| (a.kind, &a.path).cmp(&(b.kind, &b.path)));
    Ok(TestOriginReport {
        entries,
        files_scanned: files.len(),
        linux,
        lupos_specific,
        unjustified,
    })
}

pub(crate) fn audit_tests_cmd() -> Result<()> {
    let repo = repo_root()?;
    let report = run_test_origin_audit(&repo)?;
    write_test_origin_report(&report)?;

    println!(
        "audit-tests: files_scanned={} linux={} lupos_specific={} unjustified={}",
        report.files_scanned, report.linux, report.lupos_specific, report.unjustified
    );
    if report.is_clean() {
        println!("audit-tests: OK");
        Ok(())
    } else {
        for entry in report.unjustified_entries() {
            eprintln!(
                "audit-tests: {} {} {}",
                entry.kind.finding(),
                entry.path,
                entry.detail
            );
        }
        bail!(
            "audit-tests: {} unjustified test-bearing files (see {})",
            report.unjustified,
            AUDIT_TESTS_REPORT_PATH
        )
    }
}

fn write_test_origin_report(report: &TestOriginReport) -> Result<()> {
    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let path = target.join("audit-tests.tsv");
    fs::write(&path, report.to_tsv()).with_context(|| format!("failed to write {}", path.display()))
}

#[derive(Clone, Debug)]
pub(crate) struct ParityEntry {
    pub path: String,
    pub tag: ParityTag,
    pub expected_source: String,
    pub source: Option<String>,
    pub source_matches: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ParityReport {
    pub entries: Vec<ParityEntry>,
    pub complete: usize,
    pub partial: usize,
    pub stub: usize,
    pub missing: usize,
    pub missing_source: usize,
    pub source_mismatch: usize,
}

impl ParityReport {
    fn record(
        &mut self,
        path: String,
        tag: ParityTag,
        expected_source: String,
        sources: BTreeSet<String>,
    ) {
        match tag {
            ParityTag::Complete => self.complete += 1,
            ParityTag::Partial => self.partial += 1,
            ParityTag::Stub => self.stub += 1,
            ParityTag::Missing => self.missing += 1,
        }
        let source_matches = sources.contains(&expected_source);
        if sources.is_empty() {
            self.missing_source += 1;
        } else if !source_matches {
            self.source_mismatch += 1;
        }
        let source = if sources.is_empty() {
            None
        } else {
            Some(sources.into_iter().collect::<Vec<_>>().join(","))
        };
        self.entries.push(ParityEntry {
            path,
            tag,
            expected_source,
            source,
            source_matches,
        });
    }

    pub(crate) fn failures(&self, mode: ParityFailMode, scope: ParityScope) -> Vec<&ParityEntry> {
        self.entries
            .iter()
            .filter(|e| scope.includes_path(&e.path) && mode.includes(e.tag))
            .collect()
    }

    pub(crate) fn tag_presence_failures(&self, scope: ParityScope) -> Vec<&ParityEntry> {
        self.entries
            .iter()
            .filter(|e| {
                scope.includes_path(&e.path)
                    && (e.tag == ParityTag::Missing || e.source.is_none() || !e.source_matches)
            })
            .collect()
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out =
            String::from("path\tparity_tag\tlinux_source\texpected_linux_source\tsource_status\n");
        for entry in &self.entries {
            out.push_str(&entry.path);
            out.push('\t');
            out.push_str(entry.tag.as_str());
            out.push('\t');
            if let Some(source) = &entry.source {
                out.push_str(source);
            }
            out.push('\t');
            out.push_str(&entry.expected_source);
            out.push('\t');
            let status = match entry.source.as_ref() {
                None => "missing",
                Some(_) if entry.source_matches => "ok",
                Some(_) => "mismatch",
            };
            out.push_str(status);
            out.push('\n');
        }
        out
    }
}

pub(crate) fn run_parity_audit(repo: &Path) -> Result<ParityReport> {
    let rs_files = collect_rs_files(repo)?;
    let (map, _) = load_or_derive_layout(repo, &rs_files)?;

    let mut report = ParityReport::default();
    for row in &map {
        // Skip layout-oracle directory mappings; the tag policy applies only
        // to Rust source files.  Directory rows surface in the map for
        // structure-only assertions that audit-layout already covers.
        if !row.lupos_path.ends_with(".rs") {
            continue;
        }
        let path = repo.join(&row.lupos_path);
        let (tag, sources) = match fs::read_to_string(&path) {
            Ok(text) => (scan_parity_tag(&text), scan_linux_source_tags(&text)),
            // audit-layout is the source of truth for missing files; here we
            // just treat them as missing-tag so the count is honest.
            Err(_) => (ParityTag::Missing, BTreeSet::new()),
        };
        report.record(row.lupos_path.clone(), tag, row.linux_path.clone(), sources);
    }
    Ok(report)
}

pub(crate) fn audit_parity_cmd(
    fail_on: ParityFailMode,
    require_tags: bool,
    scope: ParityScope,
) -> Result<()> {
    let repo = repo_root()?;
    let report = run_parity_audit(&repo)?;
    write_parity_report(&report)?;

    let total = report.complete + report.partial + report.stub + report.missing;
    println!(
        "audit-parity: total={} complete={} partial={} stub={} missing={} missing_source={} source_mismatch={} scope={}",
        total,
        report.complete,
        report.partial,
        report.stub,
        report.missing,
        report.missing_source,
        report.source_mismatch,
        scope.as_arg()
    );

    if require_tags {
        let failures = report.tag_presence_failures(scope);
        if !failures.is_empty() {
            for entry in &failures {
                let source = entry.source.as_deref().unwrap_or("<missing>");
                eprintln!(
                    "audit-parity: tag/source {} {} expected_source={} source={}",
                    entry.tag.as_str(),
                    entry.path,
                    entry.expected_source,
                    source
                );
            }
            bail!(
                "audit-parity: {} entries fail --require-tags (see {})",
                failures.len(),
                AUDIT_PARITY_REPORT_PATH
            );
        }
    }

    let failures = report.failures(fail_on, scope);
    if failures.is_empty() {
        println!(
            "audit-parity: OK (--fail-on {}, require_tags={}, scope={})",
            fail_on.as_arg(),
            require_tags,
            scope.as_arg()
        );
        Ok(())
    } else {
        for entry in &failures {
            eprintln!("audit-parity: {} {}", entry.tag.as_str(), entry.path);
        }
        bail!(
            "audit-parity: {} entries fail --fail-on {} (see {})",
            failures.len(),
            fail_on.as_arg(),
            AUDIT_PARITY_REPORT_PATH
        )
    }
}

impl ParityFailMode {
    fn as_arg(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Stub => "stub",
            Self::Partial => "partial",
            Self::Missing => "missing",
        }
    }
}

fn write_parity_report(report: &ParityReport) -> Result<()> {
    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let path = target.join("audit-parity.tsv");
    fs::write(&path, report.to_tsv()).with_context(|| format!("failed to write {}", path.display()))
}

// ===========================================================================
// MM visible-symbol audit
// ===========================================================================

pub(crate) const AUDIT_MM_SYMBOLS_REPORT_PATH: &str = "target/xtask/audit-mm-symbols.tsv";
pub(crate) const MM_PARITY_DOC_DIR: &str = "src/docs/mm-parity";

const MM_LINUX_NAMED_HEADERS: &[&str] = &[
    "vendor/linux/include/linux/mman.h",
    "vendor/linux/include/linux/rmap.h",
    "vendor/linux/include/linux/pagemap.h",
    "vendor/linux/include/linux/vmalloc.h",
    "vendor/linux/include/linux/slab.h",
    "vendor/linux/include/linux/swap.h",
    "vendor/linux/include/linux/shmem_fs.h",
    "vendor/linux/include/linux/mempolicy.h",
    "vendor/linux/include/linux/huge_mm.h",
    "vendor/linux/include/linux/hugetlb.h",
    "vendor/linux/include/linux/page-flags.h",
    "vendor/linux/include/linux/page_ref.h",
    "vendor/linux/include/linux/pagewalk.h",
    "vendor/linux/include/linux/mmu_notifier.h",
    "vendor/linux/include/linux/memremap.h",
    "vendor/linux/include/linux/ksm.h",
    "vendor/linux/include/linux/zswap.h",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MmLinuxSymbolKind {
    Inline,
    Prototype,
    Export,
    Syscall,
}

impl MmLinuxSymbolKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Prototype => "prototype",
            Self::Export => "export",
            Self::Syscall => "syscall",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MmLinuxSymbol {
    pub name: String,
    pub kind: MmLinuxSymbolKind,
    pub source: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RustVisibility {
    Public,
    Crate,
    Private,
}

impl RustVisibility {
    fn as_str(self) -> &'static str {
        match self {
            Self::Public => "pub",
            Self::Crate => "pub(crate)",
            Self::Private => "private",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Public => 3,
            Self::Crate => 2,
            Self::Private => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MmRustSymbol {
    pub name: String,
    pub path: String,
    pub visibility: RustVisibility,
    pub parity: ParityTag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MmSymbolStatus {
    Complete,
    Partial,
    Stub,
    Missing,
}

impl MmSymbolStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Stub => "stub",
            Self::Missing => "missing",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MmSymbolFailMode {
    Never,
    Missing,
    Stub,
    Partial,
}

impl MmSymbolFailMode {
    pub(crate) fn parse(value: &str) -> Result<Self> {
        match value {
            "never" => Ok(Self::Never),
            "missing" => Ok(Self::Missing),
            "stub" => Ok(Self::Stub),
            "partial" => Ok(Self::Partial),
            other => {
                bail!("unknown --fail-on value `{other}`; expected never|missing|stub|partial")
            }
        }
    }

    fn includes(self, status: MmSymbolStatus) -> bool {
        match self {
            Self::Never => false,
            Self::Missing => matches!(status, MmSymbolStatus::Missing),
            Self::Stub => matches!(status, MmSymbolStatus::Missing | MmSymbolStatus::Stub),
            Self::Partial => matches!(
                status,
                MmSymbolStatus::Missing | MmSymbolStatus::Stub | MmSymbolStatus::Partial
            ),
        }
    }

    fn as_arg(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Missing => "missing",
            Self::Stub => "stub",
            Self::Partial => "partial",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MmSymbolEntry {
    pub linux: MmLinuxSymbol,
    pub rust: Option<MmRustSymbol>,
    pub status: MmSymbolStatus,
    pub notes: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct MmSymbolReport {
    pub entries: Vec<MmSymbolEntry>,
    pub linux_symbols_scanned: usize,
    pub rust_symbols_scanned: usize,
    pub complete: usize,
    pub partial: usize,
    pub stub: usize,
    pub missing: usize,
}

impl MmSymbolReport {
    pub(crate) fn failures(&self, mode: MmSymbolFailMode) -> Vec<&MmSymbolEntry> {
        self.entries
            .iter()
            .filter(|entry| mode.includes(entry.status))
            .collect()
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out = String::from(
            "linux_symbol\tkind\tlinux_source\trust_symbol\trust_path\trust_visibility\trust_parity\tstatus\tnotes\n",
        );
        for entry in &self.entries {
            out.push_str(&entry.linux.name);
            out.push('\t');
            out.push_str(entry.linux.kind.as_str());
            out.push('\t');
            out.push_str(&entry.linux.source);
            out.push('\t');
            if let Some(rust) = &entry.rust {
                out.push_str(&rust.name);
                out.push('\t');
                out.push_str(&rust.path);
                out.push('\t');
                out.push_str(rust.visibility.as_str());
                out.push('\t');
                out.push_str(rust.parity.as_str());
            } else {
                out.push_str("\t\t\t");
            }
            out.push('\t');
            out.push_str(entry.status.as_str());
            out.push('\t');
            out.push_str(&entry.notes);
            out.push('\n');
        }
        out
    }

    fn record(&mut self, entry: MmSymbolEntry) {
        match entry.status {
            MmSymbolStatus::Complete => self.complete += 1,
            MmSymbolStatus::Partial => self.partial += 1,
            MmSymbolStatus::Stub => self.stub += 1,
            MmSymbolStatus::Missing => self.missing += 1,
        }
        self.entries.push(entry);
    }
}

pub(crate) fn run_mm_symbol_audit(repo: &Path) -> Result<MmSymbolReport> {
    let linux_symbols = collect_mm_linux_symbols(repo)?;
    let rust_symbols = collect_mm_rust_symbols(repo)?;
    let rust_symbols_scanned = rust_symbols.values().map(Vec::len).sum();

    let mut report = MmSymbolReport {
        linux_symbols_scanned: linux_symbols.len(),
        rust_symbols_scanned,
        ..MmSymbolReport::default()
    };

    for linux in linux_symbols {
        let rust = rust_symbols
            .get(&linux.name)
            .and_then(|matches| best_mm_rust_symbol(matches))
            .cloned();
        let (status, notes) = classify_mm_symbol_match(rust.as_ref());
        report.record(MmSymbolEntry {
            linux,
            rust,
            status,
            notes,
        });
    }
    report
        .entries
        .sort_by(|a, b| (&a.linux.name, a.linux.kind).cmp(&(&b.linux.name, b.linux.kind)));
    Ok(report)
}

pub(crate) fn audit_mm_symbols_cmd(fail_on: MmSymbolFailMode, write_docs: bool) -> Result<()> {
    let repo = repo_root()?;
    let report = run_mm_symbol_audit(&repo)?;
    write_mm_symbol_report(&report)?;
    if write_docs {
        write_mm_parity_docs(&repo, &report)?;
    }

    println!(
        "audit-mm-symbols: linux_symbols={} rust_symbols={} complete={} partial={} stub={} missing={}",
        report.linux_symbols_scanned,
        report.rust_symbols_scanned,
        report.complete,
        report.partial,
        report.stub,
        report.missing
    );
    if write_docs {
        println!("audit-mm-symbols: wrote {MM_PARITY_DOC_DIR}");
    }

    let failures = report.failures(fail_on);
    if failures.is_empty() {
        println!("audit-mm-symbols: OK (--fail-on {})", fail_on.as_arg());
        Ok(())
    } else {
        for entry in failures.iter().take(50) {
            eprintln!(
                "audit-mm-symbols: {} {} ({})",
                entry.status.as_str(),
                entry.linux.name,
                entry.linux.source
            );
        }
        if failures.len() > 50 {
            eprintln!(
                "audit-mm-symbols: ... {} more failing symbols omitted",
                failures.len() - 50
            );
        }
        bail!(
            "audit-mm-symbols: {} entries fail --fail-on {} (see {})",
            failures.len(),
            fail_on.as_arg(),
            AUDIT_MM_SYMBOLS_REPORT_PATH
        )
    }
}

fn collect_mm_linux_symbols(repo: &Path) -> Result<Vec<MmLinuxSymbol>> {
    let mut symbols: BTreeMap<String, MmLinuxSymbol> = BTreeMap::new();
    let mut mm_sources = Vec::new();
    collect_files_with_extensions(
        &repo.join("vendor/linux/mm"),
        repo,
        &["c", "h"],
        &mut mm_sources,
    )?;

    for rel in mm_sources {
        let path = repo.join(&rel);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        for line in text.lines() {
            for name in extract_syscall_symbols(line) {
                insert_linux_symbol(
                    &mut symbols,
                    name,
                    MmLinuxSymbolKind::Syscall,
                    path_to_unix(&rel),
                );
            }
            for name in extract_export_symbols(line) {
                insert_linux_symbol(
                    &mut symbols,
                    name,
                    MmLinuxSymbolKind::Export,
                    path_to_unix(&rel),
                );
            }
        }
    }

    for rel in mm_header_paths(repo)? {
        let path = repo.join(&rel);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut brace_depth = 0i32;
        for line in text.lines() {
            if brace_depth == 0 {
                if let Some((name, kind)) = extract_header_fn_symbol(line) {
                    insert_linux_symbol(&mut symbols, name, kind, path_to_unix(&rel));
                }
            }
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;
            if brace_depth < 0 {
                brace_depth = 0;
            }
        }
    }

    Ok(symbols.into_values().collect())
}

fn mm_header_paths(repo: &Path) -> Result<Vec<PathBuf>> {
    let include_linux = repo.join("vendor/linux/include/linux");
    let mut headers = BTreeSet::new();
    if include_linux.is_dir() {
        for entry in fs::read_dir(&include_linux)
            .with_context(|| format!("failed to read {}", include_linux.display()))?
        {
            let entry = entry
                .with_context(|| format!("error reading entry in {}", include_linux.display()))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if name.starts_with("mm") && name.ends_with(".h") {
                headers.insert(rel_path(repo, &path)?);
            }
        }
    }
    for header in MM_LINUX_NAMED_HEADERS {
        let path = repo.join(header);
        if path.is_file() {
            headers.insert(PathBuf::from(header));
        }
    }
    Ok(headers.into_iter().collect())
}

fn collect_mm_rust_symbols(repo: &Path) -> Result<BTreeMap<String, Vec<MmRustSymbol>>> {
    let mut files = Vec::new();
    collect_files_with_extensions(&repo.join("src/mm"), repo, &["rs"], &mut files)?;
    collect_files_with_extensions(&repo.join("src/arch/x86/mm"), repo, &["rs"], &mut files)?;
    files.sort();
    files.dedup();

    let mut symbols: BTreeMap<String, Vec<MmRustSymbol>> = BTreeMap::new();
    for rel in files {
        let path = repo.join(&rel);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let parity = scan_parity_tag(&text);
        for line in text.lines() {
            if let Some((visibility, name)) = extract_rust_fn_symbol(line) {
                symbols.entry(name.clone()).or_default().push(MmRustSymbol {
                    name,
                    path: path_to_unix(&rel),
                    visibility,
                    parity,
                });
            }
        }
    }
    Ok(symbols)
}

fn collect_files_with_extensions(
    dir: &Path,
    repo: &Path,
    extensions: &[&str],
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("error reading entry in {}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_extensions(&path, repo, extensions, out)?;
        } else {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if extensions.iter().any(|candidate| *candidate == ext) {
                out.push(rel_path(repo, &path)?);
            }
        }
    }
    Ok(())
}

fn rel_path(repo: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(repo)
        .map(Path::to_path_buf)
        .with_context(|| format!("path {} not under repo {}", path.display(), repo.display()))
}

fn insert_linux_symbol(
    symbols: &mut BTreeMap<String, MmLinuxSymbol>,
    name: String,
    kind: MmLinuxSymbolKind,
    source: String,
) {
    if !is_c_identifier(&name) {
        return;
    }
    match symbols.get_mut(&name) {
        Some(existing) => {
            if kind > existing.kind {
                existing.kind = kind;
            }
            if !existing.source.split(';').any(|seen| seen == source) {
                existing.source.push(';');
                existing.source.push_str(&source);
            }
        }
        None => {
            symbols.insert(name.clone(), MmLinuxSymbol { name, kind, source });
        }
    }
}

fn extract_syscall_symbols(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for macro_name in ["SYSCALL_DEFINE", "COMPAT_SYSCALL_DEFINE"] {
        let mut rest = line;
        while let Some(idx) = rest.find(macro_name) {
            rest = &rest[idx + macro_name.len()..];
            let after_arity = rest.trim_start_matches(|ch: char| ch.is_ascii_digit());
            let Some(open_idx) = after_arity.find('(') else {
                break;
            };
            let args = &after_arity[open_idx + 1..];
            let name = args
                .split([',', ')'])
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches("__se_sys_")
                .to_owned();
            if is_c_identifier(&name) {
                out.push(name);
            }
            rest = &args[args.find([',', ')']).map_or(args.len(), |i| i)..];
        }
    }
    out
}

fn extract_export_symbols(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    for macro_name in [
        "EXPORT_SYMBOL",
        "EXPORT_SYMBOL_GPL",
        "EXPORT_SYMBOL_NS",
        "EXPORT_SYMBOL_NS_GPL",
        "EXPORT_PER_CPU_SYMBOL",
        "EXPORT_PER_CPU_SYMBOL_GPL",
    ] {
        let mut rest = line;
        while let Some(idx) = rest.find(macro_name) {
            rest = &rest[idx + macro_name.len()..];
            let Some(open_idx) = rest.find('(') else {
                break;
            };
            let args = &rest[open_idx + 1..];
            let name = args
                .split([',', ')'])
                .next()
                .unwrap_or("")
                .trim()
                .to_owned();
            if is_c_identifier(&name) {
                out.push(name);
            }
            rest = &args[args.find([',', ')']).map_or(args.len(), |i| i)..];
        }
    }
    out.sort();
    out.dedup();
    out
}

fn extract_header_fn_symbol(line: &str) -> Option<(String, MmLinuxSymbolKind)> {
    let cleaned = strip_known_c_attributes(strip_inline_c_comment(line));
    let line = cleaned.trim();
    if line.is_empty()
        || line.starts_with('#')
        || line.starts_with("typedef")
        || line.starts_with("struct ")
        || line.starts_with("union ")
        || line.starts_with("enum ")
        || line.starts_with("return ")
        || line.starts_with("if ")
        || line.starts_with("while ")
        || line.starts_with("for ")
        || line.starts_with("switch ")
        || line.starts_with("WARN")
    {
        return None;
    }

    let is_inline = line.contains(" inline ")
        || line.starts_with("static inline")
        || line.contains("__always_inline");
    let declaration_like = is_inline
        || line.starts_with("extern ")
        || line.starts_with("__must_check")
        || line.ends_with(';');
    if !declaration_like {
        return None;
    }

    let open_idx = line.find('(')?;
    let before = line[..open_idx].trim_end();
    if before.contains('=') || before.contains("(*") {
        return None;
    }
    if !is_inline
        && !line.starts_with("extern ")
        && !line.starts_with("__must_check")
        && !before.chars().any(char::is_whitespace)
    {
        return None;
    }
    let name = before
        .split(|ch: char| ch.is_whitespace() || ch == '*' || ch == '(' || ch == ')')
        .filter(|token| !token.is_empty())
        .next_back()?;
    if !is_c_identifier(name) || is_c_keyword(name) {
        return None;
    }
    let kind = if is_inline {
        MmLinuxSymbolKind::Inline
    } else {
        MmLinuxSymbolKind::Prototype
    };
    Some((name.to_owned(), kind))
}

fn strip_inline_c_comment(line: &str) -> &str {
    line.split("//")
        .next()
        .unwrap_or(line)
        .split("/*")
        .next()
        .unwrap_or(line)
}

fn strip_known_c_attributes(line: &str) -> String {
    let mut out = line.to_owned();
    for attr in [
        "__alloc_size",
        "__realloc_size",
        "__printf",
        "__scanf",
        "__counted_by",
    ] {
        while let Some(idx) = out.find(attr) {
            let after_attr = idx + attr.len();
            let rest = out[after_attr..].trim_start();
            if !rest.starts_with('(') {
                break;
            }
            let ws = out[after_attr..].len() - rest.len();
            let open = after_attr + ws;
            let Some(close_rel) = out[open..].find(')') else {
                break;
            };
            let close = open + close_rel + 1;
            out.replace_range(idx..close, "");
        }
    }
    for attr in [
        "__assume_kmalloc_alignment",
        "__assume_page_alignment",
        "__malloc",
        "__must_check",
    ] {
        out = out.replace(attr, "");
    }
    out
}

fn extract_rust_fn_symbol(line: &str) -> Option<(RustVisibility, String)> {
    let line = line.split("//").next().unwrap_or(line).trim_start();
    if line.starts_with("macro_rules!") || line.starts_with('#') {
        return None;
    }
    let idx = line.find("fn ")?;
    let prefix = line[..idx].trim();
    let visibility = if prefix.starts_with("pub(crate)") || prefix.starts_with("pub(super)") {
        RustVisibility::Crate
    } else if prefix.starts_with("pub ") || prefix == "pub" || prefix.starts_with("pub(") {
        RustVisibility::Public
    } else {
        RustVisibility::Private
    };
    let after = &line[idx + 3..];
    let name: String = after
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    if is_c_identifier(&name) {
        Some((visibility, name))
    } else {
        None
    }
}

fn best_mm_rust_symbol(matches: &[MmRustSymbol]) -> Option<&MmRustSymbol> {
    matches.iter().max_by_key(|symbol| {
        (
            mm_symbol_status_rank(classify_mm_symbol_match(Some(symbol)).0),
            symbol.visibility.rank(),
            match symbol.parity {
                ParityTag::Complete => 3,
                ParityTag::Partial => 2,
                ParityTag::Stub => 1,
                ParityTag::Missing => 0,
            },
        )
    })
}

fn classify_mm_symbol_match(symbol: Option<&MmRustSymbol>) -> (MmSymbolStatus, String) {
    let Some(symbol) = symbol else {
        return (
            MmSymbolStatus::Missing,
            "no exact Rust function name under src/mm or src/arch/x86/mm".to_owned(),
        );
    };
    if symbol.visibility == RustVisibility::Private {
        return (
            MmSymbolStatus::Partial,
            "exact name exists but is private; expose a module-boundary wrapper when behavior is ready"
                .to_owned(),
        );
    }
    match symbol.parity {
        ParityTag::Complete => {
            let note = if symbol.visibility == RustVisibility::Crate {
                "exact name exists with crate visibility"
            } else {
                "exact name exists in a complete parity file"
            };
            (MmSymbolStatus::Complete, note.to_owned())
        }
        ParityTag::Partial | ParityTag::Missing => (
            MmSymbolStatus::Partial,
            format!(
                "exact name exists in {} parity file",
                symbol.parity.as_str()
            ),
        ),
        ParityTag::Stub => (
            MmSymbolStatus::Stub,
            "exact name exists in stub parity file".to_owned(),
        ),
    }
}

fn mm_symbol_status_rank(status: MmSymbolStatus) -> u8 {
    match status {
        MmSymbolStatus::Complete => 4,
        MmSymbolStatus::Partial => 3,
        MmSymbolStatus::Stub => 2,
        MmSymbolStatus::Missing => 1,
    }
}

fn is_c_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn is_c_keyword(value: &str) -> bool {
    matches!(
        value,
        "if" | "for"
            | "while"
            | "switch"
            | "return"
            | "sizeof"
            | "typeof"
            | "struct"
            | "union"
            | "enum"
            | "static"
            | "extern"
            | "inline"
            | "const"
            | "volatile"
    )
}

fn write_mm_symbol_report(report: &MmSymbolReport) -> Result<()> {
    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let path = target.join("audit-mm-symbols.tsv");
    fs::write(&path, report.to_tsv()).with_context(|| format!("failed to write {}", path.display()))
}

fn write_mm_parity_docs(repo: &Path, report: &MmSymbolReport) -> Result<()> {
    let dir = repo.join(MM_PARITY_DOC_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let mut symbol_map = String::from("linux_symbol\tkind\tlinux_source\n");
    for entry in &report.entries {
        symbol_map.push_str(&entry.linux.name);
        symbol_map.push('\t');
        symbol_map.push_str(entry.linux.kind.as_str());
        symbol_map.push('\t');
        symbol_map.push_str(&entry.linux.source);
        symbol_map.push('\n');
    }
    fs::write(dir.join("symbol-map.tsv"), symbol_map)
        .with_context(|| format!("failed to write {MM_PARITY_DOC_DIR}/symbol-map.tsv"))?;

    fs::write(dir.join("coverage.tsv"), report.to_tsv())
        .with_context(|| format!("failed to write {MM_PARITY_DOC_DIR}/coverage.tsv"))?;

    let mut blocked = String::from("linux_symbol\tstatus\treason\n");
    for entry in &report.entries {
        if entry.status != MmSymbolStatus::Complete {
            blocked.push_str(&entry.linux.name);
            blocked.push('\t');
            blocked.push_str(entry.status.as_str());
            blocked.push('\t');
            blocked.push_str(&entry.notes);
            blocked.push('\n');
        }
    }
    fs::write(dir.join("blocked.tsv"), blocked)
        .with_context(|| format!("failed to write {MM_PARITY_DOC_DIR}/blocked.tsv"))?;

    Ok(())
}

// ===========================================================================
// audit-kunit: KunitCase placeholder detector
// ===========================================================================

pub(crate) const AUDIT_KUNIT_REPORT_PATH: &str = "target/xtask/audit-kunit.tsv";

/// Minimum number of "real" statements a KunitCase `run` body must carry.
/// Below this it almost certainly is a placeholder.  Calibrated against the
/// known placeholder set in `src/kernel/kunit.rs`:
/// `lib_arithmetic_and_ordering` (3 statements), `mm_range_accounting` (3),
/// `kernel_range_exclusion` (3), `generated_case_ok` (1).
const KUNIT_PLACEHOLDER_STATEMENT_THRESHOLD: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct KunitCaseRecord {
    pub file: String,
    pub suite: String,
    pub name: String,
    pub source: String,
    pub run_fn: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum KunitFinding {
    /// `case.source` points at a vendor path that does not exist.
    MissingSource,
    /// `run` function body has fewer than the placeholder threshold of
    /// statements — almost certainly a placeholder.
    LowStatementCount,
    /// `run` function name could not be located in the same file.
    UnresolvedRunFn,
}

impl KunitFinding {
    fn as_str(self) -> &'static str {
        match self {
            Self::MissingSource => "missing_source",
            Self::LowStatementCount => "low_statement_count",
            Self::UnresolvedRunFn => "unresolved_run_fn",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct KunitFindingEntry {
    pub finding: KunitFinding,
    pub case: KunitCaseRecord,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct KunitAuditReport {
    pub cases_scanned: usize,
    pub files_scanned: usize,
    pub entries: Vec<KunitFindingEntry>,
}

impl KunitAuditReport {
    pub(crate) fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out = String::from("finding\tfile\tsuite\tname\tsource\trun_fn\tdetail\n");
        for entry in &self.entries {
            out.push_str(entry.finding.as_str());
            out.push('\t');
            out.push_str(&entry.case.file);
            out.push('\t');
            out.push_str(&entry.case.suite);
            out.push('\t');
            out.push_str(&entry.case.name);
            out.push('\t');
            out.push_str(&entry.case.source);
            out.push('\t');
            out.push_str(&entry.case.run_fn);
            out.push('\t');
            out.push_str(&entry.detail);
            out.push('\n');
        }
        out
    }
}

/// Collect every Rust file that may host KunitCase tables: `src/kernel/kunit.rs`
/// (the central runner) and any future `src/<sub>/kunit.rs` or
/// `src/<sub>/kunit/**/*.rs` files added by the Milestone 95 refactor.
pub(crate) fn collect_kunit_files(repo: &Path) -> Result<Vec<PathBuf>> {
    let mut rs = collect_rs_files(repo)?;
    rs.retain(|p| {
        let unix = path_to_unix(p);
        unix.ends_with("/kunit.rs") || unix.contains("/kunit/") || unix == "src/kernel/kunit.rs"
    });
    Ok(rs)
}

/// Extract every `KunitCase { ... }` literal in a Rust source file.  Uses a
/// brace-balanced scan rather than syn so xtask stays dependency-free.
pub(crate) fn scan_kunit_cases(file: &str, text: &str) -> Vec<KunitCaseRecord> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let needle = b"KunitCase {";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let body_start = i + needle.len();
            if let Some(body_end) = find_matching_brace(bytes, body_start) {
                let body = &text[body_start..body_end];
                if let Some(case) = parse_kunit_case_body(file, body) {
                    out.push(case);
                }
                i = body_end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn find_matching_brace(bytes: &[u8], start: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'"' => {
                // Skip string literal contents, including escaped quotes.
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn parse_kunit_case_body(file: &str, body: &str) -> Option<KunitCaseRecord> {
    let suite = extract_string_field(body, "suite")?;
    let name = extract_string_field(body, "name")?;
    let source = extract_string_field(body, "source")?;
    let run_fn = extract_ident_field(body, "run")?;
    Some(KunitCaseRecord {
        file: file.to_owned(),
        suite,
        name,
        source,
        run_fn,
    })
}

fn extract_string_field(body: &str, field: &str) -> Option<String> {
    let key = format!("{field}:");
    let idx = body.find(&key)?;
    let rest = &body[idx + key.len()..];
    let after_ws: &str = rest.trim_start();
    let after_ws = after_ws.strip_prefix('"')?;
    let end = after_ws.find('"')?;
    Some(after_ws[..end].to_owned())
}

fn extract_ident_field(body: &str, field: &str) -> Option<String> {
    let key = format!("{field}:");
    let idx = body.find(&key)?;
    let rest = &body[idx + key.len()..];
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .unwrap_or(rest.len());
    let ident = &rest[..end];
    if ident.is_empty() {
        None
    } else {
        Some(ident.to_owned())
    }
}

/// Count meaningful statements inside `fn <name>(...) -> bool { ... }`.
/// "Statement" here means a non-blank, non-brace, non-comment line inside the
/// outermost body braces — sufficient to detect placeholders like
/// `lib_arithmetic_and_ordering` without pulling in syn.  Returns `None` if
/// the function is not found.
pub(crate) fn count_fn_statements(text: &str, fn_name: &str) -> Option<usize> {
    let needle = format!("fn {fn_name}(");
    let idx = text.find(&needle)?;
    let after = &text[idx..];
    let brace_idx = after.find('{')?;
    let bytes = after.as_bytes();
    let body_start = brace_idx + 1;
    let body_end = find_matching_brace(bytes, body_start)?;
    let body = &after[body_start..body_end];
    let mut count = 0usize;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if trimmed == "{" || trimmed == "}" {
            continue;
        }
        count += 1;
    }
    Some(count)
}

pub(crate) fn run_kunit_audit(repo: &Path) -> Result<KunitAuditReport> {
    let files = collect_kunit_files(repo)?;
    let mut report = KunitAuditReport::default();
    report.files_scanned = files.len();
    for rel in &files {
        let path = repo.join(rel);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let file_str = path_to_unix(rel);
        let cases = scan_kunit_cases(&file_str, &text);
        report.cases_scanned += cases.len();
        for case in cases {
            // 1. Vendor source path must exist.
            if !repo.join(&case.source).exists() {
                let detail = format!("source `{}` not found", case.source);
                report.entries.push(KunitFindingEntry {
                    finding: KunitFinding::MissingSource,
                    case: case.clone(),
                    detail,
                });
            }
            // 2. Run function must be findable in the same file.
            match count_fn_statements(&text, &case.run_fn) {
                Some(n) if n < KUNIT_PLACEHOLDER_STATEMENT_THRESHOLD => {
                    let detail = format!(
                        "run body has {n} statement(s); placeholder threshold is {KUNIT_PLACEHOLDER_STATEMENT_THRESHOLD}"
                    );
                    report.entries.push(KunitFindingEntry {
                        finding: KunitFinding::LowStatementCount,
                        case: case.clone(),
                        detail,
                    });
                }
                None => {
                    let detail = format!("run function `{}` not found in file", case.run_fn);
                    report.entries.push(KunitFindingEntry {
                        finding: KunitFinding::UnresolvedRunFn,
                        case: case.clone(),
                        detail,
                    });
                }
                Some(_) => {}
            }
        }
    }
    report.entries.sort_by(|a, b| {
        (a.finding, &a.case.file, &a.case.name).cmp(&(b.finding, &b.case.file, &b.case.name))
    });
    Ok(report)
}

pub(crate) fn audit_kunit_cmd() -> Result<()> {
    let repo = repo_root()?;
    let report = run_kunit_audit(&repo)?;
    write_kunit_report(&report)?;

    println!(
        "audit-kunit: files_scanned={} cases_scanned={}",
        report.files_scanned, report.cases_scanned
    );
    if report.is_clean() {
        println!("audit-kunit: OK");
        Ok(())
    } else {
        for entry in &report.entries {
            eprintln!(
                "audit-kunit: {} {}::{} ({}) — {}",
                entry.finding.as_str(),
                entry.case.suite,
                entry.case.name,
                entry.case.file,
                entry.detail,
            );
        }
        bail!(
            "audit-kunit: {} placeholder/missing entries (see {})",
            report.entries.len(),
            AUDIT_KUNIT_REPORT_PATH
        )
    }
}

fn write_kunit_report(report: &KunitAuditReport) -> Result<()> {
    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let path = target.join("audit-kunit.tsv");
    fs::write(&path, report.to_tsv()).with_context(|| format!("failed to write {}", path.display()))
}

// ===========================================================================
// parity-table: per-file Linux parity map (lupos | linux | parity % | todos)
// ===========================================================================
//
// Honest, regenerable parity view. The `//! linux-parity:` tags drifted into
// over-claims (e.g. a 60-line `kvm/x86.rs` tagged `complete` against a 14,571
// line `kvm/x86.c`). This computes parity from an SLOC ratio against the
// mapped `vendor/linux/` file, reconciles it against the declared tag, and
// emits both a Markdown table (spliced into ROADMAP.md between sentinels) and
// a tag-mismatch report that drives honest re-tagging.

pub(crate) const PARITY_TABLE_MD_PATH: &str = "target/xtask/parity-table.md";
pub(crate) const PARITY_TABLE_MISMATCH_PATH: &str = "target/xtask/parity-table-mismatch.tsv";
pub(crate) const PARITY_TABLE_BEGIN: &str =
    "<!-- PARITY-TABLE:BEGIN (generated by `cargo xtask parity-table`) -->";
pub(crate) const PARITY_TABLE_END: &str = "<!-- PARITY-TABLE:END -->";

/// x86 subtrees mirrored only for Linux layout parity: KVM/hypervisor, perf
/// uarch counters, Hyper-V/Xen/coco guests, SGX, resctrl, MTRR, microcode,
/// power, purgatory. None are on the boot/Arch path and Lupos does not
/// reimplement drivers/hypervisor (CLAUDE.md 2b), so they are honestly `stub`.
pub(crate) const OUT_OF_SCOPE_PREFIXES: &[&str] = &[
    "src/arch/x86/kvm/",
    "src/arch/x86/events/",
    "src/arch/x86/hyperv/",
    "src/arch/x86/coco/",
    "src/arch/x86/xen/",
    "src/arch/x86/kernel/cpu/sgx/",
    "src/arch/x86/kernel/cpu/resctrl/",
    "src/arch/x86/kernel/cpu/mtrr/",
    "src/arch/x86/kernel/cpu/microcode/",
    "src/arch/x86/power/",
    "src/arch/x86/purgatory/",
];

pub(crate) fn is_out_of_scope(path: &str) -> bool {
    OUT_OF_SCOPE_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// Rough "source line" count: non-blank lines that are not pure comments.
/// Applied identically to Rust and C so the ratio stays fair. Block comments
/// are tracked across lines; a line that opens code before `/*` still counts.
pub(crate) fn count_sloc(text: &str) -> usize {
    let mut in_block = false;
    let mut n = 0usize;
    for raw in text.lines() {
        let line = raw.trim();
        if in_block {
            if let Some(end) = line.find("*/") {
                in_block = false;
                let after = line[end + 2..].trim();
                if !after.is_empty() && !after.starts_with("//") {
                    n += 1;
                }
            }
            continue;
        }
        if line.is_empty() || line.starts_with("//") || line.starts_with('*') {
            continue;
        }
        if line.starts_with("/*") {
            if !line.contains("*/") {
                in_block = true;
            }
            continue;
        }
        n += 1;
    }
    n
}

pub(crate) fn strip_inline_cfg_test_modules(text: &str) -> String {
    let mut out = String::new();
    let mut pending_cfg_test = false;
    let mut skipping_test_mod = false;
    let mut brace_depth = 0isize;

    for raw in text.lines() {
        let line = raw.trim_start();

        if skipping_test_mod {
            brace_depth += raw.matches('{').count() as isize;
            brace_depth -= raw.matches('}').count() as isize;
            if brace_depth <= 0 {
                skipping_test_mod = false;
                brace_depth = 0;
            }
            continue;
        }

        if pending_cfg_test {
            if line.starts_with("mod tests") || line.contains(" mod tests") {
                skipping_test_mod = true;
                brace_depth += raw.matches('{').count() as isize;
                brace_depth -= raw.matches('}').count() as isize;
                if brace_depth <= 0 && raw.contains('{') {
                    skipping_test_mod = false;
                    brace_depth = 0;
                }
                pending_cfg_test = false;
                continue;
            }

            out.push_str("#[cfg(test)]\n");
            pending_cfg_test = false;
        }

        if line == "#[cfg(test)]" {
            pending_cfg_test = true;
            continue;
        }

        out.push_str(raw);
        out.push('\n');
    }

    if pending_cfg_test {
        out.push_str("#[cfg(test)]\n");
    }

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParityRowKind {
    /// lupos file maps to a single `vendor/linux` file: parity = SLOC ratio.
    FileMap,
    /// lupos file maps to a `vendor/linux` directory (aggregate): parity from tag.
    DirMap,
}

#[derive(Clone, Debug)]
pub(crate) struct ParityRow {
    pub lupos_path: String,
    pub linux_path: String,
    pub subsystem: String,
    pub kind: ParityRowKind,
    pub lup_sloc: usize,
    pub lnx_sloc: usize,
    pub parity: u32,
    pub tag: ParityTag,
    pub out_of_scope: bool,
    pub todos: String,
    /// `over` (complete tag but low parity) / `under` (stub tag but high parity).
    pub mismatch: Option<&'static str>,
}

/// Group a lupos path into a subsystem bucket. `arch/x86` is split one level
/// deeper because it dominates the tree; everything else groups at the first
/// directory under `src/`.
pub(crate) fn subsystem_of(path: &str) -> String {
    let rest = path.strip_prefix("src/").unwrap_or(path);
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return "(root)".to_owned();
    }
    if parts[0] == "arch" && parts.len() >= 3 {
        // arch/x86/<sub>
        if parts.len() >= 4 {
            format!("arch/{}/{}", parts[1], parts[2])
        } else {
            format!("arch/{}", parts[1])
        }
    } else {
        parts[0].to_owned()
    }
}

fn tag_parity_fallback(tag: ParityTag) -> u32 {
    match tag {
        ParityTag::Complete => 90,
        ParityTag::Partial => 50,
        ParityTag::Stub => 10,
        ParityTag::Missing => 0,
    }
}

pub(crate) fn run_parity_table(repo: &Path) -> Result<Vec<ParityRow>> {
    let rs_files = collect_rs_files(repo)?;
    let (map, _) = load_or_derive_layout(repo, &rs_files)?;

    let mut rows = Vec::new();
    for row in &map {
        // Only file-backed lupos sources get a parity row; pure module/dir
        // oracle rows where the lupos side is a directory are skipped.
        let lup_abs = repo.join(&row.lupos_path);
        if !lup_abs.is_file() {
            continue;
        }
        let lup_text = fs::read_to_string(&lup_abs).unwrap_or_default();
        let lup_text_for_sloc = strip_inline_cfg_test_modules(&lup_text);
        let lup_sloc = count_sloc(&lup_text_for_sloc);
        let tag = scan_parity_tag(&lup_text);
        let out_of_scope = is_out_of_scope(&row.lupos_path);

        let lnx_abs = repo.join(&row.linux_path);
        let (kind, lnx_sloc, parity) = if lnx_abs.is_file() {
            let lnx_text = fs::read_to_string(&lnx_abs).unwrap_or_default();
            let lnx_sloc = count_sloc(&lnx_text);
            let parity = if lnx_sloc == 0 {
                if lup_sloc > 0 { 100 } else { 0 }
            } else {
                ((lup_sloc as f64 / lnx_sloc as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32
            };
            (ParityRowKind::FileMap, lnx_sloc, parity)
        } else {
            // Directory / aggregate mapping: no honest LOC ratio; fall back to tag.
            (ParityRowKind::DirMap, 0, tag_parity_fallback(tag))
        };

        let mismatch = if kind == ParityRowKind::FileMap {
            if tag == ParityTag::Complete && parity < 70 {
                Some("over")
            } else if tag == ParityTag::Stub && parity >= 40 && !out_of_scope {
                // Out-of-scope placeholders are intentionally `stub` regardless of
                // how small their Linux counterpart is; do not flag them as under.
                Some("under")
            } else {
                None
            }
        } else {
            None
        };

        let todos = match kind {
            ParityRowKind::DirMap => {
                format!("aggregates Linux dir; parity from tag ({})", tag.as_str())
            }
            ParityRowKind::FileMap if out_of_scope && parity < 25 => {
                "layout-only placeholder; out of scope for boot/Arch goal".to_owned()
            }
            ParityRowKind::FileMap if parity < 15 => {
                format!("stub: ~{parity}% of Linux SLOC ({lup_sloc}/{lnx_sloc})")
            }
            ParityRowKind::FileMap if parity < 70 => {
                format!("partial: ~{parity}% of Linux SLOC ({lup_sloc}/{lnx_sloc})")
            }
            ParityRowKind::FileMap => "—".to_owned(),
        };

        rows.push(ParityRow {
            lupos_path: row.lupos_path.clone(),
            linux_path: row.linux_path.clone(),
            subsystem: subsystem_of(&row.lupos_path),
            kind,
            lup_sloc,
            lnx_sloc,
            parity,
            tag,
            out_of_scope,
            todos,
            mismatch,
        });
    }
    rows.sort_by(|a, b| (&a.subsystem, &a.lupos_path).cmp(&(&b.subsystem, &b.lupos_path)));
    Ok(rows)
}

/// SLOC-weighted parity over the FileMap rows in `rows` (DirMap rows excluded
/// from the denominator since they have no measured Linux SLOC).
fn weighted_parity(rows: &[&ParityRow]) -> u32 {
    let mut lup = 0usize;
    let mut lnx = 0usize;
    for r in rows {
        if r.kind == ParityRowKind::FileMap {
            // Cap each file's contribution at its Linux SLOC so a file larger
            // than its counterpart cannot push the subsystem average past 100%.
            lup += r.lup_sloc.min(r.lnx_sloc);
            lnx += r.lnx_sloc;
        }
    }
    if lnx == 0 {
        return 0;
    }
    (((lup as f64 / lnx as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0)) as u32
}

pub(crate) fn render_parity_markdown(rows: &[ParityRow]) -> String {
    use std::collections::BTreeMap;
    let mut by_sub: BTreeMap<&str, Vec<&ParityRow>> = BTreeMap::new();
    for r in rows {
        by_sub.entry(r.subsystem.as_str()).or_default().push(r);
    }

    let mut out = String::new();
    out.push_str(PARITY_TABLE_BEGIN);
    out.push_str("\n\n## Linux Parity Map\n\n");
    out.push_str(
        "Per-file parity against `vendor/linux/`. `parity %` is an SLOC ratio of the lupos\n\
         file vs its mapped Linux file (capped at 100%); directory/aggregate mappings fall\n\
         back to the declared `//! linux-parity:` tag. Regenerate with `cargo xtask parity-table`.\n\n",
    );

    // Per-subsystem summary first.
    out.push_str("### Summary by subsystem\n\n");
    out.push_str("| subsystem | files | parity % | scope |\n|---|---:|---:|---|\n");
    let mut on_goal: Vec<&ParityRow> = Vec::new();
    let mut all_rows: Vec<&ParityRow> = Vec::new();
    for (sub, srows) in &by_sub {
        let p = weighted_parity(srows);
        let oos = srows.iter().all(|r| r.out_of_scope);
        let scope = if oos { "out-of-scope" } else { "on-goal" };
        out.push_str(&format!(
            "| {} | {} | {}% | {} |\n",
            sub,
            srows.len(),
            p,
            scope
        ));
        for r in srows {
            all_rows.push(r);
            if !r.out_of_scope {
                on_goal.push(r);
            }
        }
    }
    out.push_str(&format!(
        "| **TOTAL (all mapped files)** | {} | **{}%** | full Linux kernel |\n",
        all_rows.len(),
        weighted_parity(&all_rows)
    ));
    out.push_str(&format!(
        "| **TOTAL (on-goal path)** | {} | **{}%** | excludes kvm/events/hyperv/coco/xen/sgx/resctrl/mtrr/microcode |\n\n",
        on_goal.len(),
        weighted_parity(&on_goal)
    ));

    // Per-subsystem detail tables.
    for (sub, srows) in &by_sub {
        let p = weighted_parity(srows);
        out.push_str(&format!(
            "### {} (~{}% · {} files)\n\n",
            sub,
            p,
            srows.len()
        ));
        out.push_str("| lupos file | linux file | parity % | todos |\n|---|---|---:|---|\n");
        for r in srows {
            let parity_cell = match r.kind {
                ParityRowKind::FileMap => format!("{}%", r.parity),
                ParityRowKind::DirMap => format!("{}% (tag)", r.parity),
            };
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                r.lupos_path, r.linux_path, parity_cell, r.todos
            ));
        }
        out.push('\n');
    }

    out.push_str(PARITY_TABLE_END);
    out.push('\n');
    out
}

fn parity_mismatch_tsv(rows: &[ParityRow]) -> String {
    let mut out = String::from(
        "kind\tlupos_path\tlinux_path\ttag\tparity\tlup_sloc\tlnx_sloc\tsuggested_tag\n",
    );
    for r in rows {
        let Some(kind) = r.mismatch else { continue };
        let suggested = if r.parity >= 70 {
            "complete"
        } else if r.parity >= 15 {
            "partial"
        } else {
            "stub"
        };
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            kind,
            r.lupos_path,
            r.linux_path,
            r.tag.as_str(),
            r.parity,
            r.lup_sloc,
            r.lnx_sloc,
            suggested
        ));
    }
    out
}

pub(crate) fn parity_table_cmd() -> Result<()> {
    let repo = repo_root()?;
    let rows = run_parity_table(&repo)?;
    let markdown = render_parity_markdown(&rows);
    let mismatch = parity_mismatch_tsv(&rows);

    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    fs::write(repo.join(PARITY_TABLE_MD_PATH), &markdown)
        .with_context(|| format!("failed to write {PARITY_TABLE_MD_PATH}"))?;
    fs::write(repo.join(PARITY_TABLE_MISMATCH_PATH), &mismatch)
        .with_context(|| format!("failed to write {PARITY_TABLE_MISMATCH_PATH}"))?;

    // Splice into ROADMAP.md between sentinels when present.
    let roadmap_path = repo.join("ROADMAP.md");
    let mut spliced = false;
    if let Ok(existing) = fs::read_to_string(&roadmap_path) {
        if let (Some(begin), Some(end_rel)) = (
            existing.find(PARITY_TABLE_BEGIN),
            existing.find(PARITY_TABLE_END),
        ) {
            let end = end_rel + PARITY_TABLE_END.len();
            let mut next = String::with_capacity(existing.len());
            next.push_str(&existing[..begin]);
            next.push_str(markdown.trim_end());
            next.push_str(&existing[end..]);
            fs::write(&roadmap_path, next).context("failed to splice ROADMAP.md")?;
            spliced = true;
        }
    }

    let file_rows = rows
        .iter()
        .filter(|r| r.kind == ParityRowKind::FileMap)
        .count();
    let mismatches = rows.iter().filter(|r| r.mismatch.is_some()).count();
    let all: Vec<&ParityRow> = rows.iter().collect();
    let on_goal: Vec<&ParityRow> = rows.iter().filter(|r| !r.out_of_scope).collect();
    println!(
        "parity-table: rows={} file_maps={} mismatches={} parity_all={}% parity_on_goal={}% roadmap_spliced={}",
        rows.len(),
        file_rows,
        mismatches,
        weighted_parity(&all),
        weighted_parity(&on_goal),
        spliced,
    );
    println!("parity-table: wrote {PARITY_TABLE_MD_PATH} and {PARITY_TABLE_MISMATCH_PATH}");
    if !spliced {
        println!(
            "parity-table: ROADMAP.md sentinels not found; add `{PARITY_TABLE_BEGIN}` / `{PARITY_TABLE_END}` to splice in place"
        );
    }
    Ok(())
}

// ===========================================================================
// Part A — config-parity: lupos_defconfig must track Linux x86_64_defconfig
// ===========================================================================
//
// `configs/lupos_defconfig` is the projection of upstream
// `vendor/linux/arch/x86/configs/x86_64_defconfig` onto the symbols Lupos's
// Kconfig defines today.  This audit diffs the overlapping symbol set and also
// requires the upstream generic x86_64 video selections.  The latter must not
// disappear merely because a local Kconfig symbol was accidentally omitted.
//
// The one sanctioned divergence is the driver-as-module policy (CLAUDE.md 2a):
// device drivers Lupos loads as Linux-built `.ko` payloads are forced to `=m`
// where upstream builds them in (`=y`).  Those `y`→`m` demotions are reported
// as `module-override` and do not fail the gate; every other mismatch does.

pub(crate) const LUPOS_DEFCONFIG_PATH: &str = "configs/lupos_defconfig";
pub(crate) const LUPOS_KCONFIG_PATH: &str = "src/kernel/Kconfig";
pub(crate) const LINUX_X86_64_DEFCONFIG_PATH: &str =
    "vendor/linux/arch/x86/configs/x86_64_defconfig";
pub(crate) const AUDIT_CONFIG_REPORT_PATH: &str = "target/xtask/config-parity.tsv";

pub(crate) const REQUIRED_X86_64_GENERIC_VIDEO_SYMBOLS: &[&str] = &[
    "CONFIG_AGP",
    "CONFIG_AGP_AMD64",
    "CONFIG_AGP_INTEL",
    "CONFIG_DRM",
    "CONFIG_DRM_I915",
    "CONFIG_DRM_VIRTIO_GPU",
];

const LINUX_DRIVER_MODULE_OVERRIDE_SYMBOLS: &[&str] = &[
    "CONFIG_AGP",
    "CONFIG_AGP_AMD64",
    "CONFIG_AGP_INTEL",
    "CONFIG_ATA",
    "CONFIG_ATA_PIIX",
    "CONFIG_BLK_DEV_SD",
    "CONFIG_BLK_DEV_SR",
    "CONFIG_CHR_DEV_SG",
    "CONFIG_8139TOO",
    "CONFIG_DRM_I915",
    "CONFIG_DRM_VIRTIO_GPU",
    "CONFIG_E100",
    "CONFIG_E1000",
    "CONFIG_E1000E",
    "CONFIG_FORCEDETH",
    "CONFIG_I2C_I801",
    "CONFIG_PHYLIB",
    "CONFIG_MII",
    "CONFIG_NET_9P",
    "CONFIG_NET_9P_VIRTIO",
    "CONFIG_NETCONSOLE",
    "CONFIG_PATA_AMD",
    "CONFIG_PATA_OLDPIIX",
    "CONFIG_PATA_SCH",
    "CONFIG_R8169",
    "CONFIG_REALTEK_PHY",
    "CONFIG_SATA_AHCI",
    "CONFIG_SCSI_SPI_ATTRS",
    "CONFIG_SCSI_VIRTIO",
    "CONFIG_SKY2",
    "CONFIG_TIGON3",
    "CONFIG_SND",
    "CONFIG_SND_HRTIMER",
    "CONFIG_SND_HDA_INTEL",
    "CONFIG_SND_SEQUENCER",
    "CONFIG_SND_SEQ_DUMMY",
    "CONFIG_SOUND",
    "CONFIG_USB_EHCI_HCD",
    "CONFIG_USB_EHCI_PCI",
    "CONFIG_USB_MON",
    "CONFIG_USB_OHCI_HCD",
    "CONFIG_USB_OHCI_HCD_PCI",
    "CONFIG_USB_STORAGE",
    "CONFIG_USB_PRINTER",
    "CONFIG_USB_UHCI_HCD",
    "CONFIG_VIRTIO_BLK",
    "CONFIG_VIRTIO_CONSOLE",
    "CONFIG_VIRTIO_INPUT",
    "CONFIG_VIRTIO_NET",
    "CONFIG_VIRTIO_PCI",
    "CONFIG_X86_PKG_TEMP_THERMAL",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ConfigParityKind {
    /// Overlapping symbol whose value matches upstream.
    Match,
    /// `y` upstream demoted to `m` locally (driver `.ko` payload, CLAUDE.md 2a).
    ModuleOverride,
    /// Overlapping symbol whose value diverges from upstream for any other reason.
    Divergence,
}

impl ConfigParityKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::ModuleOverride => "module-override",
            Self::Divergence => "divergence",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ConfigParityEntry {
    pub symbol: String,
    pub lupos_value: String,
    pub linux_value: String,
    pub kind: ConfigParityKind,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ConfigParityReport {
    pub entries: Vec<ConfigParityEntry>,
    pub matched: usize,
    pub module_overrides: usize,
    pub divergences: usize,
}

impl ConfigParityReport {
    pub(crate) fn is_clean(&self) -> bool {
        self.divergences == 0
    }

    pub(crate) fn divergent_entries(&self) -> impl Iterator<Item = &ConfigParityEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == ConfigParityKind::Divergence)
    }

    pub(crate) fn to_tsv(&self) -> String {
        let mut out = String::from("kind\tsymbol\tlupos_value\tlinux_value\n");
        for entry in &self.entries {
            out.push_str(entry.kind.as_str());
            out.push('\t');
            out.push_str(&entry.symbol);
            out.push('\t');
            out.push_str(&entry.lupos_value);
            out.push('\t');
            out.push_str(&entry.linux_value);
            out.push('\n');
        }
        out
    }
}

/// Parse a Linux-style `defconfig` into a symbol → value map.
///
/// `CONFIG_X=y|m|"str"|123` records the literal value; `# CONFIG_X is not set`
/// records `n`.  Comments and blank lines are ignored.
pub(crate) fn parse_defconfig(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            // `# CONFIG_X is not set` is the canonical "disabled" form.
            let rest = rest.trim();
            if let Some(sym) = rest.strip_prefix("CONFIG_") {
                if let Some(name) = sym.strip_suffix(" is not set") {
                    map.insert(format!("CONFIG_{}", name.trim()), "n".to_owned());
                }
            }
            continue;
        }
        if let Some((sym, value)) = line.split_once('=') {
            let sym = sym.trim();
            if sym.starts_with("CONFIG_") {
                map.insert(sym.to_owned(), value.trim().to_owned());
            }
        }
    }
    map
}

pub(crate) fn run_config_parity_audit(repo: &Path) -> Result<ConfigParityReport> {
    let lupos_text = fs::read_to_string(repo.join(LUPOS_DEFCONFIG_PATH))
        .with_context(|| format!("failed to read {LUPOS_DEFCONFIG_PATH}"))?;
    let kconfig_text = fs::read_to_string(repo.join(LUPOS_KCONFIG_PATH))
        .with_context(|| format!("failed to read {LUPOS_KCONFIG_PATH}"))?;
    let linux_path = repo.join(LINUX_X86_64_DEFCONFIG_PATH);
    let linux_text = fs::read_to_string(&linux_path).with_context(|| {
        format!(
            "failed to read {LINUX_X86_64_DEFCONFIG_PATH} (is vendor/linux/ populated? run vendor/setup_linux.sh)"
        )
    })?;
    let mut report = audit_config_parity(&lupos_text, &linux_text);
    audit_required_video_kconfig_symbols(&mut report, &kconfig_text, &linux_text);
    Ok(report)
}

fn parse_kconfig_symbols(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("config ")
                .or_else(|| line.strip_prefix("menuconfig "))
                .map(|name| format!("CONFIG_{}", name.trim()))
        })
        .collect()
}

fn audit_required_video_kconfig_symbols(
    report: &mut ConfigParityReport,
    kconfig_text: &str,
    linux_text: &str,
) {
    let declared = parse_kconfig_symbols(kconfig_text);
    let linux = parse_defconfig(linux_text);

    for &symbol in REQUIRED_X86_64_GENERIC_VIDEO_SYMBOLS {
        let Some(linux_value) = linux.get(symbol) else {
            continue;
        };
        if declared.contains(symbol) {
            continue;
        }

        if let Some(entry) = report
            .entries
            .iter_mut()
            .find(|entry| entry.symbol == symbol)
        {
            if entry.kind == ConfigParityKind::Divergence {
                continue;
            }
            match entry.kind {
                ConfigParityKind::Match => report.matched -= 1,
                ConfigParityKind::ModuleOverride => report.module_overrides -= 1,
                ConfigParityKind::Divergence => unreachable!(),
            }
            entry.kind = ConfigParityKind::Divergence;
            entry.lupos_value = "<missing-kconfig>".to_owned();
            report.divergences += 1;
        } else {
            report.entries.push(ConfigParityEntry {
                symbol: symbol.to_owned(),
                lupos_value: "<missing-kconfig>".to_owned(),
                linux_value: linux_value.clone(),
                kind: ConfigParityKind::Divergence,
            });
            report.divergences += 1;
        }
    }
    report
        .entries
        .sort_by(|a, b| (a.kind, &a.symbol).cmp(&(b.kind, &b.symbol)));
}

pub(crate) fn audit_config_parity(lupos_text: &str, linux_text: &str) -> ConfigParityReport {
    let lupos = parse_defconfig(lupos_text);
    let linux = parse_defconfig(linux_text);

    let mut report = ConfigParityReport::default();
    let mut compared_symbols = lupos
        .keys()
        .filter(|symbol| linux.contains_key(*symbol))
        .cloned()
        .collect::<BTreeSet<_>>();
    compared_symbols.extend(
        REQUIRED_X86_64_GENERIC_VIDEO_SYMBOLS
            .iter()
            .filter(|symbol| linux.contains_key(**symbol))
            .map(|symbol| (*symbol).to_owned()),
    );

    for symbol in compared_symbols {
        let linux_value = linux
            .get(&symbol)
            .expect("compared symbols must be present in Linux defconfig");
        let Some(lupos_value) = lupos.get(&symbol) else {
            report.divergences += 1;
            report.entries.push(ConfigParityEntry {
                symbol,
                lupos_value: "<missing>".to_owned(),
                linux_value: linux_value.clone(),
                kind: ConfigParityKind::Divergence,
            });
            continue;
        };
        let kind = if lupos_value == linux_value {
            report.matched += 1;
            ConfigParityKind::Match
        } else if lupos_value == "m"
            && linux_value == "y"
            && LINUX_DRIVER_MODULE_OVERRIDE_SYMBOLS.contains(&symbol.as_str())
        {
            report.module_overrides += 1;
            ConfigParityKind::ModuleOverride
        } else {
            report.divergences += 1;
            ConfigParityKind::Divergence
        };
        report.entries.push(ConfigParityEntry {
            symbol,
            lupos_value: lupos_value.clone(),
            linux_value: linux_value.clone(),
            kind,
        });
    }
    report
        .entries
        .sort_by(|a, b| (a.kind, &a.symbol).cmp(&(b.kind, &b.symbol)));
    report
}

fn write_config_parity_report(report: &ConfigParityReport) -> Result<()> {
    let target = xtask_target_dir()?;
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    let path = target.join("config-parity.tsv");
    fs::write(&path, report.to_tsv()).with_context(|| format!("failed to write {}", path.display()))
}

pub(crate) fn config_parity_cmd() -> Result<()> {
    let repo = repo_root()?;
    let report = run_config_parity_audit(&repo)?;
    write_config_parity_report(&report)?;

    println!(
        "config-parity: compared={} match={} module_override={} divergence={}",
        report.entries.len(),
        report.matched,
        report.module_overrides,
        report.divergences,
    );
    if report.is_clean() {
        println!("config-parity: OK");
        Ok(())
    } else {
        for entry in report.divergent_entries() {
            eprintln!(
                "config-parity: {} lupos={} upstream={}",
                entry.symbol, entry.lupos_value, entry.linux_value
            );
        }
        bail!(
            "config-parity: {} symbol(s) diverge from {} (see {})",
            report.divergences,
            LINUX_X86_64_DEFCONFIG_PATH,
            AUDIT_CONFIG_REPORT_PATH
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_repo() -> PathBuf {
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let base =
            std::env::temp_dir().join(format!("lupos-xtask-audit-{}-{}", std::process::id(), n));
        if base.exists() {
            fs::remove_dir_all(&base).unwrap();
        }
        fs::create_dir_all(&base).unwrap();
        base
    }

    fn write(repo: &Path, rel: &str, content: &str) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn write_basic_tree(repo: &Path) {
        // Mapped Rust file + its Linux counterpart.
        write(repo, "src/arch/x86/boot/a20.rs", "// rust\n");
        write(repo, "vendor/linux/arch/x86/boot/a20.c", "/* c */\n");
        // Exception Rust file (no Linux counterpart).
        write(repo, "src/arch/x86/lib/insn_eval.rs", "// rust\n");
        // Crate root — listed as an exception (Cargo module index).
        write(repo, "src/lib.rs", "// crate root\n");
        // Layout TSVs.
        write(
            repo,
            LAYOUT_MAP_PATH,
            "lupos_path\tlinux_path\tnote\n\
             src/arch/x86/boot/a20.rs\tvendor/linux/arch/x86/boot/a20.c\tlayout-oracle\n",
        );
        write(
            repo,
            LAYOUT_EXCEPTIONS_PATH,
            "lupos_path\treason\n\
             src/arch/x86/lib/insn_eval.rs\tRust implementation without Linux counterpart\n\
             src/lib.rs\tCargo crate module index only; no runtime implementation logic\n",
        );
    }

    #[test]
    fn parse_layout_map_accepts_canonical_header() {
        let text = "lupos_path\tlinux_path\tnote\n\
                    src/a.rs\tvendor/linux/a.c\tlayout-oracle\n";
        let rows = parse_layout_map(text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lupos_path, "src/a.rs");
        assert_eq!(rows[0].linux_path, "vendor/linux/a.c");
        assert_eq!(rows[0].note, "layout-oracle");
    }

    #[test]
    fn parse_layout_map_rejects_wrong_header() {
        let text = "wrong\theader\n";
        assert!(parse_layout_map(text).is_err());
    }

    #[test]
    fn parse_layout_map_rejects_short_row() {
        let text = "lupos_path\tlinux_path\tnote\nonly_one_col\n";
        assert!(parse_layout_map(text).is_err());
    }

    #[test]
    fn parse_layout_exceptions_accepts_canonical_header() {
        let text = "lupos_path\treason\nsrc/x.rs\tnone\n";
        let rows = parse_layout_exceptions(text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lupos_path, "src/x.rs");
        assert_eq!(rows[0].reason, "none");
    }

    #[test]
    fn audit_clean_tree_has_no_findings() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        let report = run_audit(&repo).unwrap();
        assert!(
            report.is_clean(),
            "expected no findings, got: {:?}",
            report.entries
        );
        assert_eq!(report.map_rows, 1);
        assert_eq!(report.exception_rows, 2);
        assert_eq!(report.rs_files_scanned, 3);
    }

    #[test]
    fn audit_flags_missing_lupos_file() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        // Remove the mapped rust file.
        fs::remove_file(repo.join("src/arch/x86/boot/a20.rs")).unwrap();
        let report = run_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == AuditFinding::MissingLuposFile
                    && e.path == "src/arch/x86/boot/a20.rs"),
            "missing-lupos-file not flagged: {:?}",
            report.entries
        );
    }

    #[test]
    fn audit_flags_missing_linux_file() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        fs::remove_file(repo.join("vendor/linux/arch/x86/boot/a20.c")).unwrap();
        let report = run_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == AuditFinding::MissingLinuxFile
                    && e.path == "src/arch/x86/boot/a20.rs"),
            "missing-linux-file not flagged: {:?}",
            report.entries
        );
    }

    #[test]
    fn audit_flags_orphan_rs_file() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        // Add an unaccounted Rust file.
        write(&repo, "src/mm/orphan.rs", "// orphan\n");
        let report = run_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == AuditFinding::OrphanRsFile && e.path == "src/mm/orphan.rs"),
            "orphan not flagged: {:?}",
            report.entries
        );
    }

    #[test]
    fn audit_flags_path_in_map_and_exceptions() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        // Add a file present in both lists.
        write(&repo, "src/mm/dup.rs", "// dup\n");
        let map_text = "lupos_path\tlinux_path\tnote\n\
             src/arch/x86/boot/a20.rs\tvendor/linux/arch/x86/boot/a20.c\tlayout-oracle\n\
             src/mm/dup.rs\tvendor/linux/mm/dup.c\tlayout-oracle\n";
        let exc_text = "lupos_path\treason\n\
             src/arch/x86/lib/insn_eval.rs\tRust implementation without Linux counterpart\n\
             src/mm/dup.rs\tduplicate\n";
        write(&repo, LAYOUT_MAP_PATH, map_text);
        write(&repo, LAYOUT_EXCEPTIONS_PATH, exc_text);
        // Linux side for the dup row so we don't trip MissingLinuxFile.
        write(&repo, "vendor/linux/mm/dup.c", "/* c */\n");
        let report = run_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == AuditFinding::DuplicateMapping && e.path == "src/mm/dup.rs"),
            "duplicate not flagged: {:?}",
            report.entries
        );
    }

    #[test]
    fn audit_flags_exception_without_file() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        // Add an exception entry whose file does not exist on disk.
        let exc_text = "lupos_path\treason\n\
             src/arch/x86/lib/insn_eval.rs\tRust implementation without Linux counterpart\n\
             src/lib.rs\tCargo crate module index only; no runtime implementation logic\n\
             src/does/not/exist.rs\tphantom exception\n";
        write(&repo, LAYOUT_EXCEPTIONS_PATH, exc_text);
        let report = run_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == AuditFinding::MissingException
                    && e.path == "src/does/not/exist.rs"),
            "missing-exception not flagged: {:?}",
            report.entries
        );
    }

    /// Live drift gate — pulls the real repo and asserts the checked-in TSVs
    /// match reality.  This is the CI-facing check that enforces CLAUDE.md
    /// rule 2a (Linux Layout Parity Is Mandatory).
    #[test]
    fn linux_layout_map_is_self_consistent() {
        let repo = repo_root().expect("repo root");
        let report = run_audit(&repo).expect("audit ran");
        if !report.is_clean() {
            let mut out = String::from("layout drift:\n");
            for entry in &report.entries {
                out.push_str(&format!(
                    "  {} {} {}\n",
                    entry.finding.as_str(),
                    entry.path,
                    entry.detail
                ));
            }
            panic!("{out}");
        }
    }

    #[test]
    fn to_tsv_emits_header_and_findings() {
        let report = AuditReport {
            entries: vec![AuditEntry {
                finding: AuditFinding::OrphanRsFile,
                path: "src/x.rs".to_owned(),
                detail: "n/a".to_owned(),
            }],
            map_rows: 0,
            exception_rows: 0,
            rs_files_scanned: 1,
        };
        let tsv = report.to_tsv();
        assert!(tsv.starts_with("finding\tpath\tdetail\n"));
        assert!(tsv.contains("orphan_rs_file\tsrc/x.rs\tn/a"));
    }

    // -----------------------------------------------------------------------
    // audit-parity tests
    // -----------------------------------------------------------------------

    #[test]
    fn scan_parity_tag_picks_up_each_variant() {
        assert_eq!(
            scan_parity_tag("//! linux-parity: complete\n"),
            ParityTag::Complete
        );
        assert_eq!(
            scan_parity_tag("//! linux-parity: partial\n"),
            ParityTag::Partial
        );
        assert_eq!(scan_parity_tag("//! linux-parity: stub\n"), ParityTag::Stub);
    }

    #[test]
    fn scan_parity_tag_treats_garbage_as_missing() {
        assert_eq!(
            scan_parity_tag("//! linux-parity: wat\n"),
            ParityTag::Missing
        );
        assert_eq!(scan_parity_tag("//! some other note\n"), ParityTag::Missing);
        assert_eq!(scan_parity_tag(""), ParityTag::Missing);
    }

    #[test]
    fn scan_linux_source_tag_picks_up_source_path() {
        assert_eq!(
            scan_linux_source_tag(
                "//! linux-parity: partial\n//! linux-source: vendor/linux/mm/mmap.c\n"
            ),
            Some("vendor/linux/mm/mmap.c".to_string())
        );
        assert_eq!(scan_linux_source_tag("//! linux-parity: partial\n"), None);
    }

    #[test]
    fn scan_test_origin_tag_picks_up_file_header_origin() {
        assert_eq!(
            scan_test_origin_tag(
                "//! linux-parity: partial\n//! test-origin: linux:vendor/linux/mm/mmap.c\n"
            ),
            Some("linux:vendor/linux/mm/mmap.c".to_string())
        );
        assert_eq!(scan_test_origin_tag("//! no provenance\n"), None);
    }

    #[test]
    fn test_origin_audit_classifies_linux_and_lupos_specific() {
        let repo = tmp_repo();
        write(
            &repo,
            "src/mm/mmap.rs",
            "//! test-origin: linux:vendor/linux/mm/mmap.c\n#[cfg(test)] mod tests {}\n",
        );
        write(&repo, "vendor/linux/mm/mmap.c", "/* c */\n");
        write(
            &repo,
            "xtask/tests/boot.rs",
            "//! test-origin: lupos-specific:xtask boot argument parser\n#[test] fn t() {}\n",
        );
        let files = vec![
            PathBuf::from("src/mm/mmap.rs"),
            PathBuf::from("xtask/tests/boot.rs"),
        ];
        let report = audit_test_origins_for_files(&repo, &files).unwrap();
        assert!(
            report.is_clean(),
            "expected clean report: {:?}",
            report.entries
        );
        assert_eq!(report.linux, 1);
        assert_eq!(report.lupos_specific, 1);
        assert_eq!(report.unjustified, 0);
    }

    #[test]
    fn test_origin_audit_flags_missing_and_bad_linux_source() {
        let repo = tmp_repo();
        write(&repo, "src/mm/missing.rs", "#[test] fn missing() {}\n");
        write(
            &repo,
            "src/mm/bad.rs",
            "//! test-origin: linux:vendor/linux/no/such/file.c\n#[test] fn bad() {}\n",
        );
        let files = vec![
            PathBuf::from("src/mm/missing.rs"),
            PathBuf::from("src/mm/bad.rs"),
        ];
        let report = audit_test_origins_for_files(&repo, &files).unwrap();
        assert!(!report.is_clean());
        assert_eq!(report.unjustified, 2);
        assert!(
            report
                .entries
                .iter()
                .any(|entry| entry.kind == TestOriginKind::Missing)
        );
        assert!(
            report
                .entries
                .iter()
                .any(|entry| entry.kind == TestOriginKind::MissingLinuxSource)
        );
    }

    #[test]
    fn scan_parity_tag_ignores_tag_past_first_50_lines() {
        let mut text = "//! filler\n".repeat(60);
        text.push_str("//! linux-parity: complete\n");
        assert_eq!(scan_parity_tag(&text), ParityTag::Missing);
    }

    #[test]
    fn parity_fail_mode_partial_includes_stub_but_not_complete() {
        assert!(ParityFailMode::Partial.includes(ParityTag::Stub));
        assert!(ParityFailMode::Partial.includes(ParityTag::Partial));
        assert!(!ParityFailMode::Partial.includes(ParityTag::Complete));
        assert!(!ParityFailMode::Partial.includes(ParityTag::Missing));
    }

    #[test]
    fn parity_fail_mode_missing_includes_everything_below_complete() {
        for tag in [ParityTag::Missing, ParityTag::Stub, ParityTag::Partial] {
            assert!(ParityFailMode::Missing.includes(tag), "tag={tag:?}");
        }
        assert!(!ParityFailMode::Missing.includes(ParityTag::Complete));
    }

    #[test]
    fn parity_fail_mode_never_includes_nothing() {
        for tag in [
            ParityTag::Missing,
            ParityTag::Stub,
            ParityTag::Partial,
            ParityTag::Complete,
        ] {
            assert!(!ParityFailMode::Never.includes(tag), "tag={tag:?}");
        }
    }

    #[test]
    fn parity_fail_mode_parse_round_trip() {
        for arg in ["never", "stub", "partial", "missing"] {
            let mode = ParityFailMode::parse(arg).unwrap();
            assert_eq!(mode.as_arg(), arg);
        }
        assert!(ParityFailMode::parse("bogus").is_err());
    }

    #[test]
    fn parity_scope_parse_accepts_critical_runtime_scopes() {
        for arg in [
            "all",
            "critical-futex",
            "critical-time",
            "critical-task",
            "critical-fd-vfs",
            "critical-runtime",
            "video",
        ] {
            let scope = ParityScope::parse(arg).unwrap();
            assert_eq!(scope.as_arg(), arg);
        }
        assert!(ParityScope::parse("critical-drivers").is_err());
    }

    #[test]
    fn critical_runtime_scope_covers_only_runtime_closure_paths() {
        assert!(ParityScope::CriticalRuntime.includes_path("src/kernel/time/posix_clock.rs"));
        assert!(ParityScope::CriticalRuntime.includes_path("src/kernel/wait.rs"));
        assert!(ParityScope::CriticalRuntime.includes_path("src/fs/fdtable.rs"));
        assert!(ParityScope::CriticalRuntime.includes_path("src/net/syscalls.rs"));
        assert!(ParityScope::CriticalRuntime.includes_path("src/kernel/futex/core_ops.rs"));
        assert!(!ParityScope::CriticalRuntime.includes_path("src/kernel/bpf/syscall.rs"));
        assert!(!ParityScope::CriticalRuntime.includes_path("src/net/linux_sources.rs"));
    }

    #[test]
    fn video_scope_covers_boot_console_framebuffer_and_drm_paths() {
        for path in [
            "src/arch/x86/boot/video_vesa.rs",
            "src/arch/x86/boot/compressed/misc.rs",
            "src/arch/x86/boot/legacy.rs",
            "src/arch/x86/boot/main.rs",
            "src/arch/x86/entry/thunk.rs",
            "src/arch/x86/kernel/alternative.rs",
            "src/arch/x86/kernel/early_quirks.rs",
            "src/arch/x86/kernel/probe_roms.rs",
            "src/arch/x86/mm/init.rs",
            "src/arch/x86/realmode/rm/video_vga.rs",
            "src/arch/x86/realmode/rm/wakemain.rs",
            "src/arch/x86/video/mod.rs",
            "src/arch/x86/xen/mod.rs",
            "src/arch/x86/xen/vga.rs",
            "src/linux_driver_abi/video/fbdev/mod.rs",
            "src/linux_driver_abi/gpu/drm/mod.rs",
            "src/linux_driver_abi/pci/enumerate.rs",
            "src/linux_driver_abi/virtio/mod.rs",
            "src/init/main.rs",
            "src/init/rootfs.rs",
            "src/kernel/console.rs",
            "src/kernel/dma/mod.rs",
            "src/kernel/printk/log.rs",
            "src/kernel/module/loader.rs",
            "src/kernel/module/relocate.rs",
            "src/fs/ops.rs",
            "src/fs/sysfs/mount.rs",
            "src/io_uring/mod.rs",
            "src/mm/fault.rs",
            "src/mm/mmap.rs",
            "src/mm/mm_init.rs",
            "src/mm/pgprot.rs",
            "src/mm/shmem.rs",
            "src/mm/vma.rs",
            "src/rust/helpers/drm.rs",
        ] {
            assert!(ParityScope::Video.includes_path(path), "{path}");
        }
        assert!(!ParityScope::Video.includes_path("src/net/ipv4/tcp.rs"));
    }

    #[test]
    fn parity_audit_counts_tags_against_layout_map_only() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        // a20.rs declares complete; eval.rs is an exception (not scanned).
        write(
            &repo,
            "src/arch/x86/boot/a20.rs",
            "//! linux-parity: complete\n//! linux-source: vendor/linux/arch/x86/boot/a20.c\n// body\n",
        );
        let report = run_parity_audit(&repo).unwrap();
        assert_eq!(report.complete, 1);
        assert_eq!(report.partial, 0);
        assert_eq!(report.stub, 0);
        assert_eq!(report.missing, 0);
        assert_eq!(report.missing_source, 0);
        assert_eq!(report.source_mismatch, 0);
        assert!(report.tag_presence_failures(ParityScope::All).is_empty());
        assert!(
            report
                .failures(ParityFailMode::Stub, ParityScope::All)
                .is_empty()
        );
        assert!(
            report
                .failures(ParityFailMode::Missing, ParityScope::All)
                .is_empty()
        );
    }

    #[test]
    fn parity_audit_flags_missing_and_stub() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        // a20.rs declares stub.
        write(
            &repo,
            "src/arch/x86/boot/a20.rs",
            "//! linux-parity: stub\n//! linux-source: vendor/linux/not/a20.c\n// body\n",
        );
        // Add another mapped file with no tag at all.
        write(&repo, "src/mm/page.rs", "// no tag\n");
        write(&repo, "vendor/linux/mm/page.c", "/* c */\n");
        let map_text = "lupos_path\tlinux_path\tnote\n\
             src/arch/x86/boot/a20.rs\tvendor/linux/arch/x86/boot/a20.c\tlayout-oracle\n\
             src/mm/page.rs\tvendor/linux/mm/page.c\tlayout-oracle\n";
        write(&repo, LAYOUT_MAP_PATH, map_text);

        let report = run_parity_audit(&repo).unwrap();
        assert_eq!(report.stub, 1);
        assert_eq!(report.missing, 1);
        assert_eq!(report.missing_source, 1);
        assert_eq!(report.source_mismatch, 1);
        assert_eq!(report.tag_presence_failures(ParityScope::All).len(), 2);
        assert_eq!(
            report
                .failures(ParityFailMode::Stub, ParityScope::All)
                .len(),
            1
        );
        assert_eq!(
            report
                .failures(ParityFailMode::Missing, ParityScope::All)
                .len(),
            2
        );
        assert!(
            report
                .failures(ParityFailMode::Never, ParityScope::All)
                .is_empty()
        );
    }

    #[test]
    fn linux_layout_mapped_files_have_parity_headers() {
        let repo = crate::repo_root().unwrap();
        let report = run_parity_audit(&repo).unwrap();
        assert_eq!(report.missing, 0, "mapped Rust files missing linux-parity");
        assert_eq!(
            report.missing_source, 0,
            "mapped Rust files missing linux-source"
        );
        assert_eq!(
            report.source_mismatch, 0,
            "mapped Rust files with linux-source not matching layout map"
        );
    }

    // -----------------------------------------------------------------------
    // audit-mm-symbols tests
    // -----------------------------------------------------------------------

    #[test]
    fn mm_symbol_parser_extracts_syscalls_exports_and_headers() {
        assert_eq!(
            extract_syscall_symbols("SYSCALL_DEFINE1(brk, unsigned long, brk)"),
            vec!["brk".to_string()]
        );
        assert_eq!(
            extract_export_symbols("EXPORT_SYMBOL_GPL(filemap_read);"),
            vec!["filemap_read".to_string()]
        );
        assert_eq!(
            extract_header_fn_symbol("extern unsigned long do_mmap(struct file *file);"),
            Some(("do_mmap".to_string(), MmLinuxSymbolKind::Prototype))
        );
        assert_eq!(
            extract_header_fn_symbol("static inline bool folio_test_dirty(struct folio *folio)"),
            Some(("folio_test_dirty".to_string(), MmLinuxSymbolKind::Inline))
        );
    }

    #[test]
    fn mm_rust_symbol_parser_tracks_visibility() {
        assert_eq!(
            extract_rust_fn_symbol("pub unsafe fn mmap_pgoff(addr: u64) -> i64 { 0 }"),
            Some((RustVisibility::Public, "mmap_pgoff".to_string()))
        );
        assert_eq!(
            extract_rust_fn_symbol("pub(crate) fn do_mmap() {}"),
            Some((RustVisibility::Crate, "do_mmap".to_string()))
        );
        assert_eq!(
            extract_rust_fn_symbol("fn helper() {}"),
            Some((RustVisibility::Private, "helper".to_string()))
        );
    }

    #[test]
    fn mm_symbol_audit_matches_exact_names_and_flags_missing() {
        let repo = tmp_repo();
        write(
            &repo,
            "vendor/linux/mm/mmap.c",
            "SYSCALL_DEFINE1(brk, unsigned long, brk)\nEXPORT_SYMBOL_GPL(do_mmap);\n",
        );
        write(
            &repo,
            "vendor/linux/include/linux/mm.h",
            "extern unsigned long do_mmap(struct file *file);\nextern int missing_mm_symbol(void);\n",
        );
        write(
            &repo,
            "src/mm/mmap.rs",
            "//! linux-parity: complete\n//! linux-source: vendor/linux/mm/mmap.c\npub unsafe fn brk(_: u64) -> i64 { 0 }\npub unsafe fn do_mmap() -> i64 { 0 }\n",
        );

        let report = run_mm_symbol_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|entry| entry.linux.name == "brk" && entry.status == MmSymbolStatus::Complete),
            "brk should be complete: {:?}",
            report.entries
        );
        assert!(
            report
                .entries
                .iter()
                .any(|entry| entry.linux.name == "missing_mm_symbol"
                    && entry.status == MmSymbolStatus::Missing),
            "missing symbol should be reported: {:?}",
            report.entries
        );
        assert_eq!(report.failures(MmSymbolFailMode::Missing).len(), 1);
    }

    #[test]
    fn mm_symbol_fail_modes_are_ordered_by_strictness() {
        assert!(MmSymbolFailMode::Missing.includes(MmSymbolStatus::Missing));
        assert!(!MmSymbolFailMode::Missing.includes(MmSymbolStatus::Stub));
        assert!(MmSymbolFailMode::Stub.includes(MmSymbolStatus::Missing));
        assert!(MmSymbolFailMode::Stub.includes(MmSymbolStatus::Stub));
        assert!(MmSymbolFailMode::Partial.includes(MmSymbolStatus::Partial));
        assert!(!MmSymbolFailMode::Never.includes(MmSymbolStatus::Missing));
        for arg in ["never", "missing", "stub", "partial"] {
            assert_eq!(MmSymbolFailMode::parse(arg).unwrap().as_arg(), arg);
        }
    }

    // -----------------------------------------------------------------------
    // audit-kunit tests
    // -----------------------------------------------------------------------

    #[test]
    fn scan_kunit_cases_extracts_suite_name_source_run() {
        let text = r#"
            const CASES: &[KunitCase] = &[
                KunitCase {
                    suite: "lib.packing",
                    name: "packing_pack_fields",
                    source: "vendor/linux/lib/packing_test.c",
                    run: lib_arithmetic_and_ordering,
                },
            ];
        "#;
        let cases = scan_kunit_cases("src/kernel/kunit.rs", text);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].suite, "lib.packing");
        assert_eq!(cases[0].name, "packing_pack_fields");
        assert_eq!(cases[0].source, "vendor/linux/lib/packing_test.c");
        assert_eq!(cases[0].run_fn, "lib_arithmetic_and_ordering");
    }

    #[test]
    fn count_fn_statements_handles_placeholder_body() {
        let text = r#"
            fn placeholder() -> bool {
                crate::kunit_expect!(1 + 1 == 2);
                crate::kunit_expect!(3 > 2);
                true
            }
        "#;
        assert_eq!(count_fn_statements(text, "placeholder"), Some(3));
    }

    #[test]
    fn count_fn_statements_returns_none_for_missing_fn() {
        assert_eq!(
            count_fn_statements("fn other() -> bool { true }", "x"),
            None
        );
    }

    #[test]
    fn count_fn_statements_ignores_blank_and_brace_only_lines() {
        let text = "fn body() -> bool {\n\n    let x = 1;\n\n    true\n}\n";
        assert_eq!(count_fn_statements(text, "body"), Some(2));
    }

    #[test]
    fn kunit_audit_flags_low_statement_count() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        write(&repo, "vendor/linux/lib/packing_test.c", "/* c */\n");
        let kunit_text = r#"
            //! header
            const CASES: &[KunitCase] = &[
                KunitCase {
                    suite: "lib.packing",
                    name: "placeholder",
                    source: "vendor/linux/lib/packing_test.c",
                    run: tiny,
                },
            ];

            fn tiny() -> bool {
                true
            }
        "#;
        write(&repo, "src/kernel/kunit.rs", kunit_text);

        let report = run_kunit_audit(&repo).unwrap();
        assert_eq!(report.cases_scanned, 1);
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == KunitFinding::LowStatementCount && e.case.run_fn == "tiny"),
            "low-statement-count not flagged: {:?}",
            report.entries
        );
    }

    #[test]
    fn kunit_audit_flags_missing_source() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        let kunit_text = r#"
            const CASES: &[KunitCase] = &[
                KunitCase {
                    suite: "x",
                    name: "y",
                    source: "vendor/linux/no/such/file.c",
                    run: realish,
                },
            ];

            fn realish() -> bool {
                let a = 1;
                let b = 2;
                let c = 3;
                let d = 4;
                a + b + c + d == 10
            }
        "#;
        write(&repo, "src/kernel/kunit.rs", kunit_text);
        let report = run_kunit_audit(&repo).unwrap();
        assert!(
            report
                .entries
                .iter()
                .any(|e| e.finding == KunitFinding::MissingSource),
            "missing-source not flagged: {:?}",
            report.entries
        );
    }

    #[test]
    fn kunit_audit_clean_when_real_body_and_real_source() {
        let repo = tmp_repo();
        write_basic_tree(&repo);
        write(&repo, "vendor/linux/lib/packing_test.c", "/* c */\n");
        let kunit_text = r#"
            const CASES: &[KunitCase] = &[
                KunitCase {
                    suite: "lib.packing",
                    name: "real_case",
                    source: "vendor/linux/lib/packing_test.c",
                    run: real_case,
                },
            ];

            fn real_case() -> bool {
                let a = 1;
                let b = 2;
                let c = 3;
                let d = 4;
                let e = 5;
                a + b + c + d + e == 15
            }
        "#;
        write(&repo, "src/kernel/kunit.rs", kunit_text);
        let report = run_kunit_audit(&repo).unwrap();
        assert!(
            report.is_clean(),
            "expected clean audit, got: {:?}",
            report.entries
        );
    }

    #[test]
    fn count_sloc_skips_blanks_and_comments() {
        let text = "// header\n\nfn a() {\n    // inline\n    let x = 1;\n}\n/* block\n still */\nlet y = 2;\n";
        // counted: `fn a() {`, `let x = 1;`, `}`, `let y = 2;` = 4
        assert_eq!(count_sloc(text), 4);
    }

    #[test]
    fn parity_table_excludes_inline_test_modules_from_lupos_sloc() {
        let repo = tmp_repo();
        write(
            &repo,
            "src/fs/fs_struct.rs",
            "//! linux-parity: partial\n\
             pub fn live() {}\n\
             #[cfg(test)]\n\
             mod tests {\n\
                 #[test]\n\
                 fn inflated() {\n\
                     let a = 1;\n\
                     let b = 2;\n\
                     assert_eq!(a + b, 3);\n\
                 }\n\
             }\n",
        );
        write(&repo, "vendor/linux/fs/fs_struct.c", "void live(void) {}\n");
        write(
            &repo,
            LAYOUT_MAP_PATH,
            "lupos_path\tlinux_path\tnote\n\
             src/fs/fs_struct.rs\tvendor/linux/fs/fs_struct.c\tlayout-oracle\n",
        );

        let rows = run_parity_table(&repo).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].lup_sloc, 1);
        assert_eq!(rows[0].lnx_sloc, 1);
        assert_eq!(rows[0].parity, 100);
    }

    #[test]
    fn subsystem_splits_arch_x86_deeper() {
        assert_eq!(subsystem_of("src/arch/x86/kvm/x86.rs"), "arch/x86/kvm");
        assert_eq!(subsystem_of("src/arch/x86/mm/fault.rs"), "arch/x86/mm");
        assert_eq!(subsystem_of("src/mm/fork.rs"), "mm");
        assert_eq!(subsystem_of("src/kernel/sched/fair.rs"), "kernel");
        assert_eq!(subsystem_of("src/lib.rs"), "(root)");
    }

    #[test]
    fn parity_row_flags_overclaim_and_out_of_scope() {
        let repo = tmp_repo();
        // Tiny lupos file tagged complete vs a large Linux file -> over-claim.
        write(
            &repo,
            "src/arch/x86/kvm/x86.rs",
            "//! linux-parity: complete\nfn a() {}\n",
        );
        let mut big = String::from("/* c */\n");
        for i in 0..500 {
            big.push_str(&format!("int v{i} = {i};\n"));
        }
        write(&repo, "vendor/linux/arch/x86/kvm/x86.c", &big);
        write(
            &repo,
            LAYOUT_MAP_PATH,
            "lupos_path\tlinux_path\tnote\n\
             src/arch/x86/kvm/x86.rs\tvendor/linux/arch/x86/kvm/x86.c\tlayout-oracle\n",
        );
        let rows = run_parity_table(&repo).unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert!(r.parity < 10, "expected tiny parity, got {}", r.parity);
        assert!(r.out_of_scope, "kvm/ must be out of scope");
        assert_eq!(r.mismatch, Some("over"));
        assert_eq!(r.tag, ParityTag::Complete);
        assert!(r.todos.contains("out of scope"));
    }

    #[test]
    fn parse_defconfig_handles_set_and_unset_symbols() {
        let map = parse_defconfig(
            "# a comment\n\
             CONFIG_SMP=y\n\
             CONFIG_VIRTIO_NET=m\n\
             # CONFIG_DEBUG_KERNEL is not set\n\
             CONFIG_CMDLINE=\"console=ttyS0\"\n\
             \n\
             NOT_A_CONFIG=ignored\n",
        );
        assert_eq!(map.get("CONFIG_SMP"), Some(&"y".to_owned()));
        assert_eq!(map.get("CONFIG_VIRTIO_NET"), Some(&"m".to_owned()));
        assert_eq!(map.get("CONFIG_DEBUG_KERNEL"), Some(&"n".to_owned()));
        assert_eq!(
            map.get("CONFIG_CMDLINE"),
            Some(&"\"console=ttyS0\"".to_owned())
        );
        assert_eq!(map.get("NOT_A_CONFIG"), None);
    }

    #[test]
    fn config_parity_passes_on_matches_and_module_overrides() {
        let lupos = "CONFIG_SMP=y\n\
                     CONFIG_NET=y\n\
                     CONFIG_VIRTIO_NET=m\n\
                     CONFIG_ACPI=y\n"; // ACPI absent upstream → out of overlap, ignored
        let linux = "CONFIG_SMP=y\n\
                     CONFIG_NET=y\n\
                     CONFIG_VIRTIO_NET=y\n";
        let report = audit_config_parity(lupos, linux);
        assert!(report.is_clean());
        assert_eq!(report.matched, 2);
        assert_eq!(report.module_overrides, 1);
        assert_eq!(report.divergences, 0);
        // The overlap excludes ACPI (upstream does not pin it).
        assert_eq!(report.entries.len(), 3);
    }

    #[test]
    fn config_parity_fails_on_value_drift() {
        // Local config disables a symbol upstream enables, and demotes a non-y
        // upstream value — both are real drift, not the sanctioned y→m policy.
        let lupos = "CONFIG_SMP=y\n\
                     # CONFIG_NET is not set\n\
                     CONFIG_HZ=250\n";
        let linux = "CONFIG_SMP=y\n\
                     CONFIG_NET=y\n\
                     CONFIG_HZ=1000\n";
        let report = audit_config_parity(lupos, linux);
        assert!(!report.is_clean());
        assert_eq!(report.divergences, 2);
        let drift: Vec<&str> = report
            .divergent_entries()
            .map(|e| e.symbol.as_str())
            .collect();
        assert!(drift.contains(&"CONFIG_NET"));
        assert!(drift.contains(&"CONFIG_HZ"));
    }

    #[test]
    fn config_parity_requires_upstream_generic_video_symbols_locally() {
        let lupos = "CONFIG_DRM=y\n";
        let linux = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=y\n\
                     CONFIG_DRM_VIRTIO_GPU=y\n";
        let report = audit_config_parity(lupos, linux);

        assert!(!report.is_clean());
        assert_eq!(report.matched, 1);
        assert_eq!(report.divergences, 2);
        for symbol in ["CONFIG_DRM_I915", "CONFIG_DRM_VIRTIO_GPU"] {
            let entry = report
                .divergent_entries()
                .find(|entry| entry.symbol == symbol)
                .unwrap_or_else(|| panic!("missing required-video finding for {symbol}"));
            assert_eq!(entry.lupos_value, "<missing>");
            assert_eq!(entry.linux_value, "y");
        }
    }

    #[test]
    fn config_parity_accepts_declared_video_driver_module_overrides() {
        let lupos = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=m\n\
                     CONFIG_DRM_VIRTIO_GPU=m\n";
        let linux = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=y\n\
                     CONFIG_DRM_VIRTIO_GPU=y\n";
        let report = audit_config_parity(lupos, linux);

        assert!(report.is_clean());
        assert_eq!(report.matched, 1);
        assert_eq!(report.module_overrides, 2);
        assert_eq!(report.divergences, 0);
    }

    #[test]
    fn config_parity_requires_generic_video_kconfig_declarations() {
        let lupos = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=m\n\
                     CONFIG_DRM_VIRTIO_GPU=m\n";
        let linux = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=y\n\
                     CONFIG_DRM_VIRTIO_GPU=y\n";
        let kconfig = "config DRM\n\
                       bool \"Direct Rendering Manager support\"\n";
        let mut report = audit_config_parity(lupos, linux);
        audit_required_video_kconfig_symbols(&mut report, kconfig, linux);

        assert!(!report.is_clean());
        assert_eq!(report.matched, 1);
        assert_eq!(report.module_overrides, 0);
        assert_eq!(report.divergences, 2);
        for symbol in ["CONFIG_DRM_I915", "CONFIG_DRM_VIRTIO_GPU"] {
            let entry = report
                .divergent_entries()
                .find(|entry| entry.symbol == symbol)
                .unwrap_or_else(|| panic!("missing Kconfig-declaration finding for {symbol}"));
            assert_eq!(entry.lupos_value, "<missing-kconfig>");
        }
    }

    #[test]
    fn config_parity_rejects_undeclared_y_to_m_demotions() {
        let lupos = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=m\n\
                     CONFIG_DRM_VIRTIO_GPU=m\n\
                     CONFIG_UNDECLARED_DRIVER=m\n";
        let linux = "CONFIG_DRM=y\n\
                     CONFIG_DRM_I915=y\n\
                     CONFIG_DRM_VIRTIO_GPU=y\n\
                     CONFIG_UNDECLARED_DRIVER=y\n";
        let report = audit_config_parity(lupos, linux);

        assert!(!report.is_clean());
        assert_eq!(report.module_overrides, 2);
        assert_eq!(report.divergences, 1);
        assert_eq!(
            report
                .divergent_entries()
                .next()
                .map(|entry| entry.symbol.as_str()),
            Some("CONFIG_UNDECLARED_DRIVER")
        );
    }

    #[test]
    fn config_parity_tracks_real_repo_defconfigs() {
        // The shipped lupos_defconfig must not drift from upstream for the
        // overlapping symbol set.  Skip when vendor/linux is not populated.
        let repo = repo_root().expect("repo root");
        if !repo.join(LINUX_X86_64_DEFCONFIG_PATH).exists() {
            return;
        }
        let report = run_config_parity_audit(&repo).expect("config parity audit");
        assert!(
            report.is_clean(),
            "lupos_defconfig diverges from upstream: {:?}",
            report.divergent_entries().collect::<Vec<_>>()
        );
    }
}
