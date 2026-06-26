//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/memory.c
//! test-origin: linux:vendor/linux/arch/x86/boot/memory.c
//! x86 real-mode memory detection.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/memory.c
//! - vendor/linux/arch/x86/boot/bioscall.S
//! - vendor/linux/arch/x86/include/uapi/asm/bootparam.h
//!
//! The Linux setup stub fills `boot_params` from three INT 15h probes. Lupos
//! keeps the BIOS interaction behind a trait so the exact register protocol can
//! be tested without invoking real-mode firmware from long mode.

use crate::arch::x86::boot::biosregs::{BiosCaller, BiosRegs, X86_EFLAGS_CF};
use crate::arch::x86::include::uapi::asm::bootparam::{BootE820Entry, BootParams, E820_MAX};

pub const SMAP: u32 = 0x534d_4150;
pub const INT15: u8 = 0x15;
pub const E820_FUNCTION: u16 = 0xe820;
pub const E801_FUNCTION: u16 = 0xe801;
pub const EXT_MEM_88_FUNCTION: u8 = 0x88;
pub const BOOT_E820_ENTRY_SIZE: u16 = 20;

/// BIOS seam for `memory.c`. The optional buffer mirrors Linux's `ireg.di`
/// pointer for the E820 scratch entry, while the returned registers mirror
/// `oreg`.
pub trait MemoryBios {
    fn int15(&mut self, ireg: &BiosRegs, e820_buf: Option<&mut BootE820Entry>) -> BiosRegs;
}

pub struct BiosCallerMemory<'a, B: BiosCaller> {
    pub caller: &'a B,
}

impl<B: BiosCaller> MemoryBios for BiosCallerMemory<'_, B> {
    fn int15(&mut self, ireg: &BiosRegs, _e820_buf: Option<&mut BootE820Entry>) -> BiosRegs {
        let mut oreg = BiosRegs::default();
        self.caller.intcall(INT15, ireg, Some(&mut oreg));
        oreg
    }
}

pub fn detect_memory<B: MemoryBios>(params: &mut BootParams, bios: &mut B) {
    detect_memory_e820(params, bios);
    detect_memory_e801(params, bios);
    detect_memory_88(params, bios);
}

pub fn detect_memory_e820<B: MemoryBios>(params: &mut BootParams, bios: &mut B) {
    let mut count = 0usize;
    let mut ireg = BiosRegs::default();
    let mut buf = BootE820Entry::default();

    ireg.set_ax(E820_FUNCTION);
    ireg.ecx = BOOT_E820_ENTRY_SIZE as u32;
    ireg.edx = SMAP;

    loop {
        let oreg = bios.int15(&ireg, Some(&mut buf));
        ireg.ebx = oreg.ebx;

        if oreg.eflags & X86_EFLAGS_CF != 0 {
            break;
        }

        if oreg.eax != SMAP {
            count = 0;
            break;
        }

        params.set_e820_entry(count, buf);
        count += 1;

        if ireg.ebx == 0 || count >= E820_MAX {
            break;
        }
    }

    params.set_e820_entries(count as u8);
}

pub fn detect_memory_e801<B: MemoryBios>(params: &mut BootParams, bios: &mut B) {
    let mut ireg = BiosRegs::default();
    ireg.set_ax(E801_FUNCTION);
    let oreg = bios.int15(&ireg, None);

    if oreg.eflags & X86_EFLAGS_CF != 0 {
        return;
    }

    let (ax, bx) = if oreg.cx() != 0 || oreg.dx() != 0 {
        (oreg.cx() as u32, oreg.dx() as u32)
    } else {
        (oreg.ax() as u32, oreg.bx() as u32)
    };

    if ax > 15 * 1024 {
        return;
    }

    let alt_mem_k = if ax == 15 * 1024 { (bx << 6) + ax } else { ax };
    params.set_alt_mem_k(alt_mem_k);
}

pub fn detect_memory_88<B: MemoryBios>(params: &mut BootParams, bios: &mut B) {
    let mut ireg = BiosRegs::default();
    ireg.set_ah(EXT_MEM_88_FUNCTION);
    let oreg = bios.int15(&ireg, None);
    params.set_screen_ext_mem_k(oreg.ax());
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[derive(Clone, Copy, Debug)]
    struct BiosStep {
        out: BiosRegs,
        entry: Option<BootE820Entry>,
    }

    #[derive(Default)]
    struct ScriptBios {
        steps: Vec<BiosStep>,
        calls: Vec<BiosRegs>,
    }

    impl ScriptBios {
        fn new(steps: Vec<BiosStep>) -> Self {
            Self {
                steps,
                calls: Vec::new(),
            }
        }
    }

    impl MemoryBios for ScriptBios {
        fn int15(&mut self, ireg: &BiosRegs, e820_buf: Option<&mut BootE820Entry>) -> BiosRegs {
            self.calls.push(*ireg);
            let step = self.steps.remove(0);
            if let (Some(entry), Some(buf)) = (step.entry, e820_buf) {
                *buf = entry;
            }
            step.out
        }
    }

    fn e820_out(next: u32) -> BiosRegs {
        BiosRegs {
            eax: SMAP,
            ebx: next,
            ..Default::default()
        }
    }

    fn e801_out(ax: u16, bx: u16, cx: u16, dx: u16) -> BiosRegs {
        BiosRegs {
            eax: ax as u32,
            ebx: bx as u32,
            ecx: cx as u32,
            edx: dx as u32,
            ..Default::default()
        }
    }

    #[test]
    fn e820_probe_copies_entries_until_bios_chain_ends() {
        let first = BootE820Entry {
            base_addr: 0,
            length: 0x9f000,
            region_type: 1,
        };
        let second = BootE820Entry {
            base_addr: 0x100000,
            length: 0x7ff0_0000,
            region_type: 1,
        };
        let mut bios = ScriptBios::new(alloc::vec![
            BiosStep {
                out: e820_out(1),
                entry: Some(first),
            },
            BiosStep {
                out: e820_out(0),
                entry: Some(second),
            },
        ]);
        let mut params = BootParams::new();

        detect_memory_e820(&mut params, &mut bios);

        assert_eq!(params.e820_entries(), 2);
        let entries: Vec<_> = params.e820_iter().collect();
        assert_eq!(entries, alloc::vec![first, second]);
        assert_eq!(bios.calls[0].ax(), E820_FUNCTION);
        assert_eq!(bios.calls[0].ecx, BOOT_E820_ENTRY_SIZE as u32);
        assert_eq!(bios.calls[0].edx, SMAP);
        assert_eq!(bios.calls[1].ebx, 1);
    }

    #[test]
    fn e820_bad_signature_discards_partial_map() {
        let entry = BootE820Entry {
            base_addr: 0,
            length: 0x1000,
            region_type: 1,
        };
        let mut bios = ScriptBios::new(alloc::vec![
            BiosStep {
                out: e820_out(1),
                entry: Some(entry),
            },
            BiosStep {
                out: BiosRegs {
                    eax: 0,
                    ..Default::default()
                },
                entry: Some(entry),
            },
        ]);
        let mut params = BootParams::new();

        detect_memory_e820(&mut params, &mut bios);

        assert_eq!(params.e820_entries(), 0);
    }

    #[test]
    fn e820_carry_flag_terminates_without_erasing_previous_entries() {
        let entry = BootE820Entry {
            base_addr: 0,
            length: 0x1000,
            region_type: 1,
        };
        let mut bios = ScriptBios::new(alloc::vec![
            BiosStep {
                out: e820_out(1),
                entry: Some(entry),
            },
            BiosStep {
                out: BiosRegs {
                    eflags: X86_EFLAGS_CF,
                    ..Default::default()
                },
                entry: None,
            },
        ]);
        let mut params = BootParams::new();

        detect_memory_e820(&mut params, &mut bios);

        assert_eq!(params.e820_entries(), 1);
        assert_eq!(params.e820_iter().next(), Some(entry));
    }

    #[test]
    fn e801_uses_cx_dx_override_and_linux_alt_mem_formula() {
        let mut bios = ScriptBios::new(alloc::vec![BiosStep {
            out: e801_out(1, 2, 15 * 1024, 1),
            entry: None,
        }]);
        let mut params = BootParams::new();

        detect_memory_e801(&mut params, &mut bios);

        assert_eq!(params.alt_mem_k(), (1 << 6) + 15 * 1024);
        assert_eq!(bios.calls[0].ax(), E801_FUNCTION);
    }

    #[test]
    fn e801_rejects_bogus_low_memory_size() {
        let mut bios = ScriptBios::new(alloc::vec![BiosStep {
            out: e801_out(15 * 1024 + 1, 0, 0, 0),
            entry: None,
        }]);
        let mut params = BootParams::new();

        detect_memory_e801(&mut params, &mut bios);

        assert_eq!(params.alt_mem_k(), 0);
    }

    #[test]
    fn int15_88_sets_screen_ext_mem_k_from_ax() {
        let mut bios = ScriptBios::new(alloc::vec![BiosStep {
            out: e801_out(0x3c00, 0, 0, 0),
            entry: None,
        }]);
        let mut params = BootParams::new();

        detect_memory_88(&mut params, &mut bios);

        assert_eq!(params.screen_ext_mem_k(), 0x3c00);
        assert_eq!(bios.calls[0].ah(), EXT_MEM_88_FUNCTION);
    }
}
