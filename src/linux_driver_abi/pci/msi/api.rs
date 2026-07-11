//! linux-parity: partial
//! linux-source: vendor/linux/drivers/pci/msi/api.c
//! test-origin: linux:vendor/linux/drivers/pci/msi/api.c
//! PCI MSI public API coverage for M55.
//!
//! Mirrors `vendor/linux/drivers/pci/msi/api.c`.

use core::ffi::c_void;

use crate::arch::x86::kernel::cpu::common::LinuxCpuMask;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::irq::msi::{msi_alloc_descs, msi_free_descs};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::linux_driver_abi::pci::device::{
    LinuxPciDev, linux_pci_config_read, linux_pci_config_write, linux_pci_device_state,
    linux_pci_slot_for_raw,
};

const ENOSPC: i32 = 28;
const ERANGE: i32 = 34;
// Linux's internal ENOTSUPP is intentionally distinct from userspace
// EOPNOTSUPP.  See vendor/linux/include/linux/errno.h.
const ENOTSUPP: i32 = 524;

const PCI_COMMAND: usize = 0x04;
const PCI_COMMAND_INTX_DISABLE: u16 = 0x0400;

const PCI_IRQ_INTX: u32 = 1 << 0;
const PCI_IRQ_MSI: u32 = 1 << 1;
const PCI_IRQ_MSIX: u32 = 1 << 2;
const PCI_IRQ_AFFINITY: u32 = 1 << 3;

type IrqAffinityCalc = unsafe extern "C" fn(*mut LinuxIrqAffinity, u32);

/// `struct irq_affinity` - `vendor/linux/include/linux/interrupt.h:295`.
#[repr(C)]
pub struct LinuxIrqAffinity {
    pre_vectors: u32,
    post_vectors: u32,
    nr_sets: u32,
    set_size: [u32; 4],
    calc_sets: Option<IrqAffinityCalc>,
    priv_: *mut c_void,
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "pci_alloc_irq_vectors_affinity",
        linux_pci_alloc_irq_vectors_affinity as usize,
        false,
    );
    export_symbol_once(
        "pci_free_irq_vectors",
        linux_pci_free_irq_vectors as usize,
        false,
    );
    export_symbol_once("pci_irq_vector", linux_pci_irq_vector as usize, false);
    export_symbol_once(
        "pci_irq_get_affinity",
        linux_pci_irq_get_affinity as usize,
        false,
    );
}

pub fn pci_alloc_irq_vectors(min_vecs: u32, max_vecs: u32) -> Result<u32, i32> {
    if min_vecs == 0 || max_vecs < min_vecs {
        return Err(EINVAL);
    }
    msi_alloc_descs(max_vecs)
}

pub fn pci_free_irq_vectors(start: u32, count: u32) {
    msi_free_descs(start, count);
}

fn legacy_irq(dev: *const c_void) -> Option<u32> {
    linux_pci_device_state(dev)?;
    // Test-only snapshot registrations are opaque tokens, not complete
    // `struct pci_dev` objects.  Production devices are always present in the
    // raw PCI object registry before a Linux driver can probe them.
    linux_pci_slot_for_raw(dev)?;
    Some(unsafe { (*dev.cast::<LinuxPciDev>()).irq })
}

fn pci_intx_enable(dev: *const c_void) {
    let Some(command) = linux_pci_config_read(dev, PCI_COMMAND, 2) else {
        return;
    };
    let command = (command as u16) & !PCI_COMMAND_INTX_DISABLE;
    let _ = linux_pci_config_write(dev, PCI_COMMAND, 2, command as u32);
}

unsafe extern "C" fn default_calc_sets(affd: *mut LinuxIrqAffinity, affvecs: u32) {
    unsafe {
        (*affd).nr_sets = 1;
        (*affd).set_size[0] = affvecs;
    }
}

unsafe fn create_single_vector_affinity(affd: *mut LinuxIrqAffinity) {
    let affvecs = unsafe {
        if 1 > (*affd).pre_vectors.saturating_add((*affd).post_vectors) {
            1 - ((*affd).pre_vectors + (*affd).post_vectors)
        } else {
            0
        }
    };
    if unsafe { (*affd).calc_sets.is_none() } {
        unsafe {
            (*affd).calc_sets = Some(default_calc_sets);
        }
    }
    if let Some(calc_sets) = unsafe { (*affd).calc_sets } {
        unsafe { calc_sets(affd, affvecs) };
    }
}

/// `pci_alloc_irq_vectors_affinity` -
/// `vendor/linux/drivers/pci/msi/api.c:252`.
///
/// Lupos does not yet have an x86 MSI irqdomain connected to the IDT.  Linux's
/// PCI MSI path returns `-ENOTSUPP` in precisely that platform state.  This is
/// important: reporting successful MSI-X allocation without programming a
/// dispatchable vector would strand interrupts.  Callers such as virtio-pci
/// then follow their vendor INTx fallback path.
#[unsafe(export_name = "pci_alloc_irq_vectors_affinity")]
pub unsafe extern "C" fn linux_pci_alloc_irq_vectors_affinity(
    dev: *mut c_void,
    min_vecs: u32,
    max_vecs: u32,
    flags: u32,
    mut affd: *mut LinuxIrqAffinity,
) -> i32 {
    if linux_pci_device_state(dev.cast_const()).is_none() {
        return -EINVAL;
    }

    if flags & PCI_IRQ_AFFINITY != 0 {
        // A NULL descriptor selects Linux's zero-initialized default affinity
        // descriptor.  It has no externally visible state when MSI setup is
        // unsupported, so no synthetic object is needed here.
    } else if !affd.is_null() {
        // Matches WARN_ON(affd) followed by affd = NULL.
        affd = core::ptr::null_mut();
    }

    let mut nvecs = -ENOSPC;
    if flags & PCI_IRQ_MSIX != 0 {
        nvecs = if max_vecs < min_vecs {
            -ERANGE
        } else {
            -ENOTSUPP
        };
        if nvecs > 0 {
            return nvecs;
        }
    }
    if flags & PCI_IRQ_MSI != 0 {
        nvecs = if max_vecs < min_vecs {
            -ERANGE
        } else {
            -ENOTSUPP
        };
        if nvecs > 0 {
            return nvecs;
        }
    }

    if flags & PCI_IRQ_INTX != 0 && min_vecs == 1 && legacy_irq(dev.cast_const()).unwrap_or(0) != 0
    {
        if !affd.is_null() {
            unsafe { create_single_vector_affinity(affd) };
        }
        pci_intx_enable(dev.cast_const());
        return 1;
    }

    nvecs
}

/// `pci_irq_vector` - `vendor/linux/drivers/pci/msi/api.c:311`.
#[unsafe(export_name = "pci_irq_vector")]
pub unsafe extern "C" fn linux_pci_irq_vector(dev: *mut c_void, nr: u32) -> i32 {
    let Some(irq) = legacy_irq(dev.cast_const()) else {
        return -EINVAL;
    };
    if nr != 0 {
        return -EINVAL;
    }
    i32::try_from(irq).unwrap_or(-EINVAL)
}

/// `pci_irq_get_affinity` - `vendor/linux/drivers/pci/msi/api.c:340`.
#[unsafe(export_name = "pci_irq_get_affinity")]
pub unsafe extern "C" fn linux_pci_irq_get_affinity(
    dev: *mut c_void,
    nr: i32,
) -> *const LinuxCpuMask {
    if nr < 0 || unsafe { linux_pci_irq_vector(dev, nr as u32) } <= 0 {
        return core::ptr::null();
    }

    // A legacy INTx interrupt has no MSI descriptor, so Linux returns the
    // canonical cpu_possible_mask object.
    find_symbol("__cpu_possible_mask")
        .map(|addr| addr as *const LinuxCpuMask)
        .unwrap_or(core::ptr::null())
}

/// `pci_free_irq_vectors` - `vendor/linux/drivers/pci/msi/api.c:379`.
#[unsafe(export_name = "pci_free_irq_vectors")]
pub unsafe extern "C" fn linux_pci_free_irq_vectors(_dev: *mut c_void) {
    // pci_disable_msix() and pci_disable_msi() are both no-ops when neither
    // mode is enabled.  Lupos never reports MSI allocation success above.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_vector_range_returns_einval() {
        assert_eq!(pci_alloc_irq_vectors(2, 1), Err(EINVAL));
    }
}
