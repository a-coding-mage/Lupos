//! linux-parity: partial
//! linux-source: vendor/linux/fs/sysfs/mount.c
//! test-origin: linux:vendor/linux/fs/sysfs/mount.c
//! sysfs mount tree builder.
//!
//! Ref: `vendor/linux/fs/sysfs/mount.c`

use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, add_child};

/// Show callback for `/sys/kernel/uevent_seqnum`.
/// Ref: `vendor/linux/lib/kobject_uevent.c` — the file is wired through
/// `vendor/linux/kernel/ksysfs.c::uevent_seqnum_show` and returns
/// `"<u64>\n"`.  libudev / systemd-udevd read this on every fresh
/// `udev_monitor_new_from_netlink` to deduplicate against their own
/// replay cursor.
fn uevent_seqnum_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let value = crate::net::uevent::current_seqnum();
    let text = alloc::format!("{}\n", value);
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

fn copy_text(buf: &mut [u8], text: &str) -> Result<usize, i32> {
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

/// `/sys/class/tty/tty0/active` reports the active virtual console.
/// `systemd-logind` opens this from `manager_connect_console()`; if the file is
/// missing it treats seat0 as a no-VT seat and rejects tty1 sessions.
fn tty0_active_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "tty1\n")
}

fn parse_usize(buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf)
        .map_err(|_| crate::include::uapi::errno::EINVAL)?
        .trim();
    s.parse().map_err(|_| crate::include::uapi::errno::EINVAL)
}

fn parse_u64(buf: &[u8]) -> Result<u64, i32> {
    let s = core::str::from_utf8(buf)
        .map_err(|_| crate::include::uapi::errno::EINVAL)?
        .trim();
    if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| crate::include::uapi::errno::EINVAL)
    } else {
        s.parse().map_err(|_| crate::include::uapi::errno::EINVAL)
    }
}

fn huge_nr_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let snap = crate::mm::huge::hugetlb_sysfs_snapshot();
    copy_text(buf, &alloc::format!("{}\n", snap.nr_hugepages))
}

fn huge_nr_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let v = parse_usize(buf)?;
    crate::mm::huge::hugetlb_sysctl_write(crate::mm::huge::HugetlbSysctl::NrHugepages, v)?;
    Ok(buf.len())
}

fn huge_free_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let snap = crate::mm::huge::hugetlb_sysfs_snapshot();
    copy_text(buf, &alloc::format!("{}\n", snap.free_hugepages))
}

fn huge_resv_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let snap = crate::mm::huge::hugetlb_sysfs_snapshot();
    copy_text(buf, &alloc::format!("{}\n", snap.resv_hugepages))
}

fn huge_surplus_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let snap = crate::mm::huge::hugetlb_sysfs_snapshot();
    copy_text(buf, &alloc::format!("{}\n", snap.surplus_hugepages))
}

fn huge_zero_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0\n")
}

fn thp_enabled_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "[always] madvise never\n")
}

fn thp_shmem_enabled_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "always within_size advise [never] deny force\n")
}

fn thp_defrag_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "always defer defer+madvise [madvise] never\n")
}

fn thp_hpage_pmd_size_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "2097152\n")
}

fn accept_policy_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    Ok(buf.len())
}

fn accept_num_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let _ = parse_usize(buf)?;
    Ok(buf.len())
}

fn thp_use_zero_page_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0\n")
}

fn thp_khugepaged_bool_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "1\n")
}

fn thp_khugepaged_alloc_sleep_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "60000\n")
}

fn thp_khugepaged_scan_sleep_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "10000\n")
}

fn thp_khugepaged_max_ptes_none_show(
    _node: &Arc<KernfsNode>,
    buf: &mut [u8],
) -> Result<usize, i32> {
    copy_text(buf, "511\n")
}

fn thp_khugepaged_max_ptes_swap_show(
    _node: &Arc<KernfsNode>,
    buf: &mut [u8],
) -> Result<usize, i32> {
    copy_text(buf, "64\n")
}

fn thp_khugepaged_max_ptes_shared_show(
    _node: &Arc<KernfsNode>,
    buf: &mut [u8],
) -> Result<usize, i32> {
    copy_text(buf, "256\n")
}

fn thp_khugepaged_pages_to_scan_show(
    _node: &Arc<KernfsNode>,
    buf: &mut [u8],
) -> Result<usize, i32> {
    copy_text(buf, "4096\n")
}

fn thp_pmd_enabled_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "always [inherit] madvise never\n")
}

fn thp_pmd_shmem_enabled_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(
        buf,
        "always within_size advise [inherit] deny force never\n",
    )
}

fn split_huge_pages_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let text = core::str::from_utf8(buf).map_err(|_| crate::include::uapi::errno::EINVAL)?;
    crate::mm::huge::record_split_huge_pages_command(text);
    Ok(buf.len())
}

fn unpoison_pfn_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let pfn = parse_u64(buf)?;
    crate::mm::huge::clear_hwpoison_pfn(pfn);
    Ok(buf.len())
}

fn add_mm_sysfs(kernel: &Arc<KernfsNode>) {
    let mm = KernfsNode::new_dir("mm", 0o555);

    let hugepages = KernfsNode::new_dir("hugepages", 0o555);
    let h2048 = KernfsNode::new_dir("hugepages-2048kB", 0o555);
    add_child(
        &h2048,
        KernfsNode::new_file(
            "nr_hugepages",
            0o644,
            Some(huge_nr_show),
            Some(huge_nr_store),
        ),
    );
    add_child(
        &h2048,
        KernfsNode::new_file("free_hugepages", 0o444, Some(huge_free_show), None),
    );
    add_child(
        &h2048,
        KernfsNode::new_file("resv_hugepages", 0o444, Some(huge_resv_show), None),
    );
    add_child(
        &h2048,
        KernfsNode::new_file("surplus_hugepages", 0o444, Some(huge_surplus_show), None),
    );
    add_child(&hugepages, h2048);
    let h1048576 = KernfsNode::new_dir("hugepages-1048576kB", 0o555);
    for name in [
        "nr_hugepages",
        "free_hugepages",
        "resv_hugepages",
        "surplus_hugepages",
    ] {
        add_child(
            &h1048576,
            KernfsNode::new_file(name, 0o444, Some(huge_zero_show), None),
        );
    }
    add_child(&hugepages, h1048576);
    add_child(&mm, hugepages);

    let thp = KernfsNode::new_dir("transparent_hugepage", 0o555);
    add_child(
        &thp,
        KernfsNode::new_file(
            "enabled",
            0o644,
            Some(thp_enabled_show),
            Some(accept_policy_store),
        ),
    );
    add_child(
        &thp,
        KernfsNode::new_file(
            "shmem_enabled",
            0o644,
            Some(thp_shmem_enabled_show),
            Some(accept_policy_store),
        ),
    );
    add_child(
        &thp,
        KernfsNode::new_file(
            "defrag",
            0o644,
            Some(thp_defrag_show),
            Some(accept_policy_store),
        ),
    );
    add_child(
        &thp,
        KernfsNode::new_file("hpage_pmd_size", 0o444, Some(thp_hpage_pmd_size_show), None),
    );
    add_child(
        &thp,
        KernfsNode::new_file(
            "use_zero_page",
            0o644,
            Some(thp_use_zero_page_show),
            Some(accept_num_store),
        ),
    );
    let khugepaged = KernfsNode::new_dir("khugepaged", 0o555);
    for (name, show) in [
        (
            "defrag",
            thp_khugepaged_bool_show as fn(&Arc<KernfsNode>, &mut [u8]) -> Result<usize, i32>,
        ),
        ("alloc_sleep_millisecs", thp_khugepaged_alloc_sleep_show),
        ("scan_sleep_millisecs", thp_khugepaged_scan_sleep_show),
        ("max_ptes_none", thp_khugepaged_max_ptes_none_show),
        ("max_ptes_swap", thp_khugepaged_max_ptes_swap_show),
        ("max_ptes_shared", thp_khugepaged_max_ptes_shared_show),
        ("pages_to_scan", thp_khugepaged_pages_to_scan_show),
    ] {
        add_child(
            &khugepaged,
            KernfsNode::new_file(name, 0o644, Some(show), Some(accept_num_store)),
        );
    }
    add_child(&thp, khugepaged);
    let thp_pmd = KernfsNode::new_dir("hugepages-2048kB", 0o555);
    add_child(
        &thp_pmd,
        KernfsNode::new_file(
            "enabled",
            0o644,
            Some(thp_pmd_enabled_show),
            Some(accept_policy_store),
        ),
    );
    add_child(
        &thp_pmd,
        KernfsNode::new_file(
            "shmem_enabled",
            0o644,
            Some(thp_pmd_shmem_enabled_show),
            Some(accept_policy_store),
        ),
    );
    add_child(&thp, thp_pmd);
    add_child(&mm, thp);

    add_child(kernel, mm);
}

fn fb0_name_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "lupos-fb\n")
}

fn fb0_dev_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    // Framebuffer devices use major 29; the bootloader framebuffer is fb0.
    copy_text(buf, "29:0\n")
}

/// Populate `/sys/class/graphics`, exposing `fb0` when a framebuffer is
/// registered.  `xf86-video-fbdev`'s probe `readlink`s
/// `/sys/class/graphics/fb0/device/subsystem` to confirm the framebuffer is not
/// a PCI device before it claims `/dev/fb0`; without this node the probe reports
/// "No devices detected" and Xorg dies with "no screens found".
///
/// Ref: `xf86-video-fbdev` `FBDevProbe`/`fbdevHWProbe`; Linux registers the
/// class in `drivers/video/fbdev/core/fbsysfs.c`.
fn add_graphics_class(class: &Arc<KernfsNode>) {
    let graphics = KernfsNode::new_dir("graphics", 0o555);
    if crate::linux_driver_abi::video::fbdev::core::fb_info().is_some() {
        let fb0 = KernfsNode::new_dir("fb0", 0o555);
        add_child(
            &fb0,
            KernfsNode::new_file("name", 0o444, Some(fb0_name_show), None),
        );
        add_child(
            &fb0,
            KernfsNode::new_file("dev", 0o444, Some(fb0_dev_show), None),
        );
        let device = KernfsNode::new_dir("device", 0o555);
        // The link target's basename must not contain "pci", so the driver
        // treats fb0 as a platform (non-PCI) framebuffer and claims it directly.
        add_child(
            &device,
            KernfsNode::new_symlink("subsystem", "../../../../bus/platform"),
        );
        add_child(&fb0, device);
        add_child(&graphics, fb0);
    }
    add_child(class, graphics);
}

fn add_tty_class(class: &Arc<KernfsNode>) {
    let tty = KernfsNode::new_dir("tty", 0o555);
    let tty0 = KernfsNode::new_dir("tty0", 0o555);
    add_child(
        &tty0,
        KernfsNode::new_file("active", 0o444, Some(tty0_active_show), None),
    );
    add_child(&tty, tty0);
    // systemd reads /sys/class/tty/console/active to identify the active console device.
    // vendor/linux/drivers/tty/tty_io.c — "console" is the kernel console device class entry.
    let console_dir = KernfsNode::new_dir("console", 0o555);
    add_child(
        &console_dir,
        KernfsNode::new_file("active", 0o444, Some(tty0_active_show), None),
    );
    add_child(&tty, console_dir);
    add_child(class, tty);
}

pub fn build_root() -> (Arc<KernfsNode>, Arc<KernfsNode>) {
    let root = KernfsNode::new_dir("/", 0o555);
    let kernel = KernfsNode::new_dir("kernel", 0o555);
    // /sys/kernel/uevent_seqnum — required by libudev's `udev_monitor`
    // bring-up.  Ref: vendor/linux/kernel/ksysfs.c::uevent_seqnum_show.
    add_child(
        &kernel,
        KernfsNode::new_file("uevent_seqnum", 0o444, Some(uevent_seqnum_show), None),
    );
    let debug = KernfsNode::new_dir("debug", 0o755);
    add_child(
        &debug,
        KernfsNode::new_file(
            "split_huge_pages",
            0o200,
            None,
            Some(split_huge_pages_store),
        ),
    );
    let hwpoison = KernfsNode::new_dir("hwpoison", 0o755);
    add_child(
        &hwpoison,
        KernfsNode::new_file("unpoison-pfn", 0o200, None, Some(unpoison_pfn_store)),
    );
    add_child(&debug, hwpoison);
    add_child(&kernel, debug);
    add_child(&kernel, KernfsNode::new_dir("security", 0o755));
    add_mm_sysfs(&kernel);

    let fs = KernfsNode::new_dir("fs", 0o555);
    add_child(&fs, KernfsNode::new_dir("cgroup", 0o555));

    // /sys/dev/{char,block} hold symlinks keyed by `<major>:<minor>` to
    // each registered character/block device.  Ref:
    // vendor/linux/drivers/base/core.c (`devices_init` → `dev_kobj`).
    let dev = KernfsNode::new_dir("dev", 0o555);
    add_child(&dev, KernfsNode::new_dir("char", 0o555));
    add_child(&dev, KernfsNode::new_dir("block", 0o555));

    // /sys/firmware holds platform firmware exports (EFI variables on UEFI
    // hosts, ACPI tables, DMI/SMBIOS data).  Present even when empty so
    // systemd's `firmware-setup-supported` probes resolve without ENOENT.
    let firmware = KernfsNode::new_dir("firmware", 0o555);

    // /sys/module exposes loaded kernel modules.  systemd-260.1's
    // `vendor/systemd/systemd-260.1/src/shared/module-util.c` walks this
    // tree during `systemd-modules-load.service` to skip already-loaded
    // modules.
    let module = KernfsNode::new_dir("module", 0o555);

    let class = KernfsNode::new_dir("class", 0o555);
    add_tty_class(&class);
    add_graphics_class(&class);

    for child in [
        kernel.clone(),
        KernfsNode::new_dir("devices", 0o555),
        KernfsNode::new_dir("bus", 0o555),
        class,
        KernfsNode::new_dir("block", 0o555),
        dev,
        firmware,
        module,
        fs,
    ] {
        add_child(&root, child);
    }
    (root, kernel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::{KernfsKind, lookup};

    /// Source-backed parity check: every sysfs top-level directory that
    /// systemd-260.1 probes during early boot must resolve, and the
    /// uevent_seqnum file must format as `<u64>\n`.  Ref:
    /// vendor/systemd/systemd-260.1/src/libsystemd/sd-device/device-monitor.c
    /// and vendor/systemd/systemd-260.1/src/basic/cgroup-util.c.
    #[test]
    fn sysfs_root_layout_matches_systemd_probes() {
        let (root, kernel) = build_root();
        for top in [
            "kernel", "devices", "bus", "class", "block", "dev", "firmware", "module", "fs",
        ] {
            assert!(
                lookup(&root, top).is_some(),
                "/sys/{top} must exist for systemd probes"
            );
        }
        // `/sys/fs/cgroup` is the cgroup2 mountpoint systemd binds to.
        let fs = lookup(&root, "fs").expect("/sys/fs");
        assert!(lookup(&fs, "cgroup").is_some(), "/sys/fs/cgroup must exist");

        let dev = lookup(&root, "dev").expect("/sys/dev");
        for sub in ["char", "block"] {
            assert!(
                lookup(&dev, sub).is_some(),
                "/sys/dev/{sub} must exist for udev device class enumeration"
            );
        }

        let class = lookup(&root, "class").expect("/sys/class");
        let tty = lookup(&class, "tty").expect("/sys/class/tty");
        let tty0 = lookup(&tty, "tty0").expect("/sys/class/tty/tty0");
        assert_eq!(
            show(&lookup(&tty0, "active").expect("/sys/class/tty/tty0/active")),
            "tty1\n"
        );

        // uevent_seqnum: formatted as decimal + newline, matches the
        // current global counter.
        let seqnum_node = lookup(&kernel, "uevent_seqnum").expect("/sys/kernel/uevent_seqnum");
        let mut buf = [0u8; 32];
        let n = uevent_seqnum_show(&seqnum_node, &mut buf).unwrap();
        let text = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(text.ends_with('\n'), "uevent_seqnum must end with newline");
        let parsed: u64 = text
            .trim()
            .parse()
            .expect("uevent_seqnum must parse as u64");
        // The reader sees a snapshot ≤ the live counter — fetch again and
        // verify monotonicity.
        let live = crate::net::uevent::current_seqnum();
        assert!(
            parsed <= live,
            "uevent_seqnum read {parsed} must be ≤ live counter {live}"
        );
    }

    fn show(node: &Arc<KernfsNode>) -> alloc::string::String {
        let KernfsKind::File { show, .. } = &node.kind else {
            panic!("not a file");
        };
        let mut buf = [0u8; 96];
        let n = (show.expect("show fn"))(node, &mut buf).expect("show ok");
        core::str::from_utf8(&buf[..n]).unwrap().into()
    }

    fn store(node: &Arc<KernfsNode>, bytes: &[u8]) -> Result<usize, i32> {
        let KernfsKind::File { store, .. } = &node.kind else {
            panic!("not a file");
        };
        (store.expect("store fn"))(node, bytes)
    }

    #[test]
    fn sysfs_kernel_mm_hugepage_controls_match_linux_layout() {
        let _guard = crate::mm::test_lock::GLOBAL_HW_TEST_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        crate::mm::huge::reset_for_tests();
        let (_root, kernel) = build_root();
        assert!(
            lookup(&kernel, "security").is_some(),
            "/sys/kernel/security must exist as the securityfs mountpoint"
        );
        let mm = lookup(&kernel, "mm").expect("/sys/kernel/mm");
        let hugepages = lookup(&mm, "hugepages").expect("/sys/kernel/mm/hugepages");
        let h2048 = lookup(&hugepages, "hugepages-2048kB").expect("2M hugepage dir");
        let nr = lookup(&h2048, "nr_hugepages").expect("nr_hugepages");
        assert_eq!(store(&nr, b"4\n"), Ok(2));
        assert_eq!(show(&nr), "4\n");
        assert_eq!(
            show(&lookup(&h2048, "free_hugepages").expect("free_hugepages")),
            "4\n"
        );
        crate::mm::huge::reset_for_tests();
    }

    #[test]
    fn sysfs_transparent_hugepage_policy_files_exist() {
        let (_root, kernel) = build_root();
        let mm = lookup(&kernel, "mm").expect("/sys/kernel/mm");
        let thp = lookup(&mm, "transparent_hugepage").expect("transparent_hugepage");
        assert!(show(&lookup(&thp, "enabled").expect("enabled")).contains("[always]"));
        assert_eq!(
            show(&lookup(&thp, "hpage_pmd_size").expect("hpage_pmd_size")),
            "2097152\n"
        );
        assert_eq!(
            show(&lookup(&thp, "use_zero_page").expect("use_zero_page")),
            "0\n"
        );
        let khugepaged = lookup(&thp, "khugepaged").expect("khugepaged");
        assert_eq!(
            show(&lookup(&khugepaged, "pages_to_scan").expect("pages_to_scan")),
            "4096\n"
        );
        let pmd = lookup(&thp, "hugepages-2048kB").expect("THP PMD size dir");
        assert!(show(&lookup(&pmd, "enabled").expect("enabled")).contains("[inherit]"));
    }
}
