//! linux-parity: complete
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
use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EEXIST, EINVAL, ENOENT, ENOEXEC};
use crate::kernel::module::relocate::{Rela, RelocType, apply_rela};
use crate::kernel::module::symbols::{export_module_symbol, find_symbol, unexport_module_symbols};

// ── ELF constants ─────────────────────────────────────────────────────────────

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_REL: u16 = 1;
const EM_X86_64: u16 = 62;
const SHT_RELA: u32 = 4;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_PROGBITS: u32 = 1;
const SHT_NOBITS: u32 = 8;
const SHF_ALLOC: u64 = 1 << 1;
const STB_GLOBAL: u8 = 1;
const STB_WEAK: u8 = 2;
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
    /// Module name conflicts with an already-loaded module.
    AlreadyLoaded,
    /// Module init() returned an error.
    InitFailed(i32),
    /// Generic invalid argument.
    Invalid,
}

// ── Module descriptor ─────────────────────────────────────────────────────────

pub type ModuleInitFn = unsafe fn() -> i32;
pub type ModuleExitFn = unsafe fn();

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
                return Err(LoadModuleError::Invalid);
            }
            Ok(Self { ptr, len })
        }
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, LoadModuleError> {
        let mut section = Self::new_zeroed(bytes.len())?;
        section.as_mut_slice().copy_from_slice(bytes);
        Ok(section)
    }

    fn as_ptr(&self) -> *const u8 {
        #[cfg(test)]
        {
            self.data.as_ptr()
        }

        #[cfg(not(test))]
        {
            self.ptr as *const u8
        }
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        #[cfg(test)]
        {
            self.data.as_mut_ptr()
        }

        #[cfg(not(test))]
        {
            self.ptr
        }
    }

    fn as_slice(&self) -> &[u8] {
        #[cfg(test)]
        {
            self.data.as_slice()
        }

        #[cfg(not(test))]
        unsafe {
            core::slice::from_raw_parts(self.as_ptr(), self.len)
        }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        #[cfg(test)]
        {
            self.data.as_mut_slice()
        }

        #[cfg(not(test))]
        unsafe {
            core::slice::from_raw_parts_mut(self.as_mut_ptr(), self.len)
        }
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
    /// Loaded section data.  We keep them alive for the module's lifetime.
    pub sections: BTreeMap<String, LoadedSection>,
    /// Linux `__ksymtab` exports owned by this loaded module.
    pub exported_symbols: Vec<String>,
    /// Resolved `init_module` function pointer.
    pub init: Option<ModuleInitFn>,
    /// Resolved `cleanup_module` function pointer.
    pub exit: Option<ModuleExitFn>,
    /// Reference count (incremented by users, decremented on put).
    pub refcount: Mutex<u32>,
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
    matches!(sh.sh_type, SHT_PROGBITS | SHT_NOBITS) && (sh._flags & SHF_ALLOC) != 0
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
        self.shndx == 0
    }
    fn is_global_or_weak(&self) -> bool {
        matches!(self.bind(), _ if self.bind() == STB_GLOBAL || self.bind() == STB_WEAK)
    }
    fn is_weak(&self) -> bool {
        self.bind() == STB_WEAK
    }
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
    if elf[4] != ELFCLASS64 {
        return Err(LoadModuleError::BadElf);
    }
    if elf[5] != ELFDATA2LSB {
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

    if e_shentsize < 64 {
        return Err(LoadModuleError::BadElf);
    }

    // ── 2. Read section headers ───────────────────────────────────────────────
    let mut shdrs: Vec<Shdr> = Vec::with_capacity(e_shnum);
    for i in 0..e_shnum {
        let off = e_shoff
            .checked_add(i.checked_mul(e_shentsize).ok_or(LoadModuleError::BadElf)?)
            .ok_or(LoadModuleError::BadElf)?;
        shdrs.push(Shdr::from(elf, off).ok_or(LoadModuleError::BadElf)?);
    }

    // Section-name string table range.
    let shstrtab_sh = shdrs.get(e_shstrndx).ok_or(LoadModuleError::BadElf)?;
    let shstrtab_range = section_data_range(elf, shstrtab_sh).ok_or(LoadModuleError::BadElf)?;

    // Helper: section name.
    let sec_name = |sh: &Shdr| -> Option<&str> {
        strtab_str(
            elf,
            shstrtab_range.0,
            shstrtab_range.1,
            sh.name_idx as usize,
        )
    };

    // ── 3. Copy loaded sections into module memory ───────────────────────────
    // Linux lays out only SHF_ALLOC PROGBITS/NOBITS sections for runtime. The
    // ELF symtab/strtab remain readable from the original module bytes below.
    let mut sections: BTreeMap<String, LoadedSection> = BTreeMap::new();
    for sh in shdrs.iter() {
        if !should_load_section(sh) {
            continue;
        }
        let name = String::from(sec_name(sh).unwrap_or(""));
        if name.is_empty() {
            continue;
        }
        let data = if sh.sh_type == SHT_NOBITS {
            let size = usize::try_from(sh.size).map_err(|_| LoadModuleError::BadElf)?;
            LoadedSection::new_zeroed(size)?
        } else {
            let (s, e) = section_data_range(elf, sh).ok_or(LoadModuleError::BadElf)?;
            LoadedSection::from_bytes(&elf[s..e])?
        };
        sections.insert(name, data);
    }

    // ── 4. Determine module name from `.gnu.linkonce.this_module` or `.modinfo` ─
    let mod_name = {
        // Try to read `name=` from `.modinfo`.
        let name_from_modinfo = sections.get(".modinfo").and_then(|data| {
            let data = data.as_slice();
            // `.modinfo` is NUL-separated `key=value` pairs.
            let mut pos = 0;
            while pos < data.len() {
                let end = data[pos..]
                    .iter()
                    .position(|&b| b == 0)
                    .map(|n| pos + n)
                    .unwrap_or(data.len());
                let pair = core::str::from_utf8(&data[pos..end]).unwrap_or("");
                if let Some(v) = pair.strip_prefix("name=") {
                    return Some(v.into());
                }
                pos = end + 1;
            }
            None
        });
        name_from_modinfo.unwrap_or_else(|| String::from("unknown"))
    };

    if MODULES.lock().contains_key(&mod_name) {
        return Err(LoadModuleError::AlreadyLoaded);
    }

    // ── 5. Build symbol table: name → virtual address ─────────────────────────
    // "Virtual address" = pointer into our heap-allocated section data.
    let mut sym_addrs: BTreeMap<String, u64> = BTreeMap::new();

    // Find the SHT_SYMTAB section and its associated string table.
    let (symtab_sh, strtab_sh) = {
        let mut s = None;
        let mut t = None;
        for sh in shdrs.iter() {
            if sh.sh_type == SHT_SYMTAB {
                s = Some(sh);
            }
            if sh.sh_type == SHT_STRTAB && sec_name(sh) != Some(".shstrtab") {
                t = Some(sh);
            }
        }
        (s, t)
    };

    if let (Some(sym_sh), Some(str_sh)) = (symtab_sh, strtab_sh) {
        let (sym_data_start, sym_data_end) =
            section_data_range(elf, sym_sh).ok_or(LoadModuleError::BadElf)?;
        let (str_data_start, str_data_end) =
            section_data_range(elf, str_sh).ok_or(LoadModuleError::BadElf)?;
        let n_syms = (sym_data_end - sym_data_start) / 24; // Elf64_Sym = 24 bytes

        for i in 0..n_syms {
            let off = sym_data_start + i * 24;
            if off + 24 > sym_data_end.min(elf.len()) {
                break;
            }
            let sym = Sym::from(elf, off).ok_or(LoadModuleError::BadElf)?;
            if !sym.is_global_or_weak() || sym.is_undef() {
                continue;
            }
            let name: String = String::from(
                strtab_str(elf, str_data_start, str_data_end, sym.name_idx as usize).unwrap_or(""),
            );
            if name.is_empty() {
                continue;
            }

            // sym.value is an offset within the section referenced by shndx.
            if let Some(sec_sh) = shdrs.get(sym.shndx as usize) {
                let sec_name_str: String = String::from(sec_name(sec_sh).unwrap_or(""));
                if let Some(sec_data) = sections.get(&sec_name_str) {
                    let vaddr = sec_data.as_ptr() as u64 + sym.value;
                    sym_addrs.insert(name, vaddr);
                }
            }
        }
    }

    // ── 6. Apply RELA relocations ─────────────────────────────────────────────
    // Each SHT_RELA section targets another section (named without the `.rela` prefix).
    for i in 0..e_shnum {
        let rela_sh = &shdrs[i];
        if rela_sh.sh_type != SHT_RELA {
            continue;
        }

        let rela_name = sec_name(rela_sh).unwrap_or("");
        // Target section name is the rela section name minus the ".rela" prefix.
        let target_name = rela_name.strip_prefix(".rela").unwrap_or(rela_name);

        // We need the linked symbol table to resolve entries.
        let sym_sh = match shdrs.get(rela_sh.link as usize) {
            Some(s) if s.sh_type == SHT_SYMTAB => s,
            _ => continue,
        };
        let str_sh = match shdrs.get(sym_sh.link as usize) {
            Some(s) if s.sh_type == SHT_STRTAB => s,
            _ => continue,
        };

        let (rela_data_start, rela_data_end) =
            section_data_range(elf, rela_sh).ok_or(LoadModuleError::BadElf)?;
        let (sym_data_start, sym_data_end) =
            section_data_range(elf, sym_sh).ok_or(LoadModuleError::BadElf)?;
        let (str_data_start, str_data_end) =
            section_data_range(elf, str_sh).ok_or(LoadModuleError::BadElf)?;
        let n_relas = (rela_data_end - rela_data_start) / 24;

        // Collect relas first so we can borrow `sections` mutably below.
        let relas: Vec<(usize, RelocType, i64, String, Option<u64>, bool)> = (0..n_relas)
            .filter_map(|j| {
                let rela = Rela::from_bytes(elf, rela_data_start + j * 24)?;
                // Resolve the symbol referenced by this relocation.
                let sym_offset = (rela.sym as usize).checked_mul(24)?;
                let sym_off = sym_data_start.checked_add(sym_offset)?;
                if sym_off.checked_add(24)? > sym_data_end {
                    return None;
                }
                let sym = Sym::from(elf, sym_off)?;
                let sym_name =
                    strtab_str(elf, str_data_start, str_data_end, sym.name_idx as usize)?;
                let local_addr = symbol_section_addr(&sym, &shdrs, &sections, elf, shstrtab_range);

                Some((
                    rela.offset as usize,
                    rela.rel_type,
                    rela.addend,
                    sym_name.into(),
                    local_addr,
                    sym.is_weak(),
                ))
            })
            .collect();

        // Resolve symbol addresses.  Linux rejects unresolved strong module
        // relocations before init runs; leaving a zero call target here would
        // turn a missing ABI export into a guest crash.
        let mut resolved: Vec<(usize, RelocType, i64, u64)> = Vec::new();
        for (off, rt, addend, name, local_addr, weak) in relas {
            if let Some(addr) = local_addr {
                resolved.push((off, rt, addend, addr));
            } else if name.is_empty() {
                if weak {
                    resolved.push((off, rt, addend, 0));
                }
                continue;
            } else if let Some(&addr) = sym_addrs.get(&name) {
                resolved.push((off, rt, addend, addr));
            } else if let Some(addr) = find_symbol(&name) {
                resolved.push((off, rt, addend, addr as u64));
            } else if weak {
                resolved.push((off, rt, addend, 0));
            } else {
                return Err(LoadModuleError::UndefinedSymbol(name));
            }
        }

        if let Some(sec_data) = sections.get_mut(target_name) {
            let sec_vaddr = sec_data.as_ptr() as u64;
            for (offset, rel_type, addend, sym_addr) in resolved.iter() {
                let patch_vaddr = sec_vaddr + *offset as u64;
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

    // ── 7. Resolve init / exit ────────────────────────────────────────────────
    let init_addr = sym_addrs
        .get("init_module")
        .copied()
        .or_else(|| find_symbol("init_module").map(|a| a as u64));
    let exit_addr = sym_addrs
        .get("cleanup_module")
        .copied()
        .or_else(|| find_symbol("cleanup_module").map(|a| a as u64));

    let init: Option<ModuleInitFn> = init_addr.map(|a| unsafe { core::mem::transmute(a as usize) });
    let exit: Option<ModuleExitFn> = exit_addr.map(|a| unsafe { core::mem::transmute(a as usize) });

    // ── 8. Construct the module and insert ────────────────────────────────────
    let module_exports = module_exports_from_sections(&sections)?;
    let exported_symbols = module_exports
        .iter()
        .map(|export| export.name.clone())
        .collect::<Vec<_>>();

    let module = Arc::new(KernelModule {
        name: mod_name.clone(),
        sections,
        exported_symbols,
        init,
        exit,
        refcount: Mutex::new(0),
    });

    for export in module_exports.iter() {
        export_module_symbol(&mod_name, &export.name, export.addr, export.gpl_only);
    }
    MODULES.lock().insert(mod_name.clone(), module.clone());

    // ── 9. Call init_module() ─────────────────────────────────────────────────
    if let Some(init_fn) = module.init {
        let rc = unsafe { init_fn() };
        if rc != 0 {
            MODULES.lock().remove(&mod_name);
            unexport_module_symbols(&mod_name);
            return Err(LoadModuleError::InitFailed(rc));
        }
    }

    Ok(module)
}

/// `delete_module` — unload a module by name.
///
/// Calls `cleanup_module()` if present, then removes the module from the
/// global table.
pub fn delete_module(name: &str) -> Result<(), i32> {
    let module = MODULES.lock().remove(name).ok_or(ENOENT)?;
    unexport_module_symbols(name);
    if let Some(exit_fn) = module.exit {
        unsafe {
            exit_fn();
        }
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
            "test sections must be within the x86 module PREL32 window"
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
        let mut sections = BTreeMap::new();
        sections.insert(
            String::from(".text"),
            LoadedSection::new_zeroed(16).expect("text allocation"),
        );
        sections.insert(
            String::from("__ksymtab"),
            LoadedSection::new_zeroed(LINUX_KERNEL_SYMBOL_SIZE).expect("ksymtab allocation"),
        );
        sections.insert(
            String::from("__kflagstab"),
            LoadedSection::from_bytes(&[KSYM_FLAG_GPL_ONLY]).expect("flags allocation"),
        );
        sections.insert(
            String::from("__ksymtab_strings"),
            LoadedSection::from_bytes(b"vp_modern_avq_num\0").expect("strings allocation"),
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
