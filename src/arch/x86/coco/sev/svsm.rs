//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev/svsm.c
//! test-origin: linux:vendor/linux/arch/x86/coco/sev/svsm.c
//! Secure VM Service Module call-area helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/svsm.c

use core::sync::atomic::{AtomicU64, Ordering};

use crate::include::uapi::errno::{EAGAIN, EINVAL, EOPNOTSUPP};

use super::core::SevTermination;
use super::internal;
use super::vc_handle::{VcForwardExceptionPlan, vc_forward_exception};
use super::vc_shared::{
    EsEmCtxt, EsResult, GHCB_DEFAULT_USAGE, Ghcb, SVM_EVTINJ_TYPE_EXEPT, SVM_EVTINJ_VALID,
    SVM_VMGEXIT_SNP_RUN_VMPL, X86_TRAP_GP, ghcb_protocol_version, ghcb_set_sw_exit_code,
    ghcb_set_sw_exit_info_1, ghcb_set_sw_exit_info_2, vc_ghcb_invalidate, verify_exception_info,
};

pub const SVSM_CORE_PVALIDATE: u32 = 1;
pub const SVSM_CORE_CREATE_VCPU: u32 = 2;
pub const SVSM_CORE_DELETE_VCPU: u32 = 3;
pub const SVSM_SUCCESS: u64 = 0;
pub const SVSM_ERR_INCOMPLETE: u64 = 0x8000_0000;
pub const SVSM_ERR_UNSUPPORTED_PROTOCOL: u64 = 0x8000_0001;
pub const SVSM_ERR_UNSUPPORTED_CALL: u64 = 0x8000_0002;
pub const SVSM_ERR_INVALID_ADDRESS: u64 = 0x8000_0003;
pub const SVSM_ERR_INVALID_FORMAT: u64 = 0x8000_0004;
pub const SVSM_ERR_INVALID_PARAMETER: u64 = 0x8000_0005;
pub const SVSM_ERR_INVALID_REQUEST: u64 = 0x8000_0006;
pub const SVSM_ERR_BUSY: u64 = 0x8000_0007;
pub const SVSM_PVALIDATE_FAIL_SIZEMISMATCH: u32 = 0x8000_1006;
pub const SVSM_PVALIDATE_MAX_COUNT: usize = 510;
pub const SVSM_ATTEST_SERVICES: u64 = 0;
pub const SVSM_ATTEST_SINGLE_SERVICE: u64 = 1;
pub const SVSM_VTPM_QUERY: u64 = 0;
pub const SVSM_VTPM_CMD: u64 = 1;
pub const SVSM_VTPM_SEND_COMMAND_BIT: u64 = 8;

static SVSM_PVALIDATE_BUFFER_PA: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
const SVSM_PVAL_LOG_CAP: usize = 8;
#[cfg(test)]
static SVSM_PVAL_LOG_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static SVSM_PVAL_LOG: spin::Mutex<[(usize, u64, u32, bool); SVSM_PVAL_LOG_CAP]> =
    spin::Mutex::new([(0, 0, 0, false); SVSM_PVAL_LOG_CAP]);
#[cfg(test)]
static TEST_SVSM_PVAL_SUCCESS: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
static TEST_SVSM_PVAL_2M_SIZE_MISMATCH: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
const SVSM_BACKEND_LOG_CAP: usize = 8;
#[cfg(test)]
static SVSM_BACKEND_LOG_LEN: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static SVSM_BACKEND_LOG: spin::Mutex<[(u64, u64, u64, u64, u64); SVSM_BACKEND_LOG_CAP]> =
    spin::Mutex::new([(0, 0, 0, 0, 0); SVSM_BACKEND_LOG_CAP]);
#[cfg(test)]
static TEST_SVSM_BACKEND_STATUS: spin::Mutex<[u64; 2]> = spin::Mutex::new([EOPNOTSUPP as u64; 2]);
#[cfg(test)]
static TEST_SVSM_BACKEND_CALLS: core::sync::atomic::AtomicUsize =
    core::sync::atomic::AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvsmCall {
    pub rax: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub r8: u64,
    pub r9: u64,
    pub ret: u64,
    pub rax_out: u64,
    pub rcx_out: u64,
    pub rdx_out: u64,
    pub r8_out: u64,
    pub r9_out: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvsmCa {
    pub call_pending: bool,
    pub mem_available: bool,
    pub call: SvsmCall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvsmPvalidateRange {
    pub start_pfn: u64,
    pub end_pfn: u64,
    pub make_private: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvsmLocEntry {
    pub pa: u64,
    pub len: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SvsmAttestCall {
    pub report_buf: SvsmLocEntry,
    pub nonce: SvsmLocEntry,
    pub manifest_buf: SvsmLocEntry,
    pub certificates_buf: SvsmLocEntry,
    pub service_guid: [u8; 16],
    pub service_manifest_ver: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvsmPvalidateEntry {
    pub page_size: u32,
    pub action: bool,
    pub ignore_cf: bool,
    pub pfn: u64,
}

const EMPTY_PVALIDATE_ENTRY: SvsmPvalidateEntry = SvsmPvalidateEntry {
    page_size: 0,
    action: false,
    ignore_cf: false,
    pfn: 0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvsmPvalidateCall {
    pub num_entries: usize,
    pub cur_index: usize,
    pub entries: [SvsmPvalidateEntry; SVSM_PVALIDATE_MAX_COUNT],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvsmPvalidateTerminationPlan {
    pub pfn: u64,
    pub action: bool,
    pub page_size: u32,
    pub ret: i32,
    pub svsm_ret: u64,
    pub termination: SevTermination,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SvsmGhcbVerifyPlan {
    pub result: EsResult,
    pub forward_exception: Option<VcForwardExceptionPlan>,
    pub errno: Option<i32>,
}

impl SvsmPvalidateCall {
    pub const fn empty() -> Self {
        Self {
            num_entries: 0,
            cur_index: 0,
            entries: [EMPTY_PVALIDATE_ENTRY; SVSM_PVALIDATE_MAX_COUNT],
        }
    }

    fn reset(&mut self) {
        self.num_entries = 0;
        self.cur_index = 0;
        self.entries = [EMPTY_PVALIDATE_ENTRY; SVSM_PVALIDATE_MAX_COUNT];
    }
}

pub fn svsm_perform_ghcb_protocol(ghcb: &mut Ghcb, call: &mut SvsmCall) -> Result<(), i32> {
    let mut ca = SvsmCa::default();
    svsm_perform_ghcb_protocol_with(ghcb, &mut ca, call, |_| {})
}

pub fn svsm_perform_ghcb_protocol_with(
    ghcb: &mut Ghcb,
    ca: &mut SvsmCa,
    call: &mut SvsmCall,
    hypervisor: impl FnOnce(&mut Ghcb),
) -> Result<(), i32> {
    svsm_perform_ghcb_protocol_with_protocol(ghcb, ca, call, ghcb_protocol_version(), hypervisor)
}

pub fn svsm_perform_ghcb_protocol_with_protocol(
    ghcb: &mut Ghcb,
    ca: &mut SvsmCa,
    call: &mut SvsmCall,
    protocol_version: u16,
    hypervisor: impl FnOnce(&mut Ghcb),
) -> Result<(), i32> {
    vc_ghcb_invalidate(ghcb);
    ghcb.protocol_version = protocol_version;
    ghcb.ghcb_usage = GHCB_DEFAULT_USAGE;
    ghcb_set_sw_exit_code(ghcb, SVM_VMGEXIT_SNP_RUN_VMPL);
    ghcb_set_sw_exit_info_1(ghcb, 0);
    ghcb_set_sw_exit_info_2(ghcb, 0);

    let mut pending = false;
    svsm_issue_call(ca, call, &mut pending);
    hypervisor(ghcb);

    if pending {
        return Err(EINVAL);
    }

    let plan = svsm_ghcb_verify_plan(ghcb);
    match plan.errno {
        None => svsm_process_result_codes(call),
        Some(errno) => Err(errno),
    }
}

pub fn svsm_perform_call_protocol(call: &mut SvsmCall) -> Result<(), i32> {
    let mut ca = SvsmCa::default();
    loop {
        let ret = svsm_perform_call_protocol_with_ca(&mut ca, call);
        if ret != Err(EAGAIN) {
            return ret;
        }
    }
}

pub const fn svsm_core_call(call: u64) -> u64 {
    call
}

pub const fn svsm_attest_call(call: u64) -> u64 {
    (1 << 32) | call
}

pub const fn svsm_vtpm_call(call: u64) -> u64 {
    (2 << 32) | call
}

pub fn set_svsm_pvalidate_buffer_pa(pa: u64) {
    SVSM_PVALIDATE_BUFFER_PA.store(pa, Ordering::Release);
}

pub fn svsm_pvalidate_buffer_pa() -> u64 {
    SVSM_PVALIDATE_BUFFER_PA.load(Ordering::Acquire)
}

fn svsm_perform_call_protocol_with_ca(ca: &mut SvsmCa, call: &mut SvsmCall) -> Result<(), i32> {
    let mut pending = false;
    svsm_issue_call(ca, call, &mut pending);
    if pending {
        return Err(EINVAL);
    }
    svsm_process_result_codes(call)
}

fn svsm_issue_call(ca: &mut SvsmCa, call: &mut SvsmCall, pending: &mut bool) {
    ca.call_pending = true;
    ca.call = *call;

    svsm_issue_call_backend(ca);

    *pending = ca.call_pending;
    ca.call_pending = false;

    call.rax_out = ca.call.rax_out;
    call.rcx_out = ca.call.rcx_out;
    call.rdx_out = ca.call.rdx_out;
    call.r8_out = ca.call.r8_out;
    call.r9_out = ca.call.r9_out;
    call.ret = call.rax_out;
}

#[cfg(test)]
fn svsm_issue_call_backend(ca: &mut SvsmCa) {
    record_svsm_backend_call(&ca.call);
    let idx = TEST_SVSM_BACKEND_CALLS.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    let status = TEST_SVSM_BACKEND_STATUS.lock()[idx.min(1)];
    if status == EOPNOTSUPP as u64 {
        return;
    }

    ca.call_pending = false;
    ca.call.rax_out = status;
    ca.call.rcx_out = ca.call.rcx.wrapping_add(1);
    ca.call.rdx_out = ca.call.rdx.wrapping_add(2);
    ca.call.r8_out = ca.call.r8.wrapping_add(3);
    ca.call.r9_out = ca.call.r9.wrapping_add(4);
}

#[cfg(not(test))]
fn svsm_issue_call_backend(_ca: &mut SvsmCa) {}

pub const fn svsm_process_result_codes(call: &SvsmCall) -> Result<(), i32> {
    match call.rax_out {
        SVSM_SUCCESS => Ok(()),
        SVSM_ERR_INCOMPLETE | SVSM_ERR_BUSY => Err(EAGAIN),
        _ => Err(EINVAL),
    }
}

pub fn svsm_ghcb_verify_plan(ghcb: &Ghcb) -> SvsmGhcbVerifyPlan {
    let mut ctxt = EsEmCtxt::default();
    let result = verify_exception_info(ghcb, &mut ctxt);
    match result {
        EsResult::Continue => SvsmGhcbVerifyPlan {
            result,
            forward_exception: None,
            errno: None,
        },
        EsResult::Exception => SvsmGhcbVerifyPlan {
            result,
            forward_exception: Some(vc_forward_exception(&ctxt)),
            errno: Some(EINVAL),
        },
        EsResult::VmmError
        | EsResult::DecodeFailed
        | EsResult::Terminate
        | EsResult::Unsupported
        | EsResult::Retry => SvsmGhcbVerifyPlan {
            result,
            forward_exception: None,
            errno: Some(EINVAL),
        },
    }
}

pub const fn svsm_build_ca_from_pfn_range(
    pfn: u64,
    pfn_end: u64,
    make_private: bool,
) -> Result<SvsmPvalidateRange, i32> {
    Ok(SvsmPvalidateRange {
        start_pfn: pfn,
        end_pfn: pfn_end,
        make_private,
    })
}

pub fn svsm_pval_terminate_plan(
    pc: &SvsmPvalidateCall,
    ret: i32,
    svsm_ret: u64,
) -> Option<SvsmPvalidateTerminationPlan> {
    if pc.cur_index >= pc.num_entries {
        return None;
    }

    let entry = pc.entries[pc.cur_index];
    let plan =
        internal::pval_terminate_plan(entry.pfn, entry.action, entry.page_size, ret, svsm_ret);
    Some(SvsmPvalidateTerminationPlan {
        pfn: plan.pfn,
        action: plan.action,
        page_size: plan.page_size,
        ret: plan.ret,
        svsm_ret: plan.svsm_ret,
        termination: plan.termination,
    })
}

fn svsm_build_ca_entries_from_pfn_range(
    mut pfn: u64,
    pfn_end: u64,
    action: bool,
    pc: &mut SvsmPvalidateCall,
) -> u64 {
    pc.reset();

    while pfn < pfn_end && pc.num_entries < SVSM_PVALIDATE_MAX_COUNT {
        pc.entries[pc.num_entries] = SvsmPvalidateEntry {
            page_size: super::core::RMP_PG_SIZE_4K,
            action,
            ignore_cf: false,
            pfn,
        };
        pc.num_entries += 1;
        pfn += 1;
    }

    pfn
}

pub fn svsm_build_ca_from_psc_desc(
    desc: &super::core::PscDesc,
    mut desc_entry: usize,
    pc: &mut SvsmPvalidateCall,
) -> usize {
    pc.reset();

    while desc_entry < desc.count && pc.num_entries < SVSM_PVALIDATE_MAX_COUNT {
        let Some(entry) = desc.entries[desc_entry] else {
            break;
        };
        pc.entries[pc.num_entries] = SvsmPvalidateEntry {
            page_size: if entry.pagesize == super::core::RMP_PG_SIZE_2M {
                super::core::RMP_PG_SIZE_2M
            } else {
                super::core::RMP_PG_SIZE_4K
            },
            action: entry.operation == super::core::VMGEXIT_PSC_OP_PRIVATE,
            ignore_cf: false,
            pfn: entry.gfn,
        };
        pc.num_entries += 1;
        desc_entry += 1;
    }

    desc_entry
}

fn svsm_perform_pvalidate_call(call: &mut SvsmCall, pc: &mut SvsmPvalidateCall) -> Result<(), i32> {
    #[cfg(test)]
    {
        record_svsm_pval_call(pc);
        if TEST_SVSM_PVAL_SUCCESS.load(core::sync::atomic::Ordering::Acquire) {
            if TEST_SVSM_PVAL_2M_SIZE_MISMATCH.load(core::sync::atomic::Ordering::Acquire)
                && pc.num_entries != 0
                && pc.entries[pc.cur_index].page_size == super::core::RMP_PG_SIZE_2M
            {
                call.rax_out = SVSM_PVALIDATE_FAIL_SIZEMISMATCH as u64;
                call.ret = SVSM_PVALIDATE_FAIL_SIZEMISMATCH as u64;
                return Err(EINVAL);
            }
            call.rax_out = SVSM_SUCCESS;
            call.ret = 0;
            return Ok(());
        }
    }

    svsm_perform_call_protocol(call)
}

pub fn svsm_pval_pages(desc: &super::core::PscDesc) -> Result<(), i32> {
    let mut pv_4k: [Option<SvsmPvalidateEntry>; super::core::VMGEXIT_PSC_MAX_ENTRY] =
        [None; super::core::VMGEXIT_PSC_MAX_ENTRY];
    let mut pv_4k_count = 0usize;
    let mut pc = SvsmPvalidateCall::empty();
    let mut call = SvsmCall {
        rax: SVSM_CORE_PVALIDATE as u64,
        rcx: svsm_pvalidate_buffer_pa(),
        ..Default::default()
    };

    let mut i = 0usize;
    while i < desc.count {
        i = svsm_build_ca_from_psc_desc(desc, i, &mut pc);

        loop {
            match svsm_perform_pvalidate_call(&mut call, &mut pc) {
                Ok(()) => break,
                Err(_err)
                    if call.rax_out == SVSM_PVALIDATE_FAIL_SIZEMISMATCH as u64
                        && pc.cur_index < pc.num_entries
                        && pc.entries[pc.cur_index].page_size == super::core::RMP_PG_SIZE_2M =>
                {
                    if pv_4k_count == pv_4k.len() {
                        return Err(EOPNOTSUPP);
                    }
                    pv_4k[pv_4k_count] = Some(pc.entries[pc.cur_index]);
                    pv_4k_count += 1;
                    pc.cur_index += 1;
                    if pc.cur_index >= pc.num_entries {
                        break;
                    }
                }
                Err(err) => {
                    let _ = svsm_pval_terminate_plan(&pc, err, call.rax_out);
                    return Err(err);
                }
            }
        }
    }

    let mut idx = 0usize;
    while idx < pv_4k_count {
        let Some(entry) = pv_4k[idx] else {
            return Err(EOPNOTSUPP);
        };
        let action = entry.action;
        let mut pfn = entry.pfn;
        let pfn_end = entry.pfn.wrapping_add(512);
        while pfn < pfn_end {
            pfn = svsm_build_ca_entries_from_pfn_range(pfn, pfn_end, action, &mut pc);
            if let Err(err) = svsm_perform_pvalidate_call(&mut call, &mut pc) {
                let _ = svsm_pval_terminate_plan(&pc, err, call.rax_out);
                return Err(err);
            }
        }
        idx += 1;
    }

    Ok(())
}

pub trait SvsmCallOps {
    fn svsm_buffer_pa(&self) -> u64;

    fn copy_attest_input(&mut self, _input: &SvsmAttestCall) {}

    fn perform_call_protocol(&mut self, call: &mut SvsmCall) -> Result<(), i32>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultSvsmCallOps;

impl SvsmCallOps for DefaultSvsmCallOps {
    fn svsm_buffer_pa(&self) -> u64 {
        svsm_pvalidate_buffer_pa()
    }

    fn perform_call_protocol(&mut self, call: &mut SvsmCall) -> Result<(), i32> {
        svsm_perform_call_protocol(call)
    }
}

fn update_attest_input(call: &SvsmCall, input: &mut SvsmAttestCall) {
    if call.rcx_out != call.rcx {
        input.manifest_buf.len = call.rcx_out as u32;
    }
    if call.rdx_out != call.rdx {
        input.certificates_buf.len = call.rdx_out as u32;
    }
    if call.r8_out != call.r8 {
        input.report_buf.len = call.r8_out as u32;
    }
}

pub fn snp_issue_svsm_attest_req_with<O: SvsmCallOps>(
    ops: &mut O,
    snp_vmpl: u8,
    call_id: u64,
    call: &mut SvsmCall,
    input: &mut SvsmAttestCall,
) -> Result<(), i32> {
    if snp_vmpl == 0 {
        return Err(EINVAL);
    }

    ops.copy_attest_input(input);
    call.rax = call_id;
    call.rcx = ops.svsm_buffer_pa();
    call.rdx = u64::MAX;
    call.r8 = u64::MAX;
    let ret = ops.perform_call_protocol(call);
    update_attest_input(call, input);
    ret
}

pub fn snp_svsm_vtpm_send_command_with<O: SvsmCallOps>(
    ops: &mut O,
    buffer_pa: u64,
) -> Result<(), i32> {
    let mut call = SvsmCall {
        rax: svsm_vtpm_call(SVSM_VTPM_CMD),
        rcx: buffer_pa,
        ..Default::default()
    };
    ops.perform_call_protocol(&mut call)
}

pub fn snp_svsm_vtpm_probe_with<O: SvsmCallOps>(ops: &mut O, snp_vmpl: u8) -> bool {
    if snp_vmpl == 0 {
        return false;
    }
    let mut call = SvsmCall {
        rax: svsm_vtpm_call(SVSM_VTPM_QUERY),
        ..Default::default()
    };
    if ops.perform_call_protocol(&mut call).is_err() {
        return false;
    }
    call.rcx_out & (1 << SVSM_VTPM_SEND_COMMAND_BIT) != 0
}

pub const fn snp_svsm_vtpm_probe(svsm_present: bool, supported_commands: u64) -> bool {
    svsm_present && supported_commands & (1 << SVSM_VTPM_SEND_COMMAND_BIT) != 0
}

pub fn snp_issue_svsm_attest_req(
    call_id: u64,
    call: &mut SvsmCall,
    input: &mut SvsmAttestCall,
) -> Result<(), i32> {
    let mut ops = DefaultSvsmCallOps;
    snp_issue_svsm_attest_req_with(&mut ops, super::core::snp_vmpl(), call_id, call, input)
}

pub fn snp_svsm_vtpm_send_command(buffer: *mut u8) -> Result<(), i32> {
    let buffer_va = buffer as u64;
    let buffer_pa = crate::arch::x86::mm::paging::virt_to_phys(buffer_va).unwrap_or(buffer_va);
    let mut ops = DefaultSvsmCallOps;
    snp_svsm_vtpm_send_command_with(&mut ops, buffer_pa)
}

#[cfg(test)]
fn record_svsm_pval_call(pc: &SvsmPvalidateCall) {
    let idx = SVSM_PVAL_LOG_LEN.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if idx < SVSM_PVAL_LOG_CAP {
        let first = pc.entries[0];
        SVSM_PVAL_LOG.lock()[idx] = (pc.num_entries, first.pfn, first.page_size, first.action);
    }
}

#[cfg(test)]
fn record_svsm_backend_call(call: &SvsmCall) {
    let idx = SVSM_BACKEND_LOG_LEN.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    if idx < SVSM_BACKEND_LOG_CAP {
        SVSM_BACKEND_LOG.lock()[idx] = (call.rax, call.rcx, call.rdx, call.r8, call.r9);
    }
}

#[cfg(test)]
fn reset_svsm_backend(status0: u64, status1: u64) {
    SVSM_BACKEND_LOG_LEN.store(0, core::sync::atomic::Ordering::Release);
    TEST_SVSM_BACKEND_CALLS.store(0, core::sync::atomic::Ordering::Release);
    *SVSM_BACKEND_LOG.lock() = [(0, 0, 0, 0, 0); SVSM_BACKEND_LOG_CAP];
    *TEST_SVSM_BACKEND_STATUS.lock() = [status0, status1];
}

#[cfg(test)]
fn svsm_backend_log() -> (usize, [(u64, u64, u64, u64, u64); SVSM_BACKEND_LOG_CAP]) {
    (
        SVSM_BACKEND_LOG_LEN
            .load(core::sync::atomic::Ordering::Acquire)
            .min(SVSM_BACKEND_LOG_CAP),
        *SVSM_BACKEND_LOG.lock(),
    )
}

#[cfg(test)]
pub fn reset_svsm_pval_log() {
    SVSM_PVAL_LOG_LEN.store(0, core::sync::atomic::Ordering::Release);
    *SVSM_PVAL_LOG.lock() = [(0, 0, 0, false); SVSM_PVAL_LOG_CAP];
}

#[cfg(test)]
pub fn svsm_pval_log() -> (usize, [(usize, u64, u32, bool); SVSM_PVAL_LOG_CAP]) {
    (
        SVSM_PVAL_LOG_LEN
            .load(core::sync::atomic::Ordering::Acquire)
            .min(SVSM_PVAL_LOG_CAP),
        *SVSM_PVAL_LOG.lock(),
    )
}

#[cfg(test)]
pub fn set_test_svsm_pval_success(enabled: bool) {
    TEST_SVSM_PVAL_SUCCESS.store(enabled, core::sync::atomic::Ordering::Release);
}

#[cfg(test)]
pub fn set_test_svsm_pval_2m_size_mismatch(enabled: bool) {
    TEST_SVSM_PVAL_2M_SIZE_MISMATCH.store(enabled, core::sync::atomic::Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::super::core::publish_snp_vmpl;
    use super::super::vc_handle::VcForwardExceptionAction;
    use super::super::vc_shared::{
        GHCB_PROTOCOL_MAX, ghcb_rax_is_valid, ghcb_set_rax, ghcb_version_test_guard,
    };
    use super::*;

    static TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn svsm_test_guard() -> spin::MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock();
        set_svsm_pvalidate_buffer_pa(0);
        publish_snp_vmpl(0);
        guard
    }

    #[test]
    fn pvalidate_range_keeps_linux_empty_or_reversed_shape() {
        let _guard = svsm_test_guard();
        let empty = svsm_build_ca_from_pfn_range(3, 3, true).unwrap();
        assert_eq!(empty.start_pfn, 3);
        assert_eq!(empty.end_pfn, 3);
        assert!(empty.make_private);
        let reversed = svsm_build_ca_from_pfn_range(5, 3, true).unwrap();
        assert_eq!(reversed.start_pfn, 5);
        assert_eq!(reversed.end_pfn, 3);
        let range = svsm_build_ca_from_pfn_range(3, 5, false).unwrap();
        assert_eq!(range.start_pfn, 3);
        assert_eq!(range.end_pfn, 5);
        assert!(!range.make_private);
    }

    #[test]
    fn svsm_call_protocol_fails_closed_without_backend() {
        let _guard = svsm_test_guard();
        reset_svsm_backend(EOPNOTSUPP as u64, EOPNOTSUPP as u64);
        let mut call = SvsmCall {
            rax: SVSM_CORE_PVALIDATE as u64,
            ..Default::default()
        };
        assert_eq!(svsm_perform_call_protocol(&mut call), Err(EINVAL));
        assert_eq!(call.ret, 0);
    }

    #[test]
    fn svsm_process_result_codes_matches_linux_mapping() {
        let _guard = svsm_test_guard();
        let mut call = SvsmCall {
            rax_out: SVSM_SUCCESS,
            ..Default::default()
        };
        assert_eq!(svsm_process_result_codes(&call), Ok(()));
        call.rax_out = SVSM_ERR_BUSY;
        assert_eq!(svsm_process_result_codes(&call), Err(EAGAIN));
        call.rax_out = SVSM_ERR_INCOMPLETE;
        assert_eq!(svsm_process_result_codes(&call), Err(EAGAIN));
        call.rax_out = SVSM_ERR_UNSUPPORTED_CALL;
        assert_eq!(svsm_process_result_codes(&call), Err(EINVAL));
    }

    #[test]
    fn svsm_call_protocol_sets_pending_and_captures_output_registers() {
        let _guard = svsm_test_guard();
        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        let mut call = SvsmCall {
            rax: SVSM_CORE_PVALIDATE as u64,
            rcx: 10,
            rdx: 20,
            r8: 30,
            r9: 40,
            ..Default::default()
        };
        assert_eq!(svsm_perform_call_protocol(&mut call), Ok(()));
        assert_eq!(call.rax_out, SVSM_SUCCESS);
        assert_eq!(call.rcx_out, 11);
        assert_eq!(call.rdx_out, 22);
        assert_eq!(call.r8_out, 33);
        assert_eq!(call.r9_out, 44);

        let (len, log) = svsm_backend_log();
        assert_eq!(len, 1);
        assert_eq!(log[0], (SVSM_CORE_PVALIDATE as u64, 10, 20, 30, 40));
    }

    #[test]
    fn svsm_call_protocol_allows_zero_call_id_like_linux() {
        let _guard = svsm_test_guard();
        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        let mut call = SvsmCall {
            rax: svsm_core_call(0),
            rcx: 0x55,
            ..Default::default()
        };
        assert_eq!(svsm_perform_call_protocol(&mut call), Ok(()));
        let (len, log) = svsm_backend_log();
        assert_eq!(len, 1);
        assert_eq!(log[0].0, 0);
        assert_eq!(log[0].1, 0x55);
    }

    #[test]
    fn svsm_ghcb_protocol_sets_linux_exit_fields_and_processes_result() {
        let _guard = svsm_test_guard();
        let _version_guard = ghcb_version_test_guard(GHCB_PROTOCOL_MAX);
        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        let mut ghcb = Ghcb::default();
        ghcb.shared_buffer[0] = 0xaa;
        ghcb_set_rax(&mut ghcb, 0xdead_beef);
        let mut ca = SvsmCa::default();
        let mut call = SvsmCall {
            rax: SVSM_CORE_PVALIDATE as u64,
            rcx: 0x1000,
            rdx: 0x2000,
            r8: 0x3000,
            r9: 0x4000,
            ..Default::default()
        };

        assert_eq!(
            svsm_perform_ghcb_protocol_with(&mut ghcb, &mut ca, &mut call, |_| {}),
            Ok(())
        );
        assert_eq!(ghcb.protocol_version, GHCB_PROTOCOL_MAX);
        assert_eq!(ghcb.ghcb_usage, GHCB_DEFAULT_USAGE);
        assert_eq!(ghcb.save.sw_exit_code, SVM_VMGEXIT_SNP_RUN_VMPL);
        assert_eq!(ghcb.save.sw_exit_info_1, 0);
        assert_eq!(ghcb.save.sw_exit_info_2, 0);
        assert_eq!(ghcb.shared_buffer[0], 0xaa);
        assert_eq!(ghcb.save.rax, 0xdead_beef);
        assert!(!ghcb_rax_is_valid(&ghcb));
        assert_eq!(call.rax_out, SVSM_SUCCESS);
        assert_eq!(call.rcx_out, 0x1001);
        assert_eq!(call.rdx_out, 0x2002);
        assert_eq!(call.r8_out, 0x3003);
        assert_eq!(call.r9_out, 0x4004);
    }

    #[test]
    fn svsm_ghcb_protocol_uses_negotiated_ghcb_version_like_linux() {
        let _guard = svsm_test_guard();
        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        let mut ghcb = Ghcb::default();
        let mut ca = SvsmCa::default();
        let mut call = SvsmCall {
            rax: SVSM_CORE_PVALIDATE as u64,
            ..Default::default()
        };

        assert_eq!(
            svsm_perform_ghcb_protocol_with_protocol(&mut ghcb, &mut ca, &mut call, 1, |_| {}),
            Ok(())
        );
        assert_eq!(ghcb.protocol_version, 1);
        assert_eq!(ghcb.ghcb_usage, GHCB_DEFAULT_USAGE);
        assert_eq!(ghcb.save.sw_exit_code, SVM_VMGEXIT_SNP_RUN_VMPL);
    }

    #[test]
    fn svsm_ghcb_protocol_rejects_pending_or_verified_exception() {
        let _guard = svsm_test_guard();
        reset_svsm_backend(EOPNOTSUPP as u64, EOPNOTSUPP as u64);
        let mut ghcb = Ghcb::default();
        let mut ca = SvsmCa::default();
        let mut call = SvsmCall {
            rax: SVSM_CORE_PVALIDATE as u64,
            ..Default::default()
        };

        assert_eq!(
            svsm_perform_ghcb_protocol_with(&mut ghcb, &mut ca, &mut call, |_| {}),
            Err(EINVAL)
        );

        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        assert_eq!(
            svsm_perform_ghcb_protocol_with(&mut ghcb, &mut ca, &mut call, |ghcb| {
                ghcb_set_sw_exit_info_1(ghcb, 1);
                ghcb_set_sw_exit_info_2(
                    ghcb,
                    SVM_EVTINJ_VALID | SVM_EVTINJ_TYPE_EXEPT | X86_TRAP_GP,
                );
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn svsm_ghcb_verify_plan_forwards_exception_before_einval_like_linux() {
        let _guard = svsm_test_guard();
        let mut ghcb = Ghcb::default();
        ghcb_set_sw_exit_info_1(&mut ghcb, 1);
        ghcb_set_sw_exit_info_2(
            &mut ghcb,
            SVM_EVTINJ_VALID | SVM_EVTINJ_TYPE_EXEPT | X86_TRAP_GP,
        );

        let plan = svsm_ghcb_verify_plan(&ghcb);
        assert_eq!(plan.result, EsResult::Exception);
        assert_eq!(plan.errno, Some(EINVAL));
        let forward = plan.forward_exception.unwrap();
        assert_eq!(forward.trapnr, X86_TRAP_GP);
        assert_eq!(forward.error_code, 0);
        assert_eq!(forward.action, VcForwardExceptionAction::GeneralProtection);

        ghcb.save.sw_exit_info_1 = 2;
        let plan = svsm_ghcb_verify_plan(&ghcb);
        assert_eq!(plan.result, EsResult::VmmError);
        assert_eq!(plan.forward_exception, None);
        assert_eq!(plan.errno, Some(EINVAL));
    }

    #[test]
    fn svsm_call_protocol_retries_busy_then_succeeds() {
        let _guard = svsm_test_guard();
        reset_svsm_backend(SVSM_ERR_BUSY, SVSM_SUCCESS);
        let mut call = SvsmCall {
            rax: SVSM_CORE_PVALIDATE as u64,
            ..Default::default()
        };
        assert_eq!(svsm_perform_call_protocol(&mut call), Ok(()));
        let (len, _) = svsm_backend_log();
        assert_eq!(len, 2);
    }

    #[test]
    fn svsm_build_ca_from_psc_desc_packs_linux_entries() {
        let _guard = svsm_test_guard();
        use super::super::core::{PscDesc, PscEntry, RMP_PG_SIZE_2M, VMGEXIT_PSC_OP_PRIVATE};

        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x400,
            pagesize: RMP_PG_SIZE_2M,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();

        let mut call = SvsmPvalidateCall::empty();
        let next = svsm_build_ca_from_psc_desc(&desc, 0, &mut call);
        assert_eq!(next, 1);
        assert_eq!(call.num_entries, 1);
        assert_eq!(
            call.entries[0],
            SvsmPvalidateEntry {
                page_size: RMP_PG_SIZE_2M,
                action: true,
                ignore_cf: false,
                pfn: 0x400,
            }
        );
    }

    #[test]
    fn svsm_pval_terminate_plan_uses_current_entry_and_linux_reason() {
        let _guard = svsm_test_guard();
        use super::super::core::{
            GHCB_TERM_PVALIDATE, PscDesc, PscEntry, RMP_PG_SIZE_2M, SEV_TERM_SET_LINUX,
            VMGEXIT_PSC_OP_PRIVATE,
        };

        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x440,
            pagesize: RMP_PG_SIZE_2M,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();
        let mut call = SvsmPvalidateCall::empty();
        svsm_build_ca_from_psc_desc(&desc, 0, &mut call);

        let plan = svsm_pval_terminate_plan(&call, EINVAL, SVSM_ERR_INVALID_REQUEST).unwrap();
        assert_eq!(plan.pfn, 0x440);
        assert!(plan.action);
        assert_eq!(plan.page_size, RMP_PG_SIZE_2M);
        assert_eq!(plan.ret, EINVAL);
        assert_eq!(plan.svsm_ret, SVSM_ERR_INVALID_REQUEST);
        assert_eq!(plan.termination.set, SEV_TERM_SET_LINUX);
        assert_eq!(plan.termination.reason, GHCB_TERM_PVALIDATE);
    }

    #[test]
    fn svsm_pval_pages_issues_one_call_per_packed_chunk() {
        let _guard = svsm_test_guard();
        use super::super::core::{PscDesc, PscEntry, RMP_PG_SIZE_4K, VMGEXIT_PSC_OP_SHARED};

        set_svsm_pvalidate_buffer_pa(0);
        let mut desc = PscDesc::empty();
        let mut i = 0;
        while i < desc.entries.len() {
            desc.push(PscEntry {
                gfn: 0x100 + i as u64,
                pagesize: RMP_PG_SIZE_4K,
                operation: VMGEXIT_PSC_OP_SHARED,
                current_page: true,
            })
            .unwrap();
            i += 1;
        }

        reset_svsm_pval_log();
        set_test_svsm_pval_success(true);
        assert_eq!(svsm_pval_pages(&desc), Ok(()));
        set_test_svsm_pval_success(false);

        let (len, log) = svsm_pval_log();
        assert_eq!(len, 1);
        assert_eq!(log[0], (desc.entries.len(), 0x100, RMP_PG_SIZE_4K, false));
    }

    #[test]
    fn svsm_pval_pages_passes_linux_pvalidate_buffer_pa_in_rcx() {
        let _guard = svsm_test_guard();
        use super::super::core::{PscDesc, PscEntry, RMP_PG_SIZE_4K, VMGEXIT_PSC_OP_PRIVATE};

        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x100,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();

        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        set_svsm_pvalidate_buffer_pa(0xfeed_c000);
        assert_eq!(svsm_pval_pages(&desc), Ok(()));
        set_svsm_pvalidate_buffer_pa(0);

        let (len, log) = svsm_backend_log();
        assert_eq!(len, 1);
        assert_eq!(log[0], (SVSM_CORE_PVALIDATE as u64, 0xfeed_c000, 0, 0, 0));
    }

    #[test]
    fn svsm_pval_pages_retries_two_megabyte_mismatch_as_four_kib_ranges() {
        let _guard = svsm_test_guard();
        use super::super::core::{
            PscDesc, PscEntry, RMP_PG_SIZE_2M, RMP_PG_SIZE_4K, VMGEXIT_PSC_OP_PRIVATE,
        };

        set_svsm_pvalidate_buffer_pa(0);
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x800,
            pagesize: RMP_PG_SIZE_2M,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();

        reset_svsm_pval_log();
        set_test_svsm_pval_success(true);
        set_test_svsm_pval_2m_size_mismatch(true);
        assert_eq!(svsm_pval_pages(&desc), Ok(()));
        set_test_svsm_pval_2m_size_mismatch(false);
        set_test_svsm_pval_success(false);

        let (len, log) = svsm_pval_log();
        assert_eq!(len, 3);
        assert_eq!(log[0], (1, 0x800, RMP_PG_SIZE_2M, true));
        assert_eq!(
            log[1],
            (SVSM_PVALIDATE_MAX_COUNT, 0x800, RMP_PG_SIZE_4K, true)
        );
        assert_eq!(
            log[2],
            (
                512 - SVSM_PVALIDATE_MAX_COUNT,
                0x800 + SVSM_PVALIDATE_MAX_COUNT as u64,
                RMP_PG_SIZE_4K,
                true
            )
        );
    }

    #[test]
    fn svsm_pval_pages_uses_linux_wrapping_pfn_end_for_four_kib_retry() {
        let _guard = svsm_test_guard();
        use super::super::core::{PscDesc, PscEntry, RMP_PG_SIZE_2M, VMGEXIT_PSC_OP_PRIVATE};

        set_svsm_pvalidate_buffer_pa(0);
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: u64::MAX - 1,
            pagesize: RMP_PG_SIZE_2M,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();

        reset_svsm_pval_log();
        set_test_svsm_pval_success(true);
        set_test_svsm_pval_2m_size_mismatch(true);
        assert_eq!(svsm_pval_pages(&desc), Ok(()));
        set_test_svsm_pval_2m_size_mismatch(false);
        set_test_svsm_pval_success(false);

        let (len, log) = svsm_pval_log();
        assert_eq!(len, 1);
        assert_eq!(log[0], (1, u64::MAX - 1, RMP_PG_SIZE_2M, true));
    }

    #[test]
    fn svsm_call_domains_match_linux_macros() {
        let _guard = svsm_test_guard();
        assert_eq!(svsm_core_call(SVSM_CORE_PVALIDATE as u64), 1);
        assert_eq!(svsm_attest_call(SVSM_ATTEST_SERVICES), 1 << 32);
        assert_eq!(svsm_attest_call(SVSM_ATTEST_SINGLE_SERVICE), (1 << 32) | 1);
        assert_eq!(svsm_vtpm_call(SVSM_VTPM_QUERY), 2 << 32);
        assert_eq!(svsm_vtpm_call(SVSM_VTPM_CMD), (2 << 32) | 1);
    }

    #[derive(Clone, Copy)]
    struct FakeOps {
        buffer_pa: u64,
        result: Result<(), i32>,
        rcx_out: u64,
        rdx_out: u64,
        r8_out: u64,
        last_call: SvsmCall,
        copied_input: Option<SvsmAttestCall>,
    }

    impl FakeOps {
        const fn new(buffer_pa: u64) -> Self {
            Self {
                buffer_pa,
                result: Ok(()),
                rcx_out: 0,
                rdx_out: 0,
                r8_out: 0,
                last_call: SvsmCall {
                    rax: 0,
                    rcx: 0,
                    rdx: 0,
                    r8: 0,
                    r9: 0,
                    ret: 0,
                    rax_out: 0,
                    rcx_out: 0,
                    rdx_out: 0,
                    r8_out: 0,
                    r9_out: 0,
                },
                copied_input: None,
            }
        }
    }

    impl SvsmCallOps for FakeOps {
        fn svsm_buffer_pa(&self) -> u64 {
            self.buffer_pa
        }

        fn copy_attest_input(&mut self, input: &SvsmAttestCall) {
            self.copied_input = Some(*input);
        }

        fn perform_call_protocol(&mut self, call: &mut SvsmCall) -> Result<(), i32> {
            self.last_call = *call;
            call.rcx_out = self.rcx_out;
            call.rdx_out = self.rdx_out;
            call.r8_out = self.r8_out;
            self.result
        }
    }

    #[test]
    fn attest_request_sets_linux_registers_and_propagates_returned_lengths() {
        let _guard = svsm_test_guard();
        let mut ops = FakeOps::new(0x7000);
        ops.rcx_out = 64;
        ops.rdx_out = 128;
        ops.r8_out = 256;
        let mut call = SvsmCall::default();
        let mut input = SvsmAttestCall {
            manifest_buf: SvsmLocEntry { pa: 0x1000, len: 1 },
            certificates_buf: SvsmLocEntry { pa: 0x2000, len: 2 },
            report_buf: SvsmLocEntry { pa: 0x3000, len: 3 },
            ..Default::default()
        };

        assert_eq!(
            snp_issue_svsm_attest_req_with(
                &mut ops,
                1,
                svsm_attest_call(SVSM_ATTEST_SERVICES),
                &mut call,
                &mut input,
            ),
            Ok(())
        );
        assert_eq!(ops.last_call.rax, svsm_attest_call(SVSM_ATTEST_SERVICES));
        assert_eq!(ops.last_call.rcx, 0x7000);
        assert_eq!(ops.last_call.rdx, u64::MAX);
        assert_eq!(ops.last_call.r8, u64::MAX);
        assert_eq!(
            ops.copied_input.unwrap(),
            SvsmAttestCall {
                manifest_buf: SvsmLocEntry { pa: 0x1000, len: 1 },
                certificates_buf: SvsmLocEntry { pa: 0x2000, len: 2 },
                report_buf: SvsmLocEntry { pa: 0x3000, len: 3 },
                ..Default::default()
            }
        );
        assert_eq!(input.manifest_buf.len, 64);
        assert_eq!(input.certificates_buf.len, 128);
        assert_eq!(input.report_buf.len, 256);
    }

    #[test]
    fn attest_request_requires_nonzero_vmpl() {
        let _guard = svsm_test_guard();
        let mut ops = FakeOps::new(0x7000);
        let mut call = SvsmCall::default();
        let mut input = SvsmAttestCall::default();
        assert_eq!(
            snp_issue_svsm_attest_req_with(&mut ops, 0, 1, &mut call, &mut input),
            Err(EINVAL)
        );
        assert_eq!(ops.copied_input, None);
    }

    #[test]
    fn public_svsm_wrappers_follow_linux_protocol_until_backend_boundary() {
        let _guard = svsm_test_guard();
        let mut call = SvsmCall::default();
        let mut input = SvsmAttestCall::default();
        let mut buffer = [0u8; 16];

        publish_snp_vmpl(0);
        assert_eq!(
            snp_issue_svsm_attest_req(0, &mut call, &mut input),
            Err(EINVAL)
        );

        publish_snp_vmpl(1);
        set_svsm_pvalidate_buffer_pa(0x7000);
        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        assert_eq!(
            snp_issue_svsm_attest_req(
                svsm_attest_call(SVSM_ATTEST_SERVICES),
                &mut call,
                &mut input,
            ),
            Ok(())
        );
        let (len, log) = svsm_backend_log();
        assert_eq!(len, 1);
        assert_eq!(
            log[0],
            (
                svsm_attest_call(SVSM_ATTEST_SERVICES),
                0x7000,
                u64::MAX,
                u64::MAX,
                0
            )
        );

        reset_svsm_backend(SVSM_SUCCESS, SVSM_SUCCESS);
        assert_eq!(snp_svsm_vtpm_send_command(buffer.as_mut_ptr()), Ok(()));
        let (len, log) = svsm_backend_log();
        assert_eq!(len, 1);
        assert_eq!(log[0].0, svsm_vtpm_call(SVSM_VTPM_CMD));
        assert_ne!(log[0].1, 0);
    }

    #[test]
    fn vtpm_send_command_and_probe_use_linux_call_ids() {
        let _guard = svsm_test_guard();
        let mut ops = FakeOps::new(0x7000);
        assert_eq!(snp_svsm_vtpm_send_command_with(&mut ops, 0x9000), Ok(()));
        assert_eq!(ops.last_call.rax, svsm_vtpm_call(SVSM_VTPM_CMD));
        assert_eq!(ops.last_call.rcx, 0x9000);
        assert_eq!(snp_svsm_vtpm_send_command_with(&mut ops, 0), Ok(()));
        assert_eq!(ops.last_call.rcx, 0);

        ops.rcx_out = 1 << SVSM_VTPM_SEND_COMMAND_BIT;
        assert!(snp_svsm_vtpm_probe_with(&mut ops, 1));
        assert_eq!(ops.last_call.rax, svsm_vtpm_call(SVSM_VTPM_QUERY));

        ops.rcx_out = 0;
        assert!(!snp_svsm_vtpm_probe_with(&mut ops, 1));
        assert!(!snp_svsm_vtpm_probe_with(&mut ops, 0));
    }

    #[test]
    fn vtpm_probe_requires_svsm_and_send_command_bit() {
        let _guard = svsm_test_guard();
        assert!(snp_svsm_vtpm_probe(true, 1 << SVSM_VTPM_SEND_COMMAND_BIT));
        assert!(!snp_svsm_vtpm_probe(true, 1));
        assert!(!snp_svsm_vtpm_probe(false, 1 << SVSM_VTPM_SEND_COMMAND_BIT));
    }
}
