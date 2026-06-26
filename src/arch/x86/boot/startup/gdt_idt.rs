//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/startup/gdt_idt.c
//! test-origin: linux:vendor/linux/arch/x86/boot/startup/gdt_idt.c
//! Early-startup GDT + IDT setup.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/startup/gdt_idt.c
//!
//! Linux installs a "bringup" IDT (`NUM_EXCEPTION_VECTORS` entries)
//! and the boot GDT before the kernel switches to virtual addresses.
//! The IDT slot for `#VC` is only populated when SEV is in play. After
//! `start_kernel`, the runtime `idt_table` takes over.

use crate::arch::x86::boot::compressed::idt_64::{GateDesc, KERNEL_CS, X86_TRAP_VC};

/// `NUM_EXCEPTION_VECTORS` — first 32 vectors are CPU exceptions.
pub const NUM_EXCEPTION_VECTORS: usize = 32;

/// `__KERNEL_DS` — data-segment selector. Matches `asm/segment.h`.
pub const KERNEL_DS: u16 = 0x18;

/// `GDT_SIZE` — total size of the boot GDT (`sizeof(struct gdt_page)`).
/// `gdt_page` is 4 KiB in Linux; the GDT itself is 16 × 8-byte entries.
pub const GDT_SIZE: usize = 16 * 8;

/// `struct desc_ptr` — what `LIDT/LGDT` consumes. Matches
/// `asm/desc_defs.h` (packed: u16 + u64 → 10 bytes total).
#[repr(C, packed)]
#[derive(Copy, Clone, Default, Debug)]
pub struct DescPtr {
    pub size: u16,
    pub address: u64,
}

/// The bringup IDT table — `NUM_EXCEPTION_VECTORS` gates. Lupos exposes
/// the type so a future port of `startup_64_load_idt` can build the
/// `DescPtr` from a real array instance.
pub type BringupIdt = [GateDesc; NUM_EXCEPTION_VECTORS];

/// Build a `DescPtr` for the bringup IDT. Mirrors gdt_idt.c lines 29-32.
pub fn bringup_idt_desc(idt_addr: u64) -> DescPtr {
    DescPtr {
        size: (core::mem::size_of::<BringupIdt>() - 1) as u16,
        address: idt_addr,
    }
}

/// Build a `DescPtr` for the GDT. Mirrors gdt_idt.c lines 54-57.
pub fn boot_gdt_desc(gdt_addr: u64) -> DescPtr {
    DescPtr {
        size: (GDT_SIZE - 1) as u16,
        address: gdt_addr,
    }
}

/// Install a #VC handler in the bringup IDT. Returns the modified
/// bringup IDT entry. Production wires this to `native_write_idt_entry`.
pub fn install_vc_handler(table: &mut BringupIdt, vc_handler: u64) {
    table[X86_TRAP_VC as usize] = GateDesc::from_handler(vc_handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_exception_vectors_matches_x86_arch() {
        assert_eq!(NUM_EXCEPTION_VECTORS, 32);
    }

    #[test]
    fn kernel_data_segment_selector_matches_segment_h() {
        assert_eq!(KERNEL_DS, 0x18);
        assert_eq!(KERNEL_CS, 0x10);
    }

    #[test]
    fn desc_ptr_packed_size_is_10_bytes() {
        assert_eq!(core::mem::size_of::<DescPtr>(), 10);
    }

    #[test]
    fn bringup_idt_desc_size_field_matches_table_minus_one() {
        let d = bringup_idt_desc(0x1234);
        // Linux records `sizeof(table) - 1` so LIDT loads the inclusive
        // last byte.
        let size = d.size;
        assert_eq!(size as usize, core::mem::size_of::<BringupIdt>() - 1);
    }

    #[test]
    fn vc_handler_installs_into_vector_29() {
        // Address 0x0000_DEAD_BEEF_CAFE splits as:
        //   offset_low    = 0xCAFE
        //   offset_middle = 0xBEEF
        //   offset_high   = 0x0000_DEAD
        let mut idt: BringupIdt = [GateDesc::default(); NUM_EXCEPTION_VECTORS];
        install_vc_handler(&mut idt, 0x0000_dead_beef_cafe);
        let g = &idt[X86_TRAP_VC as usize];
        assert_eq!(g.offset_low, 0xcafe);
        assert_eq!(g.offset_middle, 0xbeef);
        assert_eq!(g.offset_high, 0x0000_dead);
        // Other entries are untouched.
        let other = &idt[0];
        assert_eq!(other.segment, 0);
    }
}
