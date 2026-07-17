//! linux-parity: partial
//! linux-source: vendor/linux/ipc/shm.c
//! test-origin: linux:vendor/linux/ipc/shm.c
//! SysV shared-memory limits, lifecycle flags, and attach/detach accounting.

extern crate alloc;

use alloc::{string::String, vec::Vec};

pub const SHMMIN: usize = 1;
pub const SHMMNI: usize = 4096;
pub const SHMMAX: usize = usize::MAX - (1usize << 24);
pub const SHMALL: usize = usize::MAX - (1usize << 24);
pub const SHMSEG: usize = SHMMNI;
pub const SHM_R: u16 = 0o400;
pub const SHM_W: u16 = 0o200;
pub const SHM_HUGETLB: i32 = 0o4000;
pub const SHM_NORESERVE: i32 = 0o10000;
pub const SHM_RDONLY: i32 = 0o10000;
pub const SHM_RND: i32 = 0o20000;
pub const SHM_REMAP: i32 = 0o40000;
pub const SHM_EXEC: i32 = 0o100000;
pub const SHM_DEST: u16 = 0o1000;
pub const SHM_LOCKED: u16 = 0o2000;
pub const SHM_LOCK: i32 = 11;
pub const SHM_UNLOCK: i32 = 12;
pub const SHM_STAT: i32 = 13;
pub const SHM_INFO: i32 = 14;
pub const SHM_STAT_ANY: i32 = 15;
pub const IPC_CREAT: i32 = 0o1000;
pub const IPC_RMID: i32 = 0;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShmSegment {
    pub id: i32,
    pub key: i32,
    pub size: usize,
    pub mode: u16,
    pub creator_pid: i32,
    pub last_pid: i32,
    pub nattch: usize,
    pub atime: i64,
    pub dtime: i64,
    pub ctime: i64,
    pub huge_tlb: bool,
    pub no_reserve: bool,
    pub destroy_on_detach: bool,
    pub locked: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShmNamespace {
    pub ctlmax: usize,
    pub ctlall: usize,
    pub ctlmni: usize,
    pub segments: Vec<ShmSegment>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShmError {
    InvalidSize,
    NoSpace,
    NotFound,
    WouldOverlap,
    Permission,
}

impl ShmNamespace {
    pub fn new() -> Self {
        Self {
            ctlmax: SHMMAX,
            ctlall: SHMALL,
            ctlmni: SHMMNI,
            segments: Vec::new(),
        }
    }

    pub fn shmget(
        &mut self,
        key: i32,
        size: usize,
        shmflg: i32,
        pid: i32,
        now: i64,
    ) -> Result<i32, ShmError> {
        if size < SHMMIN || size > self.ctlmax {
            return Err(ShmError::InvalidSize);
        }
        if self.segments.len() >= self.ctlmni {
            return Err(ShmError::NoSpace);
        }
        let id = self.segments.len() as i32;
        self.segments.push(ShmSegment {
            id,
            key,
            size,
            mode: (shmflg as u16) & 0o777,
            creator_pid: pid,
            last_pid: pid,
            nattch: 0,
            atime: 0,
            dtime: 0,
            ctime: now,
            huge_tlb: shmflg & SHM_HUGETLB != 0,
            no_reserve: shmflg & SHM_NORESERVE != 0,
            destroy_on_detach: false,
            locked: false,
        });
        Ok(id)
    }

    pub fn segment(&self, id: i32) -> Option<&ShmSegment> {
        self.segments.iter().find(|seg| seg.id == id)
    }

    pub fn segment_mut(&mut self, id: i32) -> Option<&mut ShmSegment> {
        self.segments.iter_mut().find(|seg| seg.id == id)
    }

    pub fn shmctl_rmid(&mut self, id: i32) -> Result<(), ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.destroy_on_detach = true;
        seg.mode |= SHM_DEST;
        if seg.nattch == 0 {
            self.segments.retain(|entry| entry.id != id);
        }
        Ok(())
    }

    pub fn shmctl_lock(&mut self, id: i32, lock: bool) -> Result<(), ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.locked = lock;
        if lock {
            seg.mode |= SHM_LOCKED;
        } else {
            seg.mode &= !SHM_LOCKED;
        }
        Ok(())
    }

    pub fn shmat(&mut self, id: i32, shmflg: i32, pid: i32, now: i64) -> Result<Attach, ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.nattch += 1;
        seg.last_pid = pid;
        seg.atime = now;
        Ok(Attach {
            shmid: id,
            readonly: shmflg & SHM_RDONLY != 0,
            executable: shmflg & SHM_EXEC != 0,
            remap: shmflg & SHM_REMAP != 0,
        })
    }

    pub fn shmdt(&mut self, id: i32, pid: i32, now: i64) -> Result<(), ShmError> {
        let seg = self.segment_mut(id).ok_or(ShmError::NotFound)?;
        seg.nattch = seg.nattch.saturating_sub(1);
        seg.last_pid = pid;
        seg.dtime = now;
        if seg.nattch == 0 && seg.destroy_on_detach {
            self.segments.retain(|entry| entry.id != id);
        }
        Ok(())
    }

    pub fn proc_sysvipc_shm_header(word_bits: usize) -> &'static str {
        if word_bits == 32 {
            "       key      shmid perms       size  cpid  lpid nattch   uid   gid  cuid  cgid      atime      dtime      ctime        rss       swap\n"
        } else {
            "       key      shmid perms                  size  cpid  lpid nattch   uid   gid  cuid  cgid      atime      dtime      ctime                   rss                  swap\n"
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Attach {
    pub shmid: i32,
    pub readonly: bool,
    pub executable: bool,
    pub remap: bool,
}

pub fn hugepage_shm_test_length() -> usize {
    256usize * 1024 * 1024
}

pub fn sysvipc_shm_empty_proc(word_bits: usize) -> String {
    String::from(ShmNamespace::proc_sysvipc_shm_header(word_bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysv_shm_rules_matches_linux_source_and_original_selftests() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/shm.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/shm.h"
        ));
        let setns_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/proc/setns-sysvipc.c"
        ));
        let hugepage_selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/mm/hugepage-shm.c"
        ));
        let ipc_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/powerpc/syscalls/ipc.h"
        ));

        assert!(source.contains("#define SHM_DEST\t01000"));
        assert!(source.contains("#define SHM_LOCKED\t02000"));
        assert!(source.contains("ns->shm_ctlmax = SHMMAX;"));
        assert!(source.contains("ns->shm_ctlall = SHMALL;"));
        assert!(source.contains("ns->shm_ctlmni = SHMMNI;"));
        assert!(source.contains("if (size < SHMMIN || size > ns->shm_ctlmax)"));
        assert!(source.contains("if (shmflg & SHM_HUGETLB) {"));
        assert!(source.contains("const bool has_no_reserve = shmflg & SHM_NORESERVE;"));
        assert!(source.contains("case SHM_STAT_ANY:"));
        assert!(source.contains("case SHM_LOCK:"));
        assert!(source.contains("SYSCALL_DEFINE3(shmget"));
        assert!(source.contains("SYSCALL_DEFINE3(shmat"));
        assert!(source.contains("SYSCALL_DEFINE1(shmdt"));
        assert!(header.contains("#define SHMMIN 1"));
        assert!(header.contains("#define SHM_HUGETLB\t04000"));
        assert!(header.contains("#define\tSHM_RDONLY\t010000"));
        assert!(
            setns_selftest
                .contains("Test that setns(CLONE_NEWIPC) points to new /proc/sysvipc content")
        );
        assert!(setns_selftest.contains("shmget(IPC_PRIVATE, 1, IPC_CREAT)"));
        assert!(setns_selftest.contains("open(\"/proc/sysvipc/shm\", O_RDONLY)"));
        assert!(setns_selftest.contains("#define S32"));
        assert!(setns_selftest.contains("#define S64"));
        assert!(hugepage_selftest.contains("#define LENGTH (256UL*1024*1024)"));
        assert!(
            hugepage_selftest
                .contains("shmget(2, LENGTH, SHM_HUGETLB | IPC_CREAT | SHM_R | SHM_W)")
        );
        assert!(hugepage_selftest.contains("shmat(shmid, ADDR, SHMAT_FLAGS)"));
        assert!(hugepage_selftest.contains("shmctl(shmid, IPC_RMID, NULL);"));
        assert!(ipc_h.contains("DO_TEST(shmat, __NR_shmat)"));
        assert!(ipc_h.contains("DO_TEST(shmdt, __NR_shmdt)"));
        assert!(ipc_h.contains("DO_TEST(shmget, __NR_shmget)"));
        assert!(ipc_h.contains("DO_TEST(shmctl, __NR_shmctl)"));

        let mut ns = ShmNamespace::new();
        let shmid = ns
            .shmget(
                2,
                hugepage_shm_test_length(),
                SHM_HUGETLB | IPC_CREAT | SHM_R as i32 | SHM_W as i32,
                10,
                1,
            )
            .unwrap();
        let seg = ns.segment(shmid).unwrap();
        assert!(seg.huge_tlb);
        assert_eq!(seg.size, hugepage_shm_test_length());

        let attach = ns.shmat(shmid, SHM_RDONLY, 11, 2).unwrap();
        assert!(attach.readonly);
        assert_eq!(ns.segment(shmid).unwrap().nattch, 1);
        ns.shmctl_rmid(shmid).unwrap();
        assert!(ns.segment(shmid).unwrap().destroy_on_detach);
        ns.shmdt(shmid, 11, 3).unwrap();
        assert!(ns.segment(shmid).is_none());

        assert_eq!(
            sysvipc_shm_empty_proc(32),
            ShmNamespace::proc_sysvipc_shm_header(32)
        );
        assert_eq!(
            sysvipc_shm_empty_proc(64),
            ShmNamespace::proc_sysvipc_shm_header(64)
        );
        assert_eq!(ns.shmget(1, 0, IPC_CREAT, 1, 0), Err(ShmError::InvalidSize));
    }
}
