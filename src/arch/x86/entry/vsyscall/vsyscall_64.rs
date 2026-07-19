//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/entry/vsyscall/vsyscall_64.c
//! test-origin: linux:vendor/linux/arch/x86/entry/vsyscall/vsyscall_64.c
//! Legacy x86-64 vsyscall emulation.
//!
//! vsyscalls are a legacy ABI: userspace calls fixed kernel addresses in the
//! `VSYSCALL_ADDR` page. A page-fault (or, with LASS, a #GP) on those addresses
//! traps into `__emulate_vsyscall`, which decodes which of the three legacy
//! calls (`gettimeofday`, `time`, `getcpu`) was requested, runs the real
//! syscall, and emulates a `ret` back to the caller.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/vsyscall/vsyscall_64.c
//!
//! The emulation operates on the real [`PtRegs`] and is wired to the live kernel
//! through [`KernelVsyscallEnv`]: `crate::arch::x86::kernel::uaccess`
//! (`access_ok`/`get_user_u64`), `crate::kernel::signal` (`force_sig*`),
//! `crate::kernel::seccomp`, and the real `__x64_sys_{gettimeofday,time,getcpu}`
//! wrappers. `set_vsyscall_pgtable_user_bits` walks the live page tables and
//! `map_vsyscall` installs the page via `paging::map_kernel_page` (the
//! `VSYSCALL_PAGE` fixmap slot is `VSYSCALL_ADDR`, asserted via
//! `fix_to_virt_vsyscall_page`). [`VsyscallEnv`] is the host-test seam.
//!
//! lupos-native adaptations (documented, not stubs):
//! - The gate area is exposed as a `(start, end)` range instead of a static
//!   `vm_area_struct` pointer; lupos mm models the gate area that way.
//! - `__vsyscall_page`'s executable bytes live in Linux's separate
//!   `vsyscall_emu_64.S`; this file only references the symbol (page placeholder).

use crate::arch::x86::kernel::ptrace::PtRegs;
use crate::include::uapi::errno::{EFAULT, EINVAL, ENOSYS};
use core::sync::atomic::{AtomicU8, Ordering};

/// Fixed virtual address of the 64-bit vsyscall page.
/// Ref: vendor/linux/arch/x86/include/asm/vsyscall.h
pub const VSYSCALL_ADDR: u64 = 0xffff_ffff_ff60_0000;
pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_MASK: u64 = !(PAGE_SIZE - 1);

// Page-fault error-code bits (vendor/linux/arch/x86/include/asm/trap_pf.h).
pub const X86_PF_WRITE: u64 = 1 << 1;
pub const X86_PF_USER: u64 = 1 << 2;
pub const X86_PF_INSTR: u64 = 1 << 4;

// Signal numbers / si_code (vendor/linux/include/uapi/asm-generic/{signal,siginfo}.h).
pub const SIGSEGV: i32 = 11;
pub const SIGSYS: i32 = 31;
pub const SEGV_MAPERR: i32 = 1;

// x86-64 syscall numbers used by the three legacy vsyscalls.
// Ref: vendor/linux/arch/x86/entry/syscalls/syscall_64.tbl
pub const NR_GETTIMEOFDAY: u64 = 96;
pub const NR_TIME: u64 = 201;
pub const NR_GETCPU: u64 = 309;

/// `_PAGE_USER` — page-table U/S bit (mirrors `crate::arch::x86::mm::paging`).
pub const _PAGE_USER: u64 = 1 << 2;

/// `vsyscall=` mode. Linux defaults to `XONLY`; `EMULATE` additionally backs the
/// page with real memory, and `NONE` disables it entirely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VsyscallMode {
    Emulate,
    XOnly,
    None,
}

/// The three legacy vsyscall slots, encoded in bits [11:10] of the address.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VsyscallNumber {
    Gettimeofday = 0,
    Time = 1,
    Getcpu = 2,
}

impl VsyscallNumber {
    /// The x86-64 syscall number this legacy slot dispatches to.
    pub const fn syscall_nr(self) -> u64 {
        match self {
            VsyscallNumber::Gettimeofday => NR_GETTIMEOFDAY,
            VsyscallNumber::Time => NR_TIME,
            VsyscallNumber::Getcpu => NR_GETCPU,
        }
    }
}

/// Parse the `vsyscall=` boot parameter. Mirrors Linux `vsyscall_setup()`.
pub const fn parse_vsyscall_mode(s: &str) -> Result<VsyscallMode, i32> {
    match s.as_bytes() {
        b"emulate" => Ok(VsyscallMode::Emulate),
        b"xonly" => Ok(VsyscallMode::XOnly),
        b"none" => Ok(VsyscallMode::None),
        _ => Err(EINVAL),
    }
}

// ── global vsyscall mode ─────────────────────────────────────────────────────

const MODE_XONLY: u8 = 0;
const MODE_EMULATE: u8 = 1;
const MODE_NONE: u8 = 2;

/// Default mirrors `CONFIG_LEGACY_VSYSCALL_XONLY` (the Linux default).
static VSYSCALL_MODE: AtomicU8 = AtomicU8::new(MODE_XONLY);

const fn mode_to_u8(mode: VsyscallMode) -> u8 {
    match mode {
        VsyscallMode::XOnly => MODE_XONLY,
        VsyscallMode::Emulate => MODE_EMULATE,
        VsyscallMode::None => MODE_NONE,
    }
}

const fn mode_from_u8(v: u8) -> VsyscallMode {
    match v {
        MODE_EMULATE => VsyscallMode::Emulate,
        MODE_NONE => VsyscallMode::None,
        _ => VsyscallMode::XOnly,
    }
}

/// Current global vsyscall mode (read on the fault path).
pub fn vsyscall_mode() -> VsyscallMode {
    mode_from_u8(VSYSCALL_MODE.load(Ordering::Relaxed))
}

/// Set the global mode (boot path only).
pub fn set_vsyscall_mode(mode: VsyscallMode) {
    VSYSCALL_MODE.store(mode_to_u8(mode), Ordering::Relaxed);
}

/// `vsyscall_setup` — parse and apply the `vsyscall=` boot parameter. Returns 0
/// on success or `-EINVAL`, matching Linux's `early_param` contract.
pub fn vsyscall_setup(str: Option<&str>) -> i32 {
    match str {
        Some(s) => match parse_vsyscall_mode(s) {
            Ok(mode) => {
                set_vsyscall_mode(mode);
                0
            }
            Err(e) => -e,
        },
        None => -EINVAL,
    }
}

/// Decode a faulting address to a vsyscall slot. Mirrors `addr_to_vsyscall_nr`.
pub const fn addr_to_vsyscall_nr(addr: u64) -> Result<VsyscallNumber, i32> {
    if (addr & !0xc00) != VSYSCALL_ADDR {
        return Err(EINVAL);
    }
    match (addr & 0xc00) >> 10 {
        0 => Ok(VsyscallNumber::Gettimeofday),
        1 => Ok(VsyscallNumber::Time),
        2 => Ok(VsyscallNumber::Getcpu),
        _ => Err(EINVAL),
    }
}

/// True if `addr` lands anywhere in the vsyscall page.
pub const fn is_vsyscall_vaddr(addr: u64) -> bool {
    (addr & PAGE_MASK) == VSYSCALL_ADDR
}

/// Outcome of `__emulate_vsyscall`. `Handled` means the trap was consumed;
/// `NotVsyscall` means the fault should propagate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmulateResult {
    Handled,
    NotVsyscall,
}

/// Hardware / subsystem seam for the emulation core. [`KernelVsyscallEnv`] is the
/// production impl wired to the live kernel; tests provide a mock. All methods
/// operate on the real [`PtRegs`] so the syscalls observe the true register file
/// and user memory.
pub trait VsyscallEnv {
    fn warn_bad_vsyscall(&mut self, _level: &str, _regs: &PtRegs, _message: &str) {}
    /// `access_ok(ptr, size)` — user pointer range check.
    fn access_ok(&self, ptr: u64, size: usize) -> bool;
    /// `get_user(caller, regs->sp)` — read the return address off the user stack.
    fn get_user_caller(&mut self, sp: u64) -> Option<u64>;
    /// `secure_computing()` — 0 to continue, non-zero to skip the syscall.
    fn secure_computing(&mut self, regs: &mut PtRegs) -> i32;
    /// Run `__x64_sys_*` against the live register file; returns the result.
    fn run_vsyscall(&mut self, nr: VsyscallNumber, regs: &mut PtRegs) -> i64;
    fn force_sig(&mut self, sig: i32);
    fn force_sig_fault(&mut self, sig: i32, code: i32, addr: u64);
    fn force_exit_sig(&mut self, sig: i32);
    /// `user_64bit_mode(regs)`.
    fn user_64bit_mode(&self, regs: &PtRegs) -> bool;
    fn nx_enabled(&self) -> bool {
        true
    }
    fn lass_enabled(&self) -> bool {
        false
    }
}

/// `write_ok_or_segv` — verify a user write target, else queue SIGSEGV.
fn write_ok_or_segv<E: VsyscallEnv>(env: &mut E, ptr: u64, size: usize) -> bool {
    if !env.access_ok(ptr, size) {
        env.force_sig_fault(SIGSEGV, SEGV_MAPERR, ptr);
        false
    } else {
        true
    }
}

/// `__emulate_vsyscall` — the full emulation state machine, operating on the real
/// register frame. Returns [`EmulateResult::NotVsyscall`] only for the
/// `!user_64bit_mode` / `vsyscall=none` early-outs (matching the C `false`).
pub fn emulate_vsyscall<E: VsyscallEnv>(
    env: &mut E,
    mode: VsyscallMode,
    regs: &mut PtRegs,
    address: u64,
) -> EmulateResult {
    if !env.user_64bit_mode(regs) {
        return EmulateResult::NotVsyscall;
    }
    if mode == VsyscallMode::None {
        env.warn_bad_vsyscall("info", regs, "vsyscall attempted with vsyscall=none");
        return EmulateResult::NotVsyscall;
    }

    let vsyscall_nr = match addr_to_vsyscall_nr(address) {
        Ok(nr) => nr,
        Err(_) => {
            env.warn_bad_vsyscall("warn", regs, "misaligned vsyscall (exploit attempt?)");
            return sigsegv(env);
        }
    };

    let caller = match env.get_user_caller(regs.rsp) {
        Some(c) => c,
        None => {
            env.warn_bad_vsyscall("warn", regs, "vsyscall with bad stack (exploit attempt?)");
            return sigsegv(env);
        }
    };

    let mut ret: i64 = 0;
    let mut faulted = false;
    match vsyscall_nr {
        VsyscallNumber::Gettimeofday => {
            // sizeof(__kernel_old_timeval) == 16, sizeof(timezone) == 8.
            if !write_ok_or_segv(env, regs.rdi, 16) || !write_ok_or_segv(env, regs.rsi, 8) {
                ret = -(EFAULT as i64);
                faulted = true;
            }
        }
        VsyscallNumber::Time => {
            if !write_ok_or_segv(env, regs.rdi, 8) {
                ret = -(EFAULT as i64);
                faulted = true;
            }
        }
        VsyscallNumber::Getcpu => {
            if !write_ok_or_segv(env, regs.rdi, 4) || !write_ok_or_segv(env, regs.rsi, 4) {
                ret = -(EFAULT as i64);
                faulted = true;
            }
        }
    }

    if !faulted {
        let syscall_nr = vsyscall_nr.syscall_nr();
        regs.orig_rax = syscall_nr;
        regs.rax = (-(ENOSYS as i64)) as u64;
        let tmp = env.secure_computing(regs);
        if (tmp == 0 && regs.orig_rax != syscall_nr) || regs.rip != address {
            env.warn_bad_vsyscall("debug", regs, "seccomp tried to change syscall nr or ip");
            env.force_exit_sig(SIGSYS);
            return EmulateResult::Handled;
        }
        regs.orig_rax = u64::MAX; // -1
        if tmp != 0 {
            return do_ret(regs, caller);
        }

        ret = match vsyscall_nr {
            VsyscallNumber::Gettimeofday => env.run_vsyscall(VsyscallNumber::Gettimeofday, regs),
            VsyscallNumber::Time => env.run_vsyscall(VsyscallNumber::Time, regs),
            VsyscallNumber::Getcpu => {
                // Linux clobbers regs->dx to 0 across the call, then restores it.
                let orig_dx = regs.rdx;
                regs.rdx = 0;
                let r = env.run_vsyscall(VsyscallNumber::Getcpu, regs);
                regs.rdx = orig_dx;
                r
            }
        };
    }

    if ret == -(EFAULT as i64) {
        env.warn_bad_vsyscall("info", regs, "vsyscall fault (exploit attempt?)");
        return sigsegv(env);
    }
    regs.rax = ret as u64;
    do_ret(regs, caller)
}

/// `do_ret:` — emulate a `ret`: jump to the caller and pop the return address.
fn do_ret(regs: &mut PtRegs, caller: u64) -> EmulateResult {
    regs.rip = caller;
    regs.rsp += 8;
    EmulateResult::Handled
}

/// `sigsegv:` — deliver SIGSEGV and report the trap as handled.
fn sigsegv<E: VsyscallEnv>(env: &mut E) -> EmulateResult {
    env.force_sig(SIGSEGV);
    EmulateResult::Handled
}

/// Page-fault entry. Mirrors `emulate_vsyscall_pf`.
pub fn emulate_vsyscall_pf<E: VsyscallEnv>(
    env: &mut E,
    mode: VsyscallMode,
    error_code: u64,
    regs: &mut PtRegs,
    address: u64,
) -> bool {
    if (error_code & (X86_PF_WRITE | X86_PF_USER)) != X86_PF_USER {
        return false;
    }
    if address != regs.rip {
        if mode == VsyscallMode::Emulate {
            return false;
        }
        env.warn_bad_vsyscall("info", regs, "vsyscall read attempt denied");
        return false;
    }
    let _ = env.nx_enabled(); // WARN_ON_ONCE(!(error_code & X86_PF_INSTR)) in Linux
    emulate_vsyscall(env, mode, regs, address) == EmulateResult::Handled
}

/// #GP entry (LASS hardware). Mirrors `emulate_vsyscall_gp`.
pub fn emulate_vsyscall_gp<E: VsyscallEnv>(
    env: &mut E,
    mode: VsyscallMode,
    regs: &mut PtRegs,
) -> bool {
    if !env.lass_enabled() {
        return false;
    }
    if !is_vsyscall_vaddr(regs.rip) {
        return false;
    }
    let ip = regs.rip;
    emulate_vsyscall(env, mode, regs, ip) == EmulateResult::Handled
}

// ── production environment (wired to the live kernel) ────────────────────────

/// Production [`VsyscallEnv`] backed by the real lupos `uaccess`, `signal`,
/// `seccomp`, GDT and syscall-table modules.
#[cfg(not(test))]
pub struct KernelVsyscallEnv;

#[cfg(not(test))]
impl VsyscallEnv for KernelVsyscallEnv {
    fn warn_bad_vsyscall(&mut self, level: &str, regs: &PtRegs, message: &str) {
        crate::linux_driver_abi::tty::serial_println!(
            "vsyscall {} {} ip={:#x} cs={:#x} sp={:#x} ax={:#x}",
            level,
            message,
            regs.rip,
            regs.cs,
            regs.rsp,
            regs.rax
        );
    }

    fn access_ok(&self, ptr: u64, size: usize) -> bool {
        crate::arch::x86::kernel::uaccess::access_ok(ptr, size as u64)
    }

    fn get_user_caller(&mut self, sp: u64) -> Option<u64> {
        unsafe { crate::arch::x86::kernel::uaccess::get_user_u64(sp as *const u64).ok() }
    }

    fn secure_computing(&mut self, regs: &mut PtRegs) -> i32 {
        let task = unsafe { crate::kernel::sched::get_current() };
        if task.is_null() {
            return 0;
        }

        let seccomp = unsafe { &(*task).m27_seccomp };
        match crate::arch::x86::entry::syscall::syscall_seccomp_check_state(regs, seccomp) {
            crate::arch::x86::entry::syscall::SeccompCheck::Allow => 0,
            crate::arch::x86::entry::syscall::SeccompCheck::Errno(errno) => {
                regs.rax = errno as u64;
                1
            }
            crate::arch::x86::entry::syscall::SeccompCheck::Trap(data) => {
                unsafe {
                    crate::arch::x86::entry::syscall::queue_seccomp_trap(regs, task, data);
                }
                1
            }
        }
    }

    fn run_vsyscall(&mut self, nr: VsyscallNumber, regs: &mut PtRegs) -> i64 {
        use crate::arch::x86::entry::syscall_table::{NR_syscalls, SYS_CALL_TABLE};
        let n = nr.syscall_nr() as usize;
        if n < NR_syscalls {
            unsafe { SYS_CALL_TABLE[n](regs as *mut PtRegs) }
        } else {
            -(ENOSYS as i64)
        }
    }

    fn force_sig(&mut self, sig: i32) {
        let task = unsafe { crate::kernel::sched::get_current() };
        if !task.is_null() {
            unsafe {
                crate::kernel::signal::send_signal_to_task(task, sig);
            }
        }
    }

    fn force_sig_fault(&mut self, sig: i32, code: i32, addr: u64) {
        let task = unsafe { crate::kernel::sched::get_current() };
        if !task.is_null() {
            let info = crate::kernel::signal::SigInfo::with_sigfault(sig, code, addr, 0);
            unsafe {
                crate::kernel::signal::send_signal_info_to_task(task, info);
            }
        }
    }

    fn force_exit_sig(&mut self, sig: i32) {
        // force_exit_sig delivers a fatal signal to the current thread.
        self.force_sig(sig);
    }

    fn user_64bit_mode(&self, regs: &PtRegs) -> bool {
        regs.cs == crate::arch::x86::kernel::gdt::sel::USER_CS as u64
    }
}

/// Page-fault hook for the live kernel: read the global mode and run the
/// emulation on the faulting frame.
///
/// # Safety
/// `regs` must point to a valid kernel-stack `PtRegs` for the faulting thread.
#[cfg(not(test))]
pub unsafe fn emulate_vsyscall_pf_current(
    error_code: u64,
    regs: *mut PtRegs,
    address: u64,
) -> bool {
    let mut env = KernelVsyscallEnv;
    emulate_vsyscall_pf(
        &mut env,
        vsyscall_mode(),
        error_code,
        unsafe { &mut *regs },
        address,
    )
}

/// #GP hook for the live kernel (LASS).
///
/// # Safety
/// `regs` must point to a valid kernel-stack `PtRegs` for the faulting thread.
#[cfg(not(test))]
pub unsafe fn emulate_vsyscall_gp_current(regs: *mut PtRegs) -> bool {
    let mut env = KernelVsyscallEnv;
    emulate_vsyscall_gp(&mut env, vsyscall_mode(), unsafe { &mut *regs })
}

// ── gate vma / in_gate_area (pure predicates) ────────────────────────────────

pub const fn gate_vma_name() -> &'static str {
    "[vsyscall]"
}

pub const fn gate_vma_range(mode: VsyscallMode) -> Option<(u64, u64)> {
    match mode {
        VsyscallMode::None => None,
        VsyscallMode::Emulate | VsyscallMode::XOnly => {
            Some((VSYSCALL_ADDR, VSYSCALL_ADDR + PAGE_SIZE))
        }
    }
}

pub const fn get_gate_vma(mode: VsyscallMode, has_vsyscall_ctx: bool) -> Option<(u64, u64)> {
    if !has_vsyscall_ctx {
        return None;
    }
    gate_vma_range(mode)
}

pub const fn in_gate_area(mode: VsyscallMode, has_vsyscall_ctx: bool, addr: u64) -> bool {
    match get_gate_vma(mode, has_vsyscall_ctx) {
        Some((start, end)) => addr >= start && addr < end,
        None => false,
    }
}

pub const fn in_gate_area_no_mm(mode: VsyscallMode, addr: u64) -> bool {
    !matches!(mode, VsyscallMode::None) && is_vsyscall_vaddr(addr)
}

// ── page-table setup + map_vsyscall (wired to crate::arch::x86::mm::paging) ──

/// Pure helper: OR `_PAGE_USER` into a page-table entry value.
pub const fn pgtable_user_bit(entry: u64) -> u64 {
    entry | _PAGE_USER
}

/// Page-table index for `VSYSCALL_ADDR` at a given level (0=PTE..3=PGD).
pub const fn vsyscall_pgtable_index(level: u32) -> usize {
    let shift = 12 + 9 * level;
    ((VSYSCALL_ADDR >> shift) & 0x1ff) as usize
}

/// `FIXADDR_TOP` (x86-64): `round_up(VSYSCALL_ADDR + PAGE_SIZE, 2 MiB) - PAGE_SIZE`.
/// Ref: vendor/linux/arch/x86/include/asm/fixmap.h
const PMD_SIZE: u64 = 1 << 21;
pub const FIXADDR_TOP: u64 =
    (((VSYSCALL_ADDR + PAGE_SIZE) + (PMD_SIZE - 1)) & !(PMD_SIZE - 1)) - PAGE_SIZE;

/// `VSYSCALL_PAGE` fixmap index: `(FIXADDR_TOP - VSYSCALL_ADDR) >> PAGE_SHIFT` (511).
pub const VSYSCALL_PAGE: u64 = (FIXADDR_TOP - VSYSCALL_ADDR) >> 12;

/// `__fix_to_virt(VSYSCALL_PAGE)`. Linux `BUILD_BUG_ON`s this equals
/// `VSYSCALL_ADDR`; `map_vsyscall` asserts the same invariant.
pub const fn fix_to_virt_vsyscall_page() -> u64 {
    FIXADDR_TOP - (VSYSCALL_PAGE << 12)
}

/// Backing storage for the vsyscall page (EMULATE mode). Linux's executable
/// trampoline bytes live in the separate `vsyscall_emu_64.S`; vsyscall_64.c only
/// references the `__vsyscall_page` symbol, so a page placeholder mirrors it.
#[cfg(not(test))]
#[repr(align(4096))]
struct VsyscallPage([u8; 4096]);
#[cfg(not(test))]
static __VSYSCALL_PAGE: VsyscallPage = VsyscallPage([0u8; 4096]);

/// `__pa_symbol(&__vsyscall_page)` — physical address of the vsyscall page.
#[cfg(not(test))]
fn vsyscall_page_phys() -> u64 {
    let va = &__VSYSCALL_PAGE as *const VsyscallPage as u64;
    crate::arch::x86::mm::paging::virt_to_phys(va).unwrap_or(0)
}

/// `set_vsyscall_pgtable_user_bits` — OR `_PAGE_USER` into the PGD/P4D/PUD/PMD
/// entries covering `VSYSCALL_ADDR`, walking the live tables. Mirrors the C
/// `set_pgd(pgd, __pgd(pgd_val(*pgd) | _PAGE_USER))` at each level.
///
/// # Safety
/// `root` must be a valid top-level page-table pointer; pokes live tables.
#[cfg(not(test))]
pub unsafe fn set_vsyscall_pgtable_user_bits(root: *mut crate::arch::x86::mm::paging::pgd_t) {
    use crate::arch::x86::mm::paging::{p4d_offset, pgd_offset_pgd, pmd_offset, pud_offset};
    unsafe {
        let pgdp = pgd_offset_pgd(root, VSYSCALL_ADDR);
        (*pgdp).0 |= _PAGE_USER;
        let p4dp = p4d_offset(pgdp, VSYSCALL_ADDR);
        (*p4dp).0 |= _PAGE_USER;
        let pudp = pud_offset(p4dp, VSYSCALL_ADDR);
        (*pudp).0 |= _PAGE_USER;
        let pmdp = pmd_offset(pudp, VSYSCALL_ADDR);
        (*pmdp).0 |= _PAGE_USER;
    }
}

/// `map_vsyscall` — set up the vsyscall page for the active mode. EMULATE backs
/// the page for real: the `VSYSCALL_PAGE` fixmap slot IS `VSYSCALL_ADDR`, so the
/// fixmap install is a `map_kernel_page` at that address, followed by opening the
/// page-table user bits. XONLY installs no PTE (the gate VMA is execute-only);
/// NONE does nothing.
///
/// # Safety
/// Pokes the live kernel page tables; call once during boot.
#[cfg(not(test))]
pub unsafe fn map_vsyscall() {
    use crate::arch::x86::mm::paging::{PAGE_KERNEL, init_pgd_phys, map_kernel_page, phys_to_virt};
    // BUILD_BUG_ON(__fix_to_virt(VSYSCALL_PAGE) != VSYSCALL_ADDR)
    debug_assert_eq!(fix_to_virt_vsyscall_page(), VSYSCALL_ADDR);

    if vsyscall_mode() == VsyscallMode::Emulate {
        let phys = vsyscall_page_phys();
        unsafe {
            map_kernel_page(VSYSCALL_ADDR, phys, PAGE_KERNEL);
            let root = phys_to_virt(init_pgd_phys()) as *mut crate::arch::x86::mm::paging::pgd_t;
            set_vsyscall_pgtable_user_bits(root);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// Deterministic mock environment recording side effects.
    struct MockEnv {
        unmapped: Vec<(u64, usize)>,
        bad_stack: bool,
        user: bool,
        seccomp: i32,
        seccomp_change_ip: Option<u64>,
        syscall_ret: i64,
        lass: bool,
        signals: Vec<i32>,
        faults: Vec<(i32, i32, u64)>,
        exit_sigs: Vec<i32>,
        ran: Vec<(VsyscallNumber, u64)>, // (nr, regs.rdx at dispatch)
    }

    impl Default for MockEnv {
        fn default() -> Self {
            MockEnv {
                unmapped: Vec::new(),
                bad_stack: false,
                user: true,
                seccomp: 0,
                seccomp_change_ip: None,
                syscall_ret: 0,
                lass: false,
                signals: Vec::new(),
                faults: Vec::new(),
                exit_sigs: Vec::new(),
                ran: Vec::new(),
            }
        }
    }

    impl VsyscallEnv for MockEnv {
        fn access_ok(&self, ptr: u64, size: usize) -> bool {
            !self.unmapped.iter().any(|&(p, s)| p == ptr && s == size)
        }
        fn get_user_caller(&mut self, _sp: u64) -> Option<u64> {
            if self.bad_stack {
                None
            } else {
                Some(0xdead_beef)
            }
        }
        fn secure_computing(&mut self, regs: &mut PtRegs) -> i32 {
            if let Some(ip) = self.seccomp_change_ip {
                regs.rip = ip;
            }
            self.seccomp
        }
        fn run_vsyscall(&mut self, nr: VsyscallNumber, regs: &mut PtRegs) -> i64 {
            // Record the live rdx so the getcpu dx=0 contract is observable.
            self.ran.push((nr, regs.rdx));
            self.syscall_ret
        }
        fn force_sig(&mut self, sig: i32) {
            self.signals.push(sig);
        }
        fn force_sig_fault(&mut self, sig: i32, code: i32, addr: u64) {
            self.faults.push((sig, code, addr));
        }
        fn force_exit_sig(&mut self, sig: i32) {
            self.exit_sigs.push(sig);
        }
        fn user_64bit_mode(&self, _regs: &PtRegs) -> bool {
            self.user
        }
        fn lass_enabled(&self) -> bool {
            self.lass
        }
    }

    fn user_regs_at(rip: u64) -> PtRegs {
        let mut regs: PtRegs = unsafe { core::mem::zeroed() };
        regs.rip = rip;
        regs.rsp = 0x7fff_0000;
        regs.rdi = 0x1000;
        regs.rsi = 0x2000;
        regs
    }

    #[test]
    fn address_decoder_accepts_three_legacy_slots() {
        assert_eq!(
            addr_to_vsyscall_nr(VSYSCALL_ADDR),
            Ok(VsyscallNumber::Gettimeofday)
        );
        assert_eq!(
            addr_to_vsyscall_nr(VSYSCALL_ADDR + 0x400),
            Ok(VsyscallNumber::Time)
        );
        assert_eq!(
            addr_to_vsyscall_nr(VSYSCALL_ADDR + 0x800),
            Ok(VsyscallNumber::Getcpu)
        );
        assert_eq!(addr_to_vsyscall_nr(VSYSCALL_ADDR + 0xc00), Err(EINVAL));
    }

    #[test]
    fn syscall_numbers_match_x86_64_abi() {
        assert_eq!(VsyscallNumber::Gettimeofday.syscall_nr(), 96);
        assert_eq!(VsyscallNumber::Time.syscall_nr(), 201);
        assert_eq!(VsyscallNumber::Getcpu.syscall_nr(), 309);
    }

    #[test]
    fn non_user_or_none_mode_is_not_handled() {
        let mut env = MockEnv::default();
        env.user = false;
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR),
            EmulateResult::NotVsyscall
        );

        let mut env = MockEnv::default();
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::None, &mut regs, VSYSCALL_ADDR),
            EmulateResult::NotVsyscall
        );
    }

    #[test]
    fn successful_gettimeofday_runs_syscall_and_emulates_ret() {
        let mut env = MockEnv::default();
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        let sp0 = regs.rsp;
        let res = emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR);
        assert_eq!(res, EmulateResult::Handled);
        assert_eq!(env.ran.len(), 1);
        assert_eq!(env.ran[0].0, VsyscallNumber::Gettimeofday);
        assert_eq!(regs.rip, 0xdead_beef);
        assert_eq!(regs.rsp, sp0 + 8);
        assert_eq!(regs.rax, 0);
        assert_eq!(regs.orig_rax, u64::MAX);
    }

    #[test]
    fn bad_stack_pointer_raises_sigsegv() {
        let mut env = MockEnv::default();
        env.bad_stack = true;
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR),
            EmulateResult::Handled
        );
        assert_eq!(env.signals, [SIGSEGV]);
        assert!(env.ran.is_empty());
    }

    #[test]
    fn unwritable_output_pointer_raises_sigsegv_fault_before_dispatch() {
        let mut env = MockEnv::default();
        env.unmapped.push((0x1000, 16)); // gettimeofday di (16 bytes)
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR),
            EmulateResult::Handled
        );
        assert_eq!(env.faults, [(SIGSEGV, SEGV_MAPERR, 0x1000)]);
        assert_eq!(env.signals, [SIGSEGV]);
        assert!(env.ran.is_empty());
    }

    #[test]
    fn syscall_efault_result_raises_sigsegv() {
        let mut env = MockEnv::default();
        env.syscall_ret = -(EFAULT as i64);
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR),
            EmulateResult::Handled
        );
        assert_eq!(env.signals, [SIGSEGV]);
    }

    #[test]
    fn seccomp_skip_emulates_ret_without_running_syscall() {
        let mut env = MockEnv::default();
        env.seccomp = 1;
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        let sp0 = regs.rsp;
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR),
            EmulateResult::Handled
        );
        assert!(env.ran.is_empty());
        assert_eq!(regs.rip, 0xdead_beef);
        assert_eq!(regs.rsp, sp0 + 8);
    }

    #[test]
    fn seccomp_changing_ip_is_treated_as_tamper_and_kills() {
        let mut env = MockEnv::default();
        env.seccomp_change_ip = Some(VSYSCALL_ADDR + 0x10);
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert_eq!(
            emulate_vsyscall(&mut env, VsyscallMode::Emulate, &mut regs, VSYSCALL_ADDR),
            EmulateResult::Handled
        );
        assert_eq!(env.exit_sigs, [SIGSYS]);
        assert!(env.ran.is_empty());
    }

    #[test]
    fn getcpu_dispatches_with_dx_zeroed_then_restores_it() {
        let mut env = MockEnv::default();
        let mut regs = user_regs_at(VSYSCALL_ADDR + 0x800);
        regs.rdx = 0xabcd;
        assert_eq!(
            emulate_vsyscall(
                &mut env,
                VsyscallMode::Emulate,
                &mut regs,
                VSYSCALL_ADDR + 0x800
            ),
            EmulateResult::Handled
        );
        assert_eq!(env.ran.len(), 1);
        assert_eq!(env.ran[0].0, VsyscallNumber::Getcpu);
        // The syscall must observe rdx == 0 on the LIVE frame...
        assert_eq!(env.ran[0].1, 0);
        // ...and rdx is restored afterwards.
        assert_eq!(regs.rdx, 0xabcd);
    }

    #[test]
    fn pf_only_emulates_user_instruction_fetch_at_rip() {
        let mut env = MockEnv::default();
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert!(emulate_vsyscall_pf(
            &mut env,
            VsyscallMode::Emulate,
            X86_PF_USER | X86_PF_INSTR,
            &mut regs,
            VSYSCALL_ADDR,
        ));
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert!(!emulate_vsyscall_pf(
            &mut env,
            VsyscallMode::Emulate,
            X86_PF_WRITE | X86_PF_USER | X86_PF_INSTR,
            &mut regs,
            VSYSCALL_ADDR,
        ));
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert!(!emulate_vsyscall_pf(
            &mut env,
            VsyscallMode::Emulate,
            X86_PF_USER | X86_PF_INSTR,
            &mut regs,
            VSYSCALL_ADDR + 0x400,
        ));
    }

    #[test]
    fn gp_emulation_requires_lass_and_vsyscall_ip() {
        let mut env = MockEnv::default();
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert!(!emulate_vsyscall_gp(
            &mut env,
            VsyscallMode::XOnly,
            &mut regs
        ));
        env.lass = true;
        let mut regs = user_regs_at(VSYSCALL_ADDR);
        assert!(emulate_vsyscall_gp(
            &mut env,
            VsyscallMode::XOnly,
            &mut regs
        ));
        let mut regs = user_regs_at(0x4000);
        assert!(!emulate_vsyscall_gp(
            &mut env,
            VsyscallMode::XOnly,
            &mut regs
        ));
    }

    #[test]
    fn gate_area_tracks_mode_and_context() {
        assert_eq!(gate_vma_range(VsyscallMode::None), None);
        assert_eq!(
            gate_vma_range(VsyscallMode::XOnly),
            Some((VSYSCALL_ADDR, VSYSCALL_ADDR + PAGE_SIZE))
        );
        assert!(in_gate_area(VsyscallMode::XOnly, true, VSYSCALL_ADDR));
        assert!(!in_gate_area(VsyscallMode::XOnly, false, VSYSCALL_ADDR));
        assert!(in_gate_area_no_mm(
            VsyscallMode::Emulate,
            VSYSCALL_ADDR + 0x10
        ));
        assert!(!in_gate_area_no_mm(VsyscallMode::None, VSYSCALL_ADDR));
    }

    #[test]
    fn fix_to_virt_invariant_holds_for_vsyscall_page() {
        // Linux BUILD_BUG_ON: __fix_to_virt(VSYSCALL_PAGE) == VSYSCALL_ADDR.
        assert_eq!(FIXADDR_TOP, 0xffff_ffff_ff7f_f000);
        assert_eq!(VSYSCALL_PAGE, 511);
        assert_eq!(fix_to_virt_vsyscall_page(), VSYSCALL_ADDR);
    }

    #[test]
    fn vsyscall_setup_parses_and_sets_global_mode() {
        assert_eq!(vsyscall_setup(Some("emulate")), 0);
        assert_eq!(vsyscall_mode(), VsyscallMode::Emulate);
        assert_eq!(vsyscall_setup(Some("none")), 0);
        assert_eq!(vsyscall_mode(), VsyscallMode::None);
        assert_eq!(vsyscall_setup(Some("xonly")), 0);
        assert_eq!(vsyscall_mode(), VsyscallMode::XOnly);
        assert_eq!(vsyscall_setup(Some("bogus")), -EINVAL);
        assert_eq!(vsyscall_setup(None), -EINVAL);
    }
}
