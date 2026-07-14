//! linux-parity: partial
//! linux-source: vendor/linux/lib/bug.c
//! Generic BUG/WARN lookup for vendor/Linux C modules.
//!
//! The selected x86-64 module ABI uses 16-byte, PREL32 `struct bug_entry`
//! records.  Linux links each formed module into `module_bug_list` before
//! making its text ROX, keeps it linked while module code can execute, and
//! removes it before freeing module memory.  Lupos mirrors that lifetime and
//! the embedded `mod->bug_list` links here.

use core::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};

use spin::Mutex;

use crate::{log_error, log_info, log_warn};

pub const BUG_ENTRY_SIZE: usize = 16;
pub const BUG_ENTRY_BUG_ADDR_OFFSET: usize = 0;
pub const BUG_ENTRY_FORMAT_OFFSET: usize = 4;
pub const BUG_ENTRY_FILE_OFFSET: usize = 8;
pub const BUG_ENTRY_LINE_OFFSET: usize = 12;
pub const BUG_ENTRY_FLAGS_OFFSET: usize = 14;

pub const BUGFLAG_WARNING: u16 = 1 << 0;
pub const BUGFLAG_ONCE: u16 = 1 << 1;
pub const BUGFLAG_DONE: u16 = 1 << 2;
pub const BUGFLAG_NO_CUT_HERE: u16 = 1 << 3;
pub const BUGFLAG_ARGS: u16 = 1 << 4;

const LEN_UD2: usize = 2;
const INSN_UD2: [u8; LEN_UD2] = [0x0f, 0x0b];
const MODULE_NAME_LEN: usize = 56;
const MODULE_NAME_OFFSET: usize = 24;
pub const MAX_BUG_STRING_LEN: usize = 4096;
const MAX_MODULE_BUG_TABLES: usize = 256;
const LIST_POISON2: usize = 0xdead_0000_0000_0122;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BugTrapType {
    None,
    Warn,
    Bug,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleBugFinalizeError {
    Duplicate,
    RegistryFull,
    InvalidListHead,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LinuxListHead {
    next: usize,
    prev: usize,
}

struct ModuleBugSlot {
    /// Publication word: zero is free and usize::MAX is being removed.
    module_addr: AtomicUsize,
    bug_list_addr: AtomicUsize,
    table_addr: AtomicUsize,
    num_bugs: AtomicUsize,
    readers: AtomicUsize,
}

impl ModuleBugSlot {
    const fn new() -> Self {
        Self {
            module_addr: AtomicUsize::new(0),
            bug_list_addr: AtomicUsize::new(0),
            table_addr: AtomicUsize::new(0),
            num_bugs: AtomicUsize::new(0),
            readers: AtomicUsize::new(0),
        }
    }

    fn acquire(&self) -> Option<ModuleBugReadGuard<'_>> {
        loop {
            let module_addr = self.module_addr.load(Ordering::Acquire);
            if module_addr == 0 || module_addr == usize::MAX {
                return None;
            }
            self.readers.fetch_add(1, Ordering::AcqRel);
            if self.module_addr.load(Ordering::Acquire) == module_addr {
                return Some(ModuleBugReadGuard {
                    slot: self,
                    module_addr,
                    table_addr: self.table_addr.load(Ordering::Acquire),
                    num_bugs: self.num_bugs.load(Ordering::Acquire),
                });
            }
            self.readers.fetch_sub(1, Ordering::Release);
        }
    }
}

struct ModuleBugReadGuard<'a> {
    slot: &'a ModuleBugSlot,
    module_addr: usize,
    table_addr: usize,
    num_bugs: usize,
}

impl Drop for ModuleBugReadGuard<'_> {
    fn drop(&mut self) {
        self.slot.readers.fetch_sub(1, Ordering::Release);
    }
}

struct ModuleBugList {
    /// Linux's `static LIST_HEAD(module_bug_list)`.
    head: LinuxListHead,
    /// Slot indices in Linux list_add_rcu() order (newest first).
    order: [usize; MAX_MODULE_BUG_TABLES],
    len: usize,
}

impl ModuleBugList {
    const fn new() -> Self {
        Self {
            head: LinuxListHead { next: 0, prev: 0 },
            order: [usize::MAX; MAX_MODULE_BUG_TABLES],
            len: 0,
        }
    }

    /// Rebuild the actual embedded Linux list heads from the writer-owned
    /// order array. report_bug() uses per-slot reader pins and does not take
    /// this mutation lock.
    fn rebuild_list(&mut self) {
        let head_addr = core::ptr::addr_of_mut!(self.head) as usize;
        self.head.next = self
            .order
            .first()
            .filter(|_| self.len != 0)
            .map_or(head_addr, |index| {
                MODULE_BUG_SLOTS[*index]
                    .bug_list_addr
                    .load(Ordering::Relaxed)
            });
        self.head.prev = self.len.checked_sub(1).map_or(head_addr, |position| {
            MODULE_BUG_SLOTS[self.order[position]]
                .bug_list_addr
                .load(Ordering::Relaxed)
        });

        for position in 0..self.len {
            let slot_index = self.order[position];
            let bug_list_addr = MODULE_BUG_SLOTS[slot_index]
                .bug_list_addr
                .load(Ordering::Relaxed);
            let prev = if position == 0 {
                head_addr
            } else {
                MODULE_BUG_SLOTS[self.order[position - 1]]
                    .bug_list_addr
                    .load(Ordering::Relaxed)
            };
            let next = if position + 1 == self.len {
                head_addr
            } else {
                MODULE_BUG_SLOTS[self.order[position + 1]]
                    .bug_list_addr
                    .load(Ordering::Relaxed)
            };

            // SAFETY: the loader validates the configured struct-module size
            // and alignment and retains every descriptor until cleanup has
            // returned. The writer mutex excludes another list mutation.
            unsafe {
                (bug_list_addr as *mut LinuxListHead).write(LinuxListHead { next, prev });
            }
        }
    }
}

static MODULE_BUG_SLOTS: [ModuleBugSlot; MAX_MODULE_BUG_TABLES] =
    [const { ModuleBugSlot::new() }; MAX_MODULE_BUG_TABLES];
static MODULE_BUG_LIST: Mutex<ModuleBugList> = Mutex::new(ModuleBugList::new());

fn insert_module_bug(
    module_addr: usize,
    bug_list_addr: usize,
    table_addr: usize,
    num_bugs: usize,
) -> Result<(), ModuleBugFinalizeError> {
    let irq_flags = crate::kernel::locking::local_irq_save();
    let result = {
        let mut list = MODULE_BUG_LIST.lock();
        if MODULE_BUG_SLOTS
            .iter()
            .any(|slot| slot.module_addr.load(Ordering::Acquire) == module_addr)
        {
            Err(ModuleBugFinalizeError::Duplicate)
        } else if list.len == MAX_MODULE_BUG_TABLES {
            Err(ModuleBugFinalizeError::RegistryFull)
        } else if let Some(slot_index) = MODULE_BUG_SLOTS
            .iter()
            .position(|slot| slot.module_addr.load(Ordering::Acquire) == 0)
        {
            let slot = &MODULE_BUG_SLOTS[slot_index];
            slot.bug_list_addr.store(bug_list_addr, Ordering::Relaxed);
            slot.table_addr.store(table_addr, Ordering::Relaxed);
            slot.num_bugs.store(num_bugs, Ordering::Relaxed);

            let len = list.len;
            list.order.copy_within(0..len, 1);
            list.order[0] = slot_index;
            list.len = len + 1;
            list.rebuild_list();
            slot.module_addr.store(module_addr, Ordering::Release);
            Ok(())
        } else {
            // A slot in the teardown sentinel state is not reusable until
            // its last report_bug() reader has dropped its pin.
            Err(ModuleBugFinalizeError::RegistryFull)
        }
    };
    crate::kernel::locking::local_irq_restore(irq_flags);
    result
}

fn remove_module_bug(module_addr: usize) {
    let irq_flags = crate::kernel::locking::local_irq_save();
    let removed_index = {
        let mut list = MODULE_BUG_LIST.lock();
        let slot_index = list.order[..list.len].iter().copied().find(|index| {
            MODULE_BUG_SLOTS[*index].module_addr.load(Ordering::Acquire) == module_addr
        });
        if let Some(slot_index) = slot_index {
            let slot = &MODULE_BUG_SLOTS[slot_index];
            let bug_list_addr = slot.bug_list_addr.load(Ordering::Relaxed);
            let old_next = unsafe { (*(bug_list_addr as *const LinuxListHead)).next };

            // Unpublish before removing the embedded list node. New readers
            // skip the sentinel; cleanup below waits out readers that pinned
            // the old published value.
            slot.module_addr.store(usize::MAX, Ordering::Release);
            let len = list.len;
            let position = list.order[..len]
                .iter()
                .position(|index| *index == slot_index)
                .expect("registered module BUG list slot");
            list.order.copy_within(position + 1..len, position);
            list.len = len - 1;
            list.order[len - 1] = usize::MAX;
            list.rebuild_list();

            // list_del_rcu() leaves ->next intact and poisons only ->prev.
            unsafe {
                let node = bug_list_addr as *mut LinuxListHead;
                (*node).next = old_next;
                (*node).prev = LIST_POISON2;
            }
        }
        slot_index
    };
    crate::kernel::locking::local_irq_restore(irq_flags);

    if let Some(slot_index) = removed_index {
        let slot = &MODULE_BUG_SLOTS[slot_index];
        while slot.readers.load(Ordering::Acquire) != 0 {
            core::hint::spin_loop();
        }
        slot.num_bugs.store(0, Ordering::Relaxed);
        slot.table_addr.store(0, Ordering::Relaxed);
        slot.bug_list_addr.store(0, Ordering::Relaxed);
        slot.module_addr.store(0, Ordering::Release);
    }
}

/// Owned registration returned by `module_bug_finalize()`.
///
/// It must be stored ahead of the section allocations in the module owner so
/// Rust drop order unlinks the table before releasing its backing pages.
pub struct ModuleBugRegistration {
    module_addr: usize,
    active: AtomicBool,
}

impl ModuleBugRegistration {
    pub fn cleanup(&self) {
        if !self.active.swap(false, Ordering::AcqRel) {
            return;
        }
        remove_module_bug(self.module_addr);
    }
}

impl Drop for ModuleBugRegistration {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Linux `module_bug_finalize()` for the selected module ABI.
///
/// The loader has already relocated and validated every table record before
/// calling this function. Registration happens before module W^X and before
/// the COMING transition, matching `complete_formation()`.
pub fn module_bug_finalize(
    module_addr: usize,
    bug_list_addr: usize,
    table_addr: usize,
    num_bugs: usize,
) -> Result<ModuleBugRegistration, ModuleBugFinalizeError> {
    if bug_list_addr % core::mem::align_of::<LinuxListHead>() != 0 {
        return Err(ModuleBugFinalizeError::InvalidListHead);
    }
    insert_module_bug(module_addr, bug_list_addr, table_addr, num_bugs)?;
    Ok(ModuleBugRegistration {
        module_addr,
        active: AtomicBool::new(true),
    })
}

fn relative_address(field_addr: usize, displacement: i32) -> Option<usize> {
    // PREL32 arithmetic is signed while x86-64 kernel addresses occupy the
    // upper canonical half. Preserve the address bit pattern when entering
    // signed arithmetic, as Linux's `(long)p + (long)disp` does.
    (field_addr as isize)
        .checked_add(displacement as isize)
        .map(|address| address as usize)
}

unsafe fn read_i32(address: usize) -> i32 {
    unsafe { (address as *const i32).read_unaligned() }
}

unsafe fn read_u16(address: usize) -> u16 {
    unsafe { (address as *const u16).read_unaligned() }
}

unsafe fn relative_cstr(field_addr: usize, displacement: i32) -> Option<&'static str> {
    let address = relative_address(field_addr, displacement)?;
    let mut len = 0usize;
    while len < MAX_BUG_STRING_LEN {
        if unsafe { (address as *const u8).add(len).read() } == 0 {
            let bytes = unsafe { core::slice::from_raw_parts(address as *const u8, len) };
            return core::str::from_utf8(bytes).ok();
        }
        len += 1;
    }
    None
}

unsafe fn module_name(module_addr: usize) -> &'static str {
    let address = module_addr + MODULE_NAME_OFFSET;
    let bytes = unsafe { core::slice::from_raw_parts(address as *const u8, MODULE_NAME_LEN) };
    let Some(len) = bytes.iter().position(|byte| *byte == 0) else {
        return "<unknown>";
    };
    core::str::from_utf8(&bytes[..len]).unwrap_or("<unknown>")
}

/// Linux `report_bug()` for an x86 UD2 raised by a loaded C module.
///
/// Core-kernel `__bug_table` lookup is intentionally not claimed here. A
/// missing module entry returns `None`, allowing the architecture's ordinary
/// invalid-opcode path to handle it.
pub fn report_bug(bug_addr: usize) -> BugTrapType {
    for slot in MODULE_BUG_SLOTS.iter() {
        let Some(slot) = slot.acquire() else {
            continue;
        };
        for index in 0..slot.num_bugs {
            let Some(entry_offset) = index.checked_mul(BUG_ENTRY_SIZE) else {
                return BugTrapType::None;
            };
            let Some(entry_addr) = slot.table_addr.checked_add(entry_offset) else {
                return BugTrapType::None;
            };
            let displacement = unsafe { read_i32(entry_addr + BUG_ENTRY_BUG_ADDR_OFFSET) };
            if relative_address(entry_addr + BUG_ENTRY_BUG_ADDR_OFFSET, displacement)
                != Some(bug_addr)
            {
                continue;
            }

            let flags_addr = entry_addr + BUG_ENTRY_FLAGS_OFFSET;
            let flags_atomic = unsafe { &*(flags_addr as *const AtomicU16) };
            let mut flags = flags_atomic.load(Ordering::Acquire);
            let instruction_matches = if flags & BUGFLAG_ARGS != 0 {
                let instruction = unsafe {
                    core::slice::from_raw_parts(
                        bug_addr as *const u8,
                        crate::arch::x86::kernel::static_call::WARNINSN.len(),
                    )
                };
                instruction == crate::arch::x86::kernel::static_call::WARNINSN
            } else {
                let instruction =
                    unsafe { core::slice::from_raw_parts(bug_addr as *const u8, LEN_UD2) };
                instruction == INSN_UD2
            };
            if !instruction_matches {
                return BugTrapType::None;
            }

            let warning = flags & BUGFLAG_WARNING != 0;
            if warning && flags & BUGFLAG_ONCE != 0 {
                let previous = flags_atomic.fetch_or(BUGFLAG_DONE, Ordering::AcqRel);
                if previous & BUGFLAG_DONE != 0 {
                    return BugTrapType::Warn;
                }
                flags = previous | BUGFLAG_DONE;
            }

            let format_displacement = unsafe { read_i32(entry_addr + BUG_ENTRY_FORMAT_OFFSET) };
            let format = if format_displacement == 0 {
                None
            } else {
                unsafe { relative_cstr(entry_addr + BUG_ENTRY_FORMAT_OFFSET, format_displacement) }
            };
            let file_displacement = unsafe { read_i32(entry_addr + BUG_ENTRY_FILE_OFFSET) };
            let file =
                unsafe { relative_cstr(entry_addr + BUG_ENTRY_FILE_OFFSET, file_displacement) }
                    .unwrap_or("<unknown>");
            let line = unsafe { read_u16(entry_addr + BUG_ENTRY_LINE_OFFSET) };
            let module = unsafe { module_name(slot.module_addr) };

            if flags & BUGFLAG_NO_CUT_HERE == 0 {
                log_info!("bug", "------------[ cut here ]------------");
                if let Some(format) = format.filter(|format| !format.is_empty()) {
                    // Lupos does not yet reconstruct Linux's register-backed
                    // varargs list for BUGFLAG_ARGS, so it logs the literal
                    // format string just like the non-args UD2 path.
                    log_warn!("bug", "{}", format);
                }
            }

            if warning {
                log_warn!(
                    "bug",
                    "WARNING: module {} at {}:{} address={:#018x} taint={}",
                    module,
                    file,
                    line,
                    bug_addr,
                    flags >> 8
                );
                return BugTrapType::Warn;
            }

            log_error!(
                "bug",
                "kernel BUG in module {} at {}:{} address={:#018x}!",
                module,
                file,
                line,
                bug_addr
            );
            return BugTrapType::Bug;
        }
    }
    BugTrapType::None
}
