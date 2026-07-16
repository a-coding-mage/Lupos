//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/idt.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/idt.c
//! Interrupt Descriptor Table (IDT) — CPU exception handlers for x86_64.
//!
//! The IDT is an array of up to 256 gate descriptors.  Each descriptor
//! maps a vector number (0–255) to an interrupt or trap handler.
//!
//! # Vectors 0–31 (CPU exceptions)
//! Defined by the Intel SDM.  We install handlers for all 32 reserved
//! exception vectors so that the kernel never triple-faults silently.
//!
//! # ISR stub design
//! The CPU pushes an exception frame on the stack before jumping to the
//! IDT handler.  We add a uniform frame by pushing the vector number and
//! a placeholder error code (for exceptions that do not push one), then
//! jump to a common stub that saves all GP registers and calls the Rust
//! dispatcher.
//!
//! Stack layout on entry to `exception_dispatch` (low address at top):
//! ```text
//!  [RSP+0]   r15          ← saved by common stub
//!  [RSP+8]   r14
//!  [RSP+16]  r13
//!  [RSP+24]  r12
//!  [RSP+32]  r11
//!  [RSP+40]  r10
//!  [RSP+48]  r9
//!  [RSP+56]  r8
//!  [RSP+64]  rdi
//!  [RSP+72]  rsi
//!  [RSP+80]  rbp
//!  [RSP+88]  rdx
//!  [RSP+96]  rcx
//!  [RSP+104] rbx
//!  [RSP+112] rax
//!  [RSP+120] vector       ← pushed by individual ISR stub
//!  [RSP+128] error_code   ← pushed by CPU (or 0 if no error code)
//!  [RSP+136] rip          ← CPU exception frame
//!  [RSP+144] cs
//!  [RSP+152] rflags
//!  [RSP+160] user_rsp     (only on privilege-level change ring3→ring0)
//!  [RSP+168] user_ss      (only on privilege-level change ring3→ring0)
//! ```
//!
//! References:
//!   Intel SDM Vol. 3A Chapter 6 "Interrupt and Exception Handling"
//!   Intel SDM Vol. 3A §6.14.1 "64-Bit Mode IDT"
//!   Intel SDM Vol. 3A Table 6-1 "Protected-Mode Exceptions and Interrupts"
//!   vendor/linux/arch/x86/kernel/idt.c
//!   vendor/linux/arch/x86/kernel/traps.c
//!   vendor/linux/arch/x86/kernel/irq.c
//!   vendor/linux/arch/x86/kernel/irq_64.c
//!   https://wiki.osdev.org/Interrupt_Descriptor_Table
//!   https://wiki.osdev.org/Exceptions

use core::mem::size_of;

use super::tss::{IST_DOUBLE_FAULT, IST_MACHINE_CHECK, IST_NMI};
use crate::arch::x86::kernel::gdt::sel;

// Log macros are `#[macro_export]` at the crate root (src/log.rs).
use crate::{log_error, log_warn};

// ── CPU exception vector numbers ────────────────────────────────────────────

/// #DE — Divide-By-Zero Error (fault; no error code)
pub const VEC_DIVIDE_ERROR: u8 = 0;
/// #DB — Debug (fault/trap; no error code)
pub const VEC_DEBUG: u8 = 1;
/// NMI — Non-Maskable Interrupt (no error code; IST2)
pub const VEC_NMI: u8 = 2;
/// #BP — Breakpoint (`int3`; trap; no error code; DPL=3 so user can use)
pub const VEC_BREAKPOINT: u8 = 3;
/// #OF — Overflow (`into`; trap; no error code)
pub const VEC_OVERFLOW: u8 = 4;
/// #BR — BOUND Range Exceeded (fault; no error code)
pub const VEC_BOUND_RANGE: u8 = 5;
/// #UD — Invalid Opcode (fault; no error code)
pub const VEC_INVALID_OPCODE: u8 = 6;
/// #NM — Device Not Available (fault; no error code)
pub const VEC_DEVICE_NOT_AVAILABLE: u8 = 7;
/// #DF — Double Fault (abort; error code always 0; IST1 mandatory)
pub const VEC_DOUBLE_FAULT: u8 = 8;
/// Coprocessor Segment Overrun (reserved/legacy; no error code)
pub const VEC_COPROC_OVERRUN: u8 = 9;
/// #TS — Invalid TSS (fault; error code = selector index)
pub const VEC_INVALID_TSS: u8 = 10;
/// #NP — Segment Not Present (fault; error code = selector index)
pub const VEC_SEGMENT_NOT_PRESENT: u8 = 11;
/// #SS — Stack-Segment Fault (fault; error code = selector index or 0)
pub const VEC_STACK_FAULT: u8 = 12;
/// #GP — General Protection Fault (fault; error code = selector or 0)
pub const VEC_GENERAL_PROTECTION: u8 = 13;
/// #PF — Page Fault (fault; error code = page-fault flags; CR2 = faulting addr)
pub const VEC_PAGE_FAULT: u8 = 14;
/// Vector 15 is reserved by Intel — should never fire.
pub const VEC_RESERVED_15: u8 = 15;
/// #MF — x87 FPU Floating-Point Error (fault; no error code)
pub const VEC_X87_FP: u8 = 16;
/// #AC — Alignment Check (fault; error code always 0)
pub const VEC_ALIGNMENT_CHECK: u8 = 17;
/// #MC — Machine Check (abort; model-specific; IST3)
pub const VEC_MACHINE_CHECK: u8 = 18;
/// #XM/#XF — SIMD Floating-Point Exception (fault; no error code)
pub const VEC_SIMD_FP: u8 = 19;
/// #VE — Virtualization Exception (fault; no error code)
pub const VEC_VIRTUALIZATION: u8 = 20;
/// #CP — Control Protection Exception (fault; error code)
pub const VEC_CONTROL_PROTECTION: u8 = 21;

// ── External (LAPIC / IPI) vectors ───────────────────────────────────────────

/// LAPIC timer interrupt vector (Milestone 6).
///
/// Lives above the legacy 8259 PIC remap range (0x20–0x2F) and below the
/// kernel IPI band (0xF0–0xFF) so it can never collide with either.
///
/// We use a fixed vector (0x40) instead of dynamically allocating one because
/// the kernel currently has no IRQ allocator; revisit when an IRQ subsystem
/// lands.
///
/// Linux reference: arch/x86/include/asm/irq_vectors.h — `LOCAL_TIMER_VECTOR`
/// (Linux uses 0xEC; we pick 0x40 to keep our pre-IRQ-allocator vector map
/// flat and to leave the 0xE_/0xF_ region free for future IPI work.
/// TODO(M7+): realign to Linux LOCAL_TIMER_VECTOR once we have an IRQ allocator.)
pub const TIMER_VECTOR: u8 = 0x40;

/// First vector used by the remapped 8259 PIC.
pub const LEGACY_IRQ_VECTOR_BASE: u8 = 0x20;

/// Last vector used by the remapped 8259 PIC.
pub const LEGACY_IRQ_VECTOR_LAST: u8 = LEGACY_IRQ_VECTOR_BASE + 15;

/// IPI vector for the "CPU ping" test (Milestone 5).
///
/// Vectors 0xF0–0xFF are conventionally reserved for kernel IPIs in Linux.
/// (See arch/x86/include/asm/irq_vectors.h — IRQ_WORK_VECTOR = 0xF1 etc.)
/// We use 0xF0 for the SMP rendezvous test IPI.
pub const IPI_PING_VECTOR: u8 = 0xF0;

/// TLB shootdown IPI vector (Milestone 6).
///
/// Sent by `tlb::flush_tlb_others` to remote CPUs to request that they
/// invalidate cached TLB entries for one or more virtual addresses.
///
/// Linux reference: arch/x86/include/asm/irq_vectors.h —
/// `INVALIDATE_TLB_VECTOR_START` (Linux uses 0xFD).  We use 0xF1 because
/// 0xF0 is already taken by `IPI_PING_VECTOR` and we want to keep the
/// 0xFD-vicinity free for future Linux-aligned placements.
/// TODO(M7+): realign to Linux INVALIDATE_TLB_VECTOR_START.
pub const TLB_SHOOTDOWN_VECTOR: u8 = 0xF1;

/// Reschedule IPI vector.
pub const RESCHEDULE_VECTOR: u8 = 0xF2;

/// Instruction-stream synchronization IPI used by live text patching.
pub const TEXT_POKE_SYNC_VECTOR: u8 = 0xF3;

// ── IDT gate descriptor ──────────────────────────────────────────────────────
//
// A 64-bit IDT gate is 16 bytes.  Bit layout:
//
//  127:96  handler_high[63:32]  upper 32 bits of handler RIP
//   95:64  _reserved            must be 0
//   63:48  handler_mid[31:16]   middle 16 bits of handler RIP
//   47:40  type_attr            P | DPL | 0 | gate_type
//   39:35  _reserved
//   34:32  ist                  IST index (0 = current stack, 1–7 = IST)
//   31:16  cs                   code segment selector
//   15:0   handler_low[15:0]    low 16 bits of handler RIP
//
// Gate types:
//   0xE = 64-bit Interrupt Gate  — clears IF on entry (disable hardware IRQs)
//   0xF = 64-bit Trap Gate       — preserves IF on entry
//
// We use interrupt gates for all exception handlers: disabling IRQs during
// exception handling prevents re-entrancy races at the cost of some latency.
// This matches Linux's default (all exception handlers use interrupt gates).
//
// Reference: Intel SDM Vol. 3A Figure 6-7 "64-Bit IDT Gate Descriptors"

/// A single 16-byte IDT gate descriptor.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct IdtEntry {
    handler_low: u16,  // bits 15:0  of handler address
    cs: u16,           // code segment selector
    ist: u8,           // bits 2:0 = IST index (0 = no IST)
    type_attr: u8,     // P | DPL(2) | 0 | type(4)
    handler_mid: u16,  // bits 31:16 of handler address
    handler_high: u32, // bits 63:32 of handler address
    _reserved: u32,    // must be zero
}

impl IdtEntry {
    /// Absent (not-present) descriptor.  The CPU will fault with #GP if it
    /// tries to use this entry, which is better than an undefined jump.
    pub const fn absent() -> Self {
        Self {
            handler_low: 0,
            cs: 0,
            ist: 0,
            type_attr: 0, // P=0 → not present
            handler_mid: 0,
            handler_high: 0,
            _reserved: 0,
        }
    }

    /// Build a 64-bit interrupt gate.
    ///
    /// An interrupt gate **clears IF** on entry, preventing nested hardware IRQs.
    ///
    /// - `handler` — address of the ISR stub (pushed to IDT, not called directly)
    /// - `cs`      — code segment selector (always `sel::KERNEL_CS = 0x08`)
    /// - `ist`     — IST index (0 = current stack; 1–7 = dedicated IST stack)
    pub fn interrupt_gate(handler: unsafe extern "C" fn(), cs: u16, ist: u8) -> Self {
        let addr = handler as u64;
        Self {
            handler_low: (addr & 0xFFFF) as u16,
            cs,
            ist: ist & 0x7,
            type_attr: 0x8E, // P=1 | DPL=0 | 0 | type=0xE (interrupt gate)
            handler_mid: ((addr >> 16) & 0xFFFF) as u16,
            handler_high: ((addr >> 32) & 0xFFFF_FFFF) as u32,
            _reserved: 0,
        }
    }

    /// Build a 64-bit trap gate.
    ///
    /// A trap gate **preserves IF** — used for exceptions that allow interrupts
    /// to remain enabled (e.g., breakpoint debugging, software exceptions).
    pub fn trap_gate(handler: unsafe extern "C" fn(), cs: u16, ist: u8) -> Self {
        let mut g = Self::interrupt_gate(handler, cs, ist);
        g.type_attr = 0x8F; // P=1 | DPL=0 | 0 | type=0xF (trap gate)
        g
    }

    /// Build a trap gate accessible from ring 3 (for `int3` / `into`).
    ///
    /// DPL=3 in `type_attr` allows user-space code to trigger this vector
    /// via software interrupt without getting a #GP.
    pub fn user_trap_gate(handler: unsafe extern "C" fn(), cs: u16, ist: u8) -> Self {
        let mut g = Self::trap_gate(handler, cs, ist);
        g.type_attr = (g.type_attr & !0x60) | 0x60; // set DPL = 3
        g
    }
}

// ── IDT structure ────────────────────────────────────────────────────────────

/// 256-entry IDT covering all possible interrupt vectors.
///
/// `align(16)` satisfies the `lidt` instruction requirement that the IDT base
/// be aligned to 8 bytes (we use 16 for cache-line friendliness).
#[repr(C, align(16))]
pub struct Idt([IdtEntry; 256]);

/// IDTR value — the 10-byte operand for `lidt` / `sidt`.
#[repr(C, packed)]
struct IdtRegister {
    limit: u16,
    base: u64,
}

/// The global kernel IDT.
static mut IDT: Idt = Idt([IdtEntry::absent(); 256]);

impl Idt {
    fn set(&mut self, vector: u8, entry: IdtEntry) {
        self.0[vector as usize] = entry;
    }

    /// Load the IDT via `lidt`.
    ///
    /// # Safety
    /// - `idt_ptr` must point to a valid, fully-populated `Idt` that will
    ///   remain at a fixed address for the lifetime of the kernel.
    /// - All ISR stubs referenced by the IDT entries must be valid.
    pub unsafe fn load(idt_ptr: *const Idt) {
        let reg = IdtRegister {
            limit: (size_of::<Idt>() - 1) as u16,
            base: idt_ptr as u64,
        };
        unsafe {
            core::arch::asm!(
                "lidt [{0}]",
                in(reg) &reg,
                options(readonly, nostack, preserves_flags),
            );
        }
    }
}

// ── Saved register frame ─────────────────────────────────────────────────────
//
// `exception_dispatch` receives a pointer to this structure (via RDI).
// The layout must match the push order in `isr_common` exactly.

/// Complete CPU state saved by the ISR common stub on exception entry.
///
/// Fields are ordered from low to high address (i.e., in push order):
/// the first field (`r15`) is at the lowest address because it was pushed last.
#[repr(C)]
pub struct ExceptionFrame {
    // Pushed by isr_common (in REVERSE order due to stack growth):
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    // Pushed by individual ISR stubs:
    pub vector: u64,     // exception vector number
    pub error_code: u64, // error code (or 0 placeholder for no-error exceptions)
    // Pushed by the CPU on exception delivery:
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    // Present only on privilege-level change (ring 3 → ring 0):
    pub user_rsp: u64,
    pub user_ss: u64,
}

// ── ISR common stub ──────────────────────────────────────────────────────────
//
// All per-exception stubs jump here.  This function:
//   1. Saves all 15 GP registers.
//   2. Passes RSP (pointing to the saved frame) as the first argument.
//   3. Calls the Rust dispatcher.
//   4. Restores GP registers and returns from interrupt via `iretq`.
//
// IMPORTANT: This function is `#[no_mangle]` so that `global_asm!` ISR stubs
// can reference it by name without symbol mangling.
//
// Reference: System V AMD64 ABI for function call conventions
//            (first arg in RDI, callee-saved: RBX, RBP, R12–R15)

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn isr_common() {
    // SAFETY: Naked function — we emit the complete prologue and epilogue.
    core::arch::naked_asm!(
        // ── Save GP registers ──────────────────────────────────────────────
        // Push in order rax…r15 (reverse of ExceptionFrame field order because
        // the stack grows downward).  After all 15 pushes, RSP points to r15.
        // Interrupts and exceptions from ring 3 do not perform SWAPGS in
        // hardware. Match Linux's entry discipline so any scheduler switch
        // from this frame sees kernel GS active and the user's GS base parked
        // in KERNEL_GS_BASE.
        "test qword ptr [rsp + 24], 3",
        "jz 2f",
        "swapgs",
        "2:",

        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rbp",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",

        // ── Save x87/SSE state ────────────────────────────────────────────
        // Kernel Rust may use XMM registers while handling a user fault.  ld.so
        // keeps bootstrap relocation state in XMM registers, so preserve the
        // interrupted FPU/SIMD image around the Rust dispatcher.
        "mov rdi, rsp",
        "sub rsp, 528",
        "and rsp, -16",
        "mov [rsp + 512], rdi",
        "fxsave64 [rsp]",

        // ── Call Rust dispatcher ───────────────────────────────────────────
        // System V AMD64 ABI: first argument in RDI = pointer to frame.
        "mov rdi, [rsp + 512]",
        "call {dispatch}",

        // Restore the interrupted x87/SSE state and the frame stack pointer.
        "fxrstor64 [rsp]",
        "mov rsp, [rsp + 512]",

        // ── Restore GP registers ───────────────────────────────────────────
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rbp",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",

        // Skip the vector + error_code slots pushed by individual ISR stubs.
        "add rsp, 16",

        // Restore the user's GS base before returning to ring 3. After the
        // two synthetic slots above are skipped, CS is at [rsp + 8].
        "test qword ptr [rsp + 8], 3",
        "jz 3f",
        "swapgs",
        "3:",

        // Return from interrupt — restores RIP, CS, RFLAGS (and RSP/SS on
        // privilege-level change ring3→ring0).
        "iretq",

        dispatch = sym exception_dispatch,
    );
}

// ── Per-exception ISR stubs ──────────────────────────────────────────────────
//
// Each stub pushes [error_code_or_0, vector_number] onto the stack to create
// a uniform frame layout, then jumps to `isr_common`.
//
// Two variants:
//   isr_no_error!   — exception does NOT push an error code (we push 0)
//   isr_with_error! — exception DOES push an error code (CPU already did it)
//
// We use `#[naked]` functions with `sym isr_common` so the linker resolves
// the reference at link time — no hardcoded addresses needed.

macro_rules! isr_no_error {
    ($name:ident, $vec:expr) => {
        #[unsafe(naked)]
        #[allow(dead_code)]
        unsafe extern "C" fn $name() {
            // SAFETY: Naked stub — CPU exception entry; we own the full body.
            core::arch::naked_asm!(
                "push 0",       // dummy error code (keeps frame layout uniform)
                "push {v}",     // vector number
                "jmp {c}",
                v = const $vec as u64,
                c = sym isr_common,
            );
        }
    };
}

macro_rules! isr_with_error {
    ($name:ident, $vec:expr) => {
        #[unsafe(naked)]
        #[allow(dead_code)]
        unsafe extern "C" fn $name() {
            // SAFETY: CPU already pushed the error code; we only push the vector.
            core::arch::naked_asm!(
                "push {v}",     // vector number
                "jmp {c}",
                v = const $vec as u64,
                c = sym isr_common,
            );
        }
    };
}

// Reference: Intel SDM Vol. 3A Table 6-1 "Protected-Mode Exceptions and Interrupts"
// Exceptions that push an error code: #DF(8), #TS(10), #NP(11), #SS(12),
// #GP(13), #PF(14), #AC(17), #CP(21).
// All others do not push an error code.
isr_no_error!(isr0, VEC_DIVIDE_ERROR);
isr_no_error!(isr1, VEC_DEBUG);
isr_no_error!(isr2, VEC_NMI);
isr_no_error!(isr3, VEC_BREAKPOINT);
isr_no_error!(isr4, VEC_OVERFLOW);
isr_no_error!(isr5, VEC_BOUND_RANGE);
isr_no_error!(isr6, VEC_INVALID_OPCODE);
isr_no_error!(isr7, VEC_DEVICE_NOT_AVAILABLE);
isr_with_error!(isr8, VEC_DOUBLE_FAULT); // error code always 0; IST required
isr_no_error!(isr9, VEC_COPROC_OVERRUN);
isr_with_error!(isr10, VEC_INVALID_TSS);
isr_with_error!(isr11, VEC_SEGMENT_NOT_PRESENT);
isr_with_error!(isr12, VEC_STACK_FAULT);
isr_with_error!(isr13, VEC_GENERAL_PROTECTION);
isr_with_error!(isr14, VEC_PAGE_FAULT);
isr_no_error!(isr15, VEC_RESERVED_15);
isr_no_error!(isr16, VEC_X87_FP);
isr_with_error!(isr17, VEC_ALIGNMENT_CHECK);
isr_no_error!(isr18, VEC_MACHINE_CHECK);
isr_no_error!(isr19, VEC_SIMD_FP);
isr_no_error!(isr20, VEC_VIRTUALIZATION);
isr_with_error!(isr21, VEC_CONTROL_PROTECTION);
// Vectors 22–31 are reserved; install generic no-error handlers.
isr_no_error!(isr22, 22u8);
isr_no_error!(isr23, 23u8);
isr_no_error!(isr24, 24u8);
isr_no_error!(isr25, 25u8);
isr_no_error!(isr26, 26u8);
isr_no_error!(isr27, 27u8);
isr_no_error!(isr28, 28u8);
isr_no_error!(isr29, 29u8);
isr_no_error!(isr30, 30u8);
isr_no_error!(isr31, 31u8);

isr_no_error!(isr_legacy_irq0, LEGACY_IRQ_VECTOR_BASE);
isr_no_error!(isr_legacy_irq1, LEGACY_IRQ_VECTOR_BASE + 1);
isr_no_error!(isr_legacy_irq2, LEGACY_IRQ_VECTOR_BASE + 2);
isr_no_error!(isr_legacy_irq3, LEGACY_IRQ_VECTOR_BASE + 3);
isr_no_error!(isr_legacy_irq4, LEGACY_IRQ_VECTOR_BASE + 4);
isr_no_error!(isr_legacy_irq5, LEGACY_IRQ_VECTOR_BASE + 5);
isr_no_error!(isr_legacy_irq6, LEGACY_IRQ_VECTOR_BASE + 6);
isr_no_error!(isr_legacy_irq7, LEGACY_IRQ_VECTOR_BASE + 7);
isr_no_error!(isr_legacy_irq8, LEGACY_IRQ_VECTOR_BASE + 8);
isr_no_error!(isr_legacy_irq9, LEGACY_IRQ_VECTOR_BASE + 9);
isr_no_error!(isr_legacy_irq10, LEGACY_IRQ_VECTOR_BASE + 10);
isr_no_error!(isr_legacy_irq11, LEGACY_IRQ_VECTOR_BASE + 11);
isr_no_error!(isr_legacy_irq12, LEGACY_IRQ_VECTOR_BASE + 12);
isr_no_error!(isr_legacy_irq13, LEGACY_IRQ_VECTOR_BASE + 13);
isr_no_error!(isr_legacy_irq14, LEGACY_IRQ_VECTOR_BASE + 14);
isr_no_error!(isr_legacy_irq15, LEGACY_IRQ_VECTOR_BASE + 15);

// IPI ping stub (Milestone 5 — SMP "CPU ping" test).
// Vector 0xF0 is sent by the BSP to an AP to verify IPI delivery.
isr_no_error!(isr_ipi_ping, IPI_PING_VECTOR);

// LAPIC timer stub (Milestone 6 — periodic system tick).
// Vector 0x40, fired by the BSP's LAPIC timer in periodic mode.
isr_no_error!(isr_timer, TIMER_VECTOR);

// TLB shootdown IPI stub (Milestone 6 — SMP TLB invalidation).
// Vector 0xF1, sent from `tlb::flush_tlb_others` to remote CPUs.
isr_no_error!(isr_tlb_shootdown, TLB_SHOOTDOWN_VECTOR);
isr_no_error!(isr_reschedule, RESCHEDULE_VECTOR);
isr_no_error!(isr_text_poke_sync, TEXT_POKE_SYNC_VECTOR);

// ── Exception handler helpers ────────────────────────────────────────────────

/// Exception names for logging (indexed by vector number).
static EXCEPTION_NAMES: [&str; 22] = [
    "#DE Divide Error",
    "#DB Debug",
    "NMI",
    "#BP Breakpoint",
    "#OF Overflow",
    "#BR BOUND Range",
    "#UD Invalid Opcode",
    "#NM Device Not Available",
    "#DF Double Fault",
    "Coprocessor Segment Overrun",
    "#TS Invalid TSS",
    "#NP Segment Not Present",
    "#SS Stack Fault",
    "#GP General Protection",
    "#PF Page Fault",
    "Reserved (15)",
    "#MF x87 FP Error",
    "#AC Alignment Check",
    "#MC Machine Check",
    "#XM SIMD FP",
    "#VE Virtualization",
    "#CP Control Protection",
];

fn exception_name(vector: u8) -> &'static str {
    EXCEPTION_NAMES
        .get(vector as usize)
        .copied()
        .unwrap_or("Reserved")
}

// ── Rust exception dispatcher ────────────────────────────────────────────────

/// Called from `isr_common` with RDI = &ExceptionFrame.
///
/// # Safety
/// The frame pointer must point to a valid `ExceptionFrame` constructed by
/// the `isr_common` stub — never call this from Rust code directly.
extern "C" fn exception_dispatch(frame: *mut ExceptionFrame) {
    // SAFETY: frame is constructed by isr_common on the exception stack.
    let frame = unsafe { &mut *frame };
    let vector = frame.vector as u8;
    let is_irq = matches!(
        vector,
        IPI_PING_VECTOR
            | TIMER_VECTOR
            | TLB_SHOOTDOWN_VECTOR
            | RESCHEDULE_VECTOR
            | TEXT_POKE_SYNC_VECTOR
    ) || legacy_irq_line(vector).is_some();
    if is_irq {
        crate::kernel::locking::preempt::__irq_enter_raw();
    }
    match vector {
        VEC_DEBUG => on_debug(frame),
        VEC_NMI => {
            crate::arch::x86::kernel::nmi::exc_nmi(frame);
        }
        VEC_BREAKPOINT => on_breakpoint(frame),
        VEC_PAGE_FAULT => on_page_fault(frame),
        VEC_GENERAL_PROTECTION => on_general_protection(frame),
        VEC_CONTROL_PROTECTION => on_control_protection(frame),
        VEC_DOUBLE_FAULT => on_double_fault(frame),
        VEC_MACHINE_CHECK => on_machine_check(frame),
        IPI_PING_VECTOR => on_ipi_ping(),
        TIMER_VECTOR => on_timer_interrupt(frame),
        TLB_SHOOTDOWN_VECTOR => on_tlb_shootdown_ipi(),
        RESCHEDULE_VECTOR => on_reschedule_ipi(),
        TEXT_POKE_SYNC_VECTOR => on_text_poke_sync_ipi(),
        v if legacy_irq_line(v).is_some() => on_legacy_irq(v),
        v => on_generic(frame, v),
    }
    if is_irq {
        irq_exit_resched(frame);
    }
}

fn on_debug(frame: &mut ExceptionFrame) {
    if crate::arch::x86::kernel::kprobes::core::kprobe_debug_handler(frame) {
        return;
    }
    on_generic(frame, VEC_DEBUG);
}

/// LAPIC timer ISR — runs on every periodic tick (Milestone 6).
///
/// Keeps the work in the ISR minimal: bumps the global tick counter (which may
/// raise a softirq) and signals EOI.  Heavier deferred work is drained from
/// the BSP idle loop in `halt_loop_with_softirq`, not from inside the ISR,
/// because interrupt gates clear IF and Linux's `__do_softirq` requires IRQs
/// to be re-enabled — see `apic_timer::on_tick` for the divergence note.
fn on_timer_interrupt(frame: &ExceptionFrame) {
    crate::arch::x86::kernel::apic_timer::on_tick(Some(frame));
    // SAFETY: ISR context — LAPIC MMIO is identity-mapped and we own EOI.
    unsafe {
        crate::arch::x86::kernel::apic::eoi();
    }
}

/// TLB shootdown IPI handler (Milestone 6, structural stub).
///
/// We currently have no kernel page table mutations, so the actual `invlpg`
/// is deferred to a future milestone.  The handler just bumps the receive +
/// ack counters so `flush_tlb_others` can prove end-to-end IPI delivery.
fn on_tlb_shootdown_ipi() {
    crate::arch::x86::mm::tlb::on_shootdown_ipi();
    // tlb::on_shootdown_ipi already issues EOI internally.
}

fn on_reschedule_ipi() {
    unsafe {
        crate::arch::x86::kernel::apic::eoi();
    }
}

fn on_text_poke_sync_ipi() {
    crate::arch::x86::kernel::alternative::text_poke_sync_ipi_handler();
}

fn legacy_irq_line(vector: u8) -> Option<u8> {
    if (LEGACY_IRQ_VECTOR_BASE..=LEGACY_IRQ_VECTOR_LAST).contains(&vector) {
        Some(vector - LEGACY_IRQ_VECTOR_BASE)
    } else {
        None
    }
}

/// Count of real device hard-IRQs (non-timer) delivered via the IDT, plus the
/// last such line. Diagnostic for whether the AHCI completion interrupt is
/// actually delivered (vs. completion only being found by the software poller).
pub static DEVICE_HARDIRQ_COUNT: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);
pub static LAST_DEVICE_HARDIRQ_LINE: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

fn on_legacy_irq(vector: u8) {
    let Some(irq) = legacy_irq_line(vector) else {
        return;
    };
    if irq != 0 {
        DEVICE_HARDIRQ_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        LAST_DEVICE_HARDIRQ_LINE.store(irq as u32, core::sync::atomic::Ordering::Relaxed);
    }
    let _ = crate::kernel::irq::generic_handle_irq(irq as u32);
    unsafe {
        crate::arch::x86::kernel::pic::send_eoi(irq);
        crate::arch::x86::kernel::apic::eoi();
    }
}

fn irq_exit_resched(frame: &ExceptionFrame) {
    crate::kernel::locking::preempt::__irq_exit_raw();
    let current = unsafe { crate::kernel::sched::get_current() };
    if current.is_null() {
        return;
    }
    let need_resched =
        unsafe { (*current).thread_info.flags & crate::kernel::task::TIF_NEED_RESCHED != 0 };
    if should_irq_exit_resched(
        frame,
        need_resched,
        crate::kernel::sched::production_smp_scheduler_enabled(),
        crate::kernel::locking::preempt::preempt_count(),
        crate::kernel::locking::preempt::in_irq(),
    ) {
        crate::kernel::locking::local_irq_enable();
        unsafe {
            crate::kernel::sched::schedule();
        }
        crate::kernel::locking::local_irq_disable();
    }
}

fn should_irq_exit_resched(
    frame: &ExceptionFrame,
    need_resched: bool,
    _production_smp_scheduler: bool,
    preempt_count: u32,
    in_irq: bool,
) -> bool {
    if !need_resched || preempt_count != 0 || in_irq {
        return false;
    }

    // Linux may preempt kernel code because its spinlocks and preemptible
    // regions update preempt_count. Lupos still has Rust-side spin locks that
    // do not participate in preempt_count, so arbitrary kernel IRQ-exit
    // preemption can switch away while a lock is held. Keep kernel-mode
    // scheduling cooperative until those lock primitives are fully wired.
    is_user_exception(frame)
}

// ── Per-exception handlers ───────────────────────────────────────────────────

/// #BP — Breakpoint (`int3`).
///
/// In a debugger-free kernel context we just log it.  A future milestone will
/// hook this into a kernel debugger / kprobe infrastructure.
fn on_breakpoint(frame: &ExceptionFrame) {
    if crate::arch::x86::kernel::alternative::text_poke_bp_handler(frame) {
        return;
    }
    if crate::arch::x86::kernel::kprobes::core::kprobe_int3_handler(frame) {
        return;
    }
    log_warn!("cpu", "cpu: #BP Breakpoint at rip={:#018x}", frame.rip);
    // Breakpoint is a trap — RIP points to the instruction *after* `int3`,
    // so returning from the handler (via `iretq`) continues execution.
}

/// #PF — Page Fault.
///
/// CR2 holds the faulting virtual address at the time the fault is delivered.
/// The error code bits encode the nature of the fault:
///   bit 0 (P)  — 0 = page not present, 1 = protection violation
///   bit 1 (W)  — 0 = read access, 1 = write access
///   bit 2 (U)  — 0 = supervisor, 1 = user mode
///   bit 3 (R)  — reserved bit violation in a PTE
///   bit 4 (I)  — instruction fetch (NX violation)
///
/// Reference: Intel SDM Vol. 3A §6.15 "Interrupt 14 — Page-Fault Exception"
fn on_page_fault(frame: &ExceptionFrame) {
    // M59: extable fixup.  If the faulting RIP is registered in __ex_table
    // (via the .pushsection __ex_table directives in arch/x86/uaccess.rs),
    // redirect execution to the fixup label instead of taking the fault.
    // RCX holds the bytes-not-copied count at the fixup site, so user-copy
    // routines return cleanly with that value.
    //
    // SAFETY: `frame` is the iretq frame on the kernel stack — writing
    // through a *mut alias mutates the saved RIP that iretq will pop.
    //
    // Ref: vendor/linux/arch/x86/mm/extable.c::fixup_exception
    if let Some(fixup) = super::extable::search_extable(frame.rip) {
        unsafe {
            let frame_mut = frame as *const ExceptionFrame as *mut ExceptionFrame;
            (*frame_mut).rip = fixup;
        }
        return;
    }

    crate::arch::x86::mm::fault::do_page_fault(frame);
}

/// #GP — General Protection Fault.
///
/// Caused by segment violations, privilege violations, or misaligned operands.
/// The error code encodes the segment selector that caused the fault (or 0).
fn on_general_protection(frame: &ExceptionFrame) {
    log_error!(
        "cpu",
        "cpu: #GP General Protection error={:#010x} rip={:#018x} cs={:#06x} rsp={:#018x}",
        frame.error_code,
        frame.rip,
        frame.cs,
        frame.user_rsp
    );
    log_error!(
        "cpu",
        "cpu: #GP regs rax={:#018x} rbx={:#018x} rcx={:#018x} rdx={:#018x}",
        frame.rax,
        frame.rbx,
        frame.rcx,
        frame.rdx
    );
    log_error!(
        "cpu",
        "cpu: #GP regs rsi={:#018x} rdi={:#018x} rbp={:#018x} r8={:#018x}",
        frame.rsi,
        frame.rdi,
        frame.rbp,
        frame.r8
    );
    log_error!(
        "cpu",
        "cpu: #GP regs r9={:#018x} r10={:#018x} r11={:#018x} r12={:#018x}",
        frame.r9,
        frame.r10,
        frame.r11,
        frame.r12
    );
    if is_user_exception(frame) {
        let task = unsafe { crate::kernel::sched::get_current() };
        if !task.is_null() {
            let pid = unsafe { (*task).pid };
            if pid == 1 {
                panic!(
                    "init died from SIGSEGV on #GP: error={:#010x} rip={:#018x}",
                    frame.error_code, frame.rip
                );
            }
        }
        unsafe {
            crate::kernel::exit::do_exit(segv_exit_code() as i64);
        }
    }
    panic!("General Protection Fault");
}

fn on_control_protection(frame: &ExceptionFrame) {
    use crate::arch::x86::kernel::cet::{
        CP_ENDBR, ControlProtectionAction, exc_control_protection_action, kernel_ibt_enabled,
    };

    let action = exc_control_protection_action(
        is_user_exception(frame),
        false,
        kernel_ibt_enabled(),
        true,
        false,
        frame.error_code,
    );
    match action {
        ControlProtectionAction::ForceSigsegv { code } => panic!(
            "user control-protection fault at rip={:#018x}, error={:#x}, si_code={code}",
            frame.rip, frame.error_code
        ),
        ControlProtectionAction::KernelBug
            if frame.error_code & crate::arch::x86::kernel::cet::CP_EC == CP_ENDBR =>
        {
            panic!(
                "kernel IBT violation: missing ENDBR at rip={:#018x}, error={:#x}",
                frame.rip, frame.error_code
            )
        }
        action => panic!(
            "unexpected control-protection fault at rip={:#018x}, error={:#x}, action={action:?}",
            frame.rip, frame.error_code
        ),
    }
}

fn is_user_exception(frame: &ExceptionFrame) -> bool {
    (frame.cs & 3) == 3
}

fn segv_exit_code() -> i32 {
    crate::kernel::wait::w_exitcode(0, crate::kernel::signal::SIGSEGV)
}

fn ill_exit_code() -> i32 {
    crate::kernel::wait::w_exitcode(0, crate::kernel::signal::SIGILL)
}

/// #DF — Double Fault.
///
/// Fires when the CPU encounters an error while trying to deliver another
/// exception (e.g., stack overflow during #PF handling → #SS → #DF).
/// This handler runs on the dedicated IST1 stack (see `tss.rs`).
/// A double fault is an *abort* — the program that caused it is unrecoverable.
fn on_double_fault(frame: &ExceptionFrame) {
    log_error!(
        "cpu",
        "cpu: #DF Double Fault (error={:#x}) rip={:#018x}",
        frame.error_code,
        frame.rip
    );
    let switch = crate::arch::x86::kernel::switch::last_switch_attempt();
    log_error!(
        "cpu",
        "cpu: #DF switch seq={} prev={:#018x} pid={}",
        switch.sequence,
        switch.prev,
        switch.prev_pid
    );
    log_error!(
        "cpu",
        "cpu: #DF prev sp={:#018x} stack={:#018x} used={}",
        switch.prev_sp,
        switch.prev_stack,
        switch.prev_stack.saturating_sub(switch.prev_sp)
    );
    log_error!(
        "cpu",
        "cpu: #DF next={:#018x} pid={}",
        switch.next,
        switch.next_pid
    );
    log_error!(
        "cpu",
        "cpu: #DF next sp={:#018x} stack={:#018x} used={}",
        switch.next_sp,
        switch.next_stack,
        switch.next_stack.saturating_sub(switch.next_sp)
    );
    panic!("Double Fault - kernel stack corrupted?");
}

/// #MC — Machine Check.
///
/// Hardware-reported error (ECC, bus errors, thermal events).  Asynchronous
/// and potentially unrecoverable.  Runs on IST3.
fn on_machine_check(frame: &ExceptionFrame) {
    log_error!("cpu", "cpu: #MC Machine Check rip={:#018x}", frame.rip);
    panic!("Machine Check Exception");
}

/// IPI ping handler (Milestone 5 — SMP "CPU ping" test).
///
/// The BSP sends a fixed IPI at vector 0xF0 to an AP to verify that
/// inter-processor interrupts work correctly.  The AP increments a shared
/// atomic counter and sends LAPIC EOI to acknowledge the interrupt.
///
/// # Ordering
/// The `Release` fence in `fetch_add` ensures that all AP setup prior to
/// this point (LAPIC init, ready-count increment) is visible to the BSP
/// when it observes the counter increment via `Acquire` load.
fn on_ipi_ping() {
    use core::sync::atomic::Ordering;
    // Increment the counter so the BSP knows the IPI was received.
    crate::arch::x86::kernel::smp::IPI_RECEIVED_COUNT.fetch_add(1, Ordering::Release);
    // Send LAPIC End-of-Interrupt to clear the in-service bit for this vector.
    // Without EOI, the LAPIC will not deliver further interrupts at or below
    // this priority on this CPU.
    // Reference: Intel SDM Vol. 3A §10.8.5 "Signaling Interrupt Servicing Completion"
    unsafe {
        crate::arch::x86::kernel::apic::eoi();
    }
}

/// Generic handler for all other CPU exceptions.
fn on_generic(frame: &mut ExceptionFrame, vector: u8) {
    if vector == VEC_INVALID_OPCODE {
        if is_user_exception(frame) {
            let task = unsafe { crate::kernel::sched::get_current() };
            if !task.is_null() {
                let pid = unsafe { (*task).pid };
                if pid == 1 {
                    panic!("init died from SIGILL on #UD: rip={:#018x}", frame.rip);
                }
            }
            unsafe {
                crate::kernel::exit::do_exit(ill_exit_code() as i64);
            }
        }

        // vendor/linux/arch/x86/kernel/traps.c::handle_bug treats UD2 as a
        // compact WARN/BUG call before ordinary exception entry. Restore the
        // interrupted IF state while reporting, then advance past UD2 only
        // for a recoverable warning. A real BUG remains at its faulting RIP
        // and falls through to the fatal invalid-opcode path below.
        let interrupted_irqs_enabled = frame.rflags & (1 << 9) != 0;
        if interrupted_irqs_enabled {
            crate::kernel::locking::local_irq_enable();
        }
        let bug_trap = crate::kernel::bug::report_bug(frame.rip as usize);
        if interrupted_irqs_enabled {
            crate::kernel::locking::local_irq_disable();
        }
        match bug_trap {
            crate::kernel::bug::BugTrapType::Warn => {
                // Linux handle_bug() resumes at ip + the decoded insn length:
                // 2 for a plain UD2 WARN, 5 for the __WARN_trap WARNINSN
                // (vendor/linux/arch/x86/kernel/traps.c::decode_bug).
                frame.rip = frame
                    .rip
                    .wrapping_add(crate::kernel::bug::bug_insn_len(frame.rip as usize) as u64);
                return;
            }
            crate::kernel::bug::BugTrapType::Bug | crate::kernel::bug::BugTrapType::None => {}
        }

        let interrupted_sp =
            unsafe { (frame as *const ExceptionFrame as *const u8).add(160) as *const u64 };
        log_error!(
            "cpu",
            "cpu: #UD regs rbx={:#018x} rbp={:#018x} stack={:#018x} pseudo_rsp={:#018x} rax={:#018x} rdi={:#018x} rsi={:#018x}",
            frame.rbx,
            frame.rbp,
            interrupted_sp as u64,
            frame.user_rsp,
            frame.rax,
            frame.rdi,
            frame.rsi
        );
        if !crate::kernel::module::with_module_address(
            frame.rip as usize,
            |module, section, offset| {
                log_error!(
                    "cpu",
                    "cpu: #UD module {}:{}+{:#x}",
                    module,
                    section,
                    offset
                );
            },
        ) {
            log_error!("cpu", "cpu: #UD module <kernel/unknown>");
        }
        if frame.rbx >= crate::arch::x86::mm::paging::PAGE_OFFSET {
            unsafe {
                let shost = frame.rbx as *const u8;
                let host_failed = core::ptr::read_unaligned(shost.add(0x1e4) as *const i32);
                let host_eh_scheduled = core::ptr::read_unaligned(shost.add(0x1e8) as *const u32);
                let shost_state = core::ptr::read_unaligned(shost.add(0x298) as *const u32);
                log_error!(
                    "cpu",
                    "cpu: #UD scsi_host? ptr={:#018x} host_failed={} host_eh_scheduled={} shost_state={}",
                    frame.rbx,
                    host_failed,
                    host_eh_scheduled,
                    shost_state
                );
            }
        }
        let mut rbp = frame.rbp as *const u64;
        for depth in 0..8 {
            let rbp_addr = rbp as u64;
            if rbp_addr < crate::arch::x86::mm::paging::PAGE_OFFSET || rbp_addr & 0x7 != 0 {
                break;
            }
            unsafe {
                let next = core::ptr::read_unaligned(rbp);
                let ret = core::ptr::read_unaligned(rbp.add(1));
                log_error!(
                    "cpu",
                    "cpu: #UD bt{} rbp={:#018x} ret={:#018x}",
                    depth,
                    rbp_addr,
                    ret
                );
                let _ = crate::kernel::module::with_module_address(
                    ret as usize,
                    |module, section, offset| {
                        log_error!(
                            "cpu",
                            "cpu: #UD bt{} module {}:{}+{:#x}",
                            depth,
                            module,
                            section,
                            offset
                        );
                    },
                );
                if next <= rbp_addr {
                    break;
                }
                rbp = next as *const u64;
            }
        }
        for i in 0..96usize {
            unsafe {
                let word = core::ptr::read_unaligned(interrupted_sp.add(i));
                if (0x0020_0000..0x0100_0000).contains(&word) {
                    log_error!("cpu", "cpu: #UD stack[{}] potential-ret={:#018x}", i, word);
                }
                let _ = crate::kernel::module::with_module_address(
                    word as usize,
                    |module, section, offset| {
                        log_error!(
                            "cpu",
                            "cpu: #UD stack[{}] module {}:{}+{:#x}",
                            i,
                            module,
                            section,
                            offset
                        );
                    },
                );
            }
        }
    }
    log_error!(
        "cpu",
        "cpu: {} (vec={}) error={:#x} rip={:#018x}",
        exception_name(vector),
        vector,
        frame.error_code,
        frame.rip
    );
    panic!("Unhandled CPU Exception vec={}", vector);
}

// ── IDT initialization ───────────────────────────────────────────────────────

/// Install all exception handlers and load the IDT.
///
/// Must be called after `gdt::init()` (the IDT entries reference `KERNEL_CS`).
///
/// # Safety
/// - Single-threaded init path only.
/// - All ISR stubs must be in executable memory.
pub unsafe fn init() {
    let cs = sel::KERNEL_CS;
    // Rust 2024: raw references to static muts don't need unsafe.
    let idt = &raw mut IDT;

    //  Standard exception gates
    let idt = unsafe { &mut *idt };
    idt.set(VEC_DIVIDE_ERROR, IdtEntry::interrupt_gate(isr0, cs, 0));
    idt.set(VEC_DEBUG, IdtEntry::interrupt_gate(isr1, cs, 0));
    // NMI on IST2 — can arrive asynchronously at any time
    idt.set(VEC_NMI, IdtEntry::interrupt_gate(isr2, cs, IST_NMI));
    // Breakpoint: trap gate (preserves IF) + DPL=3 (user-accessible via int3)
    idt.set(VEC_BREAKPOINT, IdtEntry::user_trap_gate(isr3, cs, 0));
    idt.set(VEC_OVERFLOW, IdtEntry::trap_gate(isr4, cs, 0));
    idt.set(VEC_BOUND_RANGE, IdtEntry::interrupt_gate(isr5, cs, 0));
    idt.set(VEC_INVALID_OPCODE, IdtEntry::interrupt_gate(isr6, cs, 0));
    idt.set(
        VEC_DEVICE_NOT_AVAILABLE,
        IdtEntry::interrupt_gate(isr7, cs, 0),
    );
    // Double Fault MUST use IST1 — it fires when the main kernel stack is
    // corrupt, so we need a dedicated, known-good stack.
    idt.set(
        VEC_DOUBLE_FAULT,
        IdtEntry::interrupt_gate(isr8, cs, IST_DOUBLE_FAULT),
    );
    idt.set(VEC_COPROC_OVERRUN, IdtEntry::interrupt_gate(isr9, cs, 0));
    idt.set(VEC_INVALID_TSS, IdtEntry::interrupt_gate(isr10, cs, 0));
    idt.set(
        VEC_SEGMENT_NOT_PRESENT,
        IdtEntry::interrupt_gate(isr11, cs, 0),
    );
    idt.set(VEC_STACK_FAULT, IdtEntry::interrupt_gate(isr12, cs, 0));
    idt.set(
        VEC_GENERAL_PROTECTION,
        IdtEntry::interrupt_gate(isr13, cs, 0),
    );
    idt.set(VEC_PAGE_FAULT, IdtEntry::interrupt_gate(isr14, cs, 0));
    idt.set(VEC_RESERVED_15, IdtEntry::interrupt_gate(isr15, cs, 0));
    idt.set(VEC_X87_FP, IdtEntry::interrupt_gate(isr16, cs, 0));
    idt.set(VEC_ALIGNMENT_CHECK, IdtEntry::interrupt_gate(isr17, cs, 0));
    // Machine Check on IST3 — hardware error, asynchronous
    idt.set(
        VEC_MACHINE_CHECK,
        IdtEntry::interrupt_gate(isr18, cs, IST_MACHINE_CHECK),
    );
    idt.set(VEC_SIMD_FP, IdtEntry::interrupt_gate(isr19, cs, 0));
    idt.set(VEC_VIRTUALIZATION, IdtEntry::interrupt_gate(isr20, cs, 0));
    idt.set(
        VEC_CONTROL_PROTECTION,
        IdtEntry::interrupt_gate(isr21, cs, 0),
    );
    idt.set(22, IdtEntry::interrupt_gate(isr22, cs, 0));
    idt.set(23, IdtEntry::interrupt_gate(isr23, cs, 0));
    idt.set(24, IdtEntry::interrupt_gate(isr24, cs, 0));
    idt.set(25, IdtEntry::interrupt_gate(isr25, cs, 0));
    idt.set(26, IdtEntry::interrupt_gate(isr26, cs, 0));
    idt.set(27, IdtEntry::interrupt_gate(isr27, cs, 0));
    idt.set(28, IdtEntry::interrupt_gate(isr28, cs, 0));
    idt.set(29, IdtEntry::interrupt_gate(isr29, cs, 0));
    idt.set(30, IdtEntry::interrupt_gate(isr30, cs, 0));
    idt.set(31, IdtEntry::interrupt_gate(isr31, cs, 0));

    idt.set(
        LEGACY_IRQ_VECTOR_BASE,
        IdtEntry::interrupt_gate(isr_legacy_irq0, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 1,
        IdtEntry::interrupt_gate(isr_legacy_irq1, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 2,
        IdtEntry::interrupt_gate(isr_legacy_irq2, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 3,
        IdtEntry::interrupt_gate(isr_legacy_irq3, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 4,
        IdtEntry::interrupt_gate(isr_legacy_irq4, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 5,
        IdtEntry::interrupt_gate(isr_legacy_irq5, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 6,
        IdtEntry::interrupt_gate(isr_legacy_irq6, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 7,
        IdtEntry::interrupt_gate(isr_legacy_irq7, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 8,
        IdtEntry::interrupt_gate(isr_legacy_irq8, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 9,
        IdtEntry::interrupt_gate(isr_legacy_irq9, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 10,
        IdtEntry::interrupt_gate(isr_legacy_irq10, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 11,
        IdtEntry::interrupt_gate(isr_legacy_irq11, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 12,
        IdtEntry::interrupt_gate(isr_legacy_irq12, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 13,
        IdtEntry::interrupt_gate(isr_legacy_irq13, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 14,
        IdtEntry::interrupt_gate(isr_legacy_irq14, cs, 0),
    );
    idt.set(
        LEGACY_IRQ_VECTOR_BASE + 15,
        IdtEntry::interrupt_gate(isr_legacy_irq15, cs, 0),
    );

    // ── IPI vector (Milestone 5) ─────────────────────────────────────────
    // Vector 0xF0: SMP "CPU ping" test — BSP→AP IPI, handler increments
    // IPI_RECEIVED_COUNT and sends LAPIC EOI.
    idt.set(
        IPI_PING_VECTOR,
        IdtEntry::interrupt_gate(isr_ipi_ping, cs, 0),
    );

    // ── External vectors (Milestone 6) ───────────────────────────────────
    // Vector 0x40: LAPIC timer tick — periodic interrupt from the BSP's
    // local APIC timer.  Handler bumps TIMER_TICKS and raises a softirq.
    idt.set(TIMER_VECTOR, IdtEntry::interrupt_gate(isr_timer, cs, 0));

    // Vector 0xF1: TLB shootdown IPI — sent by `tlb::flush_tlb_others`.
    // Handler bumps the shootdown ack counter and sends EOI.  Real `invlpg`
    // is deferred until kernel page tables become mutable (M9+).
    idt.set(
        TLB_SHOOTDOWN_VECTOR,
        IdtEntry::interrupt_gate(isr_tlb_shootdown, cs, 0),
    );
    idt.set(
        RESCHEDULE_VECTOR,
        IdtEntry::interrupt_gate(isr_reschedule, cs, 0),
    );
    idt.set(
        TEXT_POKE_SYNC_VECTOR,
        IdtEntry::interrupt_gate(isr_text_poke_sync, cs, 0),
    );

    // Load the IDT.
    unsafe {
        Idt::load(&raw const IDT);
    }
}

pub unsafe fn send_reschedule_ipi(target_id: u8) {
    unsafe {
        crate::arch::x86::kernel::apic::send_ipi(target_id, RESCHEDULE_VECTOR);
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    // ── IdtEntry layout ──────────────────────────────────────────────────────

    #[test]
    fn idt_entry_is_16_bytes() {
        assert_eq!(
            size_of::<IdtEntry>(),
            16,
            "IDT gate must be 16 bytes (SDM §6.14.1)"
        );
    }

    #[test]
    fn idt_size_is_4096_bytes() {
        // 256 entries × 16 bytes = 4096 bytes = 1 page.
        assert_eq!(size_of::<Idt>(), 4096);
    }

    #[test]
    fn absent_entry_is_all_zero_except_for_cs() {
        // type_attr P=0 means not-present; handler and reserved are zero.
        let e = IdtEntry::absent();
        assert_eq!(e.type_attr, 0, "absent entry must have P=0");
        assert_eq!(e.handler_low, 0);
        assert_eq!(e.handler_mid, 0);
        assert_eq!(e.handler_high, 0);
        assert_eq!(e._reserved, 0);
    }

    #[test]
    fn interrupt_gate_encodes_handler_address_correctly() {
        // Use a known address and verify all three address slices.
        let addr: u64 = 0x_DEAD_BEEF_1234_5678;

        // We can't use a real function pointer here (naked fns can't be called),
        // so we use transmute to convert the u64 address to the expected type.
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(addr) };
        let entry = IdtEntry::interrupt_gate(handler, sel::KERNEL_CS, 0);

        assert_eq!(entry.handler_low, (addr & 0xFFFF) as u16, "low 16 bits");
        assert_eq!(
            entry.handler_mid,
            ((addr >> 16) & 0xFFFF) as u16,
            "mid 16 bits"
        );
        assert_eq!(
            entry.handler_high,
            ((addr >> 32) & 0xFFFF_FFFF) as u32,
            "high 32 bits"
        );
    }

    #[test]
    fn interrupt_gate_type_attr_is_0x8e() {
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(1u64) };
        let e = IdtEntry::interrupt_gate(handler, sel::KERNEL_CS, 0);
        assert_eq!(
            e.type_attr, 0x8E,
            "interrupt gate type_attr = P(1)|DPL(0)|0|type(E)"
        );
    }

    #[test]
    fn trap_gate_type_attr_is_0x8f() {
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(1u64) };
        let e = IdtEntry::trap_gate(handler, sel::KERNEL_CS, 0);
        assert_eq!(
            e.type_attr, 0x8F,
            "trap gate type_attr = P(1)|DPL(0)|0|type(F)"
        );
    }

    #[test]
    fn user_trap_gate_has_dpl3() {
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(1u64) };
        let e = IdtEntry::user_trap_gate(handler, sel::KERNEL_CS, 0);
        let dpl = (e.type_attr >> 5) & 0x3;
        assert_eq!(dpl, 3, "user trap gate DPL must be 3 (SDM §6.12.1.3)");
    }

    #[test]
    fn interrupt_gate_cs_stored_correctly() {
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(1u64) };
        let e = IdtEntry::interrupt_gate(handler, sel::KERNEL_CS, 0);
        assert_eq!(e.cs, sel::KERNEL_CS, "IDT entry CS must be KERNEL_CS");
    }

    #[test]
    fn interrupt_gate_ist_clipped_to_3_bits() {
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(1u64) };
        // IST field is 3 bits (0–7); input 0xFF should be masked to 0x7.
        let e = IdtEntry::interrupt_gate(handler, sel::KERNEL_CS, 0xFF);
        assert_eq!(e.ist, 7, "IST field must be clipped to 3 bits");
    }

    #[test]
    fn interrupt_gate_reserved_is_zero() {
        let handler: unsafe extern "C" fn() = unsafe { core::mem::transmute(u64::MAX) };
        let e = IdtEntry::interrupt_gate(handler, sel::KERNEL_CS, 0);
        assert_eq!(e._reserved, 0, "IDT gate reserved field must be zero");
    }

    // ── Exception frame layout ───────────────────────────────────────────────

    #[test]
    fn idtr_operand_is_10_bytes() {
        assert_eq!(
            size_of::<IdtRegister>(),
            10,
            "IDTR operand must be 10 bytes"
        );
        assert_eq!(
            offset_of!(IdtRegister, limit),
            0,
            "IDTR limit must start at byte 0"
        );
        assert_eq!(
            offset_of!(IdtRegister, base),
            2,
            "IDTR base must start at byte 2"
        );
    }

    #[test]
    fn exception_frame_vector_at_correct_offset() {
        // After 15 GP registers (15 × 8 = 120 bytes), `vector` is at offset 120.
        assert_eq!(offset_of!(ExceptionFrame, vector), 120);
    }

    #[test]
    fn exception_frame_error_code_after_vector() {
        assert_eq!(offset_of!(ExceptionFrame, error_code), 128);
    }

    #[test]
    fn exception_frame_rip_at_offset_136() {
        // RIP is the first CPU-pushed field, after vector(8) + error_code(8) = 16 bytes
        // added by our stubs, plus 15 × 8 = 120 bytes for saved registers.
        assert_eq!(offset_of!(ExceptionFrame, rip), 136);
    }

    // ── Vector constant sanity ───────────────────────────────────────────────

    fn test_exception_frame(cs: u64) -> ExceptionFrame {
        ExceptionFrame {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            r10: 0,
            r9: 0,
            r8: 0,
            rdi: 0,
            rsi: 0,
            rbp: 0,
            rdx: 0,
            rcx: 0,
            rbx: 0,
            rax: 0,
            vector: VEC_GENERAL_PROTECTION as u64,
            error_code: 0,
            rip: 0x4000,
            cs,
            rflags: 0x202,
            user_rsp: 0x8000,
            user_ss: sel::USER_DS as u64,
        }
    }

    #[test]
    fn user_exception_is_detected_from_rpl() {
        assert!(is_user_exception(&test_exception_frame(
            sel::USER_CS as u64
        )));
        assert!(!is_user_exception(&test_exception_frame(
            sel::KERNEL_CS as u64
        )));
    }

    #[test]
    fn isr_common_swaps_gs_around_user_frames() {
        let source = include_str!("idt.rs");
        let body = source
            .split("unsafe extern \"C\" fn isr_common()")
            .nth(1)
            .expect("isr_common must exist");
        let entry_test = body
            .find("\"test qword ptr [rsp + 24], 3\"")
            .expect("entry must test interrupted CS before swapgs");
        let entry_swapgs = body[entry_test..]
            .find("\"swapgs\"")
            .map(|off| entry_test + off)
            .expect("entry must swapgs for user frames");
        let exit_skip = body
            .find("\"add rsp, 16\"")
            .expect("exit must skip vector and error code");
        let exit_test = body[exit_skip..]
            .find("\"test qword ptr [rsp + 8], 3\"")
            .map(|off| exit_skip + off)
            .expect("exit must test return CS before swapgs");
        let exit_swapgs = body[exit_test..]
            .find("\"swapgs\"")
            .map(|off| exit_test + off)
            .expect("exit must swapgs before iret to user");
        let iret = body[exit_swapgs..]
            .find("\"iretq\"")
            .map(|off| exit_swapgs + off)
            .expect("isr_common must return with iretq");

        assert!(entry_test < entry_swapgs);
        assert!(entry_swapgs < exit_skip);
        assert!(exit_test < exit_swapgs);
        assert!(exit_swapgs < iret);
    }

    #[test]
    fn irq_exit_resched_allows_legacy_user_return_only() {
        let user = test_exception_frame(sel::USER_CS as u64);
        let kernel = test_exception_frame(sel::KERNEL_CS as u64);

        assert!(should_irq_exit_resched(&user, true, false, 0, false));
        assert!(!should_irq_exit_resched(&kernel, true, false, 0, false));
        assert!(should_irq_exit_resched(&user, true, true, 0, false));
        assert!(!should_irq_exit_resched(&kernel, true, true, 0, false));
        assert!(!should_irq_exit_resched(&user, false, false, 0, false));
        assert!(!should_irq_exit_resched(&user, true, false, 1, false));
        assert!(!should_irq_exit_resched(&user, true, false, 0, true));
    }

    #[test]
    fn irq_exit_resched_enables_irqs_only_around_schedule() {
        let source = include_str!("idt.rs");
        let body = source
            .split("fn irq_exit_resched(frame: &ExceptionFrame)")
            .nth(1)
            .expect("irq exit reschedule helper must exist");
        let decision = body
            .find("should_irq_exit_resched(")
            .expect("irq exit must gate schedule");
        let enable = body[decision..]
            .find("crate::kernel::locking::local_irq_enable();")
            .map(|off| decision + off)
            .expect("irq-exit preemption must enable IRQs before schedule");
        let schedule = body[enable..]
            .find("crate::kernel::sched::schedule();")
            .map(|off| enable + off)
            .expect("irq exit must schedule after enabling IRQs");
        let disable = body[schedule..]
            .find("crate::kernel::locking::local_irq_disable();")
            .map(|off| schedule + off)
            .expect("irq exit must disable IRQs before IRET restore");

        assert!(decision < enable);
        assert!(enable < schedule);
        assert!(schedule < disable);
    }

    #[test]
    fn general_protection_user_exit_code_is_sigsegv() {
        assert_eq!(
            segv_exit_code(),
            crate::kernel::wait::w_exitcode(0, crate::kernel::signal::SIGSEGV)
        );
    }

    #[test]
    fn invalid_opcode_user_exit_code_is_sigill() {
        assert_eq!(
            ill_exit_code(),
            crate::kernel::wait::w_exitcode(0, crate::kernel::signal::SIGILL)
        );
    }

    #[test]
    fn vec_page_fault_is_14() {
        // The Linux ABI and all x86 OS conventions depend on #PF = 14.
        assert_eq!(VEC_PAGE_FAULT, 14);
    }

    #[test]
    fn vec_double_fault_is_8() {
        assert_eq!(VEC_DOUBLE_FAULT, 8);
    }

    #[test]
    fn ipi_ping_vector_is_in_kernel_ipi_range() {
        // 0xF0–0xFF is the conventional Linux kernel IPI vector range.
        // Our vector must fall within this range and above the PIC range (0x2F).
        assert!(IPI_PING_VECTOR >= 0xF0);
        assert!(IPI_PING_VECTOR <= 0xFF);
    }

    #[test]
    fn timer_vector_above_pic_range() {
        // The 8259 PIC remap window is 0x20–0x2F (16 IRQ lines).  The LAPIC
        // timer vector must sit above that range so legacy IRQs can never
        // collide with timer ticks once the PIC is unmasked or replaced by
        // an I/O APIC.
        assert!(TIMER_VECTOR > 0x2F);
        // Must also stay below the IPI band so we don't shadow IPIs.
        assert!(TIMER_VECTOR < 0xF0);
    }

    #[test]
    fn legacy_irq_vectors_cover_pic_window() {
        assert_eq!(legacy_irq_line(LEGACY_IRQ_VECTOR_BASE), Some(0));
        assert_eq!(legacy_irq_line(LEGACY_IRQ_VECTOR_LAST), Some(15));
        assert_eq!(legacy_irq_line(LEGACY_IRQ_VECTOR_LAST + 1), None);
    }

    #[test]
    fn tlb_vector_in_ipi_range() {
        // Linux convention: kernel IPIs occupy 0xF0–0xFF.
        assert!(TLB_SHOOTDOWN_VECTOR >= 0xF0);
        assert!(TLB_SHOOTDOWN_VECTOR <= 0xFF);
    }

    #[test]
    fn timer_and_tlb_vectors_distinct_from_ping() {
        // All three external vectors must be unique — duplicating any would
        // cause one handler to silently shadow another at IDT install time.
        assert_ne!(TIMER_VECTOR, IPI_PING_VECTOR);
        assert_ne!(TLB_SHOOTDOWN_VECTOR, IPI_PING_VECTOR);
        assert_ne!(TIMER_VECTOR, TLB_SHOOTDOWN_VECTOR);
    }
}
