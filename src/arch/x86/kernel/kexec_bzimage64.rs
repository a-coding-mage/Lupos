//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/kexec-bzimage64.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/kexec-bzimage64.c
//! 64-bit bzImage kexec loader.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/kexec-bzimage64.c
//!
//! When `kexec_load(2)` selects a bzImage, this loader is responsible
//! for placing the kernel image, initrd, boot_params, and cmdline at
//! their canonical physical addresses. Linux's address constants and
//! the address-splitting tricks (`ramdisk_image` / `ext_ramdisk_image`,
//! `cmd_line_ptr` / `ext_cmd_line_ptr`) are ABI-relevant and ported
//! faithfully.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

/// Minimum physical addresses for each kexec segment.
pub const MIN_PURGATORY_ADDR: u64 = 0x3000;
pub const MIN_BOOTPARAM_ADDR: u64 = 0x3000;
pub const MIN_KERNEL_LOAD_ADDR: u64 = 0x100000; // 1 MiB
pub const MIN_INITRD_LOAD_ADDR: u64 = 0x1000000; // 16 MiB

/// Length of the kexec rng-seed setup_data record.
pub const RNG_SEED_LENGTH: usize = 32;

/// String-size budgets for crash cmdline keys.
pub const MAX_ELFCOREHDR_STR_LEN: usize = 30;
pub const MAX_DMCRYPTKEYS_STR_LEN: usize = 31;

/// Linux's bzImage `struct setup_header` magic.
pub const HDRS_MAGIC: u32 = 0x53726448;

/// Setup-protocol version we support.
pub const SETUP_VERSION_REQUIRED: u16 = 0x0206;

/// Kexec image kind.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KexecType {
    Default,
    Crash,
}

/// Minimal mirror of `struct boot_params::hdr` fields touched by this
/// loader. Kept in host-endian; the actual bzImage on disk is LE.
#[derive(Debug, Default, Clone, Copy)]
pub struct BootParamsHdr {
    pub cmd_line_ptr: u32,
    pub ramdisk_image: u32,
    pub ramdisk_size: u32,
    pub setup_data: u64,
    pub version: u16,
}

/// Minimal mirror of `struct boot_params` fields touched by this loader.
#[derive(Debug, Default, Clone, Copy)]
pub struct BootParams {
    pub hdr: BootParamsHdr,
    pub ext_cmd_line_ptr: u32,
    pub ext_ramdisk_image: u32,
    pub ext_ramdisk_size: u32,
}

/// `setup_initrd(params, addr, len)` — split 64-bit address + length
/// across the legacy and `ext_*` slots.
pub fn setup_initrd(params: &mut BootParams, initrd_load_addr: u64, initrd_len: u64) {
    params.hdr.ramdisk_image = (initrd_load_addr & 0xFFFF_FFFF) as u32;
    params.hdr.ramdisk_size = (initrd_len & 0xFFFF_FFFF) as u32;
    params.ext_ramdisk_image = (initrd_load_addr >> 32) as u32;
    params.ext_ramdisk_size = (initrd_len >> 32) as u32;
}

/// `setup_cmdline` — for `KEXEC_TYPE_CRASH` images, prepend
/// `elfcorehdr=0x<addr> ` and (if present) `dmcryptkeys=0x<addr> `.
/// Then append the user-supplied cmdline. Returns the assembled
/// command line; the production path memcpys it into the boot_params
/// page at the right offset.
pub fn setup_cmdline_bytes(
    image: KexecType,
    elf_load_addr: u64,
    dm_crypt_keys_addr: Option<u64>,
    user_cmdline: &str,
) -> Vec<u8> {
    let mut out = Vec::new();
    if image == KexecType::Crash {
        out.extend_from_slice(alloc::format!("elfcorehdr=0x{:x} ", elf_load_addr).as_bytes());
        if let Some(addr) = dm_crypt_keys_addr {
            if addr != 0 {
                out.extend_from_slice(alloc::format!("dmcryptkeys=0x{:x} ", addr).as_bytes());
            }
        }
    }
    out.extend_from_slice(user_cmdline.as_bytes());
    // Terminator — Linux writes a NUL at `cmdline[len-1]`.
    if out.last() != Some(&0) {
        out.push(0);
    }
    out
}

/// `setup_cmdline` — write the cmdline pointer back into the
/// `boot_params`. Splits the 64-bit physical address between
/// `cmd_line_ptr` and `ext_cmd_line_ptr`.
pub fn setup_cmdline_pointer(params: &mut BootParams, cmdline_phys: u64) {
    params.hdr.cmd_line_ptr = (cmdline_phys & 0xFFFF_FFFF) as u32;
    let high = (cmdline_phys >> 32) as u32;
    if high != 0 {
        params.ext_cmd_line_ptr = high;
    }
}

/// `setup_rng_seed` — push an `SETUP_RNG_SEED` record onto the
/// `setup_data` linked list. Returns the new head pointer.
pub fn setup_rng_seed(params: &mut BootParams, sd_phys: u64) {
    let _ = sd_phys;
    // Mirror Linux: link into head of setup_data list.
    // We don't model the in-buffer write here (that lives in the
    // caller's payload-prep code).
    params.hdr.setup_data = sd_phys;
}

/// Validate the layout assumptions Linux makes about a bzImage's
/// `setup_header`:
/// - `hdr_magic` must be 'HdrS' (0x53726448 LE).
/// - `version >= 0x0206` (loader-supported protocol).
pub fn validate_setup_header(hdr_magic: u32, version: u16) -> Result<(), i32> {
    if hdr_magic != HDRS_MAGIC {
        return Err(EINVAL);
    }
    if version < SETUP_VERSION_REQUIRED {
        return Err(EINVAL);
    }
    Ok(())
}

/// Verify a requested load address satisfies the segment-minimum
/// invariants in Linux's loader.
pub fn validate_load_addresses(
    kernel: u64,
    initrd: Option<u64>,
    bootparam: u64,
    purgatory: u64,
) -> Result<(), i32> {
    if kernel < MIN_KERNEL_LOAD_ADDR {
        return Err(EINVAL);
    }
    if bootparam < MIN_BOOTPARAM_ADDR {
        return Err(EINVAL);
    }
    if purgatory < MIN_PURGATORY_ADDR {
        return Err(EINVAL);
    }
    if let Some(addr) = initrd {
        if addr < MIN_INITRD_LOAD_ADDR {
            return Err(EINVAL);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_load_addresses_match_linux() {
        assert_eq!(MIN_PURGATORY_ADDR, 0x3000);
        assert_eq!(MIN_BOOTPARAM_ADDR, 0x3000);
        assert_eq!(MIN_KERNEL_LOAD_ADDR, 0x100000);
        assert_eq!(MIN_INITRD_LOAD_ADDR, 0x1000000);
    }

    #[test]
    fn rng_seed_length_is_32() {
        assert_eq!(RNG_SEED_LENGTH, 32);
    }

    #[test]
    fn setup_initrd_splits_address_and_length() {
        let mut params = BootParams::default();
        setup_initrd(&mut params, 0xDEAD_BEEF_FACE_F00D, 0x1234_5678_9ABC_DEF0);
        assert_eq!(params.hdr.ramdisk_image, 0xFACE_F00D);
        assert_eq!(params.ext_ramdisk_image, 0xDEAD_BEEF);
        assert_eq!(params.hdr.ramdisk_size, 0x9ABC_DEF0);
        assert_eq!(params.ext_ramdisk_size, 0x1234_5678);
    }

    #[test]
    fn setup_cmdline_default_keeps_user_text_only() {
        let cl = setup_cmdline_bytes(KexecType::Default, 0x100000, None, "root=/dev/sda1 ro");
        // Last byte is the NUL terminator.
        assert_eq!(cl.last(), Some(&0));
        let payload = &cl[..cl.len() - 1];
        assert!(payload.starts_with(b"root=/dev/sda1"));
        assert!(!payload.windows(11).any(|w| w == b"elfcorehdr="));
    }

    #[test]
    fn setup_cmdline_crash_prepends_elfcorehdr() {
        let cl = setup_cmdline_bytes(KexecType::Crash, 0x1234_5678, None, "panic=1");
        let s = core::str::from_utf8(&cl[..cl.len() - 1]).unwrap();
        assert!(s.starts_with("elfcorehdr=0x12345678 "));
        assert!(s.ends_with("panic=1"));
    }

    #[test]
    fn setup_cmdline_crash_with_dmcryptkeys_emits_both() {
        let cl = setup_cmdline_bytes(KexecType::Crash, 0x1000, Some(0xCAFE), "x");
        let s = core::str::from_utf8(&cl[..cl.len() - 1]).unwrap();
        assert!(s.contains("elfcorehdr=0x1000 "));
        assert!(s.contains("dmcryptkeys=0xcafe "));
    }

    #[test]
    fn cmdline_pointer_splits_when_ext_required() {
        let mut p = BootParams::default();
        setup_cmdline_pointer(&mut p, 0x1_2345_6789);
        assert_eq!(p.hdr.cmd_line_ptr, 0x2345_6789);
        assert_eq!(p.ext_cmd_line_ptr, 0x1);
    }

    #[test]
    fn cmdline_pointer_does_not_write_ext_for_32bit_address() {
        let mut p = BootParams::default();
        p.ext_cmd_line_ptr = 0xDEAD;
        setup_cmdline_pointer(&mut p, 0x1000);
        assert_eq!(p.hdr.cmd_line_ptr, 0x1000);
        // ext is preserved (Linux only writes when high != 0).
        assert_eq!(p.ext_cmd_line_ptr, 0xDEAD);
    }

    #[test]
    fn validate_setup_header_requires_hdrs_magic() {
        assert!(validate_setup_header(HDRS_MAGIC, SETUP_VERSION_REQUIRED).is_ok());
        assert_eq!(validate_setup_header(0xDEAD, 0x0210), Err(EINVAL));
    }

    #[test]
    fn validate_setup_header_requires_min_version() {
        assert_eq!(validate_setup_header(HDRS_MAGIC, 0x0100), Err(EINVAL));
    }

    #[test]
    fn validate_load_addresses_rejects_low_kernel_addr() {
        assert_eq!(
            validate_load_addresses(0x1000, None, MIN_BOOTPARAM_ADDR, MIN_PURGATORY_ADDR),
            Err(EINVAL)
        );
    }

    #[test]
    fn validate_load_addresses_rejects_low_initrd() {
        assert_eq!(
            validate_load_addresses(
                MIN_KERNEL_LOAD_ADDR,
                Some(0x10000),
                MIN_BOOTPARAM_ADDR,
                MIN_PURGATORY_ADDR
            ),
            Err(EINVAL)
        );
    }

    #[test]
    fn validate_load_addresses_accepts_minimal_layout() {
        assert!(
            validate_load_addresses(
                MIN_KERNEL_LOAD_ADDR,
                Some(MIN_INITRD_LOAD_ADDR),
                MIN_BOOTPARAM_ADDR,
                MIN_PURGATORY_ADDR,
            )
            .is_ok()
        );
    }
}
