//! test-origin: lupos-specific:xtask repository layout integration tests
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has repo parent")
        .to_path_buf()
}

fn walk_files(root: &Path, out: &mut Vec<PathBuf>) {
    let skip_dirs = [".claude", ".git", "target", "vendor"];
    for entry in fs::read_dir(root).expect("read directory") {
        let entry = entry.expect("read directory entry");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if skip_dirs.contains(&name.as_ref()) {
                continue;
            }
            walk_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn is_scanned_source(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if matches!(
        name,
        "Cargo.toml" | "Cargo.lock" | "Makefile" | ".gitignore"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs" | "toml" | "json" | "S" | "tbl" | "sh" | "ps1")
    )
}

#[test]
fn required_linux_roots_exist() {
    let root = repo_root();
    for rel in [
        "src/arch",
        "src/block",
        "src/linux_driver_abi",
        "src/fs",
        "src/include",
        "src/init",
        "src/io_uring",
        "src/ipc",
        "src/kernel",
        "src/lib",
        "src/mm",
        "src/net",
        "src/security",
        "src/usr",
    ] {
        assert!(root.join(rel).is_dir(), "missing Linux root {rel}");
    }
}

#[test]
fn root_contains_only_allowed_visible_entries() {
    let root = repo_root();
    let allowed_dirs = [
        "branding", "configs", "scripts", "src", "target", "vendor", "xtask",
    ];
    let allowed_files = [
        ".config",
        ".config.old",
        ".editorconfig",
        ".gitattributes",
        ".gitignore",
        "AGENTS.MD",
        "build.rs",
        "Cargo.lock",
        "Cargo.toml",
        "CLAUDE.md",
        "FAQ.md",
        "Makefile",
        "README.md",
        "ROADMAP.md",
        "rust-toolchain.toml",
        "x86_64_every_kernel_file_table.md",
        "x86_64-lupos.json",
    ];

    for entry in fs::read_dir(&root).expect("read repo root") {
        let entry = entry.expect("read root entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && entry.path().is_dir() {
            continue;
        }
        let allowed = if entry.path().is_dir() {
            allowed_dirs.contains(&name.as_str())
        } else {
            allowed_files.contains(&name.as_str())
        };
        assert!(
            allowed,
            "unexpected visible repo-root entry after src/ consolidation: {name}"
        );
    }
}

#[test]
fn old_userland_source_root_is_absent() {
    let root = repo_root();
    assert!(
        !root.join("userland").exists(),
        "top-level userland/ source root must be usr/"
    );
}

#[test]
fn old_flattened_paths_are_absent() {
    let root = repo_root();
    let mut files = Vec::new();
    walk_files(&root, &mut files);

    let forbidden = [
        format!("crate::{}", "memory"),
        format!("crate::{}", "uapi"),
        format!("{}/{}", "src", "memory"),
        format!("{}/{}", "src", "uapi"),
        format!("{}/{}/{}", "src", "kernel", "driver"),
        format!("{}/{}/{}", "src", "kernel", "security"),
        format!("{}/{}", "userland", "init"),
    ];

    for path in files.into_iter().filter(|p| is_scanned_source(p)) {
        let text = fs::read_to_string(&path).unwrap_or_default();
        for needle in &forbidden {
            assert!(
                !text.contains(needle),
                "{} still references old layout path `{needle}`",
                path.strip_prefix(&root).unwrap_or(&path).display()
            );
        }
    }
}

/// The generated layout TSVs were retired; `xtask/src/audit.rs` now derives
/// the same mapping from each file's `//! linux-source:` header and
/// `cargo xtask test` enforces it via `audit-layout`.  The TSV shape checks
/// below only apply to checkouts that still carry the generated files.
fn layout_map_tsv(root: &Path) -> Option<String> {
    fs::read_to_string(root.join("src/docs/linux-layout-map.tsv")).ok()
}

#[test]
fn linux_layout_map_paths_exist() {
    let root = repo_root();
    let Some(map) = layout_map_tsv(&root) else {
        return;
    };
    for (idx, line) in map.lines().enumerate() {
        let line = line.trim_start_matches('\u{feff}');
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "bad map row {}: {line}", idx + 1);
        assert!(
            root.join(fields[0]).exists(),
            "map row {} references missing Lupos path {}",
            idx + 1,
            fields[0]
        );
        assert!(
            root.join(fields[1]).exists(),
            "map row {} references missing Linux path {}",
            idx + 1,
            fields[1]
        );
    }

    let exceptions_path = root.join("src/docs/linux-layout-exceptions.tsv");
    let exceptions = fs::read_to_string(&exceptions_path).expect("read linux layout exceptions");
    for (idx, line) in exceptions.lines().enumerate() {
        let line = line.trim_start_matches('\u{feff}');
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 2, "bad exception row {}: {line}", idx + 1);
        assert!(
            root.join(fields[0]).exists(),
            "exception row {} references missing Lupos path {}",
            idx + 1,
            fields[0]
        );
        assert!(
            !fields[1].trim().is_empty(),
            "exception row {} must explain the layout exception",
            idx + 1
        );
        const ALLOWED_PREFIXES: &[&str] = &[
            "build-glue:",
            "crate-index:",
            "cargo-config:",
            "vcs-config:",
            "cargo-lock:",
            "cargo-manifest:",
            "userspace-payload:",
            "pending-removal:",
            "pending-migration:",
        ];
        assert!(
            ALLOWED_PREFIXES
                .iter()
                .any(|p| fields[1].trim_start().starts_with(p)),
            "exception row {} reason must start with a known category prefix; got: {}",
            idx + 1,
            fields[1]
        );
    }
}

#[test]
fn linux_layout_c_mappings_are_complete_translations() {
    let root = repo_root();
    let Some(map) = layout_map_tsv(&root) else {
        return;
    };
    let mut bad = Vec::new();

    for (idx, line) in map.lines().enumerate() {
        let line = line.trim_start_matches('\u{feff}');
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "bad map row {}: {line}", idx + 1);
        if !fields[1].ends_with(".c") {
            continue;
        }
        let lupos_path = root.join(fields[0]);
        let text = fs::read_to_string(&lupos_path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", fields[0]));
        let header = text.lines().next().unwrap_or_default().trim();
        if header != "//! linux-parity: complete" {
            bad.push(format!("{} -> {} ({header})", fields[0], fields[1]));
        }
    }

    assert!(
        bad.is_empty(),
        "Linux .c layout mappings must be complete 1:1 translations; move non-complete rows to linux-layout-exceptions.tsv until they are complete:\n  {}",
        bad.join("\n  ")
    );
}

#[test]
fn syscall_64_tbl_matches_vendor_linux() {
    let root = repo_root();
    let lupos = fs::read_to_string(root.join("src/arch/x86/entry/syscalls/syscall_64.tbl"))
        .expect("read Lupos syscall_64.tbl");
    let linux =
        fs::read_to_string(root.join("vendor/linux/arch/x86/entry/syscalls/syscall_64.tbl"))
            .expect("read Linux syscall_64.tbl");
    assert_eq!(
        lupos.replace("\r\n", "\n"),
        linux.replace("\r\n", "\n"),
        "x86_64 syscall table must match vendor/linux"
    );
}

#[test]
fn every_source_file_is_categorized() {
    use std::collections::HashSet;
    let root = repo_root();
    if layout_map_tsv(&root).is_none() {
        return;
    }
    let mut files = Vec::new();
    walk_files(&root.join("src"), &mut files);
    files.push(root.join("build.rs"));

    let mut categorized: HashSet<PathBuf> = HashSet::new();
    for tsv in [
        "src/docs/linux-layout-map.tsv",
        "src/docs/linux-layout-exceptions.tsv",
    ] {
        let text = fs::read_to_string(root.join(tsv)).expect("read tsv");
        for (idx, line) in text.lines().enumerate() {
            let line = line.trim_start_matches('\u{feff}');
            if idx == 0 || line.trim().is_empty() {
                continue;
            }
            let lupos = line.split('\t').next().expect("lupos field");
            categorized.insert(root.join(lupos));
        }
    }

    let mut missing = Vec::new();
    for path in files.into_iter().filter(|p| is_scanned_source(p)) {
        if !categorized.contains(&path) {
            let rel = path.strip_prefix(&root).unwrap_or(&path).to_path_buf();
            missing.push(rel);
        }
    }
    assert!(
        missing.is_empty(),
        "{} source file(s) not in layout-map or exceptions:\n  {}",
        missing.len(),
        missing
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
