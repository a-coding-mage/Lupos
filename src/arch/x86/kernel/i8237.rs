//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/i8237.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/i8237.c
//! 8237A DMA controller suspend/resume support.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/i8237.c
//!
//! The 8237 is the legacy ISA DMA controller. Modern systems (Skylake-PCH
//! and later) drop it entirely, so this module's resume hook is a no-op on
//! anything more recent. Port presence is probed via `dma_inb(DMA_PAGE_0)`
//! returning 0xff.

#![allow(dead_code)]

extern crate alloc;

use crate::include::uapi::errno::ENODEV;

// === I/O port constants — mirror vendor/linux/arch/x86/include/asm/dma.h ===

pub const DMA1_CMD_REG: u16 = 0x08;
pub const DMA1_STAT_REG: u16 = 0x08;
pub const DMA1_REQ_REG: u16 = 0x09;
pub const DMA1_MASK_REG: u16 = 0x0A;
pub const DMA1_MODE_REG: u16 = 0x0B;
pub const DMA1_CLEAR_FF_REG: u16 = 0x0C;
pub const DMA1_RESET_REG: u16 = 0x0D;
pub const DMA1_CLR_MASK_REG: u16 = 0x0E;
pub const DMA1_MASK_ALL_REG: u16 = 0x0F;

pub const DMA2_CMD_REG: u16 = 0xD0;
pub const DMA2_STAT_REG: u16 = 0xD0;
pub const DMA2_REQ_REG: u16 = 0xD2;
pub const DMA2_MASK_REG: u16 = 0xD4;
pub const DMA2_MODE_REG: u16 = 0xD6;
pub const DMA2_CLEAR_FF_REG: u16 = 0xD8;
pub const DMA2_RESET_REG: u16 = 0xDA;
pub const DMA2_CLR_MASK_REG: u16 = 0xDC;
pub const DMA2_MASK_ALL_REG: u16 = 0xDE;

pub const DMA_PAGE_0: u16 = 0x87;
pub const DMA_PAGE_1: u16 = 0x83;
pub const DMA_PAGE_2: u16 = 0x81;
pub const DMA_PAGE_3: u16 = 0x82;
pub const DMA_PAGE_5: u16 = 0x8B;
pub const DMA_PAGE_6: u16 = 0x89;
pub const DMA_PAGE_7: u16 = 0x8A;

pub const DMA_MODE_READ: u8 = 0x44;
pub const DMA_MODE_WRITE: u8 = 0x48;
pub const DMA_MODE_CASCADE: u8 = 0xC0;
pub const DMA_AUTOINIT: u8 = 0x10;

/// Number of physical DMA channels (0-3 on DMA1, 4-7 on DMA2).
pub const DMA_CHANNELS: usize = 8;

/// Cascade channel — used to wire DMA2's master output into DMA1.
/// Linux's `enable_dma(4)` enables channel 4 (the cascade slot on DMA2).
pub const DMA_CASCADE_CHANNEL: u32 = 4;

/// Trait seam for the port-I/O `dma_outb`/`dma_inb` macros, plus the
/// channel-level helpers Linux exposes from `asm/dma.h`.
pub trait DmaController {
    fn outb(&self, port: u16, value: u8);
    fn inb(&self, port: u16) -> u8;
    fn set_dma_addr(&self, channel: u32, addr: u32);
    fn set_dma_count(&self, channel: u32, count: u32);
    fn enable_dma(&self, channel: u32);
}

/// Trait seam for `claim_dma_lock`/`release_dma_lock`.
pub trait DmaLock {
    type Guard;
    fn claim(&self) -> Self::Guard;
}

/// Linux's `i8237A_resume` — clear both DMA controllers, zero every
/// channel's address and load-count register, re-enable the cascade.
pub fn i8237a_resume<L: DmaLock, D: DmaController>(lock: &L, dma: &D) {
    let _guard = lock.claim();

    dma.outb(DMA1_RESET_REG, 0);
    dma.outb(DMA2_RESET_REG, 0);

    for i in 0..(DMA_CHANNELS as u32) {
        dma.set_dma_addr(i, 0);
        // Count is intentionally non-zero (Linux comment: "DMA count is a
        // bit weird so this is not 0"); programming 0 would mean 65536.
        dma.set_dma_count(i, 1);
    }

    dma.enable_dma(DMA_CASCADE_CHANNEL);
}

/// Linux's `i8237A_init_ops` probe gate. Returns `Err(ENODEV)` if the
/// controller is absent (reads back 0xff on `DMA_PAGE_0`) or if the
/// platform is a 2017-or-later PnPBIOS-disabled SoC.
pub fn i8237a_init_ops<D: DmaController>(
    dma: &D,
    pnpbios_disabled: bool,
    dmi_bios_year: u32,
) -> Result<(), i32> {
    if dma.inb(DMA_PAGE_0) == 0xFF {
        return Err(ENODEV);
    }
    if pnpbios_disabled && dmi_bios_year >= 2017 {
        return Err(ENODEV);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    #[derive(Default)]
    struct MockDma {
        ports: RefCell<alloc::collections::BTreeMap<u16, u8>>,
        addrs: RefCell<[u32; DMA_CHANNELS]>,
        counts: RefCell<[u32; DMA_CHANNELS]>,
        enabled: RefCell<[bool; DMA_CHANNELS]>,
    }

    impl DmaController for MockDma {
        fn outb(&self, port: u16, value: u8) {
            self.ports.borrow_mut().insert(port, value);
        }
        fn inb(&self, port: u16) -> u8 {
            *self.ports.borrow().get(&port).unwrap_or(&0)
        }
        fn set_dma_addr(&self, channel: u32, addr: u32) {
            self.addrs.borrow_mut()[channel as usize] = addr;
        }
        fn set_dma_count(&self, channel: u32, count: u32) {
            self.counts.borrow_mut()[channel as usize] = count;
        }
        fn enable_dma(&self, channel: u32) {
            self.enabled.borrow_mut()[channel as usize] = true;
        }
    }

    struct NoLock;
    impl DmaLock for NoLock {
        type Guard = ();
        fn claim(&self) -> () {}
    }

    #[test]
    fn resume_clears_both_controllers_and_enables_cascade() {
        let dma = MockDma::default();
        i8237a_resume(&NoLock, &dma);
        assert_eq!(*dma.ports.borrow().get(&DMA1_RESET_REG).unwrap(), 0);
        assert_eq!(*dma.ports.borrow().get(&DMA2_RESET_REG).unwrap(), 0);
        assert!(dma.enabled.borrow()[DMA_CASCADE_CHANNEL as usize]);
    }

    #[test]
    fn resume_zeros_addr_and_loads_count_one_on_every_channel() {
        let dma = MockDma::default();
        i8237a_resume(&NoLock, &dma);
        for ch in 0..DMA_CHANNELS {
            assert_eq!(dma.addrs.borrow()[ch], 0);
            assert_eq!(dma.counts.borrow()[ch], 1);
        }
    }

    #[test]
    fn init_rejects_when_page0_reads_0xff() {
        let dma = MockDma::default();
        dma.ports.borrow_mut().insert(DMA_PAGE_0, 0xFF);
        assert_eq!(i8237a_init_ops(&dma, false, 2010), Err(ENODEV));
    }

    #[test]
    fn init_rejects_post_2017_pnp_disabled() {
        let dma = MockDma::default();
        dma.ports.borrow_mut().insert(DMA_PAGE_0, 0x00);
        assert_eq!(i8237a_init_ops(&dma, true, 2017), Err(ENODEV));
        assert_eq!(i8237a_init_ops(&dma, true, 2024), Err(ENODEV));
    }

    #[test]
    fn init_accepts_modern_legacy_capable_platform() {
        let dma = MockDma::default();
        dma.ports.borrow_mut().insert(DMA_PAGE_0, 0x00);
        assert!(i8237a_init_ops(&dma, false, 2015).is_ok());
    }

    #[test]
    fn register_constants_match_linux_dma_h() {
        assert_eq!(DMA1_RESET_REG, 0x0D);
        assert_eq!(DMA2_RESET_REG, 0xDA);
        assert_eq!(DMA_PAGE_0, 0x87);
        assert_eq!(DMA_PAGE_2, 0x81);
    }
}
