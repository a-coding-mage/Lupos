//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/jump_label.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/jump_label.c
//! Static jump-label code patching.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/kernel/jump_label.c
//!
//! Static keys compile down to either a 2-byte or 5-byte instruction site.
//! When the key is toggled, the site is rewritten between NOP and JMP. The
//! Rust side ports the byte-level transform algorithm plus the
//! expected-byte verification; the actual text mutation goes through a
//! `TextPoke` trait seam so we can keep "live text mutation behind a
//! fail-closed seam" (per `alternative.rs` precedent).

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::ffi::c_void;
use core::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

use spin::Mutex;

use crate::include::uapi::errno::{EFAULT, EINVAL};
use crate::kernel::module::{export_symbol, find_symbol};

use super::alternative::{JMP32_INSN_OPCODE, X86_NOP2, x86_nop};

// === Vendor constants — mirror asm/text-patching.h ===

pub const JMP32_INSN_SIZE: usize = 5;
pub const JMP8_INSN_OPCODE: u8 = 0xEB;
pub const JMP8_INSN_SIZE: usize = 2;
pub const JUMP_ENTRY_SIZE: usize = 16;
pub const JUMP_ENTRY_FLAGS: usize = 3;

static STATIC_KEY_INITIALIZED: bool = true;
static NEXT_JUMP_LABEL_OWNER: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Copy)]
struct RegisteredJumpEntry {
    owner: usize,
    code: usize,
    target: usize,
    key: usize,
    flags: usize,
    size: usize,
}

static MODULE_JUMP_ENTRIES: Mutex<Vec<RegisteredJumpEntry>> = Mutex::new(Vec::new());
static JUMP_LABEL_UPDATE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct JumpLabelRegistration {
    owner: usize,
}

impl Drop for JumpLabelRegistration {
    fn drop(&mut self) {
        self.unregister();
    }
}

impl JumpLabelRegistration {
    /// Stop resolving live static-key updates through this module's sites.
    /// This is deliberately idempotent so explicit module teardown and Drop
    /// can share the same cleanup path.
    pub fn unregister(&self) {
        // Serialize with update_key().  That function deliberately copies the
        // matching entries out of MODULE_JUMP_ENTRIES before doing text pokes;
        // without this lock an unload could remove the registry entries and
        // free module text while an updater still held copied addresses.
        let _update = JUMP_LABEL_UPDATE_LOCK.lock();
        MODULE_JUMP_ENTRIES
            .lock()
            .retain(|entry| entry.owner != self.owner);
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once(
        "static_key_initialized",
        &raw const STATIC_KEY_INITIALIZED as usize,
        false,
    );
    export_symbol_once("static_key_count", linux_static_key_count as usize, true);
    export_symbol_once(
        "static_key_fast_inc_not_disabled",
        linux_static_key_fast_inc_not_disabled as usize,
        true,
    );
    export_symbol_once(
        "static_key_slow_inc",
        linux_static_key_slow_inc as usize,
        true,
    );
    export_symbol_once(
        "static_key_slow_inc_cpuslocked",
        linux_static_key_slow_inc as usize,
        true,
    );
    export_symbol_once(
        "static_key_slow_dec",
        linux_static_key_slow_dec as usize,
        true,
    );
    export_symbol_once(
        "static_key_slow_dec_cpuslocked",
        linux_static_key_slow_dec as usize,
        true,
    );
    export_symbol_once("static_key_enable", linux_static_key_enable as usize, true);
    export_symbol_once(
        "static_key_enable_cpuslocked",
        linux_static_key_enable as usize,
        true,
    );
    export_symbol_once(
        "static_key_disable",
        linux_static_key_disable as usize,
        true,
    );
    export_symbol_once(
        "static_key_disable_cpuslocked",
        linux_static_key_disable as usize,
        true,
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModuleJumpEntry {
    pub code: usize,
    pub target: usize,
    pub key: usize,
    pub flags: usize,
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_i64(bytes: &[u8], offset: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn encode_prel32(from: usize, to: usize) -> Result<i32, i32> {
    let displacement = to.wrapping_sub(from) as u32 as i32;
    if from.wrapping_add_signed(displacement as isize) == to {
        Ok(displacement)
    } else {
        Err(EINVAL)
    }
}

/// Sort relocated module `struct jump_entry` records by key and code while
/// preserving PREL32/PREL64 encoding at their new slots.
///
/// Mirrors `jump_label_sort_entries()` and `jump_label_swap()` in
/// `vendor/linux/kernel/jump_label.c`.
pub fn sort_module_jump_entries(base: usize, bytes: &mut [u8]) -> Result<usize, i32> {
    if bytes.len() % JUMP_ENTRY_SIZE != 0 {
        return Err(EINVAL);
    }
    let mut entries = alloc::vec::Vec::with_capacity(bytes.len() / JUMP_ENTRY_SIZE);
    for offset in (0..bytes.len()).step_by(JUMP_ENTRY_SIZE) {
        let entry = base.wrapping_add(offset);
        let code = entry.wrapping_add_signed(read_i32(bytes, offset).ok_or(EINVAL)? as isize);
        let target = entry
            .wrapping_add(4)
            .wrapping_add_signed(read_i32(bytes, offset + 4).ok_or(EINVAL)? as isize);
        let raw_key = read_i64(bytes, offset + 8).ok_or(EINVAL)?;
        let flags = raw_key as usize & JUMP_ENTRY_FLAGS;
        let key = entry
            .wrapping_add(8)
            .wrapping_add_signed((raw_key & !(JUMP_ENTRY_FLAGS as i64)) as isize);
        entries.push(ModuleJumpEntry {
            code,
            target,
            key,
            flags,
        });
    }
    entries.sort_by_key(|entry| (entry.key, entry.code));
    for (index, entry) in entries.iter().copied().enumerate() {
        let offset = index * JUMP_ENTRY_SIZE;
        let slot = base.wrapping_add(offset);
        let code = encode_prel32(slot, entry.code)?;
        let target = encode_prel32(slot.wrapping_add(4), entry.target)?;
        let key = (entry.key.wrapping_sub(slot.wrapping_add(8)) | entry.flags) as i64;
        bytes[offset..offset + 4].copy_from_slice(&code.to_le_bytes());
        bytes[offset + 4..offset + 8].copy_from_slice(&target.to_le_bytes());
        bytes[offset + 8..offset + 16].copy_from_slice(&key.to_le_bytes());
    }
    Ok(entries.len())
}

/// `enum jump_label_type` — direction of the transform.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum JumpLabelType {
    /// `nop` site → `jmp` (key is now true).
    Jmp,
    /// `jmp` site → `nop` (key is now false).
    Nop,
}

/// `enum SystemState` — subset relevant to the jump-label transform path.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SystemState {
    Booting,
    Scheduling,
    Running,
}

/// Result of `__jump_label_patch` — the bytes to write into the
/// instruction site.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct JumpLabelPatch {
    pub code: alloc::vec::Vec<u8>,
    pub size: usize,
}

/// `text_gen_insn(opcode, addr, dest)` — write the opcode followed by a
/// PC-relative displacement (rip-relative = `dest - (addr + size)`).
pub fn text_gen_insn(opcode: u8, size: usize, addr: u64, dest: u64) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec::Vec::with_capacity(size);
    out.push(opcode);
    let rel = dest as i64 - (addr as i64 + size as i64);
    match size {
        JMP8_INSN_SIZE => {
            out.push(rel as i8 as u8);
        }
        JMP32_INSN_SIZE => {
            let rel32 = rel as i32;
            out.extend_from_slice(&rel32.to_le_bytes());
        }
        _ => {
            // Other instruction sizes are not jump-label-relevant.
        }
    }
    out
}

/// Construct the patch payload for an instruction site of `size` bytes,
/// transitioning to `type_` direction.
///
/// On `Jmp` the site previously held the NOP; on `Nop` it previously held
/// the JMP. The function returns the *new* bytes to write; callers verify
/// the *current* bytes (the "expected" form) before applying.
pub fn jump_label_patch(
    addr: u64,
    dest: u64,
    size: usize,
    type_: JumpLabelType,
) -> Result<JumpLabelPatch, i32> {
    let nop = match size {
        JMP8_INSN_SIZE => X86_NOP2.to_vec(),
        JMP32_INSN_SIZE => x86_nop(JMP32_INSN_SIZE).ok_or(EINVAL)?.to_vec(),
        _ => return Err(EINVAL),
    };
    let jmp = match size {
        JMP8_INSN_SIZE => text_gen_insn(JMP8_INSN_OPCODE, JMP8_INSN_SIZE, addr, dest),
        JMP32_INSN_SIZE => text_gen_insn(JMP32_INSN_OPCODE, JMP32_INSN_SIZE, addr, dest),
        _ => return Err(EINVAL),
    };
    let code = match type_ {
        JumpLabelType::Jmp => jmp,
        JumpLabelType::Nop => nop,
    };
    Ok(JumpLabelPatch { code, size })
}

/// The *expected* current bytes at `addr` before the transform — the
/// other variant of the pair.
pub fn jump_label_expected(
    addr: u64,
    dest: u64,
    size: usize,
    type_: JumpLabelType,
) -> Result<alloc::vec::Vec<u8>, i32> {
    let opposite = match type_ {
        JumpLabelType::Jmp => JumpLabelType::Nop,
        JumpLabelType::Nop => JumpLabelType::Jmp,
    };
    Ok(jump_label_patch(addr, dest, size, opposite)?.code)
}

/// Trait seam for the actual text write. Production wires this to
/// `text_poke_early` / `smp_text_poke_single`; tests use a `Vec<u8>`.
pub trait TextPoke {
    fn poke(&self, addr: u64, bytes: &[u8]) -> Result<(), i32>;
    fn read(&self, addr: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32>;
}

/// Linux's `__jump_label_transform`: verify the expected current bytes,
/// then write the new bytes. Returns `EFAULT` on byte mismatch — Linux
/// `BUG()`s here; we surface an errno instead to keep the kernel alive.
pub fn jump_label_transform<P: TextPoke>(
    poker: &P,
    addr: u64,
    dest: u64,
    size: usize,
    type_: JumpLabelType,
) -> Result<(), i32> {
    let expected = jump_label_expected(addr, dest, size, type_)?;
    let current = poker.read(addr, size)?;
    if current != expected {
        return Err(EFAULT);
    }
    let patch = jump_label_patch(addr, dest, size, type_)?;
    poker.poke(addr, &patch.code)
}

struct ProductionPoke {
    early: bool,
}

impl TextPoke for ProductionPoke {
    fn poke(&self, addr: u64, bytes: &[u8]) -> Result<(), i32> {
        if self.early {
            super::alternative::text_poke_early(addr as usize, bytes)
        } else {
            super::alternative::text_poke_live(addr as usize, bytes)
        }
    }

    fn read(&self, addr: u64, len: usize) -> Result<Vec<u8>, i32> {
        super::alternative::text_poke_read(addr as usize, len)
    }
}

fn decode_module_entries(base: usize, bytes: &[u8]) -> Result<Vec<ModuleJumpEntry>, i32> {
    if bytes.len() % JUMP_ENTRY_SIZE != 0 {
        return Err(EINVAL);
    }
    let mut entries = Vec::with_capacity(bytes.len() / JUMP_ENTRY_SIZE);
    for offset in (0..bytes.len()).step_by(JUMP_ENTRY_SIZE) {
        let slot = base.wrapping_add(offset);
        let code = slot.wrapping_add_signed(read_i32(bytes, offset).ok_or(EINVAL)? as isize);
        let target = slot
            .wrapping_add(4)
            .wrapping_add_signed(read_i32(bytes, offset + 4).ok_or(EINVAL)? as isize);
        let raw_key = read_i64(bytes, offset + 8).ok_or(EINVAL)?;
        entries.push(ModuleJumpEntry {
            code,
            target,
            key: slot
                .wrapping_add(8)
                .wrapping_add_signed((raw_key & !(JUMP_ENTRY_FLAGS as i64)) as isize),
            flags: raw_key as usize & JUMP_ENTRY_FLAGS,
        });
    }
    Ok(entries)
}

fn key_counter(key: usize) -> Result<&'static AtomicI32, i32> {
    if key == 0 || key & (core::mem::align_of::<AtomicI32>() - 1) != 0 {
        return Err(EINVAL);
    }
    Ok(unsafe { &*(key as *const AtomicI32) })
}

fn desired_type(entry: &RegisteredJumpEntry, enabled: bool) -> JumpLabelType {
    if enabled ^ (entry.flags & 1 != 0) {
        JumpLabelType::Jmp
    } else {
        JumpLabelType::Nop
    }
}

fn transform_registered(
    entry: &RegisteredJumpEntry,
    enabled: bool,
    early: bool,
) -> Result<(), i32> {
    let desired = desired_type(entry, enabled);
    let poker = ProductionPoke { early };
    let desired_bytes =
        jump_label_patch(entry.code as u64, entry.target as u64, entry.size, desired)?.code;
    let current = poker.read(entry.code as u64, entry.size)?;
    if current == desired_bytes {
        return Ok(());
    }
    let expected =
        jump_label_expected(entry.code as u64, entry.target as u64, entry.size, desired)?;
    if current != expected {
        return Err(EFAULT);
    }
    poker.poke(entry.code as u64, &desired_bytes)
}

/// Register a relocated, sorted module jump table and normalize every site to
/// the key's current dynamic state before module text becomes executable.
pub fn register_module_jump_entries(
    base: usize,
    bytes: &[u8],
) -> Result<Option<JumpLabelRegistration>, i32> {
    // A static-key transition must not fall between reading the key counter
    // and publishing the module sites.  Linux provides the same exclusion
    // with jump_label_lock() around jump_label_add_module().
    let _update = JUMP_LABEL_UPDATE_LOCK.lock();
    let decoded = decode_module_entries(base, bytes)?;
    if decoded.is_empty() {
        return Ok(None);
    }
    let owner = NEXT_JUMP_LABEL_OWNER.fetch_add(1, Ordering::Relaxed);
    let mut registered = Vec::with_capacity(decoded.len());
    for entry in decoded {
        let current = super::alternative::text_poke_read(entry.code, JMP32_INSN_SIZE)?;
        let size = arch_jump_entry_size(&current)?;
        let registered_entry = RegisteredJumpEntry {
            owner,
            code: entry.code,
            target: entry.target,
            key: entry.key,
            flags: entry.flags,
            size,
        };
        let enabled = key_counter(entry.key)?.load(Ordering::Acquire) != 0;
        transform_registered(&registered_entry, enabled, true)?;
        registered.push(registered_entry);
    }
    MODULE_JUMP_ENTRIES.lock().extend(registered);
    Ok(Some(JumpLabelRegistration { owner }))
}

fn update_key(key: usize, enabled: bool) -> Result<(), i32> {
    let entries = MODULE_JUMP_ENTRIES
        .lock()
        .iter()
        .filter(|entry| entry.key == key)
        .copied()
        .collect::<Vec<_>>();
    for (index, entry) in entries.iter().enumerate() {
        if let Err(error) = transform_registered(entry, enabled, false) {
            // Keep the text and counter transition atomic from callers'
            // perspective.  A failed later poke must not leave earlier sites
            // in the new state while slow_inc/slow_dec restores the counter.
            for previous in entries[..index].iter().rev() {
                let _ = transform_registered(previous, !enabled, false);
            }
            return Err(error);
        }
    }
    Ok(())
}

#[unsafe(export_name = "static_key_count")]
pub unsafe extern "C" fn linux_static_key_count(key: *mut c_void) -> i32 {
    key_counter(key as usize)
        .map(|counter| counter.load(Ordering::Acquire).max(0))
        .unwrap_or(0)
}

#[unsafe(export_name = "static_key_fast_inc_not_disabled")]
pub unsafe extern "C" fn linux_static_key_fast_inc_not_disabled(key: *mut c_void) -> bool {
    let Ok(counter) = key_counter(key as usize) else {
        return false;
    };
    let mut value = counter.load(Ordering::Acquire);
    loop {
        if value <= 0 || value == i32::MAX {
            return false;
        }
        match counter.compare_exchange_weak(value, value + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return true,
            Err(observed) => value = observed,
        }
    }
}

#[unsafe(export_name = "static_key_slow_inc")]
pub unsafe extern "C" fn linux_static_key_slow_inc(key: *mut c_void) -> bool {
    if unsafe { linux_static_key_fast_inc_not_disabled(key) } {
        return true;
    }
    let _update = JUMP_LABEL_UPDATE_LOCK.lock();
    let Ok(counter) = key_counter(key as usize) else {
        return false;
    };
    let value = counter.load(Ordering::Acquire);
    if value == 0 {
        counter.store(-1, Ordering::Release);
        if update_key(key as usize, true).is_err() {
            counter.store(0, Ordering::Release);
            return false;
        }
        counter.store(1, Ordering::Release);
        true
    } else if value > 0 && value != i32::MAX {
        counter.store(value + 1, Ordering::Release);
        true
    } else {
        false
    }
}

#[unsafe(export_name = "static_key_slow_dec")]
pub unsafe extern "C" fn linux_static_key_slow_dec(key: *mut c_void) {
    let _update = JUMP_LABEL_UPDATE_LOCK.lock();
    let Ok(counter) = key_counter(key as usize) else {
        return;
    };
    let value = counter.load(Ordering::Acquire);
    if value > 1 {
        counter.store(value - 1, Ordering::Release);
    } else if value == 1 {
        counter.store(0, Ordering::Release);
        if let Err(error) = update_key(key as usize, false) {
            counter.store(1, Ordering::Release);
            crate::log_error!(
                "jump_label",
                "static-key disable rejected: key={:#x} errno={}",
                key as usize,
                error
            );
        }
    }
}

#[unsafe(export_name = "static_key_enable")]
pub unsafe extern "C" fn linux_static_key_enable(key: *mut c_void) {
    let Ok(counter) = key_counter(key as usize) else {
        return;
    };
    if counter.load(Ordering::Acquire) == 0 {
        let _ = unsafe { linux_static_key_slow_inc(key) };
    }
}

#[unsafe(export_name = "static_key_disable")]
pub unsafe extern "C" fn linux_static_key_disable(key: *mut c_void) {
    let Ok(counter) = key_counter(key as usize) else {
        return;
    };
    if counter.load(Ordering::Acquire) == 1 {
        unsafe { linux_static_key_slow_dec(key) };
    }
}

/// `arch_jump_entry_size` analogue — given the bytes at the patch site,
/// return whether it is a 2-byte or 5-byte slot. A real implementation
/// would decode via the `insn` module; this is the byte-shape mirror.
pub fn arch_jump_entry_size(bytes: &[u8]) -> Result<usize, i32> {
    if bytes.len() < 2 {
        return Err(EINVAL);
    }
    // JMP8 sites are 2 bytes; everything else is treated as the 5-byte
    // slot. Linux BUG()s on neither; we return EINVAL.
    let len5_ok = bytes.len() >= 5;
    let first = bytes[0];
    if first == JMP8_INSN_OPCODE || (first == X86_NOP2[0] && bytes[1] == X86_NOP2[1]) {
        // 2-byte slot
        if first == X86_NOP2[0] && len5_ok {
            // Long-NOP 5-byte slot starts with 0x66 too; disambiguate by
            // checking byte 2.
            if bytes[1] == 0x0f {
                return Ok(JMP32_INSN_SIZE);
            }
        }
        return Ok(JMP8_INSN_SIZE);
    }
    if len5_ok {
        Ok(JMP32_INSN_SIZE)
    } else {
        Err(EINVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::RefCell;

    extern crate alloc;
    use alloc::collections::BTreeMap;

    #[derive(Default)]
    struct MemPoker {
        memory: RefCell<BTreeMap<u64, u8>>,
    }

    impl MemPoker {
        fn seed(&self, addr: u64, bytes: &[u8]) {
            let mut m = self.memory.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(addr + i as u64, *b);
            }
        }
    }

    impl TextPoke for MemPoker {
        fn poke(&self, addr: u64, bytes: &[u8]) -> Result<(), i32> {
            let mut m = self.memory.borrow_mut();
            for (i, b) in bytes.iter().enumerate() {
                m.insert(addr + i as u64, *b);
            }
            Ok(())
        }
        fn read(&self, addr: u64, len: usize) -> Result<alloc::vec::Vec<u8>, i32> {
            let m = self.memory.borrow();
            (0..len)
                .map(|i| m.get(&(addr + i as u64)).copied().ok_or(EFAULT))
                .collect()
        }
    }

    fn write_prel32(bytes: &mut [u8], offset: usize, from: usize, to: usize) {
        bytes[offset..offset + 4].copy_from_slice(&encode_prel32(from, to).unwrap().to_le_bytes());
    }

    fn write_prel64(bytes: &mut [u8], offset: usize, from: usize, to: usize) {
        let relative = to as i128 - from as i128;
        assert!((i64::MIN as i128..=i64::MAX as i128).contains(&relative));
        bytes[offset..offset + 8].copy_from_slice(&(relative as i64).to_le_bytes());
    }

    #[test]
    fn opcode_constants_match_linux() {
        assert_eq!(JMP32_INSN_OPCODE, 0xE9);
        assert_eq!(JMP32_INSN_SIZE, 5);
        assert_eq!(JMP8_INSN_OPCODE, 0xEB);
        assert_eq!(JMP8_INSN_SIZE, 2);
    }

    #[test]
    fn jump_entry_sort_preserves_prel32_across_the_canonical_alias_wrap() {
        let base = 0xffff_ffff_c100_0000usize;
        let code = base + 0x100;
        let target = 0x0000_0000_0050_0000usize;
        let key = base + 0x300;
        let mut metadata = [0u8; JUMP_ENTRY_SIZE];
        write_prel32(&mut metadata, 0, base, code);
        write_prel32(&mut metadata, 4, base + 4, target);
        write_prel64(&mut metadata, 8, base + 8, key);

        assert_eq!(sort_module_jump_entries(base, &mut metadata), Ok(1));
        let entry = decode_module_entries(base, &metadata).unwrap();
        assert_eq!(entry[0].code, code);
        assert_eq!(entry[0].target, target);
        assert_eq!(entry[0].key, key);
    }

    #[test]
    fn text_gen_insn_emits_signed_rel32() {
        let bytes = text_gen_insn(JMP32_INSN_OPCODE, JMP32_INSN_SIZE, 0x1000, 0x2000);
        assert_eq!(bytes[0], 0xE9);
        let rel = i32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        // dest - (addr + size) = 0x2000 - 0x1005 = 0xFFB
        assert_eq!(rel, 0x0FFB);
    }

    #[test]
    fn text_gen_insn_emits_signed_rel8_for_short_jump() {
        let bytes = text_gen_insn(JMP8_INSN_OPCODE, JMP8_INSN_SIZE, 0x100, 0x110);
        assert_eq!(bytes[0], 0xEB);
        assert_eq!(bytes[1] as i8, 0x0E);
    }

    #[test]
    fn jump_label_patch_jmp_produces_jmp_bytes() {
        let p = jump_label_patch(0x1000, 0x2000, 5, JumpLabelType::Jmp).unwrap();
        assert_eq!(p.size, 5);
        assert_eq!(p.code[0], JMP32_INSN_OPCODE);
    }

    #[test]
    fn jump_label_patch_nop_produces_nop_bytes() {
        let p = jump_label_patch(0x1000, 0x2000, 5, JumpLabelType::Nop).unwrap();
        assert_eq!(p.size, 5);
        let expected = x86_nop(5).unwrap();
        assert_eq!(&p.code[..], expected);
    }

    #[test]
    fn transform_writes_jmp_when_site_currently_has_nop() {
        let mem = MemPoker::default();
        let nop = x86_nop(5).unwrap();
        mem.seed(0x1000, nop);

        let r = jump_label_transform(&mem, 0x1000, 0x2000, 5, JumpLabelType::Jmp);
        assert!(r.is_ok());
        let after = mem.read(0x1000, 5).unwrap();
        assert_eq!(after[0], JMP32_INSN_OPCODE);
    }

    #[test]
    fn transform_rejects_unexpected_current_bytes() {
        let mem = MemPoker::default();
        mem.seed(0x1000, &[0xCC, 0xCC, 0xCC, 0xCC, 0xCC]);
        let r = jump_label_transform(&mem, 0x1000, 0x2000, 5, JumpLabelType::Jmp);
        assert_eq!(r, Err(EFAULT));
    }

    #[test]
    fn module_registration_and_live_key_updates_patch_the_real_site() {
        let mut site = [0u8; JMP32_INSN_SIZE];
        site.copy_from_slice(x86_nop(JMP32_INSN_SIZE).unwrap());
        let target = site.as_ptr() as usize + 0x100;
        let key = AtomicI32::new(0);
        let mut metadata = [0u8; JUMP_ENTRY_SIZE];
        let base = metadata.as_ptr() as usize;
        write_prel32(&mut metadata, 0, base, site.as_ptr() as usize);
        write_prel32(&mut metadata, 4, base + 4, target);
        write_prel64(
            &mut metadata,
            8,
            base + 8,
            &key as *const AtomicI32 as usize,
        );

        let registration = register_module_jump_entries(base, &metadata)
            .unwrap()
            .unwrap();
        assert_eq!(&site, x86_nop(JMP32_INSN_SIZE).unwrap());

        assert!(unsafe { linux_static_key_slow_inc((&key as *const AtomicI32).cast_mut().cast()) });
        let enabled = text_gen_insn(
            JMP32_INSN_OPCODE,
            JMP32_INSN_SIZE,
            site.as_ptr() as u64,
            target as u64,
        );
        assert_eq!(&site, enabled.as_slice());
        assert_eq!(key.load(Ordering::Acquire), 1);

        unsafe { linux_static_key_slow_dec((&key as *const AtomicI32).cast_mut().cast()) };
        assert_eq!(&site, x86_nop(JMP32_INSN_SIZE).unwrap());
        assert_eq!(key.load(Ordering::Acquire), 0);
        drop(registration);
    }

    #[test]
    fn entry_size_recognises_jmp8_vs_jmp32() {
        // 0xEB short jump → 2-byte
        assert_eq!(arch_jump_entry_size(&[0xEB, 0x10]).unwrap(), 2);
        // 0xE9 long jump → 5-byte
        assert_eq!(
            arch_jump_entry_size(&[0xE9, 0x00, 0x00, 0x00, 0x00]).unwrap(),
            5
        );
    }
}
