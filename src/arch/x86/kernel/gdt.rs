//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel
//! test-origin: linux:vendor/linux/arch/x86/kernel
//! Per-CPU x86-64 Global Descriptor Table (GDT).
//!
//! The minimal GDT set up by `arch/x86/boot/header.S` has only three entries (null, code,
//! data). This module replaces it with Linux's generic x86-64 16-entry layout:
//!
//! | Index | Selector | Purpose                                      |
//! |-------|----------|----------------------------------------------|
//! | 1     | 0x08     | 32-bit kernel code                          |
//! | 2     | 0x10     | 64-bit kernel code                          |
//! | 3     | 0x18     | kernel data                                 |
//! | 4     | 0x23     | default 32-bit user code                    |
//! | 5     | 0x2b     | default user data / SYSRET SS               |
//! | 6     | 0x33     | default 64-bit user code / SYSRET CS        |
//! | 8–9   | 0x40     | 64-bit TSS descriptor                       |
//! | 10–11 | 0x50     | LDT descriptor                              |
//! | 12–14 | 0x63…73  | per-task TLS descriptors                    |
//! | 15    | 0x7b     | CPU/node segment                            |
//!
//! Why is the TSS 16 bytes?  In 64-bit mode the base address of the TSS is
//! 64 bits wide, so the descriptor is extended to 128 bits by appending a
//! second 8-byte slot in the GDT.
//!
//! The SYSCALL/SYSRET instruction uses the
//! STAR MSR to derive segment selectors via fixed arithmetic:
//!   SYSRET CS = STAR[63:48] + 16   → 0x23 + 16 = 0x33 = USER_CS ✓
//!   SYSRET SS = STAR[63:48] + 8    → 0x23 + 8  = 0x2b = USER_DS ✓
//!
//! References:
//!   Intel SDM Vol. 3A §3.4 "Segment Descriptors"
//!   Intel SDM Vol. 3A §7.2.3 "TSS Descriptor in 64-bit mode"
//!   vendor/linux/arch/x86/kernel/cpu/common.c
//!   https://wiki.osdev.org/Global_Descriptor_Table
//!   https://wiki.osdev.org/GDT_Tutorial
//!
//! Remaining descriptor-table work outside this TLS path: Linux's read-only
//! fixmap alias and a populated CPU/node descriptor are not implemented.

use core::mem::size_of;

use super::tss::Tss;
use crate::kernel::sched::MAX_CPUS;
use crate::kernel::thread::{DescStruct, ThreadStruct};

pub const GDT_ENTRIES: usize = 16;
pub const GDT_SIZE: usize = GDT_ENTRIES * size_of::<GdtEntry>();
pub const GDT_ENTRY_KERNEL32_CS: usize = 1;
pub const GDT_ENTRY_KERNEL_CS: usize = 2;
pub const GDT_ENTRY_KERNEL_DS: usize = 3;
pub const GDT_ENTRY_DEFAULT_USER32_CS: usize = 4;
pub const GDT_ENTRY_DEFAULT_USER_DS: usize = 5;
pub const GDT_ENTRY_DEFAULT_USER_CS: usize = 6;
pub const GDT_ENTRY_TSS: usize = 8;
pub const GDT_ENTRY_LDT: usize = 10;
pub const GDT_ENTRY_TLS_MIN: usize = 12;
pub const GDT_ENTRY_TLS_ENTRIES: usize = 3;
pub const GDT_ENTRY_TLS_MAX: usize = GDT_ENTRY_TLS_MIN + GDT_ENTRY_TLS_ENTRIES - 1;
pub const GDT_ENTRY_CPUNODE: usize = 15;

// ── Segment selector constants ───────────────────────────────────────────────
//
// A segment selector is a 16-bit index into the GDT (bits 15:3), plus a TI bit
// (bit 2, 0 = GDT) and a 2-bit Requested Privilege Level (bits 1:0).
//
// Reference: Intel SDM Vol. 3A §3.4.2 "Segment Selectors"

pub mod sel {
    use super::{
        GDT_ENTRY_DEFAULT_USER_CS, GDT_ENTRY_DEFAULT_USER_DS, GDT_ENTRY_DEFAULT_USER32_CS,
        GDT_ENTRY_KERNEL_CS, GDT_ENTRY_KERNEL_DS, GDT_ENTRY_TSS,
    };

    /// Null descriptor — never used as a selector, but must be present.
    pub const NULL: u16 = 0x00;
    /// Kernel 64-bit code segment (RPL = 0, DPL = 0).
    pub const KERNEL_CS: u16 = (GDT_ENTRY_KERNEL_CS * 8) as u16;
    /// Kernel data segment (RPL = 0, DPL = 0).
    pub const KERNEL_DS: u16 = (GDT_ENTRY_KERNEL_DS * 8) as u16;
    /// User 32-bit compatibility code segment with RPL = 3.
    pub const USER32_CS: u16 = (GDT_ENTRY_DEFAULT_USER32_CS * 8 + 3) as u16;
    /// User data segment with RPL = 3 (DPL = 3).
    ///
    /// Used as SS on SYSRET: STAR[63:48]=0x23, SYSRET SS = 0x2b.
    pub const USER_DS: u16 = (GDT_ENTRY_DEFAULT_USER_DS * 8 + 3) as u16;
    /// User 64-bit code segment with RPL = 3 (DPL = 3, L = 1).
    ///
    /// Used as CS on SYSRET: STAR[63:48]=0x23, SYSRET CS = 0x33.
    pub const USER_CS: u16 = (GDT_ENTRY_DEFAULT_USER_CS * 8 + 3) as u16;
    /// TSS selector — no RPL (loaded via `ltr`, a privileged instruction).
    pub const TSS: u16 = (GDT_ENTRY_TSS * 8) as u16;
}

// ── 8-byte GDT entry ─────────────────────────────────────────────────────────
//
// Bit layout of a code or data segment descriptor:
//
//  63:56  Base[31:24]
//  55     G  — Granularity: 0=byte, 1=4KiB page units for limit
//  54     D  — Default size: 0=16-bit, 1=32-bit (must be 0 when L=1)
//  53     L  — Long mode code: 1=64-bit code, 0=other
//  52     AVL — Available for software use
//  51:48  Limit[19:16]
//  47     P  — Present: must be 1 for a valid descriptor
//  46:45  DPL — Descriptor Privilege Level (0=kernel, 3=user)
//  44     S  — Descriptor type: 1=code/data, 0=system (TSS, LDT, …)
//  43:40  Type — See Intel SDM Table 3-1 (code/data) or 3-2 (system)
//  39:16  Base[23:0]
//  15:0   Limit[15:0]
//
// In 64-bit mode the processor ignores limit/base for code and data segments
// (effective base is always 0, limit is ignored).  We still set them to the
// conventional values (base=0, limit=0xFFFFF) for correctness.
//
// Reference: Intel SDM Vol. 3A Figure 3-8 "Segment Descriptor"

/// A single 8-byte GDT descriptor.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct GdtEntry(u64);

impl GdtEntry {
    /// Required null descriptor — the first GDT entry must always be zero.
    pub const fn null() -> Self {
        Self(0)
    }

    /// Build a code or data descriptor from an access byte and flags nibble.
    ///
    /// - `access`  — byte 5 of the descriptor (P | DPL(2) | S | Type(4))
    /// - `flags`   — upper nibble of byte 6 (G | D | L | AVL)
    ///
    /// The limit is set to 0xFFFFF (all 20 bits = max) with G=1 (4 KiB granularity)
    /// covering the full 4 GiB address space.  Base is 0.
    const fn segment(access: u8, flags: u8) -> Self {
        //  Bits  0-15: limit_low  = 0xFFFF
        //  Bits 16-39: base_low   = 0x000000
        //  Bits 40-47: access     = access
        //  Bits 48-51: limit_high = 0xF
        //  Bits 52-55: flags      = flags (upper nibble, 4 bits)
        //  Bits 56-63: base_high  = 0x00
        let limit_low: u64 = 0xFFFF;
        let access64: u64 = (access as u64) << 40;
        let limit_high: u64 = 0xF << 48;
        let flags64: u64 = ((flags & 0xF) as u64) << 52;
        Self(limit_low | access64 | limit_high | flags64)
    }

    /// 64-bit kernel code segment (DPL=0, L=1).
    ///
    /// Access byte 0x9B = P(1) | DPL(00) | S(1) | Type(1011)
    ///   Type 0xB = accessed Execute/Read code segment.
    /// Flags nibble 0xA = G(1) | D(0) | L(1) | AVL(0)
    ///   L=1 → 64-bit code segment; D MUST be 0 when L=1 (SDM §3.4.5.1).
    pub const fn kernel_code64() -> Self {
        Self::segment(0x9B, 0xA)
    }

    /// 32-bit kernel compatibility code segment.
    pub const fn kernel_code32() -> Self {
        Self::segment(0x9B, 0xC)
    }

    /// Kernel data segment (DPL=0).
    ///
    /// Access byte 0x93 = P(1) | DPL(00) | S(1) | Type(0011)
    ///   Type 0x3 = accessed Read/Write data segment.
    /// Flags nibble 0xC = G(1) | D(1) | L(0) | AVL(0)
    ///   D=1 → 32-bit default size for data (irrelevant in 64-bit mode,
    ///          but conventional to set).
    pub const fn kernel_data() -> Self {
        Self::segment(0x93, 0xC)
    }

    /// User data segment (DPL=3).
    ///
    /// Access byte 0xF3 = P(1) | DPL(11) | S(1) | Type(0011)
    pub const fn user_data() -> Self {
        Self::segment(0xF3, 0xC)
    }

    /// User 64-bit code segment (DPL=3, L=1).
    ///
    /// Access byte 0xFB = P(1) | DPL(11) | S(1) | Type(1011)
    pub const fn user_code64() -> Self {
        Self::segment(0xFB, 0xA)
    }

    /// User 32-bit compatibility code segment.
    pub const fn user_code32() -> Self {
        Self::segment(0xFB, 0xC)
    }
}

// ── Full GDT structure ───────────────────────────────────────────────────────
//
// The TSS descriptor spans two consecutive 8-byte slots (`tss_low` + `tss_high`)
// because it needs to encode a 64-bit base address.
//
// Reference: Intel SDM Vol. 3A §7.2.3 "TSS Descriptor in 64-bit mode"
// Reference: Intel SDM Vol. 3A Figure 7-4 "Format of TSS and LDT Descriptors in 64-bit Mode"

/// One page-aligned per-CPU GDT page, matching Linux `struct gdt_page`.
///
/// `#[repr(C)]` and page aligned like Linux's per-CPU allocation; `lgdt`
/// reads the GDTR base without alignment requirements but whose address we
/// pass directly from Rust.
#[repr(C, align(4096))]
#[derive(Clone, Copy)]
pub struct Gdt {
    entries: [GdtEntry; GDT_ENTRIES],
}

/// GDTR value — the 10-byte operand for `lgdt` / `sgdt`.
///
/// `#[repr(C, packed)]` is critical: `lgdt` reads the limit as bytes 0–1
/// and the base as bytes 2–9 with no alignment padding between them.
///
/// Reference: Intel SDM Vol. 3A §2.4 "Memory-Management Registers"
#[repr(C, packed)]
struct GdtRegister {
    limit: u16, // GDT size in bytes minus 1
    base: u64,  // Linear address of the first GDT entry
}

/// Global kernel GDT.  Declared `static mut` so we can call `set_tss()` once
/// at boot (single-threaded init) and so the address is `'static`.
pub static mut GDT: Gdt = Gdt::new();

/// Private GDTs for APs.
///
/// `ltr` marks a TSS descriptor busy, so APs cannot safely reuse CPU0's GDT
/// entry. Linux installs a per-CPU TSS descriptor; this array gives each AP its
/// own descriptor slot while preserving the existing CPU0 `GDT` symbol.
static mut AP_GDTS: [Gdt; MAX_CPUS] = [Gdt::new(); MAX_CPUS];

const fn cpu_slot(cpu: usize) -> usize {
    if cpu >= MAX_CPUS { MAX_CPUS - 1 } else { cpu }
}

unsafe fn gdt_for_cpu_mut(cpu: usize) -> *mut Gdt {
    let slot = cpu_slot(cpu);
    if slot == 0 {
        &raw mut GDT
    } else {
        &raw mut AP_GDTS[slot]
    }
}

/// Copy `thread.tls_array` into GDT entries 12..=14 for `cpu`.
///
/// This is Linux `native_load_tls()`. The caller must pin execution to the
/// supplied CPU until the copy finishes.
///
/// # Safety
/// `cpu` must identify the CPU whose GDT is active for the calling switch or
/// syscall path.
pub unsafe fn load_tls(thread: &ThreadStruct, cpu: usize) {
    let gdt = unsafe { gdt_for_cpu_mut(cpu) };
    unsafe {
        (*gdt).install_tls(thread.tls_array);
    }
}

#[cfg(not(test))]
#[inline]
pub unsafe fn read_ds() -> u16 {
    let selector: u64;
    unsafe {
        core::arch::asm!(
            "mov {selector:x}, ds",
            selector = lateout(reg) selector,
            options(nomem, nostack, preserves_flags),
        );
    }
    selector as u16
}

#[cfg(test)]
#[inline]
pub const unsafe fn read_ds() -> u16 {
    0
}

#[cfg(not(test))]
#[inline]
pub unsafe fn read_es() -> u16 {
    let selector: u64;
    unsafe {
        core::arch::asm!(
            "mov {selector:x}, es",
            selector = lateout(reg) selector,
            options(nomem, nostack, preserves_flags),
        );
    }
    selector as u16
}

#[cfg(test)]
#[inline]
pub const unsafe fn read_es() -> u16 {
    0
}

#[cfg(not(test))]
#[inline]
pub unsafe fn read_fs() -> u16 {
    let selector: u64;
    unsafe {
        core::arch::asm!(
            "mov {selector:x}, fs",
            selector = lateout(reg) selector,
            options(nomem, nostack, preserves_flags),
        );
    }
    selector as u16
}

#[cfg(test)]
#[inline]
pub const unsafe fn read_fs() -> u16 {
    0
}

#[cfg(not(test))]
#[inline]
pub unsafe fn read_gs() -> u16 {
    let selector: u64;
    unsafe {
        core::arch::asm!(
            "mov {selector:x}, gs",
            selector = lateout(reg) selector,
            options(nomem, nostack, preserves_flags),
        );
    }
    selector as u16
}

#[cfg(test)]
#[inline]
pub const unsafe fn read_gs() -> u16 {
    0
}

#[cfg(not(test))]
#[inline]
pub unsafe fn load_ds(selector: u16) {
    let selector = selector as u64;
    unsafe {
        core::arch::asm!(
            "mov ds, {selector:x}",
            selector = in(reg) selector,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
pub const unsafe fn load_ds(_selector: u16) {}

#[cfg(not(test))]
#[inline]
pub unsafe fn load_es(selector: u16) {
    let selector = selector as u64;
    unsafe {
        core::arch::asm!(
            "mov es, {selector:x}",
            selector = in(reg) selector,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
pub const unsafe fn load_es(_selector: u16) {}

#[cfg(not(test))]
#[inline]
pub unsafe fn load_fs(selector: u16) {
    let selector = selector as u64;
    unsafe {
        core::arch::asm!(
            "mov fs, {selector:x}",
            selector = in(reg) selector,
            options(nostack, preserves_flags),
        );
    }
}

#[cfg(test)]
#[inline]
pub const unsafe fn load_fs(_selector: u16) {}

/// Load the inactive (userspace) GS selector while kernel GS is active.
///
/// Linux implements this as `asm_load_gs_index`: disable interrupts, SWAPGS,
/// load the selector, and SWAPGS back.
#[cfg(not(test))]
#[inline]
pub unsafe fn load_gs_index(selector: u16) {
    let flags = crate::kernel::locking::irqflags::local_irq_save();
    let selector = selector as u64;
    unsafe {
        core::arch::asm!(
            "swapgs",
            "mov gs, {selector:x}",
            "swapgs",
            selector = in(reg) selector,
            options(nostack, preserves_flags),
        );
    }
    crate::kernel::locking::irqflags::local_irq_restore(flags);
}

#[cfg(test)]
#[inline]
pub const unsafe fn load_gs_index(_selector: u16) {}

impl Gdt {
    /// Construct Linux's generic x86-64 GDT. TSS, LDT, TLS, and CPU/node
    /// slots remain empty until their owning paths populate them.
    pub const fn new() -> Self {
        let mut entries = [GdtEntry::null(); GDT_ENTRIES];
        entries[GDT_ENTRY_KERNEL32_CS] = GdtEntry::kernel_code32();
        entries[GDT_ENTRY_KERNEL_CS] = GdtEntry::kernel_code64();
        entries[GDT_ENTRY_KERNEL_DS] = GdtEntry::kernel_data();
        entries[GDT_ENTRY_DEFAULT_USER32_CS] = GdtEntry::user_code32();
        entries[GDT_ENTRY_DEFAULT_USER_DS] = GdtEntry::user_data();
        entries[GDT_ENTRY_DEFAULT_USER_CS] = GdtEntry::user_code64();
        Self { entries }
    }

    /// Fill the TSS descriptor slots from a raw pointer to the TSS.
    ///
    /// The 64-bit TSS descriptor format (SDM Vol. 3A Figure 7-4):
    ///
    /// Low 8 bytes:
    ///   63:56  base[31:24]
    ///   55:52  flags (G=0, AVL=0, 0, 0)
    ///   51:48  limit[19:16]
    ///   47:40  access: P=1 | DPL=0 | S=0 | type=0x9 (available 64-bit TSS)
    ///   39:16  base[23:0]
    ///   15:0   limit[15:0]
    ///
    /// High 8 bytes:
    ///   63:32  reserved (must be 0)
    ///   31:0   base[63:32]
    ///
    /// `tss` must point to a `'static` object — the GDT entry stores its address.
    pub fn set_tss(&mut self, tss: *const Tss) {
        let base = tss as u64;
        let limit = (size_of::<Tss>() - 1) as u64;

        let mut low: u64 = 0;
        low |= limit & 0xFFFF; // limit[15:0]
        low |= (base & 0x00FF_FFFF) << 16; // base[23:0]
        low |= 0x89u64 << 40; // P=1, DPL=0, type=0x9 (available TSS)
        low |= ((limit >> 16) & 0xF) << 48; // limit[19:16]
        low |= ((base >> 24) & 0xFF) << 56; // base[31:24]

        // High 8 bytes: base[63:32] in the low 32 bits, upper 32 bits reserved.
        let high: u64 = (base >> 32) & 0xFFFF_FFFF;

        self.entries[GDT_ENTRY_TSS] = GdtEntry(low);
        self.entries[GDT_ENTRY_TSS + 1] = GdtEntry(high);
    }

    /// Linux `native_load_tls()`: copy all three task TLS descriptors to the
    /// CPU-local GDT before any selector belonging to the task is restored.
    fn install_tls(&mut self, tls: [DescStruct; GDT_ENTRY_TLS_ENTRIES]) {
        let mut index = 0;
        while index < GDT_ENTRY_TLS_ENTRIES {
            self.entries[GDT_ENTRY_TLS_MIN + index] = GdtEntry(tls[index].0);
            index += 1;
        }
    }

    /// Load a GDT, reload all segment registers, and load the TSS.
    ///
    /// Steps:
    ///   1. `lgdt`  — point GDTR at the GDT at `gdt_ptr`.
    ///   2. Far return (`retfq`) — the only reliable way to reload CS in 64-bit
    ///      mode without a `ljmp` (not directly encodable in 64-bit mode).
    ///   3. Reload DS, ES, SS with `KERNEL_DS`.  Zero FS and GS.
    ///   4. `ltr`   — load the TSS selector into the Task Register.
    ///
    /// # Safety
    /// - `gdt_ptr` must point to a valid, fully-populated `Gdt` that will
    ///   remain at a fixed address for the lifetime of the kernel.
    /// - `set_tss()` must have been called on the GDT before this.
    /// - Must be called once, from `kernel_main`, before any interrupt fires.
    pub unsafe fn load(gdt_ptr: *const Gdt) {
        let reg = GdtRegister {
            // Linux exposes only the 16 descriptor entries through GDTR. The
            // page-aligned tail is allocation padding, not part of the table.
            limit: (GDT_SIZE - 1) as u16,
            base: gdt_ptr as u64,
        };

        unsafe {
            core::arch::asm!(
                // 1. Load the GDTR.
                "lgdt [{reg}]",

                // 2. Reload CS via a far return.
                //
                // In 64-bit mode there is no `ljmp` to a near offset — the
                // only way to load a new CS is via `retfq` (far return) or
                // `iretq`.  We push [CS, RIP] on the stack, then `retfq` pops
                // RIP first and CS second.
                //
                // Stack layout after the two pushes (low addr → high addr):
                //   [RSP+0] = return address (label `2:`)   → loaded into RIP
                //   [RSP+8] = new CS selector (KERNEL_CS)   → loaded into CS
                //
                // Note: avoid labels `0` and `1` — LLVM bug makes them
                // ambiguous with binary literals on x86 (GitHub #99547).
                //
                // Reference: https://wiki.osdev.org/Reloading_Segment_Registers_Automatically
                "push {cs}",
                "lea {tmp}, [rip + 2f]",
                "push {tmp}",
                "retfq",
                "2:",

                // 3. Reload data/stack segments with KERNEL_DS.
                // In 64-bit mode DS/ES/SS are mostly ignored for addressing,
                // but their descriptors must be valid for CPL=0 execution.
                // FS and GS are zeroed here; they will be used for per-CPU
                // data in a later milestone.
                "mov {ds:x}, {kds}",
                "mov ds, {ds:x}",
                "mov es, {ds:x}",
                "mov ss, {ds:x}",
                "xor {ds:e}, {ds:e}",
                "mov fs, {ds:x}",
                "mov gs, {ds:x}",

                // 4. Load the TSS selector into the Task Register.
                // `ltr` marks the TSS descriptor as "busy" (type 0x9 → 0xB).
                // This is normal and expected.
                "ltr {tss:x}",

                reg = in(reg) &reg,
                cs  = in(reg) sel::KERNEL_CS as u64,
                tmp = out(reg) _,
                kds = const sel::KERNEL_DS,
                ds  = out(reg) _,
                tss = in(reg) sel::TSS,
                options(preserves_flags),
            );
        }

        let tr = task_register_selector();
        assert_eq!(
            tr,
            sel::TSS,
            "GDT load left TR at selector {tr:#x}, expected TSS selector {:#x}",
            sel::TSS
        );
    }
}

/// Read the visible selector in the CPU Task Register.
///
/// This mirrors the post-`ltr` sanity Linux relies on when installing the
/// per-CPU TSS descriptor from `vendor/linux/arch/x86/kernel/cpu/common.c`.
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn task_register_selector() -> u16 {
    let selector: u64;
    unsafe {
        core::arch::asm!(
            "str {selector:x}",
            selector = lateout(reg) selector,
            options(nomem, nostack, preserves_flags),
        );
    }
    selector as u16
}

/// Initialise and load the global GDT.
///
/// Must be called after `tss::init()`.
///
/// # Safety
/// Same as `Gdt::load()`.
pub unsafe fn init() {
    // Rust 2024: use raw pointers instead of creating &/&mut to static muts.
    unsafe {
        let gdt = &raw mut GDT;
        gdt.write(Gdt::new());
        (*gdt).set_tss(&raw const super::tss::TSS);
        Gdt::load(&raw const GDT);
    }
}

/// Initialise and load the descriptor state for an AP.
///
/// This mirrors Linux's per-CPU TSS/GDT bring-up: each CPU owns a TSS and the
/// GDT descriptor that `ltr` marks busy. The IDT remains global and is already
/// loaded by the AP trampoline before Rust code runs.
///
/// # Safety
/// Must run on the target AP before interrupts are enabled on that AP.
pub unsafe fn init_ap(cpu: usize) {
    unsafe {
        super::tss::init_cpu(cpu);
        let gdt = gdt_for_cpu_mut(cpu);
        gdt.write(Gdt::new());
        (*gdt).set_tss(super::tss::tss_for_cpu(cpu));
        Gdt::load(gdt as *const Gdt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    // ── Descriptor encoding tests ────────────────────────────────────────────
    // We verify the bit layout of each descriptor matches the Intel SDM spec.
    // These run on the host (no hardware access needed).

    fn present(e: GdtEntry) -> bool {
        e.0 & (1 << 47) != 0
    }

    fn dpl(e: GdtEntry) -> u8 {
        ((e.0 >> 45) & 0x3) as u8
    }

    fn l_bit(e: GdtEntry) -> bool {
        e.0 & (1 << 53) != 0
    }

    fn d_bit(e: GdtEntry) -> bool {
        e.0 & (1 << 54) != 0
    }

    #[test]
    fn null_descriptor_is_all_zero() {
        assert_eq!(GdtEntry::null().0, 0);
    }

    #[test]
    fn all_valid_descriptors_have_present_bit() {
        for e in [
            GdtEntry::kernel_code32(),
            GdtEntry::kernel_code64(),
            GdtEntry::kernel_data(),
            GdtEntry::user_code32(),
            GdtEntry::user_code64(),
            GdtEntry::user_data(),
        ] {
            assert!(present(e), "P bit must be set: {:#018x}", e.0);
        }
    }

    #[test]
    fn kernel_code64_l_bit_set_d_bit_clear() {
        // SDM §3.4.5.1: L=1 selects 64-bit code; D must be 0 when L=1.
        let e = GdtEntry::kernel_code64();
        assert!(l_bit(e), "kernel code64: L bit must be 1");
        assert!(
            !d_bit(e),
            "kernel code64: D bit must be 0 when L=1 (SDM §3.4.5.1)"
        );
    }

    #[test]
    fn user_code64_l_bit_set() {
        assert!(
            l_bit(GdtEntry::user_code64()),
            "user code64: L bit must be 1"
        );
    }

    #[test]
    fn kernel_descriptors_are_ring0() {
        assert_eq!(
            dpl(GdtEntry::kernel_code64()),
            0,
            "kernel code DPL must be 0"
        );
        assert_eq!(dpl(GdtEntry::kernel_data()), 0, "kernel data DPL must be 0");
    }

    #[test]
    fn user_descriptors_are_ring3() {
        assert_eq!(dpl(GdtEntry::user_code64()), 3, "user code DPL must be 3");
        assert_eq!(dpl(GdtEntry::user_data()), 3, "user data DPL must be 3");
    }

    #[test]
    fn gdt_page_has_linux_size_and_descriptor_limit() {
        // Origin: vendor/linux/arch/x86/include/asm/desc.h `struct gdt_page`
        // and vendor/linux/arch/x86/include/asm/segment.h `GDT_SIZE`.
        assert_eq!(size_of::<Gdt>(), 4096);
        assert_eq!(core::mem::align_of::<Gdt>(), 4096);
        assert_eq!(GDT_SIZE, 16 * 8);
        assert_eq!(offset_of!(Gdt, entries), 0);
    }

    #[test]
    fn gdtr_operand_is_10_bytes() {
        assert_eq!(
            size_of::<GdtRegister>(),
            10,
            "GDTR operand must be 10 bytes"
        );
        assert_eq!(
            offset_of!(GdtRegister, limit),
            0,
            "GDTR limit must start at byte 0"
        );
        assert_eq!(
            offset_of!(GdtRegister, base),
            2,
            "GDTR base must start at byte 2"
        );
    }

    #[test]
    fn selector_constants_are_consistent_with_gdt_layout() {
        // Origin: vendor/linux/arch/x86/include/asm/segment.h.
        assert_eq!(sel::KERNEL_CS, 0x10);
        assert_eq!(sel::KERNEL_DS, 0x18);
        assert_eq!(sel::USER32_CS, 0x23);
        assert_eq!(sel::USER_DS, 0x2b);
        assert_eq!(sel::USER_CS, 0x33);
        assert_eq!(sel::TSS, 0x40);
        assert_eq!(GDT_ENTRY_TLS_MIN, 12);
        assert_eq!(GDT_ENTRY_TLS_MAX, 14);
    }

    #[test]
    fn tss_descriptor_occupies_linux_entries_eight_and_nine() {
        let mut gdt = Gdt::new();
        gdt.set_tss(0x1234_5678_9abc_def0usize as *const Tss);
        assert_ne!(gdt.entries[GDT_ENTRY_TSS].0, 0);
        assert_eq!(
            gdt.entries[GDT_ENTRY_TSS + 1].0,
            0x1234_5678,
            "upper TSS descriptor holds base[63:32]"
        );
        assert_eq!(gdt.entries[7].0, 0);
        assert_eq!(gdt.entries[GDT_ENTRY_LDT].0, 0);
    }

    #[test]
    fn native_load_tls_updates_exactly_linux_slots_twelve_through_fourteen() {
        // Origin: vendor/linux/arch/x86/include/asm/desc.h::native_load_tls.
        let mut gdt = Gdt::new();
        let before = gdt.entries;
        let tls = [
            DescStruct(0x1111_1111_1111_1111),
            DescStruct(0x2222_2222_2222_2222),
            DescStruct(0x3333_3333_3333_3333),
        ];
        gdt.install_tls(tls);

        for index in 0..GDT_ENTRIES {
            if (GDT_ENTRY_TLS_MIN..=GDT_ENTRY_TLS_MAX).contains(&index) {
                assert_eq!(gdt.entries[index].0, tls[index - GDT_ENTRY_TLS_MIN].0);
            } else {
                assert_eq!(gdt.entries[index].0, before[index].0);
            }
        }
    }

    #[test]
    fn per_cpu_gdt_slot_clamps_to_sched_cpu_storage() {
        assert!(MAX_CPUS >= 2);
        assert_eq!(cpu_slot(0), 0);
        assert_eq!(cpu_slot(1), 1);
        assert_eq!(cpu_slot(MAX_CPUS + 4), MAX_CPUS - 1);
    }

    #[test]
    fn sysret_selectors_obey_star_arithmetic() {
        // SYSRET 64-bit: CS = STAR[63:48] + 16, SS = STAR[63:48] + 8
        // Linux programs STAR[63:48] to __USER32_CS (0x23).
        let star_base = sel::USER32_CS as u32;
        assert_eq!(
            star_base + 8,
            sel::USER_DS as u32,
            "SYSRET SS must map to USER_DS"
        );
        assert_eq!(
            star_base + 16,
            sel::USER_CS as u32,
            "SYSRET CS must map to USER_CS"
        );
    }

    #[test]
    fn runtime_selector_inventory_has_no_pre_linux_layout_consumers() {
        // Behavioral assertions above prove the table. This inventory catches
        // the few assembly/standalone constants that cannot import `sel`.
        assert_eq!(crate::arch::x86::kernel::kgdb::KERNEL_CS, 0x10);
        assert_eq!(crate::arch::x86::kernel::kgdb::KERNEL_DS, 0x18);
        assert_eq!(crate::arch::x86::kernel::fred::KERNEL_DS, 0x18);
        assert_eq!(crate::arch::x86::entry::syscall_32::USER32_CS, 0x23);
        assert_eq!(crate::arch::x86::entry::syscall_32::USER_DS, 0x2b);

        let trampoline = include_str!("../realmode/rm/trampoline_64.S");
        assert!(trampoline.contains("push qword 0x10"));
        assert!(trampoline.contains("BSP's KERNEL_DS = 0x18"));
    }
}
