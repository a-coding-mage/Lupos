//! linux-parity: complete
//! linux-source: vendor/linux/kernel
//! test-origin: linux:vendor/linux/kernel
//! ELF binfmt + `execve` core flow (M24).
//!
//! This module implements Linux-shaped `execve` plumbing over the current
//! kernel substrate: path resolution from initramfs, ELF PT_LOAD mapping into
//! a fresh `mm_struct`, initial userspace stack/auxv synthesis, and per-task
//! start-context publication for the arch return path.

extern crate alloc;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::ffi::c_char;
use core::sync::atomic::Ordering;

use spin::Mutex;

use crate::arch::x86::mm::paging::{
    p4d_offset, pgd_none, pgd_offset_pgd, pgd_t, pmd_huge, pmd_none, pmd_offset, pte_offset_kernel,
    pte_phys, pte_present, pte_t, pte_write, pud_huge, pud_none, pud_offset,
};
use crate::include::uapi::{
    mount::MS_NOSUID,
    stat::{S_ISGID, S_ISUID},
};
use crate::kernel::{
    capability::KernelCapT,
    cred::{self, Cred, KGid, KUid},
    sched,
    task::TaskStruct,
};
use crate::mm::{
    buddy::{is_buddy_ready, page_to_pfn, with_global_buddy},
    fault::{FAULT_FLAG_USER, FAULT_FLAG_WRITE, VM_FAULT_ERROR, handle_mm_fault},
    frame::PAGE_SIZE,
    mm_types::MmStruct,
    mmap::{
        MAP_ANONYMOUS, MAP_FIXED, MAP_GROWSDOWN, MAP_PRIVATE, PROT_EXEC, PROT_READ, PROT_WRITE,
        TASK_SIZE, do_mmap,
    },
    mprotect::do_mprotect,
    page_flags::GFP_KERNEL,
    vma::find_vma,
};
use crate::security;

const ELF_MAGIC: &[u8; 4] = b"\x7FELF";
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ET_EXEC: u16 = 2;
const ET_DYN: u16 = 3;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const PF_X: u32 = 1;
const PF_W: u32 = 2;
const PF_R: u32 = 4;
const DT_NULL: i64 = 0;
const DT_RELA: i64 = 7;
const DT_RELASZ: i64 = 8;
const DT_RELAENT: i64 = 9;
const DT_RELRSZ: i64 = 35;
const DT_RELR: i64 = 36;
const DT_RELRENT: i64 = 37;
const R_X86_64_RELATIVE: u64 = 8;

const MAX_INTERP_RECURSION: usize = 4;
const MAX_ARG_STRLEN: usize = 128 * 1024;
const MAX_ARG_COUNT: usize = 4096;
const MAX_EXEC_FILE_BYTES: usize = 128 * 1024 * 1024;
const EXEC_READ_CHUNK: usize = 64 * 1024;
const STACK_SIZE: u64 = 8 * 1024 * 1024;
const PIE_LOAD_BIAS: u64 = 0x0000_5555_5555_4000;
const INTERP_LOAD_BIAS: u64 = 0x0000_7fff_0000_0000;

pub const AT_NULL: u64 = 0;
pub const AT_PHDR: u64 = 3;
pub const AT_PHENT: u64 = 4;
pub const AT_PHNUM: u64 = 5;
pub const AT_PAGESZ: u64 = 6;
pub const AT_BASE: u64 = 7;
pub const AT_ENTRY: u64 = 9;
pub const AT_UID: u64 = 11;
pub const AT_EUID: u64 = 12;
pub const AT_GID: u64 = 13;
pub const AT_EGID: u64 = 14;
pub const AT_HWCAP: u64 = 16;
pub const AT_SECURE: u64 = 23;
pub const AT_RANDOM: u64 = 25;
pub const AT_EXECFN: u64 = 31;
pub const AT_SYSINFO_EHDR: u64 = 33;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UserStartContext {
    pub ip: u64,
    pub sp: u64,
    pub rflags: u64,
    pub old_mm: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElfLoadSegment {
    pub vaddr: u64,
    pub memsz: u64,
    pub filesz: u64,
    pub flags: u32,
    pub offset: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ElfDynamicSegment {
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElfImage {
    pub entry: u64,
    pub et_dyn: bool,
    pub phoff: u64,
    pub phentsize: u16,
    pub phnum: u16,
    pub interp: Option<String>,
    pub dynamic: Option<ElfDynamicSegment>,
    pub load_segments: Vec<ElfLoadSegment>,
}

#[derive(Clone)]
pub struct ExecResolution {
    pub requested_path: String,
    pub resolved_path: String,
    pub elf: ElfImage,
    pub inode: crate::fs::types::InodeRef,
    pub dentry: crate::fs::types::DentryRef,
    pub mount: alloc::sync::Arc<crate::fs::mount::Mount>,
}

#[derive(Clone)]
struct LoadedImage {
    path: String,
    elf: ElfImage,
    bytes: Vec<u8>,
    inode: crate::fs::types::InodeRef,
    dentry: crate::fs::types::DentryRef,
    mount: alloc::sync::Arc<crate::fs::mount::Mount>,
    from_script: bool,
}

#[derive(Clone)]
struct LoadedProgram {
    main: LoadedImage,
    interp: Option<LoadedImage>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecLoadLayout {
    main_bias: u64,
    interp_bias: u64,
    at_base: u64,
    entry_ip: u64,
    at_entry: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExecRelocationPlan {
    relocate_main_relative: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ElfDynamicRelocations {
    rela_vaddr: u64,
    rela_size: usize,
    rela_ent: usize,
    relr_vaddr: u64,
    relr_size: usize,
    relr_ent: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ElfLoadWindow {
    map_start: u64,
    map_len: u64,
    file_offset: usize,
    file_len: usize,
    zero_start: u64,
    zero_len: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShebangSpec {
    interpreter: String,
    arg: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecSecurityContext {
    pub uid: u32,
    pub euid: u32,
    pub gid: u32,
    pub egid: u32,
    pub secure_exec: bool,
}

struct ProposedExecCreds {
    cred: *mut Cred,
    security: ExecSecurityContext,
}

impl Drop for ProposedExecCreds {
    fn drop(&mut self) {
        if !self.cred.is_null() {
            unsafe { Cred::put(self.cred) };
        }
    }
}

impl ProposedExecCreds {
    fn security(&self) -> ExecSecurityContext {
        self.security
    }

    fn commit(mut self) {
        let cred = self.cred;
        self.cred = core::ptr::null_mut();
        cred::commit_creds(cred);
    }
}

fn securebit_mask(bit: u32) -> u32 {
    1u32 << bit
}

fn exec_creds_privileged_before(old: &Cred, new: &Cred) -> bool {
    old.uid != new.uid
        || old.euid != new.euid
        || old.gid != new.gid
        || old.egid != new.egid
        || old.suid != new.suid
        || old.sgid != new.sgid
        || old.fsuid != new.fsuid
        || old.fsgid != new.fsgid
        || old.cap_permitted != new.cap_permitted
        || old.cap_effective != new.cap_effective
}

fn final_exec_nosuid(program: &LoadedProgram) -> bool {
    (program.main.mount.flags.load(Ordering::Acquire) & MS_NOSUID as u32) != 0
}

fn final_exec_has_file_caps(image: &LoadedImage) -> bool {
    image
        .inode
        .xattrs
        .lock()
        .contains_key("security.capability")
}

fn prepare_exec_creds(program: &LoadedProgram) -> Result<ProposedExecCreds, i32> {
    let task = unsafe { sched::get_current() };
    let old_ptr = cred::current_cred();
    if old_ptr.is_null() {
        return Err(-3);
    }
    let Some(new_ptr) = cred::prepare_creds() else {
        return Err(-12);
    };

    let new = unsafe { &mut *new_ptr };
    let old = unsafe { &*old_ptr };
    let mode = program.main.inode.mode.load(Ordering::Acquire);
    let file_uid = KUid(program.main.inode.uid.load(Ordering::Acquire));
    let file_gid = KGid(program.main.inode.gid.load(Ordering::Acquire));
    let no_new_privs = !task.is_null() && unsafe { (*task).m27.no_new_privs != 0 };
    let nosuid = final_exec_nosuid(program);
    let script = program.main.from_script;
    let has_file_caps = final_exec_has_file_caps(&program.main);
    let setid_or_caps = (mode & (S_ISUID | S_ISGID)) != 0 || has_file_caps;
    let allow_privilege = !nosuid && !no_new_privs && !script;

    if allow_privilege {
        if mode & S_ISUID != 0 {
            new.euid = file_uid;
            new.suid = file_uid;
            new.fsuid = file_uid;
        }
        if mode & S_ISGID != 0 {
            new.egid = file_gid;
            new.sgid = file_gid;
            new.fsgid = file_gid;
        }
    }

    if has_file_caps || (!allow_privilege && setid_or_caps) {
        new.cap_ambient = KernelCapT::empty();
    }
    if new.euid.0 == 0
        && old.euid.0 != 0
        && new.securebits & securebit_mask(cred::securebits::SECURE_NOROOT) == 0
        && new.securebits & securebit_mask(cred::securebits::SECURE_NO_SETUID_FIXUP) == 0
    {
        new.cap_permitted = new.cap_bset;
        new.cap_effective = new.cap_permitted;
    }
    if new.euid.0 != 0
        && new.securebits & securebit_mask(cred::securebits::SECURE_NO_SETUID_FIXUP) == 0
    {
        new.cap_effective = KernelCapT::empty();
    }

    let changed_privilege = exec_creds_privileged_before(old, new);
    let secure_exec = changed_privilege || (!allow_privilege && setid_or_caps);
    let security = ExecSecurityContext {
        uid: new.uid.0,
        euid: new.euid.0,
        gid: new.gid.0,
        egid: new.egid.0,
        secure_exec,
    };
    Ok(ProposedExecCreds {
        cred: new_ptr,
        security,
    })
}

static EXEC_STARTS: Mutex<Vec<(i32, UserStartContext)>> = Mutex::new(Vec::new());

pub fn take_exec_start_for_current() -> Option<UserStartContext> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return None;
    }
    let pid = unsafe { (*task).pid };
    let mut starts = EXEC_STARTS.lock();
    let idx = starts.iter().position(|(p, _)| *p == pid)?;
    Some(starts.swap_remove(idx).1)
}

fn set_exec_start_for_pid(pid: i32, ctx: UserStartContext) {
    let mut starts = EXEC_STARTS.lock();
    if let Some(entry) = starts.iter_mut().find(|(p, _)| *p == pid) {
        *entry = (pid, ctx);
    } else {
        starts.push((pid, ctx));
    }
}

pub fn parse_elf_image(bytes: &[u8]) -> Result<ElfImage, i32> {
    if bytes.len() < 64 {
        return Err(-8); // ENOEXEC
    }
    if &bytes[0..4] != ELF_MAGIC {
        return Err(-8);
    }
    if bytes[4] != ELFCLASS64 || bytes[5] != ELFDATA2LSB || bytes[6] != EV_CURRENT {
        return Err(-8);
    }

    let e_type = read_u16(bytes, 16)?;
    let e_machine = read_u16(bytes, 18)?;
    let e_entry = read_u64(bytes, 24)?;
    let e_phoff = read_u64(bytes, 32)?;
    let e_phentsize = read_u16(bytes, 54)?;
    let e_phnum = read_u16(bytes, 56)?;

    if (e_type != ET_EXEC && e_type != ET_DYN) || e_machine != EM_X86_64 {
        return Err(-8);
    }
    if e_phentsize < 56 {
        return Err(-8);
    }

    let mut interp = None;
    let mut dynamic = None;
    let mut loads = Vec::new();
    for idx in 0..e_phnum {
        let off = e_phoff as usize + (idx as usize * e_phentsize as usize);
        let end = off.checked_add(e_phentsize as usize).ok_or(-8)?;
        if end > bytes.len() {
            return Err(-8);
        }

        let p_type = read_u32(bytes, off)?;
        let p_flags = read_u32(bytes, off + 4)?;
        let p_offset = read_u64(bytes, off + 8)?;
        let p_vaddr = read_u64(bytes, off + 16)?;
        let p_filesz = read_u64(bytes, off + 32)?;
        let p_memsz = read_u64(bytes, off + 40)?;

        if p_type == PT_LOAD {
            loads.push(ElfLoadSegment {
                vaddr: p_vaddr,
                memsz: p_memsz,
                filesz: p_filesz,
                flags: p_flags,
                offset: p_offset,
            });
        } else if p_type == PT_DYNAMIC {
            dynamic = Some(ElfDynamicSegment {
                offset: p_offset,
                vaddr: p_vaddr,
                filesz: p_filesz,
            });
        } else if p_type == PT_INTERP {
            let start = p_offset as usize;
            let stop = start.checked_add(p_filesz as usize).ok_or(-8)?;
            if stop > bytes.len() || p_filesz == 0 {
                return Err(-8);
            }
            let raw = &bytes[start..stop];
            let nul = raw.iter().position(|b| *b == 0).ok_or(-8)?;
            let txt = core::str::from_utf8(&raw[..nul]).map_err(|_| -8)?;
            interp = Some(txt.to_string());
        }
    }

    if loads.is_empty() {
        return Err(-8);
    }

    Ok(ElfImage {
        entry: e_entry,
        et_dyn: e_type == ET_DYN,
        phoff: e_phoff,
        phentsize: e_phentsize,
        phnum: e_phnum,
        interp,
        dynamic,
        load_segments: loads,
    })
}

pub fn resolve_exec_image(path: &str) -> Result<ExecResolution, i32> {
    let loaded = load_image_with_shebang(path, 0)?;
    Ok(ExecResolution {
        requested_path: path.to_string(),
        resolved_path: loaded.path,
        elf: loaded.elf,
        inode: loaded.inode,
        dentry: loaded.dentry,
        mount: loaded.mount,
    })
}

fn load_program(path: &str) -> Result<LoadedProgram, i32> {
    let main = load_image_with_shebang(path, 0)?;
    let interp = if let Some(ref interp_path) = main.elf.interp {
        Some(load_image_with_shebang(interp_path, 0)?)
    } else {
        None
    };
    Ok(LoadedProgram { main, interp })
}

fn load_image_with_shebang(path: &str, depth: usize) -> Result<LoadedImage, i32> {
    if depth > MAX_INTERP_RECURSION {
        return Err(-40); // ELOOP
    }
    let meta = read_exec_file_meta(path)?;
    if let Some(next) = parse_shebang_interpreter(&meta.bytes)? {
        let mut image = load_image_with_shebang(&next.interpreter, depth + 1)?;
        image.from_script = true;
        return Ok(image);
    }
    let elf = parse_elf_image(&meta.bytes)?;
    Ok(LoadedImage {
        path: normalize_exec_path(path),
        elf,
        bytes: meta.bytes,
        inode: meta.inode,
        dentry: meta.dentry,
        mount: meta.mount,
        from_script: false,
    })
}

fn read_shebang_spec(path: &str) -> Result<Option<ShebangSpec>, i32> {
    let bytes = read_exec_file(path)?;
    parse_shebang_interpreter(&bytes)
}

struct ExecFileMeta {
    bytes: Vec<u8>,
    inode: crate::fs::types::InodeRef,
    dentry: crate::fs::types::DentryRef,
    mount: alloc::sync::Arc<crate::fs::mount::Mount>,
}

fn read_exec_file(path: &str) -> Result<Vec<u8>, i32> {
    read_exec_file_meta(path).map(|meta| meta.bytes)
}

fn read_exec_file_meta(path: &str) -> Result<ExecFileMeta, i32> {
    let (mount, dentry) =
        crate::fs::mount::resolve_path_follow(path).map_err(|errno| -(errno as i32))?;
    let inode = dentry.inode().ok_or(-2)?;
    if inode.kind != crate::fs::types::InodeKind::Regular {
        return Err(-13);
    }
    let bytes = match &inode.private {
        crate::fs::types::InodePrivate::StaticBytes(bytes) => Ok(bytes.to_vec()),
        crate::fs::types::InodePrivate::StaticCowBytes { base, overlay } => {
            if let Some(bytes) = overlay.lock().as_ref() {
                Ok(bytes.clone())
            } else {
                Ok(base.to_vec())
            }
        }
        crate::fs::types::InodePrivate::RamBytes(bytes) => Ok(bytes.lock().clone()),
        _ => read_regular_inode_bytes(path, dentry.clone(), inode.clone()),
    }?;
    Ok(ExecFileMeta {
        bytes,
        inode,
        dentry,
        mount,
    })
}

fn read_regular_inode_bytes(
    path: &str,
    dentry: crate::fs::types::DentryRef,
    inode: crate::fs::types::InodeRef,
) -> Result<Vec<u8>, i32> {
    let expected = inode.size.load(Ordering::Acquire) as usize;
    if expected > MAX_EXEC_FILE_BYTES {
        return Err(-7); // E2BIG
    }

    let file = crate::fs::file::alloc_file(
        dentry,
        crate::include::uapi::fcntl::O_RDONLY,
        inode.mode.load(Ordering::Acquire),
        inode.fops,
    );
    crate::fs::file::set_path_hint(&file, path.to_string());

    let result = (|| {
        let mut out = Vec::with_capacity(expected);
        let mut chunk = vec![0u8; EXEC_READ_CHUNK.min(MAX_EXEC_FILE_BYTES)];
        loop {
            let n = crate::fs::read_write::vfs_read(&file, &mut chunk)
                .map_err(|errno| -(errno as i32))?;
            if n == 0 {
                break;
            }
            let next_len = out.len().checked_add(n).ok_or(-7)?;
            if next_len > MAX_EXEC_FILE_BYTES {
                return Err(-7);
            }
            out.extend_from_slice(&chunk[..n]);
        }
        Ok(out)
    })();

    crate::fs::file::fput(file);
    result
}

fn parse_shebang_interpreter(bytes: &[u8]) -> Result<Option<ShebangSpec>, i32> {
    if bytes.len() < 2 || &bytes[0..2] != b"#!" {
        return Ok(None);
    }
    let line_end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len());
    let line = core::str::from_utf8(&bytes[2..line_end]).map_err(|_| -8)?;
    let mut words = line.split_whitespace();
    let interpreter = words.next().ok_or(-8)?;
    if interpreter.is_empty() {
        return Err(-8);
    }
    Ok(Some(ShebangSpec {
        interpreter: interpreter.to_string(),
        arg: words.next().map(ToString::to_string),
    }))
}

fn rewrite_argv_for_shebang(script_path: &str, argv: &[String], spec: &ShebangSpec) -> Vec<String> {
    let mut rewritten = Vec::with_capacity(argv.len() + 3);
    rewritten.push(spec.interpreter.clone());
    if let Some(arg) = spec.arg.as_ref() {
        rewritten.push(arg.clone());
    }
    rewritten.push(script_path.to_string());
    if argv.len() > 1 {
        rewritten.extend_from_slice(&argv[1..]);
    }
    rewritten
}

fn normalize_exec_path(path: &str) -> String {
    crate::fs::fs_struct::absolute_from_cwd(path)
}

fn resolve_exec_path_for_load(path: &str) -> Result<String, i32> {
    let mut normalized = normalize_exec_path(path);
    for _ in 0..8 {
        match crate::fs::proc::fd::current_fd_path_from_proc_path(&normalized) {
            Some(Ok(path)) => {
                #[cfg(not(test))]
                if crate::kernel::debug_trace::proc_enabled()
                    && (normalized.starts_with("/proc/self/fd/")
                        || normalized.starts_with("/dev/fd/"))
                {
                    crate::linux_driver_abi::tty::serial_println!(
                        "trace-proc-exec-resolve path={} resolved={}",
                        normalized,
                        path
                    );
                }
                normalized = normalize_exec_path(&path);
            }
            Some(Err(errno)) => {
                #[cfg(not(test))]
                if crate::kernel::debug_trace::proc_enabled()
                    && (normalized.starts_with("/proc/self/fd/")
                        || normalized.starts_with("/dev/fd/"))
                {
                    crate::linux_driver_abi::tty::serial_println!(
                        "trace-proc-exec-resolve path={} errno={}",
                        normalized,
                        errno
                    );
                }
                return Err(-(errno as i32));
            }
            None => {
                #[cfg(not(test))]
                if crate::kernel::debug_trace::proc_enabled()
                    && (normalized.starts_with("/proc/self/fd/")
                        || normalized.starts_with("/dev/fd/"))
                {
                    crate::linux_driver_abi::tty::serial_println!(
                        "trace-proc-exec-resolve path={} miss",
                        normalized
                    );
                }
                return Ok(normalized);
            }
        }
    }
    Err(-40)
}

fn exec_load_layout(program: &LoadedProgram) -> Result<ExecLoadLayout, i32> {
    let main_bias = if program.main.elf.et_dyn {
        PIE_LOAD_BIAS
    } else {
        0
    };
    let at_entry = main_bias.checked_add(program.main.elf.entry).ok_or(-12)?;
    if let Some(interp) = program.interp.as_ref() {
        let interp_bias = if interp.elf.et_dyn {
            INTERP_LOAD_BIAS
        } else {
            0
        };
        let entry_ip = interp_bias.checked_add(interp.elf.entry).ok_or(-12)?;
        Ok(ExecLoadLayout {
            main_bias,
            interp_bias,
            at_base: interp_bias,
            entry_ip,
            at_entry,
        })
    } else {
        Ok(ExecLoadLayout {
            main_bias,
            interp_bias: 0,
            at_base: 0,
            entry_ip: at_entry,
            at_entry,
        })
    }
}

fn exec_relocation_plan(program: &LoadedProgram) -> ExecRelocationPlan {
    let _ = program;
    ExecRelocationPlan {
        // Linux's ELF loader maps the executable image and enters either the
        // PT_INTERP loader or the program entry point. Dynamic relocations are
        // resolved in userspace: by ld.so for interpreted PIEs, or by the
        // static PIE startup code for ET_DYN binaries without PT_INTERP.
        relocate_main_relative: false,
    }
}

unsafe fn copy_user_cstr(ptr: *const c_char) -> Result<String, i32> {
    if ptr.is_null() {
        return Err(-14); // EFAULT
    }
    let mut out = Vec::new();
    for i in 0..MAX_ARG_STRLEN {
        let b = unsafe { *ptr.add(i) } as u8;
        if b == 0 {
            let s = core::str::from_utf8(&out).map_err(|_| -14)?;
            return Ok(s.to_string());
        }
        out.push(b);
    }
    Err(-7) // E2BIG
}

unsafe fn copy_user_cstr_array(list: *const *const c_char) -> Result<Vec<String>, i32> {
    if list.is_null() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for idx in 0..MAX_ARG_COUNT {
        let p = unsafe { *list.add(idx) };
        if p.is_null() {
            return Ok(out);
        }
        out.push(unsafe { copy_user_cstr(p)? });
    }
    Err(-7) // E2BIG
}

fn set_task_comm_from_path(task: *mut TaskStruct, path: &str) {
    let base = path.rsplit('/').next().unwrap_or(path).as_bytes();
    let n = core::cmp::min(base.len(), 15);
    unsafe {
        (*task).comm.fill(0);
        (&mut (*task).comm)[..n].copy_from_slice(&base[..n]);
    }
}

#[cfg(not(test))]
fn trace_ping_exec_commit(path: &str, exec_path: &str) {
    let task = unsafe { crate::kernel::sched::get_current() };
    let pid = if task.is_null() {
        -1
    } else {
        unsafe { (*task).pid }
    };
    if crate::kernel::debug_trace::remember_ping_pid_for_exec(pid, path, exec_path) {
        crate::linux_driver_abi::tty::serial_println!(
            "trace-ping-track pid={} path={} exec_path={}",
            pid,
            path,
            exec_path
        );
    }
}

unsafe fn alloc_exec_mm() -> Result<*mut MmStruct, i32> {
    if !is_buddy_ready() {
        return Err(-12); // ENOMEM
    }

    let pgd_page = with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL)).ok_or(-12)?;
    let pgd_pfn = page_to_pfn(pgd_page);
    let pgd_virt = crate::arch::x86::mm::paging::pfn_to_virt(pgd_pfn) as *mut u8;
    unsafe { core::ptr::write_bytes(pgd_virt, 0, PAGE_SIZE) };

    #[cfg(not(test))]
    {
        let kernel_pgd = crate::arch::x86::mm::paging::phys_to_virt(
            crate::arch::x86::mm::paging::init_pgd_phys(),
        ) as *const pgd_t;
        // The current kernel still runs out of the low-half identity mapping
        // (see arch/x86/kernel/vmlinux.lds.S placing the image at 0x00200000).
        // Preserve kernel-side mappings when building a userspace PGD, but
        // privatize slot 0's lower tables so low user VMAs cannot mutate the
        // boot PGD tables shared with the direct map.
        //
        // Do not copy from active CR3 here: after the first userspace launch,
        // active CR3 is a process mm and contains stale user mappings.
        unsafe { copy_exec_kernel_pgd_entries(pgd_virt as *mut pgd_t, kernel_pgd)? };
    }

    Ok(Box::into_raw(Box::new(MmStruct::new(pgd_virt as usize))))
}

unsafe fn copy_exec_kernel_pgd_entries(
    dst: *mut pgd_t,
    kernel_pgd: *const pgd_t,
) -> Result<(), i32> {
    unsafe {
        core::ptr::copy_nonoverlapping(kernel_pgd, dst, 512);
        // Slot 0 keeps the low identity map used by the current kernel image.
        // Slots 256..511 keep the direct-map / higher-half kernel windows.
        // Everything else is user address space and must start empty for exec.
        let mut idx = 1usize;
        while idx < 256 {
            *dst.add(idx) = pgd_t(0);
            idx += 1;
        }

        #[cfg(not(test))]
        crate::arch::x86::mm::paging::clone_low_identity_pgd_slot_for_user(dst, kernel_pgd)
            .ok_or(-12)?;
    }

    Ok(())
}

fn prot_from_segment_flags(flags: u32) -> u32 {
    let mut prot = 0u32;
    if (flags & PF_R) != 0 {
        prot |= PROT_READ;
    }
    if (flags & PF_W) != 0 {
        prot |= PROT_WRITE;
    }
    if (flags & PF_X) != 0 {
        prot |= PROT_EXEC;
    }
    prot
}

fn align_down(v: u64, align: u64) -> u64 {
    v & !(align - 1)
}

fn align_up(v: u64, align: u64) -> u64 {
    (v + align - 1) & !(align - 1)
}

unsafe fn map_elf_into_mm(
    mm: *mut MmStruct,
    image: &ElfImage,
    data: &[u8],
    load_bias: u64,
) -> Result<(), i32> {
    for seg in image.load_segments.iter() {
        let Some(window) = elf_load_window(seg, load_bias)? else {
            continue;
        };
        let final_prot = prot_from_segment_flags(seg.flags);
        // We copy segment contents into anonymous memory, so we must temporarily
        // allow writes even for RX mappings; afterwards we mprotect back to the
        // final ELF p_flags-derived prot.
        let load_prot = final_prot | PROT_WRITE;

        unsafe {
            do_mmap(
                &mut *mm,
                window.map_start,
                window.map_len,
                load_prot,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                0,
                0,
            )?
        };

        if window.file_len > 0 {
            let file_end = window.file_offset.checked_add(window.file_len).ok_or(-8)?;
            if file_end > data.len() {
                return Err(-8);
            }
            if let Err(err) =
                unsafe { user_write(mm, window.map_start, &data[window.file_offset..file_end]) }
            {
                crate::kernel::printk::log_error!(
                    "exec",
                    "map_elf: file write failed vaddr={:#x} len={:#x} err={}",
                    window.map_start,
                    window.file_len,
                    err
                );
                return Err(err);
            }
        }

        if window.zero_len > 0 {
            let zero_len = window.zero_len as usize;
            let zeroes = vec![0u8; zero_len];
            if let Err(err) = unsafe { user_write(mm, window.zero_start, &zeroes) } {
                crate::kernel::printk::log_error!(
                    "exec",
                    "map_elf: zero write failed vaddr={:#x} len={:#x} err={}",
                    window.zero_start,
                    zero_len,
                    err
                );
                return Err(err);
            }
        }

        // Downgrade back to the final mapping protections (e.g. RX for text).
        if (final_prot & PROT_WRITE) == 0 {
            unsafe { do_mprotect(&mut *mm, window.map_start, window.map_len, final_prot)? };
        }
    }
    Ok(())
}

fn elf_load_window(seg: &ElfLoadSegment, load_bias: u64) -> Result<Option<ElfLoadWindow>, i32> {
    if seg.memsz == 0 {
        return Ok(None);
    }
    if seg.filesz > seg.memsz {
        return Err(-8);
    }

    let seg_start = seg.vaddr.checked_add(load_bias).ok_or(-12)?;
    let map_start = align_down(seg_start, PAGE_SIZE as u64);
    let page_offset = seg_start.checked_sub(map_start).ok_or(-12)?;
    let image_len = page_offset.checked_add(seg.memsz).ok_or(-12)?;
    let map_len = align_up(image_len, PAGE_SIZE as u64);
    let file_len = if seg.filesz == 0 {
        0
    } else {
        page_offset.checked_add(seg.filesz).ok_or(-12)?
    };
    let file_offset = if file_len == 0 {
        0
    } else {
        seg.offset.checked_sub(page_offset).ok_or(-8)?
    };
    let zero_start = seg_start.checked_add(seg.filesz).ok_or(-12)?;
    let map_end = map_start.checked_add(map_len).ok_or(-12)?;
    let zero_len = map_end.checked_sub(zero_start).ok_or(-12)?;

    Ok(Some(ElfLoadWindow {
        map_start,
        map_len,
        file_offset: usize::try_from(file_offset).map_err(|_| -8)?,
        file_len: usize::try_from(file_len).map_err(|_| -8)?,
        zero_start,
        zero_len,
    }))
}

fn elf_vaddr_to_file_offset(image: &ElfImage, vaddr: u64) -> Option<usize> {
    for seg in image.load_segments.iter() {
        let start = seg.vaddr;
        let end = start.checked_add(seg.filesz)?;
        if vaddr >= start && vaddr.checked_add(8)? <= end {
            let off = seg.offset.checked_add(vaddr.checked_sub(start)?)?;
            return usize::try_from(off).ok();
        }
    }
    None
}

unsafe fn apply_elf_relative_relocations(
    mm: *mut MmStruct,
    image: &ElfImage,
    data: &[u8],
    load_bias: u64,
    strict: bool,
) -> Result<(), i32> {
    let Some(relocs) = elf_dynamic_relocations(image, data)? else {
        return Ok(());
    };
    unsafe { apply_elf_rela_relative_relocations(mm, image, data, load_bias, strict, &relocs)? };
    unsafe { apply_elf_relr_relative_relocations(mm, image, data, load_bias, &relocs)? };
    Ok(())
}

fn elf_dynamic_relocations(
    image: &ElfImage,
    data: &[u8],
) -> Result<Option<ElfDynamicRelocations>, i32> {
    let Some(dynamic) = image.dynamic else {
        return Ok(None);
    };
    let dyn_start = dynamic.offset as usize;
    let dyn_end = dyn_start.checked_add(dynamic.filesz as usize).ok_or(-8)?;
    if dyn_end > data.len() {
        return Err(-8);
    }

    let mut relocs = ElfDynamicRelocations {
        rela_ent: 24,
        relr_ent: 8,
        ..ElfDynamicRelocations::default()
    };
    let mut off = dyn_start;
    while off + 16 <= dyn_end {
        let tag = read_i64(data, off)?;
        let val = read_u64(data, off + 8)?;
        match tag {
            DT_NULL => break,
            DT_RELA => relocs.rela_vaddr = val,
            DT_RELASZ => {
                relocs.rela_size = val as usize;
            }
            DT_RELAENT => relocs.rela_ent = val as usize,
            DT_RELR => relocs.relr_vaddr = val,
            DT_RELRSZ => {
                relocs.relr_size = val as usize;
            }
            DT_RELRENT => relocs.relr_ent = val as usize,
            _ => {}
        }
        off += 16;
    }

    Ok(Some(relocs))
}

unsafe fn apply_elf_rela_relative_relocations(
    mm: *mut MmStruct,
    image: &ElfImage,
    data: &[u8],
    load_bias: u64,
    strict: bool,
    relocs: &ElfDynamicRelocations,
) -> Result<(), i32> {
    if relocs.rela_vaddr == 0 || relocs.rela_size == 0 {
        return Ok(());
    }
    if relocs.rela_ent < 24 || relocs.rela_size % relocs.rela_ent != 0 {
        return Err(-8);
    }
    let rela_off = elf_vaddr_to_file_offset(image, relocs.rela_vaddr).ok_or(-8)?;
    let rela_end = rela_off.checked_add(relocs.rela_size).ok_or(-8)?;
    if rela_end > data.len() {
        return Err(-8);
    }

    let mut cur = rela_off;
    while cur < rela_end {
        let r_offset = read_u64(data, cur)?;
        let r_info = read_u64(data, cur + 8)?;
        let r_addend = read_i64(data, cur + 16)?;
        let r_type = r_info & 0xffff_ffff;
        if r_type != R_X86_64_RELATIVE {
            if strict {
                return Err(-8);
            }
            cur += relocs.rela_ent;
            continue;
        }
        let target = load_bias.checked_add(r_offset).ok_or(-12)?;
        let value = load_bias.wrapping_add(r_addend as u64);
        unsafe { user_write_u64(mm, target, value)? };
        cur += relocs.rela_ent;
    }
    Ok(())
}

unsafe fn apply_elf_relr_relative_relocations(
    mm: *mut MmStruct,
    image: &ElfImage,
    data: &[u8],
    load_bias: u64,
    relocs: &ElfDynamicRelocations,
) -> Result<(), i32> {
    walk_elf_relr_relocations(image, data, relocs, |target_vaddr| {
        let target = load_bias.checked_add(target_vaddr).ok_or(-12)?;
        let value = unsafe { user_read_u64(mm, target)? }.wrapping_add(load_bias);
        unsafe { user_write_u64(mm, target, value)? };
        Ok(())
    })
}

fn walk_elf_relr_relocations<F>(
    image: &ElfImage,
    data: &[u8],
    relocs: &ElfDynamicRelocations,
    mut visit: F,
) -> Result<(), i32>
where
    F: FnMut(u64) -> Result<(), i32>,
{
    if relocs.relr_vaddr == 0 || relocs.relr_size == 0 {
        return Ok(());
    }
    if relocs.relr_ent != 8 || relocs.relr_size % relocs.relr_ent != 0 {
        return Err(-8);
    }
    let relr_off = elf_vaddr_to_file_offset(image, relocs.relr_vaddr).ok_or(-8)?;
    let relr_end = relr_off.checked_add(relocs.relr_size).ok_or(-8)?;
    if relr_end > data.len() {
        return Err(-8);
    }

    let mut cur = relr_off;
    let mut next_vaddr = 0u64;
    while cur < relr_end {
        let entry = read_u64(data, cur)?;
        if (entry & 1) == 0 {
            visit(entry)?;
            next_vaddr = entry.checked_add(8).ok_or(-12)?;
        } else {
            if next_vaddr == 0 {
                return Err(-8);
            }
            let mut bits = entry >> 1;
            let mut target_vaddr = next_vaddr;
            while bits != 0 {
                if (bits & 1) != 0 {
                    visit(target_vaddr)?;
                }
                bits >>= 1;
                target_vaddr = target_vaddr.checked_add(8).ok_or(-12)?;
            }
            next_vaddr = next_vaddr.checked_add(63 * 8).ok_or(-12)?;
        }
        cur += relocs.relr_ent;
    }
    Ok(())
}

fn build_auxv(
    main: &ElfImage,
    main_bias: u64,
    at_base: u64,
    entry_ip: u64,
    random_ptr: u64,
    execfn_ptr: u64,
    vdso_ehdr: u64,
    security: ExecSecurityContext,
) -> Vec<(u64, u64)> {
    let mut auxv = vec![
        (AT_PAGESZ, PAGE_SIZE as u64),
        (AT_PHDR, main_bias + main.phoff),
        (AT_PHENT, main.phentsize as u64),
        (AT_PHNUM, main.phnum as u64),
        (AT_BASE, at_base),
        (AT_ENTRY, entry_ip),
        (AT_UID, security.uid as u64),
        (AT_EUID, security.euid as u64),
        (AT_GID, security.gid as u64),
        (AT_EGID, security.egid as u64),
        (AT_HWCAP, 0),
        (AT_SECURE, security.secure_exec as u64),
        (AT_RANDOM, random_ptr),
        (AT_EXECFN, execfn_ptr),
    ];
    if vdso_ehdr != 0 {
        auxv.push((AT_SYSINFO_EHDR, vdso_ehdr));
    }
    auxv
}

unsafe fn map_stack(mm: *mut MmStruct) -> Result<(u64, u64), i32> {
    let stack_top = align_down(TASK_SIZE - PAGE_SIZE as u64, PAGE_SIZE as u64);
    let stack_start = stack_top.checked_sub(STACK_SIZE).ok_or(-12)?;
    unsafe {
        do_mmap(
            &mut *mm,
            stack_start,
            STACK_SIZE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED | MAP_GROWSDOWN,
            0,
            0,
        )?
    };
    Ok((stack_start, stack_top))
}

unsafe fn build_initial_stack(
    mm: *mut MmStruct,
    stack_top: u64,
    execfn: &str,
    argv: &[String],
    envp: &[String],
    main: &ElfImage,
    main_bias: u64,
    at_base: u64,
    at_entry: u64,
    vdso_ehdr: u64,
    security: ExecSecurityContext,
) -> Result<(u64, u64, u64), i32> {
    let mut sp = stack_top;

    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    for b in execfn.as_bytes() {
        seed = seed.rotate_left(5) ^ (*b as u64);
    }
    let mut random = [0u8; 16];
    for slot in random.iter_mut() {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        *slot = (seed & 0xff) as u8;
    }
    sp = sp.checked_sub(random.len() as u64).ok_or(-12)?;
    unsafe { user_write(mm, sp, &random)? };
    let random_ptr = sp;

    sp = sp.checked_sub(execfn.len() as u64 + 1).ok_or(-12)?;
    unsafe { user_write(mm, sp, execfn.as_bytes())? };
    unsafe { user_write(mm, sp + execfn.len() as u64, &[0])? };
    let execfn_ptr = sp;

    let mut env_ptrs = Vec::new();
    for item in envp.iter().rev() {
        sp = sp.checked_sub(item.len() as u64 + 1).ok_or(-12)?;
        unsafe { user_write(mm, sp, item.as_bytes())? };
        unsafe { user_write(mm, sp + item.len() as u64, &[0])? };
        env_ptrs.push(sp);
    }
    env_ptrs.reverse();

    let mut argv_ptrs = Vec::new();
    for item in argv.iter().rev() {
        sp = sp.checked_sub(item.len() as u64 + 1).ok_or(-12)?;
        unsafe { user_write(mm, sp, item.as_bytes())? };
        unsafe { user_write(mm, sp + item.len() as u64, &[0])? };
        argv_ptrs.push(sp);
    }
    argv_ptrs.reverse();

    let auxv = build_auxv(
        main, main_bias, at_base, at_entry, random_ptr, execfn_ptr, vdso_ehdr, security,
    );
    let mut words = Vec::new();
    words.push(argv_ptrs.len() as u64);
    words.extend(argv_ptrs.iter().copied());
    words.push(0);
    words.extend(env_ptrs.iter().copied());
    words.push(0);
    for (key, val) in auxv {
        words.push(key);
        words.push(val);
    }
    words.push(AT_NULL);
    words.push(0);

    let frame_size = (words.len() * core::mem::size_of::<u64>()) as u64;
    sp = align_down(sp.checked_sub(frame_size).ok_or(-12)?, 16);
    for (idx, word) in words.iter().enumerate() {
        unsafe { user_write_u64(mm, sp + (idx as u64 * 8), *word)? };
    }

    unsafe {
        (*mm).start_stack = sp;
        if let Some(first) = argv_ptrs.first().copied() {
            (*mm).arg_start = first;
            let last = argv_ptrs.last().copied().unwrap_or(first);
            (*mm).arg_end = last + argv.last().map(|s| s.len() as u64 + 1).unwrap_or(0);
        }
        if let Some(first) = env_ptrs.first().copied() {
            (*mm).env_start = first;
            let last = env_ptrs.last().copied().unwrap_or(first);
            (*mm).env_end = last + envp.last().map(|s| s.len() as u64 + 1).unwrap_or(0);
        }
    }

    Ok((sp, random_ptr, execfn_ptr))
}

unsafe fn commit_exec_for_current(
    path: &str,
    argv: &[String],
    envp: &[String],
    program: &LoadedProgram,
    proposed_creds: ProposedExecCreds,
) -> Result<UserStartContext, i32> {
    let task = unsafe { sched::get_current() };
    if task.is_null() {
        return Err(-3);
    }

    let mm = unsafe { alloc_exec_mm()? };
    let layout = exec_load_layout(program)?;
    let relocation_plan = exec_relocation_plan(program);
    let main_bias = layout.main_bias;
    unsafe { map_elf_into_mm(mm, &program.main.elf, &program.main.bytes, main_bias)? };
    if relocation_plan.relocate_main_relative {
        unsafe {
            apply_elf_relative_relocations(
                mm,
                &program.main.elf,
                &program.main.bytes,
                main_bias,
                true,
            )?
        };
    }

    let entry_ip = if let Some(interp) = program.interp.as_ref() {
        let interp_bias = layout.interp_bias;
        unsafe { map_elf_into_mm(mm, &interp.elf, &interp.bytes, interp_bias)? };
        layout.entry_ip
    } else {
        layout.entry_ip
    };

    let vdso_ehdr = unsafe { crate::arch::x86::kernel::vdso::exec_vdso_ehdr(mm)? };
    let (_stack_start, stack_top) = unsafe { map_stack(mm)? };
    let (sp, _, _) = unsafe {
        build_initial_stack(
            mm,
            stack_top,
            path,
            argv,
            envp,
            &program.main.elf,
            main_bias,
            layout.at_base,
            layout.at_entry,
            vdso_ehdr,
            proposed_creds.security(),
        )?
    };

    // Close fds marked FD_CLOEXEC, mirroring Linux's
    // `do_close_on_exec()`.  systemd's executor model assumes any
    // O_CLOEXEC fd inherited from the manager is gone by the time the
    // new image runs; without this, executor children inherit stale
    // sockets (signalfd, pidfd, journal Unix sockets) and dup2/fcntl
    // on those slots returns EBADF, which trips `safe_fclose()`'s
    // assertion in `src/basic/fd-util.c:140` and aborts PID 1.
    //
    // Ref: vendor/linux/fs/exec.c::flush_old_exec -> do_close_on_exec.
    if let Some(files) = unsafe { crate::kernel::files::get_task_files(task) } {
        files.close_on_exec();
    }
    crate::kernel::syscalls::clear_current_rseq_registration_for_exec();
    crate::kernel::signal::flush_signal_handlers_for_exec(false);

    let old_mm = unsafe { (*task).mm };

    unsafe {
        security::security_bprm_committing_creds(path.as_bytes());
        proposed_creds.commit();
        (*task).mm = mm;
        (*task).active_mm = mm;
        (*task).thread.fsbase = 0;
        (*task).thread.gsbase = 0;
        (*task).thread.fsindex = 0;
        (*task).thread.gsindex = 0;
        set_task_comm_from_path(task, path);
        (*mm).start_code = main_bias
            + program
                .main
                .elf
                .load_segments
                .iter()
                .map(|s| s.vaddr)
                .min()
                .unwrap_or(0);
        (*mm).end_code = main_bias
            + program
                .main
                .elf
                .load_segments
                .iter()
                .map(|s| s.vaddr + s.memsz)
                .max()
                .unwrap_or(0);
        let brk_base = align_up((*mm).end_code, PAGE_SIZE as u64);
        (*mm).start_brk = brk_base;
        (*mm).brk = brk_base;
    }

    let ctx = UserStartContext {
        ip: entry_ip,
        sp,
        rflags: 0x202,
        old_mm: old_mm as usize,
    };
    let pid = unsafe { (*task).pid };
    set_exec_start_for_pid(pid, ctx);
    security::security_bprm_committed_creds(path.as_bytes());
    Ok(ctx)
}

fn prepare_exec_security(path: &str) -> Result<(), i32> {
    let path_bytes = path.as_bytes();
    let err = security::security_bprm_creds_for_exec(path_bytes);
    if err != 0 {
        return Err(err);
    }
    let err = security::security_bprm_check(path_bytes);
    if err != 0 {
        return Err(err);
    }
    Ok(())
}

fn measure_exec_program(program: &LoadedProgram) -> usize {
    let mut measured = 0;
    if security::integrity::ima::measure_file_for_hook(
        security::integrity::ima::ImaHook::BprmCheck,
        &program.main.path,
        &program.main.bytes,
    )
    .unwrap_or(false)
    {
        measured += 1;
    }
    if let Some(interp) = program.interp.as_ref() {
        if security::integrity::ima::measure_file_for_hook(
            security::integrity::ima::ImaHook::BprmCheck,
            &interp.path,
            &interp.bytes,
        )
        .unwrap_or(false)
        {
            measured += 1;
        }
    }
    measured
}

pub unsafe fn sys_execve(
    filename: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> i64 {
    let path = match unsafe { copy_user_cstr(filename) } {
        Ok(p) => match resolve_exec_path_for_load(&p) {
            Ok(path) => path,
            Err(e) => return e as i64,
        },
        Err(e) => return e as i64,
    };
    let mut argv_vec = match unsafe { copy_user_cstr_array(argv) } {
        Ok(v) => v,
        Err(e) => {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-exec-err path={} errno={}",
                    path,
                    e
                );
            }
            return e as i64;
        }
    };
    if argv_vec.is_empty() {
        argv_vec.push(path.clone());
    }
    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { crate::kernel::sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-exec-enter pid={} path={}",
            pid,
            path
        );
    }
    let exec_path = match read_shebang_spec(&path) {
        Ok(Some(spec)) => {
            argv_vec = rewrite_argv_for_shebang(&path, &argv_vec, &spec);
            normalize_exec_path(&spec.interpreter)
        }
        Ok(None) => path.clone(),
        Err(e) => {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-exec-err path={} errno={}",
                    path,
                    e
                );
            }
            return e as i64;
        }
    };
    let env_vec = match unsafe { copy_user_cstr_array(envp) } {
        Ok(v) => v,
        Err(e) => {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-exec-err path={} errno={}",
                    path,
                    e
                );
            }
            return e as i64;
        }
    };

    let program = match load_program(&exec_path) {
        Ok(p) => p,
        Err(e) => {
            #[cfg(not(test))]
            if crate::kernel::debug_trace::proc_enabled() {
                crate::linux_driver_abi::tty::serial_println!(
                    "trace-proc-exec-err path={} exec_path={} errno={}",
                    path,
                    exec_path,
                    e
                );
            }
            return e as i64;
        }
    };
    if let Err(e) = prepare_exec_security(&path) {
        #[cfg(not(test))]
        if crate::kernel::debug_trace::proc_enabled() {
            crate::linux_driver_abi::tty::serial_println!(
                "trace-proc-exec-err path={} errno={}",
                path,
                e
            );
        }
        return e as i64;
    }
    measure_exec_program(&program);

    #[cfg(not(test))]
    if crate::kernel::debug_trace::proc_enabled() {
        let task = unsafe { crate::kernel::sched::get_current() };
        let pid = if task.is_null() {
            -1
        } else {
            unsafe { (*task).pid }
        };
        let has_xdg_runtime_dir = env_vec
            .iter()
            .any(|entry| entry.starts_with("XDG_RUNTIME_DIR="));
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-exec-env pid={} path={} argc={} envc={} xdg_runtime_dir={}",
            pid,
            path,
            argv_vec.len(),
            env_vec.len(),
            has_xdg_runtime_dir
        );
        crate::linux_driver_abi::tty::serial_println!(
            "trace-proc-exec-commit path={} exec_path={}",
            path,
            exec_path
        );
    }
    let proposed_creds = match prepare_exec_creds(&program) {
        Ok(creds) => creds,
        Err(e) => return e as i64,
    };
    match unsafe { commit_exec_for_current(&path, &argv_vec, &env_vec, &program, proposed_creds) } {
        Ok(_) => {
            #[cfg(not(test))]
            trace_ping_exec_commit(&path, &exec_path);
            0
        }
        Err(e) => {
            crate::kernel::printk::log_error!("exec", "execve: commit failed {} errno={}", path, e);
            e as i64
        }
    }
}

pub unsafe fn sys_execveat(
    dirfd: i32,
    filename: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
    flags: i32,
) -> i64 {
    let allowed = (crate::include::uapi::fcntl::AT_EMPTY_PATH
        | crate::include::uapi::fcntl::AT_SYMLINK_NOFOLLOW) as i32;
    if flags & !allowed != 0 {
        return -22;
    }
    let raw_path = match unsafe { copy_user_cstr(filename) } {
        Ok(path) => path,
        Err(e) => return e as i64,
    };
    let path = if raw_path.is_empty() {
        if flags & crate::include::uapi::fcntl::AT_EMPTY_PATH as i32 == 0 {
            return -2;
        }
        match crate::fs::proc::fd::current_fd_path(dirfd) {
            Ok(path) => normalize_exec_path(&path),
            Err(errno) => return -(errno as i64),
        }
    } else if raw_path.starts_with('/') || dirfd == crate::include::uapi::fcntl::AT_FDCWD {
        match resolve_exec_path_for_load(&raw_path) {
            Ok(path) => path,
            Err(e) => return e as i64,
        }
    } else {
        let dir = match crate::fs::proc::fd::current_fd_path(dirfd) {
            Ok(path) => path,
            Err(errno) => return -(errno as i64),
        };
        let mut joined = normalize_exec_path(&dir);
        if !joined.ends_with('/') {
            joined.push('/');
        }
        joined.push_str(&raw_path);
        match resolve_exec_path_for_load(&joined) {
            Ok(path) => path,
            Err(e) => return e as i64,
        }
    };

    let path_cstr = path.as_bytes();
    let mut nul_path = Vec::with_capacity(path_cstr.len() + 1);
    nul_path.extend_from_slice(path_cstr);
    nul_path.push(0);
    unsafe { sys_execve(nul_path.as_ptr() as *const c_char, argv, envp) }
}

/// Kernel-internal `execve` helper for early PID1 bring-up.
///
/// This bypasses user-pointer copying and is intended for boot code that wants
/// to hand off to `/sbin/init` by reusing the same ELF loader and credential
/// hooks as the syscall path.
pub fn execve_from_kernel(
    path: &str,
    argv: &[&str],
    envp: &[&str],
) -> Result<UserStartContext, i32> {
    let mut argv_vec = argv.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    if argv_vec.is_empty() {
        argv_vec.push(path.to_string());
    }
    let env_vec = envp.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();

    let exec_path = if let Some(spec) = read_shebang_spec(path)? {
        argv_vec = rewrite_argv_for_shebang(path, &argv_vec, &spec);
        normalize_exec_path(&spec.interpreter)
    } else {
        path.to_string()
    };
    let program = load_program(&exec_path)?;
    prepare_exec_security(path)?;
    measure_exec_program(&program);
    let proposed_creds = prepare_exec_creds(&program)?;
    let ctx =
        unsafe { commit_exec_for_current(path, &argv_vec, &env_vec, &program, proposed_creds) }?;
    // Kernel callers enter userspace directly with the returned context, not
    // through the syscall exit path.  Drop the pending syscall-return handoff
    // so a later successful user syscall cannot restart the new image.
    let _ = take_exec_start_for_current();
    Ok(ctx)
}

pub unsafe fn user_write(mm: *mut MmStruct, mut addr: u64, mut data: &[u8]) -> Result<(), i32> {
    while !data.is_empty() {
        let dst = match unsafe { ensure_user_ptr(mm, addr, true) } {
            Ok(dst) => dst,
            Err(err) => {
                crate::kernel::printk::log_error!(
                    "exec",
                    "user_write: ensure failed addr={:#x} len={} err={}",
                    addr,
                    data.len(),
                    err
                );
                return Err(err);
            }
        };
        let page_off = (addr as usize) & (PAGE_SIZE - 1);
        let chunk = core::cmp::min(PAGE_SIZE - page_off, data.len());
        unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), dst, chunk) };
        addr += chunk as u64;
        data = &data[chunk..];
    }
    Ok(())
}

pub unsafe fn user_read(mm: *mut MmStruct, mut addr: u64, mut dst: &mut [u8]) -> Result<(), i32> {
    while !dst.is_empty() {
        let src = unsafe { ensure_user_ptr(mm, addr, false)? };
        let page_off = (addr as usize) & (PAGE_SIZE - 1);
        let chunk = core::cmp::min(PAGE_SIZE - page_off, dst.len());
        unsafe { core::ptr::copy_nonoverlapping(src as *const u8, dst.as_mut_ptr(), chunk) };
        addr += chunk as u64;
        let (_, rest) = dst.split_at_mut(chunk);
        dst = rest;
    }
    Ok(())
}

pub unsafe fn user_write_u64(mm: *mut MmStruct, addr: u64, val: u64) -> Result<(), i32> {
    unsafe { user_write(mm, addr, &val.to_ne_bytes()) }
}

pub unsafe fn user_read_u64(mm: *mut MmStruct, addr: u64) -> Result<u64, i32> {
    let mut buf = [0u8; 8];
    unsafe { user_read(mm, addr, &mut buf)? };
    Ok(u64::from_ne_bytes(buf))
}

unsafe fn ensure_user_ptr(mm: *mut MmStruct, addr: u64, write: bool) -> Result<*mut u8, i32> {
    if let Some(p) = unsafe { translate_user_addr(mm, addr, write) } {
        return Ok(p);
    }
    if write {
        let Some(vma) = find_vma(unsafe { &*mm }, addr) else {
            crate::kernel::printk::log_error!(
                "exec",
                "ensure_user_ptr: no VMA for write addr={:#x}",
                addr
            );
            return Err(-14); // EFAULT
        };
        let vma_ref = unsafe { &*vma };
        if addr < vma_ref.vm_start || addr >= vma_ref.vm_end {
            crate::kernel::printk::log_error!(
                "exec",
                "ensure_user_ptr: VMA miss addr={:#x} vma=[{:#x},{:#x}) flags={:#x}",
                addr,
                vma_ref.vm_start,
                vma_ref.vm_end,
                vma_ref.vm_flags
            );
            return Err(-14);
        }
        let flags = FAULT_FLAG_USER | FAULT_FLAG_WRITE;
        let fault = handle_mm_fault(vma, addr, flags);
        if (fault & VM_FAULT_ERROR) != 0 {
            crate::kernel::printk::log_error!(
                "exec",
                "ensure_user_ptr: fault failed addr={:#x} vma=[{:#x},{:#x}) flags={:#x} fault={:#x}",
                addr,
                vma_ref.vm_start,
                vma_ref.vm_end,
                vma_ref.vm_flags,
                fault
            );
            return Err(-14);
        }
        if let Some(p) = unsafe { translate_user_addr(mm, addr, true) } {
            return Ok(p);
        }
        crate::kernel::printk::log_error!(
            "exec",
            "ensure_user_ptr: translation missing after fault addr={:#x} vma=[{:#x},{:#x}) flags={:#x}",
            addr,
            vma_ref.vm_start,
            vma_ref.vm_end,
            vma_ref.vm_flags
        );
    }
    Err(-14)
}

unsafe fn translate_user_addr(mm: *mut MmStruct, addr: u64, write: bool) -> Option<*mut u8> {
    let pgd = unsafe { (*mm).pgd as *mut pgd_t };
    if pgd.is_null() {
        return None;
    }
    let pgdp = unsafe { pgd_offset_pgd(pgd, addr) };
    if unsafe { pgd_none(*pgdp) } {
        return None;
    }
    let p4dp = unsafe { p4d_offset(pgdp, addr) };
    let pudp = unsafe { pud_offset(p4dp, addr) };
    if unsafe { pud_none(*pudp) || pud_huge(*pudp) } {
        return None;
    }
    let pmdp = unsafe { pmd_offset(pudp, addr) };
    if unsafe { pmd_none(*pmdp) || pmd_huge(*pmdp) } {
        return None;
    }
    let ptep = unsafe { pte_offset_kernel(pmdp, addr) };
    let pte: pte_t = unsafe { *ptep };
    if !pte_present(pte) {
        return None;
    }
    if write && !pte_write(pte) {
        return None;
    }
    let phys = pte_phys(pte) + (addr & (PAGE_SIZE as u64 - 1));
    Some(crate::arch::x86::mm::paging::phys_to_virt(phys))
}

fn read_u16(buf: &[u8], off: usize) -> Result<u16, i32> {
    let slice = buf.get(off..off + 2).ok_or(-8)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(buf: &[u8], off: usize) -> Result<u32, i32> {
    let slice = buf.get(off..off + 4).ok_or(-8)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(buf: &[u8], off: usize) -> Result<u64, i32> {
    let slice = buf.get(off..off + 8).ok_or(-8)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn read_i64(buf: &[u8], off: usize) -> Result<i64, i32> {
    let slice = buf.get(off..off + 8).ok_or(-8)?;
    Ok(i64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use alloc::vec;
    use std::sync::Mutex;

    use crate::security::hooks::{LsmHooks, NOOP_HOOKS};
    use crate::security::lsm_list::{TEST_LSM_LOCK, reset_for_test};
    use crate::security::register_lsm;

    static EXEC_HOOK_LOG: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

    fn test_bprm_creds_for_exec(_filename: &[u8]) -> i32 {
        EXEC_HOOK_LOG.lock().unwrap().push("creds");
        0
    }

    fn test_bprm_check(_filename: &[u8]) -> i32 {
        EXEC_HOOK_LOG.lock().unwrap().push("check");
        0
    }

    fn test_bprm_committing_creds(_filename: &[u8]) {
        EXEC_HOOK_LOG.lock().unwrap().push("committing");
    }

    fn test_bprm_committed_creds(_filename: &[u8]) {
        EXEC_HOOK_LOG.lock().unwrap().push("committed");
    }

    #[test]
    fn user_start_context_layout_matches_enter_userspace_asm() {
        assert_eq!(core::mem::offset_of!(UserStartContext, ip), 0);
        assert_eq!(core::mem::offset_of!(UserStartContext, sp), 8);
        assert_eq!(core::mem::offset_of!(UserStartContext, rflags), 16);
        assert_eq!(core::mem::offset_of!(UserStartContext, old_mm), 24);
        assert_eq!(core::mem::size_of::<UserStartContext>(), 32);
    }

    #[test]
    fn copy_user_cstr_array_preserves_large_systemd_environment() {
        let strings = (0..300)
            .map(|idx| std::format!("VAR_{idx}=value\0").into_bytes())
            .collect::<Vec<_>>();
        let mut ptrs = strings
            .iter()
            .map(|entry| entry.as_ptr() as *const c_char)
            .collect::<Vec<_>>();
        ptrs.push(core::ptr::null());

        let copied = unsafe { copy_user_cstr_array(ptrs.as_ptr()) }.expect("copy envp");

        assert_eq!(copied.len(), 300);
        assert_eq!(copied[299], "VAR_299=value");
    }

    #[test]
    fn copy_user_cstr_array_rejects_unterminated_vectors() {
        let strings = (0..MAX_ARG_COUNT)
            .map(|idx| std::format!("VAR_{idx}=value\0").into_bytes())
            .collect::<Vec<_>>();
        let ptrs = strings
            .iter()
            .map(|entry| entry.as_ptr() as *const c_char)
            .collect::<Vec<_>>();

        let err = unsafe { copy_user_cstr_array(ptrs.as_ptr()) }.expect_err("unterminated envp");

        assert_eq!(err, -7);
    }

    fn tiny_elf(interp: Option<&str>) -> Vec<u8> {
        let phnum = if interp.is_some() { 2u16 } else { 1u16 };
        let phoff = 64u64;
        let phentsize = 56u16;
        let interp_off = 64 + (phnum as usize * 56);
        let mut out = vec![0u8; interp_off + 64];

        out[0..4].copy_from_slice(ELF_MAGIC);
        out[4] = ELFCLASS64;
        out[5] = ELFDATA2LSB;
        out[6] = EV_CURRENT;
        out[16..18].copy_from_slice(&ET_DYN.to_le_bytes());
        out[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        out[24..32].copy_from_slice(&0x401000u64.to_le_bytes());
        out[32..40].copy_from_slice(&phoff.to_le_bytes());
        out[54..56].copy_from_slice(&phentsize.to_le_bytes());
        out[56..58].copy_from_slice(&phnum.to_le_bytes());

        let p0 = 64usize;
        out[p0..p0 + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
        out[p0 + 4..p0 + 8].copy_from_slice(&(PF_R | PF_X).to_le_bytes());
        out[p0 + 8..p0 + 16].copy_from_slice(&0u64.to_le_bytes());
        out[p0 + 16..p0 + 24].copy_from_slice(&0x400000u64.to_le_bytes());
        out[p0 + 32..p0 + 40].copy_from_slice(&4096u64.to_le_bytes());
        out[p0 + 40..p0 + 48].copy_from_slice(&4096u64.to_le_bytes());

        if let Some(path) = interp {
            let p1 = 120usize;
            let text = std::format!("{path}\0");
            out[p1..p1 + 4].copy_from_slice(&PT_INTERP.to_le_bytes());
            out[p1 + 8..p1 + 16].copy_from_slice(&(interp_off as u64).to_le_bytes());
            out[p1 + 32..p1 + 40].copy_from_slice(&(text.len() as u64).to_le_bytes());
            out[p1 + 40..p1 + 48].copy_from_slice(&(text.len() as u64).to_le_bytes());
            out[interp_off..interp_off + text.len()].copy_from_slice(text.as_bytes());
        }
        out
    }

    fn test_loaded_image(path: &str, elf: ElfImage, bytes: Vec<u8>) -> LoadedImage {
        let dentry = crate::fs::dcache::d_alloc(path.rsplit('/').next().unwrap_or(path));
        let inode = crate::fs::types::Inode::new(
            1,
            crate::fs::types::InodeKind::Regular,
            0o755,
            &crate::fs::ops::NOOP_INODE_OPS,
            &crate::fs::ops::NOOP_FILE_OPS,
            crate::fs::types::InodePrivate::RamBytes(spin::Mutex::new(bytes.clone())),
        );
        dentry.instantiate(inode.clone());
        let root = crate::fs::dcache::d_alloc("/");
        let sb =
            crate::fs::types::SuperBlock::alloc("exec-test", 0, &crate::fs::ops::NOOP_SUPER_OPS);
        let mount = crate::fs::mount::Mount::alloc(sb, root, 0);
        LoadedImage {
            path: path.to_string(),
            elf,
            bytes,
            inode,
            dentry,
            mount,
            from_script: false,
        }
    }

    fn test_security(
        uid: u32,
        euid: u32,
        gid: u32,
        egid: u32,
        secure_exec: bool,
    ) -> ExecSecurityContext {
        ExecSecurityContext {
            uid,
            euid,
            gid,
            egid,
            secure_exec,
        }
    }

    fn test_setid_program(
        mode: u32,
        uid: u32,
        gid: u32,
        mount_flags: u32,
        from_script: bool,
    ) -> LoadedProgram {
        let bytes = tiny_elf(None);
        let elf = parse_elf_image(&bytes).expect("parse");
        let mut image = test_loaded_image("/usr/bin/sudo", elf, bytes);
        image.inode.mode.store(mode, Ordering::Release);
        image.inode.uid.store(uid, Ordering::Release);
        image.inode.gid.store(gid, Ordering::Release);
        image.mount.flags.store(mount_flags, Ordering::Release);
        image.from_script = from_script;
        LoadedProgram {
            main: image,
            interp: None,
        }
    }

    fn test_nonroot_cred(uid: u32, gid: u32) -> Box<Cred> {
        Box::new(Cred {
            usage: core::sync::atomic::AtomicUsize::new(1),
            uid: KUid(uid),
            gid: KGid(gid),
            suid: KUid(uid),
            sgid: KGid(gid),
            euid: KUid(uid),
            egid: KGid(gid),
            fsuid: KUid(uid),
            fsgid: KGid(gid),
            cap_inheritable: KernelCapT::empty(),
            cap_permitted: KernelCapT::empty(),
            cap_effective: KernelCapT::empty(),
            cap_bset: KernelCapT::full(),
            cap_ambient: KernelCapT::empty(),
            securebits: 0,
            group_info: crate::kernel::cred::GroupInfo::default(),
            user_ns: core::ptr::null(),
        })
    }

    fn install_test_task<'a>(
        cred: &'a Cred,
        no_new_privs: u8,
    ) -> (Box<TaskStruct>, *mut TaskStruct) {
        let mut task: Box<TaskStruct> = unsafe { Box::new(core::mem::zeroed()) };
        task.cred = cred as *const Cred;
        task.m27.real_cred = cred as *const Cred;
        task.m27.no_new_privs = no_new_privs;
        let previous = unsafe { sched::get_current() };
        unsafe { sched::set_current(&mut *task) };
        (task, previous)
    }

    #[test]
    fn exec_creds_apply_root_owned_04755_sudo() {
        let old = test_nonroot_cred(1000, 1000);
        let (_task, previous) = install_test_task(&old, 0);
        let program = test_setid_program(0o4755, 0, 0, 0, false);
        let proposed = prepare_exec_creds(&program).expect("prepare exec creds");

        assert_eq!(
            proposed.security(),
            test_security(1000, 0, 1000, 1000, true)
        );
        unsafe {
            assert_eq!((*proposed.cred).euid, KUid(0));
            assert_eq!((*proposed.cred).suid, KUid(0));
        }
        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn exec_creds_ignore_setuid_on_nosuid_mount_and_enable_secure_mode() {
        let old = test_nonroot_cred(1000, 1000);
        let (_task, previous) = install_test_task(&old, 0);
        let program = test_setid_program(0o4755, 123, 0, MS_NOSUID as u32, false);
        let proposed = prepare_exec_creds(&program).expect("prepare exec creds");

        assert_eq!(proposed.security().euid, 1000);
        assert!(proposed.security().secure_exec);
        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn exec_creds_ignore_setuid_when_no_new_privs_is_set() {
        let old = test_nonroot_cred(1000, 1000);
        let (_task, previous) = install_test_task(&old, 1);

        let program = test_setid_program(0o4755, 123, 0, 0, false);
        let proposed = prepare_exec_creds(&program).expect("prepare exec creds");

        assert_eq!(proposed.security().euid, 1000);
        assert!(proposed.security().secure_exec);
        unsafe { sched::set_current(previous) };
    }

    #[test]
    fn parse_elf_extracts_loads_and_interp() {
        let elf = tiny_elf(Some("/lib/ld-musl-x86_64.so.1"));
        let parsed = parse_elf_image(&elf).expect("parse");
        assert!(parsed.et_dyn);
        assert_eq!(parsed.load_segments.len(), 1);
        assert_eq!(parsed.interp.as_deref(), Some("/lib/ld-musl-x86_64.so.1"));
    }

    #[test]
    fn shebang_parser_returns_interpreter() {
        let bytes = b"#!/bin/sh -e\nexit 0\n";
        let interp = parse_shebang_interpreter(bytes).expect("ok");
        assert_eq!(
            interp,
            Some(ShebangSpec {
                interpreter: "/bin/sh".to_string(),
                arg: Some("-e".to_string()),
            })
        );
    }

    #[test]
    fn shebang_parser_ignores_non_script() {
        let interp = parse_shebang_interpreter(b"ELF...").expect("ok");
        assert!(interp.is_none());
    }

    const EXEC_OPAQUE_TEST_BYTES: &[u8] = b"opaque-vfs-exec-image";

    fn exec_opaque_test_read(
        _file: &crate::fs::types::FileRef,
        buf: &mut [u8],
        pos: &mut u64,
    ) -> Result<usize, i32> {
        let start = *pos as usize;
        if start >= EXEC_OPAQUE_TEST_BYTES.len() {
            return Ok(0);
        }
        let end = (start + buf.len()).min(EXEC_OPAQUE_TEST_BYTES.len());
        let n = end - start;
        buf[..n].copy_from_slice(&EXEC_OPAQUE_TEST_BYTES[start..end]);
        *pos += n as u64;
        Ok(n)
    }

    static EXEC_OPAQUE_TEST_FOPS: crate::fs::ops::FileOps = crate::fs::ops::FileOps {
        name: "exec-opaque-test",
        read: Some(exec_opaque_test_read),
        write: None,
        llseek: None,
        fsync: None,
        poll: None,
        ioctl: None,
        mmap: None,
        release: None,
        readdir: None,
    };

    #[test]
    fn read_exec_file_falls_back_to_vfs_for_opaque_regular_inode() {
        let dentry = crate::fs::dcache::d_alloc("exec-opaque");
        let inode = crate::fs::types::Inode::new(
            99,
            crate::fs::types::InodeKind::Regular,
            0o755,
            &crate::fs::ops::NOOP_INODE_OPS,
            &EXEC_OPAQUE_TEST_FOPS,
            crate::fs::types::InodePrivate::Opaque(99),
        );
        inode
            .size
            .store(EXEC_OPAQUE_TEST_BYTES.len() as u64, Ordering::Release);
        dentry.instantiate(inode.clone());

        let bytes = read_regular_inode_bytes("/sbin/init", dentry, inode).expect("vfs read");

        assert_eq!(bytes, EXEC_OPAQUE_TEST_BYTES);
    }

    #[test]
    fn shebang_rewrites_argv_for_interpreter() {
        let spec = ShebangSpec {
            interpreter: "/bin/bash".to_string(),
            arg: Some("-e".to_string()),
        };
        let argv = vec![
            "/opt/demo.sh".to_string(),
            "alpha".to_string(),
            "beta".to_string(),
        ];
        let rewritten = rewrite_argv_for_shebang("/opt/demo.sh", &argv, &spec);
        assert_eq!(
            rewritten,
            vec![
                "/bin/bash".to_string(),
                "-e".to_string(),
                "/opt/demo.sh".to_string(),
                "alpha".to_string(),
                "beta".to_string(),
            ]
        );
    }

    #[test]
    fn build_auxv_omits_sysinfo_ehdr_when_vdso_is_not_published() {
        let elf = tiny_elf(None);
        let parsed = parse_elf_image(&elf).expect("parse");
        let aux = build_auxv(
            &parsed,
            0x400000,
            0,
            0x401000,
            0x7fff_1000,
            0x7fff_2000,
            0,
            test_security(1000, 1000, 100, 100, false),
        );
        let mut keys = aux.iter().map(|(k, _)| *k).collect::<Vec<_>>();
        keys.sort_unstable();
        assert!(keys.contains(&AT_PHDR));
        assert!(keys.contains(&AT_PHENT));
        assert!(keys.contains(&AT_PHNUM));
        assert!(keys.contains(&AT_RANDOM));
        assert!(keys.contains(&AT_EXECFN));
        assert!(keys.contains(&AT_ENTRY));
        assert!(!keys.contains(&AT_SYSINFO_EHDR));
    }

    #[test]
    fn build_auxv_includes_sysinfo_ehdr_when_vdso_is_published() {
        let elf = tiny_elf(None);
        let parsed = parse_elf_image(&elf).expect("parse");
        let aux = build_auxv(
            &parsed,
            0x400000,
            0,
            0x401000,
            0x7fff_1000,
            0x7fff_2000,
            0x7fff_f000,
            test_security(1000, 1000, 100, 100, true),
        );
        assert!(aux.contains(&(AT_SYSINFO_EHDR, 0x7fff_f000)));
    }

    #[test]
    fn exec_layout_with_pt_interp_uses_interpreter_bias_as_at_base() {
        let main_bytes = tiny_elf(Some("/lib64/ld-linux-x86-64.so.2"));
        let main_elf = parse_elf_image(&main_bytes).expect("main parse");
        let interp_bytes = tiny_elf(None);
        let interp_elf = parse_elf_image(&interp_bytes).expect("interp parse");
        let program = LoadedProgram {
            main: test_loaded_image("/sbin/init", main_elf.clone(), main_bytes),
            interp: Some(test_loaded_image(
                "/lib64/ld-linux-x86-64.so.2",
                interp_elf.clone(),
                interp_bytes,
            )),
        };

        let layout = exec_load_layout(&program).expect("layout");

        assert_eq!(layout.main_bias, PIE_LOAD_BIAS);
        assert_eq!(layout.interp_bias, INTERP_LOAD_BIAS);
        assert_eq!(layout.at_base, INTERP_LOAD_BIAS);
        assert_eq!(layout.entry_ip, INTERP_LOAD_BIAS + interp_elf.entry);
        assert_eq!(layout.at_entry, PIE_LOAD_BIAS + main_elf.entry);
    }

    #[test]
    fn exec_plan_leaves_pt_interp_relocations_to_ld_so() {
        let main_bytes = tiny_elf(Some("/lib64/ld-linux-x86-64.so.2"));
        let main_elf = parse_elf_image(&main_bytes).expect("main parse");
        let interp_bytes = tiny_elf(None);
        let interp_elf = parse_elf_image(&interp_bytes).expect("interp parse");
        let program = LoadedProgram {
            main: test_loaded_image("/sbin/init", main_elf, main_bytes),
            interp: Some(test_loaded_image(
                "/lib64/ld-linux-x86-64.so.2",
                interp_elf,
                interp_bytes,
            )),
        };

        let plan = exec_relocation_plan(&program);

        assert!(!plan.relocate_main_relative);
    }

    #[test]
    fn exec_plan_leaves_no_interp_et_dyn_relocations_to_static_pie_startup() {
        let main_bytes = tiny_elf(None);
        let main_elf = parse_elf_image(&main_bytes).expect("main parse");
        let program = LoadedProgram {
            main: test_loaded_image("/sbin/init", main_elf, main_bytes),
            interp: None,
        };

        let plan = exec_relocation_plan(&program);

        assert!(!plan.relocate_main_relative);
    }

    fn stage_window_for_test(data: &[u8], window: ElfLoadWindow) -> Vec<u8> {
        let mut mapped = vec![0xa5; window.map_len as usize];
        if window.file_len > 0 {
            let file_end = window.file_offset + window.file_len;
            mapped[..window.file_len].copy_from_slice(&data[window.file_offset..file_end]);
        }
        if window.zero_len > 0 {
            let zero_start = (window.zero_start - window.map_start) as usize;
            let zero_end = zero_start + window.zero_len as usize;
            mapped[zero_start..zero_end].fill(0);
        }
        mapped
    }

    #[test]
    fn elf_load_window_copies_page_prefix_payload_and_zeroes_tail() {
        let mut data = vec![0u8; 0x4000];
        for (idx, byte) in data.iter_mut().enumerate() {
            *byte = (idx & 0xff) as u8;
        }
        let seg = ElfLoadSegment {
            vaddr: 0x1234,
            memsz: 0x80,
            filesz: 0x30,
            flags: PF_R,
            offset: 0x3234,
        };

        let window = elf_load_window(&seg, 0x1_0000)
            .expect("window")
            .expect("mapped segment");
        let mapped = stage_window_for_test(&data, window);
        let payload_offset = (0x1_0000 + seg.vaddr - window.map_start) as usize;

        assert_eq!(window.map_start, 0x1_1000);
        assert_eq!(window.map_len, PAGE_SIZE as u64);
        assert_eq!(window.file_offset, 0x3000);
        assert_eq!(window.file_len, 0x234 + 0x30);
        assert_eq!(&mapped[..payload_offset], &data[0x3000..0x3234]);
        assert_eq!(
            &mapped[payload_offset..payload_offset + seg.filesz as usize],
            &data[0x3234..0x3264]
        );
        assert!(
            mapped[payload_offset + seg.filesz as usize..]
                .iter()
                .all(|byte| *byte == 0)
        );
    }

    fn put_u64(buf: &mut [u8], off: usize, val: u64) {
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }

    fn put_i64(buf: &mut [u8], off: usize, val: i64) {
        buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }

    fn dynamic_test_image(data_len: usize) -> ElfImage {
        ElfImage {
            entry: 0,
            et_dyn: true,
            phoff: 64,
            phentsize: 56,
            phnum: 1,
            interp: None,
            dynamic: Some(ElfDynamicSegment {
                offset: 0x80,
                vaddr: 0x3000,
                filesz: 0x80,
            }),
            load_segments: vec![ElfLoadSegment {
                vaddr: 0,
                memsz: data_len as u64,
                filesz: data_len as u64,
                flags: PF_R,
                offset: 0,
            }],
        }
    }

    #[test]
    fn dynamic_relocation_parser_tracks_rela_and_relr_tables() {
        let mut data = vec![0u8; 0x200];
        put_i64(&mut data, 0x80, DT_RELA);
        put_u64(&mut data, 0x88, 0x180);
        put_i64(&mut data, 0x90, DT_RELASZ);
        put_u64(&mut data, 0x98, 24);
        put_i64(&mut data, 0xa0, DT_RELAENT);
        put_u64(&mut data, 0xa8, 24);
        put_i64(&mut data, 0xb0, DT_RELR);
        put_u64(&mut data, 0xb8, 0x100);
        put_i64(&mut data, 0xc0, DT_RELRSZ);
        put_u64(&mut data, 0xc8, 24);
        put_i64(&mut data, 0xd0, DT_RELRENT);
        put_u64(&mut data, 0xd8, 8);
        put_i64(&mut data, 0xe0, DT_NULL);

        let image = dynamic_test_image(data.len());
        let relocs = elf_dynamic_relocations(&image, &data)
            .expect("dynamic parse")
            .expect("dynamic present");

        assert_eq!(relocs.rela_vaddr, 0x180);
        assert_eq!(relocs.rela_size, 24);
        assert_eq!(relocs.rela_ent, 24);
        assert_eq!(relocs.relr_vaddr, 0x100);
        assert_eq!(relocs.relr_size, 24);
        assert_eq!(relocs.relr_ent, 8);
    }

    #[test]
    fn relr_walker_decodes_address_entries_and_bitmaps() {
        let mut data = vec![0u8; 0x300];
        put_u64(&mut data, 0x100, 0x200);
        put_u64(&mut data, 0x108, 0b1011);
        put_u64(&mut data, 0x110, 0x280);
        let image = dynamic_test_image(data.len());
        let relocs = ElfDynamicRelocations {
            relr_vaddr: 0x100,
            relr_size: 24,
            relr_ent: 8,
            ..ElfDynamicRelocations::default()
        };
        let mut targets = Vec::new();

        walk_elf_relr_relocations(&image, &data, &relocs, |target| {
            targets.push(target);
            Ok(())
        })
        .expect("walk RELR");

        assert_eq!(targets, vec![0x200, 0x208, 0x218, 0x280]);
    }

    #[test]
    fn exec_pgd_template_copy_uses_supplied_init_root() {
        let mut init = [pgd_t(0); 512];
        let mut active_user = [pgd_t(0); 512];
        let mut dst = [pgd_t(0); 512];

        init[0] = pgd_t(0x1000_023);
        init[256] = pgd_t(0x2000_023);
        init[511] = pgd_t(0x3000_003);
        init[255] = pgd_t(0xbeef_067);
        active_user[170] = pgd_t(0xdead_beef_067);

        unsafe { copy_exec_kernel_pgd_entries(dst.as_mut_ptr(), init.as_ptr()) }
            .expect("copy exec PGD template");

        assert_eq!(dst[0], init[0]);
        assert_eq!(dst[256], init[256]);
        assert_eq!(dst[511], init[511]);
        assert_eq!(dst[170], pgd_t(0));
        assert_eq!(dst[255], pgd_t(0));
        assert_ne!(dst[170], active_user[170]);
    }

    #[test]
    fn exec_security_prepare_runs_bprm_hooks_in_linux_order() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        EXEC_HOOK_LOG.lock().unwrap().clear();
        register_lsm(LsmHooks {
            name: "test_exec_prepare",
            bprm_creds_for_exec: Some(test_bprm_creds_for_exec),
            bprm_check: Some(test_bprm_check),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        prepare_exec_security("/bin/test").expect("prepare_exec_security");
        assert_eq!(&*EXEC_HOOK_LOG.lock().unwrap(), &["creds", "check"]);
    }

    #[test]
    fn ima_exec_measurement_records_loaded_images() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        crate::security::integrity::ima::reset_for_test();
        crate::security::integrity::ima::init();

        let main_bytes = tiny_elf(Some("/lib64/ld-linux-x86-64.so.2"));
        let main_elf = parse_elf_image(&main_bytes).expect("main parse");
        let interp_bytes = tiny_elf(None);
        let interp_elf = parse_elf_image(&interp_bytes).expect("interp parse");
        let program = LoadedProgram {
            main: test_loaded_image("/bin/app", main_elf, main_bytes),
            interp: Some(test_loaded_image(
                "/lib64/ld-linux-x86-64.so.2",
                interp_elf,
                interp_bytes,
            )),
        };

        assert_eq!(measure_exec_program(&program), 2);
        assert_eq!(measure_exec_program(&program), 0);

        let ascii = crate::security::integrity::ima::ascii_runtime_measurements_sha1();
        assert!(ascii.contains("boot_aggregate"));
        assert!(ascii.contains("/bin/app"));
        assert!(ascii.contains("/lib64/ld-linux-x86-64.so.2"));
    }

    #[test]
    fn exec_security_commit_runs_notification_hooks() {
        let _guard = TEST_LSM_LOCK.lock();
        reset_for_test();
        EXEC_HOOK_LOG.lock().unwrap().clear();
        register_lsm(LsmHooks {
            name: "test_exec_commit",
            bprm_committing_creds: Some(test_bprm_committing_creds),
            bprm_committed_creds: Some(test_bprm_committed_creds),
            ..NOOP_HOOKS
        })
        .expect("register_lsm");

        security::security_bprm_committing_creds(b"/bin/test");
        security::security_bprm_committed_creds(b"/bin/test");
        assert_eq!(
            &*EXEC_HOOK_LOG.lock().unwrap(),
            &["committing", "committed"]
        );
    }
}
