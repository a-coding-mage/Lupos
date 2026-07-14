//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kernel/cpu/common.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/common.c
//! x86 per-CPU ABI symbols exported to Linux-built modules.
//!
//! Also carries the single-CPU cpumask objects Linux modules reference from
//! `vendor/linux/kernel/cpu.c`.

use core::ptr;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use crate::kernel::module::{export_symbol, find_symbol};

static LINUX_CONST_CURRENT_TASK: AtomicUsize = AtomicUsize::new(0);

/// `struct cpuinfo_x86 boot_cpu_data` - `vendor/linux/arch/x86/kernel/setup.c:133`.
///
/// The module loader uses `BOOT_CPU_CAPS` below for alternatives. This data
/// symbol is for runtime module reads of the Linux `cpuinfo_x86` layout; keep
/// it conservative until the full struct is populated.
static mut LINUX_BOOT_CPU_DATA: [u8; 512] = [0; 512];
static LINUX_BOOT_CPU_DATA_INITIALIZED: AtomicBool = AtomicBool::new(false);
static LINUX_MAX_DIES_PER_PACKAGE: AtomicU32 = AtomicU32::new(1);
static LINUX_MAX_LOGICAL_PACKAGES: AtomicU32 = AtomicU32::new(1);

// ── Boot-CPU capability words ────────────────────────────────────────────────
//
// Mirror of `boot_cpu_data.x86_capability` (`vendor/linux/arch/x86/kernel/
// cpu/common.c::get_cpu_cap` + the software bits `identify_cpu` forces).
// Consumers: the module loader's `apply_alternatives`, which patches vendor
// `.altinstructions` sites by testing these exact `X86_FEATURE_*` numbers.
//
// The words must reflect what Lupos actually enabled, not raw CPUID —
// `setup_smap()` parity below clears X86_FEATURE_SMAP unless CR4.SMAP is
// really set, because a patched-in STAC/CLAC would #UD otherwise.

/// `NCAPINTS`/`NBUGINTS` — `vendor/linux/arch/x86/include/asm/cpufeatures.h:8`.
pub const NCAPINTS: usize = 22;
pub const NBUGINTS: usize = 2;

pub const X86_FEATURE_ALWAYS: u32 = 3 * 32 + 21;
pub const X86_FEATURE_REP_GOOD: u32 = 3 * 32 + 16;
pub const X86_FEATURE_XMM2: u32 = 0 * 32 + 26;
pub const X86_FEATURE_ERMS: u32 = 9 * 32 + 9;
pub const X86_FEATURE_SMAP: u32 = 9 * 32 + 20;
pub const X86_FEATURE_DTHERM: u32 = 14 * 32;
pub const X86_FEATURE_PTS: u32 = 14 * 32 + 6;
pub const X86_FEATURE_LFENCE_RDTSC: u32 = 20 * 32 + 2;

pub const LINUX_CPUINFO_X86_SIZE: usize = 512;

// `struct cpuinfo_x86::x86_capability` offset for the vendor x86_64
// configuration (`CONFIG_X86_VMX_FEATURE_NAMES=y`).
const LINUX_CPUINFO_X86_CAPABILITY_OFFSET: usize = 48;
const LINUX_CPUINFO_X86_MODEL_OFFSET: usize = 0;
const LINUX_CPUINFO_X86_FAMILY_OFFSET: usize = 1;
const LINUX_CPUINFO_X86_VENDOR_OFFSET: usize = 2;
const LINUX_CPUINFO_X86_STEPPING_OFFSET: usize = 4;
const LINUX_CPUINFO_X86_CPUID_LEVEL_OFFSET: usize = 40;

const X86_CR4_SMAP: u64 = 1 << 21;
const MSR_IA32_MISC_ENABLE: u32 = 0x1a0;
const MSR_IA32_MISC_ENABLE_FAST_STRING: u64 = 1;

#[allow(clippy::declare_interior_mutable_const)]
const CAP_WORD_INIT: AtomicU32 = AtomicU32::new(0);
static BOOT_CPU_CAPS: [AtomicU32; NCAPINTS + NBUGINTS] = [CAP_WORD_INIT; NCAPINTS + NBUGINTS];
static BOOT_CPU_CAPS_LOADED: AtomicBool = AtomicBool::new(false);

/// `boot_cpu_has(bit)` — true iff the boot CPU capability bit is set.
/// Out-of-range bits are false (callers validating module metadata reject
/// them separately, mirroring the vendor `BUG_ON` in `apply_alternatives`).
pub fn boot_cpu_has(bit: u32) -> bool {
    ensure_boot_cpu_caps();
    let word = (bit / 32) as usize;
    word < NCAPINTS + NBUGINTS
        && BOOT_CPU_CAPS[word].load(Ordering::Acquire) & (1 << (bit % 32)) != 0
}

/// Highest valid feature number plus one — the vendor
/// `(NCAPINTS + NBUGINTS) * 32` bound `apply_alternatives` enforces.
pub const fn x86_feature_limit() -> u32 {
    ((NCAPINTS + NBUGINTS) * 32) as u32
}

pub fn set_cpu_cap(bit: u32) {
    let word = (bit / 32) as usize;
    if word < NCAPINTS + NBUGINTS {
        BOOT_CPU_CAPS[word].fetch_or(1 << (bit % 32), Ordering::AcqRel);
    }
}

pub fn setup_clear_cpu_cap(bit: u32) {
    let word = (bit / 32) as usize;
    if word < NCAPINTS + NBUGINTS {
        BOOT_CPU_CAPS[word].fetch_and(!(1 << (bit % 32)), Ordering::AcqRel);
    }
}

fn ensure_boot_cpu_caps() {
    if BOOT_CPU_CAPS_LOADED.swap(true, Ordering::AcqRel) {
        return;
    }
    init_boot_cpu_caps();
}

/// CPUID-sourced capability words per `get_cpu_cap` (common.c), restricted
/// to the words vendor modules reference today (0, 1, 4, 6, 9, 12, 14, 16,
/// 18, 20), plus the software bits below.
fn init_boot_cpu_caps() {
    use crate::arch::x86::kernel::cpuid::cpuid;

    let leaf0 = cpuid(0, 0);
    let max_basic = leaf0.eax;
    let vendor = [leaf0.ebx, leaf0.edx, leaf0.ecx];
    const INTEL: [u32; 3] = [0x756e_6547, 0x4965_6e69, 0x6c65_746e]; // GenuineIntel
    const AMD: [u32; 3] = [0x6874_7541, 0x6974_6e65, 0x444d_4163]; // AuthenticAMD

    let mut family = 0u32;
    let mut model = 0u32;
    if (0x0000_0001..=0x0000_ffff).contains(&max_basic) {
        let l1 = cpuid(1, 0);
        BOOT_CPU_CAPS[0].store(l1.edx, Ordering::Release); // CPUID_1_EDX
        BOOT_CPU_CAPS[4].store(l1.ecx, Ordering::Release); // CPUID_1_ECX
        let tfms = l1.eax;
        family = (tfms >> 8) & 0xf;
        model = (tfms >> 4) & 0xf;
        if family == 0xf {
            family += (tfms >> 20) & 0xff;
        }
        if family >= 6 {
            model += ((tfms >> 16) & 0xf) << 4;
        }
    }
    if max_basic >= 0x0000_0007 {
        let l7 = cpuid(7, 0);
        BOOT_CPU_CAPS[9].store(l7.ebx, Ordering::Release); // CPUID_7_0_EBX
        BOOT_CPU_CAPS[16].store(l7.ecx, Ordering::Release); // CPUID_7_ECX
        BOOT_CPU_CAPS[18].store(l7.edx, Ordering::Release); // CPUID_7_EDX
        if l7.eax >= 1 {
            let l7_1 = cpuid(7, 1);
            BOOT_CPU_CAPS[12].store(l7_1.eax, Ordering::Release); // CPUID_7_1_EAX
        }
    }
    if max_basic >= 0x0000_0006 {
        BOOT_CPU_CAPS[14].store(cpuid(6, 0).eax, Ordering::Release); // CPUID_6_EAX
    }
    let max_ext = cpuid(0x8000_0000, 0).eax;
    if (0x8000_0001..=0x8000_ffff).contains(&max_ext) {
        let lext1 = cpuid(0x8000_0001, 0);
        BOOT_CPU_CAPS[6].store(lext1.ecx, Ordering::Release); // CPUID_8000_0001_ECX
        BOOT_CPU_CAPS[1].store(lext1.edx, Ordering::Release); // CPUID_8000_0001_EDX
    }
    if max_ext >= 0x8000_0021 {
        // AMD Extended Feature 2 — word 20 (X86_FEATURE_LFENCE_RDTSC lives here).
        BOOT_CPU_CAPS[20].store(cpuid(0x8000_0021, 0).eax, Ordering::Release);
    }

    // identify_cpu(): setup_force_cpu_cap(X86_FEATURE_ALWAYS) — common.c:1821.
    set_cpu_cap(X86_FEATURE_ALWAYS);

    if vendor == INTEL {
        // init_intel() — intel.c:302-311: REP_GOOD tracks the architectural
        // MISC_ENABLE.FAST_STRING knob on Dothan (f6 m13) and later.
        if family > 6 || (family == 6 && model >= 0x0d) {
            match unsafe { crate::arch::x86::kernel::msr::rdmsr_safe(MSR_IA32_MISC_ENABLE) } {
                Ok(misc) if misc & MSR_IA32_MISC_ENABLE_FAST_STRING != 0 => {
                    set_cpu_cap(X86_FEATURE_REP_GOOD);
                }
                _ => {
                    setup_clear_cpu_cap(X86_FEATURE_REP_GOOD);
                    setup_clear_cpu_cap(X86_FEATURE_ERMS);
                }
            }
        }
        // intel.c:541-542: LFENCE is always serializing given SSE2.
        if boot_cpu_has_raw(X86_FEATURE_XMM2) {
            set_cpu_cap(X86_FEATURE_LFENCE_RDTSC);
        }
    } else if vendor == AMD {
        // init_amd() — amd.c:1056-1057 (families 0x10+; the K8 C+ stepping
        // case at amd.c:700-702 is skipped: Lupos does not target K8).
        if family >= 0x10 {
            set_cpu_cap(X86_FEATURE_REP_GOOD);
        }
    }

    // setup_smap() parity: never advertise SMAP unless CR4.SMAP is truly
    // enabled — patched STAC/CLAC would #UD on a CR4 without it.
    #[cfg(not(test))]
    {
        let cr4: u64;
        unsafe {
            core::arch::asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack, preserves_flags));
        }
        if cr4 & X86_CR4_SMAP == 0 {
            setup_clear_cpu_cap(X86_FEATURE_SMAP);
        }
    }
}

fn boot_cpu_has_raw(bit: u32) -> bool {
    let word = (bit / 32) as usize;
    word < NCAPINTS + NBUGINTS
        && BOOT_CPU_CAPS[word].load(Ordering::Acquire) & (1 << (bit % 32)) != 0
}

fn linux_vendor_id() -> u8 {
    match super::CpuVendor::current() {
        super::CpuVendor::Intel => 0,
        super::CpuVendor::Amd => 2,
        super::CpuVendor::Centaur => 5,
        super::CpuVendor::Hygon => 9,
        super::CpuVendor::Zhaoxin => 10,
        super::CpuVendor::Unknown(_) => 0xff,
    }
}

fn linux_boot_cpu_caps_snapshot() -> [u32; NCAPINTS + NBUGINTS] {
    ensure_boot_cpu_caps();
    let mut caps = [0u32; NCAPINTS + NBUGINTS];
    for (index, word) in BOOT_CPU_CAPS.iter().enumerate() {
        caps[index] = word.load(Ordering::Acquire);
    }
    caps
}

pub(crate) fn write_linux_cpuinfo_x86(cpuinfo: *mut u8) {
    if cpuinfo.is_null() {
        return;
    }

    let leaf0 = crate::arch::x86::kernel::cpuid::cpuid(0, 0);
    let max_basic = leaf0.eax;
    let signature = if max_basic >= 1 {
        super::CpuSignature::from_leaf1_eax(crate::arch::x86::kernel::cpuid::cpuid(1, 0).eax)
    } else {
        super::CpuSignature {
            stepping: 0,
            model: 0,
            family: 0,
            processor_type: 0,
        }
    };
    let caps = linux_boot_cpu_caps_snapshot();

    unsafe {
        ptr::write_bytes(cpuinfo, 0, LINUX_CPUINFO_X86_SIZE);
        cpuinfo
            .add(LINUX_CPUINFO_X86_MODEL_OFFSET)
            .write(signature.model);
        cpuinfo
            .add(LINUX_CPUINFO_X86_FAMILY_OFFSET)
            .write(signature.family);
        cpuinfo
            .add(LINUX_CPUINFO_X86_VENDOR_OFFSET)
            .write(linux_vendor_id());
        cpuinfo
            .add(LINUX_CPUINFO_X86_STEPPING_OFFSET)
            .write(signature.stepping);
        ptr::write_unaligned(
            cpuinfo
                .add(LINUX_CPUINFO_X86_CPUID_LEVEL_OFFSET)
                .cast::<i32>(),
            max_basic as i32,
        );
        let cap_ptr = cpuinfo.add(LINUX_CPUINFO_X86_CAPABILITY_OFFSET) as *mut u32;
        for (index, value) in caps.iter().enumerate() {
            ptr::write_unaligned(cap_ptr.add(index), *value);
        }
    }
}

fn ensure_linux_boot_cpu_data() {
    if LINUX_BOOT_CPU_DATA_INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }
    write_linux_cpuinfo_x86(core::ptr::addr_of_mut!(LINUX_BOOT_CPU_DATA).cast::<u8>());
}

/// `struct cpumask` - `vendor/linux/include/linux/cpumask_types.h`.
#[repr(C)]
pub struct LinuxCpuMask {
    pub bits: [usize; 1],
}

static LINUX_CPU_POSSIBLE_MASK: LinuxCpuMask = LinuxCpuMask { bits: [1] };

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    super::cpu_match::register_module_exports();
    super::hypervisor::register_module_exports();
    ensure_linux_boot_cpu_data();
    export_symbol_once(
        "boot_cpu_data",
        core::ptr::addr_of_mut!(LINUX_BOOT_CPU_DATA) as usize,
        false,
    );
    export_symbol_once(
        "cpu_info",
        crate::arch::x86::kernel::setup_percpu::cpu_info_symbol(),
        false,
    );
    export_symbol_once(
        "__max_dies_per_package",
        core::ptr::addr_of!(LINUX_MAX_DIES_PER_PACKAGE) as usize,
        false,
    );
    export_symbol_once(
        "__max_logical_packages",
        core::ptr::addr_of!(LINUX_MAX_LOGICAL_PACKAGES) as usize,
        false,
    );
    export_symbol_once(
        "__preempt_count",
        crate::arch::x86::kernel::setup_percpu::preempt_count_symbol(),
        true,
    );
    export_symbol_once(
        "const_current_task",
        core::ptr::addr_of!(LINUX_CONST_CURRENT_TASK) as usize,
        true,
    );
    export_symbol_once(
        "__cpu_possible_mask",
        core::ptr::addr_of!(LINUX_CPU_POSSIBLE_MASK) as usize,
        false,
    );
    export_symbol_once(
        "cpu_number",
        crate::arch::x86::kernel::setup_percpu::cpu_number_symbol(),
        false,
    );
    export_symbol_once(
        "this_cpu_off",
        crate::arch::x86::kernel::setup_percpu::this_cpu_off_symbol(),
        false,
    );
}

pub fn set_linux_current_task(task: *mut crate::kernel::task::TaskStruct) {
    #[cfg(not(test))]
    if crate::kernel::sched::current_cpu() != 0 {
        return;
    }
    LINUX_CONST_CURRENT_TASK.store(task as usize, Ordering::Release);
}

pub fn linux_current_task() -> *mut crate::kernel::task::TaskStruct {
    LINUX_CONST_CURRENT_TASK.load(Ordering::Acquire) as *mut crate::kernel::task::TaskStruct
}

#[cfg(test)]
pub fn linux_current_task_for_tests() -> *mut crate::kernel::task::TaskStruct {
    linux_current_task()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_cpu_common_exports_register_for_modules() {
        register_module_exports();
        assert!(crate::kernel::module::find_symbol("__preempt_count").is_some());
        assert!(crate::kernel::module::find_symbol("const_current_task").is_some());
        assert!(crate::kernel::module::find_symbol("x86_match_cpu").is_some());
        assert!(crate::kernel::module::find_symbol("x86_hyper_type").is_some());
        assert!(crate::kernel::module::find_symbol("cpu_info").is_some());
        assert!(crate::kernel::module::find_symbol("__max_dies_per_package").is_some());
        assert!(crate::kernel::module::find_symbol("__max_logical_packages").is_some());
        assert_eq!(
            crate::kernel::module::find_symbol("__cpu_possible_mask"),
            Some(core::ptr::addr_of!(LINUX_CPU_POSSIBLE_MASK) as usize)
        );
        assert_eq!(LINUX_CPU_POSSIBLE_MASK.bits[0], 1);
    }

    #[test]
    fn linux_cpuinfo_x86_carries_capability_words() {
        let mut cpuinfo = [0u8; LINUX_CPUINFO_X86_SIZE];
        write_linux_cpuinfo_x86(cpuinfo.as_mut_ptr());
        let cap_ptr =
            unsafe { cpuinfo.as_ptr().add(LINUX_CPUINFO_X86_CAPABILITY_OFFSET) as *const u32 };
        let always_word = unsafe {
            cap_ptr
                .add((X86_FEATURE_ALWAYS / 32) as usize)
                .read_unaligned()
        };

        assert_ne!(always_word & (1 << (X86_FEATURE_ALWAYS % 32)), 0);
    }

    #[test]
    fn linux_current_task_export_tracks_pointer_value() {
        let task = 0x12345000usize as *mut crate::kernel::task::TaskStruct;

        set_linux_current_task(task);

        assert_eq!(linux_current_task_for_tests(), task);
        assert_eq!(
            LINUX_CONST_CURRENT_TASK.load(Ordering::Acquire),
            task as usize
        );
    }
}
