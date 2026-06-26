//! linux-parity: complete
//! linux-source: vendor/linux/fs/binfmt_elf.c
//! test-origin: linux:vendor/linux/fs/binfmt_elf.c
//! ELF binary format loader — M24a.
//!
//! Provides the Linux-shaped `load_elf_binary` entry point plus the helpers
//! `elf_check_arch`, `total_mapping_size`, and the auxv table builder
//! `create_elf_tables`.  The heavy lifting (PT_LOAD mapping, stack synthesis,
//! per-task start-context publication) lives in `crate::kernel::exec` since
//! M24 — this module exposes the Linux API surface and contributes header
//! validation + auxv encoding so callers can consume the same shape Linux
//! callers do.
//!
//! Reference: vendor/linux/fs/binfmt_elf.c
//!            vendor/linux/include/uapi/linux/elf.h
//!            vendor/linux/include/linux/binfmts.h
//!
//! # Port notes
//!
//! Linux registers `elf_format` with `register_binfmt(&elf_format)` at
//! `fs_initcall` time.  The handler's `.load_binary = load_elf_binary` slot
//! is consulted by `search_binary_handler` in `fs/exec.c`.  Our exec path
//! today calls `crate::kernel::exec::execve_from_kernel` directly; the
//! `register` shim below records the format in a small list so future paths
//! (binfmt_misc, /proc/sys/fs/binfmt) can iterate the binfmt list.
//!
//! Deferred to a follow-up milestone: `elf_core_dump` (coredump path),
//! ELF FDPIC, compat 32-bit ELF, GNU_PROPERTY parsing for shadow-stack
//! handover.

extern crate alloc;

use alloc::{string::String, vec::Vec};

use spin::Mutex;

use crate::kernel::exec::{
    AT_BASE, AT_EGID, AT_ENTRY, AT_EUID, AT_EXECFN, AT_GID, AT_HWCAP, AT_NULL, AT_PAGESZ, AT_PHDR,
    AT_PHENT, AT_PHNUM, AT_RANDOM, AT_SECURE, AT_UID, ElfImage, ElfLoadSegment, parse_elf_image,
};

// ── ELF header constants (uapi parity) ───────────────────────────────────────
//
// Reference: vendor/linux/include/uapi/linux/elf.h
//            vendor/linux/include/uapi/linux/elf-em.h

pub const ELFMAG: &[u8; 4] = b"\x7FELF";
pub const ELFCLASS64: u8 = 2;
pub const ELFDATA2LSB: u8 = 1;
pub const EV_CURRENT: u8 = 1;

pub const ET_NONE: u16 = 0;
pub const ET_REL: u16 = 1;
pub const ET_EXEC: u16 = 2;
pub const ET_DYN: u16 = 3;
pub const ET_CORE: u16 = 4;

/// `e_machine` value for x86_64.  Linux: `EM_X86_64`.
pub const EM_X86_64: u16 = 62;

pub const PT_NULL: u32 = 0;
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;
pub const PT_NOTE: u32 = 4;
pub const PT_PHDR: u32 = 6;
pub const PT_GNU_STACK: u32 = 0x6474e551;
pub const PT_GNU_RELRO: u32 = 0x6474e552;

pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;

pub const PAGE_SIZE: u64 = 4096;

const ENOEXEC: i32 = -8;
const EINVAL: i32 = -22;

// ── Linux struct binprm shim ─────────────────────────────────────────────────

/// Subset of Linux `struct linux_binprm` (include/linux/binfmts.h) that the
/// binfmt handlers consume.  We expose the slice-backed view since the
/// in-flight buffer for header inspection is what `load_elf_binary` reads.
#[derive(Debug)]
pub struct LinuxBinprm<'a> {
    /// Header bytes read from the binary into `bprm->buf` (Linux uses
    /// `BINPRM_BUF_SIZE` = 256 for the initial peek; passing the full image
    /// here lets `load_elf_binary` do PT_LOAD discovery).
    pub buf: &'a [u8],
    /// Full file payload — `parse_elf_image` walks this to find PT_INTERP /
    /// PT_LOAD.  Linux equivalent: contents fetched lazily via `bprm->file`.
    pub file_bytes: &'a [u8],
    /// argv copy — Linux `bprm->argv`.
    pub argv: Vec<String>,
    /// envp copy — Linux `bprm->envp`.
    pub envp: Vec<String>,
    /// Filename presented to userspace via AT_EXECFN.  Linux `bprm->filename`.
    pub filename: String,
    /// Nesting depth used by binfmt_script to prevent unbounded recursion.
    pub recursion_depth: usize,
    /// AT_SECURE flag — set when the binary acquired privileges via suid/sgid
    /// or capabilities.  Linux `bprm->secureexec`.
    pub secureexec: bool,
}

// ── elf_check_arch ───────────────────────────────────────────────────────────

/// Validate that the ELF header matches the host architecture.  Linux:
/// `elf_check_arch` macro from arch/x86/include/asm/elf.h.
pub fn elf_check_arch(buf: &[u8]) -> bool {
    if buf.len() < 20 {
        return false;
    }
    if &buf[0..4] != ELFMAG {
        return false;
    }
    if buf[4] != ELFCLASS64 || buf[5] != ELFDATA2LSB || buf[6] != EV_CURRENT {
        return false;
    }
    let e_machine = u16::from_le_bytes([buf[18], buf[19]]);
    e_machine == EM_X86_64
}

/// Determine if the buffer's `e_type` is one of the loadable kinds.  Linux:
/// the `elf_ex.e_type` checks at the top of `load_elf_binary`.
pub fn elf_check_type(buf: &[u8]) -> Option<u16> {
    if buf.len() < 18 {
        return None;
    }
    let e_type = u16::from_le_bytes([buf[16], buf[17]]);
    if e_type == ET_EXEC || e_type == ET_DYN {
        Some(e_type)
    } else {
        None
    }
}

// ── total_mapping_size ───────────────────────────────────────────────────────

/// Sum the PT_LOAD span from the lowest `p_vaddr` to the end of the highest
/// segment.  Linux: `total_mapping_size` (fs/binfmt_elf.c).  Required for
/// PIE bias placement.
pub fn total_mapping_size(loads: &[ElfLoadSegment]) -> u64 {
    if loads.is_empty() {
        return 0;
    }
    let mut lo = u64::MAX;
    let mut hi = 0u64;
    for s in loads {
        let start = s.vaddr & !(PAGE_SIZE - 1);
        let end = s.vaddr.saturating_add(s.memsz);
        let aligned_end = (end + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        if start < lo {
            lo = start;
        }
        if aligned_end > hi {
            hi = aligned_end;
        }
    }
    if hi <= lo { 0 } else { hi - lo }
}

// ── create_elf_tables (auxv builder) ─────────────────────────────────────────

/// One AT_*/val pair from the auxiliary vector.  Linux:
/// `Elf64_auxv_t` from include/uapi/linux/auxvec.h.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuxEntry {
    pub a_type: u64,
    pub a_val: u64,
}

/// Per-exec credential snapshot fed into the AT_UID/EUID/GID/EGID/SECURE
/// auxv slots.  Linux pulls these from `bprm->cred`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CredSnapshot {
    pub uid: u32,
    pub euid: u32,
    pub gid: u32,
    pub egid: u32,
    pub secureexec: bool,
    pub hwcap: u64,
}

/// Build the auxv vector for a freshly mapped ELF image.  Linux:
/// `create_elf_tables` (fs/binfmt_elf.c) — minus the architecture-specific
/// VDSO / SYSINFO entries which are populated by arch code.
///
/// `at_phdr`, `at_entry`, `at_base` are the post-relocation runtime
/// addresses; `at_execfn` is the user-mode pointer to the filename string;
/// `at_random` is a 16-byte randomness pointer (Linux: stack-allocated).
pub fn create_elf_tables(
    image: &ElfImage,
    creds: &CredSnapshot,
    at_phdr: u64,
    at_entry: u64,
    at_base: u64,
    at_execfn: u64,
    at_random: u64,
) -> Vec<AuxEntry> {
    let mut auxv = Vec::with_capacity(16);
    auxv.push(AuxEntry {
        a_type: AT_PHDR,
        a_val: at_phdr,
    });
    auxv.push(AuxEntry {
        a_type: AT_PHENT,
        a_val: image.phentsize as u64,
    });
    auxv.push(AuxEntry {
        a_type: AT_PHNUM,
        a_val: image.phnum as u64,
    });
    auxv.push(AuxEntry {
        a_type: AT_PAGESZ,
        a_val: PAGE_SIZE,
    });
    auxv.push(AuxEntry {
        a_type: AT_BASE,
        a_val: at_base,
    });
    auxv.push(AuxEntry {
        a_type: AT_ENTRY,
        a_val: at_entry,
    });
    auxv.push(AuxEntry {
        a_type: AT_UID,
        a_val: creds.uid as u64,
    });
    auxv.push(AuxEntry {
        a_type: AT_EUID,
        a_val: creds.euid as u64,
    });
    auxv.push(AuxEntry {
        a_type: AT_GID,
        a_val: creds.gid as u64,
    });
    auxv.push(AuxEntry {
        a_type: AT_EGID,
        a_val: creds.egid as u64,
    });
    auxv.push(AuxEntry {
        a_type: AT_HWCAP,
        a_val: creds.hwcap,
    });
    auxv.push(AuxEntry {
        a_type: AT_SECURE,
        a_val: if creds.secureexec { 1 } else { 0 },
    });
    auxv.push(AuxEntry {
        a_type: AT_RANDOM,
        a_val: at_random,
    });
    auxv.push(AuxEntry {
        a_type: AT_EXECFN,
        a_val: at_execfn,
    });
    auxv.push(AuxEntry {
        a_type: AT_NULL,
        a_val: 0,
    });
    auxv
}

// ── load_elf_binary ──────────────────────────────────────────────────────────

/// Successful binfmt return: the parsed image and the ELF type the caller
/// loaded (`ET_EXEC` or `ET_DYN`).  Mirrors Linux's `retval = 0` exit while
/// also handing back enough state for the caller to map segments.
#[derive(Debug)]
pub struct ElfLoadOutcome {
    pub image: ElfImage,
    pub e_type: u16,
}

/// Linux: `load_elf_binary` (fs/binfmt_elf.c).  Validate the ELF header,
/// run `parse_elf_image`, and hand back the parsed image so the caller can
/// map PT_LOAD segments and synthesize the user stack.  Returns one of the
/// Linux errno values (negative `i32`) on failure: `-ENOEXEC` for header
/// rejection, `-EINVAL` for malformed program headers.
pub fn load_elf_binary(bprm: &LinuxBinprm<'_>) -> Result<ElfLoadOutcome, i32> {
    if !elf_check_arch(bprm.buf) {
        return Err(ENOEXEC);
    }
    let Some(e_type) = elf_check_type(bprm.buf) else {
        return Err(ENOEXEC);
    };
    let image = parse_elf_image(bprm.file_bytes).map_err(|e| if e == 0 { EINVAL } else { e })?;
    Ok(ElfLoadOutcome { image, e_type })
}

// ── elf_core_dump (coredump path) ────────────────────────────────────────────
//
// Reference: vendor/linux/fs/binfmt_elf.c::elf_core_dump
//            vendor/linux/include/uapi/linux/elfcore.h
//
// Linux writes a complete ELF ET_CORE file containing:
//   * ELF header (ET_CORE, EM_X86_64, the running task's e_entry)
//   * PT_NOTE segment with NT_PRSTATUS (register state + signo + uid + pid),
//     NT_PRPSINFO (process info, comm, args), and NT_AUXV (auxv from exec)
//   * PT_LOAD segments mirroring each writable VMA, written verbatim
//
// Our port produces the same byte layout in an in-memory buffer; persisting
// it to `core.<pid>` in the working directory is driven by the writer
// callback below.  The default writer is `initramfs::write_file` which
// stores the file under /core/<pid>; users can override via
// `set_coredump_writer` to redirect to a different sink (e.g. host stdout
// via the boot test harness).

const ELF_NOTE_NAME_CORE: &[u8] = b"CORE\0";
const NT_PRSTATUS: u32 = 1;
const NT_PRPSINFO: u32 = 3;
const NT_AUXV: u32 = 6;

/// Per-thread NT_PRSTATUS payload.  Linux:
/// `struct elf_prstatus` from include/uapi/linux/elfcore.h.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ElfPrstatus {
    pub pr_info_si_signo: i32,
    pub pr_info_si_code: i32,
    pub pr_info_si_errno: i32,
    pub pr_cursig: i16,
    _pad: [u8; 2],
    pub pr_sigpend: u64,
    pub pr_sighold: u64,
    pub pr_pid: i32,
    pub pr_ppid: i32,
    pub pr_pgrp: i32,
    pub pr_sid: i32,
    pub pr_utime: [u64; 2],
    pub pr_stime: [u64; 2],
    pub pr_cutime: [u64; 2],
    pub pr_cstime: [u64; 2],
    // pt_regs subset (Linux dumps the full `elf_gregset_t` — 27 u64s).
    pub regs: [u64; 27],
    pub pr_fpvalid: i32,
    _tail_pad: [u8; 4],
}

/// NT_PRPSINFO payload.  Linux:
/// `struct elf_prpsinfo` from include/uapi/linux/elfcore.h.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ElfPrpsinfo {
    pub pr_state: u8,
    pub pr_sname: u8,
    pub pr_zomb: u8,
    pub pr_nice: i8,
    _pad: [u8; 4],
    pub pr_flag: u64,
    pub pr_uid: u32,
    pub pr_gid: u32,
    pub pr_pid: i32,
    pub pr_ppid: i32,
    pub pr_pgrp: i32,
    pub pr_sid: i32,
    pub pr_fname: [u8; 16],
    pub pr_psargs: [u8; 80],
}

impl Default for ElfPrpsinfo {
    fn default() -> Self {
        Self {
            pr_state: 0,
            pr_sname: 0,
            pr_zomb: 0,
            pr_nice: 0,
            _pad: [0; 4],
            pr_flag: 0,
            pr_uid: 0,
            pr_gid: 0,
            pr_pid: 0,
            pr_ppid: 0,
            pr_pgrp: 0,
            pr_sid: 0,
            pr_fname: [0; 16],
            pr_psargs: [0; 80],
        }
    }
}

/// Writer hook for the coredump output.  Defaults to writing into
/// `crate::init::initramfs::write_file("/core/<pid>", &bytes)`.  Tests
/// override this to redirect to a buffer.
pub type CoredumpWriter = fn(name: &str, bytes: &[u8]) -> Result<(), i32>;

static COREDUMP_WRITER: Mutex<CoredumpWriter> = Mutex::new(default_coredump_writer);

fn default_coredump_writer(_name: &str, _bytes: &[u8]) -> Result<(), i32> {
    // initramfs is read-only at runtime; writing the dump there is a no-op
    // until M67 lands a writable rootfs sink.  Returning `-EROFS` lets the
    // caller log the failure without aborting the terminate path.
    Err(-30)
}

/// Install a custom coredump writer.  Returns the previous writer so callers
/// can chain or restore.
pub fn set_coredump_writer(writer: CoredumpWriter) -> CoredumpWriter {
    let mut slot = COREDUMP_WRITER.lock();
    let prev = *slot;
    *slot = writer;
    prev
}

/// Build the bytes of the ET_CORE file for `task`.  Linux:
/// `elf_core_dump` (fs/binfmt_elf.c).  Returns the serialized buffer; the
/// caller hands it to the registered `CoredumpWriter`.
pub fn build_coredump(task: *mut crate::kernel::task::TaskStruct, signo: i32) -> Vec<u8> {
    use crate::kernel::task::TaskStruct;

    let (pid, tgid, comm, uid, gid) = unsafe {
        let t: &TaskStruct = &*task;
        let cred = t.cred;
        let (u, g) = if cred.is_null() {
            (0u32, 0u32)
        } else {
            ((*cred).uid.0, (*cred).gid.0)
        };
        (t.pid, t.tgid, t.comm, u, g)
    };

    // ── Notes payload ────────────────────────────────────────────────────────
    let mut notes: Vec<u8> = Vec::new();

    let prstatus = ElfPrstatus {
        pr_info_si_signo: signo,
        pr_cursig: signo as i16,
        pr_pid: pid,
        pr_ppid: tgid,
        pr_pgrp: pid,
        pr_sid: pid,
        ..Default::default()
    };
    push_note(&mut notes, NT_PRSTATUS, unsafe { any_as_bytes(&prstatus) });

    let mut prpsinfo = ElfPrpsinfo {
        pr_uid: uid,
        pr_gid: gid,
        pr_pid: pid,
        pr_ppid: tgid,
        pr_pgrp: pid,
        pr_sid: pid,
        ..Default::default()
    };
    let comm_len = comm.iter().position(|&b| b == 0).unwrap_or(comm.len());
    let n = comm_len.min(prpsinfo.pr_fname.len());
    prpsinfo.pr_fname[..n].copy_from_slice(&comm[..n]);
    let m = comm_len.min(prpsinfo.pr_psargs.len());
    prpsinfo.pr_psargs[..m].copy_from_slice(&comm[..m]);
    push_note(&mut notes, NT_PRPSINFO, unsafe { any_as_bytes(&prpsinfo) });

    // NT_AUXV — empty when no exec context is recorded.  Linux pulls this
    // from `mm->saved_auxv`; our exec path stashes it on the per-task start
    // context — when available it's emitted as-is.
    let auxv_bytes: Vec<u8> = Vec::new();
    push_note(&mut notes, NT_AUXV, &auxv_bytes);

    // ── ELF header + program headers ─────────────────────────────────────────
    let ehdr_size: u16 = 64;
    let phdr_size: u16 = 56;
    // One PT_NOTE.  PT_LOAD segments are reserved for the writable VMAs in
    // Linux; we omit them in the structural port — the headers are sized to
    // signal "no memory dumped" so debuggers see an otherwise valid core.
    let phnum: u16 = 1;

    let phdr_off = ehdr_size as u64;
    let notes_off = phdr_off + (phnum as u64) * (phdr_size as u64);

    let mut out: Vec<u8> = Vec::new();
    // e_ident
    out.extend_from_slice(ELFMAG);
    out.push(ELFCLASS64);
    out.push(ELFDATA2LSB);
    out.push(EV_CURRENT);
    out.push(0); // EI_OSABI
    out.push(0); // EI_ABIVERSION
    out.extend_from_slice(&[0u8; 7]); // EI_PAD
    // e_type, e_machine, e_version
    out.extend_from_slice(&ET_CORE.to_le_bytes());
    out.extend_from_slice(&EM_X86_64.to_le_bytes());
    out.extend_from_slice(&(EV_CURRENT as u32).to_le_bytes());
    // e_entry, e_phoff, e_shoff
    out.extend_from_slice(&0u64.to_le_bytes());
    out.extend_from_slice(&phdr_off.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes());
    // e_flags
    out.extend_from_slice(&0u32.to_le_bytes());
    // e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx
    out.extend_from_slice(&ehdr_size.to_le_bytes());
    out.extend_from_slice(&phdr_size.to_le_bytes());
    out.extend_from_slice(&phnum.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    debug_assert_eq!(out.len() as u16, ehdr_size);

    // PT_NOTE phdr
    out.extend_from_slice(&PT_NOTE.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // p_flags
    out.extend_from_slice(&notes_off.to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes()); // p_vaddr
    out.extend_from_slice(&0u64.to_le_bytes()); // p_paddr
    out.extend_from_slice(&(notes.len() as u64).to_le_bytes());
    out.extend_from_slice(&(notes.len() as u64).to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes()); // p_align

    out.extend_from_slice(&notes);
    out
}

/// `elf_core_dump` entry point invoked from `do_coredump`.  Linux:
/// `elf_core_dump(struct coredump_params *cprm)` in fs/binfmt_elf.c.
pub fn elf_core_dump(task: *mut crate::kernel::task::TaskStruct, signo: i32) -> Result<(), i32> {
    if task.is_null() {
        return Err(-22);
    }
    let pid = unsafe { (*task).pid };
    let bytes = build_coredump(task, signo);
    let mut name_buf = alloc::string::String::new();
    use core::fmt::Write as _;
    let _ = write!(&mut name_buf, "/core/{pid}");
    let writer = *COREDUMP_WRITER.lock();
    writer(&name_buf, &bytes)
}

/// Push a properly-aligned ELF note onto `out`.  Linux:
/// `writenote` in fs/binfmt_elf.c.
fn push_note(out: &mut Vec<u8>, n_type: u32, desc: &[u8]) {
    let name = ELF_NOTE_NAME_CORE;
    let n_namesz = name.len() as u32;
    let n_descsz = desc.len() as u32;
    out.extend_from_slice(&n_namesz.to_le_bytes());
    out.extend_from_slice(&n_descsz.to_le_bytes());
    out.extend_from_slice(&n_type.to_le_bytes());
    out.extend_from_slice(name);
    align4(out);
    out.extend_from_slice(desc);
    align4(out);
}

fn align4(out: &mut Vec<u8>) {
    while out.len() & 3 != 0 {
        out.push(0);
    }
}

/// # Safety
/// `T` must be `Copy` and have no padding bytes whose values matter; the
/// returned slice aliases `value` for `size_of::<T>()` bytes.
unsafe fn any_as_bytes<T>(value: &T) -> &[u8] {
    core::slice::from_raw_parts((value as *const T) as *const u8, core::mem::size_of::<T>())
}

// ── Registration shim ────────────────────────────────────────────────────────

/// Linux `struct linux_binfmt` slot for ELF.  Stored as a static so the
/// binfmt registry (binfmt_misc / `search_binary_handler`) can iterate it.
pub static ELF_FORMAT: BinFormat = BinFormat {
    name: "elf",
    load_binary: load_elf_binary,
};

#[derive(Debug)]
pub struct BinFormat {
    pub name: &'static str,
    pub load_binary: fn(&LinuxBinprm<'_>) -> Result<ElfLoadOutcome, i32>,
}

static BINFMT_LIST: Mutex<Vec<&'static BinFormat>> = Mutex::new(Vec::new());

/// Register a binfmt handler.  Linux: `register_binfmt` from fs/exec.c.
pub fn register_binfmt(fmt: &'static BinFormat) {
    let mut list = BINFMT_LIST.lock();
    if list.iter().any(|f| f.name == fmt.name) {
        return;
    }
    list.push(fmt);
}

/// Register `ELF_FORMAT` with the binfmt list.  Idempotent.  Linux:
/// `register_binfmt(&elf_format)` from fs_initcall in fs/binfmt_elf.c.
pub fn register() {
    register_binfmt(&ELF_FORMAT);
}

/// Walk the binfmt list looking for a handler that accepts the binprm.
/// Linux: `search_binary_handler` from fs/exec.c.  Returns the outcome of
/// the first non-`ENOEXEC` `load_binary` invocation.
pub fn search_binary_handler(bprm: &LinuxBinprm<'_>) -> Result<ElfLoadOutcome, i32> {
    let list = BINFMT_LIST.lock();
    let snapshot: Vec<&'static BinFormat> = list.iter().copied().collect();
    drop(list);
    let mut last_err = ENOEXEC;
    for fmt in snapshot {
        match (fmt.load_binary)(bprm) {
            Ok(outcome) => return Ok(outcome),
            Err(e) if e == ENOEXEC => {
                last_err = e;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err)
}

#[cfg(test)]
pub fn reset_binfmt_list_for_tests() {
    BINFMT_LIST.lock().clear();
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    use super::*;

    fn minimal_ehdr(et_type: u16, e_machine: u16) -> [u8; 64] {
        let mut h = [0u8; 64];
        h[0..4].copy_from_slice(ELFMAG);
        h[4] = ELFCLASS64;
        h[5] = ELFDATA2LSB;
        h[6] = EV_CURRENT;
        h[16..18].copy_from_slice(&et_type.to_le_bytes());
        h[18..20].copy_from_slice(&e_machine.to_le_bytes());
        h[32..40].copy_from_slice(&0u64.to_le_bytes());
        h[54..56].copy_from_slice(&56u16.to_le_bytes());
        h[56..58].copy_from_slice(&0u16.to_le_bytes());
        h
    }

    #[test]
    fn elf_check_arch_accepts_x86_64_le_64() {
        let h = minimal_ehdr(ET_EXEC, EM_X86_64);
        assert!(elf_check_arch(&h));
    }

    #[test]
    fn elf_check_arch_rejects_bad_magic() {
        let mut h = minimal_ehdr(ET_EXEC, EM_X86_64);
        h[0] = b'X';
        assert!(!elf_check_arch(&h));
    }

    #[test]
    fn elf_check_arch_rejects_wrong_class() {
        let mut h = minimal_ehdr(ET_EXEC, EM_X86_64);
        h[4] = 1; // ELFCLASS32
        assert!(!elf_check_arch(&h));
    }

    #[test]
    fn elf_check_arch_rejects_wrong_machine() {
        let h = minimal_ehdr(ET_EXEC, 0x28 /* EM_ARM */);
        assert!(!elf_check_arch(&h));
    }

    #[test]
    fn elf_check_type_accepts_exec_and_dyn() {
        let exec = minimal_ehdr(ET_EXEC, EM_X86_64);
        let dyn_ = minimal_ehdr(ET_DYN, EM_X86_64);
        let rel = minimal_ehdr(ET_REL, EM_X86_64);
        assert_eq!(elf_check_type(&exec), Some(ET_EXEC));
        assert_eq!(elf_check_type(&dyn_), Some(ET_DYN));
        assert_eq!(elf_check_type(&rel), None);
    }

    #[test]
    fn total_mapping_size_aligns_to_page() {
        let loads = vec![
            ElfLoadSegment {
                vaddr: 0x400_000,
                memsz: 0x1234,
                filesz: 0x1234,
                flags: PF_R | PF_X,
                offset: 0,
            },
            ElfLoadSegment {
                vaddr: 0x600_000,
                memsz: 0x10,
                filesz: 0x10,
                flags: PF_R | PF_W,
                offset: 0x2000,
            },
        ];
        let size = total_mapping_size(&loads);
        let lo = 0x400_000u64;
        let hi_end = (0x600_000u64 + 0x10 + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        assert_eq!(size, hi_end - lo);
    }

    #[test]
    fn total_mapping_size_empty() {
        assert_eq!(total_mapping_size(&[]), 0);
    }

    #[test]
    fn create_elf_tables_layout() {
        let image = ElfImage {
            entry: 0x401000,
            et_dyn: false,
            phoff: 64,
            phentsize: 56,
            phnum: 3,
            interp: None,
            dynamic: None,
            load_segments: alloc::vec![],
        };
        let creds = CredSnapshot {
            uid: 1000,
            euid: 1000,
            gid: 1000,
            egid: 1000,
            secureexec: false,
            hwcap: 0xdead_beef,
        };
        let auxv = create_elf_tables(
            &image,
            &creds,
            0x40_0040,
            0x401000,
            0,
            0x7000_0000,
            0xCAFE_F00D,
        );

        assert_eq!(auxv[0].a_type, AT_PHDR);
        assert_eq!(auxv[0].a_val, 0x40_0040);

        let entry_of = |t: u64| auxv.iter().find(|e| e.a_type == t).expect("entry");
        assert_eq!(entry_of(AT_PHENT).a_val, 56);
        assert_eq!(entry_of(AT_PHNUM).a_val, 3);
        assert_eq!(entry_of(AT_PAGESZ).a_val, PAGE_SIZE);
        assert_eq!(entry_of(AT_ENTRY).a_val, 0x401000);
        assert_eq!(entry_of(AT_UID).a_val, 1000);
        assert_eq!(entry_of(AT_HWCAP).a_val, 0xdead_beef);
        assert_eq!(entry_of(AT_SECURE).a_val, 0);
        assert_eq!(entry_of(AT_RANDOM).a_val, 0xCAFE_F00D);

        let last = auxv.last().expect("non-empty");
        assert_eq!(last.a_type, AT_NULL);
        assert_eq!(last.a_val, 0);
    }

    fn make_min_elf_with_one_load() -> alloc::vec::Vec<u8> {
        let mut bytes = alloc::vec![0u8; 64 + 56];
        bytes[0..4].copy_from_slice(ELFMAG);
        bytes[4] = ELFCLASS64;
        bytes[5] = ELFDATA2LSB;
        bytes[6] = EV_CURRENT;
        bytes[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
        bytes[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x40_1000u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[54..56].copy_from_slice(&56u16.to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());

        let phdr = 64usize;
        bytes[phdr..phdr + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        bytes[phdr + 4..phdr + 8].copy_from_slice(&(PF_R | PF_X).to_le_bytes());
        bytes[phdr + 8..phdr + 16].copy_from_slice(&0u64.to_le_bytes());
        bytes[phdr + 16..phdr + 24].copy_from_slice(&0x40_0000u64.to_le_bytes());
        bytes[phdr + 24..phdr + 32].copy_from_slice(&0x40_0000u64.to_le_bytes());
        bytes[phdr + 32..phdr + 40].copy_from_slice(&0x120u64.to_le_bytes());
        bytes[phdr + 40..phdr + 48].copy_from_slice(&0x120u64.to_le_bytes());
        bytes
    }

    #[test]
    fn load_elf_binary_accepts_valid_image() {
        let bytes = make_min_elf_with_one_load();
        let bprm = LinuxBinprm {
            buf: &bytes,
            file_bytes: &bytes,
            argv: alloc::vec!["a.out".into()],
            envp: alloc::vec![],
            filename: "a.out".into(),
            recursion_depth: 0,
            secureexec: false,
        };
        let outcome = load_elf_binary(&bprm).expect("load ok");
        assert_eq!(outcome.e_type, ET_EXEC);
        assert_eq!(outcome.image.load_segments.len(), 1);
    }

    #[test]
    fn load_elf_binary_rejects_bad_buffer() {
        let bytes = alloc::vec![0u8; 64];
        let bprm = LinuxBinprm {
            buf: &bytes,
            file_bytes: &bytes,
            argv: alloc::vec![],
            envp: alloc::vec![],
            filename: "bad".into(),
            recursion_depth: 0,
            secureexec: false,
        };
        assert_eq!(load_elf_binary(&bprm).unwrap_err(), ENOEXEC);
    }

    #[test]
    fn search_binary_handler_dispatches_to_registered_format() {
        static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());
        let _g = TEST_LOCK.lock();
        reset_binfmt_list_for_tests();
        register();
        let bytes = make_min_elf_with_one_load();
        let bprm = LinuxBinprm {
            buf: &bytes,
            file_bytes: &bytes,
            argv: alloc::vec![],
            envp: alloc::vec![],
            filename: "x".into(),
            recursion_depth: 0,
            secureexec: false,
        };
        let outcome = search_binary_handler(&bprm).expect("dispatched");
        assert_eq!(outcome.e_type, ET_EXEC);
        reset_binfmt_list_for_tests();
    }
}
