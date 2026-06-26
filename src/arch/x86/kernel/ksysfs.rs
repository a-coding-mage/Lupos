//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/ksysfs.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/ksysfs.c
//! x86 `/sys/kernel/boot_params` data model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/ksysfs.c

#![allow(dead_code)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::arch::x86::include::uapi::asm::bootparam::BootParams;
use crate::arch::x86::kernel::kdebugfs::{PhysMem, SetupDataNode, collect_setup_data_chain};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootParamsSysfs {
    pub version: String,
    pub data: Vec<u8>,
    pub setup_data: Vec<SetupDataNode>,
}

pub fn boot_params_version_show(bp: &BootParams) -> String {
    format!("0x{:04x}\n", bp.boot_header_version())
}

pub fn boot_params_data_read(bp: &BootParams, pos: usize, out: &mut [u8]) -> usize {
    if pos >= bp.data.len() {
        return 0;
    }
    let n = out.len().min(bp.data.len() - pos);
    out[..n].copy_from_slice(&bp.data[pos..pos + n]);
    n
}

pub fn build_boot_params_sysfs<M: PhysMem>(
    bp: &BootParams,
    mem: &M,
    max_setup_records: usize,
) -> Result<BootParamsSysfs, i32> {
    let setup_data = collect_setup_data_chain(mem, bp.setup_data(), max_setup_records)?;
    Ok(BootParamsSysfs {
        version: boot_params_version_show(bp),
        data: bp.data.to_vec(),
        setup_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use core::cell::RefCell;

    #[derive(Default)]
    struct Mem(RefCell<BTreeMap<u64, u8>>);

    impl PhysMem for Mem {
        fn read(&self, paddr: u64, out: &mut [u8]) -> Result<(), i32> {
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = *self.0.borrow().get(&(paddr + i as u64)).unwrap_or(&0);
            }
            Ok(())
        }
    }

    #[test]
    fn version_is_hex_and_data_read_is_bounded() {
        let mut bp = BootParams::new();
        bp.set_boot_header_version(0x020f);
        assert_eq!(boot_params_version_show(&bp), "0x020f\n");
        bp.data[10] = 0xaa;
        let mut out = [0u8; 1];
        assert_eq!(boot_params_data_read(&bp, 10, &mut out), 1);
        assert_eq!(out[0], 0xaa);
    }

    #[test]
    fn sysfs_model_walks_empty_setup_data() {
        let bp = BootParams::new();
        let model = build_boot_params_sysfs(&bp, &Mem::default(), 8).unwrap();
        assert!(model.setup_data.is_empty());
        assert_eq!(
            model.data.len(),
            crate::arch::x86::include::uapi::asm::bootparam::BOOT_PARAMS_SIZE
        );
    }
}
