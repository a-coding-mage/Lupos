//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry/syscall_32.c
//! test-origin: linux:vendor/linux/arch/x86/entry/syscall_32.c
//! 32-bit system call dispatch (INT 0x80 and the SYSENTER/SYSCALL32 fast path).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/syscall_32.c
//!
//! The pure register/decision helpers are composed by the orchestration entry
//! points (`do_int80_emulation`, `do_fast_syscall_32`, ...) over a
//! [`Syscall32Env`] trait seam, so the dispatch and SYSEXIT-vs-IRET logic is
//! unit-testable on a host.
//!
//! STATUS — translation incomplete (tagged `partial`):
//! - The pure helpers (`prepare_int80_regs`, `do_fast_syscall_32_exit`,
//!   `int80_is_external`, `__ia32_enabled`, ...) are faithful translations, but
//!   there is NO production `Syscall32Env` impl yet: the orchestration must be
//!   wired to the real APIC read, `uaccess::get_user_u32`, the ia32 syscall
//!   table, and the syscall enter/exit work before this is truly translated.
//!
//! On x86-64 Linux does not materialise `sys_call_table[]` (only `ia32_sys_call`
//! is used); lupos follows the same shape via [`Syscall32Env::dispatch`].

use crate::include::uapi::errno::{EFAULT, ENOSYS};
use core::sync::atomic::{AtomicBool, Ordering};

/// `IA32_NR_syscalls` — number of 32-bit syscall slots.
pub const IA32_NR_SYSCALLS: u32 = 461;
pub const X86_EFLAGS_RF: u64 = 1 << 16;
pub const X86_EFLAGS_TF: u64 = 1 << 8;
pub const X86_EFLAGS_VM: u64 = 1 << 17;
pub const X86_EFLAGS_IF: u64 = 1 << 9;
/// `__USER32_CS` / `__USER_DS` selectors checked before SYSEXIT/SYSRETL.
pub const USER32_CS: u64 = 0x23;
pub const USER_DS: u64 = 0x2b;

/// `APIC_ISR` base offset (vendor/linux/arch/x86/include/asm/apicdef.h).
pub const APIC_ISR: u32 = 0x100;
/// Software-interrupt vector for the 32-bit syscall ABI.
pub const INT80_VECTOR: u32 = 0x80;

/// The subset of `struct pt_regs` the 32-bit entry path reads/writes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PtRegs32Entry {
    pub ax: u64,
    pub bx: u64,
    pub cx: u64,
    pub dx: u64,
    pub si: u64,
    pub di: u64,
    pub bp: u64,
    pub sp: u64,
    pub ip: u64,
    pub cs: u64,
    pub ss: u64,
    pub flags: u64,
    pub orig_ax: u64,
    pub compat_status: bool,
}

// ── __ia32_enabled global (CONFIG_IA32_EMULATION) ────────────────────────────
//
// `bool __ia32_enabled = !IS_ENABLED(CONFIG_IA32_EMULATION_DEFAULT_DISABLED)`,
// overridable by the `ia32_emulation=` boot parameter via kstrtobool().

static IA32_ENABLED: AtomicBool = AtomicBool::new(true);

/// Read `__ia32_enabled`.
pub fn ia32_enabled() -> bool {
    IA32_ENABLED.load(Ordering::Relaxed)
}

/// `ia32_emulation_override_cmdline` — apply the `ia32_emulation=` boot param.
/// Returns 0 on success or `-EINVAL`, matching `kstrtobool`'s contract.
pub fn ia32_emulation_override_cmdline(arg: &str) -> i32 {
    match parse_ia32_emulation_arg(arg) {
        Some(value) => {
            IA32_ENABLED.store(value, Ordering::Relaxed);
            0
        }
        None => -(crate::include::uapi::errno::EINVAL),
    }
}

// ── pure dispatch / decision helpers ─────────────────────────────────────────

/// `ia32_sys_call(regs, nr)` — dispatch one 32-bit syscall through `call`
/// (the ia32 syscall table), falling back to `-ENOSYS` (`__ia32_sys_ni_syscall`).
pub fn ia32_sys_call<F>(regs: &PtRegs32Entry, nr: u32, mut call: F) -> i64
where
    F: FnMut(&PtRegs32Entry, u32) -> Option<i64>,
{
    call(regs, nr).unwrap_or(-(ENOSYS as i64))
}

/// `syscall_32_enter` — flag the thread as 32-bit-compat (`TS_COMPAT`) when
/// IA32 emulation is enabled and return the truncated syscall number.
pub fn syscall_32_enter(regs: &mut PtRegs32Entry, ia32_emulation: bool) -> i32 {
    if ia32_emulation {
        regs.compat_status = true;
    }
    regs.orig_ax as i32
}

/// `do_syscall_32_irqs_on` — range-check `nr` and dispatch. Negative numbers
/// become very large unsigned values and miss the in-range branch; `nr == -1`
/// (a restart/poked value) is left untouched, matching the C.
pub fn do_syscall_32_irqs_on<F>(regs: &mut PtRegs32Entry, nr: i32, mut call: F)
where
    F: FnMut(&PtRegs32Entry, u32) -> Option<i64>,
{
    let unr = nr as u32;
    if unr < IA32_NR_SYSCALLS {
        // array_index_nospec() is a speculation barrier with no functional
        // effect; the in-range guarantee is the bound we already checked.
        regs.ax = ia32_sys_call(regs, unr, &mut call) as u64;
    } else if nr != -1 {
        regs.ax = (-(ENOSYS as i64)) as u64;
    }
}

/// Establish the 32-bit syscall convention used by the INT 0x80 / FRED paths:
/// save the 32-bit-truncated number in `orig_ax` and invalidate `ax` (`-ENOSYS`).
pub fn prepare_int80_regs(regs: &mut PtRegs32Entry) {
    regs.orig_ax = regs.ax & 0xffff_ffff;
    regs.ax = (-(ENOSYS as i64)) as u64;
}

/// `do_SYSENTER_32` prologue: SYSENTER loses RSP (the vDSO saved it in RBP) and
/// clobbers EFLAGS.IF (assumed set in usermode).
pub fn do_sysenter_32_setup(regs: &mut PtRegs32Entry) {
    regs.sp = regs.bp;
    regs.flags |= X86_EFLAGS_IF;
}

/// SYSEXIT/SYSRETL vs IRET decision (the tail of `do_fast_syscall_32`):
/// SYSEXIT is only safe on non-XENPV when RIP is the vDSO landing pad, CS/SS
/// match `MSR_STAR`, and none of RF/TF/VM are set; otherwise use IRET.
pub fn do_fast_syscall_32_exit(regs: &PtRegs32Entry, landing_pad: u64, xenpv: bool) -> bool {
    if xenpv {
        return false;
    }
    if regs.ip != landing_pad {
        return false;
    }
    if regs.cs != USER32_CS || regs.ss != USER_DS {
        return false;
    }
    regs.flags & (X86_EFLAGS_RF | X86_EFLAGS_TF | X86_EFLAGS_VM) == 0
}

/// Store the vDSO-stashed EBP fetched from the user stack, or report `-EFAULT`
/// into `regs.ax` when the user pointer faulted.
pub fn fetch_vdso_saved_ebp(stack_word: Option<u32>, regs: &mut PtRegs32Entry) -> Result<(), i32> {
    match stack_word {
        Some(ebp) => {
            regs.bp = ebp as u64;
            Ok(())
        }
        None => {
            regs.ax = (-(EFAULT as i64)) as u64;
            Err(EFAULT)
        }
    }
}

/// `kstrtobool`-style parse of the `ia32_emulation=` value.
pub const fn parse_ia32_emulation_arg(arg: &str) -> Option<bool> {
    match arg.as_bytes() {
        b"1" | b"y" | b"Y" | b"on" | b"true" => Some(true),
        b"0" | b"n" | b"N" | b"off" | b"false" => Some(false),
        _ => None,
    }
}

// ── hardware / subsystem seam ────────────────────────────────────────────────

/// Seam for the 32-bit entry orchestration. Production wires it to the real
/// kernel; tests provide a deterministic mock.
pub trait Syscall32Env {
    /// `user_mode(regs)` — did the event come from user space?
    fn user_mode(&self, regs: &PtRegs32Entry) -> bool;
    /// `apic_read(reg)` — read a local-APIC register (for `int80_is_external`).
    fn apic_read(&self, reg: u32) -> u32;
    /// `cpu_feature_enabled(X86_FEATURE_XENPV)` — fake APIC on XENPV guests.
    fn xenpv(&self) -> bool;
    /// `get_user(*(u32*)&regs->bp, (u32 __user *)(u32)regs->sp)` for the fast path.
    fn get_user_u32(&self, addr: u64) -> Option<u32>;
    /// `syscall_enter_from_user_mode_work(regs, nr)` — ptrace/seccomp/audit;
    /// may rewrite the syscall number. Default: identity.
    fn syscall_enter_work(&self, _regs: &mut PtRegs32Entry, nr: i32) -> i32 {
        nr
    }
    /// `ia32_sys_call` table lookup. `None` => `__ia32_sys_ni_syscall`.
    fn dispatch(&self, regs: &PtRegs32Entry, nr: u32) -> Option<i64>;
    /// `current->mm->context.vdso + vdso32_image.sym_int80_landing_pad`.
    fn vdso_landing_pad(&self) -> u64;
    /// `IS_ENABLED(CONFIG_IA32_EMULATION)` for this build/runtime.
    fn ia32_emulation(&self) -> bool {
        ia32_enabled()
    }
    /// Unexpected INT 0x80 in kernel / from an external interrupt: `panic()`.
    fn panic_unexpected_int80(&self) -> !;
    // Context-tracking hooks (no-ops by default / in tests).
    fn enter_from_user_mode(&self, _regs: &PtRegs32Entry) {}
    fn exit_to_user_mode(&self, _regs: &PtRegs32Entry) {}
    fn add_random_kstack_offset(&self) {}
}

/// `int80_is_external` — true if APIC ISR has vector 0x80 set (a real external
/// interrupt, not a soft INT). XENPV guests have a fake APIC, so return false.
pub fn int80_is_external<E: Syscall32Env>(env: &E) -> bool {
    if env.xenpv() {
        return false;
    }
    let offs = (INT80_VECTOR / 32) * 0x10;
    let bit = 1u32 << (INT80_VECTOR % 32);
    env.apic_read(APIC_ISR + offs) & bit != 0
}

// ── orchestration entry points ───────────────────────────────────────────────

/// `do_int80_emulation` — INT 0x80 C entry. The kernel never issues INT 0x80,
/// so a kernel-mode or genuinely-external 0x80 is a fatal anomaly.
pub fn do_int80_emulation<E: Syscall32Env>(env: &E, regs: &mut PtRegs32Entry) {
    if !env.user_mode(regs) {
        env.panic_unexpected_int80();
    }
    env.enter_from_user_mode(regs);
    env.add_random_kstack_offset();
    if int80_is_external(env) {
        env.panic_unexpected_int80();
    }
    prepare_int80_regs(regs);
    let mut nr = syscall_32_enter(regs, env.ia32_emulation());
    nr = env.syscall_enter_work(regs, nr);
    do_syscall_32_irqs_on(regs, nr, |r, n| env.dispatch(r, n));
    env.exit_to_user_mode(regs);
}

/// FRED INT 0x80 entry. FRED separates INT insns from external interrupts, so
/// the `int80_is_external()` check is intentionally absent (calling it would be
/// incorrect); otherwise identical to [`do_int80_emulation`].
pub fn do_int80_emulation_fred<E: Syscall32Env>(env: &E, regs: &mut PtRegs32Entry) {
    env.enter_from_user_mode(regs);
    env.add_random_kstack_offset();
    prepare_int80_regs(regs);
    let mut nr = syscall_32_enter(regs, env.ia32_emulation());
    nr = env.syscall_enter_work(regs, nr);
    do_syscall_32_irqs_on(regs, nr, |r, n| env.dispatch(r, n));
    env.exit_to_user_mode(regs);
}

/// `do_int80_syscall_32` — INT 0x80 entry on a kernel built without
/// CONFIG_IA32_EMULATION (no `int80_is_external` / kernel-mode panic guard).
pub fn do_int80_syscall_32<E: Syscall32Env>(env: &E, regs: &mut PtRegs32Entry) {
    let mut nr = syscall_32_enter(regs, env.ia32_emulation());
    nr = env.syscall_enter_work(regs, nr);
    env.add_random_kstack_offset();
    do_syscall_32_irqs_on(regs, nr, |r, n| env.dispatch(r, n));
    env.exit_to_user_mode(regs);
}

/// `__do_fast_syscall_32` — fetch EBP (stashed by the vDSO) from the user stack,
/// then dispatch like a normal syscall. Returns false (use IRET) if EBP faults.
pub fn do_fast_syscall_32_inner<E: Syscall32Env>(env: &E, regs: &mut PtRegs32Entry) -> bool {
    let mut nr = syscall_32_enter(regs, env.ia32_emulation());
    env.enter_from_user_mode(regs);
    env.add_random_kstack_offset();
    // The pointer is explicitly 32-bit, so it cannot be out of range.
    let ebp = env.get_user_u32((regs.sp as u32) as u64);
    if fetch_vdso_saved_ebp(ebp, regs).is_err() {
        env.exit_to_user_mode(regs);
        return false;
    }
    nr = env.syscall_enter_work(regs, nr);
    do_syscall_32_irqs_on(regs, nr, |r, n| env.dispatch(r, n));
    env.exit_to_user_mode(regs);
    true
}

/// `do_fast_syscall_32` — vDSO SYSENTER/SYSCALL32 entry. Returns true to exit
/// via SYSEXIT/SYSRETL, false to use IRET.
pub fn do_fast_syscall_32<E: Syscall32Env>(env: &E, regs: &mut PtRegs32Entry) -> bool {
    // Make the frame look like an INT 0x80: land on the vDSO int80 pad.
    let landing_pad = env.vdso_landing_pad();
    regs.ip = landing_pad;
    if !do_fast_syscall_32_inner(env, regs) {
        return false;
    }
    do_fast_syscall_32_exit(regs, landing_pad, env.xenpv())
}

/// `do_SYSENTER_32` — SYSENTER entry: restore RSP from RBP and EFLAGS.IF, then
/// run the fast-syscall path.
pub fn do_sysenter_32<E: Syscall32Env>(env: &E, regs: &mut PtRegs32Entry) -> bool {
    do_sysenter_32_setup(regs);
    do_fast_syscall_32(env, regs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::{Cell, RefCell};

    struct MockEnv {
        user: bool,
        xenpv: bool,
        apic_isr: u32,
        ebp: Option<u32>,
        landing_pad: u64,
        ia32: bool,
        nr_override: Option<i32>,
        dispatched: RefCell<alloc::vec::Vec<u32>>,
        dispatch_ret: i64,
        enter_count: Cell<u32>,
        exit_count: Cell<u32>,
    }

    impl Default for MockEnv {
        fn default() -> Self {
            MockEnv {
                user: true,
                xenpv: false,
                apic_isr: 0,
                ebp: Some(0xb0b0_b0b0),
                landing_pad: 0x7fff_1000,
                ia32: true,
                nr_override: None,
                dispatched: RefCell::new(alloc::vec::Vec::new()),
                dispatch_ret: 42,
                enter_count: Cell::new(0),
                exit_count: Cell::new(0),
            }
        }
    }

    impl Syscall32Env for MockEnv {
        fn user_mode(&self, _regs: &PtRegs32Entry) -> bool {
            self.user
        }
        fn apic_read(&self, _reg: u32) -> u32 {
            self.apic_isr
        }
        fn xenpv(&self) -> bool {
            self.xenpv
        }
        fn get_user_u32(&self, _addr: u64) -> Option<u32> {
            self.ebp
        }
        fn syscall_enter_work(&self, _regs: &mut PtRegs32Entry, nr: i32) -> i32 {
            self.nr_override.unwrap_or(nr)
        }
        fn dispatch(&self, _regs: &PtRegs32Entry, nr: u32) -> Option<i64> {
            self.dispatched.borrow_mut().push(nr);
            Some(self.dispatch_ret)
        }
        fn vdso_landing_pad(&self) -> u64 {
            self.landing_pad
        }
        fn ia32_emulation(&self) -> bool {
            self.ia32
        }
        fn panic_unexpected_int80(&self) -> ! {
            panic!("Unexpected external interrupt 0x80");
        }
        fn enter_from_user_mode(&self, _regs: &PtRegs32Entry) {
            self.enter_count.set(self.enter_count.get() + 1);
        }
        fn exit_to_user_mode(&self, _regs: &PtRegs32Entry) {
            self.exit_count.set(self.exit_count.get() + 1);
        }
    }

    #[test]
    fn int80_preparation_truncates_syscall_number() {
        let mut regs = PtRegs32Entry {
            ax: 0x1_0000_0005,
            ..Default::default()
        };
        prepare_int80_regs(&mut regs);
        assert_eq!(regs.orig_ax, 5);
        assert_eq!(regs.ax as i64, -(ENOSYS as i64));
    }

    #[test]
    fn syscall_dispatch_uses_ni_for_out_of_range() {
        let mut regs = PtRegs32Entry::default();
        do_syscall_32_irqs_on(&mut regs, IA32_NR_SYSCALLS as i32, |_, _| Some(7));
        assert_eq!(regs.ax as i64, -(ENOSYS as i64));
        // nr == -1 (restart marker) leaves ax untouched.
        regs.ax = 0x1234;
        do_syscall_32_irqs_on(&mut regs, -1, |_, _| Some(7));
        assert_eq!(regs.ax, 0x1234);
    }

    #[test]
    fn int80_is_external_reads_vector_0x80_isr_bit() {
        let mut env = MockEnv::default();
        // 0x80 % 32 == 0, so bit 0 of the ISR word at APIC_ISR + 0x40.
        env.apic_isr = 1;
        assert!(int80_is_external(&env));
        env.apic_isr = 0;
        assert!(!int80_is_external(&env));
        // XENPV always reports a soft interrupt.
        env.apic_isr = 1;
        env.xenpv = true;
        assert!(!int80_is_external(&env));
    }

    #[test]
    fn do_int80_emulation_dispatches_truncated_number() {
        let env = MockEnv::default();
        let mut regs = PtRegs32Entry {
            ax: 0x1_0000_0004,
            ..Default::default()
        };
        do_int80_emulation(&env, &mut regs);
        assert_eq!(*env.dispatched.borrow(), [4]);
        assert!(regs.compat_status);
        assert_eq!(regs.ax, 42);
    }

    #[test]
    #[should_panic(expected = "Unexpected external interrupt 0x80")]
    fn do_int80_emulation_panics_on_kernel_mode() {
        let mut env = MockEnv::default();
        env.user = false;
        let mut regs = PtRegs32Entry::default();
        do_int80_emulation(&env, &mut regs);
    }

    #[test]
    #[should_panic(expected = "Unexpected external interrupt 0x80")]
    fn do_int80_emulation_panics_on_external_interrupt() {
        let mut env = MockEnv::default();
        env.apic_isr = 1; // vector 0x80 pending in ISR
        let mut regs = PtRegs32Entry::default();
        do_int80_emulation(&env, &mut regs);
    }

    #[test]
    fn fast_syscall_32_uses_sysexit_when_frame_is_clean() {
        let env = MockEnv::default();
        let mut regs = PtRegs32Entry {
            // The fast-path asm entry leaves the syscall number in orig_ax.
            orig_ax: 3,
            cs: USER32_CS,
            ss: USER_DS,
            sp: 0x4000,
            ..Default::default()
        };
        let sysexit = do_fast_syscall_32(&env, &mut regs);
        assert!(sysexit, "clean frame on the landing pad should use SYSEXIT");
        assert_eq!(regs.ip, env.landing_pad);
        assert_eq!(regs.bp, 0xb0b0_b0b0, "EBP fetched from the user stack");
        assert_eq!(*env.dispatched.borrow(), [3]);
    }

    #[test]
    fn fast_syscall_32_uses_iret_when_ebp_faults() {
        let mut env = MockEnv::default();
        env.ebp = None; // get_user faults
        let mut regs = PtRegs32Entry {
            ax: 3,
            cs: USER32_CS,
            ss: USER_DS,
            ..Default::default()
        };
        let sysexit = do_fast_syscall_32(&env, &mut regs);
        assert!(!sysexit, "EBP fault must fall back to IRET");
        assert_eq!(regs.ax as i64, -(EFAULT as i64));
        assert!(env.dispatched.borrow().is_empty());
        assert_eq!(env.enter_count.get(), 1);
        assert_eq!(env.exit_count.get(), 1);
    }

    #[test]
    fn sysenter_32_restores_sp_and_if_then_runs_fast_path() {
        let env = MockEnv::default();
        let mut regs = PtRegs32Entry {
            ax: 1,
            bp: 0x9000,
            cs: USER32_CS,
            ss: USER_DS,
            ..Default::default()
        };
        let sysexit = do_sysenter_32(&env, &mut regs);
        assert_eq!(regs.sp, 0x9000, "SYSENTER restores SP from BP");
        assert_ne!(regs.flags & X86_EFLAGS_IF, 0, "EFLAGS.IF assumed set");
        assert!(sysexit);
    }

    #[test]
    fn ia32_emulation_override_parses_and_sets_global() {
        assert_eq!(ia32_emulation_override_cmdline("0"), 0);
        assert!(!ia32_enabled());
        assert_eq!(ia32_emulation_override_cmdline("1"), 0);
        assert!(ia32_enabled());
        assert_eq!(
            ia32_emulation_override_cmdline("maybe"),
            -(crate::include::uapi::errno::EINVAL)
        );
        // restore default for other tests.
        IA32_ENABLED.store(true, Ordering::Relaxed);
    }

    #[test]
    fn fast_exit_requires_landing_segments_and_clear_debug_flags() {
        let regs = PtRegs32Entry {
            ip: 0x8000,
            cs: USER32_CS,
            ss: USER_DS,
            flags: 0,
            ..Default::default()
        };
        assert!(do_fast_syscall_32_exit(&regs, 0x8000, false));
        assert!(!do_fast_syscall_32_exit(
            &PtRegs32Entry {
                flags: X86_EFLAGS_TF,
                ..regs
            },
            0x8000,
            false
        ));
        // XENPV always uses IRET.
        assert!(!do_fast_syscall_32_exit(&regs, 0x8000, true));
    }
}
