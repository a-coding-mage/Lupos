//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/um/tls_64.c
//! test-origin: linux:vendor/linux/arch/x86/um/tls_64.c
//! UML x86-64 TLS register bookkeeping.

pub const HOST_FS_BASE: usize = 21;
pub const FS_BASE_SLOT: usize = HOST_FS_BASE;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UmlTask {
    pub thread: UmlThread,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UmlThread {
    pub regs: UmlRegs,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UmlRegs {
    pub gp: [usize; 32],
}

impl Default for UmlTask {
    fn default() -> Self {
        Self {
            thread: UmlThread {
                regs: UmlRegs { gp: [0; 32] },
            },
        }
    }
}

pub fn clear_flushed_tls(_task: &mut UmlTask) {}

pub fn arch_set_tls(task: &mut UmlTask, tls: usize) -> i32 {
    task.thread.regs.gp[FS_BASE_SLOT] = tls;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_set_tls_stores_fs_base_slot() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/um/tls_64.c"
        ));
        assert!(source.contains("void clear_flushed_tls(struct task_struct *task)"));
        assert!(source.contains("t->thread.regs.regs.gp[FS_BASE / sizeof(unsigned long)] = tls;"));

        let mut task = UmlTask::default();
        clear_flushed_tls(&mut task);
        assert_eq!(arch_set_tls(&mut task, 0x1234_5678), 0);
        assert_eq!(task.thread.regs.gp[FS_BASE_SLOT], 0x1234_5678);
    }
}
