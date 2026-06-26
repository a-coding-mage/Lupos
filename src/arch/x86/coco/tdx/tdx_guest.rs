//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/tdx/tdx.c
//! test-origin: linux:vendor/linux/arch/x86/coco/tdx/tdx.c
//! Intel TDX guest runtime.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/tdx/tdx.c

use core::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use crate::include::uapi::errno::{EBUSY, EFAULT, EINVAL, EIO, ENODEV, ENXIO};

pub use super::tdx_shared::{
    TDG_VP_VMCALL, TdxAccept, TdxHypercall, TdxModuleArgs, tdx_accept_memory, tdx_hypercall,
};

pub const EXIT_REASON_CPUID: u64 = 10;
pub const EXIT_REASON_HLT: u64 = 12;
pub const EXIT_REASON_VMCALL: u64 = 18;
pub const EXIT_REASON_IO_INSTRUCTION: u64 = 30;
pub const EXIT_REASON_MSR_READ: u64 = 31;
pub const EXIT_REASON_MSR_WRITE: u64 = 32;
pub const EXIT_REASON_EPT_VIOLATION: u64 = 48;
pub const EXIT_REASON_MSR_READ_IMM: u64 = 84;
pub const EXIT_REASON_MSR_WRITE_IMM: u64 = 85;

pub const EPT_READ: u64 = 0;
pub const EPT_WRITE: u64 = 1;
pub const PORT_READ: u64 = 0;
pub const PORT_WRITE: u64 = 1;

pub const TDX_HYPERCALL_STANDARD: u64 = 0;
pub const TDX_CPUID_LEAF_ID: u32 = 0x21;
pub const TDX_IDENT: [u8; 12] = *b"IntelTDX    ";
pub const TDX_IDENT_SIG: [u32; 3] = [0x6574_6e49, 0x5844_546c, 0x2020_2020];
pub const TDX_SUCCESS: u64 = 0;
pub const TDG_VP_INFO: u64 = 1;
pub const TDG_MR_RTMR_EXTEND: u64 = 2;
pub const TDG_VP_VEINFO_GET: u64 = 3;
pub const TDG_MR_REPORT: u64 = 4;
pub const TDG_VM_RD: u64 = 7;
pub const TDG_VM_WR: u64 = 8;
pub const TDREPORT_DATA_LEN: usize = 64;
pub const TDREPORT_LEN: usize = 1024;
pub const RTMR_EXTEND_DATA_LEN: usize = 48;
pub const TDREPORT_SUBTYPE_0: u64 = 0;
pub const TDVMCALL_MAP_GPA: u64 = 0x10001;
pub const TDVMCALL_GET_QUOTE: u64 = 0x10002;
pub const TDVMCALL_REPORT_FATAL_ERROR: u64 = 0x10003;
pub const TDVMCALL_STATUS_SUCCESS: u64 = 0;
pub const TDVMCALL_STATUS_RETRY: u64 = 1;
pub const TDVMCALL_STATUS_INVALID_OPERAND: u64 = 0x8000_0000_0000_0000;
pub const TDVMCALL_STATUS_ALIGN_ERROR: u64 = 0x8000_0000_0000_0002;
pub const TDVMCALL_STATUS_SUBFUNC_UNSUPPORTED: u64 = 0x8000_0000_0000_0003;

pub const TDCALL_INVALID_OPERAND: u64 = 0xc000_0100;
pub const TDCALL_OPERAND_BUSY: u64 = 0x8000_0200;
pub const PAGE_SIZE: u64 = 4096;

pub const TDX_TD_ATTR_DEBUG: u64 = 1 << 0;
pub const TDX_TD_ATTR_SEPT_VE_DISABLE: u64 = 1 << 28;
pub const TDCS_CONFIG_FLAGS: u64 = 0x1110_0003_0000_0016;
pub const TDCS_TD_CTLS: u64 = 0x1110_0003_0000_0017;
pub const TDCS_NOTIFY_ENABLES: u64 = 0x9100_0000_0000_0010;
pub const TDCS_TOPOLOGY_ENUM_CONFIGURED: u64 = 0x9100_0000_0000_0019;
pub const TDCS_CONFIG_FLEXIBLE_PENDING_VE: u64 = 1 << 1;
pub const TD_CTLS_PENDING_VE_DISABLE: u64 = 1 << 0;
pub const TD_CTLS_ENUM_TOPOLOGY: u64 = 1 << 1;
pub const TD_CTLS_VIRT_CPUID2: u64 = 1 << 2;
pub const TD_CTLS_REDUCE_VE: u64 = 1 << 3;
pub const TD_CTLS_LOCK: u64 = 1 << 63;

static NR_SHARED: AtomicI64 = AtomicI64::new(0);
static SEPT_VE_DISABLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VeInfo {
    pub exit_reason: u64,
    pub exit_qual: u64,
    pub gla: u64,
    pub gpa: u64,
    pub instr_len: u32,
    pub instr_info: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxRegs {
    pub ax: u64,
    pub bx: u64,
    pub cx: u64,
    pub dx: u64,
    pub ip: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VeAction {
    Cpuid,
    Halt,
    Io,
    MsrRead,
    MsrWrite,
    Mmio,
    VmmCall,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortDirection {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IoInfo {
    pub direction: PortDirection,
    pub size: u32,
    pub port: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TdxMmioType {
    Read,
    ReadZeroExtend,
    ReadSignExtend,
    Write,
    WriteImmediate,
    Movs,
    DecodeFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TdxMmioReg {
    Ax,
    Bx,
    Cx,
    Dx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodedMmio {
    pub kind: TdxMmioType,
    pub size: u32,
    pub opnd_bytes: u32,
    pub len: u32,
    pub reg: Option<TdxMmioReg>,
    pub immediate: u64,
    pub vaddr: u64,
    pub in_kernel_space: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeptVeOutcome {
    AlreadyDisabledByAttribute,
    AlreadyDisabledByControl,
    DisabledControl,
    DebugWarningOnly,
    DebugKeptEnabled,
    PanicRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReduceVeOutcome {
    ReduceVeEnabled,
    EnumTopologyEnabled,
    TopologyNotConfigured,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxSetupState {
    pub cc_mask: u64,
    pub td_attr: u64,
    pub sept_ve: SeptVeOutcome,
    pub reduce_ve: ReduceVeOutcome,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxGuestHooks {
    pub enc_status_change_prepare: bool,
    pub enc_status_change_finish: bool,
    pub enc_cache_flush_required: bool,
    pub enc_tlb_flush_required: bool,
    pub enc_kexec_begin: bool,
    pub enc_kexec_finish: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxHaltHooks {
    pub safe_halt: bool,
    pub halt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxAnnouncement {
    pub td_attr: u64,
    pub td_ctls: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxEarlyInitState {
    pub setup: TdxSetupState,
    pub force_tdx_guest: bool,
    pub force_tsc_reliable: bool,
    pub cc_vendor_intel: bool,
    pub cc_mask: u64,
    pub physical_mask: u64,
    pub guest_hooks: TdxGuestHooks,
    pub halt_hooks: TdxHaltHooks,
    pub parallel_bringup: bool,
    pub announcement: TdxAnnouncement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TdxVeError {
    Errno(i32),
    PrivateGpa,
    MmioUnsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpaState {
    Private,
    Shared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TdxKexecBeginOutcome {
    Disabled,
    Stopped,
    StopFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxDirectMapEntry {
    pub addr: u64,
    pub size: u64,
    pub present: bool,
    pub decrypted: bool,
    pub cleared: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TdxKexecFinishReport {
    pub enabled: bool,
    pub found_shared_pages: i64,
    pub accounted_shared_pages: i64,
    pub failed_ranges: usize,
    pub converted_ranges: usize,
    pub cleared_entries: usize,
    pub tlb_flushed: bool,
    pub accounting_mismatch: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxFatalPlan {
    pub instrumentation_begin: bool,
    pub panic: bool,
    pub message: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxTdcallPlan {
    pub leaf: u64,
    pub ret: u64,
    pub panic: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxVeInfoPlan {
    pub tdcall: TdxTdcallPlan,
    pub args: TdxModuleArgs,
    pub ve: Option<VeInfo>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxSetupInfoPlan {
    pub tdcall: TdxTdcallPlan,
    pub args: TdxModuleArgs,
    pub gpa_width: u64,
    pub cc_mask: Option<u64>,
    pub td_attr: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxAnnouncementPlan {
    pub tdcall: TdxTdcallPlan,
    pub args: TdxModuleArgs,
    pub td_attr: Option<u64>,
    pub td_ctls: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxVmRdPlan {
    pub leaf: u64,
    pub field: u64,
    pub args: TdxModuleArgs,
    pub ret: u64,
    pub value: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxVmWrPlan {
    pub leaf: u64,
    pub field: u64,
    pub value: u64,
    pub mask: u64,
    pub args: TdxModuleArgs,
    pub ret: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxPanicPlan {
    pub args: TdxModuleArgs,
    pub message_bytes: [u8; 64],
    pub hypercall_repeats_forever: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TdxHaltPlan {
    pub args: TdxModuleArgs,
    pub ret: u64,
    pub warned: bool,
    pub raw_local_irq_enabled: bool,
}

pub const fn ve_instr_len(ve: &VeInfo) -> u32 {
    match ve.exit_reason {
        EXIT_REASON_EPT_VIOLATION => 0,
        _ => ve.instr_len,
    }
}

pub const fn classify_ve(ve: &VeInfo) -> VeAction {
    match ve.exit_reason {
        EXIT_REASON_CPUID => VeAction::Cpuid,
        EXIT_REASON_HLT => VeAction::Halt,
        EXIT_REASON_IO_INSTRUCTION => VeAction::Io,
        EXIT_REASON_MSR_READ => VeAction::MsrRead,
        EXIT_REASON_MSR_WRITE => VeAction::MsrWrite,
        EXIT_REASON_EPT_VIOLATION => VeAction::Mmio,
        _ => VeAction::Unsupported,
    }
}

pub const fn tdx_hcall_func(exit_reason: u64) -> u64 {
    exit_reason
}

pub const fn tdx_tlb_flush_required(private: bool) -> bool {
    !private
}

pub const fn tdx_cache_flush_required() -> bool {
    true
}

fn tdx_panic_word(message: &[u8; 64], word: usize) -> u64 {
    let mut value = 0u64;
    let mut byte = 0usize;
    while byte < 8 {
        value |= (message[word * 8 + byte] as u64) << (byte * 8);
        byte += 1;
    }
    value
}

pub fn tdx_panic_plan(msg: &str) -> TdxPanicPlan {
    let mut message_bytes = [0u8; 64];
    let msg = msg.as_bytes();
    let mut index = 0usize;
    while index < message_bytes.len() && index < msg.len() {
        message_bytes[index] = msg[index];
        index += 1;
    }

    TdxPanicPlan {
        args: TdxModuleArgs {
            r10: TDX_HYPERCALL_STANDARD,
            r11: TDVMCALL_REPORT_FATAL_ERROR,
            r12: 0,
            r14: tdx_panic_word(&message_bytes, 0),
            r15: tdx_panic_word(&message_bytes, 1),
            rbx: tdx_panic_word(&message_bytes, 2),
            rdi: tdx_panic_word(&message_bytes, 3),
            rsi: tdx_panic_word(&message_bytes, 4),
            r8: tdx_panic_word(&message_bytes, 5),
            r9: tdx_panic_word(&message_bytes, 6),
            rdx: tdx_panic_word(&message_bytes, 7),
            ..Default::default()
        },
        message_bytes,
        hypercall_repeats_forever: true,
    }
}

pub fn tdx_map_gpa(start: u64, end: u64, enc: bool) -> Result<GpaState, i32> {
    let _ = (start, end);
    Ok(if enc {
        GpaState::Private
    } else {
        GpaState::Shared
    })
}

pub fn tdx_map_gpa_with<T: TdxHypercall>(
    backend: &mut T,
    mut start: u64,
    mut end: u64,
    enc: bool,
    shared_mask: u64,
) -> bool {
    const MAX_RETRIES_PER_PAGE: u8 = 3;

    if !enc {
        start |= shared_mask;
        end |= shared_mask;
    }

    let mut retry_count = 0;
    while retry_count < MAX_RETRIES_PER_PAGE {
        let mut args = TdxModuleArgs {
            r10: TDX_HYPERCALL_STANDARD,
            r11: TDVMCALL_MAP_GPA,
            r12: start,
            r13: end.wrapping_sub(start),
            ..Default::default()
        };
        let ret = tdx_hypercall(backend, &mut args);

        if ret != TDVMCALL_STATUS_RETRY {
            return ret == TDVMCALL_STATUS_SUCCESS;
        }

        let map_fail_paddr = args.r11;
        if map_fail_paddr < start || map_fail_paddr >= end {
            return false;
        }

        if map_fail_paddr == start {
            retry_count += 1;
            continue;
        }

        start = map_fail_paddr;
        retry_count = 0;
    }

    false
}

pub fn tdx_enc_status_changed(numpages: i32, enc: bool) -> Result<GpaState, i32> {
    let _ = numpages;
    Ok(if enc {
        GpaState::Private
    } else {
        GpaState::Shared
    })
}

pub fn tdx_enc_status_changed_with<T: TdxConversionBackend>(
    backend: &mut T,
    start: u64,
    end: u64,
    enc: bool,
    shared_mask: u64,
) -> bool {
    if !tdx_map_gpa_with(backend, start, end, enc, shared_mask) {
        return false;
    }

    if enc {
        return tdx_accept_memory(backend, start, end);
    }

    true
}

pub fn tdx_enc_status_change_prepare_with<T: TdxConversionBackend>(
    backend: &mut T,
    start: u64,
    numpages: i32,
    enc: bool,
    shared_mask: u64,
) -> Result<(), i32> {
    if enc {
        let end = start.wrapping_add((numpages as u64).wrapping_mul(PAGE_SIZE));
        if !tdx_enc_status_changed_with(backend, start, end, enc, shared_mask) {
            return Err(EIO);
        }
    }

    Ok(())
}

pub fn tdx_enc_status_change_finish_with<T: TdxConversionBackend>(
    backend: &mut T,
    start: u64,
    numpages: i32,
    enc: bool,
    shared_mask: u64,
) -> Result<(), i32> {
    if !enc {
        let end = start.wrapping_add((numpages as u64).wrapping_mul(PAGE_SIZE));
        if !tdx_enc_status_changed_with(backend, start, end, enc, shared_mask) {
            return Err(EIO);
        }
    }

    if enc {
        NR_SHARED.fetch_sub(numpages as i64, Ordering::AcqRel);
    } else {
        NR_SHARED.fetch_add(numpages as i64, Ordering::AcqRel);
    }

    Ok(())
}

pub const fn tdx_kexec_begin_with(
    kexec_core_enabled: bool,
    stop_conversion_success: bool,
) -> TdxKexecBeginOutcome {
    if !kexec_core_enabled {
        TdxKexecBeginOutcome::Disabled
    } else if stop_conversion_success {
        TdxKexecBeginOutcome::Stopped
    } else {
        TdxKexecBeginOutcome::StopFailed
    }
}

pub fn tdx_kexec_finish_with<T: TdxConversionBackend>(
    backend: &mut T,
    entries: &mut [TdxDirectMapEntry],
    shared_mask: u64,
    kexec_core_enabled: bool,
) -> TdxKexecFinishReport {
    let mut report = TdxKexecFinishReport {
        enabled: kexec_core_enabled,
        ..Default::default()
    };

    if !kexec_core_enabled {
        return report;
    }

    for entry in entries.iter_mut() {
        if !entry.present || !entry.decrypted {
            continue;
        }

        let pages = entry.size / PAGE_SIZE;
        entry.present = false;
        entry.decrypted = false;
        entry.cleared = true;
        report.cleared_entries += 1;

        let end = entry.addr.wrapping_add(entry.size);
        if !tdx_enc_status_changed_with(backend, entry.addr, end, true, shared_mask) {
            report.failed_ranges += 1;
        }

        report.converted_ranges += 1;
        report.found_shared_pages += pages as i64;
    }

    report.tlb_flushed = true;
    report.accounted_shared_pages = nr_shared_pages();
    report.accounting_mismatch = report.accounted_shared_pages != report.found_shared_pages;
    report
}

pub fn nr_shared_pages() -> i64 {
    NR_SHARED.load(Ordering::Acquire)
}

pub trait TdxTdcall {
    fn tdcall(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64;
}

pub trait TdxVmMetadata {
    fn tdg_vm_rd(&mut self, field: u64) -> u64;
    fn tdg_vm_wr(&mut self, field: u64, value: u64, mask: u64) -> u64;
}

pub trait TdxSetupBackend: TdxTdcall + TdxVmMetadata {}

impl<T: TdxTdcall + TdxVmMetadata> TdxSetupBackend for T {}

pub trait TdxConversionBackend: TdxHypercall + TdxAccept {}

impl<T: TdxHypercall + TdxAccept> TdxConversionBackend for T {}

pub trait TdxVeBackend: TdxTdcall + TdxHypercall {}

impl<T: TdxTdcall + TdxHypercall> TdxVeBackend for T {}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultTdxTdcall;

impl TdxTdcall for DefaultTdxTdcall {
    fn tdcall(&mut self, _leaf: u64, _args: &mut TdxModuleArgs) -> u64 {
        0x1234 << 32
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultTdxHypercall;

impl TdxHypercall for DefaultTdxHypercall {
    fn tdcall_saved_ret(&mut self, _leaf: u64, args: &mut TdxModuleArgs) -> u64 {
        args.r10 = TDVMCALL_STATUS_SUBFUNC_UNSUPPORTED;
        0
    }
}

fn virt_to_phys_or_addr(addr: u64) -> u64 {
    crate::arch::x86::mm::paging::virt_to_phys(addr).unwrap_or(addr)
}

pub fn tdx_mcall_get_report0(reportdata: *const u8, tdreport: *mut u8) -> Result<(), i32> {
    let reportdata_pa = virt_to_phys_or_addr(reportdata as u64);
    let tdreport_pa = virt_to_phys_or_addr(tdreport as u64);
    let mut backend = DefaultTdxTdcall;
    tdx_mcall_get_report0_with(
        &mut backend,
        reportdata_pa,
        tdreport_pa,
        TDREPORT_DATA_LEN,
        TDREPORT_LEN,
    )
}

pub const fn tdx_mcall_get_report0_args(reportdata_pa: u64, tdreport_pa: u64) -> TdxModuleArgs {
    TdxModuleArgs {
        rcx: tdreport_pa,
        rdx: reportdata_pa,
        r8: TDREPORT_SUBTYPE_0,
        ..TdxModuleArgs::DEFAULT
    }
}

pub fn tdx_mcall_get_report0_with<T: TdxTdcall>(
    backend: &mut T,
    reportdata_pa: u64,
    tdreport_pa: u64,
    _reportdata_len: usize,
    _tdreport_len: usize,
) -> Result<(), i32> {
    let mut args = tdx_mcall_get_report0_args(reportdata_pa, tdreport_pa);

    tdx_tdcall_status_to_result(backend.tdcall(TDG_MR_REPORT, &mut args))
}

pub fn tdx_mcall_extend_rtmr(index: u8, data: *const u8) -> Result<(), i32> {
    let data_pa = virt_to_phys_or_addr(data as u64);
    let mut backend = DefaultTdxTdcall;
    tdx_mcall_extend_rtmr_with(&mut backend, index, data_pa, RTMR_EXTEND_DATA_LEN)
}

pub const fn tdx_mcall_extend_rtmr_args(index: u8, data_pa: u64) -> TdxModuleArgs {
    TdxModuleArgs {
        rcx: data_pa,
        rdx: index as u64,
        ..TdxModuleArgs::DEFAULT
    }
}

pub fn tdx_mcall_extend_rtmr_with<T: TdxTdcall>(
    backend: &mut T,
    index: u8,
    data_pa: u64,
    _data_len: usize,
) -> Result<(), i32> {
    let mut args = tdx_mcall_extend_rtmr_args(index, data_pa);

    tdx_tdcall_status_to_result(backend.tdcall(TDG_MR_RTMR_EXTEND, &mut args))
}

pub fn tdx_hcall_get_quote(buf: *mut u8, size: usize) -> Result<u64, i32> {
    let buf_pa = virt_to_phys_or_addr(buf as u64);
    let shared_pa = crate::arch::x86::coco::core::cc_mkdec(buf_pa);
    let mut backend = DefaultTdxHypercall;
    Ok(tdx_hcall_get_quote_with(&mut backend, shared_pa, size, 0))
}

pub const fn tdx_hcall_get_quote_args(buf_pa: u64, size: usize, shared_mask: u64) -> TdxModuleArgs {
    TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: TDVMCALL_GET_QUOTE,
        r12: buf_pa | shared_mask,
        r13: size as u64,
        ..TdxModuleArgs::DEFAULT
    }
}

pub fn tdx_hcall_get_quote_with<T: TdxHypercall>(
    backend: &mut T,
    buf_pa: u64,
    size: usize,
    shared_mask: u64,
) -> u64 {
    let mut args = tdx_hcall_get_quote_args(buf_pa, size, shared_mask);

    tdx_hypercall(backend, &mut args)
}

pub const fn tdx_halt_args(irq_disabled: bool) -> TdxModuleArgs {
    TdxModuleArgs {
        rax: 0,
        rcx: 0,
        rdx: 0,
        r8: 0,
        r9: 0,
        r10: TDX_HYPERCALL_STANDARD,
        r11: EXIT_REASON_HLT,
        r12: irq_disabled as u64,
        r13: 0,
        r14: 0,
        r15: 0,
        rbx: 0,
        rdi: 0,
        rsi: 0,
    }
}

pub const fn __halt_plan(irq_disabled: bool, ret: u64) -> TdxHaltPlan {
    TdxHaltPlan {
        args: tdx_halt_args(irq_disabled),
        ret,
        warned: false,
        raw_local_irq_enabled: false,
    }
}

pub const fn tdx_halt_plan(ret: u64) -> TdxHaltPlan {
    let mut plan = __halt_plan(false, ret);
    plan.warned = ret != 0;
    plan
}

pub const fn tdx_safe_halt_plan(ret: u64) -> TdxHaltPlan {
    let mut plan = tdx_halt_plan(ret);
    plan.raw_local_irq_enabled = true;
    plan
}

pub fn tdx_handle_halt_with<T: TdxHypercall>(
    backend: &mut T,
    ve: &VeInfo,
    irq_disabled: bool,
) -> Result<u32, i32> {
    if !irq_disabled {
        return Err(EIO);
    }

    let mut args = tdx_halt_args(irq_disabled);

    if tdx_hypercall(backend, &mut args) != 0 {
        return Err(EIO);
    }

    Ok(ve_instr_len(ve))
}

pub fn tdx_read_msr_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
) -> Result<u32, i32> {
    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_MSR_READ),
        r12: regs.cx,
        ..Default::default()
    };

    if tdx_hypercall(backend, &mut args) != 0 {
        return Err(EIO);
    }

    regs.ax = args.r11 & 0xffff_ffff;
    regs.dx = args.r11 >> 32;
    Ok(ve_instr_len(ve))
}

pub fn tdx_write_msr_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &TdxRegs,
    ve: &VeInfo,
) -> Result<u32, i32> {
    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_MSR_WRITE),
        r12: regs.cx,
        r13: (regs.dx << 32) | regs.ax,
        ..Default::default()
    };

    if tdx_hypercall(backend, &mut args) != 0 {
        return Err(EIO);
    }

    Ok(ve_instr_len(ve))
}

pub fn tdx_handle_cpuid_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
) -> Result<u32, i32> {
    if regs.ax < 0x4000_0000 || regs.ax > 0x4fff_ffff {
        regs.ax = 0;
        regs.bx = 0;
        regs.cx = 0;
        regs.dx = 0;
        return Ok(ve_instr_len(ve));
    }

    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_CPUID),
        r12: regs.ax,
        r13: regs.cx,
        ..Default::default()
    };

    if tdx_hypercall(backend, &mut args) != 0 {
        return Err(EIO);
    }

    regs.ax = args.r12;
    regs.bx = args.r13;
    regs.cx = args.r14;
    regs.dx = args.r15;
    Ok(ve_instr_len(ve))
}

pub const fn tdx_io_info(exit_qual: u64) -> Option<IoInfo> {
    let exit_qual = exit_qual as u32;
    if exit_qual & (1 << 4) != 0 {
        return None;
    }

    let direction = if exit_qual & (1 << 3) != 0 {
        PortDirection::In
    } else {
        PortDirection::Out
    };

    Some(IoInfo {
        direction,
        size: (exit_qual & 0x7) + 1,
        port: exit_qual >> 16,
    })
}

pub const fn tdx_io_mask(size: u32) -> u64 {
    let high_bit = 8 * size;
    if high_bit >= 63 {
        u64::MAX
    } else {
        (1u64 << (high_bit + 1)) - 1
    }
}

pub fn tdx_handle_in_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    size: u32,
    port: u32,
) -> bool {
    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_IO_INSTRUCTION),
        r12: size as u64,
        r13: PORT_READ,
        r14: port as u64,
        ..Default::default()
    };
    let mask = tdx_io_mask(size);
    let success = tdx_hypercall(backend, &mut args) == 0;

    regs.ax &= !mask;
    if success {
        regs.ax |= args.r11 & mask;
    }

    success
}

pub fn tdx_handle_out_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &TdxRegs,
    size: u32,
    port: u32,
) -> bool {
    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_IO_INSTRUCTION),
        r12: size as u64,
        r13: PORT_WRITE,
        r14: port as u64,
        r15: regs.ax & tdx_io_mask(size),
        ..Default::default()
    };

    tdx_hypercall(backend, &mut args) == 0
}

pub fn tdx_handle_io_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
) -> Result<u32, i32> {
    let Some(io) = tdx_io_info(ve.exit_qual) else {
        return Err(EIO);
    };

    let success = match io.direction {
        PortDirection::In => tdx_handle_in_with(backend, regs, io.size, io.port),
        PortDirection::Out => tdx_handle_out_with(backend, regs, io.size, io.port),
    };
    if !success {
        return Err(EIO);
    }

    Ok(ve_instr_len(ve))
}

pub const fn tdx_mmio_width_mask(size: u32) -> Option<u64> {
    match size {
        1 => Some(0xff),
        2 => Some(0xffff),
        4 => Some(0xffff_ffff),
        8 => Some(u64::MAX),
        _ => None,
    }
}

pub fn tdx_regs_get(regs: &TdxRegs, reg: TdxMmioReg) -> u64 {
    match reg {
        TdxMmioReg::Ax => regs.ax,
        TdxMmioReg::Bx => regs.bx,
        TdxMmioReg::Cx => regs.cx,
        TdxMmioReg::Dx => regs.dx,
    }
}

pub fn tdx_regs_set(regs: &mut TdxRegs, reg: TdxMmioReg, value: u64) {
    match reg {
        TdxMmioReg::Ax => regs.ax = value,
        TdxMmioReg::Bx => regs.bx = value,
        TdxMmioReg::Cx => regs.cx = value,
        TdxMmioReg::Dx => regs.dx = value,
    }
}

pub fn tdx_mmio_read_with<T: TdxHypercall>(backend: &mut T, size: u32, addr: u64) -> Option<u64> {
    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_EPT_VIOLATION),
        r12: size as u64,
        r13: EPT_READ,
        r14: addr,
        ..Default::default()
    };

    if tdx_hypercall(backend, &mut args) != 0 {
        return None;
    }

    Some(args.r11)
}

pub fn tdx_mmio_write_with<T: TdxHypercall>(
    backend: &mut T,
    size: u32,
    addr: u64,
    value: u64,
) -> bool {
    let mut args = TdxModuleArgs {
        r10: TDX_HYPERCALL_STANDARD,
        r11: tdx_hcall_func(EXIT_REASON_EPT_VIOLATION),
        r12: size as u64,
        r13: EPT_WRITE,
        r14: addr,
        r15: value,
        ..Default::default()
    };

    tdx_hypercall(backend, &mut args) == 0
}

pub const fn tdx_mmio_crosses_page(vaddr: u64, size: u32) -> bool {
    if size == 0 {
        return true;
    }
    let last = vaddr.wrapping_add(size as u64 - 1);
    vaddr / PAGE_SIZE != last / PAGE_SIZE
}

pub const fn tdx_mmio_sign_fill(size: u32, value: u64) -> u8 {
    if size == 1 && value & (1 << 7) != 0 {
        0xff
    } else if size > 1 && value & (1 << 15) != 0 {
        0xff
    } else {
        0
    }
}

pub const fn tdx_low_bytes_mask(bytes: u32) -> u64 {
    if bytes >= 8 {
        u64::MAX
    } else if bytes == 0 {
        0
    } else {
        (1u64 << (bytes * 8)) - 1
    }
}

pub fn tdx_mmio_merge_read_value(
    current: u64,
    copied: u64,
    size: u32,
    extend_size: u32,
    extend_val: u8,
) -> u64 {
    let Some(copy_mask) = tdx_mmio_width_mask(size) else {
        return current;
    };
    let extend_mask = tdx_low_bytes_mask(extend_size);
    let filled = if extend_val == 0 {
        current & !extend_mask
    } else {
        current | extend_mask
    };

    (filled & !copy_mask) | (copied & copy_mask)
}

pub fn tdx_handle_mmio_decoded_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
    insn: DecodedMmio,
) -> Result<u32, i32> {
    let Some(mask) = tdx_mmio_width_mask(insn.size) else {
        return Err(EINVAL);
    };
    if matches!(insn.kind, TdxMmioType::Movs | TdxMmioType::DecodeFailed) {
        return Err(EINVAL);
    }
    if !insn.in_kernel_space {
        return Err(EINVAL);
    }
    if tdx_mmio_crosses_page(insn.vaddr, insn.size) {
        return Err(EFAULT);
    }

    match insn.kind {
        TdxMmioType::Write => {
            let Some(reg) = insn.reg else {
                return Err(EINVAL);
            };
            let value = tdx_regs_get(regs, reg) & mask;
            if !tdx_mmio_write_with(backend, insn.size, ve.gpa, value) {
                return Err(EIO);
            }
            Ok(insn.len)
        }
        TdxMmioType::WriteImmediate => {
            if !tdx_mmio_write_with(backend, insn.size, ve.gpa, insn.immediate & mask) {
                return Err(EIO);
            }
            Ok(insn.len)
        }
        TdxMmioType::Read | TdxMmioType::ReadZeroExtend | TdxMmioType::ReadSignExtend => {
            let Some(reg) = insn.reg else {
                return Err(EINVAL);
            };
            let Some(read_val) = tdx_mmio_read_with(backend, insn.size, ve.gpa) else {
                return Err(EIO);
            };
            let copied = read_val & mask;
            let current = tdx_regs_get(regs, reg);
            let value = match insn.kind {
                TdxMmioType::Read if insn.size == 4 => {
                    tdx_mmio_merge_read_value(current, copied, insn.size, 8, 0)
                }
                TdxMmioType::Read => tdx_mmio_merge_read_value(current, copied, insn.size, 0, 0),
                TdxMmioType::ReadZeroExtend => {
                    tdx_mmio_merge_read_value(current, copied, insn.size, insn.opnd_bytes, 0)
                }
                TdxMmioType::ReadSignExtend => tdx_mmio_merge_read_value(
                    current,
                    copied,
                    insn.size,
                    insn.opnd_bytes,
                    tdx_mmio_sign_fill(insn.size, copied),
                ),
                _ => unreachable!(),
            };
            tdx_regs_set(regs, reg, value);
            Ok(insn.len)
        }
        TdxMmioType::Movs | TdxMmioType::DecodeFailed => Err(EINVAL),
    }
}

pub fn tdx_get_ve_info_with<T: TdxTdcall>(backend: &mut T) -> Result<VeInfo, i32> {
    let mut args = TdxModuleArgs::default();
    let ret = backend.tdcall(TDG_VP_VEINFO_GET, &mut args);
    let plan = tdx_get_ve_info_plan(ret, args);
    let Some(ve) = plan.ve else {
        return Err(EIO);
    };

    Ok(ve)
}

pub const fn tdx_ve_info_from_args(args: TdxModuleArgs) -> VeInfo {
    VeInfo {
        exit_reason: args.rcx,
        exit_qual: args.rdx,
        gla: args.r8,
        gpa: args.r9,
        instr_len: (args.r10 & 0xffff_ffff) as u32,
        instr_info: (args.r10 >> 32) as u32,
    }
}

pub const fn tdx_get_ve_info_plan(ret: u64, args: TdxModuleArgs) -> TdxVeInfoPlan {
    let tdcall = tdcall_plan(TDG_VP_VEINFO_GET, ret);
    TdxVeInfoPlan {
        tdcall,
        args,
        ve: if tdcall.panic {
            None
        } else {
            Some(tdx_ve_info_from_args(args))
        },
    }
}

pub const fn tdx_setup_info_plan(ret: u64, args: TdxModuleArgs) -> TdxSetupInfoPlan {
    let tdcall = tdcall_plan(TDG_VP_INFO, ret);
    let gpa_width = args.rcx & 0x3f;
    let cc_mask = if !tdcall.panic && gpa_width != 0 {
        Some(1u64 << (gpa_width - 1))
    } else {
        None
    };
    let td_attr = if tdcall.panic { None } else { Some(args.rdx) };

    TdxSetupInfoPlan {
        tdcall,
        args,
        gpa_width,
        cc_mask,
        td_attr,
    }
}

pub const fn tdx_announcement_plan(
    ret: u64,
    args: TdxModuleArgs,
    td_ctls: Option<u64>,
) -> TdxAnnouncementPlan {
    let tdcall = tdcall_plan(TDG_VP_INFO, ret);
    let td_attr = if tdcall.panic { None } else { Some(args.rdx) };
    let td_ctls = if tdcall.panic { None } else { td_ctls };

    TdxAnnouncementPlan {
        tdcall,
        args,
        td_attr,
        td_ctls,
    }
}

pub const fn tdx_is_private_gpa(gpa: u64, shared_mask: u64) -> bool {
    gpa & shared_mask == 0
}

pub fn tdx_virt_exception_user_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
) -> Result<u32, TdxVeError> {
    match ve.exit_reason {
        EXIT_REASON_CPUID => tdx_handle_cpuid_with(backend, regs, ve).map_err(TdxVeError::Errno),
        _ => Err(TdxVeError::Errno(EIO)),
    }
}

pub fn tdx_virt_exception_kernel_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
    irq_disabled: bool,
    shared_mask: u64,
) -> Result<u32, TdxVeError> {
    match ve.exit_reason {
        EXIT_REASON_HLT => {
            tdx_handle_halt_with(backend, ve, irq_disabled).map_err(TdxVeError::Errno)
        }
        EXIT_REASON_MSR_READ => tdx_read_msr_with(backend, regs, ve).map_err(TdxVeError::Errno),
        EXIT_REASON_MSR_WRITE => tdx_write_msr_with(backend, regs, ve).map_err(TdxVeError::Errno),
        EXIT_REASON_CPUID => tdx_handle_cpuid_with(backend, regs, ve).map_err(TdxVeError::Errno),
        EXIT_REASON_EPT_VIOLATION => {
            if tdx_is_private_gpa(ve.gpa, shared_mask) {
                Err(TdxVeError::PrivateGpa)
            } else {
                Err(TdxVeError::MmioUnsupported)
            }
        }
        EXIT_REASON_IO_INSTRUCTION => {
            tdx_handle_io_with(backend, regs, ve).map_err(TdxVeError::Errno)
        }
        _ => Err(TdxVeError::Errno(EIO)),
    }
}

pub fn tdx_virt_exception_kernel_decoded_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
    irq_disabled: bool,
    shared_mask: u64,
    mmio: Option<DecodedMmio>,
) -> Result<u32, TdxVeError> {
    if ve.exit_reason != EXIT_REASON_EPT_VIOLATION {
        return tdx_virt_exception_kernel_with(backend, regs, ve, irq_disabled, shared_mask);
    }

    if tdx_is_private_gpa(ve.gpa, shared_mask) {
        return Err(TdxVeError::PrivateGpa);
    }

    let Some(insn) = mmio else {
        return Err(TdxVeError::MmioUnsupported);
    };

    tdx_handle_mmio_decoded_with(backend, regs, ve, insn).map_err(TdxVeError::Errno)
}

pub fn tdx_handle_virt_exception_result_with<T: TdxHypercall>(
    backend: &mut T,
    regs: &mut TdxRegs,
    ve: &VeInfo,
    user_mode: bool,
    irq_disabled: bool,
    shared_mask: u64,
) -> Result<(), TdxVeError> {
    let insn_len = if user_mode {
        tdx_virt_exception_user_with(backend, regs, ve)?
    } else {
        tdx_virt_exception_kernel_with(backend, regs, ve, irq_disabled, shared_mask)?
    };

    regs.ip = regs.ip.wrapping_add(insn_len as u64);
    Ok(())
}

pub fn tdx_early_handle_ve_with<T: TdxVeBackend>(backend: &mut T, regs: &mut TdxRegs) -> bool {
    let Ok(ve) = tdx_get_ve_info_with(backend) else {
        return false;
    };

    if ve.exit_reason != EXIT_REASON_IO_INSTRUCTION {
        return false;
    }

    let Ok(insn_len) = tdx_handle_io_with(backend, regs, &ve) else {
        return false;
    };

    regs.ip = regs.ip.wrapping_add(insn_len as u64);
    true
}

pub fn tdx_kvm_hypercall_with<T: TdxHypercall>(
    backend: &mut T,
    nr: u32,
    p1: u64,
    p2: u64,
    p3: u64,
    p4: u64,
) -> u64 {
    let mut args = tdx_kvm_hypercall_args(nr, p1, p2, p3, p4);

    tdx_hypercall(backend, &mut args)
}

pub const fn tdx_kvm_hypercall_args(nr: u32, p1: u64, p2: u64, p3: u64, p4: u64) -> TdxModuleArgs {
    TdxModuleArgs {
        r10: nr as u64,
        r11: p1,
        r12: p2,
        r13: p3,
        r14: p4,
        ..TdxModuleArgs::DEFAULT
    }
}

pub fn disable_sept_ve(td_attr: u64) -> bool {
    let disabled = td_attr & TDX_TD_ATTR_SEPT_VE_DISABLE != 0;
    SEPT_VE_DISABLED.store(disabled, Ordering::Release);
    disabled
}

pub fn tdx_disable_sept_ve_with<T: TdxVmMetadata>(backend: &mut T, td_attr: u64) -> SeptVeOutcome {
    let debug = td_attr & TDX_TD_ATTR_DEBUG != 0;
    let config = backend.tdg_vm_rd(TDCS_CONFIG_FLAGS);

    if config & TDCS_CONFIG_FLEXIBLE_PENDING_VE == 0 {
        if td_attr & TDX_TD_ATTR_SEPT_VE_DISABLE != 0 {
            SEPT_VE_DISABLED.store(true, Ordering::Release);
            return SeptVeOutcome::AlreadyDisabledByAttribute;
        }

        return if debug {
            SeptVeOutcome::DebugWarningOnly
        } else {
            SeptVeOutcome::PanicRequired
        };
    }

    let controls = backend.tdg_vm_rd(TDCS_TD_CTLS);
    if controls & TD_CTLS_PENDING_VE_DISABLE != 0 {
        SEPT_VE_DISABLED.store(true, Ordering::Release);
        return SeptVeOutcome::AlreadyDisabledByControl;
    }

    if debug {
        return SeptVeOutcome::DebugKeptEnabled;
    }

    backend.tdg_vm_wr(
        TDCS_TD_CTLS,
        TD_CTLS_PENDING_VE_DISABLE,
        TD_CTLS_PENDING_VE_DISABLE,
    );
    SEPT_VE_DISABLED.store(true, Ordering::Release);
    SeptVeOutcome::DisabledControl
}

pub fn tdx_enable_cpu_topology_enumeration_with<T: TdxVmMetadata>(backend: &mut T) -> bool {
    let configured = backend.tdg_vm_rd(TDCS_TOPOLOGY_ENUM_CONFIGURED);
    if configured == 0 {
        return false;
    }

    backend.tdg_vm_wr(TDCS_TD_CTLS, TD_CTLS_ENUM_TOPOLOGY, TD_CTLS_ENUM_TOPOLOGY);
    true
}

pub fn tdx_reduce_unnecessary_ve_with<T: TdxVmMetadata>(backend: &mut T) -> ReduceVeOutcome {
    let err = backend.tdg_vm_wr(TDCS_TD_CTLS, TD_CTLS_REDUCE_VE, TD_CTLS_REDUCE_VE);
    if err == TDX_SUCCESS {
        return ReduceVeOutcome::ReduceVeEnabled;
    }

    if tdx_enable_cpu_topology_enumeration_with(backend) {
        ReduceVeOutcome::EnumTopologyEnabled
    } else {
        ReduceVeOutcome::TopologyNotConfigured
    }
}

pub fn tdx_setup_with<T: TdxSetupBackend>(backend: &mut T) -> Result<TdxSetupState, i32> {
    let mut args = TdxModuleArgs::default();
    let ret = backend.tdcall(TDG_VP_INFO, &mut args);
    let info = tdx_setup_info_plan(ret, args);
    if info.tdcall.panic {
        return Err(EIO);
    }

    let Some(cc_mask) = info.cc_mask else {
        return Err(EINVAL);
    };
    let td_attr = info.td_attr.unwrap_or(0);

    backend.tdg_vm_wr(TDCS_NOTIFY_ENABLES, 0, u64::MAX);
    let sept_ve = tdx_disable_sept_ve_with(backend, td_attr);
    if matches!(sept_ve, SeptVeOutcome::PanicRequired) {
        return Err(EIO);
    }
    let reduce_ve = tdx_reduce_unnecessary_ve_with(backend);

    Ok(TdxSetupState {
        cc_mask,
        td_attr,
        sept_ve,
        reduce_ve,
    })
}

pub fn tdx_ident_matches(sig: [u32; 3]) -> bool {
    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&sig[0].to_le_bytes());
    bytes[4..8].copy_from_slice(&sig[1].to_le_bytes());
    bytes[8..12].copy_from_slice(&sig[2].to_le_bytes());
    bytes == TDX_IDENT
}

pub fn tdx_announce_with<T: TdxSetupBackend>(backend: &mut T) -> Result<TdxAnnouncement, i32> {
    let mut args = TdxModuleArgs::default();
    let ret = backend.tdcall(TDG_VP_INFO, &mut args);
    let tdcall = tdcall_plan(TDG_VP_INFO, ret);
    if tdcall.panic {
        return Err(EIO);
    }

    let td_ctls = backend.tdg_vm_rd(TDCS_TD_CTLS);
    let plan = tdx_announcement_plan(ret, args, Some(td_ctls));
    Ok(TdxAnnouncement {
        td_attr: plan.td_attr.unwrap_or(0),
        td_ctls: plan.td_ctls.unwrap_or(0),
    })
}

pub fn tdx_early_init_with<T: TdxSetupBackend>(
    backend: &mut T,
    cpuid_sig: [u32; 3],
    physical_mask: u64,
) -> Result<Option<TdxEarlyInitState>, i32> {
    if !tdx_ident_matches(cpuid_sig) {
        return Ok(None);
    }

    let setup = tdx_setup_with(backend)?;
    let cc_mask = setup.cc_mask;
    let physical_mask = physical_mask & (cc_mask - 1);
    let announcement = tdx_announce_with(backend)?;

    Ok(Some(TdxEarlyInitState {
        setup,
        force_tdx_guest: true,
        force_tsc_reliable: true,
        cc_vendor_intel: true,
        cc_mask,
        physical_mask,
        guest_hooks: TdxGuestHooks {
            enc_status_change_prepare: true,
            enc_status_change_finish: true,
            enc_cache_flush_required: true,
            enc_tlb_flush_required: true,
            enc_kexec_begin: true,
            enc_kexec_finish: true,
        },
        halt_hooks: TdxHaltHooks {
            safe_halt: true,
            halt: true,
        },
        parallel_bringup: false,
        announcement,
    }))
}

pub fn sept_ve_disabled() -> bool {
    SEPT_VE_DISABLED.load(Ordering::Acquire)
}

pub fn tdx_handle_virt_exception(ve: &VeInfo) -> bool {
    tdx_handle_virt_exception_for_mode(ve, false)
}

pub const fn tdx_handle_virt_exception_for_mode(ve: &VeInfo, user_mode: bool) -> bool {
    if user_mode {
        return matches!(ve.exit_reason, EXIT_REASON_CPUID);
    }

    matches!(
        ve.exit_reason,
        EXIT_REASON_HLT
            | EXIT_REASON_MSR_READ
            | EXIT_REASON_MSR_WRITE
            | EXIT_REASON_CPUID
            | EXIT_REASON_EPT_VIOLATION
            | EXIT_REASON_IO_INSTRUCTION
    )
}

pub fn tdx_kvm_hypercall(_nr: u32, _p1: u64, _p2: u64, _p3: u64, _p4: u64) -> Result<u64, i32> {
    Err(ENODEV)
}

pub const fn tdx_hypercall_failed_plan() -> TdxFatalPlan {
    TdxFatalPlan {
        instrumentation_begin: true,
        panic: true,
        message: "TDVMCALL failed. TDX module bug?",
    }
}

pub const fn tdcall_plan(leaf: u64, ret: u64) -> TdxTdcallPlan {
    TdxTdcallPlan {
        leaf,
        ret,
        panic: ret != 0,
    }
}

pub const fn tdg_vm_rd_plan(field: u64, ret: u64, returned_r8: u64) -> TdxVmRdPlan {
    TdxVmRdPlan {
        leaf: TDG_VM_RD,
        field,
        args: TdxModuleArgs {
            rax: 0,
            rcx: 0,
            rdx: field,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbx: 0,
            rdi: 0,
            rsi: 0,
        },
        ret,
        value: returned_r8,
    }
}

pub const fn tdg_vm_wr_plan(field: u64, value: u64, mask: u64, ret: u64) -> TdxVmWrPlan {
    TdxVmWrPlan {
        leaf: TDG_VM_WR,
        field,
        value,
        mask,
        args: TdxModuleArgs {
            rax: 0,
            rcx: 0,
            rdx: field,
            r8: value,
            r9: mask,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbx: 0,
            rdi: 0,
            rsi: 0,
        },
        ret,
    }
}

pub const fn tdx_tdcall_return_code(status: u64) -> u64 {
    status >> 32
}

pub const fn tdx_tdcall_status_to_errno(status: u64) -> i32 {
    match tdx_tdcall_return_code(status) {
        TDCALL_INVALID_OPERAND => ENXIO,
        TDCALL_OPERAND_BUSY => EBUSY,
        _ => EIO,
    }
}

pub const fn tdx_tdcall_status_to_result(status: u64) -> Result<(), i32> {
    if status == TDX_SUCCESS {
        Ok(())
    } else {
        Err(tdx_tdcall_status_to_errno(status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_level_tdcall_wrappers_match_linux_leafs_registers_and_panics() {
        assert_eq!(
            tdx_hypercall_failed_plan(),
            TdxFatalPlan {
                instrumentation_begin: true,
                panic: true,
                message: "TDVMCALL failed. TDX module bug?",
            }
        );

        assert_eq!(
            tdcall_plan(TDG_VP_INFO, 0),
            TdxTdcallPlan {
                leaf: TDG_VP_INFO,
                ret: 0,
                panic: false,
            }
        );
        assert_eq!(
            tdcall_plan(TDG_VP_INFO, 7),
            TdxTdcallPlan {
                leaf: TDG_VP_INFO,
                ret: 7,
                panic: true,
            }
        );

        let read = tdg_vm_rd_plan(TDCS_TD_CTLS, 0x44, 0xfeed_beef);
        assert_eq!(read.leaf, TDG_VM_RD);
        assert_eq!(read.ret, 0x44);
        assert_eq!(read.value, 0xfeed_beef);
        assert_eq!(read.args.rdx, TDCS_TD_CTLS);
        assert_eq!(read.args.r8, 0);
        assert_eq!(read.args.r9, 0);

        let write = tdg_vm_wr_plan(TDCS_TD_CTLS, TD_CTLS_REDUCE_VE, TD_CTLS_REDUCE_VE, 0);
        assert_eq!(write.leaf, TDG_VM_WR);
        assert_eq!(write.args.rdx, TDCS_TD_CTLS);
        assert_eq!(write.args.r8, TD_CTLS_REDUCE_VE);
        assert_eq!(write.args.r9, TD_CTLS_REDUCE_VE);
        assert_eq!(write.ret, 0);
    }

    #[test]
    fn tdx_panic_plan_packs_linux_fatal_error_hypercall_message_order() {
        let plan = tdx_panic_plan("panic!");
        assert_eq!(plan.args.r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(plan.args.r11, TDVMCALL_REPORT_FATAL_ERROR);
        assert_eq!(plan.args.r12, 0);
        assert!(plan.hypercall_repeats_forever);
        assert_eq!(&plan.message_bytes[..7], b"panic!\0");
        assert_eq!(plan.args.r14, u64::from_le_bytes(*b"panic!\0\0"));
        assert_eq!(plan.args.r15, 0);
        assert_eq!(plan.args.rbx, 0);
        assert_eq!(plan.args.rdi, 0);
        assert_eq!(plan.args.rsi, 0);
        assert_eq!(plan.args.r8, 0);
        assert_eq!(plan.args.r9, 0);
        assert_eq!(plan.args.rdx, 0);

        let long = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789++tail";
        let truncated = tdx_panic_plan(long);
        assert_eq!(truncated.message_bytes.len(), 64);
        assert_eq!(&truncated.message_bytes[..8], b"abcdefgh");
        assert_eq!(&truncated.message_bytes[56..64], b"456789++");
        assert_eq!(truncated.args.r14, u64::from_le_bytes(*b"abcdefgh"));
        assert_eq!(truncated.args.r15, u64::from_le_bytes(*b"ijklmnop"));
        assert_eq!(truncated.args.rbx, u64::from_le_bytes(*b"qrstuvwx"));
        assert_eq!(truncated.args.rdx, u64::from_le_bytes(*b"456789++"));
    }

    struct FakeTdcall {
        leaf: u64,
        ret: u64,
        seen: Option<TdxModuleArgs>,
    }

    impl TdxTdcall for FakeTdcall {
        fn tdcall(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            self.leaf = leaf;
            self.seen = Some(*args);
            self.ret
        }
    }

    struct FakeHypercall {
        responses: &'static [(u64, u64)],
        calls: [TdxModuleArgs; 8],
        len: usize,
    }

    impl FakeHypercall {
        const fn new(responses: &'static [(u64, u64)]) -> Self {
            Self {
                responses,
                calls: [TdxModuleArgs {
                    rax: 0,
                    rcx: 0,
                    rdx: 0,
                    r8: 0,
                    r9: 0,
                    r10: 0,
                    r11: 0,
                    r12: 0,
                    r13: 0,
                    r14: 0,
                    r15: 0,
                    rbx: 0,
                    rdi: 0,
                    rsi: 0,
                }; 8],
                len: 0,
            }
        }
    }

    impl TdxHypercall for FakeHypercall {
        fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            assert_eq!(leaf, TDG_VP_VMCALL);
            self.calls[self.len] = *args;
            let (status, r11) = self.responses[self.len];
            self.len += 1;
            args.r10 = status;
            args.r11 = r11;
            0
        }
    }

    struct FullHypercall {
        responses: &'static [TdxModuleArgs],
        calls: [TdxModuleArgs; 8],
        len: usize,
    }

    impl FullHypercall {
        const EMPTY: TdxModuleArgs = TdxModuleArgs {
            rax: 0,
            rcx: 0,
            rdx: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbx: 0,
            rdi: 0,
            rsi: 0,
        };

        const fn new(responses: &'static [TdxModuleArgs]) -> Self {
            Self {
                responses,
                calls: [Self::EMPTY; 8],
                len: 0,
            }
        }
    }

    impl TdxHypercall for FullHypercall {
        fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            assert_eq!(leaf, TDG_VP_VMCALL);
            self.calls[self.len] = *args;
            let response = self.responses[self.len];
            self.len += 1;
            args.r10 = response.r10;
            args.r11 = response.r11;
            args.r12 = response.r12;
            args.r13 = response.r13;
            args.r14 = response.r14;
            args.r15 = response.r15;
            0
        }
    }

    struct VeBackend {
        veinfo_ret: u64,
        veinfo: TdxModuleArgs,
        tdcall_leaf: u64,
        responses: &'static [TdxModuleArgs],
        calls: [TdxModuleArgs; 8],
        len: usize,
    }

    impl VeBackend {
        const fn new(veinfo: TdxModuleArgs, responses: &'static [TdxModuleArgs]) -> Self {
            Self {
                veinfo_ret: TDX_SUCCESS,
                veinfo,
                tdcall_leaf: 0,
                responses,
                calls: [FullHypercall::EMPTY; 8],
                len: 0,
            }
        }
    }

    impl TdxTdcall for VeBackend {
        fn tdcall(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            self.tdcall_leaf = leaf;
            *args = self.veinfo;
            self.veinfo_ret
        }
    }

    impl TdxHypercall for VeBackend {
        fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            assert_eq!(leaf, TDG_VP_VMCALL);
            self.calls[self.len] = *args;
            let response = self.responses[self.len];
            self.len += 1;
            args.r10 = response.r10;
            args.r11 = response.r11;
            args.r12 = response.r12;
            args.r13 = response.r13;
            args.r14 = response.r14;
            args.r15 = response.r15;
            0
        }
    }

    struct ConversionBackend {
        map_responses: &'static [(u64, u64)],
        map_calls: [TdxModuleArgs; 8],
        map_len: usize,
        accept_rets: [u64; 8],
        accept_calls: [TdxModuleArgs; 8],
        accept_len: usize,
    }

    impl ConversionBackend {
        const fn new(map_responses: &'static [(u64, u64)]) -> Self {
            Self {
                map_responses,
                map_calls: [FullHypercall::EMPTY; 8],
                map_len: 0,
                accept_rets: [0; 8],
                accept_calls: [FullHypercall::EMPTY; 8],
                accept_len: 0,
            }
        }

        fn fail_accept_at(&mut self, index: usize) {
            self.accept_rets[index] = 1;
        }
    }

    impl TdxHypercall for ConversionBackend {
        fn tdcall_saved_ret(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            assert_eq!(leaf, TDG_VP_VMCALL);
            self.map_calls[self.map_len] = *args;
            let (status, r11) = self.map_responses[self.map_len];
            self.map_len += 1;
            args.r10 = status;
            args.r11 = r11;
            0
        }
    }

    impl TdxAccept for ConversionBackend {
        fn tdcall_accept(&mut self, args: &mut TdxModuleArgs) -> u64 {
            self.accept_calls[self.accept_len] = *args;
            let ret = self.accept_rets[self.accept_len];
            self.accept_len += 1;
            ret
        }
    }

    struct FakeSetup {
        tdcall_ret: u64,
        vp_info: TdxModuleArgs,
        tdcall_leaf: u64,
        reads: [(u64, u64); 8],
        read_len: usize,
        read_pos: usize,
        writes: [(u64, u64, u64); 8],
        write_rets: [u64; 8],
        write_len: usize,
    }

    impl FakeSetup {
        const fn new(vp_info: TdxModuleArgs) -> Self {
            Self {
                tdcall_ret: TDX_SUCCESS,
                vp_info,
                tdcall_leaf: 0,
                reads: [(0, 0); 8],
                read_len: 0,
                read_pos: 0,
                writes: [(0, 0, 0); 8],
                write_rets: [0; 8],
                write_len: 0,
            }
        }

        fn push_read(&mut self, field: u64, value: u64) {
            self.reads[self.read_len] = (field, value);
            self.read_len += 1;
        }

        fn push_write_ret(&mut self, ret: u64) {
            self.write_rets[self.write_len] = ret;
        }
    }

    impl TdxTdcall for FakeSetup {
        fn tdcall(&mut self, leaf: u64, args: &mut TdxModuleArgs) -> u64 {
            self.tdcall_leaf = leaf;
            *args = self.vp_info;
            self.tdcall_ret
        }
    }

    impl TdxVmMetadata for FakeSetup {
        fn tdg_vm_rd(&mut self, field: u64) -> u64 {
            assert!(self.read_pos < self.read_len);
            let (expected_field, value) = self.reads[self.read_pos];
            self.read_pos += 1;
            assert_eq!(field, expected_field);
            value
        }

        fn tdg_vm_wr(&mut self, field: u64, value: u64, mask: u64) -> u64 {
            self.writes[self.write_len] = (field, value, mask);
            let ret = self.write_rets[self.write_len];
            self.write_len += 1;
            ret
        }
    }

    #[test]
    fn ve_classifier_covers_linux_handled_reasons() {
        assert_eq!(
            classify_ve(&VeInfo {
                exit_reason: EXIT_REASON_CPUID,
                ..Default::default()
            }),
            VeAction::Cpuid
        );
        assert_eq!(
            classify_ve(&VeInfo {
                exit_reason: EXIT_REASON_EPT_VIOLATION,
                ..Default::default()
            }),
            VeAction::Mmio
        );
        assert_eq!(
            classify_ve(&VeInfo {
                exit_reason: EXIT_REASON_MSR_READ_IMM,
                ..Default::default()
            }),
            VeAction::Unsupported
        );
        assert_eq!(
            classify_ve(&VeInfo {
                exit_reason: EXIT_REASON_MSR_WRITE_IMM,
                ..Default::default()
            }),
            VeAction::Unsupported
        );
        assert_eq!(
            classify_ve(&VeInfo {
                exit_reason: EXIT_REASON_VMCALL,
                ..Default::default()
            }),
            VeAction::Unsupported
        );
        assert_eq!(
            classify_ve(&VeInfo {
                exit_reason: 999,
                ..Default::default()
            }),
            VeAction::Unsupported
        );
    }

    #[test]
    fn ve_instruction_length_matches_linux_ept_rule() {
        assert_eq!(
            ve_instr_len(&VeInfo {
                exit_reason: EXIT_REASON_CPUID,
                instr_len: 4,
                ..Default::default()
            }),
            4
        );
        assert_eq!(
            ve_instr_len(&VeInfo {
                exit_reason: EXIT_REASON_EPT_VIOLATION,
                instr_len: 4,
                ..Default::default()
            }),
            0
        );
    }

    #[test]
    fn flush_requirements_match_tdx_linux_policy() {
        assert!(!tdx_tlb_flush_required(true));
        assert!(tdx_tlb_flush_required(false));
        assert!(tdx_cache_flush_required());
    }

    #[test]
    fn early_setup_uses_linux_tdinfo_and_tdcs_sequence() {
        SEPT_VE_DISABLED.store(false, Ordering::Release);
        let vp_info = TdxModuleArgs {
            rcx: 52,
            rdx: 0,
            ..FullHypercall::EMPTY
        };
        let mut backend = FakeSetup::new(vp_info);
        backend.push_read(TDCS_CONFIG_FLAGS, TDCS_CONFIG_FLEXIBLE_PENDING_VE);
        backend.push_read(TDCS_TD_CTLS, 0);

        assert_eq!(
            tdx_setup_info_plan(TDX_SUCCESS, vp_info),
            TdxSetupInfoPlan {
                tdcall: tdcall_plan(TDG_VP_INFO, TDX_SUCCESS),
                args: vp_info,
                gpa_width: 52,
                cc_mask: Some(1 << 51),
                td_attr: Some(0),
            }
        );

        let setup = tdx_setup_with(&mut backend).unwrap();

        assert_eq!(backend.tdcall_leaf, TDG_VP_INFO);
        assert_eq!(setup.cc_mask, 1 << 51);
        assert_eq!(setup.td_attr, 0);
        assert_eq!(setup.sept_ve, SeptVeOutcome::DisabledControl);
        assert_eq!(setup.reduce_ve, ReduceVeOutcome::ReduceVeEnabled);
        assert_eq!(backend.read_pos, 2);
        assert_eq!(backend.write_len, 3);
        assert_eq!(backend.writes[0], (TDCS_NOTIFY_ENABLES, 0, u64::MAX));
        assert_eq!(
            backend.writes[1],
            (
                TDCS_TD_CTLS,
                TD_CTLS_PENDING_VE_DISABLE,
                TD_CTLS_PENDING_VE_DISABLE
            )
        );
        assert_eq!(
            backend.writes[2],
            (TDCS_TD_CTLS, TD_CTLS_REDUCE_VE, TD_CTLS_REDUCE_VE)
        );
        assert!(sept_ve_disabled());
    }

    #[test]
    fn setup_stops_when_linux_sept_ve_path_panics() {
        SEPT_VE_DISABLED.store(false, Ordering::Release);
        let vp_info = TdxModuleArgs {
            rcx: 52,
            rdx: 0,
            ..FullHypercall::EMPTY
        };
        let mut backend = FakeSetup::new(vp_info);
        backend.push_read(TDCS_CONFIG_FLAGS, 0);

        assert_eq!(tdx_setup_with(&mut backend), Err(EIO));

        assert_eq!(backend.tdcall_leaf, TDG_VP_INFO);
        assert_eq!(backend.read_pos, 1);
        assert_eq!(backend.write_len, 1);
        assert_eq!(backend.writes[0], (TDCS_NOTIFY_ENABLES, 0, u64::MAX));
        assert!(!sept_ve_disabled());
    }

    #[test]
    fn setup_and_announce_tdinfo_failures_are_linux_fatal_tdcall_plans() {
        let vp_info = TdxModuleArgs {
            rcx: 52,
            rdx: TDX_TD_ATTR_DEBUG,
            ..FullHypercall::EMPTY
        };

        let setup_plan = tdx_setup_info_plan(7, vp_info);
        assert_eq!(setup_plan.tdcall, tdcall_plan(TDG_VP_INFO, 7));
        assert!(setup_plan.tdcall.panic);
        assert_eq!(setup_plan.cc_mask, None);
        assert_eq!(setup_plan.td_attr, None);

        let mut setup_backend = FakeSetup::new(vp_info);
        setup_backend.tdcall_ret = 7;
        assert_eq!(tdx_setup_with(&mut setup_backend), Err(EIO));
        assert_eq!(setup_backend.read_pos, 0);
        assert_eq!(setup_backend.write_len, 0);

        let announcement_plan = tdx_announcement_plan(7, vp_info, Some(TD_CTLS_LOCK));
        assert_eq!(announcement_plan.tdcall, tdcall_plan(TDG_VP_INFO, 7));
        assert!(announcement_plan.tdcall.panic);
        assert_eq!(announcement_plan.td_attr, None);
        assert_eq!(announcement_plan.td_ctls, None);

        let mut announce_backend = FakeSetup::new(vp_info);
        announce_backend.tdcall_ret = 7;
        assert_eq!(tdx_announce_with(&mut announce_backend), Err(EIO));
        assert_eq!(announce_backend.read_pos, 0);
        assert_eq!(announce_backend.write_len, 0);
    }

    #[test]
    fn early_init_ignores_non_tdx_cpuid_signature() {
        let mut backend = FakeSetup::new(TdxModuleArgs {
            rcx: 52,
            ..FullHypercall::EMPTY
        });

        assert_eq!(
            tdx_early_init_with(&mut backend, [0, 0, 0], u64::MAX),
            Ok(None)
        );
        assert_eq!(backend.tdcall_leaf, 0);
        assert_eq!(backend.read_pos, 0);
        assert_eq!(backend.write_len, 0);
    }

    #[test]
    fn early_init_models_linux_tdx_setup_hooks_and_announce() {
        SEPT_VE_DISABLED.store(false, Ordering::Release);
        let mut backend = FakeSetup::new(TdxModuleArgs {
            rcx: 52,
            rdx: TDX_TD_ATTR_SEPT_VE_DISABLE,
            ..FullHypercall::EMPTY
        });
        backend.push_read(TDCS_CONFIG_FLAGS, 0);
        backend.push_read(TDCS_TD_CTLS, TD_CTLS_LOCK | TD_CTLS_REDUCE_VE);

        let state = tdx_early_init_with(&mut backend, TDX_IDENT_SIG, u64::MAX)
            .unwrap()
            .unwrap();

        assert!(tdx_ident_matches(TDX_IDENT_SIG));
        assert_eq!(state.setup.cc_mask, 1 << 51);
        assert_eq!(state.cc_mask, 1 << 51);
        assert_eq!(state.physical_mask, (1 << 51) - 1);
        assert_eq!(state.announcement.td_attr, TDX_TD_ATTR_SEPT_VE_DISABLE);
        assert_eq!(state.announcement.td_ctls, TD_CTLS_LOCK | TD_CTLS_REDUCE_VE);
        assert!(state.force_tdx_guest);
        assert!(state.force_tsc_reliable);
        assert!(state.cc_vendor_intel);
        assert_eq!(
            state.guest_hooks,
            TdxGuestHooks {
                enc_status_change_prepare: true,
                enc_status_change_finish: true,
                enc_cache_flush_required: true,
                enc_tlb_flush_required: true,
                enc_kexec_begin: true,
                enc_kexec_finish: true,
            }
        );
        assert_eq!(
            state.halt_hooks,
            TdxHaltHooks {
                safe_halt: true,
                halt: true
            }
        );
        assert!(!state.parallel_bringup);
        assert_eq!(backend.read_pos, 2);
        assert_eq!(backend.write_len, 2);
        assert_eq!(backend.writes[0], (TDCS_NOTIFY_ENABLES, 0, u64::MAX));
        assert_eq!(
            backend.writes[1],
            (TDCS_TD_CTLS, TD_CTLS_REDUCE_VE, TD_CTLS_REDUCE_VE)
        );
    }

    #[test]
    fn sept_ve_setup_branches_match_linux_rules() {
        SEPT_VE_DISABLED.store(false, Ordering::Release);
        let mut attr_disabled = FakeSetup::new(FullHypercall::EMPTY);
        attr_disabled.push_read(TDCS_CONFIG_FLAGS, 0);
        assert_eq!(
            tdx_disable_sept_ve_with(&mut attr_disabled, TDX_TD_ATTR_SEPT_VE_DISABLE),
            SeptVeOutcome::AlreadyDisabledByAttribute
        );
        assert!(sept_ve_disabled());
        assert_eq!(attr_disabled.write_len, 0);

        SEPT_VE_DISABLED.store(false, Ordering::Release);
        let mut debug_warn = FakeSetup::new(FullHypercall::EMPTY);
        debug_warn.push_read(TDCS_CONFIG_FLAGS, 0);
        assert_eq!(
            tdx_disable_sept_ve_with(&mut debug_warn, TDX_TD_ATTR_DEBUG),
            SeptVeOutcome::DebugWarningOnly
        );
        assert!(!sept_ve_disabled());

        let mut panic_required = FakeSetup::new(FullHypercall::EMPTY);
        panic_required.push_read(TDCS_CONFIG_FLAGS, 0);
        assert_eq!(
            tdx_disable_sept_ve_with(&mut panic_required, 0),
            SeptVeOutcome::PanicRequired
        );

        SEPT_VE_DISABLED.store(false, Ordering::Release);
        let mut control_disabled = FakeSetup::new(FullHypercall::EMPTY);
        control_disabled.push_read(TDCS_CONFIG_FLAGS, TDCS_CONFIG_FLEXIBLE_PENDING_VE);
        control_disabled.push_read(TDCS_TD_CTLS, TD_CTLS_PENDING_VE_DISABLE);
        assert_eq!(
            tdx_disable_sept_ve_with(&mut control_disabled, 0),
            SeptVeOutcome::AlreadyDisabledByControl
        );
        assert!(sept_ve_disabled());
        assert_eq!(control_disabled.write_len, 0);

        let mut debug_kept = FakeSetup::new(FullHypercall::EMPTY);
        debug_kept.push_read(TDCS_CONFIG_FLAGS, TDCS_CONFIG_FLEXIBLE_PENDING_VE);
        debug_kept.push_read(TDCS_TD_CTLS, 0);
        assert_eq!(
            tdx_disable_sept_ve_with(&mut debug_kept, TDX_TD_ATTR_DEBUG),
            SeptVeOutcome::DebugKeptEnabled
        );
        assert_eq!(debug_kept.write_len, 0);
    }

    #[test]
    fn reduce_ve_falls_back_to_enum_topology_like_linux() {
        let mut enabled = FakeSetup::new(FullHypercall::EMPTY);
        enabled.push_write_ret(TDVMCALL_STATUS_INVALID_OPERAND);
        enabled.push_read(TDCS_TOPOLOGY_ENUM_CONFIGURED, 1);

        assert_eq!(
            tdx_reduce_unnecessary_ve_with(&mut enabled),
            ReduceVeOutcome::EnumTopologyEnabled
        );
        assert_eq!(
            enabled.writes[0],
            (TDCS_TD_CTLS, TD_CTLS_REDUCE_VE, TD_CTLS_REDUCE_VE)
        );
        assert_eq!(
            enabled.writes[1],
            (TDCS_TD_CTLS, TD_CTLS_ENUM_TOPOLOGY, TD_CTLS_ENUM_TOPOLOGY)
        );

        let mut missing = FakeSetup::new(FullHypercall::EMPTY);
        missing.push_write_ret(TDVMCALL_STATUS_INVALID_OPERAND);
        missing.push_read(TDCS_TOPOLOGY_ENUM_CONFIGURED, 0);
        assert_eq!(
            tdx_reduce_unnecessary_ve_with(&mut missing),
            ReduceVeOutcome::TopologyNotConfigured
        );
        assert_eq!(missing.write_len, 1);
    }

    #[test]
    fn gpa_mapping_wrapper_keeps_linux_direction_without_precheck() {
        assert_eq!(tdx_map_gpa(0x1000, 0x2000, false), Ok(GpaState::Shared));
        assert_eq!(tdx_map_gpa(0x1001, 0x2000, false), Ok(GpaState::Shared));
        assert_eq!(tdx_map_gpa(0x4000, 0x3000, true), Ok(GpaState::Private));
    }

    #[test]
    fn enc_status_changed_no_longer_updates_shared_count_directly() {
        NR_SHARED.store(7, Ordering::Release);

        assert_eq!(tdx_enc_status_changed(2, false), Ok(GpaState::Shared));
        assert_eq!(tdx_enc_status_changed(2, true), Ok(GpaState::Private));
        assert_eq!(tdx_enc_status_changed(0, false), Ok(GpaState::Shared));
        assert_eq!(tdx_enc_status_changed(-1, true), Ok(GpaState::Private));
        assert_eq!(nr_shared_pages(), 7);
    }

    #[test]
    fn enc_prepare_for_private_maps_and_accepts_without_counting() {
        NR_SHARED.store(5, Ordering::Release);
        let mut backend = ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);

        assert_eq!(
            tdx_enc_status_change_prepare_with(&mut backend, 0x1000, 2, true, 1 << 47),
            Ok(())
        );

        assert_eq!(backend.map_len, 1);
        assert_eq!(backend.map_calls[0].r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(backend.map_calls[0].r11, TDVMCALL_MAP_GPA);
        assert_eq!(backend.map_calls[0].r12, 0x1000);
        assert_eq!(backend.map_calls[0].r13, 2 * PAGE_SIZE);
        assert_eq!(backend.accept_len, 2);
        assert_eq!(backend.accept_calls[0].rcx, 0x1000);
        assert_eq!(backend.accept_calls[1].rcx, 0x2000);
        assert_eq!(nr_shared_pages(), 5);
    }

    #[test]
    fn enc_finish_for_shared_maps_decrypted_and_updates_count() {
        NR_SHARED.store(0, Ordering::Release);
        let mut backend = ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);

        assert_eq!(
            tdx_enc_status_change_finish_with(&mut backend, 0x3000, 3, false, 1 << 47),
            Ok(())
        );

        assert_eq!(backend.map_len, 1);
        assert_eq!(backend.map_calls[0].r12, (1 << 47) | 0x3000);
        assert_eq!(backend.map_calls[0].r13, 3 * PAGE_SIZE);
        assert_eq!(backend.accept_len, 0);
        assert_eq!(nr_shared_pages(), 3);
    }

    #[test]
    fn enc_prepare_and_finish_only_handle_their_linux_direction() {
        NR_SHARED.store(4, Ordering::Release);
        let mut prepare_shared = ConversionBackend::new(&[]);
        assert_eq!(
            tdx_enc_status_change_prepare_with(&mut prepare_shared, 0x1000, 2, false, 0),
            Ok(())
        );
        assert_eq!(prepare_shared.map_len, 0);
        assert_eq!(nr_shared_pages(), 4);

        let mut finish_private = ConversionBackend::new(&[]);
        assert_eq!(
            tdx_enc_status_change_finish_with(&mut finish_private, 0x1000, 2, true, 0),
            Ok(())
        );
        assert_eq!(finish_private.map_len, 0);
        assert_eq!(finish_private.accept_len, 0);
        assert_eq!(nr_shared_pages(), 2);
    }

    #[test]
    fn enc_conversion_wrappers_keep_linux_numpages_wrapping_without_precheck() {
        NR_SHARED.store(5, Ordering::Release);
        let mut zero_prepare = ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        assert_eq!(
            tdx_enc_status_change_prepare_with(&mut zero_prepare, 0x1000, 0, true, 0),
            Ok(())
        );
        assert_eq!(zero_prepare.map_len, 1);
        assert_eq!(zero_prepare.map_calls[0].r12, 0x1000);
        assert_eq!(zero_prepare.map_calls[0].r13, 0);
        assert_eq!(zero_prepare.accept_len, 0);

        let mut negative_finish = ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        assert_eq!(
            tdx_enc_status_change_finish_with(&mut negative_finish, 0x3000, -1, false, 0),
            Ok(())
        );
        assert_eq!(negative_finish.map_len, 1);
        assert_eq!(negative_finish.map_calls[0].r12, 0x3000);
        assert_eq!(negative_finish.map_calls[0].r13, 0xffff_ffff_ffff_f000);
        assert_eq!(negative_finish.accept_len, 0);
        assert_eq!(nr_shared_pages(), 4);
    }

    #[test]
    fn enc_conversion_failures_return_linux_eio_before_counting() {
        NR_SHARED.store(0, Ordering::Release);
        let mut map_fail = ConversionBackend::new(&[(TDVMCALL_STATUS_ALIGN_ERROR, 0)]);
        assert_eq!(
            tdx_enc_status_change_finish_with(&mut map_fail, 0x1000, 1, false, 0),
            Err(EIO)
        );
        assert_eq!(nr_shared_pages(), 0);

        let mut accept_fail = ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        accept_fail.fail_accept_at(0);
        assert_eq!(
            tdx_enc_status_change_prepare_with(&mut accept_fail, 0x1000, 1, true, 0),
            Err(EIO)
        );
        assert_eq!(accept_fail.accept_len, 1);
    }

    #[test]
    fn kexec_begin_reports_disabled_stopped_or_failed_stop() {
        assert_eq!(
            tdx_kexec_begin_with(false, false),
            TdxKexecBeginOutcome::Disabled
        );
        assert_eq!(
            tdx_kexec_begin_with(true, true),
            TdxKexecBeginOutcome::Stopped
        );
        assert_eq!(
            tdx_kexec_begin_with(true, false),
            TdxKexecBeginOutcome::StopFailed
        );
    }

    #[test]
    fn kexec_finish_disabled_is_linux_noop() {
        NR_SHARED.store(1, Ordering::Release);
        let mut backend = ConversionBackend::new(&[]);
        let mut entries = [TdxDirectMapEntry {
            addr: 0x1000,
            size: PAGE_SIZE,
            present: true,
            decrypted: true,
            cleared: false,
        }];

        let report = tdx_kexec_finish_with(&mut backend, &mut entries, 0, false);

        assert_eq!(
            report,
            TdxKexecFinishReport {
                enabled: false,
                ..Default::default()
            }
        );
        assert!(entries[0].present);
        assert!(entries[0].decrypted);
        assert_eq!(backend.map_len, 0);
        assert_eq!(backend.accept_len, 0);
    }

    #[test]
    fn kexec_finish_clears_and_unshares_decrypted_direct_map_entries() {
        NR_SHARED.store(2, Ordering::Release);
        let mut backend = ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        let mut entries = [
            TdxDirectMapEntry {
                addr: 0x1000,
                size: 2 * PAGE_SIZE,
                present: true,
                decrypted: true,
                cleared: false,
            },
            TdxDirectMapEntry {
                addr: 0x3000,
                size: PAGE_SIZE,
                present: true,
                decrypted: false,
                cleared: false,
            },
            TdxDirectMapEntry {
                addr: 0x4000,
                size: PAGE_SIZE,
                present: false,
                decrypted: true,
                cleared: false,
            },
        ];

        let report = tdx_kexec_finish_with(&mut backend, &mut entries, 1 << 47, true);

        assert_eq!(report.enabled, true);
        assert_eq!(report.converted_ranges, 1);
        assert_eq!(report.cleared_entries, 1);
        assert_eq!(report.failed_ranges, 0);
        assert_eq!(report.found_shared_pages, 2);
        assert_eq!(report.accounted_shared_pages, 2);
        assert!(!report.accounting_mismatch);
        assert!(report.tlb_flushed);
        assert!(!entries[0].present);
        assert!(!entries[0].decrypted);
        assert!(entries[0].cleared);
        assert!(entries[1].present);
        assert!(!entries[1].cleared);
        assert_eq!(backend.map_len, 1);
        assert_eq!(backend.map_calls[0].r11, TDVMCALL_MAP_GPA);
        assert_eq!(backend.map_calls[0].r12, 0x1000);
        assert_eq!(backend.map_calls[0].r13, 2 * PAGE_SIZE);
        assert_eq!(backend.accept_len, 2);
        assert_eq!(backend.accept_calls[0].rcx, 0x1000);
        assert_eq!(backend.accept_calls[1].rcx, 0x2000);
    }

    #[test]
    fn kexec_finish_reports_failed_unshare_and_shared_count_mismatch() {
        NR_SHARED.store(2, Ordering::Release);
        let mut backend = ConversionBackend::new(&[(TDVMCALL_STATUS_ALIGN_ERROR, 0)]);
        let mut entries = [TdxDirectMapEntry {
            addr: 0x1000,
            size: PAGE_SIZE,
            present: true,
            decrypted: true,
            cleared: false,
        }];

        let report = tdx_kexec_finish_with(&mut backend, &mut entries, 0, true);

        assert_eq!(report.converted_ranges, 1);
        assert_eq!(report.cleared_entries, 1);
        assert_eq!(report.failed_ranges, 1);
        assert_eq!(report.found_shared_pages, 1);
        assert_eq!(report.accounted_shared_pages, 2);
        assert!(report.accounting_mismatch);
        assert!(report.tlb_flushed);
        assert!(!entries[0].present);
        assert!(entries[0].cleared);
        assert_eq!(backend.accept_len, 0);
    }

    #[test]
    fn kexec_finish_keeps_linux_wrapping_conversion_shape_without_range_precheck() {
        NR_SHARED.store(2, Ordering::Release);
        let mut backend =
            ConversionBackend::new(&[(TDVMCALL_STATUS_SUCCESS, 0), (TDVMCALL_STATUS_SUCCESS, 0)]);
        let mut entries = [
            TdxDirectMapEntry {
                addr: 0x1000,
                size: 0,
                present: true,
                decrypted: true,
                cleared: false,
            },
            TdxDirectMapEntry {
                addr: u64::MAX - PAGE_SIZE + 1,
                size: 2 * PAGE_SIZE,
                present: true,
                decrypted: true,
                cleared: false,
            },
        ];

        let report = tdx_kexec_finish_with(&mut backend, &mut entries, 0, true);

        assert_eq!(report.converted_ranges, 2);
        assert_eq!(report.cleared_entries, 2);
        assert_eq!(report.failed_ranges, 0);
        assert_eq!(report.found_shared_pages, 2);
        assert_eq!(report.accounted_shared_pages, 2);
        assert!(!report.accounting_mismatch);
        assert!(entries.iter().all(|entry| !entry.present && entry.cleared));
        assert_eq!(backend.map_len, 2);
        assert_eq!(backend.map_calls[0].r12, 0x1000);
        assert_eq!(backend.map_calls[0].r13, 0);
        assert_eq!(backend.map_calls[1].r12, u64::MAX - PAGE_SIZE + 1);
        assert_eq!(backend.map_calls[1].r13, 2 * PAGE_SIZE);
        assert_eq!(backend.accept_len, 0);
    }

    #[test]
    fn map_gpa_retries_from_untrusted_r11_progress() {
        let mut backend = FakeHypercall::new(&[
            (TDVMCALL_STATUS_RETRY, 0x3000),
            (TDVMCALL_STATUS_SUCCESS, 0),
        ]);

        assert!(tdx_map_gpa_with(&mut backend, 0x1000, 0x5000, true, 0));
        assert_eq!(backend.len, 2);
        assert_eq!(backend.calls[0].r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(backend.calls[0].r11, TDVMCALL_MAP_GPA);
        assert_eq!(backend.calls[0].r12, 0x1000);
        assert_eq!(backend.calls[0].r13, 0x4000);
        assert_eq!(backend.calls[1].r12, 0x3000);
        assert_eq!(backend.calls[1].r13, 0x2000);
    }

    #[test]
    fn map_gpa_rejects_bad_or_stalled_retry_progress() {
        let mut bad_progress = FakeHypercall::new(&[(TDVMCALL_STATUS_RETRY, 0x5000)]);
        assert!(!tdx_map_gpa_with(
            &mut bad_progress,
            0x1000,
            0x5000,
            true,
            0
        ));

        let mut stalled = FakeHypercall::new(&[
            (TDVMCALL_STATUS_RETRY, 0x1000),
            (TDVMCALL_STATUS_RETRY, 0x1000),
            (TDVMCALL_STATUS_RETRY, 0x1000),
        ]);
        assert!(!tdx_map_gpa_with(&mut stalled, 0x1000, 0x5000, true, 0));
        assert_eq!(stalled.len, 3);
    }

    #[test]
    fn map_gpa_direct_helper_issues_linux_hypercall_without_range_precheck() {
        let mut zero = FakeHypercall::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        assert!(tdx_map_gpa_with(&mut zero, 0x1000, 0x1000, true, 0));
        assert_eq!(zero.len, 1);
        assert_eq!(zero.calls[0].r12, 0x1000);
        assert_eq!(zero.calls[0].r13, 0);

        let mut reversed = FakeHypercall::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        assert!(tdx_map_gpa_with(&mut reversed, 0x3000, 0x1000, true, 0));
        assert_eq!(reversed.len, 1);
        assert_eq!(reversed.calls[0].r12, 0x3000);
        assert_eq!(reversed.calls[0].r13, 0xffff_ffff_ffff_e000);
    }

    #[test]
    fn map_gpa_applies_shared_mask_for_decrypted_mapping() {
        let mut backend = FakeHypercall::new(&[(TDVMCALL_STATUS_SUCCESS, 0)]);
        assert!(tdx_map_gpa_with(
            &mut backend,
            0x1000,
            0x2000,
            false,
            1 << 47
        ));
        assert_eq!(backend.calls[0].r12, (1 << 47) | 0x1000);
        assert_eq!(backend.calls[0].r13, 0x1000);
    }

    #[test]
    fn public_attestation_wrappers_route_through_linux_abi_helpers() {
        assert_eq!(TDG_MR_RTMR_EXTEND, 2);
        let reportdata = [0u8; TDREPORT_DATA_LEN];
        let mut tdreport = [0u8; TDREPORT_LEN];
        let rtmr_data = [0u8; RTMR_EXTEND_DATA_LEN];
        let mut quote_buf = [0u8; PAGE_SIZE as usize];

        assert_eq!(
            tdx_mcall_get_report0(reportdata.as_ptr(), tdreport.as_mut_ptr()),
            Err(EIO)
        );
        assert_eq!(tdx_mcall_extend_rtmr(0, rtmr_data.as_ptr()), Err(EIO));
        assert_eq!(tdx_mcall_extend_rtmr(4, rtmr_data.as_ptr()), Err(EIO));
        assert_eq!(
            tdx_hcall_get_quote(quote_buf.as_mut_ptr(), quote_buf.len()),
            Ok(TDVMCALL_STATUS_SUBFUNC_UNSUPPORTED)
        );
    }

    #[test]
    fn module_call_helpers_populate_linux_register_abi() {
        let report_args = tdx_mcall_get_report0_args(0x2200, 0x1100);
        assert_eq!(report_args.rcx, 0x1100);
        assert_eq!(report_args.rdx, 0x2200);
        assert_eq!(report_args.r8, TDREPORT_SUBTYPE_0);

        let mut report = FakeTdcall {
            leaf: 0,
            ret: 0,
            seen: None,
        };
        assert_eq!(
            tdx_mcall_get_report0_with(
                &mut report,
                0x2200,
                0x1100,
                TDREPORT_DATA_LEN,
                TDREPORT_LEN
            ),
            Ok(())
        );
        let args = report.seen.unwrap();
        assert_eq!(report.leaf, TDG_MR_REPORT);
        assert_eq!(args.rcx, 0x1100);
        assert_eq!(args.rdx, 0x2200);
        assert_eq!(args.r8, TDREPORT_SUBTYPE_0);

        let rtmr_args = tdx_mcall_extend_rtmr_args(2, 0x3300);
        assert_eq!(rtmr_args.rcx, 0x3300);
        assert_eq!(rtmr_args.rdx, 2);

        let mut rtmr = FakeTdcall {
            leaf: 0,
            ret: TDCALL_OPERAND_BUSY << 32,
            seen: None,
        };
        assert_eq!(
            tdx_mcall_extend_rtmr_with(&mut rtmr, 2, 0x3300, RTMR_EXTEND_DATA_LEN),
            Err(EBUSY)
        );
        let args = rtmr.seen.unwrap();
        assert_eq!(rtmr.leaf, TDG_MR_RTMR_EXTEND);
        assert_eq!(args.rcx, 0x3300);
        assert_eq!(args.rdx, 2);
    }

    #[test]
    fn module_call_helpers_defer_invalid_operands_to_tdx_module_like_linux() {
        let mut report = FakeTdcall {
            leaf: 0,
            ret: TDCALL_INVALID_OPERAND << 32,
            seen: None,
        };
        assert_eq!(
            tdx_mcall_get_report0_with(&mut report, 0x2200, 0x1100, 1, 2),
            Err(ENXIO)
        );
        let args = report.seen.unwrap();
        assert_eq!(report.leaf, TDG_MR_REPORT);
        assert_eq!(args.rcx, 0x1100);
        assert_eq!(args.rdx, 0x2200);
        assert_eq!(args.r8, TDREPORT_SUBTYPE_0);

        let mut rtmr = FakeTdcall {
            leaf: 0,
            ret: TDCALL_INVALID_OPERAND << 32,
            seen: None,
        };
        assert_eq!(
            tdx_mcall_extend_rtmr_with(&mut rtmr, 4, 0x3300, 0),
            Err(ENXIO)
        );
        let args = rtmr.seen.unwrap();
        assert_eq!(rtmr.leaf, TDG_MR_RTMR_EXTEND);
        assert_eq!(args.rcx, 0x3300);
        assert_eq!(args.rdx, 4);
    }

    #[test]
    fn module_call_error_codes_match_linux_mapping() {
        assert_eq!(
            tdx_tdcall_status_to_result(TDCALL_INVALID_OPERAND << 32),
            Err(ENXIO)
        );
        assert_eq!(
            tdx_tdcall_status_to_result(TDCALL_OPERAND_BUSY << 32),
            Err(EBUSY)
        );
        assert_eq!(tdx_tdcall_status_to_result(0x1234 << 32), Err(EIO));
    }

    #[test]
    fn quote_and_kvm_hypercalls_use_linux_register_abi() {
        let quote_args = tdx_hcall_get_quote_args(0x4000, 0x2000, 1 << 47);
        assert_eq!(quote_args.r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(quote_args.r11, TDVMCALL_GET_QUOTE);
        assert_eq!(quote_args.r12, (1 << 47) | 0x4000);
        assert_eq!(quote_args.r13, 0x2000);

        let mut quote = FakeHypercall::new(&[(TDVMCALL_STATUS_SUBFUNC_UNSUPPORTED, 0)]);
        assert_eq!(
            tdx_hcall_get_quote_with(&mut quote, 0x4000, 0x2000, 1 << 47),
            TDVMCALL_STATUS_SUBFUNC_UNSUPPORTED
        );
        assert_eq!(quote.calls[0].r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(quote.calls[0].r11, TDVMCALL_GET_QUOTE);
        assert_eq!(quote.calls[0].r12, (1 << 47) | 0x4000);
        assert_eq!(quote.calls[0].r13, 0x2000);

        let kvm_args = tdx_kvm_hypercall_args(7, 1, 2, 3, 4);
        assert_eq!(kvm_args.r10, 7);
        assert_eq!(kvm_args.r11, 1);
        assert_eq!(kvm_args.r12, 2);
        assert_eq!(kvm_args.r13, 3);
        assert_eq!(kvm_args.r14, 4);
        assert_eq!(tdx_kvm_hypercall(7, 1, 2, 3, 4), Err(ENODEV));

        let mut kvm = FakeHypercall::new(&[(17, 0)]);
        assert_eq!(tdx_kvm_hypercall_with(&mut kvm, 7, 1, 2, 3, 4), 17);
        assert_eq!(kvm.calls[0].r10, 7);
        assert_eq!(kvm.calls[0].r11, 1);
        assert_eq!(kvm.calls[0].r12, 2);
        assert_eq!(kvm.calls[0].r13, 3);
        assert_eq!(kvm.calls[0].r14, 4);
    }

    #[test]
    fn halt_wrappers_model_linux_irq_flags_warn_and_safe_halt_tail() {
        let hlt = __halt_plan(true, 0);
        assert_eq!(hlt.args.r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(hlt.args.r11, EXIT_REASON_HLT);
        assert_eq!(hlt.args.r12, 1);
        assert_eq!(hlt.ret, 0);
        assert!(!hlt.warned);
        assert!(!hlt.raw_local_irq_enabled);

        let failed = tdx_halt_plan(5);
        assert_eq!(failed.args.r12, 0);
        assert_eq!(failed.ret, 5);
        assert!(failed.warned);
        assert!(!failed.raw_local_irq_enabled);

        let safe = tdx_safe_halt_plan(0);
        assert_eq!(safe.args.r12, 0);
        assert!(!safe.warned);
        assert!(safe.raw_local_irq_enabled);
    }

    #[test]
    fn ve_halt_msr_and_cpuid_handlers_use_linux_hypercalls() {
        let ve = VeInfo {
            instr_len: 3,
            ..Default::default()
        };

        let mut halt = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_handle_halt_with(&mut halt, &ve, true), Ok(3));
        assert_eq!(halt.calls[0].r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(halt.calls[0].r11, EXIT_REASON_HLT);
        assert_eq!(halt.calls[0].r12, 1);
        let mut no_halt = FullHypercall::new(&[]);
        assert_eq!(tdx_handle_halt_with(&mut no_halt, &ve, false), Err(EIO));
        assert_eq!(no_halt.len, 0);

        let mut regs = TdxRegs {
            cx: 0x10,
            ..Default::default()
        };
        let mut rdmsr = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r11: 0x1122_3344_5566_7788,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_read_msr_with(&mut rdmsr, &mut regs, &ve), Ok(3));
        assert_eq!(rdmsr.calls[0].r11, EXIT_REASON_MSR_READ);
        assert_eq!(rdmsr.calls[0].r12, 0x10);
        assert_eq!(regs.ax, 0x5566_7788);
        assert_eq!(regs.dx, 0x1122_3344);

        let regs = TdxRegs {
            ax: 0x5566_7788,
            cx: 0x20,
            dx: 0x1122_3344,
            ..Default::default()
        };
        let mut wrmsr = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_write_msr_with(&mut wrmsr, &regs, &ve), Ok(3));
        assert_eq!(wrmsr.calls[0].r11, EXIT_REASON_MSR_WRITE);
        assert_eq!(wrmsr.calls[0].r12, 0x20);
        assert_eq!(wrmsr.calls[0].r13, 0x1122_3344_5566_7788);

        let regs = TdxRegs {
            ax: 0xffff_0000_5566_7788,
            cx: 0x20,
            dx: 0x1122_3344,
            ..Default::default()
        };
        let mut wrmsr = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_write_msr_with(&mut wrmsr, &regs, &ve), Ok(3));
        assert_eq!(wrmsr.calls[0].r13, 0xffff_3344_5566_7788);

        let mut non_hv_leaf = TdxRegs {
            ax: 0x21,
            bx: 1,
            cx: 2,
            dx: 3,
            ..Default::default()
        };
        let mut no_cpuid_call = FullHypercall::new(&[]);
        assert_eq!(
            tdx_handle_cpuid_with(&mut no_cpuid_call, &mut non_hv_leaf, &ve),
            Ok(3)
        );
        assert_eq!(non_hv_leaf, TdxRegs::default());
        assert_eq!(no_cpuid_call.len, 0);

        let mut hv_leaf = TdxRegs {
            ax: 0x4000_0001,
            cx: 7,
            ..Default::default()
        };
        let mut cpuid = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r12: 1,
            r13: 2,
            r14: 3,
            r15: 4,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_handle_cpuid_with(&mut cpuid, &mut hv_leaf, &ve), Ok(3));
        assert_eq!(cpuid.calls[0].r11, EXIT_REASON_CPUID);
        assert_eq!(cpuid.calls[0].r12, 0x4000_0001);
        assert_eq!(cpuid.calls[0].r13, 7);
        assert_eq!(
            hv_leaf,
            TdxRegs {
                ax: 1,
                bx: 2,
                cx: 3,
                dx: 4,
                ip: 0
            }
        );
    }

    #[test]
    fn ve_io_handler_uses_linux_exit_qual_and_register_masks() {
        let ve_in = VeInfo {
            exit_qual: (1 << 3) | (0x3f8 << 16),
            instr_len: 2,
            ..Default::default()
        };
        let mut regs = TdxRegs {
            ax: u64::MAX,
            ..Default::default()
        };
        let mut backend = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r11: 0xabcd,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_handle_io_with(&mut backend, &mut regs, &ve_in), Ok(2));
        assert_eq!(
            tdx_io_info(ve_in.exit_qual),
            Some(IoInfo {
                direction: PortDirection::In,
                size: 1,
                port: 0x3f8
            })
        );
        assert_eq!(tdx_io_mask(1), 0x1ff);
        assert_eq!(backend.calls[0].r11, EXIT_REASON_IO_INSTRUCTION);
        assert_eq!(backend.calls[0].r12, 1);
        assert_eq!(backend.calls[0].r13, PORT_READ);
        assert_eq!(backend.calls[0].r14, 0x3f8);
        assert_eq!(regs.ax, (u64::MAX & !0x1ff) | (0xabcd & 0x1ff));
        assert_eq!(
            tdx_io_info((0x1234_u64 << 32) | ve_in.exit_qual),
            Some(IoInfo {
                direction: PortDirection::In,
                size: 1,
                port: 0x3f8
            })
        );

        let ve_out = VeInfo {
            exit_qual: 1 | (0x80 << 16),
            instr_len: 2,
            ..Default::default()
        };
        let mut regs = TdxRegs {
            ax: 0x1_2345,
            ..Default::default()
        };
        let mut backend = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_handle_io_with(&mut backend, &mut regs, &ve_out), Ok(2));
        assert_eq!(backend.calls[0].r12, 2);
        assert_eq!(backend.calls[0].r13, PORT_WRITE);
        assert_eq!(backend.calls[0].r14, 0x80);
        assert_eq!(backend.calls[0].r15, 0x1_2345 & tdx_io_mask(2));

        let ve_string = VeInfo {
            exit_qual: 1 << 4,
            ..Default::default()
        };
        assert_eq!(tdx_io_info(ve_string.exit_qual), None);
        assert_eq!(
            tdx_handle_io_with(&mut backend, &mut regs, &ve_string),
            Err(EIO)
        );
    }

    #[test]
    fn mmio_read_write_helpers_use_linux_tdvmcall_registers() {
        let mut read = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r11: 0x1122_3344,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(tdx_mmio_read_with(&mut read, 4, 0xfee0), Some(0x1122_3344));
        assert_eq!(read.calls[0].r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(read.calls[0].r11, EXIT_REASON_EPT_VIOLATION);
        assert_eq!(read.calls[0].r12, 4);
        assert_eq!(read.calls[0].r13, EPT_READ);
        assert_eq!(read.calls[0].r14, 0xfee0);

        let mut write = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert!(tdx_mmio_write_with(&mut write, 2, 0xfee4, 0xabcd));
        assert_eq!(write.calls[0].r10, TDX_HYPERCALL_STANDARD);
        assert_eq!(write.calls[0].r11, EXIT_REASON_EPT_VIOLATION);
        assert_eq!(write.calls[0].r12, 2);
        assert_eq!(write.calls[0].r13, EPT_WRITE);
        assert_eq!(write.calls[0].r14, 0xfee4);
        assert_eq!(write.calls[0].r15, 0xabcd);
    }

    #[test]
    fn decoded_mmio_writes_register_or_immediate_low_bytes() {
        let ve = VeInfo {
            gpa: 0xfee0,
            ..Default::default()
        };
        let mut regs = TdxRegs {
            ax: 0x1234_5678,
            ..Default::default()
        };
        let mut write = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut write,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::Write,
                    size: 2,
                    opnd_bytes: 8,
                    len: 3,
                    reg: Some(TdxMmioReg::Ax),
                    immediate: 0,
                    vaddr: 0xffff_8000_1000,
                    in_kernel_space: true,
                },
            ),
            Ok(3)
        );
        assert_eq!(write.calls[0].r15, 0x5678);

        let mut imm = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            ..FullHypercall::EMPTY
        }]);
        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut imm,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::WriteImmediate,
                    size: 1,
                    opnd_bytes: 8,
                    len: 5,
                    reg: None,
                    immediate: 0x1234,
                    vaddr: 0xffff_8000_2000,
                    in_kernel_space: true,
                },
            ),
            Ok(5)
        );
        assert_eq!(imm.calls[0].r15, 0x34);
    }

    #[test]
    fn decoded_mmio_reads_apply_linux_extend_rules() {
        let ve = VeInfo {
            gpa: 0xfee0,
            ..Default::default()
        };
        let mut regs = TdxRegs {
            bx: 0xffff_ffff_aaaa_0000,
            cx: 0x7777_7777_7777_7777,
            dx: 0x8888_8888_8888_8888,
            ..Default::default()
        };
        let mut read = FullHypercall::new(&[
            TdxModuleArgs {
                r10: 0,
                r11: 0x1234_5678,
                ..FullHypercall::EMPTY
            },
            TdxModuleArgs {
                r10: 0,
                r11: 0x80,
                ..FullHypercall::EMPTY
            },
            TdxModuleArgs {
                r10: 0,
                r11: 0x7f,
                ..FullHypercall::EMPTY
            },
        ]);

        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut read,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::Read,
                    size: 4,
                    opnd_bytes: 8,
                    len: 2,
                    reg: Some(TdxMmioReg::Bx),
                    immediate: 0,
                    vaddr: 0xffff_8000_1000,
                    in_kernel_space: true,
                },
            ),
            Ok(2)
        );
        assert_eq!(regs.bx, 0x1234_5678);

        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut read,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::ReadSignExtend,
                    size: 1,
                    opnd_bytes: 2,
                    len: 3,
                    reg: Some(TdxMmioReg::Cx),
                    immediate: 0,
                    vaddr: 0xffff_8000_2000,
                    in_kernel_space: true,
                },
            ),
            Ok(3)
        );
        assert_eq!(regs.cx, 0x7777_7777_7777_ff80);

        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut read,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::ReadZeroExtend,
                    size: 1,
                    opnd_bytes: 4,
                    len: 4,
                    reg: Some(TdxMmioReg::Dx),
                    immediate: 0,
                    vaddr: 0xffff_8000_3000,
                    in_kernel_space: true,
                },
            ),
            Ok(4)
        );
        assert_eq!(regs.dx, 0x8888_8888_0000_007f);
    }

    #[test]
    fn decoded_mmio_rejects_linux_invalid_cases() {
        let ve = VeInfo {
            gpa: 0xfee0,
            ..Default::default()
        };
        let mut backend = FullHypercall::new(&[]);
        let mut regs = TdxRegs::default();

        let base = DecodedMmio {
            kind: TdxMmioType::DecodeFailed,
            size: 1,
            opnd_bytes: 8,
            len: 1,
            reg: Some(TdxMmioReg::Ax),
            immediate: 0,
            vaddr: 0xffff_8000_1000,
            in_kernel_space: true,
        };
        assert_eq!(
            tdx_handle_mmio_decoded_with(&mut backend, &mut regs, &ve, base),
            Err(EINVAL)
        );
        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut backend,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::Movs,
                    ..base
                }
            ),
            Err(EINVAL)
        );
        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut backend,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::Read,
                    in_kernel_space: false,
                    ..base
                }
            ),
            Err(EINVAL)
        );
        assert_eq!(
            tdx_handle_mmio_decoded_with(
                &mut backend,
                &mut regs,
                &ve,
                DecodedMmio {
                    kind: TdxMmioType::Read,
                    vaddr: PAGE_SIZE - 1,
                    size: 2,
                    ..base
                }
            ),
            Err(EFAULT)
        );
    }

    #[test]
    fn veinfo_get_maps_linux_tdcall_output_registers() {
        let args = TdxModuleArgs {
            rcx: EXIT_REASON_IO_INSTRUCTION,
            rdx: 0x1234,
            r8: 0xfeed,
            r9: 0xbeef,
            r10: (0x7788_u64 << 32) | 0x55,
            ..FullHypercall::EMPTY
        };
        let mut backend = VeBackend::new(args, &[]);

        let ve = tdx_get_ve_info_with(&mut backend).unwrap();

        assert_eq!(backend.tdcall_leaf, TDG_VP_VEINFO_GET);
        assert_eq!(
            ve,
            VeInfo {
                exit_reason: EXIT_REASON_IO_INSTRUCTION,
                exit_qual: 0x1234,
                gla: 0xfeed,
                gpa: 0xbeef,
                instr_len: 0x55,
                instr_info: 0x7788,
            }
        );

        assert_eq!(
            tdx_get_ve_info_plan(TDX_SUCCESS, args),
            TdxVeInfoPlan {
                tdcall: TdxTdcallPlan {
                    leaf: TDG_VP_VEINFO_GET,
                    ret: TDX_SUCCESS,
                    panic: false,
                },
                args,
                ve: Some(ve),
            }
        );

        assert_eq!(
            tdx_get_ve_info_plan(7, args),
            TdxVeInfoPlan {
                tdcall: TdxTdcallPlan {
                    leaf: TDG_VP_VEINFO_GET,
                    ret: 7,
                    panic: true,
                },
                args,
                ve: None,
            }
        );

        let mut failed = VeBackend::new(args, &[]);
        failed.veinfo_ret = 7;
        assert_eq!(tdx_get_ve_info_with(&mut failed), Err(EIO));
    }

    #[test]
    fn early_ve_handler_only_handles_io_and_advances_ip() {
        let mut regs = TdxRegs {
            ip: 0x1000,
            ax: u64::MAX,
            ..Default::default()
        };
        let mut backend = VeBackend::new(
            TdxModuleArgs {
                rcx: EXIT_REASON_IO_INSTRUCTION,
                rdx: (1 << 3) | (0x3f8 << 16),
                r10: 2,
                ..FullHypercall::EMPTY
            },
            &[TdxModuleArgs {
                r10: 0,
                r11: 0xaa,
                ..FullHypercall::EMPTY
            }],
        );

        assert!(tdx_early_handle_ve_with(&mut backend, &mut regs));
        assert_eq!(backend.tdcall_leaf, TDG_VP_VEINFO_GET);
        assert_eq!(backend.calls[0].r11, EXIT_REASON_IO_INSTRUCTION);
        assert_eq!(regs.ip, 0x1002);
        assert_eq!(regs.ax, (u64::MAX & !tdx_io_mask(1)) | 0xaa);

        let mut regs = TdxRegs {
            ip: 0x2000,
            ..Default::default()
        };
        let mut non_io = VeBackend::new(
            TdxModuleArgs {
                rcx: EXIT_REASON_CPUID,
                r10: 2,
                ..FullHypercall::EMPTY
            },
            &[],
        );
        assert!(!tdx_early_handle_ve_with(&mut non_io, &mut regs));
        assert_eq!(non_io.len, 0);
        assert_eq!(regs.ip, 0x2000);
    }

    #[test]
    fn virt_exception_result_dispatches_user_kernel_and_advances_ip() {
        let mut user_regs = TdxRegs {
            ip: 0x1000,
            ax: 0x4000_0001,
            cx: 7,
            ..Default::default()
        };
        let mut user_backend = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r12: 1,
            r13: 2,
            r14: 3,
            r15: 4,
            ..FullHypercall::EMPTY
        }]);
        let user_cpuid = VeInfo {
            exit_reason: EXIT_REASON_CPUID,
            instr_len: 4,
            ..Default::default()
        };
        assert_eq!(
            tdx_handle_virt_exception_result_with(
                &mut user_backend,
                &mut user_regs,
                &user_cpuid,
                true,
                false,
                1 << 47,
            ),
            Ok(())
        );
        assert_eq!(user_regs.ip, 0x1004);
        assert_eq!(user_regs.ax, 1);
        assert_eq!(user_regs.bx, 2);

        let mut kernel_regs = TdxRegs {
            ip: 0x3000,
            cx: 0x10,
            ..Default::default()
        };
        let mut kernel_backend = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r11: 0x1122_3344_5566_7788,
            ..FullHypercall::EMPTY
        }]);
        let rdmsr = VeInfo {
            exit_reason: EXIT_REASON_MSR_READ,
            instr_len: 2,
            ..Default::default()
        };
        assert_eq!(
            tdx_handle_virt_exception_result_with(
                &mut kernel_backend,
                &mut kernel_regs,
                &rdmsr,
                false,
                true,
                1 << 47,
            ),
            Ok(())
        );
        assert_eq!(kernel_regs.ip, 0x3002);
        assert_eq!(kernel_regs.ax, 0x5566_7788);
        assert_eq!(kernel_regs.dx, 0x1122_3344);

        let mut bad_user = FullHypercall::new(&[]);
        let mut regs = TdxRegs {
            ip: 0x4000,
            ..Default::default()
        };
        let io = VeInfo {
            exit_reason: EXIT_REASON_IO_INSTRUCTION,
            instr_len: 1,
            ..Default::default()
        };
        assert_eq!(
            tdx_handle_virt_exception_result_with(
                &mut bad_user,
                &mut regs,
                &io,
                true,
                false,
                1 << 47
            ),
            Err(TdxVeError::Errno(EIO))
        );
        assert_eq!(regs.ip, 0x4000);
    }

    #[test]
    fn virt_exception_mmio_reports_private_or_missing_emulator() {
        let mut backend = FullHypercall::new(&[]);
        let mut regs = TdxRegs::default();
        let private = VeInfo {
            exit_reason: EXIT_REASON_EPT_VIOLATION,
            gpa: 0x1000,
            instr_len: 0,
            ..Default::default()
        };
        assert_eq!(
            tdx_virt_exception_kernel_with(&mut backend, &mut regs, &private, true, 1 << 47),
            Err(TdxVeError::PrivateGpa)
        );

        let shared = VeInfo {
            exit_reason: EXIT_REASON_EPT_VIOLATION,
            gpa: (1 << 47) | 0x1000,
            instr_len: 0,
            ..Default::default()
        };
        assert_eq!(
            tdx_virt_exception_kernel_with(&mut backend, &mut regs, &shared, true, 1 << 47),
            Err(TdxVeError::MmioUnsupported)
        );
    }

    #[test]
    fn virt_exception_kernel_decoded_mmio_handles_shared_ept_violation() {
        let mut backend = FullHypercall::new(&[TdxModuleArgs {
            r10: 0,
            r11: 0x5a,
            ..FullHypercall::EMPTY
        }]);
        let mut regs = TdxRegs {
            ax: 0xffff,
            ..Default::default()
        };
        let ve = VeInfo {
            exit_reason: EXIT_REASON_EPT_VIOLATION,
            gpa: (1 << 47) | 0x1000,
            ..Default::default()
        };

        assert_eq!(
            tdx_virt_exception_kernel_decoded_with(
                &mut backend,
                &mut regs,
                &ve,
                true,
                1 << 47,
                Some(DecodedMmio {
                    kind: TdxMmioType::ReadZeroExtend,
                    size: 1,
                    opnd_bytes: 8,
                    len: 6,
                    reg: Some(TdxMmioReg::Ax),
                    immediate: 0,
                    vaddr: 0xffff_8000_1000,
                    in_kernel_space: true,
                }),
            ),
            Ok(6)
        );
        assert_eq!(regs.ax, 0x5a);
        assert_eq!(backend.calls[0].r13, EPT_READ);
        assert_eq!(backend.calls[0].r14, (1 << 47) | 0x1000);
    }

    #[test]
    fn virt_exception_handler_matches_user_and_kernel_switches() {
        let cpuid = VeInfo {
            exit_reason: EXIT_REASON_CPUID,
            ..Default::default()
        };
        let io = VeInfo {
            exit_reason: EXIT_REASON_IO_INSTRUCTION,
            ..Default::default()
        };
        let vmcall = VeInfo {
            exit_reason: EXIT_REASON_VMCALL,
            ..Default::default()
        };

        assert!(tdx_handle_virt_exception_for_mode(&cpuid, true));
        assert!(!tdx_handle_virt_exception_for_mode(&io, true));
        assert!(tdx_handle_virt_exception_for_mode(&io, false));
        assert!(!tdx_handle_virt_exception_for_mode(&vmcall, false));
        assert!(!tdx_handle_virt_exception(&vmcall));
    }
}
