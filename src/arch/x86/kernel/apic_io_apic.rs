//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! IO-APIC route-entry model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/apic/io_apic.c

use crate::include::uapi::errno::EINVAL;

#[cfg(not(test))]
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TriggerMode {
    Edge,
    Level,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Polarity {
    High,
    Low,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoApicRouteEntry {
    pub vector: u8,
    pub dest: u8,
    pub trigger: TriggerMode,
    pub polarity: Polarity,
    pub masked: bool,
}

pub const fn route_entry(entry: IoApicRouteEntry) -> Result<u64, i32> {
    if entry.vector < 0x10 {
        return Err(EINVAL);
    }
    let mut raw = entry.vector as u64 | ((entry.dest as u64) << 56);
    if matches!(entry.trigger, TriggerMode::Level) {
        raw |= 1 << 15;
    }
    if matches!(entry.polarity, Polarity::Low) {
        raw |= 1 << 13;
    }
    if entry.masked {
        raw |= 1 << 16;
    }
    Ok(raw)
}

#[cfg(not(test))]
const IO_APIC_DEFAULT_PHYS: u64 = 0xfec0_0000;
#[cfg(not(test))]
const IO_APIC_MMIO_SIZE: u64 = 0x20;
#[cfg(not(test))]
const IO_APIC_REGSEL: usize = 0x00;
#[cfg(not(test))]
const IO_APIC_WINDOW: usize = 0x10;
#[cfg(not(test))]
const IO_APIC_REDTBL_BASE: u32 = 0x10;
#[cfg(not(test))]
const PCI_INTX_GSI_FIRST: u8 = 16;
#[cfg(not(test))]
const PCI_INTX_GSI_LAST: u8 = 23;

#[cfg(not(test))]
static IO_APIC_VIRT: AtomicU64 = AtomicU64::new(0);

#[cfg(not(test))]
unsafe fn io_apic_base() -> Option<*mut u8> {
    let existing = IO_APIC_VIRT.load(Ordering::Acquire);
    if existing != 0 {
        return Some(existing as *mut u8);
    }
    let Ok(mapping) = (unsafe {
        crate::arch::x86::mm::ioremap::ioremap(IO_APIC_DEFAULT_PHYS, IO_APIC_MMIO_SIZE)
    }) else {
        crate::log_warn!(
            "ioapic",
            "failed to map IO-APIC at phys=0x{:x}",
            IO_APIC_DEFAULT_PHYS
        );
        return None;
    };
    IO_APIC_VIRT.store(mapping.virt, Ordering::Release);
    Some(mapping.virt as *mut u8)
}

#[cfg(not(test))]
unsafe fn io_apic_write(reg: u32, value: u32) -> bool {
    let Some(base) = (unsafe { io_apic_base() }) else {
        return false;
    };
    unsafe {
        core::ptr::write_volatile(base.add(IO_APIC_REGSEL).cast::<u32>(), reg);
        core::ptr::write_volatile(base.add(IO_APIC_WINDOW).cast::<u32>(), value);
    }
    true
}

#[cfg(not(test))]
unsafe fn write_redirection_entry(gsi: u8, raw: u64) -> bool {
    let reg = IO_APIC_REDTBL_BASE + (gsi as u32) * 2;
    unsafe { io_apic_write(reg + 1, (raw >> 32) as u32) && io_apic_write(reg, raw as u32) }
}

/// Route the PC-compatible PCI INTx GSIs into the existing legacy IRQ vector.
///
/// QEMU/VirtualBox expose AHCI as PCI INTx while their PCI interrupt line byte
/// still says IRQ 11. The interrupt is delivered by the I/O APIC on one of the
/// PCI INTx GSIs instead of by the 8259 PIC, so route those pins to vector
/// `0x20 + irq` until the full ACPI PCI routing path lands.
#[cfg(not(test))]
pub unsafe fn route_pci_intx_for_legacy_irq(irq: u8) {
    if irq >= 16 {
        return;
    }
    let vector = crate::arch::x86::kernel::idt::LEGACY_IRQ_VECTOR_BASE + irq;
    let dest = unsafe { crate::arch::x86::kernel::apic::id() };
    // In APIC mode the IMCR disconnects the 8259 from the processor. ISA
    // sources (timer/keyboard/cascade/aux) therefore need their own fixed
    // IO-APIC route, just as Linux installs legacy GSI routes before device
    // startup. PC-compatible ISA lines are edge-triggered, active-high.
    if let Ok(isa_raw) = route_entry(IoApicRouteEntry {
        vector,
        dest,
        trigger: TriggerMode::Edge,
        polarity: Polarity::High,
        masked: false,
    }) {
        let _ = unsafe { write_redirection_entry(irq, isa_raw) };
    }

    let Ok(raw) = route_entry(IoApicRouteEntry {
        vector,
        dest,
        trigger: TriggerMode::Level,
        polarity: Polarity::Low,
        masked: false,
    }) else {
        return;
    };

    if irq == 11 {
        for gsi in PCI_INTX_GSI_FIRST..=PCI_INTX_GSI_LAST {
            if unsafe { write_redirection_entry(gsi, raw) } {
                crate::log_info!(
                    "ioapic",
                    "routed PCI INTx GSI {} to irq {} vector=0x{:02x}",
                    gsi,
                    irq,
                    vector
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_apic_route_entry_encodes_level_low_masked() {
        let raw = route_entry(IoApicRouteEntry {
            vector: 0x30,
            dest: 2,
            trigger: TriggerMode::Level,
            polarity: Polarity::Low,
            masked: true,
        })
        .unwrap();
        assert_eq!(raw & 0xff, 0x30);
        assert_ne!(raw & (1 << 15), 0);
        assert_ne!(raw & (1 << 13), 0);
        assert_ne!(raw & (1 << 16), 0);
    }
}
