//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/nommu.c
//! test-origin: linux:vendor/linux/fs/proc/nommu.c
//! NOMMU procfs memory views.
//!
//! Ref: `vendor/linux/fs/proc/nommu.c`

extern crate alloc;

use alloc::{format, string::String};

use crate::mm::vm_flags::{VM_EXEC, VM_MAYSHARE, VM_READ, VM_SHARED, VM_WRITE, VmFlags};

pub const PAGE_SHIFT: u64 = 12;
pub const NOMMU_SEQ_WIDTH: usize = 25 + core::mem::size_of::<usize>() * 6 - 1;
pub const PROC_NOMMU_MAPS_MODE: u16 = 0o444;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NommuFile {
    pub dev_major: u32,
    pub dev_minor: u32,
    pub inode: u64,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NommuRegion {
    pub vm_start: u64,
    pub vm_end: u64,
    pub vm_flags: VmFlags,
    pub vm_pgoff: u64,
    pub file: Option<NommuFile>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NommuSeqOperations {
    pub start: &'static str,
    pub next: &'static str,
    pub stop: &'static str,
    pub show: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcNommuInit {
    pub name: &'static str,
    pub mode: u16,
    pub seq_operations: NommuSeqOperations,
    pub return_value: i32,
}

pub fn enabled() -> bool {
    false
}

pub const PROC_NOMMU_REGION_LIST_SEQOP: NommuSeqOperations = NommuSeqOperations {
    start: "nommu_region_list_start",
    next: "nommu_region_list_next",
    stop: "nommu_region_list_stop",
    show: "nommu_region_list_show",
};

pub fn nommu_region_show(region: &NommuRegion) -> String {
    let file = region.file.as_ref();
    let (dev_major, dev_minor, inode) = if let Some(file) = file {
        (file.dev_major, file.dev_minor, file.inode)
    } else {
        (0, 0, 0)
    };
    let mut out = format!(
        "{:08x}-{:08x} {}{}{}{} {:08x} {:02x}:{:02x} {} ",
        region.vm_start,
        region.vm_end,
        if region.vm_flags & VM_READ != 0 {
            'r'
        } else {
            '-'
        },
        if region.vm_flags & VM_WRITE != 0 {
            'w'
        } else {
            '-'
        },
        if region.vm_flags & VM_EXEC != 0 {
            'x'
        } else {
            '-'
        },
        nommu_share_mode(region.vm_flags),
        region.vm_pgoff << PAGE_SHIFT,
        dev_major,
        dev_minor,
        inode
    );

    if let Some(file) = file {
        while out.len() < NOMMU_SEQ_WIDTH {
            out.push(' ');
        }
        out.push_str(&file.path);
    }

    out.push('\n');
    out
}

pub fn nommu_region_list_show(regions: &[NommuRegion], index: usize) -> Option<String> {
    regions.get(index).map(nommu_region_show)
}

pub fn nommu_region_list_start(regions: &[NommuRegion], position: usize) -> Option<usize> {
    if position < regions.len() {
        Some(position)
    } else {
        None
    }
}

pub fn nommu_region_list_next(
    regions: &[NommuRegion],
    current: usize,
    position: &mut usize,
) -> Option<usize> {
    *position += 1;
    let next = current + 1;
    if next < regions.len() {
        Some(next)
    } else {
        None
    }
}

pub const fn nommu_region_list_stop() {}

pub const fn proc_nommu_init() -> ProcNommuInit {
    ProcNommuInit {
        name: "maps",
        mode: PROC_NOMMU_MAPS_MODE,
        seq_operations: PROC_NOMMU_REGION_LIST_SEQOP,
        return_value: 0,
    }
}

pub const fn nommu_share_mode(flags: VmFlags) -> char {
    if flags & VM_MAYSHARE != 0 {
        if flags & VM_SHARED != 0 { 'S' } else { 's' }
    } else {
        'p'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nommu_region_format_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/nommu.c"
        ));
        assert!(source.contains("static int nommu_region_show"));
        assert!(source.contains("seq_setwidth(m, 25 + sizeof(void *) * 6 - 1);"));
        assert!(source.contains("%08lx-%08lx %c%c%c%c %08llx %02x:%02x %lu "));
        assert!(source.contains("flags & VM_READ ? 'r' : '-'"));
        assert!(source.contains("flags & VM_MAYSHARE ? flags & VM_SHARED ? 'S' : 's' : 'p'"));
        assert!(source.contains("((loff_t)region->vm_pgoff) << PAGE_SHIFT"));
        assert!(source.contains("seq_path(m, file_user_path(file), \"\");"));

        let region = NommuRegion {
            vm_start: 0x1000,
            vm_end: 0x3000,
            vm_flags: VM_READ | VM_WRITE | VM_MAYSHARE,
            vm_pgoff: 2,
            file: Some(NommuFile {
                dev_major: 8,
                dev_minor: 1,
                inode: 42,
                path: String::from("/bin/app"),
            }),
        };
        let line = nommu_region_show(&region);
        assert!(line.starts_with("00001000-00003000 rw-s 00002000 08:01 42 "));
        assert!(line.ends_with("/bin/app\n"));
        assert!(line.find("/bin/app").unwrap() >= NOMMU_SEQ_WIDTH);

        let private = NommuRegion {
            file: None,
            vm_flags: VM_EXEC,
            ..region
        };
        assert_eq!(
            nommu_region_show(&private),
            "00001000-00003000 --xp 00002000 00:00 0 \n"
        );
    }

    #[test]
    fn nommu_seq_iteration_and_init_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/nommu.c"
        ));
        assert!(source.contains("down_read(&nommu_region_sem);"));
        assert!(source.contains("for (p = rb_first(&nommu_region_tree); p; p = rb_next(p))"));
        assert!(source.contains("up_read(&nommu_region_sem);"));
        assert!(source.contains("return rb_next((struct rb_node *) v);"));
        assert!(source.contains("static const struct seq_operations proc_nommu_region_list_seqop"));
        assert!(source.contains("proc_create_seq(\"maps\", S_IRUGO, NULL"));
        assert!(source.contains("fs_initcall(proc_nommu_init);"));

        let regions = [
            NommuRegion {
                vm_start: 0,
                vm_end: 1,
                vm_flags: VM_READ,
                vm_pgoff: 0,
                file: None,
            },
            NommuRegion {
                vm_start: 2,
                vm_end: 3,
                vm_flags: VM_WRITE,
                vm_pgoff: 0,
                file: None,
            },
        ];
        let mut pos = 0;
        let first = nommu_region_list_start(&regions, pos);
        assert_eq!(first, Some(0));
        assert_eq!(
            nommu_region_list_next(&regions, first.unwrap(), &mut pos),
            Some(1)
        );
        assert_eq!(pos, 1);
        assert_eq!(nommu_region_list_next(&regions, 1, &mut pos), None);
        assert_eq!(nommu_region_list_start(&regions, 2), None);
        assert!(
            nommu_region_list_show(&regions, 1)
                .unwrap()
                .starts_with("00000002-00000003 -w-p")
        );
        assert_eq!(
            proc_nommu_init(),
            ProcNommuInit {
                name: "maps",
                mode: 0o444,
                seq_operations: PROC_NOMMU_REGION_LIST_SEQOP,
                return_value: 0,
            }
        );
        assert!(!enabled());
    }
}
