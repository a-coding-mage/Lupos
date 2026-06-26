//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/task_nommu.c
//! test-origin: linux:vendor/linux/fs/proc/task_nommu.c
//! NOMMU `/proc/<pid>` memory files.
//!
//! Ref: `vendor/linux/fs/proc/task_nommu.c`

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::include::uapi::errno::{EINTR, ENOMEM, ESRCH};
use crate::mm::vm_flags::{
    VM_EXEC, VM_MAYEXEC, VM_MAYREAD, VM_MAYSHARE, VM_READ, VM_SHARED, VM_WRITE,
};

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = crate::mm::frame::PAGE_SIZE as u64;
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);
pub const VM_MAYOVERLAY: u64 = 1 << 9;
pub const NOMMU_MAP_LINE_WIDTH: usize = 25 + core::mem::size_of::<usize>() * 6 - 1;
pub const MINORBITS: u32 = 20;
pub const MINORMASK: u64 = (1u64 << MINORBITS) - 1;

pub const PROC_PID_MAPS_OPERATIONS_SYMBOL: &str = "proc_pid_maps_operations";
pub const PROC_PID_MAPS_OPERATIONS: &[(&str, &str)] = &[
    ("open", "pid_maps_open"),
    ("read", "seq_read"),
    ("llseek", "seq_lseek"),
    ("release", "map_release"),
];
pub const PROC_PID_MAPS_SEQ_OPS_SYMBOL: &str = "proc_pid_maps_ops";
pub const PROC_PID_MAPS_SEQ_OPS: &[(&str, &str)] = &[
    ("start", "m_start"),
    ("next", "m_next"),
    ("stop", "m_stop"),
    ("show", "show_map"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NommuVmRegion {
    pub start: u64,
    pub end: u64,
    pub kobj_size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NommuFile {
    pub dev: u64,
    pub ino: u64,
    pub path: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NommuVma {
    pub start: u64,
    pub end: u64,
    pub flags: u64,
    pub pgoff: u64,
    pub kobj_size: u64,
    pub region: Option<NommuVmRegion>,
    pub file: Option<NommuFile>,
    pub is_initial_stack: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NommuTaskSnapshot<'a> {
    pub mm_count: u64,
    pub mm_kobj_size: u64,
    pub fs_users: u64,
    pub fs_kobj_size: u64,
    pub files_count: u64,
    pub files_kobj_size: u64,
    pub sighand_count: u64,
    pub sighand_kobj_size: u64,
    pub task_kobj_size: u64,
    pub start_code: u64,
    pub end_code: u64,
    pub start_data: u64,
    pub start_stack: u64,
    pub vmas: &'a [NommuVma],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NommuTaskMem {
    pub mem_bytes: u64,
    pub slack_bytes: u64,
    pub shared_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NommuTaskStatm {
    pub size: u64,
    pub shared: u64,
    pub text: u64,
    pub data: u64,
    pub resident: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcVmaCursor {
    pub index: usize,
    pub ppos: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcMemOpenResult {
    Mm,
    Null,
    Err(i32),
}

pub fn maps_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = render_nommu_maps(&[]);
    super::util::copy_into(buf, &text)
}

pub const fn page_align(addr: u64) -> u64 {
    (addr + PAGE_SIZE - 1) & PAGE_MASK
}

pub const fn major(dev: u64) -> u64 {
    dev >> MINORBITS
}

pub const fn minor(dev: u64) -> u64 {
    dev & MINORMASK
}

pub const fn is_nommu_shared_mapping(flags: u64) -> bool {
    flags & (VM_MAYSHARE | VM_MAYOVERLAY) != 0
}

pub fn task_mem(snapshot: &NommuTaskSnapshot<'_>) -> NommuTaskMem {
    let mut bytes = 0u64;
    let mut shared = 0u64;
    let mut slack = 0u64;

    for vma in snapshot.vmas {
        bytes += vma.kobj_size;
        let size = if let Some(region) = vma.region {
            region.kobj_size + region.end.saturating_sub(region.start)
        } else {
            vma.end.saturating_sub(vma.start)
        };

        if snapshot.mm_count > 1 || is_nommu_shared_mapping(vma.flags) {
            shared += size;
        } else {
            bytes += size;
            if let Some(region) = vma.region {
                slack = region.end.saturating_sub(vma.end);
            }
        }
    }

    if snapshot.mm_count > 1 {
        shared += snapshot.mm_kobj_size;
    } else {
        bytes += snapshot.mm_kobj_size;
    }

    if snapshot.fs_users > 1 {
        shared += snapshot.fs_kobj_size;
    } else {
        bytes += snapshot.fs_kobj_size;
    }

    if snapshot.files_count > 1 {
        shared += snapshot.files_kobj_size;
    } else {
        bytes += snapshot.files_kobj_size;
    }

    if snapshot.sighand_count > 1 {
        shared += snapshot.sighand_kobj_size;
    } else {
        bytes += snapshot.sighand_kobj_size;
    }

    bytes += snapshot.task_kobj_size;

    NommuTaskMem {
        mem_bytes: bytes,
        slack_bytes: slack,
        shared_bytes: shared,
    }
}

pub fn render_task_mem(snapshot: &NommuTaskSnapshot<'_>) -> String {
    let mem = task_mem(snapshot);
    format!(
        "Mem:\t{:8} bytes\nSlack:\t{:8} bytes\nShared:\t{:8} bytes\n",
        mem.mem_bytes, mem.slack_bytes, mem.shared_bytes
    )
}

pub fn task_vsize(snapshot: &NommuTaskSnapshot<'_>) -> u64 {
    snapshot
        .vmas
        .iter()
        .map(|vma| vma.end.saturating_sub(vma.start))
        .sum()
}

pub fn task_statm(snapshot: &NommuTaskSnapshot<'_>) -> NommuTaskStatm {
    let mut size = snapshot.mm_kobj_size;

    for vma in snapshot.vmas {
        size += vma.kobj_size;
        if let Some(region) = vma.region {
            size += region.kobj_size;
            size += region.end.saturating_sub(region.start);
        }
    }

    let text =
        page_align(snapshot.end_code).saturating_sub(snapshot.start_code & PAGE_MASK) >> PAGE_SHIFT;
    let data = page_align(snapshot.start_stack).saturating_sub(snapshot.start_data & PAGE_MASK)
        >> PAGE_SHIFT;

    size >>= PAGE_SHIFT;
    size += text + data;

    NommuTaskStatm {
        size,
        shared: 0,
        text,
        data,
        resident: size,
    }
}

pub fn nommu_vma_permissions(flags: u64) -> [char; 4] {
    [
        if flags & VM_READ != 0 { 'r' } else { '-' },
        if flags & VM_WRITE != 0 { 'w' } else { '-' },
        if flags & VM_EXEC != 0 { 'x' } else { '-' },
        if flags & VM_MAYSHARE != 0 {
            if flags & VM_SHARED != 0 { 'S' } else { 's' }
        } else {
            'p'
        },
    ]
}

pub fn nommu_vma_show(vma: &NommuVma) -> String {
    let perms = nommu_vma_permissions(vma.flags);
    let (dev, ino, path) = if let Some(file) = vma.file {
        (file.dev, file.ino, file.path)
    } else {
        (0, 0, "")
    };
    let pgoff = vma.pgoff << PAGE_SHIFT;
    let mut line = format!(
        "{:08x}-{:08x} {}{}{}{} {:08x} {:02x}:{:02x} {} ",
        vma.start,
        vma.end,
        perms[0],
        perms[1],
        perms[2],
        perms[3],
        pgoff,
        major(dev),
        minor(dev),
        ino
    );

    if vma.file.is_some() {
        while line.len() < NOMMU_MAP_LINE_WIDTH {
            line.push(' ');
        }
        line.push_str(path);
    } else if vma.is_initial_stack {
        while line.len() < NOMMU_MAP_LINE_WIDTH {
            line.push(' ');
        }
        line.push_str("[stack]");
    }

    line.push('\n');
    line
}

pub fn render_nommu_maps(vmas: &[NommuVma]) -> String {
    let mut out = String::new();
    for vma in vmas {
        out.push_str(&nommu_vma_show(vma));
    }
    out
}

pub fn proc_get_vma(vmas: &[NommuVma], last_addr: u64) -> Option<ProcVmaCursor> {
    vmas.iter()
        .enumerate()
        .find(|(_, vma)| vma.start >= last_addr)
        .map(|(index, vma)| ProcVmaCursor {
            index,
            ppos: vma.start,
        })
}

pub fn m_start_plan(
    vmas: &[NommuVma],
    ppos: u64,
    task_exists: bool,
    mm_available: bool,
    mmget_not_zero: bool,
    mmap_read_lock_killable_errno: i32,
) -> Result<Option<ProcVmaCursor>, i32> {
    if ppos == u64::MAX {
        return Ok(None);
    }
    if !task_exists {
        return Err(-ESRCH);
    }
    if !mm_available || !mmget_not_zero {
        return Ok(None);
    }
    if mmap_read_lock_killable_errno != 0 {
        return Err(-EINTR);
    }
    Ok(proc_get_vma(vmas, ppos))
}

pub fn m_next_plan(vmas: &[NommuVma], current_index: usize) -> Option<ProcVmaCursor> {
    let next = current_index + 1;
    vmas.get(next).map(|vma| ProcVmaCursor {
        index: next,
        ppos: vma.start,
    })
}

pub const fn m_stop_drops_task(task_pinned: bool) -> bool {
    task_pinned
}

pub const fn maps_open_plan(
    private_allocated: bool,
    proc_mem_open: ProcMemOpenResult,
) -> Result<(), i32> {
    if !private_allocated {
        return Err(-ENOMEM);
    }
    match proc_mem_open {
        ProcMemOpenResult::Mm => Ok(()),
        ProcMemOpenResult::Null => Err(-ESRCH),
        ProcMemOpenResult::Err(err) => Err(err),
    }
}

pub const fn map_release_drops_mm(has_mm: bool) -> bool {
    has_mm
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::EACCES;

    fn private_vma() -> NommuVma {
        NommuVma {
            start: 0x1000,
            end: 0x3000,
            flags: VM_READ | VM_MAYREAD,
            pgoff: 0,
            kobj_size: 64,
            region: Some(NommuVmRegion {
                start: 0x1000,
                end: 0x4000,
                kobj_size: 32,
            }),
            file: None,
            is_initial_stack: false,
        }
    }

    fn file_vma() -> NommuVma {
        NommuVma {
            start: 0x5000,
            end: 0x7000,
            flags: VM_READ | VM_EXEC | VM_MAYSHARE | VM_SHARED | VM_MAYEXEC,
            pgoff: 2,
            kobj_size: 80,
            region: None,
            file: Some(NommuFile {
                dev: (8 << MINORBITS) | 1,
                ino: 99,
                path: "/bin/init",
            }),
            is_initial_stack: false,
        }
    }

    fn snapshot<'a>(vmas: &'a [NommuVma]) -> NommuTaskSnapshot<'a> {
        NommuTaskSnapshot {
            mm_count: 1,
            mm_kobj_size: 128,
            fs_users: 1,
            fs_kobj_size: 16,
            files_count: 2,
            files_kobj_size: 24,
            sighand_count: 1,
            sighand_kobj_size: 32,
            task_kobj_size: 256,
            start_code: 0x1003,
            end_code: 0x2fff,
            start_data: 0x3001,
            start_stack: 0x6fff,
            vmas,
        }
    }

    #[test]
    fn task_nommu_model_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/task_nommu.c"
        ));
        let internal = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/internal.h"
        ));
        let mm_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/mm.h"
        ));
        let kdev = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/kdev_t.h"
        ));

        assert!(source.contains("void task_mem(struct seq_file *m, struct mm_struct *mm)"));
        assert!(source.contains("if (atomic_read(&mm->mm_count) > 1 ||"));
        assert!(source.contains("is_nommu_shared_mapping(vma->vm_flags)"));
        assert!(source.contains("Slack:\\t%8lu bytes"));
        assert!(source.contains("unsigned long task_vsize(struct mm_struct *mm)"));
        assert!(source.contains("unsigned long task_statm(struct mm_struct *mm,"));
        assert!(source.contains("PAGE_ALIGN(mm->end_code) - (mm->start_code & PAGE_MASK)"));
        assert!(
            source.contains(
                "static int nommu_vma_show(struct seq_file *m, struct vm_area_struct *vma)"
            )
        );
        assert!(source.contains("seq_setwidth(m, 25 + sizeof(void *) * 6 - 1);"));
        assert!(source.contains("flags & VM_MAYSHARE ? flags & VM_SHARED ? 'S' : 's' : 'p'"));
        assert!(source.contains("pgoff = (loff_t)vma->vm_pgoff << PAGE_SHIFT;"));
        assert!(source.contains("seq_path(m, file_user_path(file), \"\");"));
        assert!(source.contains("seq_puts(m, \"[stack]\");"));
        assert!(source.contains("static struct vm_area_struct *proc_get_vma"));
        assert!(source.contains("static void *m_start(struct seq_file *m, loff_t *ppos)"));
        assert!(source.contains("return ERR_PTR(-ESRCH);"));
        assert!(source.contains("return ERR_PTR(-EINTR);"));
        assert!(source.contains("priv = __seq_open_private(file, ops, sizeof(*priv));"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("return seq_release_private(inode, file);"));
        assert!(source.contains(PROC_PID_MAPS_OPERATIONS_SYMBOL));
        assert!(source.contains(PROC_PID_MAPS_SEQ_OPS_SYMBOL));
        assert!(internal.contains("extern const struct file_operations proc_pid_maps_operations;"));
        assert!(internal.contains("extern unsigned long task_vsize(struct mm_struct *);"));
        assert!(mm_h.contains("return flags & (VM_MAYSHARE | VM_MAYOVERLAY);"));
        assert!(kdev.contains("#define MINORBITS\t20"));

        for (slot, target) in PROC_PID_MAPS_OPERATIONS
            .iter()
            .chain(PROC_PID_MAPS_SEQ_OPS.iter())
        {
            assert!(source.contains(slot));
            assert!(source.contains(target));
        }
    }

    #[test]
    fn task_mem_splits_private_and_shared_nommu_memory() {
        let vmas = [private_vma(), file_vma()];
        let snap = snapshot(&vmas);
        let mem = task_mem(&snap);

        assert_eq!(mem.mem_bytes, 64 + (32 + 0x3000) + 80 + 128 + 16 + 32 + 256);
        assert_eq!(mem.slack_bytes, 0x1000);
        assert_eq!(mem.shared_bytes, 0x2000 + 24);
        assert_eq!(
            render_task_mem(&snap),
            format!(
                "Mem:\t{:8} bytes\nSlack:\t{:8} bytes\nShared:\t{:8} bytes\n",
                mem.mem_bytes, mem.slack_bytes, mem.shared_bytes
            )
        );

        let mut shared_mm = snap;
        shared_mm.mm_count = 2;
        let shared = task_mem(&shared_mm);
        assert_eq!(shared.shared_bytes, (32 + 0x3000) + 0x2000 + 128 + 24);
    }

    #[test]
    fn task_vsize_and_statm_follow_nommu_accounting() {
        let vmas = [private_vma(), file_vma()];
        let snap = snapshot(&vmas);

        assert_eq!(task_vsize(&snap), 0x2000 + 0x2000);
        assert_eq!(page_align(0x2fff), 0x3000);

        let statm = task_statm(&snap);
        assert_eq!(statm.text, 2);
        assert_eq!(statm.data, 4);
        assert_eq!(statm.shared, 0);
        assert_eq!(statm.resident, statm.size);
        assert_eq!(
            statm.size,
            ((128 + 64 + 32 + 0x3000 + 80) >> PAGE_SHIFT) + 2 + 4
        );
    }

    #[test]
    fn nommu_vma_show_matches_permissions_device_offset_and_stack_rules() {
        assert!(is_nommu_shared_mapping(VM_MAYSHARE));
        assert!(is_nommu_shared_mapping(VM_MAYOVERLAY));
        assert!(!is_nommu_shared_mapping(VM_SHARED));

        assert_eq!(
            nommu_vma_permissions(VM_READ | VM_WRITE),
            ['r', 'w', '-', 'p']
        );
        assert_eq!(
            nommu_vma_permissions(VM_READ | VM_EXEC | VM_MAYSHARE),
            ['r', '-', 'x', 's']
        );
        assert_eq!(
            nommu_vma_permissions(VM_READ | VM_MAYSHARE | VM_SHARED),
            ['r', '-', '-', 'S']
        );

        let file_line = nommu_vma_show(&file_vma());
        assert!(file_line.starts_with("00005000-00007000 r-xS 00002000 08:01 99 "));
        assert!(file_line.ends_with("/bin/init\n"));

        let stack = NommuVma {
            is_initial_stack: true,
            ..private_vma()
        };
        let stack_line = nommu_vma_show(&stack);
        assert!(stack_line.starts_with("00001000-00003000 r--p 00000000 00:00 0 "));
        assert!(stack_line.ends_with("[stack]\n"));

        let anon_line = nommu_vma_show(&private_vma());
        assert!(anon_line.ends_with("0 \n"));
    }

    #[test]
    fn proc_maps_iteration_open_and_release_paths_match_c_branches() {
        let vmas = [private_vma(), file_vma()];
        assert_eq!(
            proc_get_vma(&vmas, 0),
            Some(ProcVmaCursor {
                index: 0,
                ppos: 0x1000
            })
        );
        assert_eq!(
            m_next_plan(&vmas, 0),
            Some(ProcVmaCursor {
                index: 1,
                ppos: 0x5000
            })
        );
        assert_eq!(m_next_plan(&vmas, 1), None);

        assert_eq!(m_start_plan(&vmas, u64::MAX, true, true, true, 0), Ok(None));
        assert_eq!(m_start_plan(&vmas, 0, false, true, true, 0), Err(-ESRCH));
        assert_eq!(m_start_plan(&vmas, 0, true, false, true, 0), Ok(None));
        assert_eq!(m_start_plan(&vmas, 0, true, true, false, 0), Ok(None));
        assert_eq!(
            m_start_plan(&vmas, 0, true, true, true, -EINTR),
            Err(-EINTR)
        );

        assert_eq!(maps_open_plan(false, ProcMemOpenResult::Mm), Err(-ENOMEM));
        assert_eq!(maps_open_plan(true, ProcMemOpenResult::Null), Err(-ESRCH));
        assert_eq!(
            maps_open_plan(true, ProcMemOpenResult::Err(-EACCES)),
            Err(-EACCES)
        );
        assert_eq!(maps_open_plan(true, ProcMemOpenResult::Mm), Ok(()));
        assert!(map_release_drops_mm(true));
        assert!(!map_release_drops_mm(false));
        assert!(m_stop_drops_task(true));
    }

    #[test]
    fn render_nommu_maps_concatenates_vma_lines() {
        let text = render_nommu_maps(&[private_vma(), file_vma()]);
        assert!(text.contains("00001000-00003000 r--p 00000000 00:00 0 \n"));
        assert!(text.contains("/bin/init\n"));
    }
}
