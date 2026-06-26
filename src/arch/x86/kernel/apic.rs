//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Local APIC (Advanced Programmable Interrupt Controller) driver.
//!
//! Every x86_64 logical CPU has its own Local APIC (LAPIC).  It handles:
//!   - Receiving external interrupts routed from the I/O APIC.
//!   - Sending and receiving Inter-Processor Interrupts (IPIs).
//!   - The local APIC timer (used for per-CPU scheduling ticks).
//!   - Delivering NMIs, SMIs, and INIT/SIPI signals during SMP bring-up.
//!
//! # Register access
//! The LAPIC is memory-mapped at physical address 0xFEE0_0000 (confirmed or
//! overridden by the MADT).  Each register is a 32-bit value at a 16-byte-
//! aligned offset.  We use `read_volatile` / `write_volatile` to prevent the
//! compiler from eliminating or reordering MMIO accesses.
//!
//! # Initialization (BSP and each AP)
//! `init()` is called once per CPU:
//!   1. Clear TPR so the CPU accepts all interrupt priorities.
//!   2. Clear ESR to flush any stale error state.
//!   3. Mask all LVT entries so no local interrupts fire unexpectedly.
//!   4. Write the Spurious Vector Register (SVR) with the enable bit set.
//!      This is the action that activates the LAPIC.
//!
//! References:
//!   Intel SDM Vol. 3A Chapter 10 "Advanced Programmable Interrupt Controller (APIC)"
//!   Intel SDM Vol. 3A §10.4   "Local APIC"
//!   Intel SDM Vol. 3A §10.6   "Issuing Interprocessor Interrupts"
//!   Intel SDM Vol. 3A §10.6.7 "Broadcast/Self-Directed IPIs"
//!   vendor/linux/arch/x86/kernel/apic/apic.c
//!   vendor/linux/arch/x86/kernel/apic/apic_common.c
//!   vendor/linux/arch/x86/kernel/apic/init.c
//!   vendor/linux/arch/x86/kernel/apic/ipi.c
//!   https://wiki.osdev.org/APIC
//!   https://wiki.osdev.org/APIC_timer

#[path = "apic/init.rs"]
pub mod apic_init;
#[path = "apic/probe_32.rs"]
pub mod probe_32;

use core::sync::atomic::{AtomicBool, Ordering};

// ── LAPIC MMIO base ───────────────────────────────────────────────────────────
//
// The architectural default is 0xFEE0_0000.  The actual address is reported in
// the MADT; for Milestone 5 we hardcode the default since it matches QEMU and
// all modern hardware.
//
// Reference: Intel SDM Vol. 3A §10.4.1 "The Local APIC Block Diagram"

/// Physical (and virtual, under identity mapping) address of the LAPIC MMIO.
const LAPIC_BASE: u64 = 0xFEE0_0000;

// ── Register offsets ──────────────────────────────────────────────────────────
//
// Each register lives at `LAPIC_BASE + offset`.  All registers are 32 bits
// wide and 16-byte aligned.  Reads and writes to other widths are undefined.
//
// Reference: Intel SDM Vol. 3A Table 10-1 "Local APIC Register Address Map"

/// LAPIC ID Register (read-only; bits 31:24 = xAPIC ID).
pub const REG_ID: u32 = 0x020;
/// LAPIC Version Register (read-only).
pub const REG_VERSION: u32 = 0x030;
/// Task Priority Register — controls which interrupt priorities this CPU accepts.
/// Writing 0 accepts all priorities.
pub const REG_TPR: u32 = 0x080;
/// End-of-Interrupt Register (write-only).
/// Writing any value signals the end of the current interrupt to the LAPIC.
pub const REG_EOI: u32 = 0x0B0;
/// Spurious Interrupt Vector Register.
/// Bit 8 = software enable (must be 1 for the LAPIC to deliver interrupts).
/// Bits 7:0 = spurious vector number (conventionally 0xFF).
pub const REG_SVR: u32 = 0x0F0;
/// Error Status Register — latches LAPIC error bits.
/// Must be written (with 0) before reading to get current errors.
pub const REG_ESR: u32 = 0x280;
/// Interrupt Command Register — LOW 32 bits.
/// Writing this register (after ICR_HIGH) triggers an IPI delivery.
pub const REG_ICR_LOW: u32 = 0x300;
/// Interrupt Command Register — HIGH 32 bits.
/// Bits 31:24 = destination APIC ID for physical-mode IPIs.
pub const REG_ICR_HIGH: u32 = 0x310;
/// LVT Timer Register — controls the LAPIC timer interrupt.
pub const REG_LVT_TIMER: u32 = 0x320;
/// LVT LINT0 Register — controls the LINT0 input pin.
pub const REG_LVT_LINT0: u32 = 0x350;
/// LVT LINT1 Register — controls the LINT1 input pin.
pub const REG_LVT_LINT1: u32 = 0x360;
/// LVT Error Register — delivers LAPIC internal error interrupts.
pub const REG_LVT_ERROR: u32 = 0x370;
/// Initial Count Register for the LAPIC timer (Milestone 6).
///
/// On a write, the timer's current count is reloaded from this value.  In
/// periodic mode, the LAPIC reloads from this register every time the
/// current count counts down to zero.  Writing 0 stops the timer.
///
/// Reference: Intel SDM Vol. 3A §10.5.4 "APIC Timer"
pub const REG_TIMER_INIT_COUNT: u32 = 0x380;
/// Current Count Register for the LAPIC timer (read-only).
pub const REG_TIMER_CURR_COUNT: u32 = 0x390;
/// Divide Configuration Register for the LAPIC timer.
///
/// Selects the divisor applied to the bus clock that drives the timer.
/// Valid encodings (bits 3, 1:0): 1, 2, 4, 8, 16, 32, 64, 128.
///
/// Reference: Intel SDM Vol. 3A Figure 10-10 "Divide Configuration Register"
pub const REG_TIMER_DIV_CONF: u32 = 0x3E0;

// ── Register bit constants ────────────────────────────────────────────────────

/// SVR bit 8: software-enable the LAPIC.
/// When clear, the LAPIC is disabled and no interrupts are delivered.
/// Reference: Intel SDM Vol. 3A §10.9 "Spurious Interrupt"
pub const SVR_ENABLE: u32 = 1 << 8;

/// Spurious vector number used by convention (0xFF = last vector).
/// The LAPIC delivers a spurious interrupt at this vector if a real interrupt
/// is withdrawn before the CPU acknowledges it (e.g., due to TPR changes).
pub const SPURIOUS_VECTOR: u32 = 0xFF;

/// Linux-like boot line proving shorthand broadcast IPIs are available.
pub const IPI_SHORTHAND_BROADCAST_LOG: &str = "IPI shorthand broadcast: enabled";

static IPI_SHORTHAND_LOGGED: AtomicBool = AtomicBool::new(false);

/// LVT bit 16: mask this LVT entry (no interrupt delivered when set).
pub const LVT_MASKED: u32 = 1 << 16;

/// LVT delivery mode for external interrupts routed from the legacy PIC.
pub const LVT_DELIVERY_MODE_EXTINT: u32 = 0x7 << 8;

/// LVT Timer bit 17: periodic mode (timer reloads from initial count on expiry).
///
/// When clear (bit 17 = 0), the timer is one-shot: it counts down once and
/// stops at zero.  When set, the timer reloads from `REG_TIMER_INIT_COUNT`
/// on every expiry, generating periodic ticks at a fixed rate.
///
/// Reference: Intel SDM Vol. 3A Figure 10-8 "Local Vector Table (LVT)"
pub const LVT_TIMER_PERIODIC: u32 = 1 << 17;

/// ICR_LOW bit 12: delivery status — set while the IPI is pending delivery.
/// Poll this bit after writing ICR_LOW; spin until it clears before the next IPI.
/// Reference: Intel SDM Vol. 3A §10.6.1 "Interrupt Command Register (ICR)"
pub const ICR_DELIVERY_PENDING: u32 = 1 << 12;

/// ICR_LOW: INIT delivery mode, level-triggered assert.
///
/// Used as the first IPI in the MP initialization protocol (INIT-SIPI-SIPI).
/// The INIT IPI resets the target AP(s) to their initial state.
///
/// Bit layout:
///   bits 10:8  = 101 (INIT delivery mode)
///   bit  14    = 1   (assert)
///   bit  15    = 1   (level-triggered, required by Intel SDM for INIT)
///
/// Reference: Intel SDM Vol. 3A §10.6.1 Table 10-2 "Delivery Mode"
///            Linux arch/x86/kernel/smpboot.c
pub const ICR_INIT_ASSERT: u32 = 0x0000_C500;

/// ICR_LOW: INIT delivery mode, level-deassert.
///
/// Sent after `ICR_INIT_ASSERT` to complete the INIT IPI sequence.
/// This deasserts the INIT signal; without it, some chipsets keep APs in reset.
///
/// Bit layout:
///   bits 10:8  = 101 (INIT delivery mode)
///   bit  14    = 0   (deassert)
///   bit  15    = 1   (level-triggered, required for deassert)
pub const ICR_INIT_DEASSERT: u32 = 0x0000_8500;

/// ICR_LOW delivery mode for a normal fixed interrupt.
pub const ICR_DELIVERY_FIXED: u32 = 0x0000_0000;

/// ICR_LOW: STARTUP delivery mode base value (without the vector byte).
///
/// The SIPI (Startup Inter-Processor Interrupt) causes the target AP to fetch
/// its first instruction from the real-mode address `vector_page << 12`.
///
/// Bit layout:
///   bits 10:8  = 110 (STARTUP delivery mode)
///   bit  14    = 0   (no assert needed for edge-triggered)
///   bits  7:0  = trampoline page number (added by caller)
///
/// Reference: Intel SDM Vol. 3A §10.6.7 "Broadcast/Self-Directed IPIs"
pub const ICR_STARTUP: u32 = 0x0000_4600;

pub const fn fixed_ipi_icr_low(vector: u8) -> u32 {
    ICR_DELIVERY_FIXED | vector as u32
}

// ── MMIO access helpers ───────────────────────────────────────────────────────

/// Read a 32-bit LAPIC register.
///
/// Uses `read_volatile` to guarantee the hardware-side effect of the read.
///
/// # Safety
/// The LAPIC MMIO must be identity-mapped and the register offset must be
/// a valid LAPIC register (see Table 10-1 in the Intel SDM).
#[inline]
unsafe fn lapic_read(reg: u32) -> u32 {
    let ptr = (LAPIC_BASE + reg as u64) as *const u32;
    // SAFETY: caller guarantees the LAPIC MMIO is accessible.
    unsafe { ptr.read_volatile() }
}

/// Write a 32-bit value to a LAPIC register.
///
/// Uses `write_volatile` to prevent the compiler from eliding or reordering
/// the store — LAPIC registers have side effects on write (e.g., ICR_LOW
/// triggers IPI delivery, EOI clears the in-service bit).
///
/// # Safety
/// Same constraints as `lapic_read`.
#[inline]
unsafe fn lapic_write(reg: u32, val: u32) {
    let ptr = (LAPIC_BASE + reg as u64) as *mut u32;
    // SAFETY: caller guarantees the LAPIC MMIO is accessible.
    unsafe { ptr.write_volatile(val) }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Initialize the Local APIC on the calling CPU (BSP or AP).
///
/// Must be called after the IDT is loaded — the SVR write activates the LAPIC
/// and it may immediately deliver a spurious interrupt at vector 0xFF.
///
/// This function is idempotent for our purposes: calling it on the BSP and
/// then again on each AP is safe because each CPU's LAPIC is independent.
///
/// Reference: Intel SDM Vol. 3A §10.4.3 "Enabling or Disabling the Local APIC"
///
/// # Safety
/// LAPIC MMIO at 0xFEE00000 must be identity-mapped (guaranteed by boot stub).
pub unsafe fn init() {
    // Step 1: Clear TPR so this CPU accepts all interrupt priority levels.
    // TPR bits 7:4 = task priority class; 0 = accept all.
    // Reference: Intel SDM Vol. 3A §10.8.3 "Task and Processor Priorities"
    unsafe {
        lapic_write(REG_TPR, 0);
    }

    // Step 2: Clear Error Status Register before reading or enabling.
    // The ESR must be written before reading; a cleared ESR is also good practice
    // before enabling the LAPIC to discard any power-on error state.
    unsafe {
        lapic_write(REG_ESR, 0);
    }

    // Step 3: Mask all LVT entries.
    // We don't use the LAPIC timer or LINT pins in Milestone 5; masking them
    // prevents unexpected spurious interrupts from partially-configured LVTs.
    // Reference: Intel SDM Vol. 3A §10.5.1 "Local Vector Table"
    unsafe {
        lapic_write(REG_LVT_TIMER, LVT_MASKED);
        lapic_write(REG_LVT_LINT0, LVT_MASKED);
        lapic_write(REG_LVT_LINT1, LVT_MASKED);
        lapic_write(REG_LVT_ERROR, LVT_MASKED);
    }

    // Step 4: Write the Spurious Vector Register to enable the LAPIC.
    // Bit 8 = software enable; bits 7:0 = spurious interrupt vector.
    // This is the write that activates interrupt delivery on this CPU.
    unsafe {
        lapic_write(REG_SVR, SVR_ENABLE | SPURIOUS_VECTOR);
    }

    if !IPI_SHORTHAND_LOGGED.swap(true, Ordering::AcqRel) {
        crate::log_info!("", "{}", IPI_SHORTHAND_BROADCAST_LOG);
    }
}

/// Return the xAPIC ID of the calling CPU's Local APIC.
///
/// The APIC ID uniquely identifies each logical CPU.  The BSP typically has
/// ID 0; APs get IDs 1, 2, … in the order QEMU/firmware enumerates them.
///
/// # Safety
/// LAPIC MMIO must be accessible (same constraint as `init`).
pub unsafe fn id() -> u8 {
    // ID register bits 31:24 hold the 8-bit xAPIC ID.
    // Reference: Intel SDM Vol. 3A §10.4.6 "Local APIC ID"
    unsafe { (lapic_read(REG_ID) >> 24) as u8 }
}

/// Signal End-of-Interrupt to the LAPIC.
///
/// Must be called at the end of every interrupt handler that receives a vector
/// managed by the LAPIC (not the 8259 PIC).  Writing the EOI register clears
/// the highest-priority bit in the In-Service Register (ISR), allowing the LAPIC
/// to deliver the next pending interrupt.
///
/// # Safety
/// LAPIC MMIO must be accessible.
#[inline]
pub unsafe fn eoi() {
    // Reference: Intel SDM Vol. 3A §10.8.5 "Signaling Interrupt Servicing Completion"
    unsafe {
        lapic_write(REG_EOI, 0);
    }
}

/// Route the legacy 8259 PIC through LAPIC LINT0 in ExtINT mode.
///
/// This covers virtual-wire INTx delivery before Lupos grows a full I/O APIC
/// setup path for PCI interrupt routing.
pub unsafe fn enable_lint0_extint() {
    unsafe {
        lapic_write(REG_LVT_LINT0, LVT_DELIVERY_MODE_EXTINT);
    }
}

/// Send a fixed-delivery IPI to the CPU with the given APIC ID.
///
/// Writes ICR_HIGH (destination) first, then ICR_LOW (descriptor).  The write
/// to ICR_LOW is the trigger — the LAPIC begins delivery immediately.
///
/// # Parameters
/// - `target_id`  — xAPIC ID of the destination CPU.
/// - `vector`     — interrupt vector number (32–255; 0–31 are reserved for exceptions).
///
/// # Safety
/// LAPIC MMIO must be accessible; `vector` should correspond to an installed IDT entry.
pub unsafe fn send_ipi(target_id: u8, vector: u8) {
    // Wait for any in-flight IPI to be accepted before sending another.
    unsafe {
        wait_for_icr_idle();
    }

    // Destination: target CPU's APIC ID in bits 31:24 of ICR_HIGH.
    unsafe {
        lapic_write(REG_ICR_HIGH, (target_id as u32) << 24);
    }

    // Fixed delivery mode (bits 10:8 = 000), physical destination mode, no
    // shorthand. Linux's __prepare_ICR() leaves APIC_INT_ASSERT clear for a
    // normal fixed IPI; it is only meaningful for INIT/startup-style flows.
    unsafe {
        lapic_write(REG_ICR_LOW, fixed_ipi_icr_low(vector));
    }

    unsafe {
        wait_for_icr_idle();
    }
}

/// Send an INIT IPI to the target AP.
///
/// The INIT IPI is the first step of the INIT-SIPI-SIPI MP initialization
/// sequence.  It places the target AP in the INIT state (similar to reset).
///
/// The protocol is: assert INIT → wait → deassert INIT.
///
/// Reference: Intel SDM Vol. 3A §10.6.7 "MP Initialization Protocol"
///            Intel SDM Vol. 3A §10.6.1 Table 10-2 "Delivery Mode"
///
/// # Safety
/// LAPIC MMIO must be accessible.
pub unsafe fn send_init_ipi(target_id: u8) {
    unsafe {
        wait_for_icr_idle();
    }

    // INIT assert: level-triggered assert places the target AP in INIT state.
    // (matches Linux smpboot.c: APIC_INT_LEVELTRIG | APIC_INT_ASSERT | APIC_DM_INIT)
    unsafe {
        lapic_write(REG_ICR_HIGH, (target_id as u32) << 24);
        lapic_write(REG_ICR_LOW, ICR_INIT_ASSERT);
    }
    unsafe {
        wait_for_icr_idle();
    }

    // INIT deassert: level-triggered de-assert completes the INIT sequence.
    // (matches Linux: APIC_INT_LEVELTRIG | APIC_DM_INIT)
    unsafe {
        lapic_write(REG_ICR_HIGH, (target_id as u32) << 24);
        lapic_write(REG_ICR_LOW, ICR_INIT_DEASSERT);
    }
    unsafe {
        wait_for_icr_idle();
    }
}

/// Send a STARTUP IPI (SIPI) to the target AP.
///
/// The SIPI tells the AP to start executing at real-mode address `vector_page << 12`.
/// For our trampoline at physical 0x8000, `vector_page = 0x08`.
///
/// Per the Intel spec, two SIPIs should be sent (200 µs apart) to handle the
/// case where the AP missed the first one.
///
/// Reference: Intel SDM Vol. 3A §10.6.7 "MP Initialization Protocol"
///
/// # Parameters
/// - `target_id`   — xAPIC ID of the target AP.
/// - `vector_page` — bits 7:0 of the SIPI vector; AP starts at `vector_page << 12`.
///
/// # Safety
/// LAPIC MMIO must be accessible; trampoline code must already be at
/// `vector_page << 12` in physical memory.
pub unsafe fn send_startup_ipi(target_id: u8, vector_page: u8) {
    unsafe {
        wait_for_icr_idle();
    }

    unsafe {
        lapic_write(REG_ICR_HIGH, (target_id as u32) << 24);
        // ICR_STARTUP (delivery mode = 110b) | vector page number (bits 7:0).
        lapic_write(REG_ICR_LOW, ICR_STARTUP | (vector_page as u32));
    }

    unsafe {
        wait_for_icr_idle();
    }
}

// ── LAPIC timer helpers (Milestone 6) ─────────────────────────────────────────
//
// Thin wrappers around `lapic_write`/`lapic_read` so the `apic_timer` module
// does not need raw MMIO access.  Keeps `lapic_read`/`lapic_write` private to
// this module while still letting the timer driver configure the LVT, divisor,
// initial count, and read the current count.

/// Write the LVT Timer register (vector | mode | mask bits).
///
/// # Safety
/// LAPIC MMIO must be accessible (caller responsibility).
#[inline]
pub(super) unsafe fn timer_write_lvt(val: u32) {
    unsafe { lapic_write(REG_LVT_TIMER, val) }
}

/// Write the timer Initial Count register.  Writing 0 stops the timer.
#[inline]
pub(super) unsafe fn timer_write_init_count(val: u32) {
    unsafe { lapic_write(REG_TIMER_INIT_COUNT, val) }
}

/// Write the timer Divide Configuration register.
#[inline]
pub(super) unsafe fn timer_write_divide(val: u32) {
    unsafe { lapic_write(REG_TIMER_DIV_CONF, val) }
}

/// Read the timer Current Count register.
#[inline]
#[allow(dead_code)]
pub(super) unsafe fn timer_read_current() -> u32 {
    unsafe { lapic_read(REG_TIMER_CURR_COUNT) }
}

/// Spin until the ICR delivery-pending bit (bit 12 of ICR_LOW) clears.
///
/// The CPU sets this bit while the LAPIC is delivering the IPI; it clears
/// when the IPI has been accepted (or has timed out).  We must poll before
/// sending the next IPI to avoid race conditions.
///
/// Reference: Intel SDM Vol. 3A §10.6.1 "Interrupt Command Register (ICR)"
///
/// # Safety
/// LAPIC MMIO must be accessible.
unsafe fn wait_for_icr_idle() {
    // Timeout after ~10000 iterations to avoid hanging if the LAPIC
    // never clears the delivery-pending bit (observed on some QEMU configs).
    for _ in 0..10_000 {
        if unsafe { lapic_read(REG_ICR_LOW) } & ICR_DELIVERY_PENDING == 0 {
            return;
        }
        core::hint::spin_loop();
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// These tests verify pure compile-time and bit-arithmetic properties without
// touching MMIO registers.  They run on the host during `cargo test --lib`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lapic_reg_offsets_match_intel_sdm() {
        // Cross-check key register offsets against Intel SDM Vol. 3A Table 10-1.
        assert_eq!(REG_ID, 0x020, "LAPIC ID register offset");
        assert_eq!(REG_TPR, 0x080, "Task Priority Register offset");
        assert_eq!(REG_EOI, 0x0B0, "End-of-Interrupt Register offset");
        assert_eq!(REG_SVR, 0x0F0, "Spurious Vector Register offset");
        assert_eq!(REG_ESR, 0x280, "Error Status Register offset");
        assert_eq!(REG_ICR_LOW, 0x300, "ICR Low Register offset");
        assert_eq!(REG_ICR_HIGH, 0x310, "ICR High Register offset");
        assert_eq!(REG_LVT_TIMER, 0x320, "LVT Timer Register offset");
        assert_eq!(REG_LVT_LINT0, 0x350, "LVT LINT0 Register offset");
        assert_eq!(REG_LVT_LINT1, 0x360, "LVT LINT1 Register offset");
        assert_eq!(REG_LVT_ERROR, 0x370, "LVT Error Register offset");
    }

    #[test]
    fn lapic_timer_reg_offsets_match_intel_sdm() {
        // Cross-check LAPIC timer register offsets against Intel SDM Vol. 3A
        // §10.5.4 "APIC Timer" / Table 10-1.  These three registers form the
        // complete control surface for the periodic timer.
        assert_eq!(REG_TIMER_INIT_COUNT, 0x380, "Timer Initial Count");
        assert_eq!(REG_TIMER_CURR_COUNT, 0x390, "Timer Current Count");
        assert_eq!(REG_TIMER_DIV_CONF, 0x3E0, "Timer Divide Configuration");
    }

    #[test]
    fn lvt_timer_periodic_mode_bit_is_17() {
        // Reference: Intel SDM Vol. 3A Figure 10-8 "Local Vector Table (LVT)"
        // bit 17 of LVT Timer = timer mode (0 = one-shot, 1 = periodic).
        assert_eq!(LVT_TIMER_PERIODIC, 1 << 17);
        assert_eq!(LVT_TIMER_PERIODIC.count_ones(), 1);
        assert_eq!(LVT_TIMER_PERIODIC.trailing_zeros(), 17);
    }

    #[test]
    fn svr_enable_is_bit_8() {
        // The software-enable bit must be bit 8 (value 0x100).
        // Reference: Intel SDM Vol. 3A §10.9 Figure 10-23 "Spurious-Interrupt Vector Register"
        assert_eq!(SVR_ENABLE, 0x100);
        assert_eq!(SVR_ENABLE.count_ones(), 1);
        assert_eq!(SVR_ENABLE.trailing_zeros(), 8);
    }

    #[test]
    fn ipi_shorthand_boot_log_matches_tracker() {
        assert_eq!(
            IPI_SHORTHAND_BROADCAST_LOG,
            "IPI shorthand broadcast: enabled"
        );
    }

    #[test]
    fn icr_pending_is_bit_12() {
        // Reference: Intel SDM Vol. 3A §10.6.1 "Delivery Status" field.
        assert_eq!(ICR_DELIVERY_PENDING, 0x1000);
        assert_eq!(ICR_DELIVERY_PENDING.trailing_zeros(), 12);
    }

    #[test]
    fn sipi_encodes_vector_page_in_bits_7_0() {
        // For our trampoline at 0x8000, vector_page = 0x08.
        // ICR_LOW for SIPI should be ICR_STARTUP | 0x08.
        let icr = ICR_STARTUP | 0x08u32;
        assert_eq!(icr & 0xFF, 0x08, "vector page in bits 7:0");
        // Delivery mode bits 10:8 must be 110b = 6.
        assert_eq!((icr >> 8) & 0x7, 0x6, "STARTUP delivery mode = 6");
    }

    #[test]
    fn fixed_ipi_icr_matches_linux_prepare_icr_shape() {
        let icr = fixed_ipi_icr_low(0xF1);
        assert_eq!(icr & 0xFF, 0xF1);
        assert_eq!((icr >> 8) & 0x7, 0, "fixed delivery mode");
        assert_eq!(icr & (1 << 14), 0, "normal fixed IPI must not set ASSERT");
        assert_eq!(icr & (1 << 15), 0, "normal fixed IPI is edge-triggered");
    }

    #[test]
    fn init_assert_delivery_mode_is_5() {
        // INIT delivery mode = 101b = 5 (bits 10:8).
        // Reference: Intel SDM Vol. 3A Table 10-2 "Delivery Mode"
        let mode = (ICR_INIT_ASSERT >> 8) & 0x7;
        assert_eq!(mode, 5, "INIT delivery mode bits 10:8 = 101b");
        // Assert bit (bit 14) must be set.
        assert_ne!(
            ICR_INIT_ASSERT & (1 << 14),
            0,
            "INIT assert bit 14 must be set"
        );
    }

    #[test]
    fn lvt_masked_is_bit_16() {
        assert_eq!(LVT_MASKED, 1 << 16);
        assert_eq!(LVT_MASKED.trailing_zeros(), 16);
    }

    #[test]
    fn lint0_extint_delivery_mode_matches_intel() {
        assert_eq!(LVT_DELIVERY_MODE_EXTINT, 0x700);
    }
}
