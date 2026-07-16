//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/static_call.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/static_call.c
//! x86 static-call patch byte generation.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/static_call.c

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicUsize, Ordering};

use spin::Mutex;

use crate::arch::x86::kernel::alternative::{
    CALL_INSN_OPCODE, JMP32_INSN_OPCODE, text_poke_copy, x86_nop,
};
use crate::arch::x86::kernel::jump_label::text_gen_insn;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

pub const CALL_INSN_SIZE: usize = 5;
pub const RET_INSN_OPCODE: u8 = 0xc3;
pub const TRAMP_UD: [u8; 3] = [0x0f, 0xb9, 0xcc];
pub const XOR5RAX: [u8; 5] = [0x2e, 0x2e, 0x2e, 0x31, 0xc0];
pub const RETINSN: [u8; 5] = [RET_INSN_OPCODE, 0xcc, 0xcc, 0xcc, 0xcc];
pub const WARNINSN: [u8; 5] = [0x67, 0x48, 0x0f, 0xb9, 0x3a];
pub const STATIC_CALL_SITE_SIZE: usize = 8;
pub const STATIC_CALL_SITE_TAIL: usize = 1;
pub const STATIC_CALL_SITE_FLAGS: usize = 3;

static NEXT_STATIC_CALL_OWNER: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy)]
struct RegisteredStaticCallSite {
    owner: usize,
    addr: usize,
    key: usize,
    tail: bool,
}

static MODULE_STATIC_CALL_SITES: Mutex<Vec<RegisteredStaticCallSite>> = Mutex::new(Vec::new());
static STATIC_CALL_UPDATE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct StaticCallRegistration {
    owner: usize,
}

impl Drop for StaticCallRegistration {
    fn drop(&mut self) {
        self.unregister();
    }
}

impl StaticCallRegistration {
    /// Remove module-owned static-call sites from the live update registry.
    /// The operation is idempotent because the registration object can
    /// outlive `delete_module()` while another caller holds the descriptor.
    pub fn unregister(&self) {
        MODULE_STATIC_CALL_SITES
            .lock()
            .retain(|site| site.owner != self.owner);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleStaticCallSite {
    pub addr: usize,
    pub key: usize,
    pub flags: usize,
}

/// Encode the x86 PREL32 relation exactly as the CPU evaluates it.  Lupos'
/// boot image is executable through a low alias while module allocations use
/// canonical high addresses, so ordinary signed integer subtraction rejects
/// a relation which is valid after x86-64 address wrapping.
fn encode_prel32(from: usize, to: usize) -> Result<i32, i32> {
    let displacement = to.wrapping_sub(from) as u32 as i32;
    if from.wrapping_add_signed(displacement as isize) == to {
        Ok(displacement)
    } else {
        Err(EINVAL)
    }
}

/// Sort module static-call sites by key while preserving each PREL32 field at
/// its new address. Mirrors `static_call_sort_entries()` and
/// `static_call_site_swap()` in vendor/linux/kernel/static_call_inline.c.
pub fn sort_module_static_call_sites(base: usize, bytes: &mut [u8]) -> Result<usize, i32> {
    if bytes.len() % STATIC_CALL_SITE_SIZE != 0 {
        return Err(EINVAL);
    }
    let mut sites = Vec::with_capacity(bytes.len() / STATIC_CALL_SITE_SIZE);
    for offset in (0..bytes.len()).step_by(STATIC_CALL_SITE_SIZE) {
        let addr_disp =
            i32::from_le_bytes(bytes[offset..offset + 4].try_into().map_err(|_| EINVAL)?);
        let key_disp = i32::from_le_bytes(
            bytes[offset + 4..offset + 8]
                .try_into()
                .map_err(|_| EINVAL)?,
        );
        let slot = base.wrapping_add(offset);
        let raw_key = slot.wrapping_add(4).wrapping_add_signed(key_disp as isize);
        sites.push(ModuleStaticCallSite {
            addr: slot.wrapping_add_signed(addr_disp as isize),
            key: raw_key & !STATIC_CALL_SITE_FLAGS,
            flags: raw_key & STATIC_CALL_SITE_FLAGS,
        });
    }
    sites.sort_by_key(|site| site.key);
    for (index, site) in sites.iter().copied().enumerate() {
        let offset = index * STATIC_CALL_SITE_SIZE;
        let slot = base.wrapping_add(offset);
        let raw_key = site.key | site.flags;
        let addr = encode_prel32(slot, site.addr)?;
        let key = encode_prel32(slot.wrapping_add(4), raw_key)?;
        bytes[offset..offset + 4].copy_from_slice(&addr.to_le_bytes());
        bytes[offset + 4..offset + 8].copy_from_slice(&key.to_le_bytes());
    }
    Ok(sites.len())
}

// The real `__WARN_trap` body and its module-exported static-call trampoline,
// mirroring vendor/linux/arch/x86/entry/entry.S::__WARN_trap and
// vendor/linux/arch/x86/kernel/traps.c::EXPORT_STATIC_CALL_TRAMP(WARN_trap).
// The `ud1 (%edx), %rdi` byte sequence is exactly WARNINSN, the pattern
// decode_bug() classifies as BUG_UD1_WARN with the bug_entry pointer in
// pt_regs->di; the #UD handler reports the warning and resumes after the
// 5-byte insn, so the RET returns to the module caller.
//
// The 16-byte alignment is load-bearing: relocated `.static_call_sites` keys
// resolve to these exported addresses and are masked with
// STATIC_CALL_SITE_FLAGS before comparison
// (vendor/linux/kernel/static_call_inline.c::static_call_add_module uses the
// low key bits as INIT/TAIL flags).  A plain Rust fn has no alignment
// guarantee, so an unluckily placed stub made the masked comparison fail and
// every vendor module carrying WARN static-call sites (scsi_mod, libata,
// drm, snd, libphy, ...) was rejected at load.
core::arch::global_asm!(
    ".pushsection .text.lupos.warn_trap, \"ax\"",
    ".balign 16",
    ".global __WARN_trap",
    "__WARN_trap:",
    ".byte 0x67, 0x48, 0x0f, 0xb9, 0x3a", // ud1 (%edx), %rdi == WARNINSN
    "ret",
    ".balign 16",
    ".global __SCT__WARN_trap",
    "__SCT__WARN_trap:",
    "jmp __WARN_trap",
    ".popsection",
);

core::arch::global_asm!(
    ".pushsection .text.lupos.static_call, \"ax\"",
    ".balign 16",
    ".global __static_call_return0",
    ".type __static_call_return0,@function",
    "__static_call_return0:",
    "endbr64",
    "xor eax, eax",
    "ret",
    ".size __static_call_return0,.-__static_call_return0",
    ".balign 16",
    ".global __static_call_return",
    ".type __static_call_return,@function",
    "__static_call_return:",
    "ret",
    "int3",
    ".size __static_call_return,.-__static_call_return",
    ".popsection",
);

unsafe extern "C" {
    pub fn __WARN_trap(bug: *mut core::ffi::c_void, ...);
    pub fn __SCT__WARN_trap(bug: *mut core::ffi::c_void, ...);
    pub fn __static_call_return0() -> isize;
    pub fn __static_call_return();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaticCallInsn {
    Call,
    Nop,
    Jmp,
    Ret,
    Jcc(u8),
}

pub fn is_jcc(insn: &[u8]) -> Option<u8> {
    if insn.len() >= 2 && insn[0] == 0x0f && (insn[1] & 0xf0) == 0x80 {
        Some(insn[1])
    } else {
        None
    }
}

pub const fn sc_insn(null: bool, tail: bool) -> StaticCallInsn {
    match (tail, null) {
        (false, false) => StaticCallInsn::Call,
        (false, true) => StaticCallInsn::Nop,
        (true, false) => StaticCallInsn::Jmp,
        (true, true) => StaticCallInsn::Ret,
    }
}

pub fn static_call_transform_bytes(
    site: u64,
    current: &[u8],
    kind: StaticCallInsn,
    func: u64,
) -> Result<Vec<u8>, i32> {
    if let StaticCallInsn::Jmp | StaticCallInsn::Ret = kind {
        if let Some(op) = is_jcc(current) {
            return Ok(static_call_jcc(site, op, func));
        }
    }
    match kind {
        StaticCallInsn::Call => Ok(text_gen_insn(CALL_INSN_OPCODE, CALL_INSN_SIZE, site, func)),
        StaticCallInsn::Nop => x86_nop(CALL_INSN_SIZE)
            .map(|bytes| bytes.to_vec())
            .ok_or(EINVAL),
        StaticCallInsn::Jmp => Ok(text_gen_insn(JMP32_INSN_OPCODE, CALL_INSN_SIZE, site, func)),
        StaticCallInsn::Ret => Ok(RETINSN.to_vec()),
        StaticCallInsn::Jcc(op) => Ok(static_call_jcc(site, op, func)),
    }
}

pub fn warn_trap_addr() -> usize {
    __WARN_trap as usize
}

pub fn warn_trap_trampoline_addr() -> usize {
    __SCT__WARN_trap as usize
}

pub fn register_module_exports() {
    if find_symbol("__WARN_trap").is_none() {
        export_symbol("__WARN_trap", warn_trap_addr(), false);
    }
    if find_symbol("__SCT__WARN_trap").is_none() {
        export_symbol("__SCT__WARN_trap", warn_trap_trampoline_addr(), false);
    }
    if find_symbol("__static_call_return0").is_none() {
        export_symbol(
            "__static_call_return0",
            __static_call_return0 as usize,
            true,
        );
    }
    if find_symbol("__static_call_update").is_none() {
        export_symbol(
            "__static_call_update",
            linux_static_call_update as usize,
            true,
        );
    }
    if find_symbol("arch_static_call_transform").is_none() {
        export_symbol(
            "arch_static_call_transform",
            linux_arch_static_call_transform as usize,
            true,
        );
    }
}

pub fn static_call_fixup_warn_site(site: &mut [u8]) -> Result<(), i32> {
    static_call_validate(site, false, false)?;
    text_poke_copy(&mut site[..WARNINSN.len()], &WARNINSN)
}

pub fn static_call_jcc(site: u64, op: u8, func: u64) -> Vec<u8> {
    let mut out = alloc::vec![0x0f, op, 0, 0, 0, 0];
    let rel = (func as i64).wrapping_sub((site + 6) as i64) as i32;
    out[2..6].copy_from_slice(&rel.to_le_bytes());
    out
}

pub fn static_call_validate(insn: &[u8], tail: bool, tramp: bool) -> Result<(), i32> {
    if tramp && (insn.len() < 8 || insn[5..8] != TRAMP_UD) {
        return Err(EINVAL);
    }
    let op = insn.first().copied().ok_or(EINVAL)?;
    if tail {
        if op == JMP32_INSN_OPCODE || op == RET_INSN_OPCODE || is_jcc(insn).is_some() {
            return Ok(());
        }
    } else if op == CALL_INSN_OPCODE
        || insn.get(..5) == Some(x86_nop(5).unwrap_or(&[]))
        || insn.get(..5) == Some(&XOR5RAX)
        || insn.get(..5) == Some(&WARNINSN)
    {
        return Ok(());
    }
    Err(EINVAL)
}

fn read_pointer(address: usize) -> Result<usize, i32> {
    if address == 0 || address & (core::mem::align_of::<usize>() - 1) != 0 {
        return Err(EINVAL);
    }
    let mut bytes = [0u8; core::mem::size_of::<usize>()];
    unsafe {
        crate::arch::x86::mm::maccess::copy_from_kernel_nofault(
            bytes.as_mut_ptr(),
            address as *const u8,
            bytes.len(),
        )
    }
    .map_err(|_| EINVAL)?;
    Ok(usize::from_le_bytes(bytes))
}

fn key_function(key: usize) -> Result<usize, i32> {
    if key == warn_trap_trampoline_addr() || key == warn_trap_addr() {
        Ok(warn_trap_addr())
    } else {
        read_pointer(key)
    }
}

fn transform_site(
    site: usize,
    func: usize,
    tail: bool,
    trampoline: bool,
    early: bool,
) -> Result<(), i32> {
    let current = crate::arch::x86::kernel::alternative::text_poke_read(site, 8)?;
    static_call_validate(&current, tail, trampoline)?;
    let kind = sc_insn(func == 0, tail);
    let bytes = if !tail && func == warn_trap_addr() {
        WARNINSN.to_vec()
    } else if !tail && func == __static_call_return0 as usize {
        XOR5RAX.to_vec()
    } else {
        static_call_transform_bytes(site as u64, &current, kind, func as u64)?
    };
    if current.get(..bytes.len()) == Some(bytes.as_slice()) {
        return Ok(());
    }
    if early {
        crate::arch::x86::kernel::alternative::text_poke_early(site, &bytes)
    } else {
        crate::arch::x86::kernel::alternative::text_poke_live(site, &bytes)
    }
}

fn decode_registered_sites(
    owner: usize,
    base: usize,
    bytes: &[u8],
) -> Result<Vec<RegisteredStaticCallSite>, i32> {
    if bytes.len() % STATIC_CALL_SITE_SIZE != 0 {
        return Err(EINVAL);
    }
    let mut sites = Vec::with_capacity(bytes.len() / STATIC_CALL_SITE_SIZE);
    for offset in (0..bytes.len()).step_by(STATIC_CALL_SITE_SIZE) {
        let slot = base.wrapping_add(offset);
        let addr_disp =
            i32::from_le_bytes(bytes[offset..offset + 4].try_into().map_err(|_| EINVAL)?);
        let key_disp = i32::from_le_bytes(
            bytes[offset + 4..offset + 8]
                .try_into()
                .map_err(|_| EINVAL)?,
        );
        let raw_key = slot.wrapping_add(4).wrapping_add_signed(key_disp as isize);
        sites.push(RegisteredStaticCallSite {
            owner,
            addr: slot.wrapping_add_signed(addr_disp as isize),
            key: raw_key & !STATIC_CALL_SITE_FLAGS,
            tail: raw_key & STATIC_CALL_SITE_TAIL != 0,
        });
    }
    Ok(sites)
}

/// Register sorted module static-call sites and patch them directly to the
/// current key target before module text becomes executable.
pub fn register_module_static_call_sites(
    base: usize,
    bytes: &[u8],
) -> Result<Option<StaticCallRegistration>, i32> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let owner = NEXT_STATIC_CALL_OWNER.fetch_add(1, Ordering::Relaxed);
    let sites = decode_registered_sites(owner, base, bytes)?;
    for site in &sites {
        transform_site(site.addr, key_function(site.key)?, site.tail, false, true)?;
    }
    MODULE_STATIC_CALL_SITES.lock().extend(sites);
    Ok(Some(StaticCallRegistration { owner }))
}

fn update_registered_key(key: usize, old_func: usize, func: usize) -> Result<(), i32> {
    let sites = MODULE_STATIC_CALL_SITES
        .lock()
        .iter()
        .filter(|site| site.key == key)
        .copied()
        .collect::<Vec<_>>();
    for (index, site) in sites.iter().enumerate() {
        if let Err(error) = transform_site(site.addr, func, site.tail, false, false) {
            for previous in sites[..index].iter().rev() {
                let _ = transform_site(previous.addr, old_func, previous.tail, false, false);
            }
            return Err(error);
        }
    }
    Ok(())
}

/// Transactional static-call update used by native Lupos subsystems.  The
/// exported Linux function is `void`, but internal callers need to know if a
/// live text mutation failed so they can avoid publishing mismatched keys,
/// trampolines, and callsites.
pub unsafe fn static_call_update_result(
    key: *mut c_void,
    trampoline: *mut c_void,
    func: *mut c_void,
) -> Result<(), i32> {
    if key.is_null() {
        return Err(EINVAL);
    }
    let _update = STATIC_CALL_UPDATE_LOCK.lock();
    let old_func = key_function(key as usize)?;
    if !trampoline.is_null() {
        transform_site(trampoline as usize, func as usize, true, true, false)?;
    }
    if let Err(error) = update_registered_key(key as usize, old_func, func as usize) {
        if !trampoline.is_null() {
            let _ = transform_site(trampoline as usize, old_func, true, true, false);
        }
        return Err(error);
    }
    unsafe {
        (key as *mut usize).write_volatile(func as usize);
    }
    Ok(())
}

#[unsafe(export_name = "arch_static_call_transform")]
pub unsafe extern "C" fn linux_arch_static_call_transform(
    site: *mut c_void,
    trampoline: *mut c_void,
    func: *mut c_void,
    tail: bool,
) {
    let _update = STATIC_CALL_UPDATE_LOCK.lock();
    if site.is_null() && !trampoline.is_null() {
        let _ = transform_site(trampoline as usize, func as usize, true, true, false);
    }
    if !site.is_null() {
        let _ = transform_site(site as usize, func as usize, tail, false, false);
    }
}

#[unsafe(export_name = "__static_call_update")]
pub unsafe extern "C" fn linux_static_call_update(
    key: *mut c_void,
    trampoline: *mut c_void,
    func: *mut c_void,
) {
    if let Err(error) = unsafe { static_call_update_result(key, trampoline, func) } {
        crate::log_error!(
            "static_call",
            "update rejected: key={:#x} trampoline={:#x} func={:#x} errno={}",
            key as usize,
            trampoline as usize,
            func as usize,
            error
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_prel32(bytes: &mut [u8], offset: usize, from: usize, to: usize) {
        bytes[offset..offset + 4].copy_from_slice(&encode_prel32(from, to).unwrap().to_le_bytes());
    }

    #[test]
    fn prel32_sort_preserves_a_key_across_the_canonical_alias_wrap() {
        let base = 0xffff_ffff_c100_0000usize;
        let key = 0x0000_0000_0050_0000usize;
        let site = base + 0x100;
        let mut metadata = [0u8; STATIC_CALL_SITE_SIZE];
        write_prel32(&mut metadata, 0, base, site);
        write_prel32(&mut metadata, 4, base + 4, key);

        assert_eq!(sort_module_static_call_sites(base, &mut metadata), Ok(1));
        let decoded = decode_registered_sites(1, base, &metadata).unwrap();
        assert_eq!(decoded[0].addr, site);
        assert_eq!(decoded[0].key, key);
    }

    #[test]
    fn transforms_match_linux_static_call_shapes() {
        let call =
            static_call_transform_bytes(0x1000, &[0xe8, 0, 0, 0, 0], StaticCallInsn::Call, 0x2000)
                .unwrap();
        assert_eq!(call[0], CALL_INSN_OPCODE);
        let nop = static_call_transform_bytes(0x1000, &call, StaticCallInsn::Nop, 0).unwrap();
        assert_eq!(nop.len(), 5);
        let ret = static_call_transform_bytes(0x1000, &[RET_INSN_OPCODE], StaticCallInsn::Ret, 0)
            .unwrap();
        assert_eq!(ret, RETINSN);
    }

    #[test]
    fn validates_tail_and_trampoline_signatures() {
        assert!(static_call_validate(&[JMP32_INSN_OPCODE, 0, 0, 0, 0], true, false).is_ok());
        assert!(static_call_validate(&[0xe8, 0, 0, 0, 0, 0x0f, 0xb9, 0xcc], false, true).is_ok());
        assert_eq!(static_call_validate(&[0x90; 8], false, true), Err(EINVAL));
    }

    #[test]
    fn module_registration_and_live_update_patch_the_real_site() {
        let mut site = [CALL_INSN_OPCODE, 0, 0, 0, 0, 0xcc, 0xcc, 0xcc];
        let first_target = site.as_ptr() as usize + 0x100;
        let second_target = site.as_ptr() as usize + 0x180;
        let mut key = [first_target, 0usize];
        let mut metadata = [0u8; STATIC_CALL_SITE_SIZE];
        let base = metadata.as_ptr() as usize;
        write_prel32(&mut metadata, 0, base, site.as_ptr() as usize);
        write_prel32(&mut metadata, 4, base + 4, key.as_mut_ptr() as usize);

        let registration = register_module_static_call_sites(base, &metadata)
            .unwrap()
            .unwrap();
        let first = static_call_transform_bytes(
            site.as_ptr() as u64,
            &site,
            StaticCallInsn::Call,
            first_target as u64,
        )
        .unwrap();
        assert_eq!(&site[..5], first.as_slice());

        unsafe {
            linux_static_call_update(
                key.as_mut_ptr().cast(),
                core::ptr::null_mut(),
                second_target as *mut c_void,
            );
        }
        let second = static_call_transform_bytes(
            site.as_ptr() as u64,
            &site,
            StaticCallInsn::Call,
            second_target as u64,
        )
        .unwrap();
        assert_eq!(&site[..5], second.as_slice());
        assert_eq!(key[0], second_target);
        drop(registration);
    }
}
