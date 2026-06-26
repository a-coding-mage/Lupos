//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! 64-bit Task State Segment (TSS) and Interrupt Stack Table (IST).
//!
//! In 64-bit (long) mode the TSS has two roles:
//!   1. **Stack switching on privilege change**: When the CPU transitions from
//!      ring 3 → ring 0 (syscall, interrupt from user), it loads RSP0 from the
//!      TSS as the kernel stack pointer.
//!   2. **Interrupt Stack Table (IST)**: Seven dedicated stacks (IST1–IST7)
//!      for critical exceptions that must survive a corrupted kernel stack
//!      (double fault, NMI, machine check).
//!
//! Unlike 32-bit mode, the CPU never performs hardware task switching via TSS
//! in 64-bit mode — the TSS is purely a data structure read by the CPU.
//!
//! The TSS descriptor in the GDT is 16 bytes (two consecutive 8-byte slots)
//! because it needs to store a 64-bit base address.  It is loaded into the
//! Task Register (TR) via the `ltr` instruction.
//!
//! References:
//!   Intel SDM Vol. 3A §7.7 "Task Management in 64-bit Mode"
//!   Intel SDM Vol. 3A Figure 7-11 "64-Bit TSS Format"
//!   vendor/linux/arch/x86/kernel/process_64.c
//!   https://wiki.osdev.org/Task_State_Segment#Long_Mode

use core::mem::size_of;

use crate::kernel::sched::MAX_CPUS;

// ── IST slot assignments ────────────────────────────────────────────────────
//
// IST indices are 1-based: 0 means "use current RSP" (no IST).
// We reserve dedicated stacks for exceptions that can arrive asynchronously
// or when the kernel stack pointer itself may be corrupt.
//
// Reference: Intel SDM Vol. 3A §6.14.5 "Interrupt Stack Table"

/// IST slot for the Double Fault handler (#DF, vector 8).
///
/// A double fault fires when the CPU cannot call the original exception handler
/// (e.g., if the kernel stack overflowed and caused a #SS during #PF handling).
/// The dedicated stack ensures the handler always has a clean stack to run on.
pub const IST_DOUBLE_FAULT: u8 = 1;

/// IST slot for the Non-Maskable Interrupt handler (NMI, vector 2).
///
/// NMIs are fully asynchronous — they can arrive in the middle of any interrupt
/// handler, including one that has not yet swapped to a safe stack.
/// A separate stack prevents RSP corruption during nested NMI delivery.
pub const IST_NMI: u8 = 2;

/// IST slot for the Machine Check handler (#MC, vector 18).
///
/// Machine checks are hardware-reported errors (ECC, bus errors, thermal events).
/// They are asynchronous and potentially corrupt the running stack.
pub const IST_MACHINE_CHECK: u8 = 3;

/// Each IST stack is 16 KiB — enough for a deep kernel backtrace.
///
/// Must be at least 16 bytes (one stack frame) but in practice 4 KiB × 4
/// gives comfortable headroom for exception handler code.
pub const IST_STACK_SIZE: usize = 4096 * 4;

// ── Dedicated exception stacks ──────────────────────────────────────────────
//
// Each stack is a `static mut` array of bytes in the kernel's BSS section.
// The CPU writes to these stacks on exception delivery — they must be writable
// and must outlive any interrupt handler invocation, hence `'static`.
//
// The IST pointer stored in the TSS must point to the **top** (high address)
// of the stack because x86 stacks grow downward.

/// SAFETY: Only the CPU and `init()` ever write to these; no concurrent access
/// before SMP is enabled.
#[repr(align(16))]
#[derive(Clone, Copy)]
struct IstStack([u8; IST_STACK_SIZE]);

static mut DOUBLE_FAULT_STACKS: [IstStack; MAX_CPUS] = [IstStack([0; IST_STACK_SIZE]); MAX_CPUS];
static mut NMI_STACKS: [IstStack; MAX_CPUS] = [IstStack([0; IST_STACK_SIZE]); MAX_CPUS];
static mut MACHINE_CHECK_STACKS: [IstStack; MAX_CPUS] = [IstStack([0; IST_STACK_SIZE]); MAX_CPUS];

// ── TSS layout ──────────────────────────────────────────────────────────────
//
// The 64-bit TSS layout is fixed by the hardware spec (Intel SDM Vol. 3A
// Figure 7-11).  Every field offset must match exactly — the CPU reads this
// structure directly during exception delivery and privilege-level transitions.
//
// Byte offsets from TSS base:
//   0    reserved0 (u32)
//   4    RSP0 (u64) — kernel stack for ring 0
//   12   RSP1 (u64)
//   20   RSP2 (u64)
//   28   reserved1 (u64)
//   36   IST1 (u64) — IST entries 1–7
//   44   IST2 (u64)
//   52   IST3 (u64)
//   60   IST4 (u64)
//   68   IST5 (u64)
//   76   IST6 (u64)
//   84   IST7 (u64)
//   92   reserved2 (u64)
//   100  reserved3 (u16)
//   102  iomap_base (u16) — offset of I/O Permission Bitmap from TSS base
//                           set to size_of::<Tss>() → no IOPB → all I/O
//                           from ring 3 is forbidden

/// 64-bit Task State Segment.
///
/// Must be `#[repr(C, packed)]` — the CPU accesses fields at fixed byte offsets;
/// any padding inserted by the compiler would shift them and cause memory corruption.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Tss {
    _reserved0: u32,
    /// Ring-0 through ring-2 stack pointers.  On a syscall or ring-3 interrupt,
    /// the CPU replaces RSP with `rsp[0]` (RSP0) before pushing the exception frame.
    pub rsp: [u64; 3],
    _reserved1: u64,
    /// Interrupt Stack Table entries (IST1–IST7, index 0 = IST1).
    /// Non-zero entries point to the top of a dedicated stack for that IST slot.
    pub ist: [u64; 7],
    _reserved2: u64,
    _reserved3: u16,
    /// Offset (in bytes) from the TSS base to the I/O Permission Bitmap.
    ///
    /// Setting this to `size_of::<Tss>()` means "bitmap starts just past the
    /// TSS", but since no bitmap bytes follow, all I/O ports are forbidden for
    /// ring-3 code.  Kernel code (ring 0) ignores the IOPB entirely.
    pub iomap_base: u16,
}

impl Tss {
    pub const fn new() -> Self {
        Self {
            _reserved0: 0,
            rsp: [0; 3],
            _reserved1: 0,
            ist: [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            iomap_base: size_of::<Tss>() as u16,
        }
    }
}

/// The global kernel TSS.  Initialised by `init()` before the GDT is loaded.
///
/// `static mut` is required because the CPU needs a stable address to point to
/// and because `Tss` is not `Sync` (it contains raw u64 IST pointers).
pub static mut TSS: Tss = Tss::new();

/// Per-CPU TSS storage for APs.
///
/// CPU0 intentionally keeps the public `TSS` symbol because the current
/// syscall entry stub reads CPU0 RSP0 by symbol before the GS-relative per-CPU
/// entry path lands. APs use their own TSS slots so `ltr` never reuses the
/// BSP's busy TSS descriptor.
static mut AP_TSS: [Tss; MAX_CPUS] = [Tss::new(); MAX_CPUS];

const fn cpu_slot(cpu: usize) -> usize {
    if cpu >= MAX_CPUS { MAX_CPUS - 1 } else { cpu }
}

#[cfg(test)]
fn current_cpu_index() -> usize {
    0
}

#[cfg(not(test))]
fn current_cpu_index() -> usize {
    crate::arch::x86::kernel::smp::current_cpu_id().min(MAX_CPUS - 1)
}

unsafe fn tss_for_cpu_mut(cpu: usize) -> *mut Tss {
    let slot = cpu_slot(cpu);
    if slot == 0 {
        &raw mut TSS
    } else {
        &raw mut AP_TSS[slot]
    }
}

/// Return the TSS pointer backing `cpu`.
///
/// # Safety
/// The caller must use the pointer only while descriptor setup owns the target
/// CPU's TSS slot.
pub unsafe fn tss_for_cpu(cpu: usize) -> *const Tss {
    unsafe { tss_for_cpu_mut(cpu) as *const Tss }
}

unsafe fn ist_top_for_cpu(cpu: usize, stacks: *const [IstStack; MAX_CPUS]) -> u64 {
    let slot = cpu_slot(cpu);
    unsafe {
        let stack = &raw const (*stacks)[slot] as *const IstStack as *const u8;
        stack as u64 + IST_STACK_SIZE as u64
    }
}

#[inline]
unsafe fn set_rsp0_raw(tss: *mut Tss, stack_top: u64) {
    debug_assert!(!tss.is_null(), "TSS pointer must not be null");
    unsafe {
        (*tss).rsp[0] = stack_top;
    }
}

/// Set the ring-0 stack pointer used for privilege transitions from userspace.
///
/// Linux keeps this in sync through `update_task_stack()` before returning to
/// user mode; Lupos updates it both during task switches and immediately before
/// the PID1 userspace handoff.
///
/// # Safety
/// Must be called only while the global TSS belongs to the running CPU.
pub unsafe fn set_rsp0(stack_top: u64) {
    let cpu = current_cpu_index();
    unsafe {
        set_rsp0_raw(tss_for_cpu_mut(cpu), stack_top);
    }
}

/// Populate the TSS IST entries with the addresses of the dedicated stacks.
///
/// Must be called before `gdt::init()` — the GDT needs a valid TSS pointer,
/// and the TSS must have its IST fields filled in before any exception can fire.
///
/// # Safety
/// Not thread-safe for the target CPU slot.  Must be called before the matching
/// GDT is loaded with that TSS descriptor.
pub unsafe fn init_cpu(cpu: usize) {
    // x86 stacks grow downward: the IST pointer must be the *high* address
    // (one past the last byte of the stack array).
    // Reference: Intel SDM Vol. 3A section 7.7, "IST field in the TSS".
    unsafe {
        let tss = tss_for_cpu_mut(cpu);
        tss.write(Tss::new());
        (*tss).ist[IST_DOUBLE_FAULT as usize - 1] =
            ist_top_for_cpu(cpu, &raw const DOUBLE_FAULT_STACKS);
        (*tss).ist[IST_NMI as usize - 1] = ist_top_for_cpu(cpu, &raw const NMI_STACKS);
        (*tss).ist[IST_MACHINE_CHECK as usize - 1] =
            ist_top_for_cpu(cpu, &raw const MACHINE_CHECK_STACKS);
    }
}

/// Populate CPU0's TSS IST entries.
///
/// # Safety
/// Not thread-safe. Must be called exactly once from `kernel_main` before
/// Application Processors are started.
pub unsafe fn init() {
    unsafe {
        init_cpu(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    // ── Hardware layout checks ───────────────────────────────────────────────
    // These tests act as a compile-time regression guard: if the Tss struct
    // layout ever drifts from the Intel SDM specification, the tests fail
    // immediately rather than corrupting the CPU state silently at runtime.

    #[test]
    fn tss_total_size_is_104_bytes() {
        // Intel SDM Figure 7-11: TSS occupies exactly 104 bytes.
        // reserved0(4) + rsp(24) + reserved1(8) + ist(56) + reserved2(8)
        // + reserved3(2) + iomap_base(2) = 104.
        assert_eq!(
            size_of::<Tss>(),
            104,
            "TSS size must be 104 bytes (SDM §7.7)"
        );
    }

    #[test]
    fn tss_rsp0_at_offset_4() {
        // RSP0 is the first meaningful field, at byte offset 4 (after the
        // 4-byte reserved0 field at the top of the TSS structure).
        assert_eq!(
            offset_of!(Tss, rsp),
            4,
            "RSP0 must be at TSS+4 (SDM Figure 7-11)"
        );
    }

    #[test]
    fn set_rsp0_writes_hardware_rsp0_slot() {
        let mut tss = Tss {
            _reserved0: 0,
            rsp: [0; 3],
            _reserved1: 0,
            ist: [0; 7],
            _reserved2: 0,
            _reserved3: 0,
            iomap_base: size_of::<Tss>() as u16,
        };
        let stack_top = 0x1234_5678_9abc_def0;

        unsafe {
            set_rsp0_raw(&raw mut tss, stack_top);
        }

        let base = &raw const tss as *const Tss as *const u8;
        let rsp0 = unsafe { base.add(4).cast::<u64>().read_unaligned() };
        assert_eq!(rsp0, stack_top);
    }

    #[test]
    fn tss_ist_at_offset_36() {
        // IST1 begins at offset 36:
        //   reserved0(4) + rsp(24) + reserved1(8) = 36
        assert_eq!(
            offset_of!(Tss, ist),
            36,
            "IST array must start at TSS+36 (SDM Figure 7-11)"
        );
    }

    #[test]
    fn tss_iomap_base_at_offset_102() {
        // The I/O Permission Bitmap offset field is at byte 102 of the TSS.
        assert_eq!(
            offset_of!(Tss, iomap_base),
            102,
            "iomap_base must be at TSS+102 (SDM Figure 7-11)"
        );
    }

    // ── IST constant sanity checks ───────────────────────────────────────────

    #[test]
    fn ist_indices_are_in_hardware_range() {
        // IST indices are 1-based; valid hardware range is 1–7.
        // Index 0 means "no IST" (use current RSP) — never use 0 as a constant.
        for &idx in &[IST_DOUBLE_FAULT, IST_NMI, IST_MACHINE_CHECK] {
            assert!(idx >= 1 && idx <= 7, "IST index {idx} out of range [1,7]");
        }
    }

    #[test]
    fn ist_indices_are_unique() {
        // Each critical exception must have its own IST slot.
        assert_ne!(IST_DOUBLE_FAULT, IST_NMI);
        assert_ne!(IST_DOUBLE_FAULT, IST_MACHINE_CHECK);
        assert_ne!(IST_NMI, IST_MACHINE_CHECK);
    }
}
