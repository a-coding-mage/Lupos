//! linux-parity: partial
//! linux-source: vendor/linux/kernel/module
//! test-origin: linux:vendor/linux/kernel/module
//! `.ko` ELF module loader — `kernel/module/main.c`.
//!
//! Loads an ET_REL ELF into kernel memory, resolves undefined symbols
//! through the EXPORT_SYMBOL table, applies RELA relocations, and invokes
//! the module's `init_module` function.
//!
//! Linux flow: `load_module` → `layout_and_allocate` →
//!             `find_module_sections` → `apply_relocations` → call init.
//!
//! References:
//!   - `kernel/module/main.c` (load flow)
//!   - `arch/x86/kernel/module.c:219` (relocation application)

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::x86::kernel::module::{X86ModuleMetadata, module_finalize};
use crate::include::uapi::errno::{EBUSY, EEXIST, EINVAL, ENOENT, ENOEXEC};
use crate::kernel::bug::{
    BUG_ENTRY_BUG_ADDR_OFFSET, BUG_ENTRY_FILE_OFFSET, BUG_ENTRY_FLAGS_OFFSET,
    BUG_ENTRY_FORMAT_OFFSET, BUG_ENTRY_SIZE, BUGFLAG_ARGS, MAX_BUG_STRING_LEN,
    ModuleBugFinalizeError, ModuleBugRegistration, module_bug_finalize,
};
use crate::kernel::module::relocate::{Rela, RelocType, apply_rela};
use crate::kernel::module::symbols::{export_module_symbol, find_symbol, unexport_module_symbols};

// ── ELF constants ─────────────────────────────────────────────────────────────

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
#[cfg(test)]
const ELFCLASS64: u8 = 2;
#[cfg(test)]
const ELFDATA2LSB: u8 = 1;
const ET_REL: u16 = 1;
const EM_X86_64: u16 = 62;
const SHT_NULL: u32 = 0;
const SHT_RELA: u32 = 4;
const SHT_REL: u32 = 9;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_PROGBITS: u32 = 1;
const SHT_NOBITS: u32 = 8;
const SHF_WRITE: u64 = 1 << 0;
const SHF_ALLOC: u64 = 1 << 1;
const SHF_EXECINSTR: u64 = 1 << 2;
const STB_GLOBAL: u8 = 1;
const STB_WEAK: u8 = 2;
const SHN_UNDEF: u16 = 0;
const SHN_LIVEPATCH: u16 = 0xFF20;
const SHN_ABS: u16 = 0xFFF1;
const SHN_COMMON: u16 = 0xFFF2;
const LINUX_STRUCT_MODULE_SIZE: u64 = 1088;
const LINUX_STRUCT_MODULE_STATE_OFFSET: usize = 0;
const LINUX_STRUCT_MODULE_LIST_OFFSET: usize = 8;
const LINUX_STRUCT_MODULE_NAME_OFFSET: usize = 24;
const LINUX_MODULE_NAME_LEN: usize = 56;
const LINUX_STRUCT_MODULE_SYMS_OFFSET: usize = 216;
const LINUX_STRUCT_MODULE_FLAGSTAB_OFFSET: usize = 232;
const LINUX_STRUCT_MODULE_NUM_SYMS_OFFSET: usize = 240;
const LINUX_STRUCT_MODULE_KP_OFFSET: usize = 272;
const LINUX_STRUCT_MODULE_NUM_KP_OFFSET: usize = 280;
const LINUX_STRUCT_MODULE_NUM_BUGS_OFFSET: usize = 832;
const LINUX_STRUCT_MODULE_BUG_LIST_OFFSET: usize = 840;
const LINUX_STRUCT_MODULE_BUG_TABLE_OFFSET: usize = 856;
const LINUX_STRUCT_MODULE_SOURCE_LIST_OFFSET: usize = 984;
const LINUX_STRUCT_MODULE_TARGET_LIST_OFFSET: usize = 1000;
const LINUX_STRUCT_MODULE_REFCNT_OFFSET: usize = 1024;
const LINUX_KERNEL_PARAM_SIZE: usize = 40;
const LINUX_MODULE_REF_BASE: u32 = 1;
const LUPOS_MODULE_VERMAGIC: &str =
    concat!(env!("CARGO_PKG_VERSION"), "-lupos SMP preempt mod_unload ");
const LINUX_KERNEL_SYMBOL_SIZE: usize = 12;
const KSYM_FLAG_GPL_ONLY: u8 = 1 << 0;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum LoadModuleError {
    /// Not a valid ELF for the host target.
    BadElf,
    /// A required symbol is not in the EXPORT_SYMBOL table.
    UndefinedSymbol(String),
    /// An unsupported relocation type was encountered.
    UnsupportedReloc,
    /// A non-empty Linux module section needs lifecycle/finalization support
    /// that Lupos does not yet implement.
    UnsupportedSection(String),
    /// Module name conflicts with an already-loaded module.
    AlreadyLoaded,
    /// Module init() returned an error.
    InitFailed(i32),
    /// Module memory allocation failed.
    OutOfMemory,
    /// Generic invalid argument.
    Invalid,
}

// ── Module descriptor ─────────────────────────────────────────────────────────

pub type ModuleInitFn = unsafe extern "C" fn() -> i32;
pub type ModuleExitFn = unsafe extern "C" fn();

/// Values of `enum module_state` from `vendor/linux/include/linux/module.h`.
///
/// The value is stored in the relocated C `struct module`, not duplicated in
/// the Rust owner.  `THIS_MODULE`, module users and the loader therefore all
/// observe the same state word, as they do in Linux.
#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleState {
    Live = 0,
    Coming = 1,
    Going = 2,
    Unformed = 3,
}

/// Loaded ELF section memory.
///
/// Linux allocates module sections through `module_memory_alloc()` and
/// architecture execmem ranges (`vendor/linux/kernel/module/main.c`,
/// `vendor/linux/arch/x86/mm/init.c`). Runtime Lupos follows that contract so
/// x86 module relocations see addresses inside `MODULES_VADDR..MODULES_END`;
/// host unit tests keep ordinary `Vec` storage because they do not install
/// kernel page tables.
pub struct LoadedSection {
    #[cfg(test)]
    data: Vec<u8>,
    #[cfg(test)]
    borrowed: *mut u8,
    #[cfg(not(test))]
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for LoadedSection {}
unsafe impl Sync for LoadedSection {}

impl LoadedSection {
    fn new_zeroed(len: usize) -> Result<Self, LoadModuleError> {
        #[cfg(test)]
        {
            Ok(Self {
                data: alloc::vec![0u8; len],
                borrowed: core::ptr::null_mut(),
                len,
            })
        }

        #[cfg(not(test))]
        {
            if len == 0 {
                return Ok(Self {
                    ptr: NonNull::<u8>::dangling().as_ptr(),
                    len,
                });
            }
            let ptr = crate::arch::x86::mm::init::execmem_alloc_rw(len);
            if ptr.is_null() {
                return Err(LoadModuleError::OutOfMemory);
            }
            Ok(Self { ptr, len })
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, LoadModuleError> {
        let mut section = Self::new_zeroed(bytes.len())?;
        section.as_mut_slice().copy_from_slice(bytes);
        Ok(section)
    }

    /// Build several host-test sections inside one retained allocation.
    ///
    /// A Linux `kernel_symbol` uses signed PREL32 offsets, while independent
    /// glibc allocations can land in arenas more than 2 GiB apart.  Runtime
    /// sections use the contiguous x86 module window; this helper lets the
    /// corresponding host test model that invariant deterministically.
    #[cfg(test)]
    fn borrowed_for_prel32_test(bytes: &mut [u8]) -> Self {
        Self {
            data: Vec::new(),
            borrowed: bytes.as_mut_ptr(),
            len: bytes.len(),
        }
    }

    fn as_ptr(&self) -> *const u8 {
        #[cfg(test)]
        {
            if self.borrowed.is_null() {
                self.data.as_ptr()
            } else {
                self.borrowed
            }
        }

        #[cfg(not(test))]
        {
            self.ptr as *const u8
        }
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        #[cfg(test)]
        {
            if self.borrowed.is_null() {
                self.data.as_mut_ptr()
            } else {
                self.borrowed
            }
        }

        #[cfg(not(test))]
        {
            self.ptr
        }
    }

    fn as_slice(&self) -> &[u8] {
        #[cfg(test)]
        {
            if self.borrowed.is_null() {
                self.data.as_slice()
            } else {
                // SAFETY: the test-only constructor receives a live mutable
                // slice, and its caller retains that backing allocation until
                // after every section view is dropped.
                unsafe { core::slice::from_raw_parts(self.borrowed, self.len) }
            }
        }

        #[cfg(not(test))]
        unsafe {
            core::slice::from_raw_parts(self.as_ptr(), self.len)
        }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        #[cfg(test)]
        {
            if self.borrowed.is_null() {
                self.data.as_mut_slice()
            } else {
                // SAFETY: the PREL32 test creates disjoint section views into
                // its retained backing allocation.
                unsafe { core::slice::from_raw_parts_mut(self.borrowed, self.len) }
            }
        }

        #[cfg(not(test))]
        unsafe {
            core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len)
        }
    }

    fn set_final_permissions(&self, flags: u64) -> Result<(), LoadModuleError> {
        if self.len == 0 {
            return Ok(());
        }
        let writable = flags & SHF_WRITE != 0;
        let executable = flags & SHF_EXECINSTR != 0;
        if writable && executable {
            return Err(LoadModuleError::UnsupportedSection(String::from(
                "writable executable module section",
            )));
        }
        crate::arch::x86::mm::init::execmem_set_final_permissions(
            self.as_ptr().cast_mut(),
            self.len,
            writable,
            executable,
        )
        .map_err(|_| LoadModuleError::UnsupportedSection(String::from("module W^X")))
    }
}

#[cfg(not(test))]
impl Drop for LoadedSection {
    fn drop(&mut self) {
        if self.len != 0 {
            crate::arch::x86::mm::init::execmem_free(self.ptr);
        }
    }
}

/// Runtime descriptor for a loaded module.
/// Mirrors `struct module` at `include/linux/module.h:397`.
pub struct KernelModule {
    pub name: String,
    /// Address of the relocated C `__this_module` object.
    ///
    /// Linux uses this object as the unique module handle.  Keeping the real
    /// address here is required because vendor drivers pass `THIS_MODULE`
    /// through their public ABI.
    this_module_addr: usize,
    /// x86 finalization state. This precedes `sections` so Linux's
    /// `module_arch_cleanup()` equivalent runs while its section memory is
    /// still retained.
    _arch_metadata: X86ModuleMetadata,
    /// Linux `module_bug_list` membership. This field precedes `sections` so
    /// drop cleanup unlinks the BUG table before its backing memory is freed.
    bug_registration: ModuleBugRegistration,
    /// Loaded section data.  We keep them alive for the module's lifetime.
    pub sections: BTreeMap<String, LoadedSection>,
    /// Linux `__ksymtab` exports owned by this loaded module.
    pub exported_symbols: Vec<String>,
    /// Configured offsets of `mod->init` and `mod->exit`, recovered from the
    /// modpost relocations rather than assumed from one C layout.
    init_field_offset: Option<usize>,
    exit_field_offset: Option<usize>,
}

impl KernelModule {
    /// Return the vendor/Linux `struct module *` identity.
    pub fn this_module_addr(&self) -> usize {
        self.this_module_addr
    }

    /// Read `mod->state` from the embedded Linux descriptor.
    pub fn state(&self) -> ModuleState {
        // SAFETY: `this_module_addr` points at the retained, validated
        // `.gnu.linkonce.this_module` allocation owned by `self.sections`.
        unsafe { read_module_state(self.this_module_addr) }
    }

    /// Mirror Linux's stores to `mod->state` during formation and teardown.
    fn set_state(&self, state: ModuleState) {
        // SAFETY: module state transitions are serialized by the module
        // registry and the descriptor remains allocated for this Arc.
        unsafe { write_module_state(self.this_module_addr, state) }
    }

    fn module_function_addr(&self, field_offset: Option<usize>) -> Option<usize> {
        let field_offset = field_offset?;
        let this_module = self.sections.get(".gnu.linkonce.this_module")?;
        let addr = read_u64(this_module.as_slice(), field_offset)?;
        usize::try_from(addr).ok().filter(|addr| *addr != 0)
    }

    /// Read the current `mod->init` value from the embedded C descriptor.
    pub fn init(&self) -> Option<ModuleInitFn> {
        self.module_function_addr(self.init_field_offset)
            .map(|addr| unsafe { core::mem::transmute(addr) })
    }

    /// Read the current `mod->exit` value from the embedded C descriptor.
    pub fn exit(&self) -> Option<ModuleExitFn> {
        self.module_function_addr(self.exit_field_offset)
            .map(|addr| unsafe { core::mem::transmute(addr) })
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ModuleExport {
    name: String,
    addr: usize,
    gpl_only: bool,
}

// ── Global module table ───────────────────────────────────────────────────────

lazy_static! {
    static ref MODULES: Mutex<BTreeMap<String, Arc<KernelModule>>> = Mutex::new(BTreeMap::new());
    /// Names are reserved from `MODULE_STATE_UNFORMED` until the module's
    /// descriptor is finally freed.  Linux does the same by linking the
    /// unformed module into its module list before relocation.
    static ref MODULE_IDENTITIES: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());
}

unsafe fn read_module_state(this_module_addr: usize) -> ModuleState {
    let state_ptr = (this_module_addr + LINUX_STRUCT_MODULE_STATE_OFFSET) as *const AtomicU32;
    // SAFETY: callers guarantee a live, size-validated `struct module`.
    match unsafe { &*state_ptr }.load(Ordering::Acquire) {
        0 => ModuleState::Live,
        1 => ModuleState::Coming,
        2 => ModuleState::Going,
        3 => ModuleState::Unformed,
        // Only the four values above exist in Linux.  The input section is
        // zero-initialized by modpost and every later write is ours.
        _ => ModuleState::Unformed,
    }
}

unsafe fn write_module_state(this_module_addr: usize, state: ModuleState) {
    let state_ptr = (this_module_addr + LINUX_STRUCT_MODULE_STATE_OFFSET) as *const AtomicU32;
    // SAFETY: callers guarantee a live, writable `struct module` allocation.
    unsafe { &*state_ptr }.store(state as u32, Ordering::Release);
}

unsafe fn write_module_usize(this_module_addr: usize, offset: usize, value: usize) {
    let field = (this_module_addr + offset) as *mut usize;
    unsafe { field.write(value) };
}

unsafe fn write_module_u32(this_module_addr: usize, offset: usize, value: u32) {
    let field = (this_module_addr + offset) as *mut u32;
    unsafe { field.write(value) };
}

unsafe fn initialize_module_list_head(this_module_addr: usize, offset: usize) {
    let head = this_module_addr + offset;
    unsafe {
        write_module_usize(this_module_addr, offset, head);
        write_module_usize(
            this_module_addr,
            offset + core::mem::size_of::<usize>(),
            head,
        );
    }
}

fn loaded_section_pointer(sections: &BTreeMap<String, LoadedSection>, name: &str) -> Option<usize> {
    sections
        .get(name)
        .filter(|section| section.len != 0)
        .map(|section| section.as_ptr() as usize)
}

/// Populate the configured fields which Linux's `find_module_sections()` and
/// `module_unload_init()` install in the relocated `struct module` before any
/// module code observes `THIS_MODULE`.
fn initialize_embedded_module(
    this_module_addr: usize,
    sections: &BTreeMap<String, LoadedSection>,
) -> Result<(), LoadModuleError> {
    let syms = sections.get("__ksymtab");
    let num_syms = syms.map_or(0, |section| section.len / LINUX_KERNEL_SYMBOL_SIZE);
    let params = sections.get("__param");
    let num_params = params.map_or(0, |section| section.len / LINUX_KERNEL_PARAM_SIZE);
    let num_syms = u32::try_from(num_syms).map_err(|_| LoadModuleError::BadElf)?;
    let num_params = u32::try_from(num_params).map_err(|_| LoadModuleError::BadElf)?;

    unsafe {
        initialize_module_list_head(this_module_addr, LINUX_STRUCT_MODULE_LIST_OFFSET);
        write_module_usize(
            this_module_addr,
            LINUX_STRUCT_MODULE_SYMS_OFFSET,
            loaded_section_pointer(sections, "__ksymtab").unwrap_or(0),
        );
        write_module_usize(
            this_module_addr,
            LINUX_STRUCT_MODULE_FLAGSTAB_OFFSET,
            loaded_section_pointer(sections, "__kflagstab").unwrap_or(0),
        );
        write_module_u32(
            this_module_addr,
            LINUX_STRUCT_MODULE_NUM_SYMS_OFFSET,
            num_syms,
        );
        write_module_usize(
            this_module_addr,
            LINUX_STRUCT_MODULE_KP_OFFSET,
            loaded_section_pointer(sections, "__param").unwrap_or(0),
        );
        write_module_u32(
            this_module_addr,
            LINUX_STRUCT_MODULE_NUM_KP_OFFSET,
            num_params,
        );
        write_module_u32(this_module_addr, LINUX_STRUCT_MODULE_NUM_BUGS_OFFSET, 0);
        initialize_module_list_head(this_module_addr, LINUX_STRUCT_MODULE_BUG_LIST_OFFSET);
        write_module_usize(this_module_addr, LINUX_STRUCT_MODULE_BUG_TABLE_OFFSET, 0);
        initialize_module_list_head(this_module_addr, LINUX_STRUCT_MODULE_SOURCE_LIST_OFFSET);
        initialize_module_list_head(this_module_addr, LINUX_STRUCT_MODULE_TARGET_LIST_OFFSET);
        write_module_u32(
            this_module_addr,
            LINUX_STRUCT_MODULE_REFCNT_OFFSET,
            LINUX_MODULE_REF_BASE,
        );
    }
    Ok(())
}

/// RAII equivalent of Linux's unformed-module list membership.  Every error
/// after allocation must release the name, while a successful load keeps the
/// identity reserved until `delete_module()` completes.
struct ModuleIdentityReservation {
    name: String,
    this_module_addr: usize,
    keep: bool,
}

impl ModuleIdentityReservation {
    fn reserve(name: &str, this_module_addr: usize) -> Result<Self, LoadModuleError> {
        let mut identities = MODULE_IDENTITIES.lock();
        if let Some(existing_addr) = identities.get(name).copied() {
            // SAFETY: entries remain registered exactly while their embedded
            // descriptor allocation is alive.
            return match unsafe { read_module_state(existing_addr) } {
                ModuleState::Live => Err(LoadModuleError::AlreadyLoaded),
                ModuleState::Coming | ModuleState::Going | ModuleState::Unformed => {
                    // The public load error preserves Linux's negative errno
                    // result for a name which is not yet loadable again.
                    Err(LoadModuleError::InitFailed(-EBUSY))
                }
            };
        }
        identities.insert(String::from(name), this_module_addr);
        Ok(Self {
            name: String::from(name),
            this_module_addr,
            keep: false,
        })
    }

    fn keep_until_unload(&mut self) {
        self.keep = true;
    }

    fn release_before_descriptor(&mut self) {
        let mut identities = MODULE_IDENTITIES.lock();
        if identities.get(&self.name).copied() == Some(self.this_module_addr) {
            identities.remove(&self.name);
        }
        // The entry is gone; prevent Drop from doing a second lookup after
        // the Arc which owns the descriptor has been released.
        self.keep = true;
    }
}

impl Drop for ModuleIdentityReservation {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        let mut identities = MODULE_IDENTITIES.lock();
        if identities.get(&self.name).copied() == Some(self.this_module_addr) {
            identities.remove(&self.name);
        }
    }
}

pub fn inserted_modules() -> Vec<String> {
    MODULES.lock().keys().cloned().collect()
}

pub fn find_module(name: &str) -> Option<Arc<KernelModule>> {
    MODULES.lock().get(name).cloned()
}

pub fn with_module_address<F>(addr: usize, f: F) -> bool
where
    F: FnOnce(&str, &str, usize),
{
    let modules = MODULES.lock();
    for module in modules.values() {
        for (section_name, section) in module.sections.iter() {
            let base = section.as_ptr() as usize;
            let Some(end) = base.checked_add(section.len) else {
                continue;
            };
            if (base..end).contains(&addr) {
                f(&module.name, section_name, addr - base);
                return true;
            }
        }
    }
    false
}

// ── ELF parsing helpers ───────────────────────────────────────────────────────

fn read_u8(d: &[u8], off: usize) -> Option<u8> {
    d.get(off).copied()
}
fn read_u16(d: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(off..off + 2)?.try_into().ok()?))
}
fn read_u32(d: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(off..off + 4)?.try_into().ok()?))
}
fn read_u64(d: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(off..off + 8)?.try_into().ok()?))
}
fn read_i64(d: &[u8], off: usize) -> Option<i64> {
    Some(i64::from_le_bytes(d.get(off..off + 8)?.try_into().ok()?))
}
fn read_i32(d: &[u8], off: usize) -> Option<i32> {
    Some(i32::from_le_bytes(d.get(off..off + 4)?.try_into().ok()?))
}

fn section_data_range(data: &[u8], sh: &Shdr) -> Option<(usize, usize)> {
    let start = usize::try_from(sh.offset).ok()?;
    let size = usize::try_from(sh.size).ok()?;
    let end = start.checked_add(size)?;
    if end > data.len() {
        return None;
    }
    Some((start, end))
}

fn should_load_section(sh: &Shdr) -> bool {
    sh.sh_type != SHT_NULL && (sh._flags & SHF_ALLOC) != 0
}

/// Null-terminated string from a strtab at `base + idx`.
fn strtab_str(data: &[u8], base: usize, end: usize, idx: usize) -> Option<&str> {
    if end > data.len() || base > end {
        return None;
    }
    let start = base.checked_add(idx)?;
    if start >= end {
        return None;
    }
    let nul = data[start..end].iter().position(|&b| b == 0)?;
    core::str::from_utf8(&data[start..start + nul]).ok()
}

fn modinfo_value<'a>(data: &'a [u8], sh: &Shdr, tag: &str) -> Option<&'a str> {
    let (start, end) = section_data_range(data, sh)?;
    let bytes = data.get(start..end)?;
    if bytes.last().copied() != Some(0) {
        return None;
    }
    let prefix = tag.as_bytes();
    bytes.split(|byte| *byte == 0).find_map(|entry| {
        let value = entry.strip_prefix(prefix)?.strip_prefix(b"=")?;
        core::str::from_utf8(value).ok()
    })
}

fn this_module_name<'a>(data: &'a [u8], sh: &Shdr) -> Option<&'a str> {
    let (start, end) = section_data_range(data, sh)?;
    let name_start = start.checked_add(LINUX_STRUCT_MODULE_NAME_OFFSET)?;
    let name_end = name_start.checked_add(LINUX_MODULE_NAME_LEN)?.min(end);
    let field = data.get(name_start..name_end)?;
    let nul = field.iter().position(|byte| *byte == 0)?;
    core::str::from_utf8(&field[..nul]).ok()
}

// ── Section header (Elf64_Shdr) ───────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Shdr {
    name_idx: u32,
    sh_type: u32,
    _flags: u64,
    _addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    _info: u32,
    _addralign: u64,
    _entsize: u64,
}

impl Shdr {
    fn from(d: &[u8], off: usize) -> Option<Self> {
        Some(Self {
            name_idx: read_u32(d, off)?,
            sh_type: read_u32(d, off + 4)?,
            _flags: read_u64(d, off + 8)?,
            _addr: read_u64(d, off + 16)?,
            offset: read_u64(d, off + 24)?,
            size: read_u64(d, off + 32)?,
            link: read_u32(d, off + 40)?,
            _info: read_u32(d, off + 44)?,
            _addralign: read_u64(d, off + 48)?,
            _entsize: read_u64(d, off + 56)?,
        })
    }
}

// ── Symbol table entry (Elf64_Sym) ────────────────────────────────────────────

struct Sym {
    name_idx: u32,
    info: u8,
    _other: u8,
    shndx: u16,
    value: u64,
    _size: u64,
}

impl Sym {
    fn from(d: &[u8], off: usize) -> Option<Self> {
        Some(Self {
            name_idx: read_u32(d, off)?,
            info: read_u8(d, off + 4)?,
            _other: read_u8(d, off + 5)?,
            shndx: read_u16(d, off + 6)?,
            value: read_u64(d, off + 8)?,
            _size: read_u64(d, off + 16)?,
        })
    }
    fn bind(&self) -> u8 {
        self.info >> 4
    }
    fn is_undef(&self) -> bool {
        self.shndx == SHN_UNDEF
    }
    fn is_global_or_weak(&self) -> bool {
        matches!(self.bind(), _ if self.bind() == STB_GLOBAL || self.bind() == STB_WEAK)
    }
    fn is_weak(&self) -> bool {
        self.bind() == STB_WEAK
    }
}

#[derive(Clone, Copy)]
struct SimplifiedSymbol {
    /// Linux rewrites `st_value` to the resolved runtime address before
    /// applying relocations.  Keep that rewritten value out of the immutable
    /// module bytes.
    value: u64,
    /// False when the value points into the temporary ELF image rather than
    /// retained module memory.  Linux may retain such values for kallsyms,
    /// but Lupos must not install a runtime relocation to memory that vanishes
    /// when `load_module()` returns.
    retained: bool,
}

fn symbol_section_addr(
    sym: &Sym,
    shdrs: &[Shdr],
    sections: &BTreeMap<String, LoadedSection>,
    elf: &[u8],
    shstrtab_range: (usize, usize),
) -> Option<u64> {
    if sym.is_undef() {
        return None;
    }
    let sec_sh = shdrs.get(sym.shndx as usize)?;
    let sec_name = strtab_str(
        elf,
        shstrtab_range.0,
        shstrtab_range.1,
        sec_sh.name_idx as usize,
    )?;
    let sec_data = sections.get(sec_name)?;
    Some(sec_data.as_ptr() as u64 + sym.value)
}

fn loaded_cstr_at(sections: &BTreeMap<String, LoadedSection>, addr: usize) -> Option<&str> {
    for section in sections.values() {
        let base = section.as_ptr() as usize;
        let Some(offset) = addr.checked_sub(base) else {
            continue;
        };
        if offset >= section.len {
            continue;
        }
        let data = &section.as_slice()[offset..];
        let end = data.iter().position(|&byte| byte == 0)?;
        return core::str::from_utf8(&data[..end]).ok();
    }
    None
}

fn loaded_section_at(
    sections: &BTreeMap<String, LoadedSection>,
    addr: usize,
    size: usize,
) -> Option<(&str, &LoadedSection, usize)> {
    for (name, section) in sections.iter() {
        let base = section.as_ptr() as usize;
        let Some(offset) = addr.checked_sub(base) else {
            continue;
        };
        let Some(end) = offset.checked_add(size) else {
            continue;
        };
        if end <= section.len {
            return Some((name.as_str(), section, offset));
        }
    }
    None
}

fn loaded_bounded_cstr_at(sections: &BTreeMap<String, LoadedSection>, addr: usize) -> Option<&str> {
    let (_, section, offset) = loaded_section_at(sections, addr, 1)?;
    let available = (section.len - offset).min(MAX_BUG_STRING_LEN);
    let bytes = &section.as_slice()[offset..offset + available];
    let end = bytes.iter().position(|byte| *byte == 0)?;
    core::str::from_utf8(&bytes[..end]).ok()
}

/// Validate the selected x86-64 `struct bug_entry` representation after all
/// PREL32 relocations have been applied and before `module_bug_finalize()`
/// publishes the table. This is the loader's fail-closed boundary: report_bug
/// may only dereference addresses proven to remain inside retained sections.
fn validate_module_bug_table(
    sections: &BTreeMap<String, LoadedSection>,
    section_flags: &BTreeMap<String, u64>,
) -> Result<(usize, usize), LoadModuleError> {
    let Some(table) = sections.get("__bug_table") else {
        return Ok((0, 0));
    };
    let flags = section_flags
        .get("__bug_table")
        .copied()
        .ok_or(LoadModuleError::BadElf)?;
    if table.len % BUG_ENTRY_SIZE != 0
        || flags & SHF_WRITE == 0
        || flags & SHF_EXECINSTR != 0
        || (table.as_ptr() as usize + BUG_ENTRY_FLAGS_OFFSET)
            % core::mem::align_of::<core::sync::atomic::AtomicU16>()
            != 0
    {
        return Err(LoadModuleError::BadElf);
    }

    for index in 0..table.len / BUG_ENTRY_SIZE {
        let entry_offset = index * BUG_ENTRY_SIZE;
        let bug_displacement = read_i32(table.as_slice(), entry_offset + BUG_ENTRY_BUG_ADDR_OFFSET)
            .ok_or(LoadModuleError::BadElf)?;
        let bug_addr = resolve_prel32_addr(
            table,
            entry_offset + BUG_ENTRY_BUG_ADDR_OFFSET,
            bug_displacement,
        )
        .ok_or(LoadModuleError::BadElf)?;
        let (bug_section_name, bug_section, bug_offset) =
            loaded_section_at(sections, bug_addr, 2).ok_or(LoadModuleError::BadElf)?;
        if section_flags.get(bug_section_name).copied().unwrap_or(0) & SHF_EXECINSTR == 0
            || bug_section.as_slice()[bug_offset..bug_offset + 2] != [0x0f, 0x0b]
        {
            return Err(LoadModuleError::UnsupportedSection(String::from(
                "__bug_table non-UD2 entry",
            )));
        }

        let entry_flags = read_u16(table.as_slice(), entry_offset + BUG_ENTRY_FLAGS_OFFSET)
            .ok_or(LoadModuleError::BadElf)?;
        // x86 BUGFLAG_ARGS entries use the static-call __WARN_trap path and
        // its register-backed varargs decoder rather than a UD2 exception.
        // That ABI is not installed yet, so accepting one would leave an
        // apparently formed table with no safe reporting path.
        if entry_flags & BUGFLAG_ARGS != 0 {
            return Err(LoadModuleError::UnsupportedSection(String::from(
                "__bug_table BUGFLAG_ARGS",
            )));
        }

        let format_displacement =
            read_i32(table.as_slice(), entry_offset + BUG_ENTRY_FORMAT_OFFSET)
                .ok_or(LoadModuleError::BadElf)?;
        if format_displacement != 0 {
            let format_addr = resolve_prel32_addr(
                table,
                entry_offset + BUG_ENTRY_FORMAT_OFFSET,
                format_displacement,
            )
            .ok_or(LoadModuleError::BadElf)?;
            loaded_bounded_cstr_at(sections, format_addr).ok_or(LoadModuleError::BadElf)?;
        }

        // CONFIG_DEBUG_BUGVERBOSE=y in the vendor module build, so every
        // entry contains a PREL32 source-file pointer followed by its line.
        let file_displacement = read_i32(table.as_slice(), entry_offset + BUG_ENTRY_FILE_OFFSET)
            .ok_or(LoadModuleError::BadElf)?;
        let file_addr = resolve_prel32_addr(
            table,
            entry_offset + BUG_ENTRY_FILE_OFFSET,
            file_displacement,
        )
        .ok_or(LoadModuleError::BadElf)?;
        loaded_bounded_cstr_at(sections, file_addr).ok_or(LoadModuleError::BadElf)?;
    }

    Ok((table.as_ptr() as usize, table.len / BUG_ENTRY_SIZE))
}

fn relocated_module_function(
    this_module: &LoadedSection,
    field_offset: Option<usize>,
) -> Result<Option<usize>, LoadModuleError> {
    let Some(field_offset) = field_offset else {
        return Ok(None);
    };
    let addr = read_u64(this_module.as_slice(), field_offset).ok_or(LoadModuleError::BadElf)?;
    if addr == 0 {
        return Ok(None);
    }
    usize::try_from(addr)
        .map(Some)
        .map_err(|_| LoadModuleError::BadElf)
}

fn resolve_prel32_addr(
    section: &LoadedSection,
    field_offset: usize,
    relative: i32,
) -> Option<usize> {
    let field_addr = (section.as_ptr() as usize).checked_add(field_offset)? as isize;
    field_addr
        .checked_add(relative as isize)
        .map(|addr| addr as usize)
}

fn module_exports_from_sections(
    sections: &BTreeMap<String, LoadedSection>,
) -> Result<Vec<ModuleExport>, LoadModuleError> {
    let Some(ksymtab) = sections.get("__ksymtab") else {
        return Ok(Vec::new());
    };
    if ksymtab.len % LINUX_KERNEL_SYMBOL_SIZE != 0 {
        return Err(LoadModuleError::BadElf);
    }

    let flags = sections
        .get("__kflagstab")
        .ok_or(LoadModuleError::UnsupportedReloc)?
        .as_slice();
    let count = ksymtab.len / LINUX_KERNEL_SYMBOL_SIZE;
    if flags.len() < count {
        return Err(LoadModuleError::BadElf);
    }

    let mut exports = Vec::new();
    let data = ksymtab.as_slice();
    for i in 0..count {
        let entry = i * LINUX_KERNEL_SYMBOL_SIZE;
        let value_rel = read_i32(data, entry).ok_or(LoadModuleError::BadElf)?;
        let name_rel = read_i32(data, entry + 4).ok_or(LoadModuleError::BadElf)?;

        let addr = resolve_prel32_addr(ksymtab, entry, value_rel).ok_or(LoadModuleError::BadElf)?;
        let name_addr =
            resolve_prel32_addr(ksymtab, entry + 4, name_rel).ok_or(LoadModuleError::BadElf)?;
        let name = loaded_cstr_at(sections, name_addr).ok_or(LoadModuleError::BadElf)?;
        if name.is_empty() {
            continue;
        }

        exports.push(ModuleExport {
            name: String::from(name),
            addr,
            gpl_only: (flags[i] & KSYM_FLAG_GPL_ONLY) != 0,
        });
    }

    Ok(exports)
}

// ── Main loader ───────────────────────────────────────────────────────────────

/// `load_module` — load a `.ko` ELF from a byte buffer.
///
/// 1. Validates the ELF header.
/// 2. Copies every allocatable section into heap memory.
/// 3. Builds a symbol table mapping name → virtual address.
/// 4. Applies RELA relocations.
/// 5. Resolves `init_module` / `cleanup_module`.
/// 6. Inserts the module into the global table.
/// 7. Calls `init_module()`.
pub fn load_module(elf: &[u8]) -> Result<Arc<KernelModule>, LoadModuleError> {
    // ── 1. ELF header validation ─────────────────────────────────────────────
    if elf.len() < 64 {
        return Err(LoadModuleError::BadElf);
    }
    if &elf[0..4] != &ELF_MAGIC {
        return Err(LoadModuleError::BadElf);
    }
    let e_type = read_u16(elf, 16).ok_or(LoadModuleError::BadElf)?;
    let e_mach = read_u16(elf, 18).ok_or(LoadModuleError::BadElf)?;
    if e_type != ET_REL {
        return Err(LoadModuleError::BadElf);
    }
    if e_mach != EM_X86_64 {
        return Err(LoadModuleError::BadElf);
    }

    let e_shoff = usize::try_from(read_u64(elf, 40).ok_or(LoadModuleError::BadElf)?)
        .map_err(|_| LoadModuleError::BadElf)?;
    let e_shentsize = read_u16(elf, 58).ok_or(LoadModuleError::BadElf)? as usize;
    let e_shnum = read_u16(elf, 60).ok_or(LoadModuleError::BadElf)? as usize;
    let e_shstrndx = read_u16(elf, 62).ok_or(LoadModuleError::BadElf)? as usize;

    // `elf_validity_cache_sechdrs()` accepts only the native Elf64_Shdr
    // layout.  Advancing by a larger caller-supplied stride would parse a
    // different section array than vendor Linux.
    if e_shentsize != 64 || e_shnum == 0 {
        return Err(LoadModuleError::BadElf);
    }

    let section_headers_size = e_shnum.checked_mul(64).ok_or(LoadModuleError::BadElf)?;
    let section_headers_end = e_shoff
        .checked_add(section_headers_size)
        .ok_or(LoadModuleError::BadElf)?;
    if e_shoff >= elf.len() || section_headers_end > elf.len() {
        return Err(LoadModuleError::BadElf);
    }

    // ── 2. Read section headers ───────────────────────────────────────────────
    let mut shdrs: Vec<Shdr> = Vec::with_capacity(e_shnum);
    for i in 0..e_shnum {
        let off = e_shoff
            .checked_add(i.checked_mul(64).ok_or(LoadModuleError::BadElf)?)
            .ok_or(LoadModuleError::BadElf)?;
        shdrs.push(Shdr::from(elf, off).ok_or(LoadModuleError::BadElf)?);
    }

    // Linux relies on section zero as the absent-section sentinel.
    let null_sh = &shdrs[0];
    if null_sh.sh_type != SHT_NULL || null_sh.size != 0 || null_sh._addr != 0 {
        return Err(LoadModuleError::BadElf);
    }

    // Validate every file-backed section before interpreting any of it.  As
    // in Linux, SHT_NULL and SHT_NOBITS have no file contents to validate.
    if shdrs
        .iter()
        .skip(1)
        .filter(|sh| !matches!(sh.sh_type, SHT_NULL | SHT_NOBITS))
        .any(|sh| section_data_range(elf, sh).is_none())
    {
        return Err(LoadModuleError::BadElf);
    }

    // Section-name string table range.
    if e_shstrndx == 0 {
        return Err(LoadModuleError::BadElf);
    }
    let shstrtab_sh = shdrs.get(e_shstrndx).ok_or(LoadModuleError::BadElf)?;
    let shstrtab_range = section_data_range(elf, shstrtab_sh).ok_or(LoadModuleError::BadElf)?;
    if shstrtab_range.0 == shstrtab_range.1 || elf[shstrtab_range.1 - 1] != 0 {
        return Err(LoadModuleError::BadElf);
    }

    // Helper: section name.
    let sec_name = |sh: &Shdr| -> Option<&str> {
        strtab_str(
            elf,
            shstrtab_range.0,
            shstrtab_range.1,
            sh.name_idx as usize,
        )
    };

    // Linux's ELF validity pass rejects section-name offsets that do not
    // resolve inside the section-name string table.  The finalization gate
    // below is name based, so treating an invalid name as an empty/unknown
    // section would also let malformed input bypass that gate.
    let shstrtab_size = shstrtab_range.1 - shstrtab_range.0;
    if shdrs.iter().any(|sh| {
        sh.sh_type != SHT_NULL && (sh.name_idx as usize >= shstrtab_size || sec_name(sh).is_none())
    }) {
        return Err(LoadModuleError::BadElf);
    }

    // Vendor Linux requires a unique .modinfo section (when present), a
    // unique struct-module section, and exactly one static symbol table.
    let modinfo_indices = shdrs
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(index, sh)| (sec_name(sh) == Some(".modinfo")).then_some(index))
        .collect::<Vec<_>>();
    if modinfo_indices.len() > 1 {
        return Err(LoadModuleError::BadElf);
    }
    let this_module_indices = shdrs
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(index, sh)| {
            (sec_name(sh) == Some(".gnu.linkonce.this_module")).then_some(index)
        })
        .collect::<Vec<_>>();
    if this_module_indices.len() != 1 {
        return Err(LoadModuleError::BadElf);
    }
    let this_module_sh = &shdrs[this_module_indices[0]];
    if this_module_sh.sh_type == SHT_NOBITS
        || (this_module_sh._flags & SHF_ALLOC) == 0
        || this_module_sh.size != LINUX_STRUCT_MODULE_SIZE
    {
        return Err(LoadModuleError::BadElf);
    }

    let symtab_indices = shdrs
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(index, sh)| (sh.sh_type == SHT_SYMTAB).then_some(index))
        .collect::<Vec<_>>();
    if symtab_indices.len() != 1 {
        return Err(LoadModuleError::BadElf);
    }
    let symtab_sh = &shdrs[symtab_indices[0]];
    let strtab_index = symtab_sh.link as usize;
    if strtab_index == 0 || strtab_index >= shdrs.len() {
        return Err(LoadModuleError::BadElf);
    }
    let strtab_sh = &shdrs[strtab_index];
    let (sym_data_start, sym_data_end) =
        section_data_range(elf, symtab_sh).ok_or(LoadModuleError::BadElf)?;
    let (str_data_start, str_data_end) =
        section_data_range(elf, strtab_sh).ok_or(LoadModuleError::BadElf)?;
    if str_data_start == str_data_end || elf[str_data_start] != 0 || elf[str_data_end - 1] != 0 {
        return Err(LoadModuleError::BadElf);
    }
    let n_syms = (sym_data_end - sym_data_start) / 24;
    for i in 0..n_syms {
        let sym = Sym::from(elf, sym_data_start + i * 24).ok_or(LoadModuleError::BadElf)?;
        if sym.name_idx as usize >= str_data_end - str_data_start {
            return Err(LoadModuleError::BadElf);
        }
    }

    // With CONFIG_MODULE_FORCE_LOAD=n, vendor Linux rejects a missing or
    // mismatched vermagic before allocating the module.  The selected C
    // artifacts are deliberately built with the same release and Kconfig
    // suffix reported by Lupos.
    let module_vermagic = modinfo_indices
        .first()
        .and_then(|index| modinfo_value(elf, &shdrs[*index], "vermagic"))
        .ok_or(LoadModuleError::BadElf)?;
    if module_vermagic != LUPOS_MODULE_VERMAGIC {
        return Err(LoadModuleError::BadElf);
    }
    // Linux may cache the `.modinfo` name for diagnostics while validating
    // the temporary image, but the runtime identity comes from the copied C
    // `struct module.name` field.
    let mod_name = this_module_name(elf, this_module_sh)
        .filter(|name| !name.is_empty())
        .map(String::from)
        .ok_or(LoadModuleError::BadElf)?;

    // Linux modules are not executable immediately after ELF relocation.
    // Generic and x86 finalization consumes these sections before init: it
    // patches alternatives/return and call thunks, registers exception/BUG/
    // jump-label/static-call/SRCU/trace metadata, parses parameters, and
    // installs W^X permissions.  The real `.gnu.linkonce.this_module` object
    // is handled below as the module's identity and state owner.  Silently
    // ignoring any remaining metadata can turn a successfully resolved module
    // into corrupted control flow or a fatal fault. Reject before allocating
    // module section memory or publishing anything until the corresponding
    // vendor lifecycle is implemented.
    const UNSUPPORTED_FINALIZATION_SECTIONS: &[&str] = &[
        ".altinstructions",
        ".retpoline_sites",
        ".return_sites",
        ".call_sites",
        ".ibt_endbr_seal",
        ".orc_unwind",
        ".orc_unwind_ip",
        "__jump_table",
        ".static_call_sites",
        "__ex_table",
        "___srcu_struct_ptrs",
        "__bpf_raw_tp_map",
        ".BTF",
        ".BTF.base",
        "__tracepoints_ptrs",
        "__tracepoints",
        "_ftrace_events",
        "_ftrace_eval_map",
        "__trace_printk_fmt",
        "__obsparm",
        "__patchable_function_entries",
        "__mcount_loc",
        "_error_injection_whitelist",
        ".kprobes.text",
        "_kprobe_blacklist",
        ".printk_index",
        "__dyndbg",
        "__dyndbg_classes",
        ".kunit_test_suites",
        ".kunit_init_test_suites",
        ".cfi_sites",
        ".ctors",
        ".init_array",
    ];
    for sh in shdrs.iter().filter(|sh| sh.size != 0) {
        if let Some(name) = sec_name(sh)
            && UNSUPPORTED_FINALIZATION_SECTIONS.contains(&name)
        {
            return Err(LoadModuleError::UnsupportedSection(name.into()));
        }
    }
    for sh in shdrs.iter().filter(|sh| sh.size != 0) {
        match sec_name(sh) {
            Some("__param") if sh.size % LINUX_KERNEL_PARAM_SIZE as u64 != 0 => {
                return Err(LoadModuleError::BadElf);
            }
            Some("__bug_table") if sh.size % BUG_ENTRY_SIZE as u64 != 0 => {
                return Err(LoadModuleError::BadElf);
            }
            _ => {}
        }
    }

    // ── 3. Copy loaded sections into module memory ───────────────────────────
    // Linux lays out non-null SHF_ALLOC sections for runtime. The ELF
    // symtab/strtab remain readable from the original module bytes below.
    let mut sections: BTreeMap<String, LoadedSection> = BTreeMap::new();
    let mut loaded_section_flags: BTreeMap<String, u64> = BTreeMap::new();
    for sh in shdrs.iter() {
        if !should_load_section(sh) {
            continue;
        }
        let name = String::from(sec_name(sh).unwrap_or(""));
        // `rewrite_section_headers()` clears SHF_ALLOC on metadata tracked
        // from the temporary ELF image rather than copied into module memory.
        if name.is_empty()
            || matches!(
                name.as_str(),
                ".modinfo" | "__versions" | "__version_ext_crcs" | "__version_ext_names"
            )
        {
            continue;
        }
        let data = if sh.sh_type == SHT_NOBITS {
            let size = usize::try_from(sh.size).map_err(|_| LoadModuleError::BadElf)?;
            LoadedSection::new_zeroed(size)?
        } else {
            let (s, e) = section_data_range(elf, sh).ok_or(LoadModuleError::BadElf)?;
            LoadedSection::from_bytes(&elf[s..e])?
        };
        loaded_section_flags.insert(name.clone(), sh._flags);
        sections.insert(name, data);
    }

    // ── 4. Publish the unformed Linux module identity ────────────────────────
    // `move_module()` returns the relocated copy of `__this_module`, then
    // `add_unformed_module()` sets this state and reserves the name before
    // symbol simplification and relocation.  Keep the same lifetime even
    // though the Rust ownership wrapper is constructed later.
    let this_module_addr = sections
        .get(".gnu.linkonce.this_module")
        .map(|section| section.as_ptr() as usize)
        .ok_or(LoadModuleError::BadElf)?;
    if this_module_addr % core::mem::align_of::<AtomicU32>() != 0 {
        return Err(LoadModuleError::BadElf);
    }
    initialize_embedded_module(this_module_addr, &sections)?;
    // SAFETY: section size and allocation flags were validated above and the
    // copied allocation remains owned by `sections`.
    unsafe { write_module_state(this_module_addr, ModuleState::Unformed) };
    let mut identity = ModuleIdentityReservation::reserve(&mod_name, this_module_addr)?;

    // ── 5. Simplify the ELF symbol table ─────────────────────────────────────
    let mut simplified_symbols = Vec::with_capacity(n_syms);
    for i in 0..n_syms {
        let off = sym_data_start + i * 24;
        let sym = Sym::from(elf, off).ok_or(LoadModuleError::BadElf)?;
        let name = strtab_str(elf, str_data_start, str_data_end, sym.name_idx as usize)
            .ok_or(LoadModuleError::BadElf)?;

        // `simplify_symbols()` deliberately starts at symbol 1.  Symbol zero
        // is the ELF null symbol and retains its on-disk value (normally 0).
        let simplified = if i == 0 {
            SimplifiedSymbol {
                value: sym.value,
                retained: true,
            }
        } else {
            match sym.shndx {
                SHN_UNDEF => {
                    if let Some(addr) = find_symbol(name) {
                        SimplifiedSymbol {
                            value: addr as u64,
                            retained: true,
                        }
                    } else if sym.is_weak() || name == "_GLOBAL_OFFSET_TABLE_" {
                        SimplifiedSymbol {
                            value: 0,
                            retained: true,
                        }
                    } else {
                        return Err(LoadModuleError::UndefinedSymbol(name.into()));
                    }
                }
                SHN_ABS => SimplifiedSymbol {
                    value: sym.value,
                    retained: true,
                },
                SHN_COMMON => {
                    if !name.starts_with("__gnu_lto") {
                        return Err(LoadModuleError::BadElf);
                    }
                    SimplifiedSymbol {
                        value: sym.value,
                        retained: true,
                    }
                }
                SHN_LIVEPATCH => SimplifiedSymbol {
                    value: sym.value,
                    retained: false,
                },
                section_index => {
                    let section_index = section_index as usize;
                    let sec_sh = shdrs.get(section_index).ok_or(LoadModuleError::BadElf)?;
                    let section_name = sec_name(sec_sh).ok_or(LoadModuleError::BadElf)?;
                    if let Some(sec_data) = sections.get(section_name) {
                        SimplifiedSymbol {
                            value: (sec_data.as_ptr() as u64).wrapping_add(sym.value),
                            retained: true,
                        }
                    } else {
                        // This is how rewrite_section_headers() represents a
                        // non-allocated symbol while the temporary image is
                        // alive.  A relocation may not retain this address.
                        SimplifiedSymbol {
                            value: (elf.as_ptr() as u64)
                                .wrapping_add(sec_sh.offset)
                                .wrapping_add(sym.value),
                            retained: false,
                        }
                    }
                }
            }
        };

        simplified_symbols.push(simplified);
    }

    // ── 6. Apply RELA relocations ─────────────────────────────────────────────
    // Each relocation section targets the section named by `sh_info`.
    // The init/exit field offsets are discovered from the relocations emitted
    // by modpost.  This follows the configured C layout even when
    // `__randomize_layout` or Kconfig moves fields inside `struct module`.
    let mut init_field_offset = None;
    let mut exit_field_offset = None;
    for i in 0..e_shnum {
        let rela_sh = &shdrs[i];
        if !matches!(rela_sh.sh_type, SHT_REL | SHT_RELA) {
            continue;
        }

        let Some(target_sh) = shdrs.get(rela_sh._info as usize) else {
            // `apply_relocations()` skips relocation sections with an invalid
            // sh_info target.
            continue;
        };
        let target_name = sec_name(target_sh).ok_or(LoadModuleError::BadElf)?;
        // Linux only applies relocations whose target section is allocated.
        // If Lupos did not retain that target as runtime section memory, do
        // not resolve or reject symbols referenced solely by discarded data.
        if !sections.contains_key(target_name) {
            continue;
        }
        // x86_64 enables CONFIG_MODULES_USE_ELF_RELA only. Vendor Linux's
        // generic SHT_REL hook rejects an allocated relocation target with
        // -ENOEXEC instead of silently ignoring it.
        if rela_sh.sh_type == SHT_REL {
            return Err(LoadModuleError::UnsupportedReloc);
        }

        let (rela_data_start, rela_data_end) =
            section_data_range(elf, rela_sh).ok_or(LoadModuleError::BadElf)?;
        let n_relas = (rela_data_end - rela_data_start) / 24;

        // Collect relas first so we can borrow `sections` mutably below.
        let mut relas: Vec<(usize, RelocType, i64, u64)> = Vec::with_capacity(n_relas);
        for j in 0..n_relas {
            let rela =
                Rela::from_bytes(elf, rela_data_start + j * 24).ok_or(LoadModuleError::BadElf)?;
            let original_sym = Sym::from(
                elf,
                sym_data_start
                    .checked_add(
                        (rela.sym as usize)
                            .checked_mul(24)
                            .ok_or(LoadModuleError::BadElf)?,
                    )
                    .ok_or(LoadModuleError::BadElf)?,
            )
            .ok_or(LoadModuleError::BadElf)?;
            let relocation_symbol_name = strtab_str(
                elf,
                str_data_start,
                str_data_end,
                original_sym.name_idx as usize,
            )
            .ok_or(LoadModuleError::BadElf)?;
            let sym = simplified_symbols
                .get(rela.sym as usize)
                .ok_or(LoadModuleError::BadElf)?;
            if !sym.retained {
                return Err(LoadModuleError::UnsupportedSection(target_name.into()));
            }
            if target_name == ".gnu.linkonce.this_module" {
                let field_offset =
                    usize::try_from(rela.offset).map_err(|_| LoadModuleError::BadElf)?;
                let slot = match relocation_symbol_name {
                    "init_module" => Some(&mut init_field_offset),
                    "cleanup_module" => Some(&mut exit_field_offset),
                    _ => None,
                };
                if let Some(slot) = slot {
                    if slot.replace(field_offset).is_some() {
                        return Err(LoadModuleError::BadElf);
                    }
                }
            }
            relas.push((
                usize::try_from(rela.offset).map_err(|_| LoadModuleError::BadElf)?,
                rela.rel_type,
                rela.addend,
                sym.value,
            ));
        }

        if let Some(sec_data) = sections.get_mut(target_name) {
            let sec_vaddr = sec_data.as_ptr() as u64;
            for (offset, rel_type, addend, sym_addr) in relas.iter() {
                let patch_vaddr = sec_vaddr
                    .checked_add(*offset as u64)
                    .ok_or(LoadModuleError::BadElf)?;
                apply_rela(
                    sec_data.as_mut_slice(),
                    *offset,
                    *rel_type,
                    *sym_addr,
                    patch_vaddr,
                    *addend,
                )
                .map_err(|_| LoadModuleError::UnsupportedReloc)?;
            }
        }
    }

    // `post_relocation()` invokes the x86 `module_finalize()` hook after all
    // RELA records have reached their runtime addresses. In particular,
    // `.smp_locks` is a table of relocated PREL32 offsets. Linux's
    // `alternatives_smp_module_add()` leaves those entries and their existing
    // SMP-safe lock prefixes untouched unless the whole kernel was previously
    // patched for UP execution; Lupos never enters that UP-patched state.
    let arch_section_names = shdrs
        .iter()
        .filter_map(|sh| sec_name(sh))
        .collect::<Vec<_>>();
    let arch_metadata = module_finalize(&arch_section_names);

    // ── 7. Resolve `mod->init` / `mod->exit` ──────────────────────────────────
    // Linux invokes the function pointers in the relocated struct module.  Do
    // not substitute similarly named ELF globals: malformed input may define
    // one without assigning the corresponding descriptor field.
    let this_module = sections
        .get(".gnu.linkonce.this_module")
        .ok_or(LoadModuleError::BadElf)?;
    let _validated_init = relocated_module_function(this_module, init_field_offset)?;
    let _validated_exit = relocated_module_function(this_module, exit_field_offset)?;

    // ── 8. Generic finalization, permissions, construct, and insert ─────────
    let module_exports = module_exports_from_sections(&sections)?;
    let exported_symbols = module_exports
        .iter()
        .map(|export| export.name.clone())
        .collect::<Vec<_>>();

    let (bug_table_addr, num_bugs) = validate_module_bug_table(&sections, &loaded_section_flags)?;
    let num_bugs_u32 = u32::try_from(num_bugs).map_err(|_| LoadModuleError::BadElf)?;
    // module_bug_finalize() initializes these fields immediately before
    // list_add_rcu(), rather than exposing a half-formed table earlier in the
    // generic section-discovery pass.
    unsafe {
        write_module_u32(
            this_module_addr,
            LINUX_STRUCT_MODULE_NUM_BUGS_OFFSET,
            num_bugs_u32,
        );
        write_module_usize(
            this_module_addr,
            LINUX_STRUCT_MODULE_BUG_TABLE_OFFSET,
            bug_table_addr,
        );
    }
    let bug_list_addr = this_module_addr
        .checked_add(LINUX_STRUCT_MODULE_BUG_LIST_OFFSET)
        .ok_or(LoadModuleError::BadElf)?;
    let bug_registration =
        module_bug_finalize(this_module_addr, bug_list_addr, bug_table_addr, num_bugs).map_err(
            |error| match error {
                ModuleBugFinalizeError::RegistryFull => LoadModuleError::UnsupportedSection(
                    String::from("module BUG registry capacity"),
                ),
                ModuleBugFinalizeError::Duplicate | ModuleBugFinalizeError::InvalidListHead => {
                    LoadModuleError::BadElf
                }
            },
        )?;

    // Linux performs this only after every relocation and architecture
    // finalizer has completed.  Text becomes ROX, read-only data becomes
    // RO+NX, and writable data remains RW+NX; a W+X input is never accepted.
    for sh in shdrs.iter().filter(|sh| should_load_section(sh)) {
        let name = sec_name(sh).ok_or(LoadModuleError::BadElf)?;
        if let Some(section) = sections.get(name) {
            section.set_final_permissions(sh._flags)?;
        }
    }

    let module = Arc::new(KernelModule {
        name: mod_name.clone(),
        this_module_addr,
        _arch_metadata: arch_metadata,
        bug_registration,
        sections,
        exported_symbols,
        init_field_offset,
        exit_field_offset,
    });

    // `complete_formation()` changes the embedded state before the COMING
    // notifier chain and before any module code is allowed to execute.
    module.set_state(ModuleState::Coming);
    for export in module_exports.iter() {
        export_module_symbol(&mod_name, &export.name, export.addr, export.gpl_only);
    }
    MODULES.lock().insert(mod_name.clone(), module.clone());

    // ── 9. Call init_module() ─────────────────────────────────────────────────
    if let Some(init_fn) = module.init() {
        let rc = unsafe { init_fn() };
        if rc < 0 {
            module.set_state(ModuleState::Going);
            MODULES.lock().remove(&mod_name);
            unexport_module_symbols(&mod_name);
            module.bug_registration.cleanup();
            identity.release_before_descriptor();
            // `-EEXIST` is reserved by [f]init_module for an already-loaded
            // module; Linux remaps that value when it comes from module init.
            let rc = if rc == -EEXIST { -EBUSY } else { rc };
            return Err(LoadModuleError::InitFailed(rc));
        }
        if rc > 0 {
            crate::log_warn!(
                "module",
                "{}: init suspiciously returned {}, expected 0 or a negative errno",
                mod_name,
                rc
            );
        }
    }

    // Linux worker threads may execute work queued by module_init() before
    // the caller observes a successful load.  Lupos's cooperative workqueue
    // model drains those callbacks here in task context; virtio-net uses this
    // path to publish its initial carrier state from config_work.
    #[cfg(not(test))]
    crate::kernel::workqueue::drain_system_workqueues();

    // `do_init_module()` makes the descriptor live only after init succeeds.
    module.set_state(ModuleState::Live);
    identity.keep_until_unload();
    Ok(module)
}

/// `delete_module` — unload a module by name.
///
/// Calls `cleanup_module()` if present, then removes the module from the
/// global table.
pub fn delete_module(name: &str) -> Result<(), i32> {
    // Linux leaves the module linked and its identity reserved while exit is
    // running, but switches to GOING first so new references and concurrent
    // deletion fail.
    let module = {
        let modules = MODULES.lock();
        let module = modules.get(name).cloned().ok_or(ENOENT)?;
        if module.state() != ModuleState::Live {
            return Err(EBUSY);
        }
        module.set_state(ModuleState::Going);
        module
    };

    if let Some(exit_fn) = module.exit() {
        unsafe {
            exit_fn();
        }
    }

    {
        let mut modules = MODULES.lock();
        if modules
            .get(name)
            .is_some_and(|loaded| Arc::ptr_eq(loaded, &module))
        {
            modules.remove(name);
        }
    }
    unexport_module_symbols(name);
    module.bug_registration.cleanup();
    let mut identities = MODULE_IDENTITIES.lock();
    if identities.get(name).copied() == Some(module.this_module_addr()) {
        identities.remove(name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_i32(data: &mut [u8], offset: usize, value: i32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn minimal_module_elf(section_count: u16, shstrndx: u16) -> Vec<u8> {
        let mut elf = alloc::vec![0u8; 64 + section_count as usize * 64];
        elf[0..4].copy_from_slice(&ELF_MAGIC);
        elf[4] = ELFCLASS64;
        elf[5] = ELFDATA2LSB;
        write_u16(&mut elf, 16, ET_REL);
        write_u16(&mut elf, 18, EM_X86_64);
        write_u64(&mut elf, 40, 64);
        write_u16(&mut elf, 58, 64);
        write_u16(&mut elf, 60, section_count);
        write_u16(&mut elf, 62, shstrndx);
        elf
    }

    fn write_shdr(
        data: &mut [u8],
        index: usize,
        name_idx: u32,
        sh_type: u32,
        offset: u64,
        size: u64,
    ) {
        let base = 64 + index * 64;
        write_u32(data, base, name_idx);
        write_u32(data, base + 4, sh_type);
        write_u64(data, base + 24, offset);
        write_u64(data, base + 32, size);
    }

    fn prel32(from: usize, to: usize) -> i32 {
        let delta = to as isize - from as isize;
        assert!(
            delta >= i32::MIN as isize && delta <= i32::MAX as isize,
            "test sections must be within the x86 module PREL32 window: from={from:#x} to={to:#x} delta={delta}"
        );
        delta as i32
    }

    #[test]
    fn rejects_non_elf() {
        let bad = alloc::vec![0u8; 64];
        assert!(matches!(load_module(&bad), Err(LoadModuleError::BadElf)));
    }

    #[test]
    fn rejects_too_short() {
        let tiny = alloc::vec![0u8; 10];
        assert!(matches!(load_module(&tiny), Err(LoadModuleError::BadElf)));
    }

    #[test]
    fn rejects_malformed_shstrtab_offset_without_panicking() {
        let mut elf = minimal_module_elf(2, 1);
        write_shdr(&mut elf, 0, 0, SHT_PROGBITS, 0, 0);
        write_shdr(&mut elf, 1, 0, SHT_STRTAB, 0x1000, 1);

        assert!(matches!(load_module(&elf), Err(LoadModuleError::BadElf)));
    }

    #[test]
    fn rejects_malformed_shstrtab_size_without_panicking() {
        let mut elf = minimal_module_elf(2, 1);
        write_shdr(&mut elf, 0, 1, SHT_PROGBITS, 0, 0);
        write_shdr(&mut elf, 1, 0, SHT_STRTAB, 128, 128);

        assert!(matches!(load_module(&elf), Err(LoadModuleError::BadElf)));
    }

    #[test]
    fn loader_keeps_only_allocatable_runtime_sections() {
        let runtime_text = Shdr {
            name_idx: 0,
            sh_type: SHT_PROGBITS,
            _flags: SHF_ALLOC,
            _addr: 0,
            offset: 0,
            size: 1,
            link: 0,
            _info: 0,
            _addralign: 1,
            _entsize: 0,
        };
        let bss = Shdr {
            sh_type: SHT_NOBITS,
            ..runtime_text
        };
        let debug_metadata = Shdr {
            _flags: 0,
            ..runtime_text
        };
        let symtab = Shdr {
            sh_type: SHT_SYMTAB,
            _flags: 0,
            ..runtime_text
        };

        assert!(should_load_section(&runtime_text));
        assert!(should_load_section(&bss));
        assert!(!should_load_section(&debug_metadata));
        assert!(!should_load_section(&symtab));
    }

    #[test]
    fn resolves_local_section_symbol_relocations_inside_module() {
        let mut elf = alloc::vec![0u8; 32];
        elf[1..7].copy_from_slice(b".text\0");
        let shdrs = alloc::vec![
            Shdr {
                name_idx: 0,
                sh_type: 0,
                _flags: 0,
                _addr: 0,
                offset: 0,
                size: 0,
                link: 0,
                _info: 0,
                _addralign: 0,
                _entsize: 0,
            },
            Shdr {
                name_idx: 1,
                sh_type: SHT_PROGBITS,
                _flags: 0,
                _addr: 0,
                offset: 0,
                size: 8,
                link: 0,
                _info: 0,
                _addralign: 0,
                _entsize: 0,
            },
        ];
        let mut sections = BTreeMap::new();
        sections.insert(
            String::from(".text"),
            LoadedSection::new_zeroed(8).expect("test section allocation"),
        );
        let sym = Sym {
            name_idx: 0,
            info: 0,
            _other: 0,
            shndx: 1,
            value: 4,
            _size: 0,
        };

        let addr = symbol_section_addr(&sym, &shdrs, &sections, &elf, (0, elf.len()))
            .expect("local section symbol should resolve");
        assert_eq!(addr, sections[".text"].as_ptr() as u64 + 4);
    }

    #[test]
    fn parses_linux_prel32_ksymtab_exports() {
        let mut backing = alloc::vec![0u8; 64];
        backing[28] = KSYM_FLAG_GPL_ONLY;
        backing[32..50].copy_from_slice(b"vp_modern_avq_num\0");
        let mut sections = BTreeMap::new();
        sections.insert(
            String::from(".text"),
            LoadedSection::borrowed_for_prel32_test(&mut backing[0..16]),
        );
        sections.insert(
            String::from("__ksymtab"),
            LoadedSection::borrowed_for_prel32_test(&mut backing[16..28]),
        );
        sections.insert(
            String::from("__kflagstab"),
            LoadedSection::borrowed_for_prel32_test(&mut backing[28..29]),
        );
        sections.insert(
            String::from("__ksymtab_strings"),
            LoadedSection::borrowed_for_prel32_test(&mut backing[32..50]),
        );

        let value_addr = sections[".text"].as_ptr() as usize + 4;
        let name_addr = sections["__ksymtab_strings"].as_ptr() as usize;
        let ksymtab_base = sections["__ksymtab"].as_ptr() as usize;
        let ksymtab = sections
            .get_mut("__ksymtab")
            .expect("ksymtab section")
            .as_mut_slice();
        write_i32(ksymtab, 0, prel32(ksymtab_base, value_addr));
        write_i32(ksymtab, 4, prel32(ksymtab_base + 4, name_addr));
        write_i32(ksymtab, 8, 0);

        let exports = module_exports_from_sections(&sections).expect("module exports");
        assert_eq!(
            exports,
            alloc::vec![ModuleExport {
                name: String::from("vp_modern_avq_num"),
                addr: value_addr,
                gpl_only: true,
            }]
        );
    }

    #[test]
    fn rejects_exported_symbols_without_linux_flagstab() {
        let mut sections = BTreeMap::new();
        sections.insert(
            String::from("__ksymtab"),
            LoadedSection::new_zeroed(LINUX_KERNEL_SYMBOL_SIZE).expect("ksymtab allocation"),
        );

        assert!(matches!(
            module_exports_from_sections(&sections),
            Err(LoadModuleError::UnsupportedReloc)
        ));
    }
}
