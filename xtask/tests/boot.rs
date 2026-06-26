//! test-origin: lupos-specific:xtask QEMU boot integration tests
use std::{collections::HashSet, sync::Mutex, time::Duration};

use anyhow::Result;

use xtask::{
    ABI_PARITY_MILESTONES, AbiParityAcceptanceCheckKind, abi_parity_acceptance_check_kind,
    abi_parity_boot_mode_from_acceptance_check,
};
use xtask::{
    ANON_MMAP_BANNER, BUDDY_BANNER, BootMode, COW_FORK_BANNER, CREDENTIALS_BANNER,
    EXIT_WAIT_PTRACE_BANNER, HELLO_BANNER, IDT_PF_BANNER, INITRAMFS_ROOTFS_BANNER,
    MM_SELFTESTS_BANNER, NAMESPACES_BANNER, PAGE_CACHE_BANNER, PANIC_PREFIX, PID1_HANDOFF_BANNER,
    PTRACE_SECCOMP_SELFTESTS_BANNER, QEMU_FAILURE_EXIT_CODE, QEMU_SUCCESS_EXIT_CODE, RunOptions,
    SLAB_BANNER, VMCORE_BANNER, assert_boot_outcome,
    build_and_run_iso as build_and_run_iso_unlocked,
};

const BOOT_TEST_TIMEOUT_SECS: u64 = 120;
static QEMU_BOOT_TEST_MUTEX: Mutex<()> = Mutex::new(());

fn build_and_run_iso(mode: BootMode, options: RunOptions) -> Result<xtask::BootRun> {
    let _guard = QEMU_BOOT_TEST_MUTEX.lock().unwrap();
    build_and_run_iso_unlocked(mode, options)
}

#[test]
fn hello_world_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::Hello,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, HELLO_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

#[test]
fn panic_path_reports_failure_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::Panic,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, PANIC_PREFIX, QEMU_FAILURE_EXIT_CODE)
}

/// Verify that the IDT correctly catches a deliberate #PF
/// and that the handler logs the expected CR2 address to serial.
///
/// The kernel boots with `test-page-fault` + `qemu-test` features, triggers a
/// page fault at 0xDEADC0DEDEADC0DE, and the handler exits with success code.
#[test]
fn idt_catches_page_fault_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::IdtTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, IDT_PF_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Verify that the buddy allocator can allocate and free
/// pages at various orders and that the free count is preserved round-trip.
#[test]
fn buddy_allocator_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::BuddyTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, BUDDY_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Verify that the slab allocator handles 10 000 small-object
/// allocations without pointer overlap and exits cleanly.
#[test]
fn slab_test_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::SlabTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, SLAB_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Verify that kmap/kunmap maps a physical page, writes a
/// sentinel, unmaps it, and observes the value still present in RAM.
#[test]
fn vmcore_test_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::VmCoreTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, VMCORE_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Verify that do_mmap / do_munmap / mprotect / brk /
/// MAP_FIXED_NOREPLACE all work end-to-end and that the correct banner appears.
#[test]
fn anon_mmap_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::AnonMmapTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, ANON_MMAP_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Ported Linux mm selftest suite
/// (map_fixed_noreplace, mremap_dontunmap, mprotect-fault, madv_populate,
/// map_hugetlb stub).  Matches `cargo xtask test-boot --mode mm-selftests`.
#[test]
fn mm_selftests_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::MmSelftests,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, MM_SELFTESTS_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// COW/fork correctness
/// (dup_mm, copy_page_range, wp_page_reuse, wp_page_copy, smaps dirty accounting).
/// Matches `cargo xtask test-boot --mode cow-fork`.
#[test]
fn cow_fork_test_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::CowForkTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, COW_FORK_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Page cache read/write round-trip
/// (AddressSpace, XArray, filemap_read/write, readahead subsystem).
/// Matches `cargo xtask test-boot --mode page-cache`.
#[test]
fn page_cache_test_boots_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::PageCacheTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, PAGE_CACHE_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Exit / wait4 / waitid / zombies / ptrace.
/// Matches `cargo xtask test-boot --mode test-exit-wait-ptrace`.
#[test]
fn exit_wait_ptrace_runs_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::ExitWaitPtraceTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, EXIT_WAIT_PTRACE_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Credentials + capabilities + seccomp (cBPF).
/// Matches `cargo xtask test-boot --mode credentials`.
#[test]
fn credentials_runs_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::CredentialsTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, CREDENTIALS_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

#[test]
fn ptrace_seccomp_selftests_run_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::PtraceSeccompSelftestsTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(
        &run,
        PTRACE_SECCOMP_SELFTESTS_BANNER,
        QEMU_SUCCESS_EXIT_CODE,
    )
}

/// Namespaces (uts/pid/ipc/mnt/user/net/cgroup).
/// Matches `cargo xtask test-boot --mode namespaces`.
#[test]
fn namespaces_runs_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::NamespacesTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, NAMESPACES_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

#[test]
fn initramfs_rootfs_bootstrap_runs_in_qemu() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::InitramfsRootfsTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, INITRAMFS_ROOTFS_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

#[test]
fn pid1_handoff_smoke_uses_pid1_transcript_fixture() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::Pid1HandoffTest,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, PID1_HANDOFF_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

/// Regression gate for the loader double-bias bug.
///
/// Boots the full glibc + systemd userland through `BootMode::Login` and
/// asserts that the dynamic loader bootstrap completes without raising a
/// general-protection fault, and that PID1 reaches the `lupos login:`
/// banner on the serial console.
///
/// Marked `#[ignore]` because it requires the staged userland under
/// `target/userland/stage` (built via `make userland` / `cargo xtask
/// userland-build`). Run with:
///
/// ```text
/// cargo +nightly test -p xtask --test boot -- --ignored userland_ldso
/// ```
///
/// This is the regression gate for the fixed `DT_RELR` double-bias failure
/// where ld.so previously raised `#GP` at `rip=0x00007fff_0001_e0d3` with
/// `rcx=0x0000fffe_00000ca8` (`2 * INTERP_LOAD_BIAS + 0xca8`). See
/// `.claude/plans/implemented-the-repo-layout-correction-goofy-otter.md`.
#[test]
#[ignore = "requires staged userland (target/userland/stage)"]
fn userland_ldso_bootstraps_without_general_protection() -> Result<()> {
    // Force the Login mode initramfs to bundle the staged real glibc/systemd
    // userland from target/userland/stage instead of the synthetic
    // pid1-handoff payload. Without this the boot exercises a 1-segment
    // static-pie Lupos test binary and never reaches ld.so.
    // SAFETY: single-threaded test, env touched only here.
    unsafe { std::env::set_var("LUPOS_STAGE_REAL_USERLAND", "1") };

    let run = build_and_run_iso(
        BootMode::Login,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(300)),
            smp_count: 1,
        },
    )?;

    assert!(
        !run.serial_output.contains("cpu: #GP General Protection"),
        "ld.so bootstrap raised #GP; serial log: {}",
        run.artifacts.serial_log.display(),
    );
    assert!(
        run.serial_output.contains("lupos login:"),
        "ld.so bootstrap did not reach the login prompt; serial log: {}",
        run.artifacts.serial_log.display(),
    );

    Ok(())
}

#[test]
fn grub_bzimage_smoke_uses_grub_boot_fixture() -> Result<()> {
    let run = build_and_run_iso(
        BootMode::Hello,
        RunOptions {
            exit_after_boot: true,
            qemu_timeout: Some(Duration::from_secs(BOOT_TEST_TIMEOUT_SECS)),
            smp_count: 1,
        },
    )?;
    assert_boot_outcome(&run, HELLO_BANNER, QEMU_SUCCESS_EXIT_CODE)
}

#[test]
fn abi_parity_acceptance_checks_have_supported_actions() -> Result<()> {
    let mut seen_modes = HashSet::new();
    let mut covered_items = HashSet::new();
    let mut mode_checks = 0usize;
    let mut command_only_checks = 0usize;

    for item in ABI_PARITY_MILESTONES {
        let mut item_has_actionable_check = false;
        for check in item.acceptance_checks {
            let kind = abi_parity_acceptance_check_kind(check).unwrap_or_else(|| {
                panic!(
                    "{} has unsupported ABI parity acceptance check: {}",
                    item.id, check
                )
            });

            if let Some(mode) = abi_parity_boot_mode_from_acceptance_check(check) {
                mode_checks += 1;
                item_has_actionable_check = true;
                seen_modes.insert(mode);
                continue;
            }

            match kind {
                AbiParityAcceptanceCheckKind::CargoXtask
                | AbiParityAcceptanceCheckKind::CargoTestAbiParity
                | AbiParityAcceptanceCheckKind::Make
                | AbiParityAcceptanceCheckKind::ReadinessMarker => {
                    command_only_checks += 1;
                    item_has_actionable_check = true;
                }
                AbiParityAcceptanceCheckKind::BootMode
                | AbiParityAcceptanceCheckKind::MockFixture => {
                    panic!(
                        "{} classified as runnable but did not resolve mode: {}",
                        item.id, check
                    );
                }
            }
        }

        assert!(
            item_has_actionable_check,
            "{} must have at least one runnable or governed acceptance check",
            item.id
        );
        covered_items.insert(item.id);
    }

    assert!(
        mode_checks > 0,
        "ABI parity should define boot-mode based acceptance checks"
    );
    assert!(
        command_only_checks >= 1,
        "ABI parity should include command-based acceptance checks (make/cargo)"
    );
    assert_eq!(
        covered_items.len(),
        ABI_PARITY_MILESTONES.len(),
        "ABI parity acceptance coverage must cover every checklist item"
    );

    Ok(())
}
