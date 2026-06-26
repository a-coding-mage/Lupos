//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! ACPI MADT multiprocessor wakeup model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/acpi/madt_wakeup.c

use crate::include::uapi::errno::EINVAL;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MadtWakeupCommand {
    Noop = 0,
    Wakeup = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MadtWakeupMailbox {
    pub version: u16,
    pub command: MadtWakeupCommand,
    pub apic_id: u32,
    pub wakeup_vector: u64,
}

pub const fn mailbox_valid(mailbox: MadtWakeupMailbox) -> Result<(), i32> {
    if mailbox.version == 0 || mailbox.wakeup_vector & 0xfff != 0 {
        Err(EINVAL)
    } else {
        Ok(())
    }
}

pub const fn prepare_wakeup(apic_id: u32, wakeup_vector: u64) -> Result<MadtWakeupMailbox, i32> {
    let mailbox = MadtWakeupMailbox {
        version: 1,
        command: MadtWakeupCommand::Wakeup,
        apic_id,
        wakeup_vector,
    };
    match mailbox_valid(mailbox) {
        Ok(()) => Ok(mailbox),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakeup_vector_must_be_page_aligned() {
        assert!(prepare_wakeup(1, 0x8000).is_ok());
        assert_eq!(prepare_wakeup(1, 0x8123), Err(EINVAL));
    }
}
