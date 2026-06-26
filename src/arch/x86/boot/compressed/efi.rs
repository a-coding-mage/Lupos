//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/efi.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/efi.c
//! Early EFI helpers used by the decompressor.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/efi.c
//!
//! Three things the decompressor needs from EFI:
//!   1. `efi_get_type(boot_params)` — was the kernel loaded by an EFI
//!      bootloader, and if so which width (32/64)?
//!   2. `efi_get_system_table(boot_params)` — physical address of the
//!      EFI system table (high/low halves combined on x86_64).
//!   3. `efi_get_conf_table` / `efi_find_vendor_table` — walk the EFI
//!      configuration tables to find the RSDP or
//!      LINUX_EFI_UNACCEPTED_MEM_TABLE_GUID entry.
//!
//! Lupos reads these fields from Linux `boot_params.efi_info`, the same
//! zeropage contract GRUB fills when it loads the generated bzImage.
//!
//! Lupos adaptations (documented, not stubs):
//! - Linux passes `struct boot_params *bp` everywhere; lupos callers hold a
//!   parsed `EfiInfo` plus the raw `boot_params.hdr.setup_data` pointer, so
//!   the two fields Linux reads off `bp` are explicit parameters here.
//! - C out-params (`*cfg_tbl_pa`, `*cfg_tbl_len`) become `Result<(u64, u32)>`.
//! - `debug_putstr()` diagnostics are comments: Linux compiles them out
//!   unless CONFIG_X86_VERBOSE_BOOTUP, and the return values carry the same
//!   information.

use crate::include::uapi::errno::EINVAL;

/// `EFI64_LOADER_SIGNATURE` — 4-byte "EL64".
pub const EFI64_LOADER_SIGNATURE: &[u8; 4] = b"EL64";
/// `EFI32_LOADER_SIGNATURE` — 4-byte "EL32".
pub const EFI32_LOADER_SIGNATURE: &[u8; 4] = b"EL32";

/// `SETUP_EFI` — `setup_data.type` for the kexec-provided EFI data.
/// Mirrors `arch/x86/include/uapi/asm/setup_data.h`.
pub const SETUP_EFI: u32 = 4;

/// `enum efi_type` — distinguishes 32-bit, 64-bit, and "no EFI".
/// `EfiMixed` is reserved in Linux (32-bit kernel on 64-bit firmware).
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum EfiType {
    None,
    Efi32,
    Efi64,
    EfiMixed,
}

/// Linux `struct efi_info` (subset that compressed/efi.c reads). Same
/// field order as `arch/x86/include/uapi/asm/bootparam.h::efi_info`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct EfiInfo {
    pub efi_loader_signature: [u8; 4],
    pub efi_systab: u32,
    pub efi_memdesc_size: u32,
    pub efi_memdesc_version: u32,
    pub efi_memmap: u32,
    pub efi_memmap_size: u32,
    pub efi_systab_hi: u32,
    pub efi_memmap_hi: u32,
}

/// `efi_table_hdr_t` — common 24-byte header of every EFI table.
/// Mirrors `include/linux/efi.h`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct EfiTableHdr {
    pub signature: u64,
    pub revision: u32,
    pub headersize: u32,
    pub crc32: u32,
    pub reserved: u32,
}

/// `efi_system_table_64_t` — fixed-width layout the decompressor reads
/// straight out of physical memory. Mirrors `include/linux/efi.h:481`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct EfiSystemTable64T {
    pub hdr: EfiTableHdr,
    /// Physical addr of CHAR16 vendor string.
    pub fw_vendor: u64,
    pub fw_revision: u32,
    pub __pad1: u32,
    pub con_in_handle: u64,
    pub con_in: u64,
    pub con_out_handle: u64,
    pub con_out: u64,
    pub stderr_handle: u64,
    pub stderr: u64,
    pub runtime: u64,
    pub boottime: u64,
    pub nr_tables: u32,
    pub __pad2: u32,
    pub tables: u64,
}

/// `efi_system_table_32_t` — mirrors `include/linux/efi.h:499`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct EfiSystemTable32T {
    pub hdr: EfiTableHdr,
    pub fw_vendor: u32,
    pub fw_revision: u32,
    pub con_in_handle: u32,
    pub con_in: u32,
    pub con_out_handle: u32,
    pub con_out: u32,
    pub stderr_handle: u32,
    pub stderr: u32,
    pub runtime: u32,
    pub boottime: u32,
    pub nr_tables: u32,
    pub tables: u32,
}

/// `efi_config_table_64_t` — one {GUID, table PA} entry.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EfiConfigTable64T {
    pub guid: [u8; 16],
    pub table: u64,
}

/// `efi_config_table_32_t`.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct EfiConfigTable32T {
    pub guid: [u8; 16],
    pub table: u32,
}

/// `struct setup_data` fixed header (the flexible `data[]` tail follows
/// in memory). Mirrors `arch/x86/include/uapi/asm/setup_data.h:27`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct SetupData {
    pub next: u64,
    pub type_: u32,
    pub len: u32,
}

/// `struct efi_setup_data` — kexec's preserved EFI pointers. Mirrors
/// `arch/x86/include/asm/setup_data.h:22`.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct EfiSetupData {
    pub fw_vendor: u64,
    pub __unused: u64,
    pub tables: u64,
    pub smbios: u64,
    pub reserved: [u64; 8],
}

/// `efi_get_type(boot_params)` — match the 4-byte signature. Mirrors
/// efi.c lines 19-50. The `!CONFIG_X86_64` systab_hi/memmap_hi guard is
/// compiled out: lupos is x86_64-only.
pub fn efi_get_type(ei: &EfiInfo) -> EfiType {
    let sig = &ei.efi_loader_signature;
    if sig == EFI64_LOADER_SIGNATURE {
        EfiType::Efi64
    } else if sig == EFI32_LOADER_SIGNATURE {
        EfiType::Efi32
    } else {
        // Linux: debug_putstr("No EFI environment detected.\n");
        EfiType::None
    }
}

/// `efi_get_system_table(boot_params)` — combine the high/low halves.
/// 0 means "EFI system table not found", same as Linux. Mirrors efi.c
/// lines 60-79.
pub fn efi_get_system_table(ei: &EfiInfo) -> u64 {
    (ei.efi_systab as u64) | ((ei.efi_systab_hi as u64) << 32)
}

/// Helper for the EFI memmap pointer — same hi/lo pattern.
pub fn efi_get_memmap(ei: &EfiInfo) -> u64 {
    (ei.efi_memmap as u64) | ((ei.efi_memmap_hi as u64) << 32)
}

/// Compare two GUIDs (16 bytes each). Matches `efi_guidcmp()` in
/// `include/linux/efi.h` (note: Linux returns 0 on equality; lupos
/// returns `true` on equality — callers invert accordingly).
#[inline]
pub fn efi_guidcmp(a: &[u8; 16], b: &[u8; 16]) -> bool {
    a == b
}

/// Decompressor-context physical read. Linux's compressed stub runs
/// identity-mapped and dereferences physical addresses directly; in the
/// lupos runtime the linear map provides the same access via
/// `phys_to_virt`. Host tests use identity addresses (same seam as
/// `arch/x86/platform/efi.rs::phys_to_ptr`).
#[cfg(not(test))]
fn phys_to_ptr<T>(phys: u64) -> *const T {
    crate::arch::x86::mm::paging::phys_to_virt(phys) as *const T
}

#[cfg(test)]
fn phys_to_ptr<T>(phys: u64) -> *const T {
    phys as *const T
}

/// Unaligned typed read at a physical address; `None` when the address
/// is 0 or unmapped.
fn read_phys<T: Copy>(phys: u64) -> Option<T> {
    if phys == 0 {
        return None;
    }
    let ptr = phys_to_ptr::<T>(phys);
    if ptr.is_null() {
        return None;
    }
    // SAFETY: callers pass physical addresses originating from the
    // firmware/bootloader-provided boot_params chain, which the linear
    // map (or the decompressor's identity map) covers.
    Some(unsafe { core::ptr::read_unaligned(ptr) })
}

/// `get_kexec_setup_data()` — walk the `boot_params.hdr.setup_data`
/// list for a `SETUP_EFI` entry; its payload is `struct efi_setup_data`
/// preserving the pre-kexec EFI config table address. Mirrors efi.c
/// lines 87-119 (CONFIG_X86_64 arm; the !X86_64 arm returns NULL).
fn get_kexec_setup_data(setup_data_pa: u64) -> Option<EfiSetupData> {
    let mut esd: Option<EfiSetupData> = None;
    let mut pa_data = setup_data_pa;
    while pa_data != 0 {
        let data: SetupData = read_phys(pa_data)?;
        if data.type_ == SETUP_EFI {
            esd = read_phys(pa_data + core::mem::size_of::<SetupData>() as u64);
            break;
        }
        pa_data = data.next;
    }

    // Linux: fall back to normal EFI boot when the kexec data carries
    // no config table ("kexec EFI environment missing valid
    // configuration table.").
    match esd {
        Some(e) if e.tables == 0 => None,
        other => other,
    }
}

/// `efi_get_conf_table(boot_params, *cfg_tbl_pa, *cfg_tbl_len)` —
/// locate the EFI configuration table. Returns `(cfg_tbl_pa,
/// cfg_tbl_len)`; on error the C out-params are left unchanged, which
/// maps to `Err(-EINVAL)`. Mirrors efi.c lines 131-166.
///
/// `setup_data_pa` is `boot_params.hdr.setup_data` (kexec checks it for
/// an alternative conf table).
pub fn efi_get_conf_table(ei: &EfiInfo, setup_data_pa: u64) -> Result<(u64, u32), i32> {
    let sys_tbl_pa = efi_get_system_table(ei);
    if sys_tbl_pa == 0 {
        return Err(-EINVAL);
    }

    // Handle EFI bitness properly.
    match efi_get_type(ei) {
        EfiType::Efi64 => {
            let stbl: EfiSystemTable64T = read_phys(sys_tbl_pa).ok_or(-EINVAL)?;
            // kexec provides an alternative EFI conf table, check for it.
            let esd = get_kexec_setup_data(setup_data_pa);
            let cfg_tbl_pa = esd.map_or(stbl.tables, |e| e.tables);
            Ok((cfg_tbl_pa, stbl.nr_tables))
        }
        EfiType::Efi32 => {
            let stbl: EfiSystemTable32T = read_phys(sys_tbl_pa).ok_or(-EINVAL)?;
            Ok((stbl.tables as u64, stbl.nr_tables))
        }
        _ => Err(-EINVAL),
    }
}

/// `get_vendor_table()` — read entry `idx` of the EFI config table.
/// Mirrors efi.c lines 169-195. The "entry above 4GB" check is for
/// !CONFIG_X86_64 only and is compiled out here.
fn get_vendor_table(cfg_tbl_pa: u64, idx: u32, et: EfiType) -> Result<(u64, [u8; 16]), i32> {
    match et {
        EfiType::Efi64 => {
            let entry_pa =
                cfg_tbl_pa + (idx as u64) * core::mem::size_of::<EfiConfigTable64T>() as u64;
            let entry: EfiConfigTable64T = read_phys(entry_pa).ok_or(-EINVAL)?;
            Ok((entry.table, entry.guid))
        }
        EfiType::Efi32 => {
            let entry_pa =
                cfg_tbl_pa + (idx as u64) * core::mem::size_of::<EfiConfigTable32T>() as u64;
            let entry: EfiConfigTable32T = read_phys(entry_pa).ok_or(-EINVAL)?;
            Ok((entry.table as u64, entry.guid))
        }
        _ => Err(-EINVAL),
    }
}

/// `efi_find_vendor_table(boot_params, cfg_tbl_pa, cfg_tbl_len, guid)`
/// — linear-scan the config table for `guid`. Returns the vendor table
/// physical address, or 0 on any error — exactly Linux's contract.
/// Mirrors efi.c lines 208-236.
pub fn efi_find_vendor_table(
    ei: &EfiInfo,
    cfg_tbl_pa: u64,
    cfg_tbl_len: u32,
    guid: &[u8; 16],
) -> u64 {
    let et = efi_get_type(ei);
    if et == EfiType::None {
        return 0;
    }

    for i in 0..cfg_tbl_len {
        let Ok((vendor_tbl_pa, vendor_tbl_guid)) = get_vendor_table(cfg_tbl_pa, i, et) else {
            return 0;
        };
        if efi_guidcmp(guid, &vendor_tbl_guid) {
            return vendor_tbl_pa;
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ei_with_sig(sig: &[u8; 4]) -> EfiInfo {
        let mut ei = EfiInfo::default();
        ei.efi_loader_signature = *sig;
        ei
    }

    /// Point an `EfiInfo` systab at a host-memory struct (identity
    /// phys_to_ptr under cfg(test)).
    fn ei_with_systab(sig: &[u8; 4], systab_pa: u64) -> EfiInfo {
        let mut ei = ei_with_sig(sig);
        ei.efi_systab = systab_pa as u32;
        ei.efi_systab_hi = (systab_pa >> 32) as u32;
        ei
    }

    fn addr_of<T>(v: &T) -> u64 {
        v as *const T as u64
    }

    #[test]
    fn signature_constants_match_uefi_protocol() {
        assert_eq!(EFI64_LOADER_SIGNATURE, b"EL64");
        assert_eq!(EFI32_LOADER_SIGNATURE, b"EL32");
    }

    #[test]
    fn efi_get_type_dispatches_on_signature() {
        assert_eq!(efi_get_type(&ei_with_sig(b"EL64")), EfiType::Efi64);
        assert_eq!(efi_get_type(&ei_with_sig(b"EL32")), EfiType::Efi32);
        assert_eq!(efi_get_type(&ei_with_sig(b"NONE")), EfiType::None);
        assert_eq!(efi_get_type(&ei_with_sig(&[0u8; 4])), EfiType::None);
    }

    #[test]
    fn efi_get_system_table_packs_hi_into_top_32_bits() {
        let mut ei = EfiInfo::default();
        ei.efi_systab = 0xdead_beef;
        ei.efi_systab_hi = 0x1234_5678;
        assert_eq!(efi_get_system_table(&ei), 0x1234_5678_dead_beef);
    }

    #[test]
    fn efi_info_layout_matches_linux_size() {
        // Linux `efi_info`: 8 × u32 = 32 bytes + 4-byte signature
        // already counted (it's the first field). Total = 32 bytes.
        assert_eq!(core::mem::size_of::<EfiInfo>(), 32);
    }

    #[test]
    fn efi_table_layouts_match_linux_sizes() {
        // efi_table_hdr_t: u64 + 4×u32 = 24.
        assert_eq!(core::mem::size_of::<EfiTableHdr>(), 24);
        // efi_system_table_64_t: 24 hdr + 8 + 4+4 + 6×8 + 8 + 8 + 4+4 + 8 = 120.
        assert_eq!(core::mem::size_of::<EfiSystemTable64T>(), 120);
        // efi_system_table_32_t: 24 hdr + 12×u32 = 72.
        assert_eq!(core::mem::size_of::<EfiSystemTable32T>(), 72);
        // efi_config_table_{64,32}_t: GUID 16 + table.
        assert_eq!(core::mem::size_of::<EfiConfigTable64T>(), 24);
        assert_eq!(core::mem::size_of::<EfiConfigTable32T>(), 20);
        // setup_data fixed header: u64 + 2×u32 = 16.
        assert_eq!(core::mem::size_of::<SetupData>(), 16);
        // efi_setup_data: 12 × u64 = 96.
        assert_eq!(core::mem::size_of::<EfiSetupData>(), 96);
        // The Linux `tables`/`nr_tables` offsets the decompressor reads.
        assert_eq!(core::mem::offset_of!(EfiSystemTable64T, nr_tables), 104);
        assert_eq!(core::mem::offset_of!(EfiSystemTable64T, tables), 112);
        assert_eq!(core::mem::offset_of!(EfiSystemTable32T, tables), 68);
    }

    #[test]
    fn guidcmp_matches_byte_equality() {
        let a = [0u8; 16];
        let mut b = [0u8; 16];
        assert!(efi_guidcmp(&a, &b));
        b[0] = 1;
        assert!(!efi_guidcmp(&a, &b));
    }

    #[test]
    fn conf_table_read_from_efi64_system_table() {
        let stbl = EfiSystemTable64T {
            nr_tables: 9,
            tables: 0xabc0,
            ..Default::default()
        };
        let ei = ei_with_systab(b"EL64", addr_of(&stbl));
        assert_eq!(efi_get_conf_table(&ei, 0), Ok((0xabc0, 9)));
    }

    #[test]
    fn conf_table_read_from_efi32_system_table() {
        let stbl = EfiSystemTable32T {
            nr_tables: 4,
            tables: 0x9000,
            ..Default::default()
        };
        let ei = ei_with_systab(b"EL32", addr_of(&stbl));
        assert_eq!(efi_get_conf_table(&ei, 0), Ok((0x9000, 4)));
    }

    #[test]
    fn conf_table_requires_systab_and_efi_environment() {
        // No system table → -EINVAL (Linux: "EFI system table not found").
        assert_eq!(efi_get_conf_table(&ei_with_sig(b"EL64"), 0), Err(-EINVAL));
        // Valid pointer but no EFI signature → -EINVAL.
        let stbl = EfiSystemTable64T::default();
        let ei = ei_with_systab(b"NONE", addr_of(&stbl));
        assert_eq!(efi_get_conf_table(&ei, 0), Err(-EINVAL));
    }

    /// A `setup_data` node with an inline `efi_setup_data` payload, so a
    /// single host allocation provides Linux's `pa + sizeof(setup_data)`
    /// adjacency.
    #[repr(C)]
    #[derive(Default)]
    struct KexecNode {
        hdr: SetupData,
        esd: EfiSetupData,
    }

    #[test]
    fn kexec_setup_data_overrides_conf_table_address() {
        let stbl = EfiSystemTable64T {
            nr_tables: 3,
            tables: 0x1111,
            ..Default::default()
        };
        let node = KexecNode {
            hdr: SetupData {
                next: 0,
                type_: SETUP_EFI,
                len: core::mem::size_of::<EfiSetupData>() as u32,
            },
            esd: EfiSetupData {
                tables: 0x2222,
                ..Default::default()
            },
        };
        let ei = ei_with_systab(b"EL64", addr_of(&stbl));
        // kexec tables win; nr_tables still comes from the system table.
        assert_eq!(efi_get_conf_table(&ei, addr_of(&node)), Ok((0x2222, 3)));
    }

    #[test]
    fn kexec_walk_follows_next_chain_past_non_efi_nodes() {
        let stbl = EfiSystemTable64T {
            nr_tables: 1,
            tables: 0x1111,
            ..Default::default()
        };
        let efi_node = KexecNode {
            hdr: SetupData {
                next: 0,
                type_: SETUP_EFI,
                len: core::mem::size_of::<EfiSetupData>() as u32,
            },
            esd: EfiSetupData {
                tables: 0x3333,
                ..Default::default()
            },
        };
        // SETUP_E820_EXT (type 1) node linking to the EFI node.
        let first = SetupData {
            next: addr_of(&efi_node),
            type_: 1,
            len: 0,
        };
        let ei = ei_with_systab(b"EL64", addr_of(&stbl));
        assert_eq!(efi_get_conf_table(&ei, addr_of(&first)), Ok((0x3333, 1)));
    }

    #[test]
    fn kexec_data_without_tables_falls_back_to_system_table() {
        let stbl = EfiSystemTable64T {
            nr_tables: 2,
            tables: 0x4444,
            ..Default::default()
        };
        // esd.tables == 0 → "missing valid configuration table" → NULL.
        let node = KexecNode {
            hdr: SetupData {
                next: 0,
                type_: SETUP_EFI,
                len: core::mem::size_of::<EfiSetupData>() as u32,
            },
            esd: EfiSetupData::default(),
        };
        let ei = ei_with_systab(b"EL64", addr_of(&stbl));
        assert_eq!(efi_get_conf_table(&ei, addr_of(&node)), Ok((0x4444, 2)));
    }

    #[test]
    fn find_vendor_table_scans_config_entries_by_guid() {
        let rsdp_guid: [u8; 16] = *b"ACPI 2.0 TABLE!!";
        let entries = [
            EfiConfigTable64T {
                guid: [0xaa; 16],
                table: 0x100,
            },
            EfiConfigTable64T {
                guid: rsdp_guid,
                table: 0x200,
            },
            EfiConfigTable64T {
                guid: [0xbb; 16],
                table: 0x300,
            },
        ];
        let ei = ei_with_sig(b"EL64");
        let cfg_pa = entries.as_ptr() as u64;
        assert_eq!(efi_find_vendor_table(&ei, cfg_pa, 3, &rsdp_guid), 0x200);
        // Absent GUID → 0.
        assert_eq!(efi_find_vendor_table(&ei, cfg_pa, 3, &[0xcc; 16]), 0);
        // Entries past cfg_tbl_len are not searched.
        assert_eq!(efi_find_vendor_table(&ei, cfg_pa, 1, &rsdp_guid), 0);
    }

    #[test]
    fn find_vendor_table_uses_32bit_entry_stride_under_efi32() {
        let needle: [u8; 16] = [7; 16];
        let entries = [
            EfiConfigTable32T {
                guid: [0xaa; 16],
                table: 0x10,
            },
            EfiConfigTable32T {
                guid: needle,
                table: 0x20,
            },
        ];
        let ei = ei_with_sig(b"EL32");
        let cfg_pa = entries.as_ptr() as u64;
        assert_eq!(efi_find_vendor_table(&ei, cfg_pa, 2, &needle), 0x20);
    }

    #[test]
    fn find_vendor_table_returns_zero_without_efi_environment() {
        // EFI_TYPE_NONE short-circuits before any memory access.
        let ei = ei_with_sig(b"NONE");
        assert_eq!(efi_find_vendor_table(&ei, 0xdead_0000, 5, &[0; 16]), 0);
    }
}
