//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/callthunks.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/callthunks.c
//! Call-depth thunk patching model.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/callthunks.c
//!
//! Linux patches direct calls so they target per-function call-depth thunks.
//! This module ports the address filtering, rel32 encoding, and accounting
//! decisions while leaving executable text mutation to `alternative.rs`.

use crate::arch::x86::kernel::alternative::{CALL_INSN_OPCODE, MAX_PATCH_LEN};
use crate::include::uapi::errno::EINVAL;

pub const SKL_CALL_THUNK_TEMPLATE_MAX: usize = MAX_PATCH_LEN;
pub const CALL_INSN_SIZE: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreText {
    pub base: u64,
    pub end: u64,
}

impl CoreText {
    pub const fn contains(self, addr: u64) -> bool {
        self.base <= addr && addr < self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpecialCallTarget {
    ErrorEntry,
    XenErrorEntry,
    ParanoidEntry,
    SwitchToAsm,
    RetFromFork,
    SoftRestartCpu,
    Fentry,
    RelocateKernel,
    Other,
}

pub const fn skip_addr(target: SpecialCallTarget) -> bool {
    !matches!(target, SpecialCallTarget::Other)
}

pub fn call_get_dest(site: u64, insn: &[u8]) -> Result<Option<u64>, i32> {
    if insn.len() < CALL_INSN_SIZE {
        return Err(EINVAL);
    }
    if insn[0] != CALL_INSN_OPCODE {
        return Ok(None);
    }
    let imm = i32::from_le_bytes([insn[1], insn[2], insn[3], insn[4]]) as i64;
    Ok(Some(
        site.wrapping_add(CALL_INSN_SIZE as u64)
            .wrapping_add(imm as u64),
    ))
}

pub fn emit_call(from: u64, dest: u64) -> Result<[u8; CALL_INSN_SIZE], i32> {
    let next = from.wrapping_add(CALL_INSN_SIZE as u64);
    let rel = dest as i128 - next as i128;
    if rel < i32::MIN as i128 || rel > i32::MAX as i128 {
        return Err(EINVAL);
    }
    let mut out = [0u8; CALL_INSN_SIZE];
    out[0] = CALL_INSN_OPCODE;
    out[1..].copy_from_slice(&(rel as i32).to_le_bytes());
    Ok(out)
}

pub fn patch_dest(function_entry: u64, template_size: usize) -> Result<u64, i32> {
    if template_size == 0 || template_size > SKL_CALL_THUNK_TEMPLATE_MAX {
        return Err(EINVAL);
    }
    function_entry
        .checked_sub(template_size as u64)
        .ok_or(EINVAL)
}

pub fn translate_call_dest(
    thunks_initialized: bool,
    dest: u64,
    template_size: usize,
    in_core_text: bool,
    skip: bool,
) -> Result<u64, i32> {
    if !thunks_initialized || skip || !in_core_text {
        return Ok(dest);
    }
    patch_dest(dest, template_size)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CallDepthCounters {
    pub calls: u64,
    pub returns: u64,
    pub stuffs: u64,
    pub context_switches: u64,
}

impl CallDepthCounters {
    pub const fn debug_line(self) -> [u64; 4] {
        [self.calls, self.returns, self.stuffs, self.context_switches]
    }
}

pub fn x86_call_depth_emit_accounting(
    thunks_initialized: bool,
    func_already_thunked: bool,
    template: &[u8],
    out: &mut [u8],
) -> Result<usize, i32> {
    if !thunks_initialized || func_already_thunked {
        return Ok(0);
    }
    if template.len() > out.len() || template.len() > SKL_CALL_THUNK_TEMPLATE_MAX {
        return Err(EINVAL);
    }
    out[..template.len()].copy_from_slice(template);
    Ok(template.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coretext_range_is_half_open() {
        let text = CoreText {
            base: 0x1000,
            end: 0x2000,
        };
        assert!(text.contains(0x1000));
        assert!(text.contains(0x1fff));
        assert!(!text.contains(0x2000));
    }

    #[test]
    fn call_get_dest_decodes_rel32_call() {
        let call = emit_call(0x1000, 0x1100).unwrap();
        assert_eq!(call_get_dest(0x1000, &call), Ok(Some(0x1100)));
    }

    #[test]
    fn emit_call_rejects_out_of_range_target() {
        assert_eq!(emit_call(0, u64::MAX), Err(EINVAL));
    }

    #[test]
    fn translation_uses_padding_before_destination() {
        assert_eq!(
            translate_call_dest(true, 0x2000, 16, true, false),
            Ok(0x1ff0)
        );
        assert_eq!(
            translate_call_dest(false, 0x2000, 16, true, false),
            Ok(0x2000)
        );
    }

    #[test]
    fn special_targets_are_skipped_like_linux() {
        assert!(skip_addr(SpecialCallTarget::ErrorEntry));
        assert!(skip_addr(SpecialCallTarget::RetFromFork));
        assert!(!skip_addr(SpecialCallTarget::Other));
    }

    #[test]
    fn accounting_template_is_emitted_once_enabled() {
        let mut out = [0u8; 8];
        let template = [0x65, 0x48, 0xff];
        assert_eq!(
            x86_call_depth_emit_accounting(true, false, &template, &mut out),
            Ok(3)
        );
        assert_eq!(&out[..3], &template);
        assert_eq!(
            x86_call_depth_emit_accounting(true, true, &template, &mut out),
            Ok(0)
        );
    }
}
