//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/util.c
//! test-origin: linux:vendor/linux/fs/proc/util.c
//! Shared procfs formatting helpers.
//!
//! Ref: `vendor/linux/fs/proc/util.c`

extern crate alloc;

use alloc::{format, string::String, sync::Arc};
use core::sync::atomic::Ordering;

use crate::fs::kernfs::KernfsNode;
use crate::kernel::{
    capability::KernelCapT,
    cred::{Cred, INIT_CRED},
    task::TaskStruct,
};

pub type ProcShow = fn(&Arc<KernfsNode>, &mut [u8]) -> Result<usize, i32>;

pub struct ProcStatusSecurity {
    pub uid: [u32; 4],
    pub gid: [u32; 4],
    pub cap_inheritable: KernelCapT,
    pub cap_permitted: KernelCapT,
    pub cap_effective: KernelCapT,
    pub cap_bset: KernelCapT,
    pub cap_ambient: KernelCapT,
    pub no_new_privs: u32,
    pub seccomp_mode: u32,
    pub seccomp_filters: usize,
}

pub struct ProcStatusView<'a> {
    pub name: &'a str,
    pub state: &'a str,
    pub tgid: i32,
    pub pid: i32,
    pub ppid: i32,
    pub locked_kb: u64,
    pub rss_anon_kb: u64,
    pub security: ProcStatusSecurity,
}

pub fn name_to_int(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut len = bytes.len();
    if len == 0 || (len > 1 && bytes[0] == b'0') {
        return u32::MAX;
    }

    let mut n = 0u32;
    let mut idx = 0usize;
    loop {
        let c = bytes[idx].wrapping_sub(b'0');
        if c > 9 || n >= (u32::MAX - 9) / 10 {
            return u32::MAX;
        }
        n = n * 10 + c as u32;
        len -= 1;
        if len == 0 {
            return n;
        }
        idx += 1;
    }
}

pub fn copy_into(buf: &mut [u8], s: &str) -> Result<usize, i32> {
    let n = s.len().min(buf.len());
    buf[..n].copy_from_slice(&s.as_bytes()[..n]);
    Ok(n)
}

pub fn task_comm(task: *mut TaskStruct) -> String {
    if task.is_null() {
        return String::from("lupos");
    }
    let bytes = unsafe { &(*task).comm };
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    match core::str::from_utf8(&bytes[..end]) {
        Ok("") | Err(_) => String::from("lupos"),
        Ok(comm) => String::from(comm),
    }
}

pub fn task_locked_vm_kb(task: *mut TaskStruct) -> u64 {
    if task.is_null() {
        return 0;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return 0;
    }
    unsafe { (*mm).locked_vm.saturating_mul(4) }
}

pub fn task_rss_anon_kb(task: *mut TaskStruct) -> u64 {
    if task.is_null() {
        return 0;
    }
    let mm = unsafe { (*task).mm };
    if mm.is_null() {
        return 0;
    }
    let mm = unsafe { &*mm };
    let mut kb = 0u64;
    for (_, _, entry) in mm.mm_mt.collect_entries() {
        let vma = unsafe { &*(entry as *const crate::mm::mm_types::VmAreaStruct) };
        if vma.vm_file != 0 || vma.vm_flags & crate::mm::vm_flags::VM_HUGEPAGE == 0 {
            continue;
        }
        if crate::mm::huge::thp_range_was_split(vma.vm_start, vma.vm_end) {
            continue;
        }
        kb = kb.saturating_add((vma.vm_end - vma.vm_start) / 1024);
    }
    kb
}

pub fn task_status_security(task: *mut TaskStruct) -> ProcStatusSecurity {
    let cred = task_cred(task);
    let seccomp = if task.is_null() {
        (0, 0, 0)
    } else {
        unsafe {
            (
                (*task).m27.no_new_privs,
                (*task).m27_seccomp.mode.load(Ordering::Acquire),
                seccomp_filter_count((*task).m27_seccomp.filter.load(Ordering::Acquire)),
            )
        }
    };
    unsafe {
        ProcStatusSecurity {
            uid: [
                (*cred).uid.0,
                (*cred).euid.0,
                (*cred).suid.0,
                (*cred).fsuid.0,
            ],
            gid: [
                (*cred).gid.0,
                (*cred).egid.0,
                (*cred).sgid.0,
                (*cred).fsgid.0,
            ],
            cap_inheritable: (*cred).cap_inheritable,
            cap_permitted: (*cred).cap_permitted,
            cap_effective: (*cred).cap_effective,
            cap_bset: (*cred).cap_bset,
            cap_ambient: (*cred).cap_ambient,
            no_new_privs: seccomp.0,
            seccomp_mode: seccomp.1,
            seccomp_filters: seccomp.2,
        }
    }
}

pub fn format_status(view: &ProcStatusView<'_>) -> String {
    format!(
        "Name:\t{}\nState:\t{}\nTgid:\t{}\nPid:\t{}\nPPid:\t{}\nUid:\t{}\t{}\t{}\t{}\nGid:\t{}\t{}\t{}\t{}\nVmLck:\t{:8} kB\nRssAnon:\t{:8} kB\nCapInh:\t{:016x}\nCapPrm:\t{:016x}\nCapEff:\t{:016x}\nCapBnd:\t{:016x}\nCapAmb:\t{:016x}\nNoNewPrivs:\t{}\nSeccomp:\t{}\nSeccomp_filters:\t{}\n",
        view.name,
        view.state,
        view.tgid,
        view.pid,
        view.ppid,
        view.security.uid[0],
        view.security.uid[1],
        view.security.uid[2],
        view.security.uid[3],
        view.security.gid[0],
        view.security.gid[1],
        view.security.gid[2],
        view.security.gid[3],
        view.locked_kb,
        view.rss_anon_kb,
        cap_mask_hex(view.security.cap_inheritable),
        cap_mask_hex(view.security.cap_permitted),
        cap_mask_hex(view.security.cap_effective),
        cap_mask_hex(view.security.cap_bset),
        cap_mask_hex(view.security.cap_ambient),
        view.security.no_new_privs,
        view.security.seccomp_mode,
        view.security.seccomp_filters,
    )
}

fn task_cred(task: *mut TaskStruct) -> *const Cred {
    if task.is_null() {
        return &raw const INIT_CRED;
    }
    let cred = unsafe { (*task).cred };
    if cred.is_null() {
        &raw const INIT_CRED
    } else {
        cred
    }
}

fn seccomp_filter_count(mut cursor: *mut crate::kernel::seccomp::SeccompFilter) -> usize {
    let mut count = 0usize;
    while !cursor.is_null() {
        count = count.saturating_add(1);
        cursor = unsafe { (*cursor).prev };
    }
    count
}

fn cap_mask_hex(caps: KernelCapT) -> u64 {
    ((caps.cap[1] as u64) << 32) | caps.cap[0] as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_util_name_to_int_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/util.c"
        ));
        assert!(source.contains("unsigned name_to_int(const struct qstr *qstr)"));
        assert!(source.contains("if (len > 1 && *name == '0')"));
        assert!(source.contains("if (n >= (~0U-9)/10)"));
        assert!(source.contains("return ~0U;"));

        assert_eq!(name_to_int("0"), 0);
        assert_eq!(name_to_int("42"), 42);
        assert_eq!(name_to_int("01"), u32::MAX);
        assert_eq!(name_to_int("4a"), u32::MAX);
        assert_eq!(name_to_int("429496728"), 429496728);
        assert_eq!(name_to_int("4294967280"), u32::MAX);
        assert_eq!(name_to_int(""), u32::MAX);
    }

    #[test]
    fn proc_status_formatter_emits_linux_capability_fields() {
        let text = format_status(&ProcStatusView {
            name: "systemd",
            state: "R (running)",
            tgid: 1,
            pid: 1,
            ppid: 0,
            locked_kb: 0,
            rss_anon_kb: 0,
            security: ProcStatusSecurity {
                uid: [0, 0, 0, 0],
                gid: [0, 0, 0, 0],
                cap_inheritable: KernelCapT::empty(),
                cap_permitted: KernelCapT::full(),
                cap_effective: KernelCapT::full(),
                cap_bset: KernelCapT::full(),
                cap_ambient: KernelCapT::empty(),
                no_new_privs: 0,
                seccomp_mode: 0,
                seccomp_filters: 0,
            },
        });
        assert!(text.contains("CapInh:\t0000000000000000"));
        assert!(text.contains("CapPrm:\t000001ffffffffff"));
        assert!(text.contains("CapEff:\t000001ffffffffff"));
        assert!(text.contains("CapBnd:\t000001ffffffffff"));
        assert!(text.contains("CapAmb:\t0000000000000000"));
        assert!(text.contains("NoNewPrivs:\t0"));
        assert!(text.contains("Seccomp:\t0"));
        assert!(text.contains("Seccomp_filters:\t0"));
    }
}
