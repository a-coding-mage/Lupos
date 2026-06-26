//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry/entry_fred.c
//! test-origin: linux:vendor/linux/arch/x86/entry/entry_fred.c
//! FRED (Flexible Return and Event Delivery) entry dispatch.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/entry_fred.c
//!
//! FRED delivers every event (interrupt, exception, syscall) through a single
//! entry point carrying an event `type` and `vector` in the stack frame
//! (`fred_ss`). These routers decide which handler an event dispatches to.
//!
//! STATUS — translation incomplete (tagged `partial`):
//! - The routing DECISION is faithfully translated and unit-tested, but it is
//!   returned as a [`FredAction`] enum rather than actually CALLING the handler
//!   (`exc_page_fault`, `do_syscall_64`, ...) the way the C does. Nothing
//!   consumes the enum yet, and lupos does not currently expose `exc_*` trap
//!   handlers under these names, so the dispatch is a model, not a translation.
//!   `fred_entry_from_kvm` is the KVM-injected path (out of scope, CLAUDE.md 2b).

use crate::include::uapi::errno::ENOSYS;

/// FRED EVENT_TYPE_OTHER vector numbers.
pub const FRED_SYSCALL: u8 = 1;
pub const FRED_SYSENTER: u8 = 2;
pub const IA32_SYSCALL_VECTOR: u8 = 0x80;

// x86 trap vectors (vendor/linux/arch/x86/include/asm/trapnr.h).
pub const X86_TRAP_DE: u8 = 0; // Divide-by-zero
pub const X86_TRAP_DB: u8 = 1; // Debug
pub const X86_TRAP_NMI: u8 = 2; // Non-maskable Interrupt
pub const X86_TRAP_BP: u8 = 3; // Breakpoint
pub const X86_TRAP_OF: u8 = 4; // Overflow
pub const X86_TRAP_BR: u8 = 5; // Bound Range Exceeded
pub const X86_TRAP_UD: u8 = 6; // Invalid Opcode
pub const X86_TRAP_NM: u8 = 7; // Device Not Available
pub const X86_TRAP_DF: u8 = 8; // Double Fault
pub const X86_TRAP_TS: u8 = 10; // Invalid TSS
pub const X86_TRAP_NP: u8 = 11; // Segment Not Present
pub const X86_TRAP_SS: u8 = 12; // Stack Segment Fault
pub const X86_TRAP_GP: u8 = 13; // General Protection Fault
pub const X86_TRAP_PF: u8 = 14; // Page Fault
pub const X86_TRAP_MF: u8 = 16; // x87 FPU error
pub const X86_TRAP_AC: u8 = 17; // Alignment Check
pub const X86_TRAP_MC: u8 = 18; // Machine Check
pub const X86_TRAP_XF: u8 = 19; // SIMD FP exception
pub const X86_TRAP_VE: u8 = 20; // Virtualization Exception
pub const X86_TRAP_CP: u8 = 21; // Control Protection Exception
pub const X86_TRAP_VC: u8 = 29; // VMM Communication (SEV-ES)

pub const FIRST_EXTERNAL_VECTOR: u8 = 0x20;
pub const FIRST_SYSTEM_VECTOR: u8 = 0xef;
/// Number of system-vector slots (`256 - FIRST_SYSTEM_VECTOR`).
pub const NR_SYSTEM_VECTORS: usize = 256 - FIRST_SYSTEM_VECTOR as usize;

/// FRED `fred_ss.type` event classes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FredEventType {
    ExtInt = 0,
    Nmi = 1,
    HwExc = 2,
    SwInt = 3,
    PrivSwExc = 4,
    SwExc = 5,
    Other = 6,
}

/// The FRED stack frame fields the routers consume.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FredFrame {
    pub event_type: FredEventType,
    pub vector: u8,
    /// `fred_ss.l` — 64-bit (long mode) event.
    pub long_mode: bool,
    /// `fred_cs.sl` — current stack level (>0 means a nested/high stack).
    pub stack_level: u8,
    pub orig_ax: u64,
    pub ax: u64,
}

/// The handler an event routes to. Each variant names a Linux `exc_*`/`do_*`
/// handler; the routing decision is what this file owns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FredAction {
    BadType,
    CommonInterrupt,
    SystemVector,
    SpuriousInterrupt,
    Nmi,
    PageFault,
    Debug,
    Breakpoint,
    Overflow,
    DivideError,
    Bounds,
    InvalidOp,
    DeviceNotAvailable,
    DoubleFault,
    InvalidTss,
    SegmentNotPresent,
    StackSegment,
    GeneralProtection,
    CoprocessorError,
    AlignmentCheck,
    SimdError,
    MachineCheck,
    Syscall64,
    Sysenter32,
    Int80,
    VirtualizationException,
    VmmCommunication,
    ControlProtection,
    KvmNmiOrExtInt,
}

/// `fred_intx` — software interrupt (`INT n`) routing. Note these are the
/// `INT 0x3`/`INT 0x4` opcodes, distinct from the `INT3`/`INTO` traps.
pub const fn fred_intx(vector: u8, ia32_enabled: bool) -> FredAction {
    match vector {
        X86_TRAP_BP => FredAction::Breakpoint,
        X86_TRAP_OF => FredAction::Overflow,
        IA32_SYSCALL_VECTOR if ia32_enabled => FredAction::Int80,
        _ => FredAction::GeneralProtection,
    }
}

/// `fred_other` — EVENT_TYPE_OTHER: the native and compat syscall entry points.
pub const fn fred_other(frame: FredFrame, ia32_enabled: bool) -> FredAction {
    if frame.vector == FRED_SYSCALL && frame.long_mode {
        FredAction::Syscall64
    } else if ia32_enabled && frame.vector == FRED_SYSENTER && !frame.long_mode {
        FredAction::Sysenter32
    } else {
        FredAction::InvalidOp
    }
}

/// `fred_hwexc` — hardware exception routing. `#PF` is the hot path; every
/// architectural vector maps to its handler, unknown vectors are fatal.
pub const fn fred_hwexc(vector: u8) -> FredAction {
    match vector {
        X86_TRAP_PF => FredAction::PageFault,
        X86_TRAP_DE => FredAction::DivideError,
        X86_TRAP_DB => FredAction::Debug,
        X86_TRAP_BR => FredAction::Bounds,
        X86_TRAP_UD => FredAction::InvalidOp,
        X86_TRAP_NM => FredAction::DeviceNotAvailable,
        X86_TRAP_DF => FredAction::DoubleFault,
        X86_TRAP_TS => FredAction::InvalidTss,
        X86_TRAP_NP => FredAction::SegmentNotPresent,
        X86_TRAP_SS => FredAction::StackSegment,
        X86_TRAP_GP => FredAction::GeneralProtection,
        X86_TRAP_MF => FredAction::CoprocessorError,
        X86_TRAP_AC => FredAction::AlignmentCheck,
        X86_TRAP_XF => FredAction::SimdError,
        X86_TRAP_MC => FredAction::MachineCheck,
        X86_TRAP_VE => FredAction::VirtualizationException,
        X86_TRAP_CP => FredAction::ControlProtection,
        X86_TRAP_VC => FredAction::VmmCommunication,
        _ => FredAction::BadType,
    }
}

/// `fred_swexc` — software exception routing (`INT3` / `INTO` opcodes).
pub const fn fred_swexc(vector: u8) -> FredAction {
    match vector {
        X86_TRAP_BP => FredAction::Breakpoint,
        X86_TRAP_OF => FredAction::Overflow,
        _ => FredAction::BadType,
    }
}

/// `fred_extint` — external-interrupt routing: below the external base is
/// invalid, system vectors go through the sysvec table, the rest are device IRQs.
pub const fn fred_extint(vector: u8) -> FredAction {
    if vector < FIRST_EXTERNAL_VECTOR {
        FredAction::BadType
    } else if vector >= FIRST_SYSTEM_VECTOR {
        FredAction::SystemVector
    } else {
        FredAction::CommonInterrupt
    }
}

/// `fred_entry_from_user` — dispatch a FRED event that came from user space.
pub const fn fred_entry_from_user(frame: FredFrame, ia32_enabled: bool) -> FredAction {
    match frame.event_type {
        FredEventType::ExtInt => fred_extint(frame.vector),
        FredEventType::Nmi if frame.vector == X86_TRAP_NMI => FredAction::Nmi,
        FredEventType::HwExc => fred_hwexc(frame.vector),
        FredEventType::SwInt => fred_intx(frame.vector, ia32_enabled),
        FredEventType::PrivSwExc if frame.vector == X86_TRAP_DB => FredAction::Debug,
        FredEventType::SwExc => fred_swexc(frame.vector),
        FredEventType::Other => fred_other(frame, ia32_enabled),
        _ => FredAction::BadType,
    }
}

/// `fred_entry_from_kernel` — dispatch a FRED event from kernel space. The
/// kernel never issues `INT n` or syscalls, so SWINT/OTHER are absent (they
/// fall through to `fred_bad_type`).
pub const fn fred_entry_from_kernel(frame: FredFrame) -> FredAction {
    match frame.event_type {
        FredEventType::ExtInt => fred_extint(frame.vector),
        FredEventType::Nmi if frame.vector == X86_TRAP_NMI => FredAction::Nmi,
        FredEventType::HwExc => fred_hwexc(frame.vector),
        FredEventType::PrivSwExc if frame.vector == X86_TRAP_DB => FredAction::Debug,
        FredEventType::SwExc => fred_swexc(frame.vector),
        _ => FredAction::BadType,
    }
}

/// `__fred_entry_from_kvm` — KVM only injects external interrupts and NMIs.
pub const fn fred_entry_from_kvm(frame: FredFrame) -> FredAction {
    match frame.event_type {
        FredEventType::ExtInt => fred_extint(frame.vector),
        FredEventType::Nmi => FredAction::KvmNmiOrExtInt,
        _ => FredAction::BadType,
    }
}

/// `fred_other` rewrites `orig_ax`/`ax` before dispatching the syscall. Returns
/// the `(orig_ax, ax)` the syscall path observes (`ax = -ENOSYS` until the
/// syscall runs), mirroring the C for the syscall actions.
pub const fn fred_syscall_regs(ax: u64) -> (u64, u64) {
    (ax, (-(ENOSYS as i64)) as u64)
}

/// Outcome of `fred_bad_type`: a high stack level is unrecoverable (`panic`);
/// otherwise the task is killed via the oops path (`SIGKILL`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BadTypeOutcome {
    FatalPanic,
    OopsKill,
}

/// `fred_bad_type` recovery decision based on the originating stack level.
pub const fn fred_bad_type_outcome(stack_level: u8) -> BadTypeOutcome {
    if stack_level > 0 {
        BadTypeOutcome::FatalPanic
    } else {
        BadTypeOutcome::OopsKill
    }
}

/// `fred_install_sysvec` — register a system-vector handler before setup is
/// complete. Rejects out-of-range vectors, late installs, and double writes.
pub fn fred_install_sysvec(
    table: &mut [Option<FredAction>],
    sysvec: u8,
    handler: FredAction,
    setup_done: bool,
) -> bool {
    if sysvec < FIRST_SYSTEM_VECTOR || setup_done {
        return false;
    }
    let index = (sysvec - FIRST_SYSTEM_VECTOR) as usize;
    if index >= table.len() || table[index].is_some() {
        return false;
    }
    table[index] = Some(handler);
    true
}

/// `fred_complete_exception_setup` — mark all reserved/system vectors in
/// `system_vectors`, and fill any empty sysvec slot with the spurious-interrupt
/// handler. Returns the new `fred_setup_done` (true). `system_vectors` is the
/// 256-bit used-vector bitmap.
pub fn fred_complete_exception_setup(
    sysvec_table: &mut [Option<FredAction>],
    system_vectors: &mut [bool; 256],
) -> bool {
    for v in 0..FIRST_EXTERNAL_VECTOR as usize {
        system_vectors[v] = true;
    }
    for i in 0..sysvec_table.len() {
        system_vectors[i + FIRST_SYSTEM_VECTOR as usize] = true;
        if sysvec_table[i].is_none() {
            sysvec_table[i] = Some(FredAction::SpuriousInterrupt);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(event_type: FredEventType, vector: u8) -> FredFrame {
        FredFrame {
            event_type,
            vector,
            long_mode: false,
            stack_level: 0,
            orig_ax: 0,
            ax: 0,
        }
    }

    #[test]
    fn user_other_routes_native_syscall_and_ia32_sysenter() {
        let mut native = frame(FredEventType::Other, FRED_SYSCALL);
        native.long_mode = true;
        native.ax = 39;
        assert_eq!(fred_entry_from_user(native, true), FredAction::Syscall64);

        let ia32 = FredFrame {
            vector: FRED_SYSENTER,
            long_mode: false,
            ..native
        };
        assert_eq!(fred_entry_from_user(ia32, true), FredAction::Sysenter32);
        assert_eq!(fred_entry_from_user(ia32, false), FredAction::InvalidOp);
    }

    #[test]
    fn int80_requires_ia32_enabled() {
        assert_eq!(fred_intx(IA32_SYSCALL_VECTOR, true), FredAction::Int80);
        assert_eq!(
            fred_intx(IA32_SYSCALL_VECTOR, false),
            FredAction::GeneralProtection
        );
    }

    #[test]
    fn hwexc_routes_every_architectural_vector() {
        // The previously-missing/mis-routed vectors are the point of this test.
        assert_eq!(fred_hwexc(X86_TRAP_PF), FredAction::PageFault);
        assert_eq!(fred_hwexc(X86_TRAP_DE), FredAction::DivideError);
        assert_eq!(fred_hwexc(X86_TRAP_BR), FredAction::Bounds);
        assert_eq!(fred_hwexc(X86_TRAP_UD), FredAction::InvalidOp);
        assert_eq!(fred_hwexc(X86_TRAP_NM), FredAction::DeviceNotAvailable);
        assert_eq!(fred_hwexc(X86_TRAP_DF), FredAction::DoubleFault);
        assert_eq!(fred_hwexc(X86_TRAP_TS), FredAction::InvalidTss);
        assert_eq!(fred_hwexc(X86_TRAP_NP), FredAction::SegmentNotPresent);
        assert_eq!(fred_hwexc(X86_TRAP_SS), FredAction::StackSegment);
        assert_eq!(fred_hwexc(X86_TRAP_GP), FredAction::GeneralProtection);
        assert_eq!(fred_hwexc(X86_TRAP_MF), FredAction::CoprocessorError);
        assert_eq!(fred_hwexc(X86_TRAP_AC), FredAction::AlignmentCheck);
        assert_eq!(fred_hwexc(X86_TRAP_XF), FredAction::SimdError);
        assert_eq!(fred_hwexc(X86_TRAP_MC), FredAction::MachineCheck);
        assert_eq!(fred_hwexc(X86_TRAP_VC), FredAction::VmmCommunication);
        assert_eq!(fred_hwexc(99), FredAction::BadType);
    }

    #[test]
    fn swexc_routes_int3_and_into_only() {
        assert_eq!(fred_swexc(X86_TRAP_BP), FredAction::Breakpoint);
        assert_eq!(fred_swexc(X86_TRAP_OF), FredAction::Overflow);
        assert_eq!(fred_swexc(X86_TRAP_GP), FredAction::BadType);
    }

    #[test]
    fn kernel_entry_has_no_syscall_or_swint_paths() {
        // EVENT_TYPE_OTHER (syscall) from kernel is impossible -> BadType.
        assert_eq!(
            fred_entry_from_kernel(frame(FredEventType::Other, FRED_SYSCALL)),
            FredAction::BadType
        );
        assert_eq!(
            fred_entry_from_kernel(frame(FredEventType::SwInt, IA32_SYSCALL_VECTOR)),
            FredAction::BadType
        );
        assert_eq!(
            fred_entry_from_kernel(frame(FredEventType::HwExc, X86_TRAP_PF)),
            FredAction::PageFault
        );
    }

    #[test]
    fn extint_classifies_reserved_device_and_system_vectors() {
        assert_eq!(fred_extint(0x10), FredAction::BadType);
        assert_eq!(
            fred_extint(FIRST_EXTERNAL_VECTOR),
            FredAction::CommonInterrupt
        );
        assert_eq!(fred_extint(FIRST_SYSTEM_VECTOR), FredAction::SystemVector);
    }

    #[test]
    fn kvm_entry_only_handles_extint_and_nmi() {
        assert_eq!(
            fred_entry_from_kvm(frame(FredEventType::Nmi, X86_TRAP_NMI)),
            FredAction::KvmNmiOrExtInt
        );
        assert_eq!(
            fred_entry_from_kvm(frame(FredEventType::HwExc, X86_TRAP_PF)),
            FredAction::BadType
        );
    }

    #[test]
    fn bad_type_panics_only_from_high_stack_level() {
        assert_eq!(fred_bad_type_outcome(0), BadTypeOutcome::OopsKill);
        assert_eq!(fred_bad_type_outcome(1), BadTypeOutcome::FatalPanic);
    }

    #[test]
    fn syscall_regs_invalidate_ax_with_enosys() {
        let (orig, ax) = fred_syscall_regs(39);
        assert_eq!(orig, 39);
        assert_eq!(ax as i64, -(ENOSYS as i64));
    }

    #[test]
    fn sysvec_install_rejects_late_and_duplicate_writes() {
        let mut table = [None; NR_SYSTEM_VECTORS];
        assert!(fred_install_sysvec(
            &mut table,
            FIRST_SYSTEM_VECTOR,
            FredAction::SystemVector,
            false
        ));
        assert!(!fred_install_sysvec(
            &mut table,
            FIRST_SYSTEM_VECTOR,
            FredAction::SystemVector,
            false
        ));
        assert!(!fred_install_sysvec(
            &mut table,
            FIRST_SYSTEM_VECTOR + 1,
            FredAction::SystemVector,
            true
        ));
    }

    #[test]
    fn complete_exception_setup_fills_spurious_and_marks_vectors() {
        let mut table = [None; NR_SYSTEM_VECTORS];
        table[0] = Some(FredAction::SystemVector);
        let mut system_vectors = [false; 256];
        assert!(fred_complete_exception_setup(
            &mut table,
            &mut system_vectors
        ));
        // Reserved range marked.
        assert!(system_vectors[0]);
        assert!(system_vectors[(FIRST_EXTERNAL_VECTOR - 1) as usize]);
        // Installed slot marked; empty slots filled and reserved for sysvec dispatch.
        assert!(system_vectors[FIRST_SYSTEM_VECTOR as usize]);
        assert_eq!(table[1], Some(FredAction::SpuriousInterrupt));
        assert!(system_vectors[FIRST_SYSTEM_VECTOR as usize + 1]);
    }
}
