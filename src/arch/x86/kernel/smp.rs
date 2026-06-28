//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/smp.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/smp.c
//! SMP (Symmetric Multi-Processing) bring-up — Milestone 5.
//!
//! This module wakes up Application Processors (APs) using the INIT-SIPI-SIPI
//! protocol, establishes a rendezvous barrier so the BSP knows all APs are
//! running, and implements the "CPU ping" IPI test.
//!
//! # INIT-SIPI-SIPI Protocol
//! The Intel MP Initialization Protocol (SDM Vol. 3A §10.6.7):
//!   1. BSP sends INIT IPI → AP enters INIT state (similar to reset).
//!   2. BSP waits 10 ms.
//!   3. BSP sends first STARTUP IPI (SIPI) with a vector byte.
//!      The AP starts executing at physical address `(vector_byte << 12)`.
//!   4. BSP waits 200 µs.
//!   5. BSP sends second SIPI (handles the case where the AP missed the first).
//!
//! # Trampoline
//! APs start in 16-bit real mode at physical 0x8000 (vector_byte = 0x08).
//! The trampoline code (`arch/x86/realmode/trampoline.S`, assembled as a flat binary) transitions
//! them to 64-bit long mode and calls `ap_main()`.
//!
//! # Rendezvous barrier
//! `AP_READY_COUNT` is an atomic counter incremented by each AP when it has
//! initialized its LAPIC and enabled interrupts.  The BSP spins on this counter
//! until all expected APs have checked in.
//!
//! References:
//!   Intel SDM Vol. 3A §10.6.7 "Broadcast/Self-Directed IPIs"
//!   Intel SDM Vol. 3A §10.6.1 "Interrupt Command Register (ICR)"
//!   vendor/linux/arch/x86/kernel/smpboot.c
//!   vendor/linux/arch/x86/kernel/smp.c
//!   https://wiki.osdev.org/Symmetric_Multiprocessing
//!   https://wiki.osdev.org/APIC#Sending_an_Inter-Processor_Interrupt

use core::sync::atomic::{AtomicU32, Ordering};

use crate::arch::x86::kernel::acpi::CpuInfo;
#[cfg(feature = "test-smp")]
use crate::arch::x86::kernel::idt::IPI_PING_VECTOR;
use crate::arch::x86::{
    include::asm::io::outb,
    kernel::{apic, msr},
};

// ── Shared atomic counters ────────────────────────────────────────────────────

/// Number of APs that have completed startup (LAPIC init + sti).
///
/// Each AP increments this with `Release` ordering before entering its spin
/// loop.  The BSP reads it with `Acquire` ordering in `wait_for_aps()`.
pub static AP_READY_COUNT: AtomicU32 = AtomicU32::new(0);

/// Number of IPI ping signals received across all APs (Milestone 5 TDD).
///
/// Incremented by the IPI ping handler in `idt.rs` (`on_ipi_ping()`).
/// The BSP reads this with `Acquire` ordering in `run_ipi_ping_test()`.
pub static IPI_RECEIVED_COUNT: AtomicU32 = AtomicU32::new(0);

/// Return the current LAPIC-backed CPU index, clamped to scheduler storage.
///
/// Non-syscall kernel paths use this during boot-test builds where depending
/// on the syscall module's helper creates an avoidable link-time coupling.
#[cfg(test)]
pub fn current_cpu_id() -> usize {
    0
}

#[cfg(not(test))]
pub fn current_cpu_id() -> usize {
    // Skip the LAPIC MMIO read (a VM-exit on VBox) when only the BSP is online.
    if AP_READY_COUNT.load(Ordering::Acquire) == 0 {
        return 0;
    }
    let cpu = unsafe { apic::id() } as usize;
    cpu.min(crate::kernel::sched::MAX_CPUS - 1)
}

// ── Trampoline binary ─────────────────────────────────────────────────────────

/// The AP boot trampoline as a flat binary blob.
///
/// Assembled from `arch/x86/realmode/trampoline.S` with `nasm -f bin` by `build.rs`.
/// The `AP_BOOT_BIN` environment variable is set by `build.rs` to the path of
/// the output binary, which is then embedded here at compile time.
///
/// At runtime the BSP copies this blob to physical address 0x8000 before
/// sending the SIPI with vector byte 0x08 (→ start address = 0x8000).
///
/// For host-side unit tests (where build.rs doesn't run NASM), we substitute
/// an empty slice — unit tests only verify constants and atomics, not the binary.
#[cfg(not(test))]
static AP_BOOT_CODE: &[u8] = include_bytes!(env!("AP_BOOT_BIN"));
#[cfg(test)]
static AP_BOOT_CODE: &[u8] = &[];

// ── Trampoline data-area offsets ──────────────────────────────────────────────
//
// The trampoline page (0x8000–0x8FFF) is divided into:
//   [0x0000–0xDFFF]  executable code (arch/x86/realmode/trampoline.S)
//   [0xE0–...]       data area written by BSP before SIPI
//
// These constants MUST match the offsets used by `times` directives in
// arch/x86/realmode/trampoline.S.  If they diverge, the AP will read garbage and triple-fault.

/// Physical base address of the AP trampoline page.
pub const TRAMPOLINE_BASE: usize = 0x8000;

/// Offset of the BSP's CR3 (PML4 physical address) — u32.
/// Data area starts at 0x110, leaving 272 bytes for code (current code ≈ 246 bytes).
/// SIPI vector page for the trampoline at `TRAMPOLINE_BASE`.
///
/// `0x08 << 12 = 0x8000`, so the BSP can wake APs with vector byte `0x08`.
pub const TRAMPOLINE_VECTOR_PAGE: u8 = (TRAMPOLINE_BASE >> 12) as u8;
pub const TRAMPOLINE_DATA_START: usize = 0x200;
pub const OFF_PML4_ADDR: usize = TRAMPOLINE_DATA_START;

/// Offset of the AP stack top pointer — u64 (8-byte aligned).
/// 0x118 = 0x110 (pml4_addr u32) + 4 (pad) + 4 padding skipped = 8-byte aligned.
pub const OFF_STACK_TOP: usize = OFF_PML4_ADDR + 8;

/// Offset of the BSP's GDTR (limit u16 + base u64 = 10 bytes).
pub const OFF_BSP_GDT: usize = OFF_STACK_TOP + 8;

/// Offset of the BSP's IDTR (limit u16 + base u64 = 10 bytes).
pub const OFF_BSP_IDT: usize = OFF_BSP_GDT + 10;

/// Offset of the `ap_main` function pointer — u64 (8-byte aligned).
/// 0x138 = 0x12A (idt_desc) + 10 (idt_desc size) + 4 (padding) = 8-byte aligned.
pub const OFF_AP_MAIN: usize = OFF_BSP_IDT + 10 + 4;

/// Offset of the BSP CR0 value used when enabling AP paging — u64.
pub const OFF_BSP_CR0: usize = OFF_AP_MAIN + 8;

/// Offset of the BSP CR4 value safe to load before paging is active — u64.
pub const OFF_BSP_CR4_PRE_PAGING: usize = OFF_BSP_CR0 + 8;

/// Offset of the full BSP CR4 value restored after long mode is active — u64.
pub const OFF_BSP_CR4_LONG: usize = OFF_BSP_CR4_PRE_PAGING + 8;

/// Offset of the BSP EFER value used before enabling long mode — u64.
pub const OFF_BSP_EFER: usize = OFF_BSP_CR4_LONG + 8;

const X86_CR4_PCIDE: u64 = 1 << 17;

const fn ap_cr4_pre_paging(bsp_cr4: u64) -> u64 {
    // Intel only allows CR4.PCIDE once paging and long mode are active.
    // The trampoline restores the full CR4 after the 64-bit transition.
    bsp_cr4 & !X86_CR4_PCIDE
}

const fn ap_efer_pre_paging(bsp_efer: u64) -> u64 {
    // EFER.LMA is read-only and becomes set by hardware after CR0.PG is set
    // with EFER.LME=1.  Preserve writable BSP bits such as SCE/NXE.
    (bsp_efer | msr::EFER_LME) & !msr::EFER_LMA
}

// ── Per-AP stacks ─────────────────────────────────────────────────────────────

/// Maximum number of APs supported.
const MAX_APS: usize = 8;

/// Stack size per AP (16 KiB — generous for the simple spin-loop workload).
const AP_STACK_SIZE: usize = 16 * 1024;

/// AP stack alignment required by the SysV AMD64 ABI.
const AP_STACK_ALIGNMENT: usize = 16;

#[allow(dead_code)]
#[repr(align(16))]
#[derive(Copy, Clone)]
struct ApStack([u8; AP_STACK_SIZE]);

/// Dedicated stacks for each AP, indexed by (apic_id - 1) for APIC IDs 1–8.
///
/// Static allocation avoids heap dependency during AP bring-up (the heap may
/// not be initialized on APs before they call `ap_main()`).
// SAFETY: written only during the single-threaded BSP init phase (before APs
// start) or by the AP itself (which has exclusive access to its own slot).
static mut AP_STACKS: [ApStack; MAX_APS] = [ApStack([0u8; AP_STACK_SIZE]); MAX_APS];

// ── GDTR/IDTR helpers ─────────────────────────────────────────────────────────

/// Read the GDTR into a 10-byte buffer and return (limit, base).
///
/// The 10-byte GDTR format (used by `sgdt`/`lgdt` in 64-bit mode):
///   bytes 0–1 : limit (u16)
///   bytes 2–9 : base  (u64)
///
/// # Safety
/// Must be called from 64-bit kernel mode.
unsafe fn read_gdtr() -> (u16, u64) {
    let mut buf = [0u8; 10];
    // SAFETY: sgdt is always valid in kernel mode; buf is a valid stack buffer.
    unsafe {
        core::arch::asm!(
            "sgdt [{0}]",
            in(reg) buf.as_mut_ptr(),
            options(nostack, preserves_flags),
        );
    }
    let limit = u16::from_le_bytes([buf[0], buf[1]]);
    let base = u64::from_le_bytes([
        buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8], buf[9],
    ]);
    (limit, base)
}

/// Read the IDTR into a 10-byte buffer and return (limit, base).
///
/// # Safety
/// Must be called from 64-bit kernel mode.
unsafe fn read_idtr() -> (u16, u64) {
    let mut buf = [0u8; 10];
    // SAFETY: sidt is always valid in kernel mode; buf is a valid stack buffer.
    unsafe {
        core::arch::asm!(
            "sidt [{0}]",
            in(reg) buf.as_mut_ptr(),
            options(nostack, preserves_flags),
        );
    }
    let limit = u16::from_le_bytes([buf[0], buf[1]]);
    let base = u64::from_le_bytes([
        buf[2], buf[3], buf[4], buf[5], buf[6], buf[7], buf[8], buf[9],
    ]);
    (limit, base)
}

#[inline]
unsafe fn read_cr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {0}, cr0",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

#[inline]
unsafe fn read_cr4() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mov {0}, cr4",
            out(reg) value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

// ── Trampoline setup ──────────────────────────────────────────────────────────

/// Copy the AP trampoline to 0x8000 and fill in the BSP-provided data fields.
///
/// This function must be called before sending SIPI.  It:
///   1. Copies `AP_BOOT_CODE` to physical 0x8000.
///   2. Writes PML4 address, stack top, GDT/IDT descriptors, and ap_main
///      pointer into the fixed data area at offsets `OFF_*`.
///
/// # Safety
/// - Physical 0x8000 must be identity-mapped (guaranteed by boot stub).
/// - Called only during single-threaded BSP init (before any SIPI is sent).
unsafe fn setup_trampoline(
    ap_index: usize, // 0-based index into AP_STACKS
    pml4_phys: u32,  // BSP's CR3 value (physical address of PML4)
    gdt_limit: u16,
    gdt_base: u64,
    idt_limit: u16,
    idt_base: u64,
    ap_main_addr: u64,
    bsp_cr0: u64,
    bsp_cr4_pre_paging: u64,
    bsp_cr4_long: u64,
    bsp_efer: u64,
) {
    // Step 1: Copy the trampoline flat binary to physical 0x8000.
    debug_assert!(
        AP_BOOT_CODE.len() <= 0x1000,
        "AP trampoline binary too large (>4 KiB page)"
    );
    // Copy only the code region (up to the writable data window) so the
    // non-volatile memcpy cannot race with the volatile data-area writes below.
    let code_len = core::cmp::min(AP_BOOT_CODE.len(), TRAMPOLINE_DATA_START);
    unsafe {
        core::ptr::copy_nonoverlapping(AP_BOOT_CODE.as_ptr(), TRAMPOLINE_BASE as *mut u8, code_len);
    }
    // Compiler fence: ensure the code copy is fully committed before
    // we write the data-area fields.  Without this, LLVM may reorder
    // the non-volatile memcpy past the volatile byte-writes below.
    core::sync::atomic::compiler_fence(Ordering::SeqCst);

    // Write a value at a fixed offset within the trampoline page.
    // Uses volatile byte-level writes to handle unaligned offsets (e.g.
    // u64 at OFF_BSP_GDT+2) and to guarantee stores are not eliminated.
    macro_rules! trampoline_write {
        ($offset:expr, $ty:ty, $val:expr) => {{
            let val: $ty = $val;
            let bytes = val.to_le_bytes();
            let dst = (TRAMPOLINE_BASE + $offset) as *mut u8;
            // SAFETY: offset is within the 4 KiB trampoline page, which is
            // identity-mapped and writable (BSS / low memory region).
            unsafe {
                for i in 0..bytes.len() {
                    dst.add(i).write_volatile(bytes[i]);
                }
            }
        }};
    }

    // Step 2: Write PML4 address (u32) at offset 0xE0.
    trampoline_write!(OFF_PML4_ADDR, u32, pml4_phys);

    // Step 3: Write AP stack top (u64) at offset 0xE8.
    // x86 stacks grow downward; RSP = top = (base + size).
    let stack_top: u64 = unsafe {
        let base = core::ptr::addr_of_mut!(AP_STACKS[ap_index]) as *mut ApStack as *mut u8 as u64;
        base + AP_STACK_SIZE as u64
    };
    debug_assert_eq!(stack_top as usize % AP_STACK_ALIGNMENT, 0);
    trampoline_write!(OFF_STACK_TOP, u64, stack_top);

    // Step 4: Write GDT descriptor (10 bytes) at offset 0xF0.
    // Format: u16 limit + u64 base.
    trampoline_write!(OFF_BSP_GDT, u16, gdt_limit);
    trampoline_write!(OFF_BSP_GDT + 2, u64, gdt_base);

    // Step 5: Write IDT descriptor (10 bytes) at offset 0xFA.
    trampoline_write!(OFF_BSP_IDT, u16, idt_limit);
    trampoline_write!(OFF_BSP_IDT + 2, u64, idt_base);

    // Step 6: Write ap_main function pointer (u64) at offset 0x104.
    trampoline_write!(OFF_AP_MAIN, u64, ap_main_addr);

    // Step 7: Write BSP control state needed for Linux-like AP bring-up.
    trampoline_write!(OFF_BSP_CR0, u64, bsp_cr0);
    trampoline_write!(OFF_BSP_CR4_PRE_PAGING, u64, bsp_cr4_pre_paging);
    trampoline_write!(OFF_BSP_CR4_LONG, u64, bsp_cr4_long);
    trampoline_write!(OFF_BSP_EFER, u64, bsp_efer);
}

// ── Public SMP API ────────────────────────────────────────────────────────────

/// Wake up the AP with the given APIC ID using the INIT-SIPI-SIPI sequence.
///
/// # Arguments
/// - `apic_id`  — xAPIC ID of the target AP (from MADT).
/// - `ap_index` — 0-based index into `AP_STACKS` for this AP's stack.
///
/// # Safety
/// - LAPIC must be initialized on the BSP (`apic::init()` called).
/// - The trampoline at 0x8000 must not be in use by another AP concurrently.
///   Start APs one-at-a-time and wait for each before starting the next.
unsafe fn start_ap(apic_id: u8, ap_index: usize) {
    // Read BSP's page tables and segment descriptor tables to share with AP.
    // In 64-bit mode, CR3 is a 64-bit register; we read into a 64-bit register
    // and then truncate to u32 for the 32-bit trampoline.
    let mut pml4_phys_64: u64 = 0;
    unsafe {
        core::arch::asm!(
            "mov {:r}, cr3",
            out(reg) pml4_phys_64,
            options(nomem, nostack, preserves_flags),
        );
    }
    let pml4_phys = pml4_phys_64 as u32;
    let (gdt_limit, gdt_base) = unsafe { read_gdtr() };
    let (idt_limit, idt_base) = unsafe { read_idtr() };
    let bsp_cr0 = unsafe { read_cr0() };
    let bsp_cr4 = unsafe { read_cr4() };
    let bsp_efer = unsafe { msr::read(msr::MSR_EFER) };

    // Set up the trampoline page with this AP's parameters.
    unsafe {
        setup_trampoline(
            ap_index,
            pml4_phys,
            gdt_limit,
            gdt_base,
            idt_limit,
            idt_base,
            ap_main as *const () as u64,
            bsp_cr0,
            ap_cr4_pre_paging(bsp_cr4),
            bsp_cr4,
            ap_efer_pre_paging(bsp_efer),
        );
    }

    // ── INIT-SIPI-SIPI sequence ────────────────────────────────────────────
    //
    // Reference: Intel SDM Vol. 3A §10.6.7 "MP Initialization Protocol"
    //
    // 1. Send INIT IPI (places AP in INIT state).
    // Verify trampoline was written correctly
    let byte0 = unsafe { (TRAMPOLINE_BASE as *const u8).read_volatile() };
    let ap_fn = unsafe { ((TRAMPOLINE_BASE + OFF_AP_MAIN) as *const u64).read_unaligned() };
    crate::linux_driver_abi::tty::serial_println!(
        "[smp] trampoline[0]={:#x} ap_main_ptr={:#x} expected={:#x}",
        byte0,
        ap_fn,
        ap_main as *const () as u64
    );
    crate::linux_driver_abi::tty::serial_println!("[smp] INIT AP {}", apic_id);
    unsafe {
        apic::send_init_ipi(apic_id);
    }

    // 2. Wait 10 ms (required by the spec after INIT).
    delay_ms(10);

    // 3. Send first SIPI.  Vector byte 0x08 → start address = 0x8000.
    crate::linux_driver_abi::tty::serial_println!("[smp] SIPI1 AP {}", apic_id);
    unsafe {
        apic::send_startup_ipi(apic_id, TRAMPOLINE_VECTOR_PAGE);
    }
    crate::linux_driver_abi::tty::serial_println!("[smp] SIPI1 done");

    // 4. Wait 200 µs, then send second SIPI only if the AP hasn't responded.
    //    QEMU processes SIPIs synchronously — sending a second while the AP
    //    is already running can leave the ICR in delivery-pending state.
    delay_us(200);

    if AP_READY_COUNT.load(Ordering::Acquire) < (ap_index + 1) as u32 {
        crate::linux_driver_abi::tty::serial_println!("[smp] SIPI2 AP {}", apic_id);
        unsafe {
            apic::send_startup_ipi(apic_id, TRAMPOLINE_VECTOR_PAGE);
        }
        crate::linux_driver_abi::tty::serial_println!("[smp] SIPI2 done");
    }
}

/// Wake up all non-BSP CPUs listed in `cpus`.
///
/// APs are started one at a time (sequential).  After each SIPI we wait for
/// the AP to increment `AP_READY_COUNT` before moving to the next, ensuring
/// the shared trampoline page at 0x8000 is not overwritten while in use.
///
/// # Safety
/// LAPIC must be initialized on the BSP.
pub unsafe fn start_aps(cpus: &[CpuInfo]) {
    let bsp_id = unsafe { apic::id() };
    let mut ap_index = 0usize;

    for cpu in cpus {
        if !cpu.enabled || cpu.apic_id == bsp_id {
            continue;
        }
        if ap_index >= MAX_APS {
            break;
        }

        let expected_after = (ap_index + 1) as u32;
        unsafe {
            start_ap(cpu.apic_id, ap_index);
        }

        // Wait up to 2 seconds for this AP to check in before sending the
        // next SIPI (which would overwrite the trampoline page).
        let tsc_2s = 2_000_000_000u64; // ~2s on a ≥1 GHz clock
        if !wait_for_aps(expected_after as usize, tsc_2s) {
            // AP didn't respond — log but continue with remaining CPUs.
            crate::kernel::printk::log_warn!(
                "smp",
                "smp: AP {} (index {}) did not respond within 2s",
                cpu.apic_id,
                ap_index
            );
        }

        ap_index += 1;
    }
}

/// Spin until `AP_READY_COUNT ≥ expected` or the TSC deadline passes.
///
/// Returns `true` if the condition was met, `false` on timeout.
///
/// # Timeout
/// `timeout_cycles` is in raw TSC ticks.  On a 1 GHz TSC, 500_000_000 ≈ 0.5s.
/// QEMU's TSC runs at a simulated frequency (usually 1–3 GHz); this is well
/// within the time needed for a QEMU AP to initialize.
pub fn wait_for_aps(expected: usize, timeout_cycles: u64) -> bool {
    let deadline = rdtsc().saturating_add(timeout_cycles);
    loop {
        if AP_READY_COUNT.load(Ordering::Acquire) as usize >= expected {
            return true;
        }
        if rdtsc() >= deadline {
            return false;
        }
        core::hint::spin_loop();
    }
}

// ── AP entry point ────────────────────────────────────────────────────────────

/// Entry point called by the AP trampoline after transitioning to 64-bit mode.
///
/// This function:
///   1. Initializes the AP's own Local APIC.
///   2. Increments `AP_READY_COUNT` to signal the BSP.
///   3. Enables interrupts so IPIs can be received.
///   4. Spins in a low-power loop (waiting for IPI ping and future work).
///
/// # ABI
/// Called with `extern "C"` from naked assembly in `arch/x86/realmode/trampoline.S`.
/// RDI = xAPIC ID of this AP (passed by the trampoline).
///
/// # Safety
/// Called from the trampoline with no prior Rust runtime setup.
/// The stack and segment registers are already configured by the trampoline.
#[unsafe(no_mangle)]
pub extern "C" fn ap_main(apic_id: u64) -> ! {
    let cpu_slot = (apic_id as usize).min(crate::kernel::sched::MAX_CPUS - 1);
    unsafe {
        crate::arch::x86::kernel::gdt::init_ap(cpu_slot);
        crate::arch::x86::kernel::fpu::init();
    }

    // Initialize this AP's Local APIC.
    // Each CPU has its own LAPIC at the same MMIO address (0xFEE00000).
    unsafe {
        apic::init();
    }

    // Milestone 6: mask the AP's LAPIC timer.  Only the BSP drives ticks
    // for now (see kernel::softirq Risk #3 — no per-CPU storage yet, so we
    // need TIMER_TICKS to remain a single-writer counter).
    let cpu = unsafe { apic::id() } as u32;
    unsafe {
        crate::kernel::sched::sched_init_ap(cpu);
        crate::arch::x86::kernel::apic_timer::init_ap();
    }
    crate::kernel::rcu::rcu_qs();

    // Signal the BSP that this AP is ready.
    AP_READY_COUNT.fetch_add(1, Ordering::Release);

    // Enable interrupts so this AP can receive IPI pings.
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }

    // Spin in a low-power loop.
    loop {
        crate::kernel::watchdog::touch_softlockup_watchdog_sched();
        crate::kernel::softirq::do_softirq();
        crate::kernel::rcu::rcu_qs();
        unsafe {
            core::arch::asm!("sti; hlt", options(nomem, nostack));
        }
    }
}

// ── TDD: CPU ping test (Milestone 5) ─────────────────────────────────────────

/// Run the "CPU ping" IPI test.
///
/// The BSP sends a fixed IPI at vector `IPI_PING_VECTOR` to the first enabled
/// non-BSP AP.  The AP's IDT handler (installed in `idt::init()`) increments
/// `IPI_RECEIVED_COUNT` and sends LAPIC EOI.  The BSP polls the counter with
/// a 2-second timeout.
///
/// On success: logs a banner and exits QEMU with code 0x21.
/// On failure: panics (which exits QEMU with code 0x01 via the panic handler).
///
/// This function is only compiled when the `test-smp` Cargo feature is active.
///
/// Reference: Intel SDM Vol. 3A §10.6 "Issuing Interprocessor Interrupts"
#[cfg(feature = "test-smp")]
pub fn run_ipi_ping_test(cpus: &[CpuInfo]) {
    let bsp_id = unsafe { apic::id() };

    // Find the first enabled non-BSP CPU.
    let target = cpus.iter().find(|c| c.enabled && c.apic_id != bsp_id);

    let Some(ap) = target else {
        panic!("smp: IPI ping test FAILED - no enabled non-BSP CPU found");
    };

    // Record the counter value before sending the IPI.
    let before = IPI_RECEIVED_COUNT.load(Ordering::Acquire);

    // Send fixed IPI to the AP at our ping vector.
    // Reference: Intel SDM Vol. 3A §10.6.1 "Interrupt Command Register (ICR)"
    unsafe {
        apic::send_ipi(ap.apic_id, IPI_PING_VECTOR);
    }

    // Poll for the counter to increment (2-second TSC timeout).
    let deadline = rdtsc().saturating_add(2_000_000_000);
    loop {
        let after = IPI_RECEIVED_COUNT.load(Ordering::Acquire);
        if after > before {
            // Success — log the TDD banner then exit QEMU cleanly.
            // The banner must match SMP_BANNER in xtask/src/lib.rs.
            crate::kernel::printk::log_info!(
                "smp",
                "smp: IPI ping test PASSED (AP {} replied)",
                ap.apic_id
            );

            #[cfg(feature = "qemu-test")]
            unsafe {
                crate::linux_driver_abi::platform::qemu::exit_success();
            }
            return;
        }
        if rdtsc() >= deadline {
            panic!(
                "smp: IPI ping test FAILED - AP {} did not reply within timeout",
                ap.apic_id
            );
        }
        core::hint::spin_loop();
    }
}

// ── Delay utilities ───────────────────────────────────────────────────────────
//
// Precise delays require a calibrated timer (HPET, APIC timer, or TSC).
// For now we use repeated I/O waits (outb to the POST diagnostic port 0x80).
// Each outb takes roughly 1–2 µs on real hardware.  On QEMU the timing is
// not strictly required for correctness — QEMU processes IPIs synchronously —
// but we include these delays for spec compliance.
//
// Reference: Intel SDM Vol. 3A §10.6.7 — "wait 10 ms" and "200 µs" delays

/// Rough delay of `ms` milliseconds using I/O port writes.
fn delay_ms(ms: u64) {
    // ~5000 io_waits ≈ 5–10 ms (conservative); actual timing depends on hardware.
    for _ in 0..(ms * 5000) {
        unsafe {
            outb(0x80, 0);
        }
    }
}

/// Rough delay of `us` microseconds using I/O port writes.
fn delay_us(us: u64) {
    // ~5 io_waits ≈ 5–10 µs (conservative).
    for _ in 0..(us * 5) {
        unsafe {
            outb(0x80, 0);
        }
    }
}

// ── Time Stamp Counter ────────────────────────────────────────────────────────

/// Read the Time Stamp Counter (TSC) as a 64-bit monotonic tick count.
///
/// Used for timeout calculations in `wait_for_aps()` and `run_ipi_ping_test()`.
/// The TSC increments at a constant rate (invariant TSC, required on x86_64).
///
/// Reference: Intel SDM Vol. 3B §17.17 "Time-Stamp Counter"
#[inline]
fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: RDTSC is always available on x86_64 (required by AMD64 spec).
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack, preserves_flags),
        );
    }
    (hi as u64) << 32 | lo as u64
}

// ── Unit tests ────────────────────────────────────────────────────────────────
//
// These tests verify compile-time layout properties and atomic initial values.
// No hardware access; runs on the host with `cargo test --lib`.

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn ap_ready_count_starts_at_zero() {
        // The global atomic must start at 0 so wait_for_aps(0, _) returns true
        // immediately (no APs expected) and wait_for_aps(1, _) waits correctly.
        assert_eq!(AP_READY_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn ipi_received_count_starts_at_zero() {
        assert_eq!(IPI_RECEIVED_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn trampoline_offsets_within_4kib_page() {
        // All data offsets must be within the 4 KiB trampoline page.
        assert!(OFF_PML4_ADDR < 0x1000, "PML4 offset must be < 4 KiB");
        assert!(OFF_STACK_TOP < 0x1000, "stack top offset must be < 4 KiB");
        assert!(
            OFF_BSP_GDT < 0x1000,
            "GDT descriptor offset must be < 4 KiB"
        );
        assert!(
            OFF_BSP_IDT < 0x1000,
            "IDT descriptor offset must be < 4 KiB"
        );
        // ap_main pointer + 8 bytes must also be within the page.
        assert!(
            OFF_AP_MAIN + 8 <= 0x1000,
            "ap_main pointer must fit in 4 KiB page"
        );
        assert!(
            OFF_BSP_EFER + 8 <= 0x1000,
            "control-state fields must fit in 4 KiB page"
        );
    }

    #[test]
    fn trampoline_layout_matches_assembly_contract() {
        assert_eq!(
            TRAMPOLINE_DATA_START, 0x200,
            "trampoline data window must start at 0x200"
        );
        assert_eq!(
            OFF_PML4_ADDR, TRAMPOLINE_DATA_START,
            "PML4 field must start data window"
        );
        assert_eq!(
            OFF_STACK_TOP,
            OFF_PML4_ADDR + 8,
            "stack top must follow padded PML4 field"
        );
        assert_eq!(
            OFF_BSP_GDT,
            OFF_STACK_TOP + 8,
            "GDT descriptor must follow stack top"
        );
        assert_eq!(
            OFF_BSP_IDT,
            OFF_BSP_GDT + 10,
            "IDT descriptor must follow GDT descriptor"
        );
        assert_eq!(
            OFF_AP_MAIN,
            OFF_BSP_IDT + 10 + 4,
            "ap_main pointer must follow IDT descriptor + pad"
        );
        assert_eq!(
            OFF_BSP_CR0,
            OFF_AP_MAIN + 8,
            "CR0 must follow ap_main pointer"
        );
        assert_eq!(
            OFF_BSP_CR4_PRE_PAGING,
            OFF_BSP_CR0 + 8,
            "pre-paging CR4 must follow CR0"
        );
        assert_eq!(
            OFF_BSP_CR4_LONG,
            OFF_BSP_CR4_PRE_PAGING + 8,
            "long-mode CR4 must follow pre-paging CR4"
        );
        assert_eq!(
            OFF_BSP_EFER,
            OFF_BSP_CR4_LONG + 8,
            "EFER must follow long-mode CR4"
        );
        assert_eq!(OFF_BSP_CR0, 0x230);
        assert_eq!(OFF_BSP_CR4_PRE_PAGING, 0x238);
        assert_eq!(OFF_BSP_CR4_LONG, 0x240);
        assert_eq!(OFF_BSP_EFER, 0x248);
    }

    #[test]
    fn trampoline_sipi_vector_page_matches_base() {
        assert_eq!(
            TRAMPOLINE_VECTOR_PAGE, 0x08,
            "SIPI vector page must be 0x08"
        );
        assert_eq!(TRAMPOLINE_BASE, (TRAMPOLINE_VECTOR_PAGE as usize) << 12);
    }

    #[test]
    fn ap_stack_is_16_byte_aligned() {
        assert_eq!(
            align_of::<ApStack>(),
            AP_STACK_ALIGNMENT,
            "AP stack alignment must be 16"
        );
        assert_eq!(
            size_of::<ApStack>(),
            AP_STACK_SIZE,
            "AP stack wrapper must not change size"
        );
        assert_eq!(
            AP_STACK_SIZE % AP_STACK_ALIGNMENT,
            0,
            "AP stack size must preserve alignment"
        );
    }

    #[test]
    fn trampoline_pml4_addr_is_u32_aligned() {
        assert_eq!(
            OFF_PML4_ADDR % 4,
            0,
            "PML4 addr offset must be 4-byte aligned"
        );
    }

    #[test]
    fn trampoline_stack_top_is_u64_aligned() {
        assert_eq!(
            OFF_STACK_TOP % 8,
            0,
            "stack top offset must be 8-byte aligned"
        );
    }

    #[test]
    fn trampoline_ap_main_is_u64_aligned() {
        assert_eq!(
            OFF_AP_MAIN % 8,
            0,
            "ap_main pointer offset must be 8-byte aligned"
        );
    }

    #[test]
    fn trampoline_control_state_fields_are_u64_aligned() {
        assert_eq!(OFF_BSP_CR0 % 8, 0, "CR0 offset must be u64 aligned");
        assert_eq!(
            OFF_BSP_CR4_PRE_PAGING % 8,
            0,
            "pre-paging CR4 offset must be u64 aligned"
        );
        assert_eq!(
            OFF_BSP_CR4_LONG % 8,
            0,
            "long-mode CR4 offset must be u64 aligned"
        );
        assert_eq!(OFF_BSP_EFER % 8, 0, "EFER offset must be u64 aligned");
    }

    #[test]
    fn ap_control_state_masks_transition_only_bits() {
        let cr4 = X86_CR4_PCIDE | (1 << 5) | (1 << 9);
        assert_eq!(ap_cr4_pre_paging(cr4), (1 << 5) | (1 << 9));

        let efer = msr::EFER_SCE | msr::EFER_LMA | msr::EFER_NX;
        assert_eq!(
            ap_efer_pre_paging(efer),
            msr::EFER_SCE | msr::EFER_LME | msr::EFER_NX
        );
    }

    #[test]
    fn ap_stack_size_is_reasonable() {
        // At least 8 KiB (two guard pages' worth) but not wasteful.
        assert!(AP_STACK_SIZE >= 8 * 1024, "AP stack must be at least 8 KiB");
        assert!(
            AP_STACK_SIZE <= 64 * 1024,
            "AP stack should not exceed 64 KiB"
        );
    }
}
