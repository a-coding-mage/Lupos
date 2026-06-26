//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/kvm/x86.c
//! test-origin: linux:vendor/linux/arch/x86/kvm/x86.c
//! Common x86 KVM vCPU policy and state model.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kvm/x86.c
//!
//! This module keeps the Linux `x86.c` control-state and ABI-facing policy
//! in compiled Rust.  Hardware VM-entry, tracepoints, and host MSR side
//! effects are represented as explicit state transitions so the surrounding
//! Lupos kernel can test the same validation and request semantics without
//! depending on VMX/SVM hardware during normal library builds.

#![allow(dead_code)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

use crate::include::uapi::errno::{EBUSY, EINVAL, ENOMEM};

pub type KvmResult<T> = Result<T, i32>;

pub const X86_CR0_PE: u64 = 1 << 0;
pub const X86_CR0_MP: u64 = 1 << 1;
pub const X86_CR0_EM: u64 = 1 << 2;
pub const X86_CR0_TS: u64 = 1 << 3;
pub const X86_CR0_ET: u64 = 1 << 4;
pub const X86_CR0_NE: u64 = 1 << 5;
pub const X86_CR0_WP: u64 = 1 << 16;
pub const X86_CR0_AM: u64 = 1 << 18;
pub const X86_CR0_NW: u64 = 1 << 29;
pub const X86_CR0_CD: u64 = 1 << 30;
pub const X86_CR0_PG: u64 = 1 << 31;

pub const X86_CR4_VME: u64 = 1 << 0;
pub const X86_CR4_PVI: u64 = 1 << 1;
pub const X86_CR4_TSD: u64 = 1 << 2;
pub const X86_CR4_DE: u64 = 1 << 3;
pub const X86_CR4_PSE: u64 = 1 << 4;
pub const X86_CR4_PAE: u64 = 1 << 5;
pub const X86_CR4_MCE: u64 = 1 << 6;
pub const X86_CR4_PGE: u64 = 1 << 7;
pub const X86_CR4_PCE: u64 = 1 << 8;
pub const X86_CR4_OSFXSR: u64 = 1 << 9;
pub const X86_CR4_OSXMMEXCPT: u64 = 1 << 10;
pub const X86_CR4_UMIP: u64 = 1 << 11;
pub const X86_CR4_LA57: u64 = 1 << 12;
pub const X86_CR4_VMXE: u64 = 1 << 13;
pub const X86_CR4_SMXE: u64 = 1 << 14;
pub const X86_CR4_FSGSBASE: u64 = 1 << 16;
pub const X86_CR4_PCIDE: u64 = 1 << 17;
pub const X86_CR4_OSXSAVE: u64 = 1 << 18;
pub const X86_CR4_SMEP: u64 = 1 << 20;
pub const X86_CR4_SMAP: u64 = 1 << 21;
pub const X86_CR4_PKE: u64 = 1 << 22;
pub const X86_CR4_CET: u64 = 1 << 23;
pub const X86_CR4_LAM_SUP: u64 = 1 << 28;

pub const EFER_SCE: u64 = 1 << 0;
pub const EFER_LME: u64 = 1 << 8;
pub const EFER_LMA: u64 = 1 << 10;
pub const EFER_NX: u64 = 1 << 11;
pub const EFER_SVME: u64 = 1 << 12;
pub const EFER_FFXSR: u64 = 1 << 14;
pub const EFER_AUTOIBRS: u64 = 1 << 21;

pub const CR0_RESERVED_BITS: u64 = !(X86_CR0_PE
    | X86_CR0_MP
    | X86_CR0_EM
    | X86_CR0_TS
    | X86_CR0_ET
    | X86_CR0_NE
    | X86_CR0_WP
    | X86_CR0_AM
    | X86_CR0_NW
    | X86_CR0_CD
    | X86_CR0_PG);

pub const CR4_RESERVED_BITS: u64 = !(X86_CR4_VME
    | X86_CR4_PVI
    | X86_CR4_TSD
    | X86_CR4_DE
    | X86_CR4_PSE
    | X86_CR4_PAE
    | X86_CR4_MCE
    | X86_CR4_PGE
    | X86_CR4_PCE
    | X86_CR4_OSFXSR
    | X86_CR4_PCIDE
    | X86_CR4_OSXSAVE
    | X86_CR4_SMEP
    | X86_CR4_FSGSBASE
    | X86_CR4_OSXMMEXCPT
    | X86_CR4_LA57
    | X86_CR4_VMXE
    | X86_CR4_SMAP
    | X86_CR4_PKE
    | X86_CR4_UMIP
    | X86_CR4_LAM_SUP
    | X86_CR4_CET);

pub const KVM_MMU_CR0_ROLE_BITS: u64 = X86_CR0_PG | X86_CR0_WP | X86_CR0_CD | X86_CR0_NW;
pub const KVM_MMU_CR4_ROLE_BITS: u64 = X86_CR4_PAE | X86_CR4_PSE | X86_CR4_PGE | X86_CR4_LA57;
pub const KVM_MMU_EFER_ROLE_BITS: u64 = EFER_LME | EFER_NX;
pub const KVM_CR3_PCID_MASK: u64 = 0xfff;
pub const KVM_NR_INTERRUPTS: usize = 256;
pub const ASYNC_PF_PER_VCPU: usize = 64;
pub const KVM_DEFAULT_TSC_SCALING_FRAC_BITS: u8 = 48;

pub const X86_FEATURE_AUTOIBRS: u128 = 1 << 0;
pub const X86_FEATURE_FXSR_OPT: u128 = 1 << 1;
pub const X86_FEATURE_SVM: u128 = 1 << 2;
pub const X86_FEATURE_LM: u128 = 1 << 3;
pub const X86_FEATURE_NX: u128 = 1 << 4;
pub const X86_FEATURE_CET: u128 = 1 << 5;
pub const X86_FEATURE_PCID: u128 = 1 << 6;
pub const X86_FEATURE_XSAVES: u128 = 1 << 7;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct X86FeatureSet {
    pub bits: u128,
}

impl X86FeatureSet {
    pub const fn new(bits: u128) -> Self {
        Self { bits }
    }

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn has(self, feature: u128) -> bool {
        self.bits & feature != 0
    }

    pub const fn with(self, feature: u128) -> Self {
        Self {
            bits: self.bits | feature,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmCaps {
    pub has_tsc_control: bool,
    pub max_guest_tsc_khz: u32,
    pub tsc_scaling_ratio_frac_bits: u8,
    pub max_tsc_scaling_ratio: u64,
    pub default_tsc_scaling_ratio: u64,
    pub has_bus_lock_exit: bool,
    pub has_notify_vmexit: bool,
    pub supported_vm_types: u32,
    pub supported_mce_cap: u64,
    pub supported_xcr0: u64,
    pub supported_xss: u64,
    pub supported_perf_cap: u64,
    pub supported_quirks: u64,
    pub inapplicable_quirks: u64,
}

impl Default for KvmCaps {
    fn default() -> Self {
        Self {
            has_tsc_control: false,
            max_guest_tsc_khz: 0,
            tsc_scaling_ratio_frac_bits: KVM_DEFAULT_TSC_SCALING_FRAC_BITS,
            max_tsc_scaling_ratio: u64::MAX,
            default_tsc_scaling_ratio: 1u64 << KVM_DEFAULT_TSC_SCALING_FRAC_BITS,
            has_bus_lock_exit: false,
            has_notify_vmexit: false,
            supported_vm_types: 1,
            supported_mce_cap: 0,
            supported_xcr0: 0,
            supported_xss: 0,
            supported_perf_cap: 0,
            supported_quirks: 0,
            inapplicable_quirks: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmHostValues {
    pub maxphyaddr: u8,
    pub efer: u64,
    pub xcr0: u64,
    pub xss: u64,
    pub s_cet: u64,
    pub arch_capabilities: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmX86Policy {
    pub guest_features: X86FeatureSet,
    pub efer_reserved_bits: u64,
    pub cr4_reserved_bits: u64,
    pub maxphyaddr: u8,
    pub vendor_allows_cr0: bool,
    pub vendor_allows_cr4: bool,
}

impl Default for KvmX86Policy {
    fn default() -> Self {
        Self {
            guest_features: X86FeatureSet::empty(),
            efer_reserved_bits: !(EFER_SCE | EFER_LME | EFER_LMA),
            cr4_reserved_bits: CR4_RESERVED_BITS,
            maxphyaddr: 52,
            vendor_allows_cr0: true,
            vendor_allows_cr4: true,
        }
    }
}

impl KvmX86Policy {
    pub const fn enable_efer_bits(mut self, mask: u64) -> Self {
        self.efer_reserved_bits &= !mask;
        self
    }

    pub const fn with_features(mut self, features: X86FeatureSet) -> Self {
        self.guest_features = features;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmX86Mode {
    pub cr0: u64,
    pub cr4: u64,
    pub efer: u64,
}

pub const fn kvm_x86_long_mode(mode: KvmX86Mode) -> bool {
    mode.cr0 & X86_CR0_PG != 0 && mode.cr4 & X86_CR4_PAE != 0 && mode.efer & EFER_LME != 0
}

pub const fn validate_kvm_x86_mode(mode: KvmX86Mode) -> KvmResult<()> {
    if mode.cr0 & X86_CR0_PG != 0 && mode.cr0 & X86_CR0_PE == 0 {
        return Err(EINVAL);
    }
    if mode.efer & EFER_LME != 0 && mode.cr4 & X86_CR4_PAE == 0 {
        return Err(EINVAL);
    }
    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmSegment {
    pub base: u64,
    pub limit: u32,
    pub selector: u16,
    pub type_: u8,
    pub present: u8,
    pub dpl: u8,
    pub db: u8,
    pub s: u8,
    pub l: u8,
    pub g: u8,
    pub avl: u8,
    pub unusable: u8,
    pub padding: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmDtable {
    pub base: u64,
    pub limit: u16,
    pub padding: [u16; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmSregs {
    pub cs: KvmSegment,
    pub ds: KvmSegment,
    pub es: KvmSegment,
    pub fs: KvmSegment,
    pub gs: KvmSegment,
    pub ss: KvmSegment,
    pub tr: KvmSegment,
    pub ldt: KvmSegment,
    pub gdt: KvmDtable,
    pub idt: KvmDtable,
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cr8: u64,
    pub efer: u64,
    pub apic_base: u64,
    pub interrupt_bitmap: [u64; 4],
}

impl Default for KvmSregs {
    fn default() -> Self {
        Self {
            cs: KvmSegment::default(),
            ds: KvmSegment::default(),
            es: KvmSegment::default(),
            fs: KvmSegment::default(),
            gs: KvmSegment::default(),
            ss: KvmSegment::default(),
            tr: KvmSegment::default(),
            ldt: KvmSegment::default(),
            gdt: KvmDtable::default(),
            idt: KvmDtable::default(),
            cr0: X86_CR0_ET,
            cr2: 0,
            cr3: 0,
            cr4: 0,
            cr8: 0,
            efer: 0,
            apic_base: 0,
            interrupt_bitmap: [0; 4],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmRegs {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmMpState {
    Runnable,
    Uninitialized,
    InitReceived,
    Halted,
    SipiReceived,
    Stopped,
    CheckStop,
    Operating,
    Load,
    ApResetHold,
}

impl Default for KvmMpState {
    fn default() -> Self {
        Self::Uninitialized
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmExceptionVector {
    De = 0,
    Db = 1,
    Nmi = 2,
    Bp = 3,
    Of = 4,
    Br = 5,
    Ud = 6,
    Nm = 7,
    Df = 8,
    Ts = 10,
    Np = 11,
    Ss = 12,
    Gp = 13,
    Pf = 14,
    Mf = 16,
    Ac = 17,
    Mc = 18,
    Xm = 19,
    Ve = 20,
    Cp = 21,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExceptionClass {
    Benign,
    Contributory,
    PageFault,
    DoubleFault,
}

pub const fn exception_class(vector: u8) -> ExceptionClass {
    match vector {
        8 => ExceptionClass::DoubleFault,
        10 | 11 | 12 | 13 => ExceptionClass::Contributory,
        14 => ExceptionClass::PageFault,
        _ => ExceptionClass::Benign,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmQueuedException {
    pub vector: u8,
    pub has_error_code: bool,
    pub error_code: u32,
    pub has_payload: bool,
    pub payload: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmExceptionState {
    pub pending: bool,
    pub injected: bool,
    pub triple_fault: bool,
    pub event: KvmQueuedException,
}

impl KvmExceptionState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn queue(&mut self, vector: u8, error_code: Option<u32>) {
        if !self.pending {
            self.pending = true;
            self.event = KvmQueuedException {
                vector,
                has_error_code: error_code.is_some(),
                error_code: error_code.unwrap_or(0),
                has_payload: false,
                payload: 0,
            };
            return;
        }

        let old = exception_class(self.event.vector);
        let new = exception_class(vector);
        if old == ExceptionClass::DoubleFault || new == ExceptionClass::DoubleFault {
            self.triple_fault = true;
            return;
        }

        if (old == ExceptionClass::Contributory && new != ExceptionClass::Benign)
            || (old == ExceptionClass::PageFault && new != ExceptionClass::Benign)
        {
            self.event = KvmQueuedException {
                vector: KvmExceptionVector::Df as u8,
                has_error_code: true,
                error_code: 0,
                has_payload: false,
                payload: 0,
            };
        } else {
            self.event = KvmQueuedException {
                vector,
                has_error_code: error_code.is_some(),
                error_code: error_code.unwrap_or(0),
                has_payload: false,
                payload: 0,
            };
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KvmRequest {
    MigrateTimer = 0,
    ReportTprAccess = 1,
    TripleFault = 2,
    MmuSync = 3,
    ClockUpdate = 4,
    LoadMmuPgd = 5,
    Event = 6,
    ApfHalt = 7,
    StealUpdate = 8,
    Nmi = 9,
    Pmu = 10,
    Pmi = 11,
    MasterclockUpdate = 13,
    ScanIoapic = 15,
    GlobalClockUpdate = 16,
    TlbFlushCurrent = 26,
    TlbFlushGuest = 27,
    ApfReady = 28,
    RecalcIntercepts = 29,
    UpdateCpuDirtyLogging = 30,
    MmuFreeObsoleteRoots = 31,
    HvTlbFlush = 32,
    UpdateProtectedGuestState = 34,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmRequestSet {
    bits: u128,
}

impl KvmRequestSet {
    pub const fn contains(self, req: KvmRequest) -> bool {
        self.bits & (1u128 << (req as u8)) != 0
    }

    pub fn make(&mut self, req: KvmRequest) {
        self.bits |= 1u128 << (req as u8);
    }

    pub fn clear(&mut self, req: KvmRequest) {
        self.bits &= !(1u128 << (req as u8));
    }

    pub fn take(&mut self, req: KvmRequest) -> bool {
        let had = self.contains(req);
        self.clear(req);
        had
    }

    pub const fn any(self) -> bool {
        self.bits != 0
    }
}

pub const KVM_MSR_FILTER_READ: u32 = 1 << 0;
pub const KVM_MSR_FILTER_WRITE: u32 = 1 << 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MsrBitmapRange {
    pub flags: u32,
    pub base: u32,
    pub nmsrs: u32,
    pub bitmap: Vec<u64>,
}

impl MsrBitmapRange {
    pub fn new(base: u32, nmsrs: u32, flags: u32, default_allowed: bool) -> KvmResult<Self> {
        let words = ((nmsrs as usize) + 63) / 64;
        if words == 0 {
            return Err(EINVAL);
        }
        let fill = if default_allowed { u64::MAX } else { 0 };
        Ok(Self {
            flags,
            base,
            nmsrs,
            bitmap: vec![fill; words],
        })
    }

    pub fn set_allowed(&mut self, index: u32, allowed: bool) -> KvmResult<()> {
        if index < self.base || index >= self.base.saturating_add(self.nmsrs) {
            return Err(EINVAL);
        }
        let bit = index - self.base;
        let word = (bit / 64) as usize;
        let mask = 1u64 << (bit % 64);
        if allowed {
            self.bitmap[word] |= mask;
        } else {
            self.bitmap[word] &= !mask;
        }
        Ok(())
    }

    fn applies_to(&self, index: u32, access: u32) -> bool {
        index >= self.base
            && index < self.base.saturating_add(self.nmsrs)
            && self.flags & access != 0
    }

    fn allowed(&self, index: u32) -> bool {
        let bit = index - self.base;
        let word = (bit / 64) as usize;
        let mask = 1u64 << (bit % 64);
        self.bitmap
            .get(word)
            .map(|word_bits| word_bits & mask != 0)
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmMsrFilter {
    pub default_allow: bool,
    pub ranges: Vec<MsrBitmapRange>,
}

impl Default for KvmMsrFilter {
    fn default() -> Self {
        Self {
            default_allow: true,
            ranges: Vec::new(),
        }
    }
}

impl KvmMsrFilter {
    pub fn allowed(&self, index: u32, access: u32) -> bool {
        if (0x800..=0x8ff).contains(&index) {
            return true;
        }
        for range in self.ranges.iter() {
            if range.applies_to(index, access) {
                return range.allowed(index);
            }
        }
        self.default_allow
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmSetOutcome {
    pub mmu_reset: bool,
    pub tlb_flush_guest: bool,
    pub apf_ready: bool,
    pub load_mmu_pgd: bool,
}

impl KvmSetOutcome {
    fn merge(&mut self, other: KvmSetOutcome) {
        self.mmu_reset |= other.mmu_reset;
        self.tlb_flush_guest |= other.tlb_flush_guest;
        self.apf_ready |= other.apf_ready;
        self.load_mmu_pgd |= other.load_mmu_pgd;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KvmTscState {
    pub host_tsc_khz: u32,
    pub virtual_tsc_khz: u32,
    pub tsc_scaling_ratio: u64,
    pub virtual_tsc_mult: u32,
    pub virtual_tsc_shift: i8,
    pub this_tsc_nsec: i64,
    pub this_tsc_write: u64,
    pub l1_tsc_offset: u64,
    pub l1_tsc_multiplier: u64,
    pub last_guest_tsc: u64,
}

impl Default for KvmTscState {
    fn default() -> Self {
        Self {
            host_tsc_khz: 0,
            virtual_tsc_khz: 0,
            tsc_scaling_ratio: 1u64 << KVM_DEFAULT_TSC_SCALING_FRAC_BITS,
            virtual_tsc_mult: 0,
            virtual_tsc_shift: 0,
            this_tsc_nsec: 0,
            this_tsc_write: 0,
            l1_tsc_offset: 0,
            l1_tsc_multiplier: 1u64 << KVM_DEFAULT_TSC_SCALING_FRAC_BITS,
            last_guest_tsc: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmTimeScale {
    pub mult: u32,
    pub shift: i8,
}

pub fn kvm_get_time_scale(scaled_hz: u64, base_hz: u64) -> KvmTimeScale {
    if scaled_hz == 0 || base_hz == 0 {
        return KvmTimeScale::default();
    }
    let mult = (((scaled_hz as u128) << 32) / (base_hz as u128)).min(u32::MAX as u128) as u32;
    KvmTimeScale { mult, shift: 0 }
}

pub fn adjust_tsc_khz(khz: u32, ppm: i32) -> u32 {
    let delta = (khz as i128 * ppm as i128) / 1_000_000;
    if delta.is_negative() {
        khz.saturating_sub((-delta) as u32)
    } else {
        khz.saturating_add(delta as u32)
    }
}

pub fn kvm_scale_tsc(tsc: u64, ratio: u64, caps: KvmCaps) -> u64 {
    if ratio == caps.default_tsc_scaling_ratio {
        return tsc;
    }
    ((tsc as u128 * ratio as u128) >> caps.tsc_scaling_ratio_frac_bits) as u64
}

pub fn kvm_calc_nested_tsc_offset(l1_offset: u64, l2_offset: u64, l2_multiplier: u64) -> u64 {
    l1_offset.wrapping_add(kvm_scale_tsc(l2_offset, l2_multiplier, KvmCaps::default()))
}

pub fn kvm_calc_nested_tsc_multiplier(l1_multiplier: u64, l2_multiplier: u64) -> u64 {
    ((l1_multiplier as u128 * l2_multiplier as u128) >> KVM_DEFAULT_TSC_SCALING_FRAC_BITS) as u64
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KvmAsyncPf {
    pub enabled: bool,
    pub interrupt_enabled: bool,
    gfns: Vec<u64>,
}

impl KvmAsyncPf {
    pub fn add_gfn(&mut self, gfn: u64) -> KvmResult<()> {
        if self.gfns.contains(&gfn) {
            return Ok(());
        }
        if self.gfns.len() >= ASYNC_PF_PER_VCPU {
            return Err(ENOMEM);
        }
        self.gfns.push(gfn);
        Ok(())
    }

    pub fn find_gfn(&self, gfn: u64) -> bool {
        self.gfns.contains(&gfn)
    }

    pub fn del_gfn(&mut self, gfn: u64) -> bool {
        if let Some(pos) = self.gfns.iter().position(|entry| *entry == gfn) {
            self.gfns.swap_remove(pos);
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmVcpuArch {
    pub cr0: u64,
    pub cr2: u64,
    pub cr3: u64,
    pub cr4: u64,
    pub cr8: u64,
    pub efer: u64,
    pub regs: KvmRegs,
    pub sregs: KvmSregs,
    pub exception: KvmExceptionState,
    pub mp_state: KvmMpState,
    pub pdptrs: [u64; 4],
    pub pdptrs_from_userspace: bool,
    pub guest_state_protected: bool,
    pub apf: KvmAsyncPf,
    pub tsc: KvmTscState,
}

impl Default for KvmVcpuArch {
    fn default() -> Self {
        let sregs = KvmSregs::default();
        Self {
            cr0: sregs.cr0,
            cr2: sregs.cr2,
            cr3: sregs.cr3,
            cr4: sregs.cr4,
            cr8: sregs.cr8,
            efer: sregs.efer,
            regs: KvmRegs {
                rflags: 0x2,
                ..KvmRegs::default()
            },
            sregs,
            exception: KvmExceptionState::default(),
            mp_state: KvmMpState::Uninitialized,
            pdptrs: [0; 4],
            pdptrs_from_userspace: false,
            guest_state_protected: false,
            apf: KvmAsyncPf::default(),
            tsc: KvmTscState::default(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmVcpu {
    pub id: u32,
    pub arch: KvmVcpuArch,
    pub policy: KvmX86Policy,
    pub requests: KvmRequestSet,
    pub msr_filter: Option<KvmMsrFilter>,
    pub has_run: bool,
}

impl KvmVcpu {
    pub fn new(id: u32, policy: KvmX86Policy) -> Self {
        Self {
            id,
            arch: KvmVcpuArch::default(),
            policy,
            requests: KvmRequestSet::default(),
            msr_filter: None,
            has_run: false,
        }
    }

    pub fn make_request(&mut self, req: KvmRequest) {
        self.requests.make(req);
    }

    pub fn check_request(&mut self, req: KvmRequest) -> bool {
        self.requests.take(req)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KvmMemorySlot {
    pub id: u32,
    pub base_gfn: u64,
    pub npages: u64,
    pub flags: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KvmVm {
    pub caps: KvmCaps,
    pub host: KvmHostValues,
    pub default_tsc_khz: u32,
    pub nr_mmu_pages: u64,
    pub memory_slots: Vec<KvmMemorySlot>,
    pub msr_filter: Option<KvmMsrFilter>,
}

impl Default for KvmVm {
    fn default() -> Self {
        Self {
            caps: KvmCaps::default(),
            host: KvmHostValues::default(),
            default_tsc_khz: 0,
            nr_mmu_pages: 0,
            memory_slots: Vec::new(),
            msr_filter: None,
        }
    }
}

impl KvmVm {
    pub fn prepare_memory_region(&self, slot: KvmMemorySlot) -> KvmResult<()> {
        if slot.npages == 0 {
            return Err(EINVAL);
        }
        if slot.base_gfn.checked_add(slot.npages).is_none() {
            return Err(EINVAL);
        }
        Ok(())
    }

    pub fn commit_memory_region(&mut self, slot: KvmMemorySlot) -> KvmResult<()> {
        self.prepare_memory_region(slot)?;
        if let Some(existing) = self
            .memory_slots
            .iter_mut()
            .find(|existing| existing.id == slot.id)
        {
            *existing = slot;
        } else {
            self.memory_slots.push(slot);
        }
        Ok(())
    }
}

pub const fn is_paging(vcpu: &KvmVcpu) -> bool {
    vcpu.arch.cr0 & X86_CR0_PG != 0
}

pub const fn is_pae(vcpu: &KvmVcpu) -> bool {
    vcpu.arch.cr4 & X86_CR4_PAE != 0
}

pub const fn is_long_mode(vcpu: &KvmVcpu) -> bool {
    vcpu.arch.efer & EFER_LMA != 0
}

pub const fn is_64_bit_mode(vcpu: &KvmVcpu) -> bool {
    is_long_mode(vcpu) && vcpu.arch.sregs.cs.l != 0
}

pub const fn kvm_is_cr4_bit_set(vcpu: &KvmVcpu, bit: u64) -> bool {
    vcpu.arch.cr4 & bit != 0
}

pub fn kvm_is_valid_cr0(vcpu: &KvmVcpu, cr0: u64) -> bool {
    if cr0 & 0xffff_ffff_0000_0000 != 0 {
        return false;
    }
    if cr0 & X86_CR0_NW != 0 && cr0 & X86_CR0_CD == 0 {
        return false;
    }
    if cr0 & X86_CR0_PG != 0 && cr0 & X86_CR0_PE == 0 {
        return false;
    }
    vcpu.policy.vendor_allows_cr0
}

pub fn kvm_is_valid_cr4(vcpu: &KvmVcpu, cr4: u64) -> bool {
    if cr4 & vcpu.policy.cr4_reserved_bits != 0 {
        return false;
    }
    if cr4 & X86_CR4_PCIDE != 0 {
        if vcpu.arch.cr0 & X86_CR0_PG == 0 || vcpu.arch.efer & EFER_LMA == 0 {
            return false;
        }
        if vcpu.arch.cr3 & KVM_CR3_PCID_MASK != 0 {
            return false;
        }
    }
    if cr4 & X86_CR4_CET != 0 && !vcpu.policy.guest_features.has(X86_FEATURE_CET) {
        return false;
    }
    vcpu.policy.vendor_allows_cr4
}

pub fn kvm_vcpu_is_legal_cr3(vcpu: &KvmVcpu, cr3: u64) -> bool {
    if vcpu.policy.maxphyaddr >= 64 {
        return true;
    }
    let reserved = cr3 >> vcpu.policy.maxphyaddr;
    reserved == 0
}

fn kvm_valid_efer_features(vcpu: &KvmVcpu, efer: u64) -> bool {
    let features = vcpu.policy.guest_features;
    if efer & EFER_AUTOIBRS != 0 && !features.has(X86_FEATURE_AUTOIBRS) {
        return false;
    }
    if efer & EFER_FFXSR != 0 && !features.has(X86_FEATURE_FXSR_OPT) {
        return false;
    }
    if efer & EFER_SVME != 0 && !features.has(X86_FEATURE_SVM) {
        return false;
    }
    if efer & (EFER_LME | EFER_LMA) != 0 && !features.has(X86_FEATURE_LM) {
        return false;
    }
    if efer & EFER_NX != 0 && !features.has(X86_FEATURE_NX) {
        return false;
    }
    true
}

pub fn kvm_valid_efer(vcpu: &KvmVcpu, efer: u64) -> bool {
    if efer & vcpu.policy.efer_reserved_bits != 0 {
        return false;
    }
    kvm_valid_efer_features(vcpu, efer)
}

pub fn kvm_is_valid_sregs(vcpu: &KvmVcpu, sregs: &KvmSregs) -> bool {
    if sregs.efer & EFER_LME != 0 && sregs.cr0 & X86_CR0_PG != 0 {
        if sregs.cr4 & X86_CR4_PAE == 0 || sregs.efer & EFER_LMA == 0 {
            return false;
        }
        if !kvm_vcpu_is_legal_cr3(vcpu, sregs.cr3) {
            return false;
        }
    } else if sregs.efer & EFER_LMA != 0 || sregs.cs.l != 0 {
        return false;
    }

    kvm_is_valid_cr4(vcpu, sregs.cr4) && kvm_is_valid_cr0(vcpu, sregs.cr0)
}

pub fn kvm_post_set_cr0(vcpu: &mut KvmVcpu, old_cr0: u64, cr0: u64) -> KvmSetOutcome {
    let mut out = KvmSetOutcome::default();
    if (cr0 ^ old_cr0) & X86_CR0_PG != 0 {
        if cr0 & X86_CR0_PG == 0 {
            out.tlb_flush_guest = true;
            vcpu.make_request(KvmRequest::TlbFlushGuest);
        } else if vcpu.arch.apf.enabled {
            out.apf_ready = true;
            vcpu.make_request(KvmRequest::ApfReady);
        }
    }
    if (cr0 ^ old_cr0) & KVM_MMU_CR0_ROLE_BITS != 0 {
        out.mmu_reset = true;
        vcpu.make_request(KvmRequest::LoadMmuPgd);
    }
    out
}

pub fn kvm_set_cr0(vcpu: &mut KvmVcpu, mut cr0: u64) -> KvmResult<KvmSetOutcome> {
    let old_cr0 = vcpu.arch.cr0;
    if !kvm_is_valid_cr0(vcpu, cr0) {
        return Err(EINVAL);
    }

    cr0 |= X86_CR0_ET;
    cr0 &= !CR0_RESERVED_BITS;

    if vcpu.arch.efer & EFER_LME != 0 && !is_paging(vcpu) && cr0 & X86_CR0_PG != 0 {
        if !is_pae(vcpu) || vcpu.arch.sregs.cs.l != 0 {
            return Err(EINVAL);
        }
    }

    if cr0 & X86_CR0_PG == 0 && (is_64_bit_mode(vcpu) || kvm_is_cr4_bit_set(vcpu, X86_CR4_PCIDE)) {
        return Err(EINVAL);
    }

    if cr0 & X86_CR0_WP == 0 && kvm_is_cr4_bit_set(vcpu, X86_CR4_CET) {
        return Err(EINVAL);
    }

    vcpu.arch.cr0 = cr0;
    vcpu.arch.sregs.cr0 = cr0;
    Ok(kvm_post_set_cr0(vcpu, old_cr0, cr0))
}

pub fn kvm_post_set_cr4(vcpu: &mut KvmVcpu, old_cr4: u64, cr4: u64) -> KvmSetOutcome {
    let mut out = KvmSetOutcome::default();
    if (cr4 ^ old_cr4) & KVM_MMU_CR4_ROLE_BITS != 0 {
        out.mmu_reset = true;
        vcpu.make_request(KvmRequest::LoadMmuPgd);
    }
    out
}

pub fn kvm_set_cr4(vcpu: &mut KvmVcpu, cr4: u64) -> KvmResult<KvmSetOutcome> {
    let old_cr4 = vcpu.arch.cr4;
    if is_long_mode(vcpu) && cr4 & X86_CR4_PAE == 0 {
        return Err(EINVAL);
    }
    if cr4 & X86_CR4_PCIDE != 0 && (vcpu.arch.cr0 & X86_CR0_PG == 0 || !is_long_mode(vcpu)) {
        return Err(EINVAL);
    }
    if !kvm_is_valid_cr4(vcpu, cr4) {
        return Err(EINVAL);
    }
    vcpu.arch.cr4 = cr4;
    vcpu.arch.sregs.cr4 = cr4;
    Ok(kvm_post_set_cr4(vcpu, old_cr4, cr4))
}

pub fn kvm_set_cr3(vcpu: &mut KvmVcpu, cr3: u64) -> KvmResult<KvmSetOutcome> {
    if !kvm_vcpu_is_legal_cr3(vcpu, cr3) {
        return Err(EINVAL);
    }
    if kvm_is_cr4_bit_set(vcpu, X86_CR4_PCIDE) && cr3 & !KVM_CR3_PCID_MASK == 0 {
        return Err(EINVAL);
    }
    vcpu.arch.cr3 = cr3;
    vcpu.arch.sregs.cr3 = cr3;
    vcpu.make_request(KvmRequest::LoadMmuPgd);
    Ok(KvmSetOutcome {
        load_mmu_pgd: true,
        ..KvmSetOutcome::default()
    })
}

pub fn kvm_set_cr8(vcpu: &mut KvmVcpu, cr8: u64) -> KvmResult<()> {
    if cr8 > 15 {
        return Err(EINVAL);
    }
    vcpu.arch.cr8 = cr8;
    vcpu.arch.sregs.cr8 = cr8;
    Ok(())
}

pub fn kvm_set_efer(
    vcpu: &mut KvmVcpu,
    efer: u64,
    host_initiated: bool,
) -> KvmResult<KvmSetOutcome> {
    if efer & vcpu.policy.efer_reserved_bits != 0 {
        return Err(EINVAL);
    }
    if !host_initiated {
        if !kvm_valid_efer(vcpu, efer) {
            return Err(EINVAL);
        }
        if is_paging(vcpu) && (vcpu.arch.efer & EFER_LME) != (efer & EFER_LME) {
            return Err(EINVAL);
        }
    }

    let old_efer = vcpu.arch.efer;
    let next = (efer & !EFER_LMA) | (old_efer & EFER_LMA);
    vcpu.arch.efer = next;
    vcpu.arch.sregs.efer = next;
    let mmu_reset = (next ^ old_efer) & KVM_MMU_EFER_ROLE_BITS != 0;
    if mmu_reset {
        vcpu.make_request(KvmRequest::LoadMmuPgd);
    }
    Ok(KvmSetOutcome {
        mmu_reset,
        ..KvmSetOutcome::default()
    })
}

pub fn kvm_set_sregs(vcpu: &mut KvmVcpu, sregs: KvmSregs) -> KvmResult<KvmSetOutcome> {
    if !kvm_is_valid_sregs(vcpu, &sregs) {
        return Err(EINVAL);
    }
    let mut out = KvmSetOutcome::default();
    out.merge(kvm_set_cr0(vcpu, sregs.cr0)?);
    out.merge(kvm_set_cr4(vcpu, sregs.cr4)?);
    out.merge(kvm_set_cr3(vcpu, sregs.cr3)?);
    kvm_set_cr8(vcpu, sregs.cr8)?;
    out.merge(kvm_set_efer(vcpu, sregs.efer, true)?);
    vcpu.arch.cr2 = sregs.cr2;
    vcpu.arch.sregs = sregs;
    Ok(out)
}

pub fn kvm_get_regs(vcpu: &KvmVcpu) -> KvmRegs {
    vcpu.arch.regs
}

pub fn kvm_set_regs(vcpu: &mut KvmVcpu, mut regs: KvmRegs) {
    regs.rflags |= 0x2;
    vcpu.arch.regs = regs;
}

pub fn kvm_msr_allowed(vcpu: &KvmVcpu, index: u32, access: u32) -> bool {
    vcpu.msr_filter
        .as_ref()
        .map(|filter| filter.allowed(index, access))
        .unwrap_or(true)
}

pub fn kvm_set_tsc_khz(
    vcpu: &mut KvmVcpu,
    caps: KvmCaps,
    user_tsc_khz: u32,
    host_tsc_khz: u32,
    scale: bool,
) -> KvmResult<()> {
    if user_tsc_khz == 0 {
        return Err(EINVAL);
    }
    let ratio = if scale && caps.has_tsc_control {
        let ratio = ((1u128 << caps.tsc_scaling_ratio_frac_bits) * user_tsc_khz as u128)
            / host_tsc_khz.max(1) as u128;
        if ratio == 0 || ratio >= caps.max_tsc_scaling_ratio as u128 {
            return Err(EINVAL);
        }
        ratio as u64
    } else {
        caps.default_tsc_scaling_ratio
    };
    let scale = kvm_get_time_scale(user_tsc_khz as u64 * 1000, 1_000_000_000);
    vcpu.arch.tsc.host_tsc_khz = host_tsc_khz;
    vcpu.arch.tsc.virtual_tsc_khz = user_tsc_khz;
    vcpu.arch.tsc.tsc_scaling_ratio = ratio;
    vcpu.arch.tsc.virtual_tsc_mult = scale.mult;
    vcpu.arch.tsc.virtual_tsc_shift = scale.shift;
    Ok(())
}

pub fn compute_guest_tsc(vcpu: &KvmVcpu, kernel_ns: i64) -> u64 {
    let delta_ns = kernel_ns.saturating_sub(vcpu.arch.tsc.this_tsc_nsec) as u128;
    let delta_cycles = delta_ns * vcpu.arch.tsc.virtual_tsc_khz as u128 / 1_000_000;
    vcpu.arch
        .tsc
        .this_tsc_write
        .wrapping_add(delta_cycles as u64)
}

pub fn kvm_read_l1_tsc(vcpu: &KvmVcpu, host_tsc: u64, caps: KvmCaps) -> u64 {
    kvm_scale_tsc(host_tsc, vcpu.arch.tsc.l1_tsc_multiplier, caps)
        .wrapping_add(vcpu.arch.tsc.l1_tsc_offset)
}

pub fn kvm_emulate_halt(vcpu: &mut KvmVcpu) {
    vcpu.arch.mp_state = KvmMpState::Halted;
}

pub fn kvm_vcpu_reset(vcpu: &mut KvmVcpu, init_event: bool) {
    let id = vcpu.id;
    let policy = vcpu.policy;
    let filter = vcpu.msr_filter.clone();
    *vcpu = KvmVcpu::new(id, policy);
    vcpu.msr_filter = filter;
    if init_event {
        vcpu.arch.mp_state = KvmMpState::InitReceived;
    }
    vcpu.arch.sregs.cs.selector = 0xf000;
    vcpu.arch.sregs.cs.base = 0xffff_0000;
    vcpu.arch.regs.rip = 0xfff0;
}

pub fn kvm_arch_vcpu_precreate(vm: &KvmVm, id: u32) -> KvmResult<()> {
    if id as usize >= 1024 {
        return Err(EINVAL);
    }
    if vm.default_tsc_khz == 0 {
        return Err(EBUSY);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lm_policy() -> KvmX86Policy {
        KvmX86Policy::default()
            .enable_efer_bits(EFER_NX)
            .with_features(
                X86FeatureSet::empty()
                    .with(X86_FEATURE_LM)
                    .with(X86_FEATURE_NX),
            )
    }

    #[test]
    fn paging_requires_protected_mode() {
        assert_eq!(
            validate_kvm_x86_mode(KvmX86Mode {
                cr0: X86_CR0_PG,
                cr4: 0,
                efer: 0,
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn long_mode_requires_paging_pae_and_lme() {
        assert!(kvm_x86_long_mode(KvmX86Mode {
            cr0: X86_CR0_PE | X86_CR0_PG,
            cr4: X86_CR4_PAE,
            efer: EFER_LME,
        }));
    }

    #[test]
    fn cr0_rejects_nw_without_cd() {
        let vcpu = KvmVcpu::new(0, lm_policy());
        assert!(!kvm_is_valid_cr0(&vcpu, X86_CR0_NW));
        assert!(kvm_is_valid_cr0(&vcpu, X86_CR0_NW | X86_CR0_CD));
    }

    #[test]
    fn cr4_pcide_requires_paging_long_mode() {
        let mut vcpu = KvmVcpu::new(0, lm_policy());
        assert_eq!(kvm_set_cr4(&mut vcpu, X86_CR4_PCIDE), Err(EINVAL));
        vcpu.arch.cr0 = X86_CR0_PE | X86_CR0_PG;
        vcpu.arch.efer = EFER_LMA;
        assert_eq!(kvm_set_cr4(&mut vcpu, X86_CR4_PCIDE), Err(EINVAL));
    }

    #[test]
    fn efer_feature_checks_gate_guest_bits() {
        let mut vcpu = KvmVcpu::new(0, lm_policy());
        assert!(kvm_valid_efer(&vcpu, EFER_LME | EFER_NX));
        assert!(!kvm_valid_efer(&vcpu, EFER_SVME));
        vcpu.policy = vcpu
            .policy
            .enable_efer_bits(EFER_SVME)
            .with_features(vcpu.policy.guest_features.with(X86_FEATURE_SVM));
        assert!(kvm_valid_efer(&vcpu, EFER_SVME));
    }

    #[test]
    fn sregs_require_lma_when_paged_lme() {
        let vcpu = KvmVcpu::new(0, lm_policy());
        let mut sregs = KvmSregs {
            cr0: X86_CR0_PE | X86_CR0_PG,
            cr4: X86_CR4_PAE,
            efer: EFER_LME,
            ..KvmSregs::default()
        };
        assert!(!kvm_is_valid_sregs(&vcpu, &sregs));
        sregs.efer |= EFER_LMA;
        assert!(kvm_is_valid_sregs(&vcpu, &sregs));
    }

    #[test]
    fn set_cr0_requests_tlb_flush_when_paging_clears() {
        let mut vcpu = KvmVcpu::new(0, lm_policy());
        vcpu.arch.cr0 = X86_CR0_PE | X86_CR0_PG;
        vcpu.arch.sregs.cr0 = vcpu.arch.cr0;
        let out = kvm_set_cr0(&mut vcpu, X86_CR0_PE).unwrap();
        assert!(out.tlb_flush_guest);
        assert!(vcpu.requests.contains(KvmRequest::TlbFlushGuest));
    }

    #[test]
    fn msr_filter_matches_linux_default_and_range_override() {
        let mut range = MsrBitmapRange::new(0x100, 8, KVM_MSR_FILTER_READ, true).unwrap();
        range.set_allowed(0x103, false).unwrap();
        let filter = KvmMsrFilter {
            default_allow: false,
            ranges: vec![range],
        };
        assert!(filter.allowed(0x101, KVM_MSR_FILTER_READ));
        assert!(!filter.allowed(0x103, KVM_MSR_FILTER_READ));
        assert!(!filter.allowed(0x101, KVM_MSR_FILTER_WRITE));
        assert!(filter.allowed(0x800, KVM_MSR_FILTER_READ));
    }

    #[test]
    fn exception_queue_escalates_to_double_and_triple_fault() {
        let mut state = KvmExceptionState::default();
        state.queue(KvmExceptionVector::Gp as u8, Some(0));
        state.queue(KvmExceptionVector::Ss as u8, Some(0));
        assert_eq!(state.event.vector, KvmExceptionVector::Df as u8);
        state.queue(KvmExceptionVector::Gp as u8, Some(0));
        assert!(state.triple_fault);
    }

    #[test]
    fn request_set_take_clears_bit() {
        let mut requests = KvmRequestSet::default();
        requests.make(KvmRequest::Event);
        assert!(requests.contains(KvmRequest::Event));
        assert!(requests.take(KvmRequest::Event));
        assert!(!requests.contains(KvmRequest::Event));
    }

    #[test]
    fn tsc_scaling_uses_caps_fraction_bits() {
        let caps = KvmCaps {
            has_tsc_control: true,
            max_tsc_scaling_ratio: u64::MAX,
            ..KvmCaps::default()
        };
        assert_eq!(
            kvm_scale_tsc(100, caps.default_tsc_scaling_ratio, caps),
            100
        );
        assert_eq!(
            kvm_scale_tsc(100, caps.default_tsc_scaling_ratio / 2, caps),
            50
        );
    }

    #[test]
    fn async_pf_tracks_gfns() {
        let mut apf = KvmAsyncPf::default();
        apf.add_gfn(42).unwrap();
        assert!(apf.find_gfn(42));
        assert!(apf.del_gfn(42));
        assert!(!apf.find_gfn(42));
    }

    #[test]
    fn memory_region_rejects_empty_slots() {
        let mut vm = KvmVm::default();
        assert_eq!(
            vm.commit_memory_region(KvmMemorySlot {
                id: 0,
                base_gfn: 0,
                npages: 0,
                flags: 0,
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn vcpu_reset_sets_reset_vector_state() {
        let mut vcpu = KvmVcpu::new(1, lm_policy());
        vcpu.arch.regs.rax = 99;
        kvm_vcpu_reset(&mut vcpu, false);
        assert_eq!(vcpu.arch.regs.rip, 0xfff0);
        assert_eq!(vcpu.arch.sregs.cs.selector, 0xf000);
        assert_eq!(vcpu.arch.regs.rax, 0);
    }
}
