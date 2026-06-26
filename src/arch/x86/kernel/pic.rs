//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/i8259.c
//! linux-source: vendor/linux/arch/x86/include/asm/i8259.h
//! test-origin: linux:vendor/linux/arch/x86/kernel/i8259.c
//! Intel 8259A Programmable Interrupt Controller support.
//!
//! This mirrors the Linux legacy PIC hardware contract in
//! `arch/x86/kernel/i8259.c`: cached IRQ masks, Linux's master-then-slave
//! init sequence, mask/unmask, specific mask-and-ack EOI ordering, IRR/ISR
//! reads, ELCR save/restore masks, shutdown masking, and IMCR disconnect for
//! APIC mode. Lupos does not expose Linux's `struct irq_chip` object model, so
//! this file keeps the same hardware operations as direct helpers.

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, Ordering};

use crate::arch::x86::include::asm::io::{inb, outb};

pub const PIC_MASTER_CMD: u16 = 0x20;
pub const PIC_MASTER_IMR: u16 = 0x21;
pub const PIC_MASTER_ISR: u16 = PIC_MASTER_CMD;
pub const PIC_MASTER_OCW3: u16 = PIC_MASTER_ISR;
pub const PIC_SLAVE_CMD: u16 = 0xA0;
pub const PIC_SLAVE_IMR: u16 = 0xA1;
pub const PIC_ELCR1: u16 = 0x4D0;
pub const PIC_ELCR2: u16 = 0x4D1;

pub const PIC_CASCADE_IR: u8 = 2;
pub const MASTER_ICW4_DEFAULT: u8 = 0x01;
pub const SLAVE_ICW4_DEFAULT: u8 = 0x01;
pub const PIC_ICW4_AEOI: u8 = 0x02;

const ICW1_INIT_WITH_ICW4: u8 = 0x11;
const OCW3_READ_IRR: u8 = 0x0A;
const OCW3_READ_ISR: u8 = 0x0B;
const SPECIFIC_EOI: u8 = 0x60;
const MASK_ALL: u8 = 0xFF;
const ELCR_MASTER_VALID_MASK: u8 = 0xF8;
const ELCR_SLAVE_VALID_MASK: u8 = 0xDE;

const IMCR_SELECT_PORT: u16 = 0x22;
const IMCR_DATA_PORT: u16 = 0x23;
const IMCR_REGISTER: u8 = 0x70;
const IMCR_APIC_MODE: u8 = 0x01;

/// First vector delivered by the master PIC after remapping.
pub const PIC1_VECTOR_BASE: u8 = 0x20;

/// First vector delivered by the slave PIC after remapping.
pub const PIC2_VECTOR_BASE: u8 = 0x28;

static CACHED_IRQ_MASK: AtomicU16 = AtomicU16::new(0xFFFF);
static I8259A_AUTO_EOI: AtomicBool = AtomicBool::new(false);
static IRQ_TRIGGER: [AtomicU8; 2] = [const { AtomicU8::new(0) }; 2];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PicWrite {
    port: u16,
    value: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElcrTrigger {
    pub master: u8,
    pub slave: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PicRegister {
    Irr,
    Isr,
}

const fn cached_master_mask(mask: u16) -> u8 {
    mask as u8
}

const fn cached_slave_mask(mask: u16) -> u8 {
    (mask >> 8) as u8
}

const fn valid_irq(irq: u8) -> bool {
    irq < 16
}

const fn irq_bit(irq: u8) -> Option<u16> {
    if valid_irq(irq) {
        Some(1u16 << irq)
    } else {
        None
    }
}

const fn mask_after_irq(mask: u16, irq: u8) -> u16 {
    match irq_bit(irq) {
        Some(bit) => mask | bit,
        None => mask,
    }
}

const fn unmask_after_irq(mask: u16, irq: u8) -> u16 {
    match irq_bit(irq) {
        Some(bit) if irq & 8 != 0 => mask & !bit & !(1u16 << PIC_CASCADE_IR),
        Some(bit) => mask & !bit,
        None => mask,
    }
}

const fn mask_write_for_irq(irq: u8, mask: u16) -> Option<PicWrite> {
    if !valid_irq(irq) {
        return None;
    }
    if irq & 8 != 0 {
        Some(PicWrite {
            port: PIC_SLAVE_IMR,
            value: cached_slave_mask(mask),
        })
    } else {
        Some(PicWrite {
            port: PIC_MASTER_IMR,
            value: cached_master_mask(mask),
        })
    }
}

const fn specific_eoi_plan(irq: u8) -> [Option<PicWrite>; 2] {
    if irq & 8 != 0 {
        [
            Some(PicWrite {
                port: PIC_SLAVE_CMD,
                value: SPECIFIC_EOI + (irq & 7),
            }),
            Some(PicWrite {
                port: PIC_MASTER_CMD,
                value: SPECIFIC_EOI + PIC_CASCADE_IR,
            }),
        ]
    } else {
        [
            Some(PicWrite {
                port: PIC_MASTER_CMD,
                value: SPECIFIC_EOI + irq,
            }),
            None,
        ]
    }
}

const fn init_8259a_plan(auto_eoi: bool, cached_mask: u16) -> [PicWrite; 11] {
    [
        PicWrite {
            port: PIC_MASTER_IMR,
            value: MASK_ALL,
        },
        PicWrite {
            port: PIC_MASTER_CMD,
            value: ICW1_INIT_WITH_ICW4,
        },
        PicWrite {
            port: PIC_MASTER_IMR,
            value: PIC1_VECTOR_BASE,
        },
        PicWrite {
            port: PIC_MASTER_IMR,
            value: 1u8 << PIC_CASCADE_IR,
        },
        PicWrite {
            port: PIC_MASTER_IMR,
            value: if auto_eoi {
                MASTER_ICW4_DEFAULT | PIC_ICW4_AEOI
            } else {
                MASTER_ICW4_DEFAULT
            },
        },
        PicWrite {
            port: PIC_SLAVE_CMD,
            value: ICW1_INIT_WITH_ICW4,
        },
        PicWrite {
            port: PIC_SLAVE_IMR,
            value: PIC2_VECTOR_BASE,
        },
        PicWrite {
            port: PIC_SLAVE_IMR,
            value: PIC_CASCADE_IR,
        },
        PicWrite {
            port: PIC_SLAVE_IMR,
            value: SLAVE_ICW4_DEFAULT,
        },
        PicWrite {
            port: PIC_MASTER_IMR,
            value: cached_master_mask(cached_mask),
        },
        PicWrite {
            port: PIC_SLAVE_IMR,
            value: cached_slave_mask(cached_mask),
        },
    ]
}

const fn saved_elcr(master: u8, slave: u8) -> ElcrTrigger {
    ElcrTrigger {
        master: master & ELCR_MASTER_VALID_MASK,
        slave: slave & ELCR_SLAVE_VALID_MASK,
    }
}

#[inline]
pub fn cached_irq_mask() -> u16 {
    CACHED_IRQ_MASK.load(Ordering::Acquire)
}

#[inline]
unsafe fn outb_pic(port: u16, value: u8) {
    unsafe {
        outb(port, value);
        io_wait();
    }
}

#[inline]
unsafe fn write_pic(write: PicWrite) {
    unsafe {
        outb(write.port, write.value);
    }
}

#[inline]
unsafe fn write_pic_delayed(write: PicWrite) {
    unsafe {
        outb_pic(write.port, write.value);
    }
}

unsafe fn delay_after_init() {
    for _ in 0..100 {
        unsafe {
            io_wait();
        }
    }
}

/// Initialize the 8259A using Linux's `init_8259A(auto_eoi)` sequence.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn init_with_auto_eoi(auto_eoi: bool) {
    I8259A_AUTO_EOI.store(auto_eoi, Ordering::Release);
    let plan = init_8259a_plan(auto_eoi, cached_irq_mask());

    unsafe {
        write_pic(plan[0]);
        for write in &plan[1..=8] {
            write_pic_delayed(*write);
        }
        delay_after_init();
        write_pic(plan[9]);
        write_pic(plan[10]);
    }
}

/// Remap the 8259 PIC vectors and mask all 16 IRQ lines.
///
/// This is the boot path used by Lupos before enabling the LAPIC.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode during early init.
pub unsafe fn init_and_mask_all() {
    CACHED_IRQ_MASK.store(0xFFFF, Ordering::Release);
    unsafe {
        init_with_auto_eoi(false);
    }
}

/// Mask a single legacy IRQ line and update Linux's cached mask mirror.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn mask_irq(irq: u8) {
    let new_mask = mask_after_irq(cached_irq_mask(), irq);
    CACHED_IRQ_MASK.store(new_mask, Ordering::Release);
    if let Some(write) = mask_write_for_irq(irq, new_mask) {
        unsafe {
            write_pic(write);
        }
    }
}

/// Unmask a single legacy IRQ line and update Linux's cached mask mirror.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn unmask_irq(irq: u8) {
    let new_mask = unmask_after_irq(cached_irq_mask(), irq);
    CACHED_IRQ_MASK.store(new_mask, Ordering::Release);
    if irq & 8 != 0 {
        unsafe {
            write_pic(PicWrite {
                port: PIC_MASTER_IMR,
                value: cached_master_mask(new_mask),
            });
            write_pic(PicWrite {
                port: PIC_SLAVE_IMR,
                value: cached_slave_mask(new_mask),
            });
        }
    } else if let Some(write) = mask_write_for_irq(irq, new_mask) {
        unsafe {
            write_pic(write);
        }
    }
}

/// Hardware-mask both PICs without changing the cached mask.
///
/// Linux uses this for temporary global masking and shutdown paths.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn mask_all() {
    unsafe {
        outb(PIC_MASTER_IMR, MASK_ALL);
        outb(PIC_SLAVE_IMR, MASK_ALL);
    }
}

/// Restore hardware masks from the cached IRQ mask.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn restore_mask() {
    let mask = cached_irq_mask();
    unsafe {
        outb(PIC_MASTER_IMR, cached_master_mask(mask));
        outb(PIC_SLAVE_IMR, cached_slave_mask(mask));
    }
}

/// Put both PICs into Linux's shutdown quiescent state.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn shutdown() {
    unsafe {
        mask_all();
    }
}

/// Mask and acknowledge an 8259 IRQ using Linux's specific EOI ordering.
///
/// For slave IRQs, Linux writes the slave EOI before the master cascade EOI.
///
/// # Safety
/// Performs x86 port I/O and must run in a legacy IRQ handler.
pub unsafe fn mask_and_ack(irq: u8) {
    if !valid_irq(irq) {
        return;
    }

    let new_mask = mask_after_irq(cached_irq_mask(), irq);
    CACHED_IRQ_MASK.store(new_mask, Ordering::Release);

    unsafe {
        if irq & 8 != 0 {
            let _ = inb(PIC_SLAVE_IMR);
            write_pic(PicWrite {
                port: PIC_SLAVE_IMR,
                value: cached_slave_mask(new_mask),
            });
        } else {
            let _ = inb(PIC_MASTER_IMR);
            write_pic(PicWrite {
                port: PIC_MASTER_IMR,
                value: cached_master_mask(new_mask),
            });
        }

        for write in specific_eoi_plan(irq).iter().flatten() {
            write_pic(*write);
        }
    }
}

/// Send a specific EOI to the appropriate PIC(s).
///
/// # Safety
/// Performs x86 port I/O and must run from a legacy IRQ handler.
#[allow(dead_code)]
pub unsafe fn send_eoi(irq: u8) {
    if !valid_irq(irq) {
        return;
    }
    unsafe {
        for write in specific_eoi_plan(irq).iter().flatten() {
            write_pic(*write);
        }
    }
}

/// Read the current Interrupt Request Register or In-Service Register.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn read_register(register: PicRegister) -> u16 {
    let ocw3 = match register {
        PicRegister::Irr => OCW3_READ_IRR,
        PicRegister::Isr => OCW3_READ_ISR,
    };

    unsafe {
        outb(PIC_MASTER_OCW3, ocw3);
        outb(PIC_SLAVE_CMD, ocw3);
        let lo = inb(PIC_MASTER_CMD) as u16;
        let hi = inb(PIC_SLAVE_CMD) as u16;

        if register == PicRegister::Isr {
            outb(PIC_MASTER_OCW3, OCW3_READ_IRR);
            outb(PIC_SLAVE_CMD, OCW3_READ_IRR);
        }

        (hi << 8) | lo
    }
}

/// Read the current Interrupt Request Register.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn read_irr() -> u16 {
    unsafe { read_register(PicRegister::Irr) }
}

/// Read the current In-Service Register.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
#[allow(dead_code)]
pub unsafe fn read_isr() -> u16 {
    unsafe { read_register(PicRegister::Isr) }
}

/// Return whether the PIC reports a pending IRQ in the IRR.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn irq_pending(irq: u8) -> bool {
    match irq_bit(irq) {
        Some(bit) => unsafe { read_irr() & bit != 0 },
        None => false,
    }
}

/// Return whether the PIC reports an IRQ as real/in-service in the ISR.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn irq_real(irq: u8) -> bool {
    match irq_bit(irq) {
        Some(bit) => unsafe { read_isr() & bit != 0 },
        None => false,
    }
}

/// Save Linux's ELCR trigger bits. Reserved IRQs are masked off.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn save_elcr() -> ElcrTrigger {
    let trigger = unsafe { saved_elcr(inb(PIC_ELCR1), inb(PIC_ELCR2)) };
    IRQ_TRIGGER[0].store(trigger.master, Ordering::Release);
    IRQ_TRIGGER[1].store(trigger.slave, Ordering::Release);
    trigger
}

/// Restore previously saved ELCR trigger bits.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn restore_elcr(trigger: ElcrTrigger) {
    unsafe {
        outb(PIC_ELCR1, trigger.master);
        outb(PIC_ELCR2, trigger.slave);
    }
}

/// Linux syscore-style suspend hook for the PIC.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn suspend() {
    unsafe {
        save_elcr();
    }
}

/// Linux syscore-style resume hook for the PIC.
///
/// # Safety
/// Performs x86 port I/O and must run in kernel mode.
pub unsafe fn resume() {
    let trigger = ElcrTrigger {
        master: IRQ_TRIGGER[0].load(Ordering::Acquire),
        slave: IRQ_TRIGGER[1].load(Ordering::Acquire),
    };
    let auto_eoi = I8259A_AUTO_EOI.load(Ordering::Acquire);
    unsafe {
        init_with_auto_eoi(auto_eoi);
        restore_elcr(trigger);
    }
}

/// Disconnect the 8259 PIC from the processor's INTR pin via the IMCR.
///
/// # Safety
/// Performs x86 port I/O and must run after LAPIC initialization.
pub unsafe fn disable_legacy() {
    unsafe {
        outb(IMCR_SELECT_PORT, IMCR_REGISTER);
        io_wait();
        outb(IMCR_DATA_PORT, IMCR_APIC_MODE);
    }
}

/// Short I/O delay needed by old PIC chips between command writes.
#[inline]
unsafe fn io_wait() {
    unsafe {
        outb(0x80, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX_I8259_C: &str = include_str!("../../../../vendor/linux/arch/x86/kernel/i8259.c");
    const LINUX_I8259_H: &str =
        include_str!("../../../../vendor/linux/arch/x86/include/asm/i8259.h");

    #[test]
    fn linux_source_contains_pic_parity_anchors() {
        assert!(LINUX_I8259_C.contains("unsigned int cached_irq_mask = 0xffff"));
        assert!(LINUX_I8259_C.contains("static void init_8259A"));
        assert!(LINUX_I8259_C.contains("mask_and_ack_8259A"));
        assert!(LINUX_I8259_C.contains("save_ELCR"));
        assert!(LINUX_I8259_C.contains("restore_ELCR"));
        assert!(LINUX_I8259_H.contains("#define PIC_MASTER_CMD\t\t0x20"));
        assert!(LINUX_I8259_H.contains("#define PIC_SLAVE_IMR\t\t0xa1"));
    }

    #[test]
    fn linux_constants_match_local_pic_constants() {
        assert_eq!(PIC_MASTER_CMD, 0x20);
        assert_eq!(PIC_MASTER_IMR, 0x21);
        assert_eq!(PIC_SLAVE_CMD, 0xA0);
        assert_eq!(PIC_SLAVE_IMR, 0xA1);
        assert_eq!(PIC_ELCR1, 0x4D0);
        assert_eq!(PIC_ELCR2, 0x4D1);
        assert_eq!(PIC_CASCADE_IR, 2);
        assert_eq!(MASTER_ICW4_DEFAULT, 0x01);
        assert_eq!(SLAVE_ICW4_DEFAULT, 0x01);
        assert_eq!(PIC_ICW4_AEOI, 0x02);
    }

    #[test]
    fn cached_mask_bytes_match_linux_little_endian_macros() {
        assert_eq!(cached_master_mask(0xA55A), 0x5A);
        assert_eq!(cached_slave_mask(0xA55A), 0xA5);
    }

    #[test]
    fn mask_unmask_math_matches_linux_cached_irq_mask_updates() {
        assert_eq!(mask_after_irq(0, 3), 0x0008);
        assert_eq!(mask_after_irq(0, 15), 0x8000);
        assert_eq!(unmask_after_irq(0xFFFF, 0), 0xFFFE);
        assert_eq!(unmask_after_irq(0xFFFF, 15), 0x7FFB);
        assert_eq!(mask_after_irq(0x1234, 16), 0x1234);
        assert_eq!(unmask_after_irq(0x1234, 16), 0x1234);
    }

    #[test]
    fn mask_write_selects_master_or_slave_imr() {
        assert_eq!(
            mask_write_for_irq(1, 0x00FF),
            Some(PicWrite {
                port: PIC_MASTER_IMR,
                value: 0xFF
            })
        );
        assert_eq!(
            mask_write_for_irq(9, 0x0200),
            Some(PicWrite {
                port: PIC_SLAVE_IMR,
                value: 0x02
            })
        );
        assert_eq!(mask_write_for_irq(16, 0), None);
    }

    #[test]
    fn init_plan_matches_linux_8259a_order() {
        let plan = init_8259a_plan(false, 0xFFFF);
        assert_eq!(
            plan,
            [
                PicWrite {
                    port: PIC_MASTER_IMR,
                    value: 0xFF
                },
                PicWrite {
                    port: PIC_MASTER_CMD,
                    value: 0x11
                },
                PicWrite {
                    port: PIC_MASTER_IMR,
                    value: PIC1_VECTOR_BASE
                },
                PicWrite {
                    port: PIC_MASTER_IMR,
                    value: 1u8 << PIC_CASCADE_IR
                },
                PicWrite {
                    port: PIC_MASTER_IMR,
                    value: MASTER_ICW4_DEFAULT
                },
                PicWrite {
                    port: PIC_SLAVE_CMD,
                    value: 0x11
                },
                PicWrite {
                    port: PIC_SLAVE_IMR,
                    value: PIC2_VECTOR_BASE
                },
                PicWrite {
                    port: PIC_SLAVE_IMR,
                    value: PIC_CASCADE_IR
                },
                PicWrite {
                    port: PIC_SLAVE_IMR,
                    value: SLAVE_ICW4_DEFAULT
                },
                PicWrite {
                    port: PIC_MASTER_IMR,
                    value: 0xFF
                },
                PicWrite {
                    port: PIC_SLAVE_IMR,
                    value: 0xFF
                },
            ]
        );
    }

    #[test]
    fn init_plan_sets_master_auto_eoi_only_when_requested() {
        let normal = init_8259a_plan(false, 0xFFFF);
        let auto = init_8259a_plan(true, 0xFFFF);
        assert_eq!(normal[4].value, MASTER_ICW4_DEFAULT);
        assert_eq!(auto[4].value, MASTER_ICW4_DEFAULT | PIC_ICW4_AEOI);
        assert_eq!(auto[8].value, SLAVE_ICW4_DEFAULT);
    }

    #[test]
    fn specific_eoi_order_matches_linux_mask_and_ack() {
        assert_eq!(
            specific_eoi_plan(3),
            [
                Some(PicWrite {
                    port: PIC_MASTER_CMD,
                    value: 0x63
                }),
                None
            ]
        );
        assert_eq!(
            specific_eoi_plan(14),
            [
                Some(PicWrite {
                    port: PIC_SLAVE_CMD,
                    value: 0x66
                }),
                Some(PicWrite {
                    port: PIC_MASTER_CMD,
                    value: 0x62
                })
            ]
        );
    }

    #[test]
    fn elcr_save_masks_reserved_linux_irqs() {
        let trigger = saved_elcr(0xFF, 0xFF);
        assert_eq!(trigger.master, 0xF8);
        assert_eq!(trigger.slave, 0xDE);
    }

    #[test]
    fn imcr_constants_match_apic_disconnect_sequence() {
        assert_eq!(IMCR_SELECT_PORT, 0x22);
        assert_eq!(IMCR_DATA_PORT, 0x23);
        assert_eq!(IMCR_REGISTER, 0x70);
        assert_eq!(IMCR_APIC_MODE, 0x01);
    }
}
