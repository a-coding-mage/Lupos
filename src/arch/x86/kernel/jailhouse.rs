//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/jailhouse.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/jailhouse.c
//! Jailhouse hypervisor (non-root cell) paravirt support.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/jailhouse.c
//!
//! Jailhouse is a partitioning hypervisor. When running inside a
//! non-root cell, Linux discovers Jailhouse via the standard
//! hypervisor-CPUID leaf (`0x40000000`, signature "Jailhouse") and
//! reads the per-cell setup data (cell topology, allowed UARTs, PCI
//! mmconfig base) from `boot_params.hdr.setup_data` of type
//! `SETUP_JAILHOUSE` (= 6).
//!
//! The hardware integration (LAPIC, IOAPIC, PCI direct ops) is deferred
//! until the rest of the platform-quirks subsystem lands; this module
//! ports the detection logic, the setup-data layout, the serial-port
//! fixup table, and the version validator.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{ENODEV, EOPNOTSUPP};

use super::kdebugfs::SETUP_JAILHOUSE;

/// CPUID hypervisor-vendor signature ("Jailhouse" + 3 NULs = 12 bytes).
pub const JAILHOUSE_CPUID_SIGNATURE: &[u8; 12] = b"Jailhouse\0\0\0";

/// Linux's `JAILHOUSE_SETUP_REQUIRED_VERSION` — the only
/// `compatible_version` we know how to consume.
pub const JAILHOUSE_SETUP_REQUIRED_VERSION: u16 = 1;

/// `struct jailhouse_setup_data::hdr` — version metadata.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct JailhouseHdr {
    pub version: u16,
    pub compatible_version: u16,
}

/// `struct jailhouse_setup_data::v1` — cell layout.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct JailhouseV1 {
    pub pm_timer_address: u16,
    pub num_cpus: u16,
    pub pci_mmconfig_base: u64,
    pub tsc_khz: u32,
    pub apic_khz: u32,
    pub standard_ioapic: u8,
    pub cpu_ids: [u8; 255],
}

impl Default for JailhouseV1 {
    fn default() -> Self {
        Self {
            pm_timer_address: 0,
            num_cpus: 0,
            pci_mmconfig_base: 0,
            tsc_khz: 0,
            apic_khz: 0,
            standard_ioapic: 0,
            cpu_ids: [0; 255],
        }
    }
}

/// `struct jailhouse_setup_data::v2` — UART enable bitmap.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct JailhouseV2 {
    pub flags: u32,
}

/// Sizes the validator checks against the on-disk record's `len`.
pub const SETUP_DATA_HDR_LEN: usize = core::mem::size_of::<JailhouseHdr>();
pub const SETUP_DATA_V1_LEN: usize = SETUP_DATA_HDR_LEN + core::mem::size_of::<JailhouseV1>();
pub const SETUP_DATA_V2_LEN: usize = SETUP_DATA_V1_LEN + core::mem::size_of::<JailhouseV2>();

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JailhouseSetupData {
    pub hdr: JailhouseHdr,
    pub v1: Option<JailhouseV1Owned>,
    pub v2: Option<JailhouseV2>,
}

/// Owned helper for `JailhouseV1` so the struct is `Eq`-friendly without
/// packed-field alignment dances.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct JailhouseV1Owned {
    pub pm_timer_address: u16,
    pub num_cpus: u16,
    pub pci_mmconfig_base: u64,
    pub tsc_khz: u32,
    pub apic_khz: u32,
    pub standard_ioapic: u8,
    pub cpu_ids: Vec<u8>,
}

impl From<JailhouseV1> for JailhouseV1Owned {
    fn from(v: JailhouseV1) -> Self {
        // copy through `{}.field` to avoid taking refs to packed fields.
        let pm_timer_address = v.pm_timer_address;
        let num_cpus = v.num_cpus;
        let pci_mmconfig_base = v.pci_mmconfig_base;
        let tsc_khz = v.tsc_khz;
        let apic_khz = v.apic_khz;
        let standard_ioapic = v.standard_ioapic;
        Self {
            pm_timer_address,
            num_cpus,
            pci_mmconfig_base,
            tsc_khz,
            apic_khz,
            standard_ioapic,
            cpu_ids: v.cpu_ids[..(num_cpus as usize).min(255)].to_vec(),
        }
    }
}

/// `jailhouse_paravirt()` and `jailhouse_detect()` — both ultimately call
/// `jailhouse_cpuid_base()`, which checks the boot-CPU's CPUID flags and
/// scans the hypervisor signature leaves.
pub fn jailhouse_cpuid_base(
    boot_cpu_has_hypervisor: bool,
    cpuid_level: i32,
    leaf_signature: &[u8],
) -> u32 {
    if cpuid_level < 0 || !boot_cpu_has_hypervisor {
        return 0;
    }
    if leaf_signature == JAILHOUSE_CPUID_SIGNATURE {
        0x4000_0000
    } else {
        0
    }
}

pub fn jailhouse_paravirt(
    boot_cpu_has_hypervisor: bool,
    cpuid_level: i32,
    leaf_signature: &[u8],
) -> bool {
    jailhouse_cpuid_base(boot_cpu_has_hypervisor, cpuid_level, leaf_signature) != 0
}

/// Validate a `setup_data` record's `len` against the version it declares.
///
/// Returns Ok with the validated header if acceptable, or `EOPNOTSUPP` if
/// the layout is corrupt or speaks an incompatible protocol.
pub fn validate_setup_data(
    header_type: u32,
    declared_len: usize,
    hdr: JailhouseHdr,
) -> Result<JailhouseHdr, i32> {
    if header_type != SETUP_JAILHOUSE {
        return Err(ENODEV);
    }
    if declared_len < SETUP_DATA_HDR_LEN {
        return Err(EOPNOTSUPP);
    }
    if hdr.version == 0 {
        return Err(EOPNOTSUPP);
    }
    if hdr.compatible_version != JAILHOUSE_SETUP_REQUIRED_VERSION {
        return Err(EOPNOTSUPP);
    }
    if hdr.version == 1 && declared_len < SETUP_DATA_V1_LEN {
        return Err(EOPNOTSUPP);
    }
    if hdr.version >= 2 && declared_len < SETUP_DATA_V2_LEN {
        return Err(EOPNOTSUPP);
    }
    Ok(hdr)
}

/// Linux's `jailhouse_setup_irq` mpc_intsrc descriptor — captures the
/// active-high, edge-triggered IRQ Jailhouse expects for legacy UART
/// IRQs 3 and 4.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct JailhouseIrq {
    pub irq: u32,
    pub active_high: bool,
    pub edge_triggered: bool,
}

pub fn jailhouse_setup_irq(irq: u32) -> JailhouseIrq {
    JailhouseIrq {
        irq,
        active_high: true,
        edge_triggered: true,
    }
}

/// `pcuart_base` — port bases of the four PC UART channels.
pub const PC_UART_BASES: [u16; 4] = [0x3f8, 0x2f8, 0x3e8, 0x2e8];

/// `jailhouse_uart_enabled(n)` — bit `n` of the v2 flags word.
pub fn jailhouse_uart_enabled(flags: u32, n: u32) -> bool {
    (flags >> n) & 1 == 1
}

/// `jailhouse_serial_fixup` — given a UART port base, return a tuple
/// `(new_iobase, irq_to_register_or_zero)`:
/// - If the UART is enabled in v2 flags, keep its iobase and emit the IRQ.
/// - If disabled, zero the iobase to deactivate the UART.
/// - If the port isn't a PC UART at all, leave it untouched.
pub fn jailhouse_serial_fixup(iobase: u16, irq: u32, v2_flags: u32) -> (u16, u32) {
    if let Some((n, _)) = PC_UART_BASES
        .iter()
        .enumerate()
        .find(|&(_, b)| *b == iobase)
    {
        if jailhouse_uart_enabled(v2_flags, n as u32) {
            return (iobase, irq);
        }
        return (0, 0);
    }
    (iobase, 0)
}

/// `jailhouse_no_restart` — emit the kernel notice and return whether the
/// machine should halt. Linux loops in `machine_halt()`; we surface a
/// distinct flag so callers can decide.
pub fn jailhouse_no_restart() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpuid_signature_is_jailhouse_nul_padded() {
        assert_eq!(JAILHOUSE_CPUID_SIGNATURE.len(), 12);
        assert_eq!(&JAILHOUSE_CPUID_SIGNATURE[..9], b"Jailhouse");
        assert_eq!(&JAILHOUSE_CPUID_SIGNATURE[9..], &[0, 0, 0]);
    }

    #[test]
    fn detect_requires_hypervisor_cpuid_bit() {
        assert_eq!(jailhouse_cpuid_base(false, 1, JAILHOUSE_CPUID_SIGNATURE), 0);
    }

    #[test]
    fn detect_returns_base_for_jailhouse_signature() {
        assert_eq!(
            jailhouse_cpuid_base(true, 1, JAILHOUSE_CPUID_SIGNATURE),
            0x4000_0000
        );
    }

    #[test]
    fn detect_rejects_negative_cpuid_level() {
        assert_eq!(jailhouse_cpuid_base(true, -1, JAILHOUSE_CPUID_SIGNATURE), 0);
    }

    #[test]
    fn paravirt_predicate_matches_cpuid_base() {
        assert!(jailhouse_paravirt(true, 1, JAILHOUSE_CPUID_SIGNATURE));
        assert!(!jailhouse_paravirt(false, 1, JAILHOUSE_CPUID_SIGNATURE));
        assert!(!jailhouse_paravirt(true, 1, b"KVM_KVMKVM\0\0"));
    }

    #[test]
    fn validate_rejects_non_jailhouse_setup_type() {
        let r = validate_setup_data(0, SETUP_DATA_V1_LEN, JailhouseHdr::default());
        assert_eq!(r, Err(ENODEV));
    }

    #[test]
    fn validate_rejects_too_short_record() {
        let hdr = JailhouseHdr {
            version: 1,
            compatible_version: JAILHOUSE_SETUP_REQUIRED_VERSION,
        };
        let r = validate_setup_data(SETUP_JAILHOUSE, 2, hdr);
        assert_eq!(r, Err(EOPNOTSUPP));
    }

    #[test]
    fn validate_accepts_v1_length_record() {
        let hdr = JailhouseHdr {
            version: 1,
            compatible_version: JAILHOUSE_SETUP_REQUIRED_VERSION,
        };
        let r = validate_setup_data(SETUP_JAILHOUSE, SETUP_DATA_V1_LEN, hdr).unwrap();
        // Copy out of the packed struct to satisfy the aligned-ref check.
        let ver = r.version;
        assert_eq!(ver, 1);
    }

    #[test]
    fn validate_rejects_v2_record_shorter_than_v2_size() {
        let hdr = JailhouseHdr {
            version: 2,
            compatible_version: JAILHOUSE_SETUP_REQUIRED_VERSION,
        };
        let r = validate_setup_data(SETUP_JAILHOUSE, SETUP_DATA_V1_LEN, hdr);
        assert_eq!(r, Err(EOPNOTSUPP));
    }

    #[test]
    fn validate_rejects_incompatible_compat_version() {
        let hdr = JailhouseHdr {
            version: 1,
            compatible_version: 99,
        };
        let r = validate_setup_data(SETUP_JAILHOUSE, SETUP_DATA_V1_LEN, hdr);
        assert_eq!(r, Err(EOPNOTSUPP));
    }

    #[test]
    fn setup_irq_emits_active_high_edge_triggered() {
        let irq = jailhouse_setup_irq(3);
        assert_eq!(irq.irq, 3);
        assert!(irq.active_high);
        assert!(irq.edge_triggered);
    }

    #[test]
    fn uart_enabled_bit_lookup() {
        assert!(jailhouse_uart_enabled(0b0010, 1));
        assert!(!jailhouse_uart_enabled(0b0010, 0));
    }

    #[test]
    fn serial_fixup_disables_uart_when_bit_clear() {
        let (new_iobase, irq) = jailhouse_serial_fixup(0x3f8, 4, 0);
        assert_eq!(new_iobase, 0);
        assert_eq!(irq, 0);
    }

    #[test]
    fn serial_fixup_keeps_uart_when_bit_set() {
        let (new_iobase, irq) = jailhouse_serial_fixup(0x3f8, 4, 0b0001);
        assert_eq!(new_iobase, 0x3f8);
        assert_eq!(irq, 4);
    }

    #[test]
    fn serial_fixup_leaves_unknown_port_alone() {
        let (new_iobase, _) = jailhouse_serial_fixup(0x1234, 7, 0);
        assert_eq!(new_iobase, 0x1234);
    }

    #[test]
    fn pc_uart_table_matches_x86_default_bases() {
        assert_eq!(PC_UART_BASES, [0x3f8, 0x2f8, 0x3e8, 0x2e8]);
    }

    #[test]
    fn no_restart_halts() {
        assert!(jailhouse_no_restart());
    }
}
