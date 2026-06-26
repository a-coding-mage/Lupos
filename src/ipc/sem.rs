//! linux-parity: partial
//! linux-source: vendor/linux/ipc/sem.c
//! test-origin: linux:vendor/linux/ipc/sem.c
//! SysV semaphore limits, semctl commands, and basic semop behavior.

extern crate alloc;

use alloc::vec::Vec;

pub const SEM_UNDO: i16 = 0x1000;
pub const GETPID: i32 = 11;
pub const GETVAL: i32 = 12;
pub const GETALL: i32 = 13;
pub const GETNCNT: i32 = 14;
pub const GETZCNT: i32 = 15;
pub const SETVAL: i32 = 16;
pub const SETALL: i32 = 17;
pub const SEM_STAT: i32 = 18;
pub const SEM_INFO: i32 = 19;
pub const SEM_STAT_ANY: i32 = 20;
pub const SEMMNI: i32 = 32_000;
pub const SEMMSL: usize = 32_000;
pub const SEMMNS: i64 = 32_000 * 32_000;
pub const SEMOPM: usize = 500;
pub const SEMVMX: i16 = 32_767;
pub const SEMMSL_FAST: usize = 256;
pub const SEMOPM_FAST: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemBuf {
    pub sem_num: usize,
    pub sem_op: i16,
    pub sem_flg: i16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Semaphore {
    pub semval: i16,
    pub sempid: i32,
    pub semncnt: usize,
    pub semzcnt: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemArray {
    pub id: i32,
    pub key: i32,
    pub mode: u16,
    pub sem_ctime: i64,
    pub sem_otime: i64,
    pub semaphores: Vec<Semaphore>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemInfo {
    pub semmni: i32,
    pub semmns: i64,
    pub semmsl: usize,
    pub semopm: usize,
    pub semvmx: i16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemError {
    InvalidSemaphoreCount,
    InvalidSemaphoreIndex,
    InvalidValue,
    TooManyOps,
    WouldBlock,
}

impl SemArray {
    pub fn new(id: i32, key: i32, nsems: usize, mode: u16, now: i64) -> Result<Self, SemError> {
        if nsems == 0 || nsems > SEMMSL {
            return Err(SemError::InvalidSemaphoreCount);
        }
        Ok(Self {
            id,
            key,
            mode,
            sem_ctime: now,
            sem_otime: 0,
            semaphores: alloc::vec![
                Semaphore {
                    semval: 0,
                    sempid: 0,
                    semncnt: 0,
                    semzcnt: 0,
                };
                nsems
            ],
        })
    }

    pub fn semctl_getval(&self, semnum: usize) -> Result<i16, SemError> {
        self.semaphores
            .get(semnum)
            .map(|sem| sem.semval)
            .ok_or(SemError::InvalidSemaphoreIndex)
    }

    pub fn semctl_setval(
        &mut self,
        semnum: usize,
        val: i16,
        pid: i32,
        now: i64,
    ) -> Result<(), SemError> {
        if !(0..=SEMVMX).contains(&val) {
            return Err(SemError::InvalidValue);
        }
        let sem = self
            .semaphores
            .get_mut(semnum)
            .ok_or(SemError::InvalidSemaphoreIndex)?;
        sem.semval = val;
        sem.sempid = pid;
        sem.semncnt = 0;
        sem.semzcnt = 0;
        self.sem_ctime = now;
        Ok(())
    }

    pub fn semctl_getall(&self) -> Vec<i16> {
        self.semaphores.iter().map(|sem| sem.semval).collect()
    }

    pub fn semctl_setall(&mut self, values: &[i16], pid: i32, now: i64) -> Result<(), SemError> {
        if values.len() != self.semaphores.len() {
            return Err(SemError::InvalidSemaphoreCount);
        }
        if values.iter().any(|val| !(0..=SEMVMX).contains(val)) {
            return Err(SemError::InvalidValue);
        }
        for (sem, val) in self.semaphores.iter_mut().zip(values.iter().copied()) {
            sem.semval = val;
            sem.sempid = pid;
            sem.semncnt = 0;
            sem.semzcnt = 0;
        }
        self.sem_ctime = now;
        Ok(())
    }

    pub fn semop(&mut self, ops: &[SemBuf], pid: i32, now: i64) -> Result<(), SemError> {
        if ops.len() > SEMOPM {
            return Err(SemError::TooManyOps);
        }

        let mut next = self.semaphores.clone();
        for op in ops {
            let sem = next
                .get_mut(op.sem_num)
                .ok_or(SemError::InvalidSemaphoreIndex)?;
            match op.sem_op.cmp(&0) {
                core::cmp::Ordering::Less => {
                    let amount = -op.sem_op;
                    if sem.semval < amount {
                        return Err(SemError::WouldBlock);
                    }
                    sem.semval -= amount;
                }
                core::cmp::Ordering::Equal => {
                    if sem.semval != 0 {
                        return Err(SemError::WouldBlock);
                    }
                }
                core::cmp::Ordering::Greater => {
                    let val = sem
                        .semval
                        .checked_add(op.sem_op)
                        .ok_or(SemError::InvalidValue)?;
                    if val > SEMVMX {
                        return Err(SemError::InvalidValue);
                    }
                    sem.semval = val;
                }
            }
            sem.sempid = pid;
        }

        self.semaphores = next;
        self.sem_otime = now;
        Ok(())
    }
}

pub const fn seminfo_defaults() -> SemInfo {
    SemInfo {
        semmni: SEMMNI,
        semmns: SEMMNS,
        semmsl: SEMMSL,
        semopm: SEMOPM,
        semvmx: SEMVMX,
    }
}

pub const fn semctl_command_requires_down(cmd: i32) -> bool {
    matches!(cmd, SETALL | IPC_SET | IPC_RMID)
}

pub const IPC_RMID: i32 = 0;
pub const IPC_SET: i32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysv_sem_rules_matches_linux_source_and_ipc_selftest_inventory() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/ipc/sem.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/sem.h"
        ));
        let ipc_unmuxed = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/powerpc/syscalls/ipc_unmuxed.c"
        ));
        let ipc_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/powerpc/syscalls/ipc.h"
        ));

        assert!(source.contains("ns->sc_semmsl = SEMMSL;"));
        assert!(source.contains("ns->sc_semmns = SEMMNS;"));
        assert!(source.contains("ns->sc_semopm = SEMOPM;"));
        assert!(source.contains("#define SEMMSL_FAST\t256"));
        assert!(source.contains("#define SEMOPM_FAST\t64"));
        assert!(
            source
                .contains("static int newary(struct ipc_namespace *ns, struct ipc_params *params)")
        );
        assert!(source.contains("if (result > SEMVMX)"));
        assert!(source.contains("if (val > SEMVMX || val < 0)"));
        assert!(source.contains("if (nsems > SEMMSL_FAST)"));
        assert!(source.contains("if (nsops > SEMOPM_FAST)"));
        assert!(source.contains("SYSCALL_DEFINE4(semctl"));
        assert!(header.contains("#define GETVAL  12"));
        assert!(header.contains("#define SETALL  17"));
        assert!(header.contains("#define SEMMNI  32000"));
        assert!(header.contains("#define SEMOPM  500"));
        assert!(
            ipc_unmuxed.contains("This test simply tests that certain syscalls are implemented")
        );
        assert!(ipc_h.contains("DO_TEST(semop, __NR_semop)"));
        assert!(ipc_h.contains("DO_TEST(semget, __NR_semget)"));
        assert!(ipc_h.contains("DO_TEST(semctl, __NR_semctl)"));
        assert!(ipc_h.contains("DO_TEST(semtimedop, __NR_semtimedop)"));

        let info = seminfo_defaults();
        assert_eq!(info.semmni, 32_000);
        assert_eq!(info.semopm, 500);
        assert_eq!(info.semvmx, 32_767);

        let mut sem = SemArray::new(1, 42, 2, 0o666, 10).unwrap();
        assert_eq!(sem.semctl_getall(), alloc::vec![0, 0]);
        sem.semctl_setval(0, 3, 100, 11).unwrap();
        assert_eq!(sem.semctl_getval(0), Ok(3));
        sem.semop(
            &[
                SemBuf {
                    sem_num: 0,
                    sem_op: -2,
                    sem_flg: 0,
                },
                SemBuf {
                    sem_num: 1,
                    sem_op: 5,
                    sem_flg: SEM_UNDO,
                },
            ],
            101,
            12,
        )
        .unwrap();
        assert_eq!(sem.semctl_getall(), alloc::vec![1, 5]);
        assert_eq!(sem.sem_otime, 12);
        assert_eq!(sem.semctl_setval(0, -1, 0, 0), Err(SemError::InvalidValue));
    }
}
