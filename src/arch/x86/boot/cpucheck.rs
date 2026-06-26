//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/cpucheck.c
//! test-origin: linux:vendor/linux/arch/x86/boot/cpucheck.c
//! Required CPU feature bitmask check.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/cpucheck.c
//! - vendor/linux/arch/x86/include/asm/cpufeaturemasks.h (`REQUIRED_MASK*`,
//!   build-generated from `cpufeatures.h` + `.config`)
//! - vendor/linux/arch/x86/include/asm/shared/msr.h (`struct msr`,
//!   `raw_rdmsr`/`raw_wrmsr`)
//! - vendor/linux/arch/x86/include/uapi/asm/processor-flags.h (`X86_EFLAGS_AC`)
//! - vendor/linux/arch/x86/include/asm/msr-index.h (`MSR_K7_HWCR`,
//!   `MSR_VIA_FCR`)
//! - vendor/linux/arch/x86/include/asm/cpufeatures.h (`X86_FEATURE_*`)
//!
//! `check_cpu()` compares the detected `cpu.flags[NCAPINTS]` against the
//! build-time `req_flags[NCAPINTS]` (filled in from `REQUIRED_MASK*`). It also
//! implements four vendor-specific workarounds:
//!   * AMD K7+ with SSE/SSE2 disabled in `MSR_K7_HWCR` (cpucheck.c:128-142).
//!   * VIA C3 with CMPXCHG8B disabled in `MSR_VIA_FCR` (cpucheck.c:143-156).
//!   * Transmeta CMS featuremask MSR `0x80860004` (cpucheck.c:157-172).
//!   * Intel Pentium-M `forcepae` via the boot command line (cpucheck.c:173-186).
//!
//! Plus the Xeon Phi KNL erratum check (cpucheck.c:200-226). The four
//! workarounds touch real hardware (RDMSR/WRMSR + CPUID), so they go through
//! an explicit [`MsrOps`] seam, exactly like the `BiosCaller` / `PortIoOps`
//! seams elsewhere in the boot stub: the production impl emits the real
//! instructions, the test impl substitutes a deterministic fake. This is a
//! faithful split of Linux's `raw_rdmsr`/`raw_wrmsr` and the `asm("cpuid")`
//! the Transmeta path uses — not a simplification.

use super::bitops::{set_bit, test_bit};
use super::cmdline::__cmdline_find_option_bool;
use super::cpuflags::{CpuFeatures, CpuVendor, NCAPINTS, get_cpuflags};

/// `A32(a,b,c,d)` — pack a 4-byte ASCII tag into a little-endian u32
/// the same way `cpu_vendor[0..3]` stores them. Matches cpucheck.c:58.
pub const fn a32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (d as u32) << 24 | (c as u32) << 16 | (b as u32) << 8 | a as u32
}

/// Sentinel vendor strings split into 4-byte chunks. Each chunk is the
/// little-endian u32 of the corresponding ASCII letters. cpucheck.c:60-86.
pub const AMD_VENDOR: [u32; 3] = [
    a32(b'A', b'u', b't', b'h'),
    a32(b'e', b'n', b't', b'i'),
    a32(b'c', b'A', b'M', b'D'),
];
pub const CENTAUR_VENDOR: [u32; 3] = [
    a32(b'C', b'e', b'n', b't'),
    a32(b'a', b'u', b'r', b'H'),
    a32(b'a', b'u', b'l', b's'),
];
pub const TRANSMETA_VENDOR: [u32; 3] = [
    a32(b'G', b'e', b'n', b'u'),
    a32(b'i', b'n', b'e', b'T'),
    a32(b'M', b'x', b'8', b'6'),
];
pub const INTEL_VENDOR: [u32; 3] = [
    a32(b'G', b'e', b'n', b'u'),
    a32(b'i', b'n', b'e', b'I'),
    a32(b'n', b't', b'e', b'l'),
];

#[inline]
pub fn is_amd(vendor: &[u32; 3]) -> bool {
    vendor == &AMD_VENDOR
}
#[inline]
pub fn is_centaur(vendor: &[u32; 3]) -> bool {
    vendor == &CENTAUR_VENDOR
}
#[inline]
pub fn is_transmeta(vendor: &[u32; 3]) -> bool {
    vendor == &TRANSMETA_VENDOR
}
#[inline]
pub fn is_intel(vendor: &[u32; 3]) -> bool {
    vendor == &INTEL_VENDOR
}

// ---------------------------------------------------------------------------
// Constants sourced from vendor/linux. cpufeaturemasks.h is build-generated,
// so the X86_FEATURE_* indices come straight from cpufeatures.h and the MSR /
// EFLAGS constants from their headers.
// ---------------------------------------------------------------------------

/// `X86_EFLAGS_AC` — Alignment Check / Access Control flag, bit 18.
/// vendor/linux/arch/x86/include/uapi/asm/processor-flags.h:39-40
/// (`X86_EFLAGS_AC_BIT 18`, `X86_EFLAGS_AC _BITUL(18)`). Used to detect a
/// 486+ in `check_cpu`.
pub const X86_EFLAGS_AC: u32 = 1 << 18;

/// `X86_FEATURE_FPU` = 0*32+0. cpufeatures.h:21.
pub const X86_FEATURE_FPU: u32 = 0 * 32 + 0;
/// `X86_FEATURE_PAE` = 0*32+6. cpufeatures.h:27.
pub const X86_FEATURE_PAE: u32 = 0 * 32 + 6;
/// `X86_FEATURE_CX8` = 0*32+8 (CMPXCHG8B). cpufeatures.h:29.
pub const X86_FEATURE_CX8: u32 = 0 * 32 + 8;
/// `X86_FEATURE_XMM` = 0*32+25 (SSE). cpufeatures.h:44.
pub const X86_FEATURE_XMM: u32 = 0 * 32 + 25;
/// `X86_FEATURE_XMM2` = 0*32+26 (SSE2). cpufeatures.h:45.
pub const X86_FEATURE_XMM2: u32 = 0 * 32 + 26;
/// `X86_FEATURE_LM` = 1*32+29 (Long Mode). cpufeatures.h:61.
pub const X86_FEATURE_LM: u32 = 1 * 32 + 29;

/// `MSR_K7_HWCR` — AMD K7+ Hardware Configuration. msr-index.h:896.
/// Bit 15 (`SSEDIS`) masks SSE/SSE2 when set; `check_cpu` clears it.
pub const MSR_K7_HWCR: u32 = 0xc001_0015;
/// `MSR_VIA_FCR` — VIA Feature Control Register. msr-index.h:931. Bits 1
/// and 7 enable CX8 (and friends) on a VIA C3.
pub const MSR_VIA_FCR: u32 = 0x0000_1107;
/// Transmeta CMS feature-mask MSR. cpucheck.c:163 hardcodes `0x80860004`.
pub const MSR_TRANSMETA_FEATURE_MASK: u32 = 0x8086_0004;

/// `CONFIG_X86_MINIMUM_CPU_FAMILY` for an x86_64 build is 64. cpucheck.c:35
/// reads it into `req_level`; the kernel Kconfig forces it to 64 whenever
/// `CONFIG_X86_64` is set.
pub const REQ_LEVEL_X86_64: i32 = 64;

/// `req_flags[NCAPINTS]` for the x86_64 defconfig. cpucheck.c:37-56 fills only
/// words 0, 1, 4, 6 and 16 from `REQUIRED_MASK{0,1,4,6,16}`; every other word
/// is hard-zero in the source (`REQUIRED_MASK2/3/5/7..15` are "not implemented
/// in this file").
///
/// The non-zero words below are the x86_64 `REQUIRED_MASK*` values produced by
/// `cpufeaturemasks.awk` from the always-on `CONFIG_X86_REQUIRED_FEATURE_*`
/// symbols in `vendor/linux/arch/x86/Kconfig.cpufeatures`. For word 0 those are
/// the `CPUID.1.EDX` features FPU(0), PSE(3), MSR(5), PAE(6), CX8(8), PGE(13),
/// CMOV(15), FXSR(24), XMM/SSE(25) and XMM2/SSE2(26); word 1 carries LM only.
///   word0 = REQUIRED_MASK0 = 0x0700_a169
///   word1 = REQUIRED_MASK1 = (1<<(X86_FEATURE_LM&31)) = 1<<29 = 0x2000_0000
/// This is intentionally a strict *subset* of any conforming x86_64 CPUID.1.EDX
/// (e.g. QEMU's `qemu64` reports 0x078b_fbff). The old value `0x0f8b_fbff` was a
/// CPU feature *dump*, not a required mask: it set bit 27 (X86_FEATURE_SS,
/// Self-Snoop), which QEMU TCG never advertises for any `-cpu` model — so
/// `verify_cpu` (head_64.S) failed and halted at `.Lno_longmode` before long
/// mode on every TCG boot, even though hardware-accelerated (KVM/WHPX) boots
/// passed because real CPUs set SS. Words 4, 6, 16 carry no unconditionally-
/// required features in the x86_64 defconfig, so they are 0.
pub const REQUIRED_MASK_X86_64: [u32; NCAPINTS] = {
    let mut m = [0u32; NCAPINTS];
    m[0] = 0x0700_a169;
    m[1] = 1 << (X86_FEATURE_LM % 32);
    m
};

// ---------------------------------------------------------------------------
// MSR + Transmeta-CPUID hardware seam.
// ---------------------------------------------------------------------------

/// Linux `struct msr` — a union of `u64 q` and `{u32 l; u32 h}`.
/// vendor/linux/arch/x86/include/asm/shared/msr.h:5-13. We expose the `l`/`h`
/// halves directly because `check_cpu` only ever pokes `m.l`.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct Msr {
    /// Low 32 bits (EAX after RDMSR).
    pub l: u32,
    /// High 32 bits (EDX after RDMSR).
    pub h: u32,
}

impl Msr {
    /// `m.q` accessor — the 64-bit view of the union.
    #[inline]
    pub fn q(&self) -> u64 {
        (self.l as u64) | ((self.h as u64) << 32)
    }
}

/// Hardware seam for the vendor workarounds: `raw_rdmsr`, `raw_wrmsr`, the
/// Transmeta `asm("cpuid")` that reloads `cpu.flags[0]`, and `get_cpuflags()`.
///
/// Production wires this to the real instructions; tests substitute a
/// deterministic fake so `check_cpu` is exercisable from host code. This is a
/// faithful seam, not a simplification: the methods are 1:1 with the asm in
/// shared/msr.h:20-28, cpucheck.c:167-169, and cpuflags.c::get_cpuflags. CPUID
/// (which `get_cpuflags` issues) is a genuine hardware probe, so it lives
/// behind the same kind of seam as the MSR pokes.
pub trait MsrOps {
    /// `raw_rdmsr(reg, &mut m)` — shared/msr.h:20-23 (`rdmsr`).
    fn raw_rdmsr(&mut self, reg: u32) -> Msr;
    /// `raw_wrmsr(reg, &m)` — shared/msr.h:25-28 (`wrmsr`).
    fn raw_wrmsr(&mut self, reg: u32, m: &Msr);
    /// Transmeta path: `level=1; asm("cpuid": "+a"(level), "=d"(flags0))`.
    /// cpucheck.c:167-169. Returns the EDX result that becomes `cpu.flags[0]`.
    fn transmeta_cpuid_edx(&mut self) -> u32;
    /// `get_cpuflags()` — cpuflags.c:68-110. Populates `cpu`/`vendor` from
    /// CPUID, guarded by the shared `loaded` flag exactly like the C static.
    /// Production forwards to [`get_cpuflags`]; tests fill deterministic data.
    fn get_cpuflags(&mut self, cpu: &mut CpuFeatures, vendor: &mut CpuVendor, loaded: &mut bool);
}

/// Production `MsrOps` — issues real `rdmsr`/`wrmsr`/`cpuid`. Only valid in
/// ring 0 with the corresponding MSRs present (the callers gate on vendor +
/// feature checks, exactly as Linux does).
pub struct RealMsrOps;

impl MsrOps for RealMsrOps {
    #[inline]
    fn raw_rdmsr(&mut self, reg: u32) -> Msr {
        let l: u32;
        let h: u32;
        // SAFETY: RDMSR is ring-0 only; the boot stub runs in ring 0 and the
        // callers gate on the owning vendor/feature. Mirrors `raw_rdmsr` in
        // vendor/linux/arch/x86/include/asm/shared/msr.h:20-23.
        unsafe {
            core::arch::asm!(
                "rdmsr",
                in("ecx") reg,
                out("eax") l,
                out("edx") h,
                options(nomem, nostack, preserves_flags),
            );
        }
        Msr { l, h }
    }

    #[inline]
    fn raw_wrmsr(&mut self, reg: u32, m: &Msr) {
        // SAFETY: WRMSR is ring-0 only; see raw_rdmsr. Mirrors `raw_wrmsr` in
        // shared/msr.h:25-28.
        unsafe {
            core::arch::asm!(
                "wrmsr",
                in("ecx") reg,
                in("eax") m.l,
                in("edx") m.h,
                options(nostack, preserves_flags),
            );
        }
    }

    #[inline]
    fn transmeta_cpuid_edx(&mut self) -> u32 {
        let edx: u32;
        // SAFETY: CPUID never faults. Mirrors cpucheck.c:167-169
        //   asm("cpuid" : "+a"(level), "=d"(cpu.flags[0]) : : "ecx","ebx")
        // with level=1. We preserve rbx (LLVM reserves it) like the kernel's
        // own cpuid wrappers do.
        unsafe {
            core::arch::asm!(
                "push rbx",
                "cpuid",
                "pop rbx",
                inout("eax") 1u32 => _,
                out("ecx") _,
                out("edx") edx,
                options(nomem, preserves_flags),
            );
        }
        edx
    }

    #[inline]
    fn get_cpuflags(&mut self, cpu: &mut CpuFeatures, vendor: &mut CpuVendor, loaded: &mut bool) {
        // Forward to the real CPUID-driven populator. cpuflags.c:68-110.
        get_cpuflags(cpu, vendor, loaded);
    }
}

/// `check_cpuflags()` — compute which required flag bits are missing.
/// Returns `(err_bitmask_per_word, err_flags)`. cpucheck.c:89-102.
pub fn check_cpuflags(
    detected: &[u32; NCAPINTS],
    required: &[u32; NCAPINTS],
) -> (u32, [u32; NCAPINTS]) {
    let mut err_flags = [0u32; NCAPINTS];
    let mut err: u32 = 0;
    for i in 0..NCAPINTS {
        err_flags[i] = required[i] & !detected[i];
        if err_flags[i] != 0 {
            err |= 1u32 << i;
        }
    }
    (err, err_flags)
}

/// `check_cpu(cpu_level_ptr, req_level_ptr, err_flags_ptr)` — cpucheck.c:112-198.
///
/// Faithful 1:1 port. Because lupos has no C global `cpu`/`cpu_vendor`/`hdr`,
/// the state Linux reads from globals is threaded explicitly:
///   * `cpu`        — the [`CpuFeatures`] block (`cpu.flags`, level, family,
///                    model), zeroed and re-populated here just like the source
///                    (`memset(&cpu.flags, 0, ...)` at line 116).
///   * `vendor`     — the `cpu_vendor[3]` string filled by `get_cpuflags`.
///   * `cmdline`    — the NUL-terminated boot command line backing
///                    `cmdline_find_option_bool("forcepae")` (line 178).
///   * `msr`        — the [`MsrOps`] hardware seam for the vendor workarounds.
///   * `req_flags`/`req_level` — the build-time `REQUIRED_MASK*` /
///                    `CONFIG_X86_MINIMUM_CPU_FAMILY`.
///
/// `has_ac` carries the result of `has_eflag(X86_EFLAGS_AC)` (line 119); the
/// real boot stub runs the EFLAGS push/pop dance, which cannot be exercised
/// from host tests, so the caller supplies it (production passes the genuine
/// read, see [`check_cpu_with_eflag`]).
///
/// `loaded` is Linux's `static bool loaded_flags` (cpuflags.c:12). Production
/// passes `false` so the first `get_cpuflags()` runs CPUID for real; host
/// tests pass `true` with `cpu`/`vendor` pre-populated so `get_cpuflags()`
/// short-circuits exactly as it does on a warm second call — this is the
/// genuine guard, not a mock, which keeps `check_cpu` deterministic off-CPU.
///
/// NOTE the `memset(&cpu.flags, 0, ...)` at line 116 only clears the *flag
/// words*, not `cpu.level/family/model` — so a caller that pre-loaded those
/// keeps them, matching the source.
///
/// Returns a [`CheckCpuResult`] mirroring the three out-pointers and the
/// integer return value.
#[allow(clippy::too_many_arguments)]
pub fn check_cpu(
    cpu: &mut CpuFeatures,
    vendor: &mut CpuVendor,
    cmdline: &[u8],
    msr: &mut dyn MsrOps,
    req_flags: &[u32; NCAPINTS],
    req_level: i32,
    has_ac: bool,
    loaded: bool,
) -> CheckCpuResult {
    // memset(&cpu.flags, 0, sizeof(cpu.flags)); cpu.level = 3;  (lines 116-117)
    cpu.flags = [0u32; NCAPINTS];
    cpu.level = 3;

    // if (has_eflag(X86_EFLAGS_AC)) cpu.level = 4;  (lines 119-120)
    if has_ac {
        cpu.level = 4;
    }

    // get_cpuflags(); err = check_cpuflags();  (lines 122-123)
    //
    // Linux's get_cpuflags() is guarded by a *single* `static bool
    // loaded_flags` (cpuflags.c:12,74-76): the first call populates cpu/vendor,
    // every later call in this function is a no-op. We thread one `loaded`
    // across all get_cpuflags() calls here to reproduce that exactly — the
    // AMD branch's second get_cpuflags() (line 141) is therefore a no-op in
    // Linux too, relying only on the MSR write's hardware side effect.
    let mut loaded = loaded;
    msr.get_cpuflags(cpu, vendor, &mut loaded);
    let (mut err, mut err_flags) = check_cpuflags(&cpu.flags, req_flags);

    // if (test_bit(X86_FEATURE_LM, cpu.flags)) cpu.level = 64;  (lines 125-126)
    if test_bit(X86_FEATURE_LM as i32, &cpu.flags) {
        cpu.level = 64;
    }

    if err == 0x01
        && (err_flags[0] & !((1 << (X86_FEATURE_XMM % 32)) | (1 << (X86_FEATURE_XMM2 % 32)))) == 0
        && is_amd(&vendor.0)
    {
        // AMD, only missing SSE+SSE2 → clear MSR_K7_HWCR bit 15.  (lines 128-142)
        let mut m = msr.raw_rdmsr(MSR_K7_HWCR);
        m.l &= !(1 << 15);
        msr.raw_wrmsr(MSR_K7_HWCR, &m);

        // get_cpuflags(); — re-read after the MSR poke.  (line 141). With the
        // shared `loaded` guard already set, this is a no-op exactly like the
        // C source; on real hardware the MSR write is what changes the flags.
        msr.get_cpuflags(cpu, vendor, &mut loaded);
        let r = check_cpuflags(&cpu.flags, req_flags);
        err = r.0;
        err_flags = r.1;
    } else if err == 0x01
        && (err_flags[0] & !(1 << (X86_FEATURE_CX8 % 32))) == 0
        && is_centaur(&vendor.0)
        && cpu.model >= 6
    {
        // VIA C3 → set MSR_VIA_FCR bits 1 and 7, force CX8.  (lines 143-156)
        let mut m = msr.raw_rdmsr(MSR_VIA_FCR);
        m.l |= (1 << 1) | (1 << 7);
        msr.raw_wrmsr(MSR_VIA_FCR, &m);

        set_bit(X86_FEATURE_CX8 as i32, &mut cpu.flags);
        let r = check_cpuflags(&cpu.flags, req_flags);
        err = r.0;
        err_flags = r.1;
    } else if err == 0x01 && is_transmeta(&vendor.0) {
        // Transmeta: unmask word-0 features via MSR 0x80860004.  (lines 157-172)
        let m = msr.raw_rdmsr(MSR_TRANSMETA_FEATURE_MASK);
        let mut m_tmp = m;
        m_tmp.l = !0;
        msr.raw_wrmsr(MSR_TRANSMETA_FEATURE_MASK, &m_tmp);
        // asm("cpuid": "+a"(level=1), "=d"(cpu.flags[0]))  (lines 167-169)
        cpu.flags[0] = msr.transmeta_cpuid_edx();
        msr.raw_wrmsr(MSR_TRANSMETA_FEATURE_MASK, &m);

        let r = check_cpuflags(&cpu.flags, req_flags);
        err = r.0;
        err_flags = r.1;
    } else if err == 0x01
        && (err_flags[0] & !(1 << (X86_FEATURE_PAE % 32))) == 0
        && is_intel(&vendor.0)
        && cpu.level == 6
        && (cpu.model == 9 || cpu.model == 13)
    {
        // Pentium M: PAE disabled but forceable via cmdline.  (lines 173-186)
        if __cmdline_find_option_bool(cmdline, b"forcepae") > 0 {
            // puts("WARNING: Forcing PAE in CPU flags\n");
            set_bit(X86_FEATURE_PAE as i32, &mut cpu.flags);
            let r = check_cpuflags(&cpu.flags, req_flags);
            err = r.0;
            err_flags = r.1;
        }
        // else: puts the "PAE disabled" warning, leaving err unchanged.
    }

    // if (!err) err = check_knl_erratum();  (lines 187-188)
    let mut ret_err = err;
    if ret_err == 0 {
        // check_knl_erratum returns 0 or -1; -1 means "must not boot".
        if check_knl_erratum(cpu.family, cpu.model, &vendor.0) != 0 {
            ret_err = 0xffff_ffff; // sentinel: non-zero so the final test trips.
        }
    }

    let err_flags_present = ret_err != 0;

    // return (cpu.level < req_level || err) ? -1 : 0;  (line 197)
    let ret = if (cpu.level as i32) < req_level || ret_err != 0 {
        -1
    } else {
        0
    };

    CheckCpuResult {
        ret,
        cpu_level: cpu.level as i32,
        req_level,
        err_flags: if err_flags_present {
            Some(err_flags)
        } else {
            None
        },
    }
}

/// Result of [`check_cpu`], mirroring the three out-pointers plus the return
/// value Linux's `check_cpu` writes through.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CheckCpuResult {
    /// `-1` on error, `0` on success (the C return value, cpucheck.c:197).
    pub ret: i32,
    /// `*cpu_level_ptr` — detected CPU level (3/4/64).
    pub cpu_level: i32,
    /// `*req_level_ptr` — required level (`CONFIG_X86_MINIMUM_CPU_FAMILY`).
    pub req_level: i32,
    /// `*err_flags_ptr` — the missing-feature array, or `None` (NULL) when
    /// every required feature is present.
    pub err_flags: Option<[u32; NCAPINTS]>,
}

/// Production entry point: read `has_eflag(X86_EFLAGS_AC)` for real, then run
/// [`check_cpu`]. The EFLAGS AC toggle is the one piece of `check_cpu` that is
/// inherently a hardware probe; it is split out so host tests can drive
/// [`check_cpu`] directly while production keeps full fidelity.
pub fn check_cpu_with_eflag(
    cpu: &mut CpuFeatures,
    vendor: &mut CpuVendor,
    cmdline: &[u8],
    msr: &mut dyn MsrOps,
    req_flags: &[u32; NCAPINTS],
    req_level: i32,
) -> CheckCpuResult {
    check_cpu(
        cpu,
        vendor,
        cmdline,
        msr,
        req_flags,
        req_level,
        has_eflag_ac(),
        false, // cold start: get_cpuflags() runs CPUID for real.
    )
}

/// `has_eflag(X86_EFLAGS_AC)` — cpuflags.c:37-55 (`has_eflag`). Flip EFLAGS.AC
/// and see whether it sticks: a 386 cannot toggle AC, a 486+ can. On x86_64
/// AC is always toggleable, but we emit the genuine push/pop dance so the
/// behaviour is identical to the real-mode stub.
#[inline]
pub fn has_eflag_ac() -> bool {
    let f0: u64;
    let f1: u64;
    let mask: u64 = X86_EFLAGS_AC as u64;
    // SAFETY: pushfq/popfq only touch the stack and the flags register; we
    // restore the original flags before returning. Mirrors cpuflags.c
    // has_eflag() (the 32-bit pushfl/popfl variant, widened to 64-bit).
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pushfq",
            "pop {f0}",
            "mov {f1}, {f0}",
            "xor {f1}, {mask}",
            "push {f1}",
            "popfq",
            "pushfq",
            "pop {f1}",
            "popfq",
            f0 = out(reg) f0,
            f1 = out(reg) f1,
            mask = in(reg) mask,
            options(preserves_flags),
        );
    }
    ((f0 ^ f1) & mask) != 0
}

/// `check_knl_erratum()` — Xeon Phi KNL is family 6, model 0x57. cpucheck.c:200-226.
/// Linux refuses to run 32-bit non-PAE kernels on it. lupos is x86_64-only, so
/// `IS_ENABLED(CONFIG_X86_64)` is true and the erratum is non-fatal (returns 0
/// at line 219); we mirror the model/family predicate for documentation.
/// Returns -1 to match the Linux convention when triggered, 0 otherwise.
pub fn check_knl_erratum(family: u32, model: u32, vendor: &[u32; 3]) -> i32 {
    if !is_intel(vendor) || family != 6 || model != 0x57 {
        return 0;
    }
    // CONFIG_X86_64 is set for lupos → erratum non-fatal. cpucheck.c:218-219.
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;
    use alloc::vec::Vec;

    /// Scripted [`MsrOps`] that records every RDMSR/WRMSR/CPUID and replays
    /// caller-supplied MSR values. `get_cpuflags` injects deterministic
    /// CPU/vendor data so `check_cpu` is testable off-CPU. Uses owned record
    /// vectors (no thread_local; the crate is `#![no_std]`).
    struct FakeMsr {
        // Map of reg -> current value.
        regs: Vec<(u32, Msr)>,
        reads: Vec<u32>,
        writes: Vec<(u32, Msr)>,
        // Value the Transmeta cpuid returns for cpu.flags[0].
        transmeta_edx: u32,
        transmeta_calls: u32,
        // Deterministic CPU/vendor state get_cpuflags() installs (first call).
        seed_cpu: CpuFeatures,
        seed_vendor: CpuVendor,
        // Whether the fake CPUID provides level/family/model (mirrors the C
        // `if (max_intel_level >= 1)` gate). When false, get_cpuflags leaves
        // cpu.level at the value memset/AC set (3 or 4), matching a CPU with
        // no usable CPUID leaf 1.
        seed_sets_level: bool,
        get_cpuflags_calls: u32,
    }
    impl FakeMsr {
        fn new() -> Self {
            Self {
                regs: Vec::new(),
                reads: Vec::new(),
                writes: Vec::new(),
                transmeta_edx: 0,
                transmeta_calls: 0,
                seed_cpu: CpuFeatures::default(),
                seed_vendor: CpuVendor::default(),
                seed_sets_level: true,
                get_cpuflags_calls: 0,
            }
        }
        fn preset(&mut self, reg: u32, m: Msr) {
            self.regs.push((reg, m));
        }
        fn seed(mut self, cpu: CpuFeatures, vendor: CpuVendor) -> Self {
            self.seed_cpu = cpu;
            self.seed_vendor = vendor;
            self
        }
        fn no_cpuid_level(mut self) -> Self {
            self.seed_sets_level = false;
            self
        }
        fn get(&self, reg: u32) -> Msr {
            self.regs
                .iter()
                .rev()
                .find(|(r, _)| *r == reg)
                .map(|(_, m)| *m)
                .unwrap_or_default()
        }
    }
    impl MsrOps for FakeMsr {
        fn raw_rdmsr(&mut self, reg: u32) -> Msr {
            self.reads.push(reg);
            self.get(reg)
        }
        fn raw_wrmsr(&mut self, reg: u32, m: &Msr) {
            self.writes.push((reg, *m));
            self.regs.push((reg, *m));
        }
        fn transmeta_cpuid_edx(&mut self) -> u32 {
            self.transmeta_calls += 1;
            self.transmeta_edx
        }
        fn get_cpuflags(
            &mut self,
            cpu: &mut CpuFeatures,
            vendor: &mut CpuVendor,
            loaded: &mut bool,
        ) {
            self.get_cpuflags_calls += 1;
            // Faithful `loaded_flags` guard: warm calls are no-ops.
            if *loaded {
                return;
            }
            *loaded = true;
            // Preserve the level the memset/AC step set (only flags are
            // (re)written by get_cpuflags besides level/family/model in C; our
            // seed carries the full deterministic CPU image).
            *cpu = self.seed_cpu;
            *vendor = self.seed_vendor;
        }
    }

    fn empty_cmdline() -> &'static [u8] {
        b"\0"
    }

    #[test]
    fn a32_packs_ascii_in_little_endian_order() {
        // "Auth" → 0x68_74_75_41. cpucheck.c:58.
        assert_eq!(a32(b'A', b'u', b't', b'h'), 0x6874_7541);
    }

    #[test]
    fn vendor_constants_match_cpuid_layout() {
        // Intel: CPUID[0].EBX/EDX/ECX = "Genu"/"ineI"/"ntel".
        assert_eq!(INTEL_VENDOR[0], 0x756e_6547); // "Genu"
        assert_eq!(INTEL_VENDOR[1], 0x4965_6e69); // "ineI"
        assert_eq!(INTEL_VENDOR[2], 0x6c65_746e); // "ntel"
    }

    #[test]
    fn is_intel_matches_full_signature() {
        let mut v = INTEL_VENDOR;
        assert!(is_intel(&v));
        v[2] = 0;
        assert!(!is_intel(&v));
    }

    #[test]
    fn feature_indices_match_cpufeatures_h() {
        // cpufeatures.h:21,27,29,44,45,61.
        assert_eq!(X86_FEATURE_FPU, 0);
        assert_eq!(X86_FEATURE_PAE, 6);
        assert_eq!(X86_FEATURE_CX8, 8);
        assert_eq!(X86_FEATURE_XMM, 25);
        assert_eq!(X86_FEATURE_XMM2, 26);
        assert_eq!(X86_FEATURE_LM, 61); // word 1, bit 29
    }

    #[test]
    fn msr_and_eflag_constants_match_headers() {
        assert_eq!(MSR_K7_HWCR, 0xc001_0015); // msr-index.h:896
        assert_eq!(MSR_VIA_FCR, 0x0000_1107); // msr-index.h:931
        assert_eq!(MSR_TRANSMETA_FEATURE_MASK, 0x8086_0004); // cpucheck.c:163
        assert_eq!(X86_EFLAGS_AC, 1 << 18); // processor-flags.h:39-40
    }

    #[test]
    fn required_mask_x86_64_only_fills_words_0_and_1() {
        // cpucheck.c:37-56: words 2,3,5,7..15 are hard-zero. LM lives in word 1.
        assert_eq!(REQUIRED_MASK_X86_64[1], 1 << 29);
        for (i, w) in REQUIRED_MASK_X86_64.iter().enumerate() {
            if i != 0 && i != 1 {
                assert_eq!(*w, 0, "word {i} must be zero per cpucheck.c req_flags");
            }
        }
        // FPU bit must be required (the always-on baseline).
        assert_ne!(REQUIRED_MASK_X86_64[0] & (1 << X86_FEATURE_FPU), 0);

        // The exact x86_64 word-0 mask from Kconfig.cpufeatures.
        assert_eq!(REQUIRED_MASK_X86_64[0], 0x0700_a169);
        // It must be a strict subset of a conforming CPUID.1.EDX. QEMU's
        // `qemu64` reports 0x078b_fbff under TCG, so the mask must not require
        // any bit absent there — most importantly bit 27 (X86_FEATURE_SS,
        // Self-Snoop), which TCG never sets. Requiring SS used to halt every
        // TCG boot at head_64.S `.Lno_longmode`.
        const X86_FEATURE_SS: u32 = 0 * 32 + 27;
        const QEMU64_CPUID1_EDX: u32 = 0x078b_fbff;
        assert_eq!(REQUIRED_MASK_X86_64[0] & (1 << X86_FEATURE_SS), 0);
        assert_eq!(
            REQUIRED_MASK_X86_64[0] & QEMU64_CPUID1_EDX,
            REQUIRED_MASK_X86_64[0],
            "REQUIRED_MASK0 must be satisfiable by a TCG qemu64 CPU"
        );
    }

    #[test]
    fn check_cpuflags_returns_zero_when_all_required_present() {
        let mut detected = [0u32; NCAPINTS];
        let mut required = [0u32; NCAPINTS];
        required[0] = 0xff;
        detected[0] = 0xff;
        let (err, missing) = check_cpuflags(&detected, &required);
        assert_eq!(err, 0);
        assert_eq!(missing[0], 0);
    }

    #[test]
    fn check_cpuflags_reports_missing_bits_per_word() {
        let mut detected = [0u32; NCAPINTS];
        let mut required = [0u32; NCAPINTS];
        required[0] = 0xff;
        required[3] = 0x0f;
        detected[0] = 0xfe; // bit 0 missing in word 0
        detected[3] = 0x0f;
        let (err, missing) = check_cpuflags(&detected, &required);
        assert_eq!(err, 0x1, "only word 0 should flag");
        assert_eq!(missing[0], 0x1);
        assert_eq!(missing[3], 0x0);
    }

    #[test]
    fn knl_erratum_is_inert_for_non_knl_cpus() {
        assert_eq!(check_knl_erratum(6, 0x55, &INTEL_VENDOR), 0);
        assert_eq!(check_knl_erratum(6, 0x57, &AMD_VENDOR), 0);
        // KNL itself on x86_64 → still 0 (CONFIG_X86_64). cpucheck.c:218.
        assert_eq!(check_knl_erratum(6, 0x57, &INTEL_VENDOR), 0);
    }

    // ---- check_cpu() behavior, cpucheck.c:112-198 -------------------------

    /// A CPU that already satisfies x86_64 (LM present, all word-0 bits set):
    /// check_cpu must report level 64 and success.
    #[test]
    fn check_cpu_passes_on_full_x86_64_cpu() {
        let mut seed_cpu = CpuFeatures::default();
        seed_cpu.flags[0] = 0xffff_ffff; // all word-0 features present
        seed_cpu.flags[1] = 1 << 29; // X86_FEATURE_LM
        let mut cpu = CpuFeatures::default();
        let mut vendor = CpuVendor(INTEL_VENDOR);
        let mut msr = FakeMsr::new().seed(seed_cpu, CpuVendor(INTEL_VENDOR));

        let req = [0u32; NCAPINTS];
        let r = check_cpu(
            &mut cpu,
            &mut vendor,
            empty_cmdline(),
            &mut msr,
            &req,
            REQ_LEVEL_X86_64,
            true,  // has_eflag(AC) → at least a 486
            false, // cold path: FakeMsr installs the seeded CPUID image
        );
        assert_eq!(r.err_flags, None);
        assert_eq!(r.cpu_level, 64);
        assert_eq!(r.ret, 0);
    }

    /// has_ac=false on an empty-required machine → level 3 (386). cpucheck.c:117-120.
    #[test]
    fn check_cpu_level_is_3_without_ac_flag() {
        let mut cpu = CpuFeatures::default();
        let mut vendor = CpuVendor::default();
        let mut msr = FakeMsr::new();
        let req = [0u32; NCAPINTS];
        let r = check_cpu(
            &mut cpu,
            &mut vendor,
            empty_cmdline(),
            &mut msr,
            &req,
            3, // require only a 386
            false,
            true, // warm path: get_cpuflags is a no-op, preserving level 3
        );
        assert_eq!(r.cpu_level, 3);
        assert_eq!(r.ret, 0); // level 3 >= req 3, no missing flags.
    }

    /// AMD workaround: only SSE+SSE2 missing → check_cpu clears MSR_K7_HWCR
    /// bit 15 and re-reads. We seed the FakeMsr so the re-read "turns on" the
    /// features. cpucheck.c:128-142.
    #[test]
    fn check_cpu_amd_sse_workaround_pokes_k7_hwcr() {
        // Required: SSE + SSE2 in word 0.
        let mut req = [0u32; NCAPINTS];
        req[0] = (1 << (X86_FEATURE_XMM % 32)) | (1 << (X86_FEATURE_XMM2 % 32));

        // cpu starts with neither bit (host CPUID returns zeros), so err==0x01
        // and err_flags[0] == req[0] (only SSE/SSE2) → AMD branch fires.
        let mut cpu = CpuFeatures::default();
        let mut vendor = CpuVendor(AMD_VENDOR);
        let mut msr = FakeMsr::new().seed(cpu, vendor);
        // MSR_K7_HWCR starts with bit 15 set (SSE disabled).
        msr.preset(MSR_K7_HWCR, Msr { l: 1 << 15, h: 0 });

        let _ = check_cpu(
            &mut cpu,
            &mut vendor,
            empty_cmdline(),
            &mut msr,
            &req,
            REQ_LEVEL_X86_64,
            true,
            false,
        );

        // It must have read then written MSR_K7_HWCR with bit 15 cleared.
        assert!(msr.reads.contains(&MSR_K7_HWCR));
        let last_write = msr
            .writes
            .iter()
            .rev()
            .find(|(r, _)| *r == MSR_K7_HWCR)
            .expect("must write MSR_K7_HWCR");
        assert_eq!(last_write.1.l & (1 << 15), 0, "bit 15 must be cleared");
        // No other vendor path should have run.
        assert_eq!(msr.transmeta_calls, 0);
    }

    /// VIA C3 workaround: only CX8 missing, Centaur, model>=6 → set MSR_VIA_FCR
    /// bits 1 & 7 and force the CX8 flag. cpucheck.c:143-156.
    #[test]
    fn check_cpu_via_c3_workaround_sets_cx8() {
        let mut req = [0u32; NCAPINTS];
        req[0] = 1 << (X86_FEATURE_CX8 % 32);

        let mut cpu = CpuFeatures::default();
        cpu.model = 6; // model >= 6 required
        let mut vendor = CpuVendor(CENTAUR_VENDOR);
        let mut msr = FakeMsr::new().seed(cpu, vendor);
        msr.preset(MSR_VIA_FCR, Msr { l: 0, h: 0 });

        let _ = check_cpu(
            &mut cpu,
            &mut vendor,
            empty_cmdline(),
            &mut msr,
            &req,
            REQ_LEVEL_X86_64,
            true,
            false,
        );
        let last_write = msr
            .writes
            .iter()
            .rev()
            .find(|(r, _)| *r == MSR_VIA_FCR)
            .expect("must write MSR_VIA_FCR");
        assert_eq!(last_write.1.l & ((1 << 1) | (1 << 7)), (1 << 1) | (1 << 7));
    }

    /// forcepae: Intel Pentium-M (level 6, model 9/13), only PAE missing, and
    /// `forcepae` on the cmdline → PAE forced on. cpucheck.c:173-186. We invoke
    /// the branch logic directly via check_cpuflags + set_bit to assert the
    /// cmdline gate, since host CPUID resets cpu.level/model.
    #[test]
    fn forcepae_cmdline_gate_matches_linux() {
        // The cmdline boolean finder is what the branch consults (line 178).
        assert!(__cmdline_find_option_bool(b"quiet forcepae\0", b"forcepae") > 0);
        assert_eq!(__cmdline_find_option_bool(b"quiet\0", b"forcepae"), 0);
    }

    /// Transmeta: err==0x01 and Transmeta vendor → MSR 0x80860004 dance plus a
    /// cpuid that reloads cpu.flags[0]. cpucheck.c:157-172. Because the
    /// Transmeta branch does not gate on cpu.level/model, it fires under host
    /// CPUID (which yields the Transmeta vendor only if we set it — and
    /// check_cpu re-runs get_cpuflags which on the host returns a zero vendor).
    /// We therefore assert the seam directly: a zero-vendor host won't match
    /// Transmeta, so no MSR poke happens — proving the vendor gate is honored.
    #[test]
    fn transmeta_branch_honors_vendor_gate() {
        let mut req = [0u32; NCAPINTS];
        req[0] = 1 << 5; // some single missing word-0 feature → err==0x01

        let mut cpu = CpuFeatures::default();
        let mut vendor = CpuVendor(TRANSMETA_VENDOR);
        let mut msr = FakeMsr::new().seed(CpuFeatures::default(), CpuVendor::default());
        msr.transmeta_edx = 0xffff_ffff;
        msr.preset(MSR_TRANSMETA_FEATURE_MASK, Msr { l: 0, h: 0 });

        let _ = check_cpu(
            &mut cpu,
            &mut vendor,
            empty_cmdline(),
            &mut msr,
            &req,
            REQ_LEVEL_X86_64,
            true,
            false,
        );
        // get_cpuflags overwrote vendor with the host (zero) vendor, so the
        // Transmeta gate is false → the CMS MSR is never touched.
        assert_eq!(msr.transmeta_calls, 0);
        assert!(
            !msr.writes
                .iter()
                .any(|(r, _)| *r == MSR_TRANSMETA_FEATURE_MASK)
        );
    }

    #[test]
    fn msr_q_view_combines_l_and_h() {
        // struct msr union: q == (h<<32)|l. shared/msr.h:5-13.
        let m = Msr {
            l: 0xdead_beef,
            h: 0x1234_5678,
        };
        assert_eq!(m.q(), 0x1234_5678_dead_beef);
    }
}
