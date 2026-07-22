//! linux-parity: partial
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
//! # Rendezvous barriers
//! Each AP publishes an early-alive bit as soon as it reaches `ap_main()`.
//! The BSP uses that bit to decide whether the second SIPI is needed and to
//! know when the shared trampoline may safely be reused. `AP_READY_COUNT` is
//! published separately, after the AP has completed its per-CPU scheduler and
//! timer initialization.
//!
//! References:
//!   Intel SDM Vol. 3A §10.6.7 "Broadcast/Self-Directed IPIs"
//!   Intel SDM Vol. 3A §10.6.1 "Interrupt Command Register (ICR)"
//!   vendor/linux/arch/x86/kernel/smpboot.c
//!   vendor/linux/arch/x86/kernel/smp.c
//!   https://wiki.osdev.org/Symmetric_Multiprocessing
//!   https://wiki.osdev.org/APIC#Sending_an_Inter-Processor_Interrupt

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::arch::x86::kernel::acpi::CpuInfo;
#[cfg(feature = "test-smp")]
use crate::arch::x86::kernel::idt::IPI_PING_VECTOR;
use crate::arch::x86::{
    include::asm::io::outb,
    kernel::{apic, msr},
};

// ── CPU maps and startup state ────────────────────────────────────────────────

/// Maximum CPUs tracked by the scheduler: one BSP plus [`MAX_APS`] APs.
const MAX_CPUS: usize = crate::kernel::sched::MAX_CPUS;

/// Sentinel for an unpublished logical-CPU to physical-APIC mapping.
const INVALID_APIC_ID: u32 = u32::MAX;

/// Linux's `x86_cpu_to_apicid`: dense logical CPU number to physical APIC ID.
///
/// The MADT may contain sparse APIC IDs, so a physical APIC ID must never be
/// used directly as an index into scheduler or per-CPU storage.
static LOGICAL_TO_APIC_ID: [AtomicU32; MAX_CPUS] =
    [const { AtomicU32::new(INVALID_APIC_ID) }; MAX_CPUS];

/// Number of entries published in [`LOGICAL_TO_APIC_ID`].
static LOGICAL_CPU_COUNT: AtomicU32 = AtomicU32::new(0);

/// APs which have entered [`ap_main`], indexed by dense logical CPU number.
///
/// This is deliberately separate from full readiness. Linux likewise has an
/// early `SYNC_STATE_ALIVE` rendezvous before `start_secondary()` completes
/// per-CPU setup.
static AP_ALIVE_MASK: AtomicU64 = AtomicU64::new(0);

/// APs which have completed all initialization, indexed by logical CPU number.
static AP_READY_MASK: AtomicU64 = AtomicU64::new(0);

/// Number of APs that have completed startup.
///
/// Each AP increments this with `Release` ordering after scheduler and timer
/// initialization, immediately before enabling local interrupts and entering
/// the scheduler idle loop. The BSP reads it with `Acquire` ordering in
/// `wait_for_aps()`.
pub static AP_READY_COUNT: AtomicU32 = AtomicU32::new(0);

/// Number of IPI ping signals received across all APs (Milestone 5 TDD).
///
/// Incremented by the IPI ping handler in `idt.rs` (`on_ipi_ping()`).
/// The BSP reads this with `Acquire` ordering in `run_ipi_ping_test()`.
pub static IPI_RECEIVED_COUNT: AtomicU32 = AtomicU32::new(0);

/// Return the physical APIC ID assigned to a dense logical CPU number.
///
/// This is the x86 destination translation required by physical-mode IPIs.
/// Unmapped and out-of-range logical CPU numbers return `None`.
pub fn logical_cpu_to_apic_id(cpu: u32) -> Option<u8> {
    let count = LOGICAL_CPU_COUNT.load(Ordering::Acquire);
    if cpu >= count {
        return None;
    }
    let apic_id = LOGICAL_TO_APIC_ID[cpu as usize].load(Ordering::Acquire);
    u8::try_from(apic_id).ok()
}

/// Translate a physical APIC ID to its dense logical CPU number.
fn apic_id_to_logical_cpu(apic_id: u8) -> Option<u32> {
    let count = LOGICAL_CPU_COUNT.load(Ordering::Acquire) as usize;
    (0..count)
        .find(|&cpu| LOGICAL_TO_APIC_ID[cpu].load(Ordering::Acquire) == apic_id as u32)
        .map(|cpu| cpu as u32)
}

#[inline]
fn logical_cpu_bit(cpu: u32) -> Option<u64> {
    if cpu < MAX_CPUS as u32 && cpu < u64::BITS {
        Some(1u64 << cpu)
    } else {
        None
    }
}

/// Return the current dense logical CPU number.
///
/// Non-syscall kernel paths use this during boot-test builds where depending
/// on the syscall module's helper creates an avoidable link-time coupling.
#[cfg(test)]
pub fn current_cpu_id() -> usize {
    0
}

#[cfg(not(test))]
pub fn current_cpu_id() -> usize {
    // Skip the LAPIC MMIO read (a VM-exit on VBox) until an AP has reached
    // Rust. Before that point only the BSP can execute this path.
    if AP_ALIVE_MASK.load(Ordering::Acquire) == 0 {
        return 0;
    }
    let apic_id = unsafe { apic::id() };
    apic_id_to_logical_cpu(apic_id)
        .unwrap_or_else(|| panic!("smp: current APIC ID {apic_id} has no logical CPU mapping"))
        as usize
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
const MAX_APS: usize = MAX_CPUS - 1;

/// Stack size per AP, matching production scheduler kernel-thread stacks.
const AP_STACK_SIZE: usize = crate::kernel::sched::KTHREAD_STACK_SIZE;

/// AP stacks use the same alignment window as scheduler task stacks.
const AP_STACK_ALIGNMENT: usize = crate::kernel::sched::KTHREAD_STACK_SIZE;

#[allow(dead_code)]
#[cfg_attr(debug_assertions, repr(align(65536)))]
#[cfg_attr(not(debug_assertions), repr(align(16384)))]
struct ApStack([u8; AP_STACK_SIZE]);

/// Dedicated stacks for each AP, indexed by `(logical_cpu - 1)`.
///
/// Static allocation avoids heap dependency during AP bring-up (the heap may
/// not be initialized on APs before they call `ap_main()`).
// SAFETY: written only during the single-threaded BSP init phase (before APs
// start) or by the AP itself (which has exclusive access to its own slot).
static mut AP_STACKS: [ApStack; MAX_APS] = [const { ApStack([0u8; AP_STACK_SIZE]) }; MAX_APS];

/// Return the exact top of the scheduler-compatible stack for an AP.
fn ap_stack_top(logical_cpu: u32) -> Option<usize> {
    let index = (logical_cpu as usize).checked_sub(1)?;
    if index >= MAX_APS {
        return None;
    }
    let stacks = core::ptr::addr_of_mut!(AP_STACKS).cast::<ApStack>();
    let base = unsafe { stacks.add(index) } as usize;
    Some(base + AP_STACK_SIZE)
}

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
    stack_top: usize, // exact top of this AP's scheduler-compatible stack
    pml4_phys: u32,   // BSP's CR3 value (physical address of PML4)
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
    // x86 stacks grow downward; RSP starts at the exact scheduler stack top.
    debug_assert_eq!(stack_top % AP_STACK_ALIGNMENT, 0);
    trampoline_write!(OFF_STACK_TOP, u64, stack_top as u64);

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

/// Build Linux-style dense logical CPU mappings from the MADT.
///
/// CPU 0 is always the running BSP. Enabled non-BSP processors receive logical
/// IDs 1..N in MADT discovery order, regardless of how sparse their physical
/// APIC IDs are.
fn publish_cpu_mappings(cpus: &[CpuInfo], bsp_apic_id: u8) -> u32 {
    assert_eq!(
        LOGICAL_CPU_COUNT.load(Ordering::Acquire),
        0,
        "smp: logical CPU mappings may only be published once"
    );
    assert_ne!(bsp_apic_id, u8::MAX, "smp: BSP reported Linux BAD_APICID");

    LOGICAL_TO_APIC_ID[0].store(bsp_apic_id as u32, Ordering::Relaxed);
    let mut logical_cpu = 1usize;

    for cpu in cpus {
        if !cpu.enabled || cpu.apic_id == bsp_apic_id {
            continue;
        }
        if cpu.apic_id == u8::MAX {
            crate::kernel::printk::log_warn!(
                "smp",
                "smp: ignoring MADT processor with invalid APIC ID {:#x}",
                cpu.apic_id
            );
            continue;
        }
        if (0..logical_cpu).any(|existing| {
            LOGICAL_TO_APIC_ID[existing].load(Ordering::Relaxed) == cpu.apic_id as u32
        }) {
            crate::kernel::printk::log_warn!(
                "smp",
                "smp: ignoring duplicate MADT APIC ID {}",
                cpu.apic_id
            );
            continue;
        }
        if logical_cpu >= MAX_CPUS {
            crate::kernel::printk::log_warn!(
                "smp",
                "smp: ignoring APIC ID {}: scheduler supports {} CPUs",
                cpu.apic_id,
                MAX_CPUS
            );
            continue;
        }

        LOGICAL_TO_APIC_ID[logical_cpu].store(cpu.apic_id as u32, Ordering::Relaxed);
        logical_cpu += 1;
    }

    // Publish all table entries together. APs acquire this count before
    // scanning the table at the first instruction of `ap_main()`.
    LOGICAL_CPU_COUNT.store(logical_cpu as u32, Ordering::Release);
    logical_cpu as u32
}

/// Wake up the AP with the given APIC ID using the INIT-SIPI-SIPI sequence.
///
/// # Arguments
/// - `apic_id`     — physical xAPIC ID of the target AP (from MADT).
/// - `logical_cpu` — dense logical CPU number assigned by the BSP.
///
/// # Safety
/// - LAPIC must be initialized on the BSP (`apic::init()` called).
/// - The trampoline at 0x8000 must not be in use by another AP concurrently.
///   Start APs one-at-a-time and wait for each before starting the next.
unsafe fn start_ap(apic_id: u8, logical_cpu: u32) {
    let stack_top = ap_stack_top(logical_cpu)
        .unwrap_or_else(|| panic!("smp: logical CPU {logical_cpu} has no AP stack"));
    let alive_bit = logical_cpu_bit(logical_cpu)
        .unwrap_or_else(|| panic!("smp: logical CPU {logical_cpu} exceeds startup mask"));

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
            stack_top,
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

    if AP_ALIVE_MASK.load(Ordering::Acquire) & alive_bit == 0 {
        crate::linux_driver_abi::tty::serial_println!("[smp] SIPI2 AP {}", apic_id);
        unsafe {
            apic::send_startup_ipi(apic_id, TRAMPOLINE_VECTOR_PAGE);
        }
        crate::linux_driver_abi::tty::serial_println!("[smp] SIPI2 done");
    }
}

/// Prepare dense logical CPU mappings for all usable MADT processors.
///
/// This is the mapping portion of Linux's `native_smp_prepare_cpus()`. It must
/// run once on the BSP before [`start_aps`], while no secondary CPU can observe
/// a partially published table.
pub fn prepare_cpus(cpus: &[CpuInfo]) -> usize {
    let bsp_id = unsafe { apic::id() };
    publish_cpu_mappings(cpus, bsp_id).saturating_sub(1) as usize
}

/// Wake up all non-BSP CPUs prepared by [`prepare_cpus`].
///
/// APs are started one at a time. After each SIPI, the BSP first waits for that
/// AP's early-alive bit, which proves the shared trampoline page is no longer
/// being read, and then waits separately for full scheduler readiness.
///
/// # Safety
/// LAPIC must be initialized on the BSP.
pub unsafe fn start_aps() {
    let logical_cpu_count = LOGICAL_CPU_COUNT.load(Ordering::Acquire);
    assert_ne!(
        logical_cpu_count, 0,
        "smp: prepare_cpus() must publish CPU mappings before start_aps()"
    );

    for logical_cpu in 1..logical_cpu_count {
        let apic_id = logical_cpu_to_apic_id(logical_cpu)
            .unwrap_or_else(|| panic!("smp: logical CPU {logical_cpu} mapping disappeared"));
        unsafe {
            start_ap(apic_id, logical_cpu);
        }

        // `ap_main()` has consumed all shared trampoline fields before it sets
        // the alive bit. If that never happens, reusing the page for another AP
        // would race a late starter, so abandon the remaining bring-up.
        let tsc_2s = 2_000_000_000u64; // ~2s on a ≥1 GHz clock
        if !wait_for_ap_state(&AP_ALIVE_MASK, logical_cpu, tsc_2s) {
            crate::kernel::printk::log_warn!(
                "smp",
                "smp: APIC ID {} (logical CPU {}) did not reach alive state within 2s",
                apic_id,
                logical_cpu
            );
            break;
        }

        // Full readiness is a distinct, later milestone. Once alive is
        // observed the trampoline is safe to reuse even if initialization
        // subsequently times out.
        if !wait_for_ap_state(&AP_READY_MASK, logical_cpu, tsc_2s) {
            crate::kernel::printk::log_warn!(
                "smp",
                "smp: APIC ID {} (logical CPU {}) did not become ready within 2s",
                apic_id,
                logical_cpu
            );
        }
    }
}

/// Wait for one logical CPU's bit in an AP startup-state mask.
fn wait_for_ap_state(state: &AtomicU64, logical_cpu: u32, timeout_cycles: u64) -> bool {
    let Some(bit) = logical_cpu_bit(logical_cpu) else {
        return false;
    };
    let deadline = rdtsc().saturating_add(timeout_cycles);
    loop {
        if state.load(Ordering::Acquire) & bit != 0 {
            return true;
        }
        if rdtsc() >= deadline {
            return false;
        }
        core::hint::spin_loop();
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
///   1. Publishes the early-alive handshake.
///   2. Initializes GDT/TSS, FPU, LAPIC, per-CPU GS, and syscall MSRs.
///   3. Installs the scheduler idle task and LAPIC timer.
///   4. Publishes full readiness and enters the scheduler idle loop.
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
    let physical_apic_id = u8::try_from(apic_id)
        .unwrap_or_else(|_| panic!("smp: trampoline supplied invalid APIC ID {apic_id:#x}"));
    let cpu = apic_id_to_logical_cpu(physical_apic_id).unwrap_or_else(|| {
        panic!("smp: APIC ID {physical_apic_id} has no dense logical CPU mapping")
    });
    assert_ne!(cpu, 0, "smp: BSP entered the AP startup path");
    let startup_bit =
        logical_cpu_bit(cpu).unwrap_or_else(|| panic!("smp: logical CPU {cpu} is out of range"));
    let stack_top =
        ap_stack_top(cpu).unwrap_or_else(|| panic!("smp: logical CPU {cpu} has no AP stack"));

    // Match Linux's early SYNC_STATE_ALIVE rendezvous. This must precede all
    // potentially lengthy per-CPU initialization so the BSP never uses full
    // readiness as its second-SIPI decision.
    let was_alive = AP_ALIVE_MASK.fetch_or(startup_bit, Ordering::Release);
    assert_eq!(
        was_alive & startup_bit,
        0,
        "smp: logical CPU {cpu} entered ap_main twice"
    );

    // Linux's start_secondary() establishes descriptor/FPU/APIC state before
    // installing the CPU-local GS base. In particular, init_ap() must load the
    // per-CPU GDT before setup_percpu_segment() writes MSR_GS_BASE.
    unsafe {
        crate::arch::x86::kernel::gdt::init_ap(cpu as usize);
        crate::arch::x86::kernel::fpu::init();
        apic::init();
    }
    crate::arch::x86::kernel::setup_percpu::setup_percpu_segment(cpu as usize);
    unsafe {
        crate::arch::x86::entry::syscall::init_ap();
    }

    unsafe {
        crate::kernel::sched::sched_init_ap(cpu, stack_top);
        crate::arch::x86::kernel::apic_timer::init_ap();
    }
    crate::kernel::sched::sched_activate_cpu(cpu);
    crate::kernel::rcu::tasks_rcu_qs();
    crate::kernel::rcu::rcu_qs();

    let was_ready = AP_READY_MASK.fetch_or(startup_bit, Ordering::Release);
    assert_eq!(
        was_ready & startup_bit,
        0,
        "smp: logical CPU {cpu} published readiness twice"
    );
    AP_READY_COUNT.fetch_add(1, Ordering::Release);

    // Linux enables local interrupts only after the CPU is online and its
    // local clock event is installed. An IPI sent after the ready publication
    // remains pending across this short window and is serviced after STI.
    unsafe {
        core::arch::asm!("sti", options(nostack));
    }

    crate::kernel::sched::idle::cpu_startup_entry()
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
pub fn run_ipi_ping_test() {
    // Logical CPU1 is the first AP accepted by prepare_cpus().
    let Some(apic_id) = logical_cpu_to_apic_id(1) else {
        panic!("smp: IPI ping test FAILED - no enabled non-BSP CPU found");
    };

    // Record the counter value before sending the IPI.
    let before = IPI_RECEIVED_COUNT.load(Ordering::Acquire);

    // Send fixed IPI to the AP at our ping vector.
    // Reference: Intel SDM Vol. 3A §10.6.1 "Interrupt Command Register (ICR)"
    unsafe {
        apic::send_ipi(apic_id, IPI_PING_VECTOR);
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
                apic_id
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
                apic_id
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
        assert_eq!(AP_ALIVE_MASK.load(Ordering::Relaxed), 0);
        assert_eq!(AP_READY_MASK.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn ipi_received_count_starts_at_zero() {
        assert_eq!(IPI_RECEIVED_COUNT.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn logical_cpu_map_starts_unpublished() {
        assert_eq!(LOGICAL_CPU_COUNT.load(Ordering::Relaxed), 0);
        assert_eq!(logical_cpu_to_apic_id(0), None);
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
    fn ap_stack_is_scheduler_aligned() {
        assert_eq!(
            align_of::<ApStack>(),
            AP_STACK_ALIGNMENT,
            "AP stack alignment must match the scheduler stack window"
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
        assert_eq!(
            AP_STACK_SIZE,
            crate::kernel::sched::KTHREAD_STACK_SIZE,
            "AP and scheduler kernel stacks must have identical size"
        );
        assert_eq!(ap_stack_top(0), None, "logical CPU 0 uses the BSP stack");
        for cpu in 1..MAX_CPUS as u32 {
            let top = ap_stack_top(cpu).expect("every supported AP needs a stack");
            assert_eq!(top % AP_STACK_ALIGNMENT, 0);
        }
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
    fn ap_stack_size_matches_scheduler_profile() {
        assert_eq!(AP_STACK_SIZE, crate::kernel::sched::KTHREAD_STACK_SIZE);
        assert_eq!(AP_STACK_ALIGNMENT, crate::kernel::sched::KTHREAD_STACK_SIZE);
    }
}
