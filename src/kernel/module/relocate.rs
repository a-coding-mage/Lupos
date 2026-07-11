//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module
//! test-origin: linux:vendor/linux/kernel/module
//! ELF relocation engine for x86_64 `.ko` files.
//!
//! Mirrors `arch/x86/kernel/module.c:apply_relocate_add` (line 219).
//! Handles the subset of `R_X86_64_*` relocation types that typical kernel
//! modules use.  Each relocation:
//!   - Finds the target symbol address (from export table or module itself).
//!   - Computes the addend.
//!   - Patches the instruction at the relocation offset.
//!
//! References:
//!   - System V AMD64 ABI §4.4.1 "Relocation Types"
//!   - `arch/x86/kernel/module.c:219`

use crate::include::uapi::errno::ENOEXEC;

/// Supported relocation types for x86_64.
/// Values mirror `elf/common.h` constants from binutils.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelocType {
    /// No relocation.
    None = 0,
    /// Absolute 64-bit address: `S + A`.
    Abs64 = 1,
    /// PC-relative 32-bit: `S + A - P`.
    Pc32 = 2,
    /// PLT-relative 32-bit: Linux treats this like `R_X86_64_PC32`.
    Plt32 = 4,
    /// GOT-relative 32-bit. x86 Linux modules reject this relocation.
    GotPcRel = 9,
    /// 32-bit zero-extended absolute: `S + A`.
    _32 = 10,
    /// 32-bit sign-extended absolute.
    _32S = 11,
    /// PC-relative 64-bit: `S + A - P`.
    Pc64 = 24,
    /// Unknown (rejected).
    Unknown,
}

impl From<u32> for RelocType {
    fn from(v: u32) -> Self {
        match v {
            0 => Self::None,
            1 => Self::Abs64,
            2 => Self::Pc32,
            4 => Self::Plt32,
            9 => Self::GotPcRel,
            10 => Self::_32,
            11 => Self::_32S,
            24 => Self::Pc64,
            _ => Self::Unknown,
        }
    }
}

/// One decoded relocation entry (from RELA sections — Linux `.ko` always uses RELA).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rela {
    /// Offset within the target section to patch.
    pub offset: u64,
    /// Symbol table index.
    pub sym: u32,
    /// Relocation type.
    pub rel_type: RelocType,
    /// Addend stored in the RELA entry.
    pub addend: i64,
}

impl Rela {
    /// Decode one `Elf64_Rela` entry from a byte slice at position `pos`.
    ///
    /// # Returns
    /// `None` if the slice is too short.
    pub fn from_bytes(data: &[u8], pos: usize) -> Option<Self> {
        if pos + 24 > data.len() {
            return None;
        }
        let offset = u64::from_le_bytes(data[pos..pos + 8].try_into().ok()?);
        let info = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().ok()?);
        let addend = i64::from_le_bytes(data[pos + 16..pos + 24].try_into().ok()?);
        let sym = (info >> 32) as u32;
        let rel_type = RelocType::from((info & 0xFFFF_FFFF) as u32);
        Some(Self {
            offset,
            sym,
            rel_type,
            addend,
        })
    }
}

/// Apply one RELA relocation to `mem`.
///
/// `sym_addr` is the resolved symbol virtual address.
/// `patch_vaddr` is the virtual address of the byte at `mem[offset]`.
///
/// Mirrors `apply_relocate_add` in `arch/x86/kernel/module.c:219`.
pub fn apply_rela(
    mem: &mut [u8],
    offset: usize,
    rel_type: RelocType,
    sym_addr: u64,
    patch_vaddr: u64,
    addend: i64,
) -> Result<(), i32> {
    if rel_type == RelocType::None {
        return Ok(());
    }

    let mut value = sym_addr.wrapping_add(addend as u64);
    let size = match rel_type {
        RelocType::Abs64 => 8,
        RelocType::_32 => {
            if value != value as u32 as u64 {
                return Err(ENOEXEC);
            }
            4
        }
        RelocType::_32S => {
            if value as i64 != value as u32 as i32 as i64 {
                return Err(ENOEXEC);
            }
            4
        }
        RelocType::Pc32 | RelocType::Plt32 => {
            // Vendor Linux deliberately writes the low 32 bits here; module
            // placement, not the relocation helper, guarantees reachability.
            value = value.wrapping_sub(patch_vaddr);
            4
        }
        RelocType::Pc64 => {
            value = value.wrapping_sub(patch_vaddr);
            8
        }
        RelocType::GotPcRel | RelocType::Unknown | RelocType::None => {
            return Err(ENOEXEC);
        }
    };

    let end = offset.checked_add(size).ok_or(ENOEXEC)?;
    let target = mem.get_mut(offset..end).ok_or(ENOEXEC)?;
    // `arch/x86/kernel/module.c:__write_relocate_add()` refuses RELA
    // targets containing any pre-existing value. Accepting one would apply an
    // addend twice or overwrite executable data from a malformed module.
    if target.iter().any(|byte| *byte != 0) {
        return Err(ENOEXEC);
    }
    target.copy_from_slice(&value.to_le_bytes()[..size]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zeroed(n: usize) -> alloc::vec::Vec<u8> {
        alloc::vec![0u8; n]
    }

    #[test]
    fn abs64_patches_correctly() {
        let mut mem = zeroed(16);
        apply_rela(&mut mem, 0, RelocType::Abs64, 0xDEAD_BEEF_0000_0000, 0, 0).unwrap();
        let v = u64::from_le_bytes(mem[0..8].try_into().unwrap());
        assert_eq!(v, 0xDEAD_BEEF_0000_0000);
    }

    #[test]
    fn pc32_patches_correctly() {
        let mut mem = zeroed(8);
        // sym=0x1010, patch_at=0x1000, addend=-4  → S+A-P = 0x1010-4-0x1000 = 0xC
        apply_rela(&mut mem, 0, RelocType::Pc32, 0x1010, 0x1000, -4).unwrap();
        let v = i32::from_le_bytes(mem[0..4].try_into().unwrap());
        assert_eq!(v, 0xC);
    }

    #[test]
    fn plt32_uses_pc_relative_formula() {
        let mut mem = zeroed(8);
        apply_rela(&mut mem, 0, RelocType::Plt32, 0x2010, 0x2000, -4).unwrap();
        let v = i32::from_le_bytes(mem[0..4].try_into().unwrap());
        assert_eq!(v, 0xC);
    }

    #[test]
    fn unknown_type_returns_error() {
        let mut mem = zeroed(8);
        let r = apply_rela(&mut mem, 0, RelocType::Unknown, 0, 0, 0);
        assert_eq!(r, Err(ENOEXEC));
    }

    #[test]
    fn abs32_rejects_overflow_like_linux() {
        let mut mem = zeroed(8);
        assert_eq!(
            apply_rela(&mut mem, 0, RelocType::_32, 0x1_0000_0000, 0, 0),
            Err(ENOEXEC)
        );
    }

    #[test]
    fn abs32s_accepts_module_window_and_rejects_direct_map() {
        let mut mem = zeroed(8);
        apply_rela(
            &mut mem,
            0,
            RelocType::_32S,
            crate::arch::x86::mm::init::MODULES_VADDR,
            0,
            0,
        )
        .unwrap();
        assert_eq!(
            i32::from_le_bytes(mem[0..4].try_into().unwrap()),
            -0x4000_0000
        );

        assert_eq!(
            apply_rela(
                &mut mem,
                0,
                RelocType::_32S,
                crate::arch::x86::mm::paging::PAGE_OFFSET,
                0,
                0,
            ),
            Err(ENOEXEC)
        );
    }
}
