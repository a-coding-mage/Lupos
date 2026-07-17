//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/alternative.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/alternative.c
//! x86 alternative instruction patching helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/alternative.c
//!
//! Linux rewrites instruction sites during early boot and module load. Lupos
//! keeps live text mutation behind a fail-closed seam, but the byte-level
//! NOP, relocation, and patch-site preparation rules are kept testable here.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, EIO};
use crate::kernel::module::{export_symbol, find_symbol};

pub const MAX_PATCH_LEN: usize = 254;

pub const DA_ALT: u32 = 0x01;
pub const DA_RET: u32 = 0x02;
pub const DA_RETPOLINE: u32 = 0x04;
pub const DA_ENDBR: u32 = 0x08;
pub const DA_SMP: u32 = 0x10;

pub const ALT_FLAGS_SHIFT: u32 = 16;
pub const ALT_FLAG_NOT: u16 = 1 << 0;
pub const ALT_FLAG_DIRECT_CALL: u16 = 1 << 1;

pub const CALL_INSN_OPCODE: u8 = 0xe8;
pub const JMP32_INSN_OPCODE: u8 = 0xe9;
pub const RET_INSN_OPCODE: u8 = 0xc3;
pub const INT3_INSN_OPCODE: u8 = 0xcc;
pub const ENDBR_INSN_SIZE: usize = 4;
pub const ENDBR64: [u8; ENDBR_INSN_SIZE] = [0xf3, 0x0f, 0x1e, 0xfa];
pub const ENDBR32: [u8; ENDBR_INSN_SIZE] = [0xf3, 0x0f, 0x1e, 0xfb];
// gen_endbr_poison(): vendor/linux/arch/x86/include/asm/ibt.h
pub const ENDBR_POISON: [u8; ENDBR_INSN_SIZE] = [0x0f, 0x1f, 0x40, 0xd6];

pub static ALTERNATIVES_PATCHED: AtomicBool = AtomicBool::new(false);

const TEXT_POKE_IDLE: u8 = 0;
const TEXT_POKE_WRITING: u8 = 1;
const TEXT_POKE_COMPLETE: u8 = 2;

static TEXT_POKE_LOCK: Mutex<()> = Mutex::new(());
static TEXT_POKE_ADDRESS: AtomicUsize = AtomicUsize::new(0);
static TEXT_POKE_STATE: AtomicU8 = AtomicU8::new(TEXT_POKE_IDLE);
static TEXT_POKE_GENERATION: AtomicU64 = AtomicU64::new(0);
static TEXT_POKE_ACK: [AtomicU64; crate::kernel::sched::MAX_CPUS] =
    [const { AtomicU64::new(0) }; crate::kernel::sched::MAX_CPUS];

// DEFINE_ASM_FUNC(nop_func/BUG_func), alternative.c. ENDBR64 is harmless when
// CET is disabled and makes both exported indirect targets valid when enabled.
core::arch::global_asm!(
    ".pushsection .entry.text, \"ax\"",
    ".balign 16",
    ".global nop_func",
    ".type nop_func,@function",
    "nop_func:",
    "endbr64",
    "jmp __x86_return_thunk",
    ".size nop_func,.-nop_func",
    ".balign 16",
    ".global BUG_func",
    ".type BUG_func,@function",
    "BUG_func:",
    "endbr64",
    "ud2",
    ".size BUG_func,.-BUG_func",
    ".popsection",
);

unsafe extern "C" {
    pub fn nop_func();
    pub fn BUG_func();
}

pub fn register_module_exports() {
    if find_symbol("nop_func").is_none() {
        export_symbol("nop_func", nop_func as usize, true);
    }
    if find_symbol("BUG_func").is_none() {
        export_symbol("BUG_func", BUG_func as usize, false);
    }
}

pub const X86_NOP1: &[u8] = &[0x90];
pub const X86_NOP2: &[u8] = &[0x66, 0x90];
pub const X86_NOP3: &[u8] = &[0x0f, 0x1f, 0x00];
pub const X86_NOP4: &[u8] = &[0x0f, 0x1f, 0x40, 0x00];
pub const X86_NOP5: &[u8] = &[0x0f, 0x1f, 0x44, 0x00, 0x00];
pub const X86_NOP6: &[u8] = &[0x66, 0x0f, 0x1f, 0x44, 0x00, 0x00];
pub const X86_NOP7: &[u8] = &[0x0f, 0x1f, 0x80, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP8: &[u8] = &[0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP9: &[u8] = &[0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP10: &[u8] = &[0x66, 0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00];
pub const X86_NOP11: &[u8] = &[
    0x66, 0x66, 0x2e, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub const ASM_NOP_MAX: usize = 11;

pub fn x86_nop(len: usize) -> Option<&'static [u8]> {
    match len {
        1 => Some(X86_NOP1),
        2 => Some(X86_NOP2),
        3 => Some(X86_NOP3),
        4 => Some(X86_NOP4),
        5 => Some(X86_NOP5),
        6 => Some(X86_NOP6),
        7 => Some(X86_NOP7),
        8 => Some(X86_NOP8),
        9 => Some(X86_NOP9),
        10 => Some(X86_NOP10),
        11 => Some(X86_NOP11),
        _ => None,
    }
}

pub fn add_nops(out: &mut [u8]) {
    let mut off = 0;
    while off < out.len() {
        let chunk = (out.len() - off).min(ASM_NOP_MAX);
        let nop = x86_nop(chunk).expect("chunk is <= ASM_NOP_MAX and non-zero");
        out[off..off + chunk].copy_from_slice(nop);
        off += chunk;
    }
}

pub fn is_nop_at(bytes: &[u8], offset: usize) -> Option<usize> {
    if offset >= bytes.len() {
        return None;
    }
    for len in (1..=ASM_NOP_MAX).rev() {
        if offset + len <= bytes.len() && x86_nop(len) == Some(&bytes[offset..offset + len]) {
            return Some(len);
        }
    }
    if bytes[offset] == 0x90 { Some(1) } else { None }
}

pub fn skip_nops(bytes: &[u8], mut offset: usize) -> usize {
    while let Some(len) = is_nop_at(bytes, offset) {
        offset += len;
    }
    offset
}

pub fn apply_reloc(width: usize, value: u64, diff: i64) -> Result<u64, i32> {
    let mask = match width {
        1 => 0xff,
        2 => 0xffff,
        4 => 0xffff_ffff,
        8 => u64::MAX,
        _ => return Err(EINVAL),
    };
    Ok(value.wrapping_add(diff as u64) & mask)
}

pub const fn need_reloc(offset: usize, src_len: usize) -> bool {
    offset < src_len
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AltInstr {
    pub cpuid: u16,
    pub instrlen: u8,
    pub replacementlen: u8,
    pub flags: u16,
}

impl AltInstr {
    pub const fn should_patch(self, feature_present: bool) -> bool {
        let patch_when_not = (self.flags & ALT_FLAG_NOT) != 0;
        feature_present != patch_when_not
    }
}

pub fn prepare_patch_site(
    original: &[u8],
    replacement: Option<&[u8]>,
    feature_present: bool,
    alt: AltInstr,
) -> Result<Vec<u8>, i32> {
    if original.len() > MAX_PATCH_LEN {
        return Err(EINVAL);
    }
    if !alt.should_patch(feature_present) {
        return Ok(original.to_vec());
    }

    let repl = replacement.ok_or(EINVAL)?;
    if repl.len() > original.len() {
        return Err(EINVAL);
    }

    let mut out = vec![0u8; original.len()];
    out[..repl.len()].copy_from_slice(repl);
    add_nops(&mut out[repl.len()..]);
    Ok(out)
}

pub fn text_poke_copy(dst: &mut [u8], opcode: &[u8]) -> Result<(), i32> {
    if dst.len() != opcode.len() {
        return Err(EINVAL);
    }
    dst.copy_from_slice(opcode);
    Ok(())
}

pub fn text_poke_set(dst: &mut [u8], byte: u8) {
    dst.fill(byte);
}

/// The mitigation state consumed by Linux `patch_retpoline()`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RetpolinePatchPolicy {
    pub retpoline: bool,
    pub retpoline_lfence: bool,
    pub call_depth: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetpolineSite {
    pub opcode: u8,
    pub register: u8,
    pub length: usize,
    pub conditional_opcode: Option<u8>,
}

/// Decode an objtool `.retpoline_sites` instruction and recover the register
/// from its direct target in `__x86_indirect_thunk_array`.
///
/// Mirrors the accepted instruction forms in
/// `vendor/linux/arch/x86/kernel/alternative.c::apply_retpolines()`.
pub fn decode_retpoline_site(
    site: usize,
    insn: &[u8],
    thunk_register: impl FnOnce(usize) -> Option<u8>,
) -> Result<RetpolineSite, i32> {
    let cs_prefixes = insn
        .iter()
        .take(3)
        .take_while(|byte| **byte == 0x2e)
        .count();
    let direct_opcode = insn.get(cs_prefixes).copied();
    let (opcode, conditional_opcode, immediate_offset, length) =
        if direct_opcode == Some(CALL_INSN_OPCODE) || direct_opcode == Some(JMP32_INSN_OPCODE) {
            (insn[cs_prefixes], None, cs_prefixes + 1, cs_prefixes + 5)
        } else if cs_prefixes == 0
            && insn.first() == Some(&0x0f)
            && insn
                .get(1)
                .is_some_and(|opcode| (0x80..=0x8f).contains(opcode))
        {
            (JMP32_INSN_OPCODE, insn.get(1).copied(), 2usize, 6usize)
        } else {
            return Err(EINVAL);
        };
    if insn.len() < length {
        return Err(EINVAL);
    }
    let displacement = i32::from_le_bytes(
        insn[immediate_offset..immediate_offset + 4]
            .try_into()
            .map_err(|_| EINVAL)?,
    );
    let target = site
        .wrapping_add(length)
        .wrapping_add_signed(displacement as isize);
    let register = thunk_register(target).ok_or(EINVAL)?;
    if register == 4 {
        // Linux BUG_ON(reg == 4): indirect CALL/JMP through RSP cannot safely
        // use the stack-based retpoline construction.
        return Err(EINVAL);
    }
    Ok(RetpolineSite {
        opcode,
        register,
        length,
        conditional_opcode,
    })
}

/// Linux `emit_indirect()`: encode `CALL/JMP *%reg`, consuming spare bytes as
/// CS prefixes for calls and INT3 padding for jumps.
pub fn emit_indirect(opcode: u8, mut register: u8, length: usize) -> Result<Vec<u8>, i32> {
    if register > 15 {
        return Err(EINVAL);
    }
    let rex = usize::from(register >= 8);
    let excess = length.checked_sub(2 + rex).ok_or(EINVAL)?;
    let (cs_prefixes, int3_padding, mut modrm) = match opcode {
        CALL_INSN_OPCODE => (excess.min(3), 0, 0xd0),
        JMP32_INSN_OPCODE => (0, excess, 0xe0),
        _ => return Err(EINVAL),
    };
    let mut bytes = Vec::with_capacity(length);
    bytes.extend(core::iter::repeat(0x2e).take(cs_prefixes));
    if register >= 8 {
        bytes.push(0x41);
        register -= 8;
    }
    modrm += register;
    bytes.extend_from_slice(&[0xff, modrm]);
    bytes.extend(core::iter::repeat(INT3_INSN_OPCODE).take(int3_padding));
    if bytes.len() > length {
        return Err(EINVAL);
    }
    Ok(bytes)
}

/// Linux `patch_retpoline()` for non-ITS, non-FineIBT module sites.
/// `Ok(None)` means Linux deliberately keeps the compiler thunk call.
pub fn patch_retpoline(
    site_addr: usize,
    site: RetpolineSite,
    policy: RetpolinePatchPolicy,
) -> Result<Option<Vec<u8>>, i32> {
    if policy.retpoline && !policy.retpoline_lfence {
        if !policy.call_depth {
            return Ok(None);
        }

        // `emit_call_track_retpoline()`: keep the original direct branch
        // shape but redirect calls through an accounting thunk and jumps
        // through the non-accounting companion thunk.
        let call = site.conditional_opcode.is_none() && site.opcode == CALL_INSN_OPCODE;
        let target = crate::arch::x86::kernel::retpoline::call_depth_retpoline_thunk_addr(
            site.register,
            call,
        )
        .ok_or(EINVAL)?;
        let mut bytes = Vec::with_capacity(site.length);
        if let Some(opcode) = site.conditional_opcode {
            bytes.push(0x0f);
            bytes.push(opcode);
            let next = site_addr.wrapping_add(6);
            let displacement = target.wrapping_sub(next) as u32 as i32;
            if next.wrapping_add_signed(displacement as isize) != target {
                return Err(EINVAL);
            }
            bytes.extend_from_slice(&displacement.to_le_bytes());
        } else {
            let prefixes = site.length.checked_sub(5).ok_or(EINVAL)?;
            bytes.extend(core::iter::repeat(0x2e).take(prefixes));
            bytes.push(site.opcode);
            let next = site_addr.wrapping_add(site.length);
            let displacement = target.wrapping_sub(next) as u32 as i32;
            if next.wrapping_add_signed(displacement as isize) != target {
                return Err(EINVAL);
            }
            bytes.extend_from_slice(&displacement.to_le_bytes());
        }
        return (bytes.len() == site.length)
            .then_some(Some(bytes))
            .ok_or(EINVAL);
    }

    let mut bytes = Vec::with_capacity(site.length);
    let mut opcode = site.opcode;
    if let Some(cc) = site.conditional_opcode {
        bytes.push(0x70 + ((cc & 0x0f) ^ 1));
        bytes.push((site.length - 2) as u8);
        opcode = JMP32_INSN_OPCODE;
    }
    if policy.retpoline_lfence {
        bytes.extend_from_slice(&[0x0f, 0xae, 0xe8]);
    }
    let remaining = site.length.checked_sub(bytes.len()).ok_or(EINVAL)?;
    bytes.extend_from_slice(&emit_indirect(opcode, site.register, remaining)?);
    if bytes.len() < site.length {
        let old_len = bytes.len();
        bytes.resize(site.length, 0x90);
        add_nops(&mut bytes[old_len..]);
    }
    Ok(Some(bytes))
}

/// Linux `patch_return()` for a compiler-emitted return-thunk tail call.
pub fn patch_return(
    site: usize,
    insn: &[u8],
    compiler_return_thunk: usize,
    selected_return_thunk: usize,
    wants_rethunk: bool,
) -> Result<Vec<u8>, i32> {
    if insn.len() < 5 || insn[0] != JMP32_INSN_OPCODE {
        return Err(EINVAL);
    }
    let displacement = i32::from_le_bytes(insn[1..5].try_into().map_err(|_| EINVAL)?);
    let destination = site
        .wrapping_add(5)
        .wrapping_add_signed(displacement as isize);
    if destination != compiler_return_thunk {
        return Err(EINVAL);
    }
    if wants_rethunk {
        let next = site.wrapping_add(5);
        let displacement = selected_return_thunk.wrapping_sub(next) as u32 as i32;
        if next.wrapping_add_signed(displacement as isize) != selected_return_thunk {
            return Err(EINVAL);
        }
        let mut bytes = vec![JMP32_INSN_OPCODE];
        bytes.extend_from_slice(&displacement.to_le_bytes());
        Ok(bytes)
    } else {
        Ok(vec![
            RET_INSN_OPCODE,
            INT3_INSN_OPCODE,
            INT3_INSN_OPCODE,
            INT3_INSN_OPCODE,
            INT3_INSN_OPCODE,
        ])
    }
}

pub fn is_endbr(bytes: &[u8]) -> bool {
    bytes == ENDBR64 || bytes == ENDBR32 || bytes == ENDBR_POISON
}

/// Linux `poison_endbr()`/`apply_seal_endbr()` for one objtool site.
pub fn seal_endbr(site: &mut [u8]) -> Result<(), i32> {
    if site.len() < ENDBR_INSN_SIZE || !is_endbr(&site[..ENDBR_INSN_SIZE]) {
        return Err(EINVAL);
    }
    text_poke_copy(&mut site[..ENDBR_INSN_SIZE], &ENDBR_POISON)
}

pub const fn live_text_poke_supported() -> Result<(), i32> {
    Ok(())
}

#[cfg(not(test))]
fn sync_core_local() {
    // CPUID is a serializing instruction and flushes any predecoded stream
    // which could still contain the pre-patch bytes.
    let _ = crate::arch::x86::kernel::cpuid::cpuid(0, 0);
}

#[cfg(test)]
fn sync_core_local() {
    core::sync::atomic::compiler_fence(Ordering::SeqCst);
}

#[cfg(not(test))]
fn read_tsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    (u64::from(high) << 32) | u64::from(low)
}

/// Serialize instruction fetch on every online CPU after a text mutation.
/// The corresponding IPI entry is installed by `idt::init()`.
#[cfg(not(test))]
fn sync_core_all() -> Result<(), i32> {
    let generation = TEXT_POKE_GENERATION.fetch_add(1, Ordering::AcqRel) + 1;
    let current = crate::kernel::sched::current_cpu() as usize;
    let online = crate::kernel::cpuhotplug::cpu_online_mask();

    for cpu in 0..crate::kernel::sched::MAX_CPUS {
        if cpu == current || online & (1u64 << cpu) == 0 {
            continue;
        }
        unsafe {
            crate::arch::x86::kernel::apic::send_ipi(
                cpu as u8,
                crate::arch::x86::kernel::idt::TEXT_POKE_SYNC_VECTOR,
            );
        }
    }
    sync_core_local();
    TEXT_POKE_ACK[current.min(TEXT_POKE_ACK.len() - 1)].store(generation, Ordering::Release);

    let deadline = read_tsc().saturating_add(2_000_000_000);
    loop {
        let complete = (0..crate::kernel::sched::MAX_CPUS).all(|cpu| {
            online & (1u64 << cpu) == 0 || TEXT_POKE_ACK[cpu].load(Ordering::Acquire) >= generation
        });
        if complete {
            return Ok(());
        }
        if read_tsc() >= deadline {
            return Err(EIO);
        }
        core::hint::spin_loop();
    }
}

#[cfg(test)]
fn sync_core_all() -> Result<(), i32> {
    sync_core_local();
    Ok(())
}

/// Remote half of `sync_core_all()`; called from the dedicated text-poke IPI.
pub fn text_poke_sync_ipi_handler() {
    sync_core_local();
    let cpu = crate::kernel::sched::current_cpu() as usize;
    let generation = TEXT_POKE_GENERATION.load(Ordering::Acquire);
    TEXT_POKE_ACK[cpu.min(TEXT_POKE_ACK.len() - 1)].store(generation, Ordering::Release);
    #[cfg(not(test))]
    unsafe {
        crate::arch::x86::kernel::apic::eoi();
    }
}

/// Breakpoint-side half of Linux's `text_poke_bp()` protocol.
///
/// A CPU which reaches the temporary INT3 waits for the replacement to become
/// complete, then restarts at the patched instruction.  This prevents any CPU
/// from decoding a mixed old/new multi-byte instruction.
pub fn text_poke_bp_handler(frame: &crate::arch::x86::kernel::idt::ExceptionFrame) -> bool {
    let address = frame.rip.wrapping_sub(1) as usize;
    if TEXT_POKE_ADDRESS.load(Ordering::Acquire) != address {
        return false;
    }
    while TEXT_POKE_STATE.load(Ordering::Acquire) == TEXT_POKE_WRITING {
        core::hint::spin_loop();
    }
    unsafe {
        let frame = frame as *const _ as *mut crate::arch::x86::kernel::idt::ExceptionFrame;
        (*frame).rip = address as u64;
    }
    true
}

/// Read currently mapped kernel text for expected-byte verification.
pub fn text_poke_read(address: usize, len: usize) -> Result<Vec<u8>, i32> {
    crate::arch::x86::mm::init::text_poke_read(address, len)
}

/// Write text before it is executable (module relocation/finalization time).
pub fn text_poke_early(address: usize, bytes: &[u8]) -> Result<(), i32> {
    if address == 0 || bytes.is_empty() || address.checked_add(bytes.len()).is_none() {
        return Err(EINVAL);
    }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
    }
    Ok(())
}

/// W^X-safe live text replacement with Linux's temporary-breakpoint protocol.
pub fn text_poke_live(address: usize, bytes: &[u8]) -> Result<(), i32> {
    if address == 0 || bytes.is_empty() || address.checked_add(bytes.len()).is_none() {
        return Err(EINVAL);
    }
    let _lock = TEXT_POKE_LOCK.lock();
    let irq_flags = crate::kernel::locking::irqflags::local_irq_save();
    let result = (|| {
        if bytes.len() == 1 {
            crate::arch::x86::mm::init::text_poke_write_alias(address, bytes)?;
            return sync_core_all();
        }

        TEXT_POKE_ADDRESS.store(address, Ordering::Release);
        TEXT_POKE_STATE.store(TEXT_POKE_WRITING, Ordering::Release);
        crate::arch::x86::mm::init::text_poke_write_alias(address, &[INT3_INSN_OPCODE])?;
        sync_core_all()?;
        crate::arch::x86::mm::init::text_poke_write_alias(address + 1, &bytes[1..])?;
        sync_core_all()?;
        crate::arch::x86::mm::init::text_poke_write_alias(address, &bytes[..1])?;
        TEXT_POKE_STATE.store(TEXT_POKE_COMPLETE, Ordering::Release);
        sync_core_all()?;
        TEXT_POKE_STATE.store(TEXT_POKE_IDLE, Ordering::Release);
        TEXT_POKE_ADDRESS.store(0, Ordering::Release);
        Ok(())
    })();
    if result.is_err() {
        TEXT_POKE_STATE.store(TEXT_POKE_IDLE, Ordering::Release);
        TEXT_POKE_ADDRESS.store(0, Ordering::Release);
    }
    crate::kernel::locking::irqflags::local_irq_restore(irq_flags);
    result
}

pub fn mark_alternatives_patched() {
    ALTERNATIVES_PATCHED.store(true, Ordering::Release);
}

pub fn alternatives_patched() -> bool {
    ALTERNATIVES_PATCHED.load(Ordering::Acquire)
}

/// Linux `alternatives_smp_module_add()` for Lupos' current SMP text state.
///
/// `vendor/linux/arch/x86/kernel/alternative.c` returns immediately unless
/// the core kernel was first rewritten for UP execution (`uniproc_patched`).
/// Lupos never performs that global lock-prefix-to-DS-prefix rewrite, so its
/// module text already contains the SMP-safe `0xf0` lock prefixes and there is
/// no UP patch to apply or module entry to retain for a later CPU hotplug.
/// Linux does not dereference the lock-table or text ranges on that branch, so
/// section presence is all the current module finalizer needs to pass here.
///
/// The return value records Linux's `smp_alt_modules` list membership for the
/// matching `module_arch_cleanup()` path. It is always false until Lupos gains
/// the global UP-alternatives transition and the live text-poke support needed
/// to reverse it safely.
pub const fn alternatives_smp_module_add() -> bool {
    false
}

/// Linux `alternatives_smp_module_del()` for a module finalized above.
pub fn alternatives_smp_module_del(registered: bool) {
    debug_assert!(!registered);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nop_sequences_match_linux_64bit_table() {
        assert_eq!(ASM_NOP_MAX, 11);
        assert_eq!(x86_nop(1), Some(&[0x90][..]));
        assert_eq!(x86_nop(5), Some(&[0x0f, 0x1f, 0x44, 0x00, 0x00][..]));
        assert_eq!(
            x86_nop(11),
            Some(&[0x66, 0x66, 0x2e, 0x0f, 0x1f, 0x84, 0, 0, 0, 0, 0][..])
        );
    }

    #[test]
    fn add_nops_uses_longest_chunks() {
        let mut bytes = [0xcc; 13];
        add_nops(&mut bytes);
        assert_eq!(&bytes[..11], X86_NOP11);
        assert_eq!(&bytes[11..], X86_NOP2);
    }

    #[test]
    fn skip_nops_walks_mixed_linux_nops() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(X86_NOP5);
        bytes.extend_from_slice(X86_NOP2);
        bytes.push(0xcc);
        assert_eq!(skip_nops(&bytes, 0), 7);
    }

    #[test]
    fn alt_flag_not_inverts_feature_predicate() {
        let alt = AltInstr {
            cpuid: 7,
            instrlen: 4,
            replacementlen: 1,
            flags: ALT_FLAG_NOT,
        };
        assert!(!alt.should_patch(true));
        assert!(alt.should_patch(false));
    }

    #[test]
    fn prepare_patch_site_pads_replacement_with_linux_nops() {
        let alt = AltInstr {
            cpuid: 1,
            instrlen: 5,
            replacementlen: 1,
            flags: 0,
        };
        let out = prepare_patch_site(&[0xcc; 5], Some(&[RET_INSN_OPCODE]), true, alt).unwrap();
        assert_eq!(out[0], RET_INSN_OPCODE);
        assert_eq!(&out[1..], X86_NOP4);
    }

    #[test]
    fn apply_reloc_wraps_to_requested_width() {
        assert_eq!(apply_reloc(1, 0xff, 2), Ok(1));
        assert_eq!(apply_reloc(4, 0xffff_fffe, 3), Ok(1));
        assert_eq!(apply_reloc(3, 0, 1), Err(EINVAL));
    }

    #[test]
    fn live_text_poke_backend_is_available() {
        assert_eq!(live_text_poke_supported(), Ok(()));
    }

    #[test]
    fn return_thunk_rel32_accepts_canonical_address_wrap() {
        let site = 0xffff_ffff_c000_1000usize;
        let compiler = 0x0050_0000usize;
        let selected = 0x0060_0000usize;
        let next = site.wrapping_add(5);
        let original_disp = compiler.wrapping_sub(next) as u32 as i32;
        let mut original = [JMP32_INSN_OPCODE, 0, 0, 0, 0];
        original[1..].copy_from_slice(&original_disp.to_le_bytes());
        let patched = patch_return(site, &original, compiler, selected, true).unwrap();
        let patched_disp = i32::from_le_bytes(patched[1..5].try_into().unwrap());
        assert_eq!(next.wrapping_add_signed(patched_disp as isize), selected);
    }
}
