//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/proc_sysctl.c
//! test-origin: linux:vendor/linux/fs/proc/proc_sysctl.c
//! `/proc/sys`.
//!
//! Ref: `vendor/linux/fs/proc/proc_sysctl.c`,
//!      `vendor/linux/kernel/utsname_sysctl.c`,
//!      `vendor/linux/fs/file_table.c::fs_stat_sysctls`,
//!      `vendor/linux/fs/file.c::sysctl_nr_open*`.
//!
//! The `fs/` subtree carries live file-table sysctls used by PID 1. The
//! `kernel/` subtree exposes the UTS strings Linux registers from
//! `utsname_sysctl.c`; journald opens `/proc/sys/kernel/hostname` during
//! manager initialization and treats ENOENT as fatal.

extern crate alloc;

use alloc::sync::Arc;
use alloc::{format, string::String};
use core::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, Ordering};

use crate::fs::kernfs::{KernfsNode, add_child};
use crate::include::uapi::errno::EINVAL;

// Linux defaults — `vendor/linux/include/uapi/linux/fs.h:205` and
// `vendor/linux/fs/file_table.c::files_stat`.  The runtime kernel replaces
// `files_stat.max_files` during `files_init()` based on RAM, but the static
// default is `NR_FILE = 8192`.
static FS_FILE_MAX: AtomicI64 = AtomicI64::new(8192);

// `vendor/linux/fs/file.c:96-101`:
//   sysctl_nr_open     = 1024 * 1024
//   sysctl_nr_open_min = BITS_PER_LONG (64 on x86_64)
//   sysctl_nr_open_max ≈ INT_MAX rounded down to BITS_PER_LONG
static FS_NR_OPEN: AtomicU32 = AtomicU32::new(1_048_576);
const FS_NR_OPEN_MIN: u32 = 64;
const FS_NR_OPEN_MAX: u32 = 1_073_741_824; // (1 << 30); INT_MAX & -64 on 64-bit.

// POSIX mqueue sysctls. Linux registers these under `/proc/sys/fs/mqueue`;
// systemd uses the directory as the API-filesystem condition for
// `dev-mqueue.mount`.
static MQ_QUEUES_MAX: AtomicU32 = AtomicU32::new(256);
static MQ_MSG_MAX: AtomicU32 = AtomicU32::new(10);
static MQ_MSGSIZE_MAX: AtomicU32 = AtomicU32::new(8192);

// SysV SHM sysctls used by the upstream MM selftests before hugetlb/shm
// probes. Linux exposes them under `/proc/sys/kernel`.
static KERNEL_SHMMAX: AtomicU64 = AtomicU64::new(4_294_967_296);
static KERNEL_SHMALL: AtomicU64 = AtomicU64::new(4_194_304);

// Linux `vm.mmap_min_addr` is exposed through the VM sysctl table. Most
// distro x86_64 configs default it to 64 KiB; the MM selftests read it to pick
// safe low-address probes.
static VM_MMAP_MIN_ADDR: AtomicU64 = AtomicU64::new(65_536);

// Linux `drivers/char/random.c::random_table` exposes `boot_id` as one UUID
// generated lazily for the whole boot, and `uuid` as a freshly generated UUID
// on each read.
static BOOT_UUID_LO: AtomicU64 = AtomicU64::new(0);
static BOOT_UUID_HI: AtomicU64 = AtomicU64::new(0);
static UUID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn file_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = FS_FILE_MAX.load(Ordering::Acquire);
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn file_max_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    let v: i64 = s.parse().map_err(|_| EINVAL)?;
    if v < 0 {
        return Err(EINVAL);
    }
    FS_FILE_MAX.store(v, Ordering::Release);
    Ok(buf.len())
}

fn nr_open_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = FS_NR_OPEN.load(Ordering::Acquire);
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn nr_open_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    let v: u32 = s.parse().map_err(|_| EINVAL)?;
    let clamped = v.clamp(FS_NR_OPEN_MIN, FS_NR_OPEN_MAX);
    FS_NR_OPEN.store(clamped, Ordering::Release);
    Ok(buf.len())
}

fn mqueue_u32_show(value: &AtomicU32, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &format!("{}\n", value.load(Ordering::Acquire)))
}

fn mqueue_u32_store(value: &AtomicU32, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    let v: u32 = s.parse().map_err(|_| EINVAL)?;
    value.store(v, Ordering::Release);
    Ok(buf.len())
}

fn queues_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    mqueue_u32_show(&MQ_QUEUES_MAX, buf)
}

fn queues_max_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    mqueue_u32_store(&MQ_QUEUES_MAX, buf)
}

fn msg_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    mqueue_u32_show(&MQ_MSG_MAX, buf)
}

fn msg_max_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    mqueue_u32_store(&MQ_MSG_MAX, buf)
}

fn msgsize_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    mqueue_u32_show(&MQ_MSGSIZE_MAX, buf)
}

fn msgsize_max_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    mqueue_u32_store(&MQ_MSGSIZE_MAX, buf)
}

fn uts_bytes_to_string(bytes: &[u8]) -> String {
    let len = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    let mut s = String::new();
    for b in &bytes[..len] {
        s.push(*b as char);
    }
    s.push('\n');
    s
}

fn uts_store_bytes(buf: &[u8]) -> Result<[u8; crate::kernel::utsname::NEW_UTS_LEN_PLUS_NUL], i32> {
    let mut len = buf.len();
    while len > 0 && (buf[len - 1] == b'\n' || buf[len - 1] == b'\r') {
        len -= 1;
    }
    if len >= crate::kernel::utsname::NEW_UTS_LEN_PLUS_NUL {
        return Err(EINVAL);
    }
    let mut out = [0u8; crate::kernel::utsname::NEW_UTS_LEN_PLUS_NUL];
    out[..len].copy_from_slice(&buf[..len]);
    Ok(out)
}

fn uts_static_show(value: &[u8], buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &uts_bytes_to_string(value))
}

fn uts_arch_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    uts_static_show(&crate::kernel::utsname::INIT_UTS_NS.name.machine, buf)
}

fn uts_ostype_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    uts_static_show(&crate::kernel::utsname::INIT_UTS_NS.name.sysname, buf)
}

fn uts_osrelease_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    uts_static_show(&crate::kernel::utsname::INIT_UTS_NS.name.release, buf)
}

fn uts_version_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    uts_static_show(&crate::kernel::utsname::INIT_UTS_NS.name.version, buf)
}

fn hostname_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(
        buf,
        &uts_bytes_to_string(&crate::kernel::utsname::current_nodename()),
    )
}

fn hostname_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let name = uts_store_bytes(buf)?;
    crate::kernel::utsname::set_current_nodename_packed(name);
    Ok(buf.len())
}

fn domainname_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(
        buf,
        &uts_bytes_to_string(&crate::kernel::utsname::current_domainname()),
    )
}

fn domainname_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let name = uts_store_bytes(buf)?;
    crate::kernel::utsname::set_current_domainname_packed(name);
    Ok(buf.len())
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn uuid_from_seed(seed: u64) -> [u8; 16] {
    let lo = splitmix64(seed);
    let hi = splitmix64(seed ^ 0xa5a5_5a5a_d3c1_b2e0);
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&lo.to_be_bytes());
    out[8..].copy_from_slice(&hi.to_be_bytes());
    out[6] = (out[6] & 0x0f) | 0x40; // RFC 4122 version 4.
    out[8] = (out[8] & 0x3f) | 0x80; // RFC 4122 variant.
    out
}

fn format_uuid(uuid: [u8; 16]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}\n",
        uuid[0],
        uuid[1],
        uuid[2],
        uuid[3],
        uuid[4],
        uuid[5],
        uuid[6],
        uuid[7],
        uuid[8],
        uuid[9],
        uuid[10],
        uuid[11],
        uuid[12],
        uuid[13],
        uuid[14],
        uuid[15]
    )
}

fn boot_uuid() -> [u8; 16] {
    let mut lo = BOOT_UUID_LO.load(Ordering::Acquire);
    let mut hi = BOOT_UUID_HI.load(Ordering::Acquire);
    if lo == 0 && hi == 0 {
        let realtime = crate::kernel::time::ktime_get_real();
        let mono = crate::kernel::time::ktime_get();
        let seed = realtime
            ^ mono.rotate_left(17)
            ^ crate::kernel::time::jiffies().rotate_left(33)
            ^ 0x6c75_706f_735f_626f;
        let uuid = uuid_from_seed(seed);
        lo = u64::from_be_bytes(uuid[..8].try_into().unwrap_or([0; 8]));
        hi = u64::from_be_bytes(uuid[8..].try_into().unwrap_or([0; 8]));
        if lo == 0 && hi == 0 {
            lo = 0x6c75_706f_0000_4000;
            hi = 0x8000_0000_0000_0001;
        }
        BOOT_UUID_LO.store(lo, Ordering::Release);
        BOOT_UUID_HI.store(hi, Ordering::Release);
    }
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&lo.to_be_bytes());
    out[8..].copy_from_slice(&hi.to_be_bytes());
    out
}

fn boot_id_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &format_uuid(boot_uuid()))
}

fn uuid_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let seq = UUID_COUNTER.fetch_add(1, Ordering::AcqRel);
    let seed = crate::kernel::time::ktime_get_real()
        ^ crate::kernel::time::ktime_get().rotate_left(11)
        ^ seq.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    super::util::copy_into(buf, &format_uuid(uuid_from_seed(seed)))
}

fn random_ro_int_show(value: i32, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &format!("{value}\n"))
}

fn poolsize_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    random_ro_int_show(256, buf)
}

fn entropy_avail_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    random_ro_int_show(256, buf)
}

fn write_wakeup_threshold_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    random_ro_int_show(256, buf)
}

fn urandom_min_reseed_secs_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    random_ro_int_show(60, buf)
}

fn random_ro_int_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    Ok(buf.len())
}

fn mmap_min_addr_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = VM_MMAP_MIN_ADDR.load(Ordering::Acquire);
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn mmap_min_addr_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    let v: u64 = s.parse().map_err(|_| EINVAL)?;
    VM_MMAP_MIN_ADDR.store(v, Ordering::Release);
    Ok(buf.len())
}

fn atomic_u64_show(value: &AtomicU64, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &format!("{}\n", value.load(Ordering::Acquire)))
}

fn atomic_u64_store(value: &AtomicU64, buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    let v: u64 = s.parse().map_err(|_| EINVAL)?;
    value.store(v, Ordering::Release);
    Ok(buf.len())
}

fn shmmax_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    atomic_u64_show(&KERNEL_SHMMAX, buf)
}

fn shmmax_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    atomic_u64_store(&KERNEL_SHMMAX, buf)
}

fn shmall_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    atomic_u64_show(&KERNEL_SHMALL, buf)
}

fn shmall_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    atomic_u64_store(&KERNEL_SHMALL, buf)
}

fn parse_usize_sysctl(buf: &[u8]) -> Result<usize, i32> {
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    s.parse().map_err(|_| EINVAL)
}

fn nr_hugepages_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = crate::mm::huge::hugetlb_sysctl_read(crate::mm::huge::HugetlbSysctl::NrHugepages);
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn nr_hugepages_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let v = parse_usize_sysctl(buf)?;
    crate::mm::huge::hugetlb_sysctl_write(crate::mm::huge::HugetlbSysctl::NrHugepages, v)?;
    Ok(buf.len())
}

fn nr_overcommit_hugepages_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v =
        crate::mm::huge::hugetlb_sysctl_read(crate::mm::huge::HugetlbSysctl::NrOvercommitHugepages);
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn nr_overcommit_hugepages_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let v = parse_usize_sysctl(buf)?;
    crate::mm::huge::hugetlb_sysctl_write(
        crate::mm::huge::HugetlbSysctl::NrOvercommitHugepages,
        v,
    )?;
    Ok(buf.len())
}

fn enable_soft_offline_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = crate::mm::huge::hugetlb_sysctl_read(crate::mm::huge::HugetlbSysctl::EnableSoftOffline);
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn enable_soft_offline_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let v = parse_usize_sysctl(buf)?;
    crate::mm::huge::hugetlb_sysctl_write(crate::mm::huge::HugetlbSysctl::EnableSoftOffline, v)?;
    Ok(buf.len())
}

fn drop_caches_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "0\n")
}

fn drop_caches_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let v = parse_usize_sysctl(buf)?;
    if v & !0x7 != 0 {
        return Err(EINVAL);
    }
    let _ = crate::mm::reclaim::drop_caches(v as u32);
    Ok(buf.len())
}

fn compact_memory_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "0\n")
}

fn compact_memory_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    let _ = parse_usize_sysctl(buf)?;
    let _ = crate::mm::migration::compact_memory();
    Ok(buf.len())
}

fn cap_last_cap_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    // Linux kernel/capability.c::cap_last_cap_show — highest valid capability.
    // systemd reads this before dropping capabilities in exec_child().
    // vendor/linux/kernel/capability.c::cap_last_cap
    let v = crate::kernel::capability::CAP_LAST_CAP;
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn ngroups_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    // Linux: NGROUPS_MAX = 65536.  vendor/linux/include/uapi/linux/limits.h.
    super::util::copy_into(buf, "65536\n")
}

fn pid_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    // Linux default: 4194304 (2^22) on 64-bit.  vendor/linux/kernel/pid.c.
    let v = crate::kernel::pid::PID_MAX_DEFAULT;
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn pid_max_store(_node: &Arc<KernfsNode>, _buf: &[u8]) -> Result<usize, i32> {
    Ok(_buf.len())
}

fn threads_max_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    // Linux default: min(THREAD_SIZE_ORDER page count * 8, ULLONG_MAX).
    // Expose a reasonable large value so systemd doesn't over-constrain forks.
    super::util::copy_into(buf, "32768\n")
}

fn overflowuid_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "65534\n")
}

fn overflowgid_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "65534\n")
}

fn dmesg_restrict_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "0\n")
}

fn kptr_restrict_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, "0\n")
}

fn printk_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    // Four values: console_loglevel, default_message_loglevel, minimum_console_loglevel,
    // default_console_loglevel.  Match Linux defaults.
    super::util::copy_into(buf, "4\t4\t1\t7\n")
}

fn add_uts_kernel_sysctls(kernel: &Arc<KernfsNode>) {
    // Linux `kernel/utsname_sysctl.c::uts_kern_table` registers these entries
    // under `/proc/sys/kernel`; hostname/domainname are writable.
    add_child(
        kernel,
        KernfsNode::new_file("arch", 0o444, Some(uts_arch_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("ostype", 0o444, Some(uts_ostype_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("osrelease", 0o444, Some(uts_osrelease_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("version", 0o444, Some(uts_version_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("hostname", 0o644, Some(hostname_show), Some(hostname_store)),
    );
    add_child(
        kernel,
        KernfsNode::new_file(
            "domainname",
            0o644,
            Some(domainname_show),
            Some(domainname_store),
        ),
    );
    add_child(
        kernel,
        KernfsNode::new_file("shmmax", 0o644, Some(shmmax_show), Some(shmmax_store)),
    );
    add_child(
        kernel,
        KernfsNode::new_file("shmall", 0o644, Some(shmall_show), Some(shmall_store)),
    );
    // capability sysctls — systemd reads cap_last_cap before dropping caps in exec_child().
    // vendor/linux/kernel/capability.c::cap_last_cap_show
    add_child(
        kernel,
        KernfsNode::new_file("cap_last_cap", 0o444, Some(cap_last_cap_show), None),
    );
    // Process limits — read by systemd for ulimit setup.
    add_child(
        kernel,
        KernfsNode::new_file("ngroups_max", 0o444, Some(ngroups_max_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("pid_max", 0o644, Some(pid_max_show), Some(pid_max_store)),
    );
    add_child(
        kernel,
        KernfsNode::new_file("threads-max", 0o644, Some(threads_max_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("overflowuid", 0o644, Some(overflowuid_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("overflowgid", 0o644, Some(overflowgid_show), None),
    );
    // Security/logging knobs — systemd reads these for hardening decisions.
    add_child(
        kernel,
        KernfsNode::new_file("dmesg_restrict", 0o644, Some(dmesg_restrict_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("kptr_restrict", 0o644, Some(kptr_restrict_show), None),
    );
    add_child(
        kernel,
        KernfsNode::new_file("printk", 0o644, Some(printk_show), None),
    );
}

fn add_random_sysctls(kernel: &Arc<KernfsNode>) {
    let random = KernfsNode::new_dir("random", 0o555);
    add_child(
        &random,
        KernfsNode::new_file("poolsize", 0o444, Some(poolsize_show), None),
    );
    add_child(
        &random,
        KernfsNode::new_file("entropy_avail", 0o444, Some(entropy_avail_show), None),
    );
    add_child(
        &random,
        KernfsNode::new_file(
            "write_wakeup_threshold",
            0o644,
            Some(write_wakeup_threshold_show),
            Some(random_ro_int_store),
        ),
    );
    add_child(
        &random,
        KernfsNode::new_file(
            "urandom_min_reseed_secs",
            0o644,
            Some(urandom_min_reseed_secs_show),
            Some(random_ro_int_store),
        ),
    );
    add_child(
        &random,
        KernfsNode::new_file("boot_id", 0o444, Some(boot_id_show), None),
    );
    add_child(
        &random,
        KernfsNode::new_file("uuid", 0o444, Some(uuid_show), None),
    );
    add_child(kernel, random);
}

fn add_vm_sysctls(vm: &Arc<KernfsNode>) {
    add_child(
        vm,
        KernfsNode::new_file(
            "mmap_min_addr",
            0o644,
            Some(mmap_min_addr_show),
            Some(mmap_min_addr_store),
        ),
    );
    add_child(
        vm,
        KernfsNode::new_file(
            "nr_hugepages",
            0o644,
            Some(nr_hugepages_show),
            Some(nr_hugepages_store),
        ),
    );
    add_child(
        vm,
        KernfsNode::new_file(
            "nr_overcommit_hugepages",
            0o644,
            Some(nr_overcommit_hugepages_show),
            Some(nr_overcommit_hugepages_store),
        ),
    );
    add_child(
        vm,
        KernfsNode::new_file(
            "enable_soft_offline",
            0o644,
            Some(enable_soft_offline_show),
            Some(enable_soft_offline_store),
        ),
    );
    add_child(
        vm,
        KernfsNode::new_file(
            "drop_caches",
            0o200,
            Some(drop_caches_show),
            Some(drop_caches_store),
        ),
    );
    add_child(
        vm,
        KernfsNode::new_file(
            "compact_memory",
            0o200,
            Some(compact_memory_show),
            Some(compact_memory_store),
        ),
    );
}

fn add_mqueue_sysctls(fs: &Arc<KernfsNode>) {
    let mqueue = KernfsNode::new_dir("mqueue", 0o555);
    add_child(
        &mqueue,
        KernfsNode::new_file(
            "queues_max",
            0o644,
            Some(queues_max_show),
            Some(queues_max_store),
        ),
    );
    add_child(
        &mqueue,
        KernfsNode::new_file("msg_max", 0o644, Some(msg_max_show), Some(msg_max_store)),
    );
    add_child(
        &mqueue,
        KernfsNode::new_file(
            "msgsize_max",
            0o644,
            Some(msgsize_max_show),
            Some(msgsize_max_store),
        ),
    );
    add_child(fs, mqueue);
}

fn vsyscall32_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = crate::arch::x86::entry::vdso::vdso32_setup::vdso32_enabled();
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn vsyscall32_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    use crate::arch::x86::entry::vdso::vdso32_setup as vdso;
    let s = core::str::from_utf8(buf).map_err(|_| EINVAL)?.trim();
    let v: i32 = s.parse().map_err(|_| EINVAL)?;
    // proc_dointvec_minmax with extra1=SYSCTL_ZERO, extra2=SYSCTL_ONE: reject
    // out-of-range writes with -EINVAL.
    vdso::vdso_sysctl_store(&vdso::vdso_table(true), v)?;
    Ok(buf.len())
}

fn add_abi_sysctls(dir: &Arc<KernfsNode>) {
    // Linux registers `abi.vsyscall32` from arch/x86/entry/vdso/vdso32-setup.c
    // (`register_sysctl("abi", vdso_table)` in `ia32_binfmt_init`); it gates the
    // 32-bit vDSO. lupos builds /proc/sys centrally, so the registration lives
    // here, wired to the real `vdso32_enabled` value.
    let abi = KernfsNode::new_dir("abi", 0o555);
    add_child(
        &abi,
        KernfsNode::new_file(
            "vsyscall32",
            0o644,
            Some(vsyscall32_show),
            Some(vsyscall32_store),
        ),
    );
    add_child(dir, abi);
}

fn legacy_tiocsti_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = crate::linux_driver_abi::tty::legacy_tiocsti_enabled() as u32;
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn legacy_tiocsti_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    // Linux `proc_dobool` — only 0 and 1 are accepted.
    let v = parse_usize_sysctl(buf)?;
    if v > 1 {
        return Err(EINVAL);
    }
    crate::linux_driver_abi::tty::set_legacy_tiocsti(v != 0);
    Ok(buf.len())
}

fn ldisc_autoload_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let v = crate::linux_driver_abi::tty::ldisc_autoload_enabled() as u32;
    super::util::copy_into(buf, &format!("{v}\n"))
}

fn ldisc_autoload_store(_node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    // Linux `proc_dointvec_minmax` with extra1=SYSCTL_ZERO, extra2=SYSCTL_ONE.
    let v = parse_usize_sysctl(buf)?;
    if v > 1 {
        return Err(EINVAL);
    }
    crate::linux_driver_abi::tty::set_ldisc_autoload(v != 0);
    Ok(buf.len())
}

fn add_dev_sysctls(dir: &Arc<KernfsNode>) {
    // Linux `tty_init()` — `register_sysctl_init("dev/tty", tty_table)`,
    // `drivers/tty/tty_io.c::tty_table`.
    let dev = KernfsNode::new_dir("dev", 0o555);
    let tty = KernfsNode::new_dir("tty", 0o555);
    add_child(
        &tty,
        KernfsNode::new_file(
            "legacy_tiocsti",
            0o644,
            Some(legacy_tiocsti_show),
            Some(legacy_tiocsti_store),
        ),
    );
    add_child(
        &tty,
        KernfsNode::new_file(
            "ldisc_autoload",
            0o644,
            Some(ldisc_autoload_show),
            Some(ldisc_autoload_store),
        ),
    );
    add_child(&dev, tty);
    add_child(dir, dev);
}

pub fn new_sys_dir() -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir("sys", 0o555);
    let kernel = KernfsNode::new_dir("kernel", 0o555);
    add_uts_kernel_sysctls(&kernel);
    add_random_sysctls(&kernel);
    add_child(&dir, kernel);
    let vm = KernfsNode::new_dir("vm", 0o555);
    add_vm_sysctls(&vm);
    add_child(&dir, vm);
    add_dev_sysctls(&dir);

    let fs = KernfsNode::new_dir("fs", 0o555);
    add_child(
        &fs,
        KernfsNode::new_file("file-max", 0o644, Some(file_max_show), Some(file_max_store)),
    );
    add_child(
        &fs,
        KernfsNode::new_file("nr_open", 0o644, Some(nr_open_show), Some(nr_open_store)),
    );
    add_mqueue_sysctls(&fs);
    add_child(&dir, fs);

    add_abi_sysctls(&dir);
    dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::kernfs::{KernfsKind, lookup};
    use lazy_static::lazy_static;
    use spin::Mutex;

    // FS_FILE_MAX and FS_NR_OPEN are process-global atomics shared with the
    // production code.  cargo's default test runner is parallel, so every
    // test that reads or writes these atomics must serialise against the
    // others to keep the per-test defaults stable.
    lazy_static! {
        static ref SYSCTL_TEST_LOCK: Mutex<()> = Mutex::new(());
    }

    fn reset_defaults() {
        FS_FILE_MAX.store(8192, Ordering::Release);
        FS_NR_OPEN.store(1_048_576, Ordering::Release);
        KERNEL_SHMMAX.store(4_294_967_296, Ordering::Release);
        KERNEL_SHMALL.store(4_194_304, Ordering::Release);
        let _ =
            crate::mm::huge::hugetlb_sysctl_write(crate::mm::huge::HugetlbSysctl::NrHugepages, 0);
        let _ = crate::mm::huge::hugetlb_sysctl_write(
            crate::mm::huge::HugetlbSysctl::NrOvercommitHugepages,
            0,
        );
        let _ = crate::mm::huge::hugetlb_sysctl_write(
            crate::mm::huge::HugetlbSysctl::EnableSoftOffline,
            1,
        );
    }

    fn file_node(name: &str) -> Arc<KernfsNode> {
        let sys = new_sys_dir();
        let fs = lookup(&sys, "fs").expect("fs/ subdir");
        lookup(&fs, name).unwrap_or_else(|| panic!("{name} missing"))
    }

    fn kernel_node(name: &str) -> Arc<KernfsNode> {
        let sys = new_sys_dir();
        let kernel = lookup(&sys, "kernel").expect("kernel/ subdir");
        lookup(&kernel, name).unwrap_or_else(|| panic!("{name} missing"))
    }

    fn random_node(name: &str) -> Arc<KernfsNode> {
        let sys = new_sys_dir();
        let kernel = lookup(&sys, "kernel").expect("kernel/ subdir");
        let random = lookup(&kernel, "random").expect("random/ subdir");
        lookup(&random, name).unwrap_or_else(|| panic!("{name} missing"))
    }

    fn vm_node(name: &str) -> Arc<KernfsNode> {
        let sys = new_sys_dir();
        let vm = lookup(&sys, "vm").expect("vm/ subdir");
        lookup(&vm, name).unwrap_or_else(|| panic!("{name} missing"))
    }

    fn abi_node(name: &str) -> Arc<KernfsNode> {
        let sys = new_sys_dir();
        let abi = lookup(&sys, "abi").expect("abi/ subdir");
        lookup(&abi, name).unwrap_or_else(|| panic!("{name} missing"))
    }

    fn show(node: &Arc<KernfsNode>) -> alloc::string::String {
        let KernfsKind::File { show, .. } = &node.kind else {
            panic!("not a file");
        };
        let mut buf = [0u8; 64];
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
    fn proc_sys_fs_file_max_reads_default_nr_file() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("file-max");
        assert_eq!(show(&node), "8192\n");
    }

    #[test]
    fn proc_sys_fs_nr_open_reads_default_one_megi() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("nr_open");
        assert_eq!(show(&node), "1048576\n");
    }

    #[test]
    fn proc_sys_fs_file_max_writes_then_reads_back() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("file-max");
        assert!(store(&node, b"65536\n").is_ok());
        assert_eq!(show(&node), "65536\n");
    }

    #[test]
    fn proc_sys_vm_mmap_min_addr_matches_x86_default_and_is_writable() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        VM_MMAP_MIN_ADDR.store(65_536, Ordering::Release);
        let node = vm_node("mmap_min_addr");
        assert_eq!(show(&node), "65536\n");
        assert_eq!(store(&node, b"32768\n"), Ok(6));
        assert_eq!(show(&node), "32768\n");
    }

    #[test]
    fn proc_sys_abi_vsyscall32_reads_default_and_rejects_out_of_range() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        use crate::arch::x86::entry::vdso::vdso32_setup as vdso;
        vdso::set_vdso32_enabled(vdso::VDSO_DEFAULT);
        let node = abi_node("vsyscall32");
        assert_eq!(show(&node), "1\n");
        // Writable 0/1, wired to the real vdso32_enabled value.
        assert_eq!(store(&node, b"0\n"), Ok(2));
        assert_eq!(show(&node), "0\n");
        assert_eq!(vdso::vdso32_enabled(), 0);
        // proc_dointvec_minmax clamp [0,1]: out-of-range is rejected, value kept.
        assert_eq!(store(&node, b"2\n"), Err(EINVAL));
        assert_eq!(show(&node), "0\n");
        vdso::set_vdso32_enabled(vdso::VDSO_DEFAULT);
    }

    #[test]
    fn proc_sys_fs_nr_open_clamps_above_max() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("nr_open");
        assert!(store(&node, b"2000000000\n").is_ok());
        assert_eq!(show(&node), format!("{FS_NR_OPEN_MAX}\n"));
    }

    #[test]
    fn proc_sys_fs_nr_open_clamps_below_min() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("nr_open");
        assert!(store(&node, b"1\n").is_ok());
        assert_eq!(show(&node), format!("{FS_NR_OPEN_MIN}\n"));
    }

    #[test]
    fn proc_sys_fs_file_max_rejects_garbage_with_einval() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("file-max");
        assert_eq!(store(&node, b"abc"), Err(EINVAL));
        // Value unchanged.
        assert_eq!(show(&node), "8192\n");
    }

    #[test]
    fn proc_sys_fs_file_max_rejects_negative_with_einval() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let node = file_node("file-max");
        assert_eq!(store(&node, b"-1\n"), Err(EINVAL));
    }

    #[test]
    fn proc_sys_kernel_hostname_reads_current_uts_nodename() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        crate::kernel::utsname::set_current_nodename_packed(crate::kernel::utsname::pack65(
            "journal-node",
        ));
        let node = kernel_node("hostname");
        assert_eq!(show(&node), "journal-node\n");
    }

    #[test]
    fn proc_sys_kernel_hostname_write_updates_current_uts_nodename() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        let node = kernel_node("hostname");
        assert_eq!(store(&node, b"lupos-host\n"), Ok("lupos-host\n".len()));
        assert_eq!(show(&node), "lupos-host\n");
        assert_eq!(
            crate::kernel::utsname::current_nodename(),
            crate::kernel::utsname::pack65("lupos-host")
        );
    }

    #[test]
    fn proc_sys_kernel_uts_table_matches_linux_entry_names() {
        let sys = new_sys_dir();
        let kernel = lookup(&sys, "kernel").expect("kernel/ subdir");
        for name in [
            "arch",
            "ostype",
            "osrelease",
            "version",
            "hostname",
            "domainname",
            "shmmax",
            "shmall",
        ] {
            assert!(lookup(&kernel, name).is_some(), "{name} sysctl missing");
        }
    }

    #[test]
    fn proc_sys_kernel_shm_limits_are_writable() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let shmmax = kernel_node("shmmax");
        let shmall = kernel_node("shmall");
        assert_eq!(show(&shmmax), "4294967296\n");
        assert_eq!(show(&shmall), "4194304\n");
        assert_eq!(store(&shmmax, b"536870912\n"), Ok(10));
        assert_eq!(store(&shmall, b"8388608\n"), Ok(8));
        assert_eq!(show(&shmmax), "536870912\n");
        assert_eq!(show(&shmall), "8388608\n");
    }

    #[test]
    fn proc_sys_vm_hugepage_controls_forward_to_mm_hugetlb_state() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let nr = vm_node("nr_hugepages");
        let overcommit = vm_node("nr_overcommit_hugepages");
        let soft_offline = vm_node("enable_soft_offline");
        assert_eq!(show(&nr), "0\n");
        assert_eq!(store(&nr, b"8\n"), Ok(2));
        assert_eq!(show(&nr), "8\n");
        assert_eq!(store(&overcommit, b"3\n"), Ok(2));
        assert_eq!(show(&overcommit), "3\n");
        assert_eq!(show(&soft_offline), "1\n");
        assert_eq!(store(&soft_offline, b"0\n"), Ok(2));
        assert_eq!(show(&soft_offline), "0\n");
        assert_eq!(store(&soft_offline, b"2\n"), Err(EINVAL));
    }

    #[test]
    fn proc_sys_vm_pressure_knobs_accept_linux_selftest_writes() {
        let _guard = SYSCTL_TEST_LOCK.lock();
        reset_defaults();
        let drop_caches = vm_node("drop_caches");
        let compact_memory = vm_node("compact_memory");
        assert_eq!(store(&drop_caches, b"3\n"), Ok(2));
        assert_eq!(store(&compact_memory, b"1\n"), Ok(2));
        assert_eq!(store(&drop_caches, b"8\n"), Err(EINVAL));
    }

    #[test]
    fn proc_sys_kernel_random_boot_id_is_stable_uuid() {
        let node = random_node("boot_id");
        let first = show(&node);
        let second = show(&node);
        assert_eq!(first, second);
        assert_eq!(first.len(), 37);
        assert_eq!(first.as_bytes()[8], b'-');
        assert_eq!(first.as_bytes()[13], b'-');
        assert_eq!(first.as_bytes()[18], b'-');
        assert_eq!(first.as_bytes()[23], b'-');
    }

    #[test]
    fn proc_sys_kernel_random_table_matches_linux_entry_names() {
        let sys = new_sys_dir();
        let kernel = lookup(&sys, "kernel").expect("kernel/ subdir");
        let random = lookup(&kernel, "random").expect("random/ subdir");
        for name in [
            "poolsize",
            "entropy_avail",
            "write_wakeup_threshold",
            "urandom_min_reseed_secs",
            "boot_id",
            "uuid",
        ] {
            assert!(lookup(&random, name).is_some(), "{name} sysctl missing");
        }
    }
}
