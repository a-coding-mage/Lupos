//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev/vc-shared.c
//! test-origin: linux:vendor/linux/arch/x86/coco/sev/vc-shared.c
//! SEV-ES shared #VC decode and GHCB protocol helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/vc-shared.c

use core::sync::atomic::{AtomicU16, Ordering};

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

use super::core::{GHCB_TERM_CPUID_HV, GHCB_TERM_REGISTER, SEV_TERM_SET_LINUX, SevTermination};

pub const SVM_EXIT_EXCP_BASE: u64 = 0x40;
pub const SVM_EXIT_LAST_EXCP: u64 = 0x5f;
pub const SVM_EXIT_RDTSC: u64 = 0x6e;
pub const SVM_EXIT_RDPMC: u64 = 0x6f;
pub const SVM_EXIT_CPUID: u64 = 0x72;
pub const SVM_EXIT_INVD: u64 = 0x76;
pub const SVM_EXIT_HLT: u64 = 0x78;
pub const SVM_EXIT_IOIO: u64 = 0x7b;
pub const SVM_EXIT_MSR: u64 = 0x7c;
pub const SVM_EXIT_VMMCALL: u64 = 0x81;
pub const SVM_EXIT_RDTSCP: u64 = 0x87;
pub const SVM_EXIT_WBINVD: u64 = 0x89;
pub const SVM_EXIT_MONITOR: u64 = 0x8a;
pub const SVM_EXIT_MWAIT: u64 = 0x8b;
pub const SVM_EXIT_NPF: u64 = 0x400;

pub const SVM_EXIT_READ_DR7: u64 = 0x27;
pub const SVM_EXIT_WRITE_DR7: u64 = 0x37;

pub const SVM_VMGEXIT_MMIO_READ: u64 = 0x8000_0001;
pub const SVM_VMGEXIT_MMIO_WRITE: u64 = 0x8000_0002;
pub const SVM_VMGEXIT_NMI_COMPLETE: u64 = 0x8000_0003;
pub const SVM_VMGEXIT_PSC: u64 = 0x8000_0010;
pub const SVM_VMGEXIT_SNP_RUN_VMPL: u64 = 0x8000_0018;
pub const SVM_VMGEXIT_HV_FEATURES: u64 = 0x8000_fffd;
pub const SVM_VMGEXIT_UNSUPPORTED_EVENT: u64 = 0x8000_ffff;

pub const GHCB_MSR_INFO_MASK: u64 = (1 << GHCB_DATA_LOW) - 1;
pub const GHCB_DATA_LOW: u64 = 12;
pub const GHCB_MSR_SEV_INFO_RESP: u64 = 0x001;
pub const GHCB_MSR_SEV_INFO_REQ: u64 = 0x002;
pub const GHCB_MSR_CPUID_REQ: u64 = 0x004;
pub const GHCB_MSR_CPUID_RESP: u64 = 0x005;
pub const GHCB_MSR_REG_GPA_REQ: u64 = 0x012;
pub const GHCB_MSR_REG_GPA_RESP: u64 = 0x013;
pub const GHCB_MSR_PSC_REQ: u64 = 0x014;
pub const GHCB_MSR_PSC_RESP: u64 = 0x015;
pub const GHCB_MSR_HV_FT_REQ: u64 = 0x080;
pub const GHCB_MSR_HV_FT_RESP: u64 = 0x081;

pub const GHCB_PROTOCOL_MIN: u16 = 1;
pub const GHCB_PROTOCOL_MAX: u16 = 2;
pub const GHCB_DEFAULT_USAGE: u32 = 0;
pub const GHCB_SHARED_BUFFER_SIZE: usize = 2032;
pub const SEV_ES_RDRAND_ERROR: &str =
    "RDRAND instruction not supported - no trusted source of randomness available";

static GHCB_VERSION: AtomicU16 = AtomicU16::new(0);

#[cfg(test)]
static GHCB_VERSION_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

#[cfg(test)]
pub(super) struct GhcbVersionTestGuard {
    _guard: spin::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for GhcbVersionTestGuard {
    fn drop(&mut self) {
        set_ghcb_protocol_version(0);
    }
}

#[cfg(test)]
pub(super) fn ghcb_version_test_guard(version: u16) -> GhcbVersionTestGuard {
    let guard = GHCB_VERSION_TEST_LOCK.lock();
    set_ghcb_protocol_version(version);
    GhcbVersionTestGuard { _guard: guard }
}

pub const X86_TRAP_UD: u64 = 6;
pub const X86_TRAP_GP: u64 = 13;
pub const X86_TRAP_PF: u64 = 14;
pub const X86_TRAP_AC: u64 = 17;
pub const X86_PF_PROT: u64 = 1 << 0;
pub const X86_PF_WRITE: u64 = 1 << 1;
pub const X86_PF_USER: u64 = 1 << 2;
pub const X86_PF_INSTR: u64 = 1 << 4;
pub const X86_EFLAGS_DF: u64 = 1 << 10;
const TASK_SIZE_MAX: u64 = 0x0000_8000_0000_0000;
const PAGE_SIZE: u64 = 4096;
const PAGE_MASK: u64 = !(PAGE_SIZE - 1);
const VSYSCALL_ADDR: u64 = 0xffff_ffff_ff60_0000;

pub const SVM_EVTINJ_VEC_MASK: u64 = 0xff;
pub const SVM_EVTINJ_TYPE_SHIFT: u64 = 8;
pub const SVM_EVTINJ_TYPE_MASK: u64 = 7 << SVM_EVTINJ_TYPE_SHIFT;
pub const SVM_EVTINJ_TYPE_EXEPT: u64 = 3 << SVM_EVTINJ_TYPE_SHIFT;
pub const SVM_EVTINJ_VALID_ERR: u64 = 1 << 11;
pub const SVM_EVTINJ_VALID: u64 = 1 << 31;

pub const IOIO_TYPE_STR: u64 = 1 << 2;
pub const IOIO_TYPE_IN: u64 = 1;
pub const IOIO_TYPE_INS: u64 = IOIO_TYPE_IN | IOIO_TYPE_STR;
pub const IOIO_TYPE_OUT: u64 = 0;
pub const IOIO_TYPE_OUTS: u64 = IOIO_TYPE_OUT | IOIO_TYPE_STR;
pub const IOIO_REP: u64 = 1 << 3;
pub const IOIO_ADDR_64: u64 = 1 << 9;
pub const IOIO_ADDR_32: u64 = 1 << 8;
pub const IOIO_ADDR_16: u64 = 1 << 7;
pub const IOIO_DATA_32: u64 = 1 << 6;
pub const IOIO_DATA_16: u64 = 1 << 5;
pub const IOIO_DATA_8: u64 = 1 << 4;
pub const IOIO_SEG_ES: u64 = 0 << 10;
pub const IOIO_SEG_DS: u64 = 3 << 10;

const GHCB_VALID_RAX: u64 = 1 << 0;
const GHCB_VALID_RBX: u64 = 1 << 1;
const GHCB_VALID_RCX: u64 = 1 << 2;
const GHCB_VALID_RDX: u64 = 1 << 3;
const GHCB_VALID_CPL: u64 = 1 << 4;
const GHCB_VALID_XCR0: u64 = 1 << 5;
const GHCB_VALID_XSS: u64 = 1 << 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevEsCpuFeaturePlan {
    pub has_rdrand: bool,
    pub ok: bool,
    pub error: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SevEsProtocolFailure {
    BadResponseCode,
    UnsupportedRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevEsProtocolNegotiationPlan {
    pub request_msr: u64,
    pub wrote_request: bool,
    pub vmgexit: bool,
    pub read_response: bool,
    pub response_msr: u64,
    pub negotiated_version: Option<u16>,
    pub failure: Option<SevEsProtocolFailure>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpCpuidHvGhcbPlan {
    pub result: EsResult,
    pub termination: Option<SevTermination>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpRegisterGhcbEarlyPlan {
    pub paddr: u64,
    pub pfn: u64,
    pub request_msr: u64,
    pub response_msr: u64,
    pub ok: bool,
    pub termination: Option<SevTermination>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcSnpCpuidResult {
    Handled,
    Unsupported,
    VmmError,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VcRegs {
    pub ax: u64,
    pub bx: u64,
    pub cx: u64,
    pub dx: u64,
    pub si: u64,
    pub di: u64,
    pub sp: u64,
    pub bp: u64,
    pub flags: u64,
    pub ip: u64,
    pub user_mode: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmulatedInsn {
    pub opcode: u32,
    pub opcode_bytes: [u8; 2],
    pub opcode_nbytes: u8,
    pub modrm: u8,
    pub immediate: u64,
    pub opnd_bytes: u8,
    pub addr_bytes: u8,
    pub length: u8,
    pub rep_prefix: bool,
    pub addr_ref: Option<u64>,
    pub ds_base: Option<u64>,
    pub es_base: Option<u64>,
}

impl EmulatedInsn {
    pub const fn from_opcode(opcode: u32, first: u8) -> Self {
        Self {
            opcode,
            opcode_bytes: [first, (opcode >> 8) as u8],
            opcode_nbytes: if opcode > 0xff { 2 } else { 1 },
            modrm: 0,
            immediate: 0,
            opnd_bytes: 4,
            addr_bytes: 8,
            length: 1,
            rep_prefix: false,
            addr_ref: None,
            ds_base: None,
            es_base: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FaultInfo {
    pub vector: u64,
    pub error_code: u64,
    pub cr2: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EsEmCtxt {
    pub regs: VcRegs,
    pub exit_code: u64,
    pub exit_info_1: u64,
    pub exit_info_2: u64,
    pub insn: EmulatedInsn,
    pub fi: FaultInfo,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcInitEmCtxtPlan {
    pub ctxt: EsEmCtxt,
    pub decode_needed: bool,
    pub result: EsResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EsResult {
    Continue,
    Retry,
    Exception,
    VmmError,
    DecodeFailed,
    Terminate,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GhcbSaveArea {
    pub sw_exit_code: u64,
    pub sw_exit_info_1: u64,
    pub sw_exit_info_2: u64,
    pub sw_scratch: u64,
    pub cpl: u64,
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub xcr0: u64,
    pub xss: u64,
    valid_bitmap: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ghcb {
    pub protocol_version: u16,
    pub ghcb_usage: u32,
    pub save: GhcbSaveArea,
    pub shared_buffer: [u8; GHCB_SHARED_BUFFER_SIZE],
}

impl Default for Ghcb {
    fn default() -> Self {
        Self {
            protocol_version: 0,
            ghcb_usage: 0,
            save: GhcbSaveArea::default(),
            shared_buffer: [0; GHCB_SHARED_BUFFER_SIZE],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuidLeaf {
    pub fn_: u32,
    pub subfn: u32,
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

pub const fn vc_decoding_needed(exit_code: u64) -> bool {
    !(exit_code >= SVM_EXIT_EXCP_BASE && exit_code <= SVM_EXIT_LAST_EXCP)
}

pub fn vc_init_em_ctxt(exit_code: u64, exit_info_1: u64, exit_info_2: u64) -> EsEmCtxt {
    EsEmCtxt {
        exit_code,
        exit_info_1,
        exit_info_2,
        ..Default::default()
    }
}

pub fn vc_init_em_ctxt_plan(
    regs: VcRegs,
    exit_code: u64,
    decode_result: EsResult,
) -> VcInitEmCtxtPlan {
    let decode_needed = vc_decoding_needed(exit_code);
    VcInitEmCtxtPlan {
        ctxt: EsEmCtxt {
            regs,
            exit_code,
            ..Default::default()
        },
        decode_needed,
        result: if decode_needed {
            decode_result
        } else {
            EsResult::Continue
        },
    }
}

pub fn vc_init_em_ctxt_with_regs(regs: VcRegs, exit_code: u64, insn: EmulatedInsn) -> EsEmCtxt {
    EsEmCtxt {
        regs,
        exit_code,
        insn,
        ..Default::default()
    }
}

pub fn vc_finish_insn(ctxt: &mut EsEmCtxt) {
    ctxt.regs.ip = ctxt.regs.ip.wrapping_add(ctxt.insn.length as u64);
}

pub const fn vc_check_opcode_bytes(ctxt: &EsEmCtxt, exit_code: u64) -> EsResult {
    let opcode = ctxt.insn.opcode;
    let modrm = ctxt.insn.modrm;

    match exit_code {
        SVM_EXIT_IOIO | SVM_EXIT_NPF => EsResult::Continue,
        SVM_EXIT_CPUID if opcode == 0xa20f => EsResult::Continue,
        SVM_EXIT_INVD if opcode == 0x080f => EsResult::Continue,
        SVM_EXIT_MONITOR if opcode == 0x010f && (modrm == 0xc8 || modrm == 0xfa) => {
            EsResult::Continue
        }
        SVM_EXIT_MWAIT if opcode == 0x010f && (modrm == 0xc9 || modrm == 0xfb) => {
            EsResult::Continue
        }
        SVM_EXIT_MSR if opcode == 0x320f || opcode == 0x300f => EsResult::Continue,
        SVM_EXIT_RDPMC if opcode == 0x330f => EsResult::Continue,
        SVM_EXIT_RDTSC if opcode == 0x310f => EsResult::Continue,
        SVM_EXIT_RDTSCP if opcode == 0x010f && modrm == 0xf9 => EsResult::Continue,
        SVM_EXIT_READ_DR7 if opcode == 0x210f && x86_modrm_reg(modrm) == 7 => EsResult::Continue,
        SVM_EXIT_WRITE_DR7 if opcode == 0x230f && x86_modrm_reg(modrm) == 7 => EsResult::Continue,
        SVM_EXIT_VMMCALL if opcode == 0x010f && modrm == 0xd9 => EsResult::Continue,
        SVM_EXIT_WBINVD if opcode == 0x090f => EsResult::Continue,
        _ => EsResult::Unsupported,
    }
}

pub const fn vc_ioio_exitinfo(ctxt: &EsEmCtxt) -> Result<u64, EsResult> {
    let opcode = ctxt.insn.opcode_bytes[0];
    let mut exitinfo = 0u64;
    let port;

    match opcode {
        0x6c | 0x6d => {
            exitinfo |= IOIO_TYPE_INS | IOIO_SEG_ES;
            port = ctxt.regs.dx & 0xffff;
        }
        0x6e | 0x6f => {
            exitinfo |= IOIO_TYPE_OUTS | IOIO_SEG_DS;
            port = ctxt.regs.dx & 0xffff;
        }
        0xe4 | 0xe5 => {
            exitinfo |= IOIO_TYPE_IN;
            port = ctxt.insn.immediate & 0xff;
        }
        0xe6 | 0xe7 => {
            exitinfo |= IOIO_TYPE_OUT;
            port = ctxt.insn.immediate & 0xff;
        }
        0xec | 0xed => {
            exitinfo |= IOIO_TYPE_IN;
            port = ctxt.regs.dx & 0xffff;
        }
        0xee | 0xef => {
            exitinfo |= IOIO_TYPE_OUT;
            port = ctxt.regs.dx & 0xffff;
        }
        _ => return Err(EsResult::DecodeFailed),
    }

    let size = match opcode {
        0x6c | 0x6e | 0xe4 | 0xe6 | 0xec | 0xee => 1,
        _ if ctxt.insn.opnd_bytes == 2 => 2,
        _ => 4,
    };

    match size {
        1 => exitinfo |= IOIO_DATA_8,
        2 => exitinfo |= IOIO_DATA_16,
        4 => exitinfo |= IOIO_DATA_32,
        _ => return Err(EsResult::DecodeFailed),
    }

    match ctxt.insn.addr_bytes {
        2 => exitinfo |= IOIO_ADDR_16,
        4 => exitinfo |= IOIO_ADDR_32,
        8 => exitinfo |= IOIO_ADDR_64,
        _ => {}
    }

    exitinfo |= port << 16;

    if ctxt.insn.rep_prefix {
        exitinfo |= IOIO_REP;
    }

    Ok(exitinfo)
}

pub const fn vc_ioio_exitinfo_compat(size: usize, port: u16, is_write: bool) -> Result<u64, i32> {
    if !(size == 1 || size == 2 || size == 4) {
        return Err(EINVAL);
    }
    let data = match size {
        1 => IOIO_DATA_8,
        2 => IOIO_DATA_16,
        4 => IOIO_DATA_32,
        _ => 0,
    };
    let dir = if is_write {
        IOIO_TYPE_OUT
    } else {
        IOIO_TYPE_IN
    };
    Ok(data | dir | ((port as u64) << 16))
}

pub const fn ioio_data_size(exitinfo: u64) -> usize {
    if exitinfo & IOIO_DATA_8 != 0 {
        1
    } else if exitinfo & IOIO_DATA_16 != 0 {
        2
    } else {
        4
    }
}

pub const fn ioio_string_plan(exitinfo: u64, cx: u64) -> Option<(usize, u64, usize)> {
    if exitinfo & IOIO_TYPE_STR == 0 {
        return None;
    }
    let io_bytes = ioio_data_size(exitinfo);
    let ghcb_count = GHCB_SHARED_BUFFER_SIZE / io_bytes;
    let op_count = if exitinfo & IOIO_REP != 0 { cx } else { 1 };
    let exit_count = if op_count < ghcb_count as u64 {
        op_count
    } else {
        ghcb_count as u64
    };
    Some((io_bytes, exit_count, io_bytes * exit_count as usize))
}

pub const fn fault_in_kernel_space(address: u64) -> bool {
    if address & PAGE_MASK == VSYSCALL_ADDR {
        return false;
    }

    address >= TASK_SIZE_MAX
}

fn vc_insn_string_check(ctxt: &mut EsEmCtxt, address: u64, write: bool) -> EsResult {
    if ctxt.regs.user_mode && fault_in_kernel_space(address) {
        ctxt.fi.vector = X86_TRAP_PF;
        ctxt.fi.error_code = X86_PF_USER;
        ctxt.fi.cr2 = address;
        if write {
            ctxt.fi.error_code |= X86_PF_WRITE;
        }

        return EsResult::Exception;
    }

    EsResult::Continue
}

pub trait VcIoioOps {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult;

    fn es_segment_base(&mut self, _ctxt: &EsEmCtxt) -> Option<u64> {
        Some(0)
    }

    fn ghcb_shared_buffer_pa(&mut self, ghcb: &Ghcb) -> u64 {
        ghcb.shared_buffer.as_ptr() as u64
    }

    fn ioio_check(&mut self, _ctxt: &mut EsEmCtxt, _port: u16, _size: usize) -> EsResult {
        EsResult::Continue
    }

    fn string_read(
        &mut self,
        _ctxt: &mut EsEmCtxt,
        _addr: u64,
        _buffer: &mut [u8],
        _io_bytes: usize,
        _count: u64,
        _df: bool,
    ) -> EsResult {
        EsResult::Unsupported
    }

    fn string_write(
        &mut self,
        _ctxt: &mut EsEmCtxt,
        _addr: u64,
        _buffer: &[u8],
        _io_bytes: usize,
        _count: u64,
        _df: bool,
    ) -> EsResult {
        EsResult::Unsupported
    }
}

pub trait VcCpuidOps {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult;

    fn xcr0(&mut self) -> u64 {
        1
    }

    fn cr4_osxsave(&mut self) -> bool {
        false
    }

    fn shstk_supported(&mut self) -> bool {
        false
    }

    fn xss(&mut self) -> u64 {
        0
    }

    fn snp_cpuid(&mut self, _leaf: &mut CpuidLeaf, _ghcb: &mut Ghcb, _ctxt: &mut EsEmCtxt) -> bool {
        false
    }

    fn snp_cpuid_result(
        &mut self,
        leaf: &mut CpuidLeaf,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
    ) -> VcSnpCpuidResult {
        if self.snp_cpuid(leaf, ghcb, ctxt) {
            VcSnpCpuidResult::Handled
        } else {
            VcSnpCpuidResult::VmmError
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultVcIoioOps;

impl VcIoioOps for DefaultVcIoioOps {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult {
        sev_es_ghcb_hv_call(ghcb, ctxt, exit_code, exit_info_1, exit_info_2)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultVcCpuidOps;

impl VcCpuidOps for DefaultVcCpuidOps {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult {
        sev_es_ghcb_hv_call(ghcb, ctxt, exit_code, exit_info_1, exit_info_2)
    }
}

pub fn vc_ioio_exitinfo_checked_with<O: VcIoioOps>(
    ops: &mut O,
    ctxt: &mut EsEmCtxt,
) -> Result<u64, EsResult> {
    let exitinfo = vc_ioio_exitinfo(ctxt)?;
    let port = ((exitinfo >> 16) & 0xffff) as u16;
    let size = ioio_data_size(exitinfo);
    let ret = ops.ioio_check(ctxt, port, size);
    if ret != EsResult::Continue {
        return Err(ret);
    }

    Ok(exitinfo)
}

pub const fn lower_bits(value: u64, bits: u64) -> u64 {
    if bits >= 64 {
        value
    } else if bits == 0 {
        0
    } else {
        value & ((1u64 << bits) - 1)
    }
}

pub fn vc_handle_ioio_with<O: VcIoioOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    let exit_info_1 = match vc_ioio_exitinfo_checked_with(ops, ctxt) {
        Ok(info) => info,
        Err(ret) => return ret,
    };

    if exit_info_1 & IOIO_TYPE_STR != 0 {
        return vc_handle_ioio_string_with(ops, ghcb, ctxt, exit_info_1);
    }

    let bits = (exit_info_1 & 0x70) >> 1;
    let mut rax = 0;
    if exit_info_1 & IOIO_TYPE_IN == 0 {
        rax = lower_bits(ctxt.regs.ax, bits);
    }
    ghcb_set_rax(ghcb, rax);

    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_IOIO, exit_info_1, 0);
    if ret != EsResult::Continue {
        return ret;
    }

    if exit_info_1 & IOIO_TYPE_IN != 0 {
        if !ghcb_rax_is_valid(ghcb) {
            return EsResult::VmmError;
        }
        ctxt.regs.ax = lower_bits(ghcb.save.rax, bits);
    }

    ret
}

fn vc_handle_ioio_string_with<O: VcIoioOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    exit_info_1: u64,
) -> EsResult {
    let df = ctxt.regs.flags & X86_EFLAGS_DF == X86_EFLAGS_DF;
    let Some((io_bytes, exit_count, exit_bytes)) = ioio_string_plan(exit_info_1, ctxt.regs.cx)
    else {
        return EsResult::DecodeFailed;
    };
    let Some(es_base) = ops.es_segment_base(ctxt) else {
        ctxt.fi.vector = X86_TRAP_GP;
        ctxt.fi.error_code = 0;
        return EsResult::Exception;
    };

    if exit_info_1 & IOIO_TYPE_IN == 0 {
        let src = es_base.wrapping_add(ctxt.regs.si);
        let ret = vc_insn_string_check(ctxt, src, false);
        if ret != EsResult::Continue {
            return ret;
        }

        let ret = ops.string_read(
            ctxt,
            src,
            &mut ghcb.shared_buffer[..exit_bytes],
            io_bytes,
            exit_count,
            df,
        );
        if ret != EsResult::Continue {
            return ret;
        }
    }

    let sw_scratch = ops.ghcb_shared_buffer_pa(ghcb);
    ghcb_set_sw_scratch(ghcb, sw_scratch);
    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_IOIO, exit_info_1, exit_count);
    if ret != EsResult::Continue {
        return ret;
    }

    if exit_info_1 & IOIO_TYPE_IN != 0 {
        let dst = es_base.wrapping_add(ctxt.regs.di);
        let ret = vc_insn_string_check(ctxt, dst, true);
        if ret != EsResult::Continue {
            return ret;
        }

        let ret = ops.string_write(
            ctxt,
            dst,
            &ghcb.shared_buffer[..exit_bytes],
            io_bytes,
            exit_count,
            df,
        );
        if ret != EsResult::Continue {
            return ret;
        }
        if df {
            ctxt.regs.di = ctxt.regs.di.wrapping_sub(exit_bytes as u64);
        } else {
            ctxt.regs.di = ctxt.regs.di.wrapping_add(exit_bytes as u64);
        }
    } else if df {
        ctxt.regs.si = ctxt.regs.si.wrapping_sub(exit_bytes as u64);
    } else {
        ctxt.regs.si = ctxt.regs.si.wrapping_add(exit_bytes as u64);
    }

    if exit_info_1 & IOIO_REP != 0 {
        ctxt.regs.cx = ctxt.regs.cx.wrapping_sub(exit_count);
    }

    if ctxt.regs.cx != 0 {
        EsResult::Retry
    } else {
        EsResult::Continue
    }
}

pub fn vc_handle_ioio(ghcb: &mut Ghcb, ctxt: &mut EsEmCtxt) -> EsResult {
    let mut ops = DefaultVcIoioOps;
    vc_handle_ioio_with(&mut ops, ghcb, ctxt)
}

pub fn sev_cpuid_hv_ghcb_ops<O: VcCpuidOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    leaf: &mut CpuidLeaf,
) -> EsResult {
    ghcb_set_rax(ghcb, leaf.fn_ as u64);
    ghcb_set_rcx(ghcb, leaf.subfn as u64);
    let xcr0 = if ops.cr4_osxsave() { ops.xcr0() } else { 1 };
    ghcb_set_xcr0(ghcb, xcr0);

    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_CPUID, 0, 0);
    if ret != EsResult::Continue {
        return ret;
    }

    if !(ghcb_rax_is_valid(ghcb)
        && ghcb_rbx_is_valid(ghcb)
        && ghcb_rcx_is_valid(ghcb)
        && ghcb_rdx_is_valid(ghcb))
    {
        return EsResult::VmmError;
    }

    leaf.eax = ghcb.save.rax as u32;
    leaf.ebx = ghcb.save.rbx as u32;
    leaf.ecx = ghcb.save.rcx as u32;
    leaf.edx = ghcb.save.rdx as u32;
    EsResult::Continue
}

pub fn vc_handle_cpuid_snp_with<O: VcCpuidOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    match vc_handle_cpuid_snp_result_with(ops, ghcb, ctxt) {
        VcSnpCpuidResult::Handled => EsResult::Continue,
        VcSnpCpuidResult::Unsupported => EsResult::Unsupported,
        VcSnpCpuidResult::VmmError => EsResult::VmmError,
    }
}

pub fn vc_handle_cpuid_snp_result_with<O: VcCpuidOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> VcSnpCpuidResult {
    let mut leaf = CpuidLeaf {
        fn_: ctxt.regs.ax as u32,
        subfn: ctxt.regs.cx as u32,
        ..Default::default()
    };

    match ops.snp_cpuid_result(&mut leaf, ghcb, ctxt) {
        VcSnpCpuidResult::Handled => {}
        other => return other,
    }

    ctxt.regs.ax = leaf.eax as u64;
    ctxt.regs.bx = leaf.ebx as u64;
    ctxt.regs.cx = leaf.ecx as u64;
    ctxt.regs.dx = leaf.edx as u64;
    VcSnpCpuidResult::Handled
}

pub fn vc_handle_cpuid_with<O: VcCpuidOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    snp_enabled: bool,
) -> EsResult {
    if snp_enabled {
        match vc_handle_cpuid_snp_result_with(ops, ghcb, ctxt) {
            VcSnpCpuidResult::Handled => return EsResult::Continue,
            VcSnpCpuidResult::VmmError => return EsResult::VmmError,
            VcSnpCpuidResult::Unsupported => {}
        }
    }

    let mut leaf = CpuidLeaf {
        fn_: ctxt.regs.ax as u32,
        subfn: ctxt.regs.cx as u32,
        ..Default::default()
    };

    if ops.shstk_supported() && ctxt.regs.ax == 0xd && ctxt.regs.cx == 1 {
        let xss = ops.xss();
        ghcb_set_xss(ghcb, xss);
    }

    let ret = sev_cpuid_hv_ghcb_ops(ops, ghcb, ctxt, &mut leaf);
    if ret != EsResult::Continue {
        return ret;
    }

    ctxt.regs.ax = leaf.eax as u64;
    ctxt.regs.bx = leaf.ebx as u64;
    ctxt.regs.cx = leaf.ecx as u64;
    ctxt.regs.dx = leaf.edx as u64;
    EsResult::Continue
}

pub fn vc_handle_cpuid(ghcb: &mut Ghcb, ctxt: &mut EsEmCtxt, snp_enabled: bool) -> EsResult {
    let mut ops = DefaultVcCpuidOps;
    vc_handle_cpuid_with(&mut ops, ghcb, ctxt, snp_enabled)
}

pub fn verify_exception_info(ghcb: &Ghcb, ctxt: &mut EsEmCtxt) -> EsResult {
    let ret = ghcb.save.sw_exit_info_1 & 0xffff_ffff;
    if ret == 0 {
        return EsResult::Continue;
    }

    if ret == 1 {
        let info = ghcb.save.sw_exit_info_2;
        let vector = info & SVM_EVTINJ_VEC_MASK;

        if info & SVM_EVTINJ_VALID != 0
            && (vector == X86_TRAP_GP || vector == X86_TRAP_UD)
            && info & SVM_EVTINJ_TYPE_MASK == SVM_EVTINJ_TYPE_EXEPT
        {
            ctxt.fi.vector = vector;
            if info & SVM_EVTINJ_VALID_ERR != 0 {
                ctxt.fi.error_code = info >> 32;
            }
            return EsResult::Exception;
        }
    }

    EsResult::VmmError
}

pub fn sev_es_ghcb_hv_call(
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    exit_code: u64,
    exit_info_1: u64,
    exit_info_2: u64,
) -> EsResult {
    sev_es_ghcb_hv_call_with(ghcb, ctxt, exit_code, exit_info_1, exit_info_2, |_| {})
}

pub fn sev_es_ghcb_hv_call_with(
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    exit_code: u64,
    exit_info_1: u64,
    exit_info_2: u64,
    hypervisor: impl FnOnce(&mut Ghcb),
) -> EsResult {
    sev_es_ghcb_hv_call_with_protocol(
        ghcb,
        ctxt,
        exit_code,
        exit_info_1,
        exit_info_2,
        ghcb_protocol_version(),
        hypervisor,
    )
}

pub fn sev_es_ghcb_hv_call_with_protocol(
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    exit_code: u64,
    exit_info_1: u64,
    exit_info_2: u64,
    protocol_version: u16,
    hypervisor: impl FnOnce(&mut Ghcb),
) -> EsResult {
    ghcb.protocol_version = protocol_version;
    ghcb.ghcb_usage = GHCB_DEFAULT_USAGE;
    ghcb_set_sw_exit_code(ghcb, exit_code);
    ghcb_set_sw_exit_info_1(ghcb, exit_info_1);
    ghcb_set_sw_exit_info_2(ghcb, exit_info_2);

    hypervisor(ghcb);
    verify_exception_info(ghcb, ctxt)
}

pub fn sev_cpuid_hv_ghcb(
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    leaf: &mut CpuidLeaf,
    cr4_osxsave: bool,
    xcr0: u64,
) -> EsResult {
    sev_cpuid_hv_ghcb_with(ghcb, ctxt, leaf, cr4_osxsave, xcr0, |_| {})
}

pub fn sev_cpuid_hv_ghcb_with(
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    leaf: &mut CpuidLeaf,
    cr4_osxsave: bool,
    xcr0: u64,
    hypervisor: impl FnOnce(&mut Ghcb),
) -> EsResult {
    ghcb_set_rax(ghcb, leaf.fn_ as u64);
    ghcb_set_rcx(ghcb, leaf.subfn as u64);
    ghcb_set_xcr0(ghcb, if cr4_osxsave { xcr0 } else { 1 });

    let ret = sev_es_ghcb_hv_call_with(ghcb, ctxt, SVM_EXIT_CPUID, 0, 0, hypervisor);
    if ret != EsResult::Continue {
        return ret;
    }

    if !(ghcb_rax_is_valid(ghcb)
        && ghcb_rbx_is_valid(ghcb)
        && ghcb_rcx_is_valid(ghcb)
        && ghcb_rdx_is_valid(ghcb))
    {
        return EsResult::VmmError;
    }

    leaf.eax = ghcb.save.rax as u32;
    leaf.ebx = ghcb.save.rbx as u32;
    leaf.ecx = ghcb.save.rcx as u32;
    leaf.edx = ghcb.save.rdx as u32;
    EsResult::Continue
}

pub fn snp_cpuid_hv_ghcb_plan(result: EsResult) -> SnpCpuidHvGhcbPlan {
    SnpCpuidHvGhcbPlan {
        result,
        termination: if result == EsResult::Continue {
            None
        } else {
            Some(SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_CPUID_HV,
            })
        },
    }
}

pub fn vc_handle_rdtsc(
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    exit_code: u64,
    secure_tsc: bool,
) -> EsResult {
    if secure_tsc {
        return EsResult::VmmError;
    }
    let rdtscp = exit_code == SVM_EXIT_RDTSCP;
    let ret = sev_es_ghcb_hv_call(ghcb, ctxt, exit_code, 0, 0);
    if ret != EsResult::Continue {
        return ret;
    }
    if !(ghcb_rax_is_valid(ghcb) && ghcb_rdx_is_valid(ghcb) && (!rdtscp || ghcb_rcx_is_valid(ghcb)))
    {
        return EsResult::VmmError;
    }
    ctxt.regs.ax = ghcb.save.rax;
    ctxt.regs.dx = ghcb.save.rdx;
    if rdtscp {
        ctxt.regs.cx = ghcb.save.rcx;
    }
    EsResult::Continue
}

pub const fn ghcb_msr_info(msr: u64) -> u64 {
    msr & GHCB_MSR_INFO_MASK
}

pub const fn ghcb_msr_cpuid_req(function: u32, register: u8) -> u64 {
    GHCB_MSR_CPUID_REQ | ((register as u64 & 0x3) << 30) | ((function as u64) << 32)
}

pub const fn ghcb_msr_psc_req(gfn: u64, op: u64) -> u64 {
    GHCB_MSR_PSC_REQ | ((gfn & 0x000f_ffff_ffff) << 12) | ((op & 0xf) << 52)
}

pub const fn ghcb_msr_reg_gpa_req(pfn: u64) -> u64 {
    ((pfn & 0x000f_ffff_ffff) << 12) | GHCB_MSR_REG_GPA_REQ
}

pub const fn ghcb_msr_reg_gpa_resp_val(msr: u64) -> u64 {
    (msr >> 12) & 0x000f_ffff_ffff
}

pub const fn ghcb_resp_code(msr: u64) -> u64 {
    msr & GHCB_MSR_INFO_MASK
}

pub const fn snp_register_ghcb_early_request(paddr: u64) -> u64 {
    ghcb_msr_reg_gpa_req(paddr >> 12)
}

pub const fn snp_register_ghcb_early_response_matches(paddr: u64, response: u64) -> bool {
    ghcb_resp_code(response) == GHCB_MSR_REG_GPA_RESP
        && ghcb_msr_reg_gpa_resp_val(response) == (paddr >> 12)
}

pub const fn snp_register_ghcb_early_plan(
    paddr: u64,
    response_msr: u64,
) -> SnpRegisterGhcbEarlyPlan {
    let ok = snp_register_ghcb_early_response_matches(paddr, response_msr);
    SnpRegisterGhcbEarlyPlan {
        paddr,
        pfn: paddr >> 12,
        request_msr: snp_register_ghcb_early_request(paddr),
        response_msr,
        ok,
        termination: if ok {
            None
        } else {
            Some(SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_REGISTER,
            })
        },
    }
}

pub const fn sev_es_check_cpu_features_plan(has_rdrand: bool) -> SevEsCpuFeaturePlan {
    SevEsCpuFeaturePlan {
        has_rdrand,
        ok: has_rdrand,
        error: if has_rdrand {
            None
        } else {
            Some(SEV_ES_RDRAND_ERROR)
        },
    }
}

pub const fn ghcb_msr_sev_info(max: u16, min: u16, cbit: u8) -> u64 {
    ((max as u64) << 48) | ((min as u64) << 32) | ((cbit as u64) << 24) | GHCB_MSR_SEV_INFO_RESP
}

pub const fn ghcb_msr_proto_max(msr: u64) -> u16 {
    ((msr >> 48) & 0xffff) as u16
}

pub const fn ghcb_msr_proto_min(msr: u64) -> u16 {
    ((msr >> 32) & 0xffff) as u16
}

pub const fn sev_es_negotiate_protocol_response(response: u64) -> Option<u16> {
    if ghcb_msr_info(response) != GHCB_MSR_SEV_INFO_RESP {
        return None;
    }
    let max = ghcb_msr_proto_max(response);
    let min = ghcb_msr_proto_min(response);
    if max < GHCB_PROTOCOL_MIN || min > GHCB_PROTOCOL_MAX {
        return None;
    }
    Some(if max < GHCB_PROTOCOL_MAX {
        max
    } else {
        GHCB_PROTOCOL_MAX
    })
}

pub fn ghcb_protocol_version() -> u16 {
    GHCB_VERSION.load(Ordering::Acquire)
}

pub fn set_ghcb_protocol_version(version: u16) {
    GHCB_VERSION.store(version, Ordering::Release);
}

pub fn sev_es_apply_negotiated_protocol(response: u64) -> bool {
    let Some(version) = sev_es_negotiate_protocol_response(response) else {
        return false;
    };
    set_ghcb_protocol_version(version);
    true
}

pub const fn sev_es_negotiate_protocol_plan(response: u64) -> SevEsProtocolNegotiationPlan {
    let mut plan = SevEsProtocolNegotiationPlan {
        request_msr: GHCB_MSR_SEV_INFO_REQ,
        wrote_request: true,
        vmgexit: true,
        read_response: true,
        response_msr: response,
        negotiated_version: None,
        failure: None,
    };

    if ghcb_msr_info(response) != GHCB_MSR_SEV_INFO_RESP {
        plan.failure = Some(SevEsProtocolFailure::BadResponseCode);
        return plan;
    }

    let max = ghcb_msr_proto_max(response);
    let min = ghcb_msr_proto_min(response);
    if max < GHCB_PROTOCOL_MIN || min > GHCB_PROTOCOL_MAX {
        plan.failure = Some(SevEsProtocolFailure::UnsupportedRange);
        return plan;
    }

    plan.negotiated_version = Some(if max < GHCB_PROTOCOL_MAX {
        max
    } else {
        GHCB_PROTOCOL_MAX
    });
    plan
}

pub fn ghcb_set_sw_exit_code(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.sw_exit_code = value;
}

pub fn ghcb_set_sw_exit_info_1(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.sw_exit_info_1 = value;
}

pub fn ghcb_set_sw_exit_info_2(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.sw_exit_info_2 = value;
}

pub fn ghcb_set_sw_scratch(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.sw_scratch = value;
}

pub fn ghcb_set_rax(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.rax = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_RAX;
}

pub fn ghcb_set_rbx(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.rbx = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_RBX;
}

pub fn ghcb_set_rcx(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.rcx = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_RCX;
}

pub fn ghcb_set_rdx(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.rdx = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_RDX;
}

pub fn ghcb_set_cpl(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.cpl = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_CPL;
}

pub fn ghcb_set_xcr0(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.xcr0 = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_XCR0;
}

pub fn ghcb_set_xss(ghcb: &mut Ghcb, value: u64) {
    ghcb.save.xss = value;
    ghcb.save.valid_bitmap |= GHCB_VALID_XSS;
}

pub fn vc_ghcb_invalidate(ghcb: &mut Ghcb) {
    ghcb.save.sw_exit_code = 0;
    ghcb.save.valid_bitmap = 0;
}

pub const fn ghcb_rax_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_RAX != 0
}

pub const fn ghcb_rbx_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_RBX != 0
}

pub const fn ghcb_rcx_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_RCX != 0
}

pub const fn ghcb_rdx_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_RDX != 0
}

pub const fn ghcb_cpl_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_CPL != 0
}

pub const fn ghcb_xcr0_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_XCR0 != 0
}

pub const fn ghcb_xss_is_valid(ghcb: &Ghcb) -> bool {
    ghcb.save.valid_bitmap & GHCB_VALID_XSS != 0
}

const fn x86_modrm_reg(modrm: u8) -> u8 {
    (modrm >> 3) & 7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctxt_for_opcode(opcode: u32, first: u8, modrm: u8) -> EsEmCtxt {
        let mut insn = EmulatedInsn::from_opcode(opcode, first);
        insn.modrm = modrm;
        vc_init_em_ctxt_with_regs(VcRegs::default(), 0, insn)
    }

    #[derive(Clone, Debug)]
    struct FakeIoioOps {
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
        call_count: usize,
        status: EsResult,
        out_rax: Option<u64>,
        segment_base: Option<u64>,
        read_addr: u64,
        write_addr: u64,
        read_count: u64,
        write_count: u64,
        read_df: bool,
        write_df: bool,
        shared_buffer_pa: u64,
        check_status: EsResult,
        check_port: u16,
        check_size: usize,
        check_count: usize,
    }

    impl Default for FakeIoioOps {
        fn default() -> Self {
            Self {
                exit_code: 0,
                exit_info_1: 0,
                exit_info_2: 0,
                call_count: 0,
                status: EsResult::Continue,
                out_rax: None,
                segment_base: Some(0),
                read_addr: 0,
                write_addr: 0,
                read_count: 0,
                write_count: 0,
                read_df: false,
                write_df: false,
                shared_buffer_pa: 0x7000,
                check_status: EsResult::Continue,
                check_port: 0,
                check_size: 0,
                check_count: 0,
            }
        }
    }

    impl VcIoioOps for FakeIoioOps {
        fn ghcb_hv_call(
            &mut self,
            ghcb: &mut Ghcb,
            _ctxt: &mut EsEmCtxt,
            exit_code: u64,
            exit_info_1: u64,
            exit_info_2: u64,
        ) -> EsResult {
            self.exit_code = exit_code;
            self.exit_info_1 = exit_info_1;
            self.exit_info_2 = exit_info_2;
            self.call_count += 1;
            if let Some(rax) = self.out_rax {
                ghcb_set_rax(ghcb, rax);
            }
            self.status
        }

        fn es_segment_base(&mut self, _ctxt: &EsEmCtxt) -> Option<u64> {
            self.segment_base
        }

        fn ghcb_shared_buffer_pa(&mut self, _ghcb: &Ghcb) -> u64 {
            self.shared_buffer_pa
        }

        fn ioio_check(&mut self, _ctxt: &mut EsEmCtxt, port: u16, size: usize) -> EsResult {
            self.check_port = port;
            self.check_size = size;
            self.check_count += 1;
            self.check_status
        }

        fn string_read(
            &mut self,
            _ctxt: &mut EsEmCtxt,
            addr: u64,
            buffer: &mut [u8],
            _io_bytes: usize,
            count: u64,
            df: bool,
        ) -> EsResult {
            self.read_addr = addr;
            self.read_count = count;
            self.read_df = df;
            let mut i = 0usize;
            while i < buffer.len() {
                buffer[i] = (i as u8).wrapping_add(1);
                i += 1;
            }
            EsResult::Continue
        }

        fn string_write(
            &mut self,
            _ctxt: &mut EsEmCtxt,
            addr: u64,
            buffer: &[u8],
            _io_bytes: usize,
            count: u64,
            df: bool,
        ) -> EsResult {
            self.write_addr = addr;
            self.write_count = count;
            self.write_df = df;
            assert!(buffer.iter().take(count as usize).any(|value| *value != 0));
            EsResult::Continue
        }
    }

    #[derive(Clone, Debug)]
    struct FakeCpuidOps {
        exit_code: u64,
        call_count: usize,
        status: EsResult,
        xcr0: u64,
        shstk_supported: bool,
        xss: u64,
        requested_rax: u64,
        requested_rcx: u64,
        requested_xcr0: u64,
        out_rax: Option<u64>,
        out_rbx: Option<u64>,
        out_rcx: Option<u64>,
        out_rdx: Option<u64>,
        cr4_osxsave: bool,
        snp_ok: bool,
        snp_result: VcSnpCpuidResult,
        snp_leaf: CpuidLeaf,
    }

    impl Default for FakeCpuidOps {
        fn default() -> Self {
            Self {
                exit_code: 0,
                call_count: 0,
                status: EsResult::Continue,
                xcr0: 1,
                shstk_supported: false,
                xss: 0,
                requested_rax: 0,
                requested_rcx: 0,
                requested_xcr0: 0,
                out_rax: None,
                out_rbx: None,
                out_rcx: None,
                out_rdx: None,
                cr4_osxsave: false,
                snp_ok: false,
                snp_result: VcSnpCpuidResult::VmmError,
                snp_leaf: CpuidLeaf::default(),
            }
        }
    }

    impl VcCpuidOps for FakeCpuidOps {
        fn ghcb_hv_call(
            &mut self,
            ghcb: &mut Ghcb,
            _ctxt: &mut EsEmCtxt,
            exit_code: u64,
            _exit_info_1: u64,
            _exit_info_2: u64,
        ) -> EsResult {
            self.exit_code = exit_code;
            self.call_count += 1;
            self.requested_rax = ghcb.save.rax;
            self.requested_rcx = ghcb.save.rcx;
            self.requested_xcr0 = ghcb.save.xcr0;
            if let Some(rax) = self.out_rax {
                ghcb_set_rax(ghcb, rax);
            }
            if let Some(rbx) = self.out_rbx {
                ghcb_set_rbx(ghcb, rbx);
            }
            if let Some(rcx) = self.out_rcx {
                ghcb_set_rcx(ghcb, rcx);
            }
            if let Some(rdx) = self.out_rdx {
                ghcb_set_rdx(ghcb, rdx);
            }
            self.status
        }

        fn xcr0(&mut self) -> u64 {
            self.xcr0
        }

        fn cr4_osxsave(&mut self) -> bool {
            self.cr4_osxsave
        }

        fn shstk_supported(&mut self) -> bool {
            self.shstk_supported
        }

        fn xss(&mut self) -> u64 {
            self.xss
        }

        fn snp_cpuid(
            &mut self,
            leaf: &mut CpuidLeaf,
            _ghcb: &mut Ghcb,
            _ctxt: &mut EsEmCtxt,
        ) -> bool {
            if !self.snp_ok {
                return false;
            }
            leaf.eax = self.snp_leaf.eax;
            leaf.ebx = self.snp_leaf.ebx;
            leaf.ecx = self.snp_leaf.ecx;
            leaf.edx = self.snp_leaf.edx;
            true
        }

        fn snp_cpuid_result(
            &mut self,
            leaf: &mut CpuidLeaf,
            ghcb: &mut Ghcb,
            ctxt: &mut EsEmCtxt,
        ) -> VcSnpCpuidResult {
            if self.snp_ok {
                let _ = self.snp_cpuid(leaf, ghcb, ctxt);
                VcSnpCpuidResult::Handled
            } else {
                self.snp_result
            }
        }
    }

    #[test]
    fn opcode_checker_matches_linux_exit_table() {
        assert_eq!(
            vc_check_opcode_bytes(&ctxt_for_opcode(0xa20f, 0x0f, 0), SVM_EXIT_CPUID),
            EsResult::Continue
        );
        assert_eq!(
            vc_check_opcode_bytes(&ctxt_for_opcode(0x010f, 0x0f, 0xfa), SVM_EXIT_MONITOR),
            EsResult::Continue
        );
        assert_eq!(
            vc_check_opcode_bytes(&ctxt_for_opcode(0x210f, 0x0f, 7 << 3), SVM_EXIT_READ_DR7),
            EsResult::Continue
        );
        assert_eq!(
            vc_check_opcode_bytes(&ctxt_for_opcode(0x210f, 0x0f, 6 << 3), SVM_EXIT_READ_DR7),
            EsResult::Unsupported
        );
    }

    #[test]
    fn exception_exits_skip_instruction_decode() {
        assert!(!vc_decoding_needed(SVM_EXIT_EXCP_BASE + X86_TRAP_GP));
        assert!(vc_decoding_needed(SVM_EXIT_CPUID));
    }

    #[test]
    fn init_em_ctxt_plan_zeroes_context_and_skips_decode_for_exception_exits() {
        let regs = VcRegs {
            ip: 0x1234,
            ax: 0x55,
            ..Default::default()
        };
        let plan = vc_init_em_ctxt_plan(
            regs,
            SVM_EXIT_EXCP_BASE + X86_TRAP_GP,
            EsResult::DecodeFailed,
        );

        assert!(!plan.decode_needed);
        assert_eq!(plan.result, EsResult::Continue);
        assert_eq!(plan.ctxt.regs, regs);
        assert_eq!(plan.ctxt.exit_code, SVM_EXIT_EXCP_BASE + X86_TRAP_GP);
        assert_eq!(plan.ctxt.insn, EmulatedInsn::default());
        assert_eq!(plan.ctxt.fi, FaultInfo::default());
    }

    #[test]
    fn init_em_ctxt_plan_invokes_decode_for_non_exception_exits() {
        let regs = VcRegs {
            ip: 0x5678,
            user_mode: true,
            ..Default::default()
        };
        let plan = vc_init_em_ctxt_plan(regs, SVM_EXIT_CPUID, EsResult::DecodeFailed);

        assert!(plan.decode_needed);
        assert_eq!(plan.result, EsResult::DecodeFailed);
        assert_eq!(plan.ctxt.regs, regs);
        assert_eq!(plan.ctxt.exit_code, SVM_EXIT_CPUID);
    }

    #[test]
    fn ioio_exitinfo_uses_linux_instruction_bits() {
        let mut regs = VcRegs {
            dx: 0x3f8,
            ..Default::default()
        };
        let mut insn = EmulatedInsn::from_opcode(0, 0xed);
        insn.opnd_bytes = 2;
        insn.addr_bytes = 8;
        let ctxt = vc_init_em_ctxt_with_regs(regs, SVM_EXIT_IOIO, insn);
        let info = vc_ioio_exitinfo(&ctxt).unwrap();
        assert_eq!(info & IOIO_TYPE_IN, IOIO_TYPE_IN);
        assert_eq!(info & IOIO_DATA_16, IOIO_DATA_16);
        assert_eq!(info & IOIO_ADDR_64, IOIO_ADDR_64);
        assert_eq!((info >> 16) & 0xffff, 0x3f8);

        regs.dx = 0x80;
        let mut insn = EmulatedInsn::from_opcode(0, 0x6f);
        insn.opnd_bytes = 4;
        insn.addr_bytes = 4;
        insn.rep_prefix = true;
        let ctxt = vc_init_em_ctxt_with_regs(regs, SVM_EXIT_IOIO, insn);
        let info = vc_ioio_exitinfo(&ctxt).unwrap();
        assert_eq!(info & IOIO_TYPE_STR, IOIO_TYPE_STR);
        assert_eq!(info & IOIO_SEG_DS, IOIO_SEG_DS);
        assert_eq!(info & IOIO_REP, IOIO_REP);
    }

    #[test]
    fn ioio_exitinfo_uses_linux_non_two_byte_default_to_dword() {
        let mut insn = EmulatedInsn::from_opcode(0, 0xed);
        insn.opnd_bytes = 8;
        let ctxt = vc_init_em_ctxt_with_regs(VcRegs::default(), SVM_EXIT_IOIO, insn);
        let info = vc_ioio_exitinfo(&ctxt).unwrap();
        assert_eq!(info & IOIO_DATA_32, IOIO_DATA_32);

        let mut insn = EmulatedInsn::from_opcode(0, 0xed);
        insn.opnd_bytes = 1;
        let ctxt = vc_init_em_ctxt_with_regs(VcRegs::default(), SVM_EXIT_IOIO, insn);
        let info = vc_ioio_exitinfo(&ctxt).unwrap();
        assert_eq!(info & IOIO_DATA_32, IOIO_DATA_32);

        let mut byte_insn = EmulatedInsn::from_opcode(0, 0xec);
        byte_insn.opnd_bytes = 8;
        let ctxt = vc_init_em_ctxt_with_regs(VcRegs::default(), SVM_EXIT_IOIO, byte_insn);
        let info = vc_ioio_exitinfo(&ctxt).unwrap();
        assert_eq!(info & IOIO_DATA_8, IOIO_DATA_8);
    }

    #[test]
    fn compat_ioio_helper_keeps_old_callers_source_anchored() {
        let info = vc_ioio_exitinfo_compat(2, 0x3f8, false).unwrap();
        assert_eq!(info & IOIO_TYPE_IN, IOIO_TYPE_IN);
        assert_eq!(info & IOIO_DATA_16, IOIO_DATA_16);
        assert_eq!((info >> 16) & 0xffff, 0x3f8);
        assert_eq!(vc_ioio_exitinfo_compat(3, 0, false), Err(EINVAL));
    }

    #[test]
    fn checked_ioio_exitinfo_preserves_linux_check_boundary() {
        let mut ops = FakeIoioOps {
            check_status: EsResult::VmmError,
            ..Default::default()
        };
        let mut insn = EmulatedInsn::from_opcode(0, 0xe5);
        insn.immediate = 0x64;
        insn.opnd_bytes = 2;
        let mut ctxt = vc_init_em_ctxt_with_regs(VcRegs::default(), SVM_EXIT_IOIO, insn);

        assert_eq!(
            vc_ioio_exitinfo_checked_with(&mut ops, &mut ctxt),
            Err(EsResult::VmmError)
        );
        assert_eq!(ops.check_count, 1);
        assert_eq!(ops.check_port, 0x64);
        assert_eq!(ops.check_size, 2);
    }

    #[test]
    fn string_io_plan_limits_rep_to_ghcb_buffer() {
        let mut insn = EmulatedInsn::from_opcode(0, 0x6c);
        insn.rep_prefix = true;
        let ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                dx: 0x60,
                cx: 4096,
                ..Default::default()
            },
            SVM_EXIT_IOIO,
            insn,
        );
        let info = vc_ioio_exitinfo(&ctxt).unwrap();
        assert_eq!(ioio_string_plan(info, ctxt.regs.cx), Some((1, 2032, 2032)));
    }

    #[test]
    fn ioio_handler_runs_linux_check_before_vmgexit() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeIoioOps {
            check_status: EsResult::Exception,
            ..Default::default()
        };
        let mut insn = EmulatedInsn::from_opcode(0, 0xe4);
        insn.immediate = 0x64;
        let mut ctxt = vc_init_em_ctxt_with_regs(VcRegs::default(), SVM_EXIT_IOIO, insn);

        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Exception
        );
        assert_eq!(ops.check_count, 1);
        assert_eq!(ops.check_port, 0x64);
        assert_eq!(ops.check_size, 1);
        assert_eq!(ops.call_count, 0);
    }

    #[test]
    fn ioio_handler_uses_linux_rax_ghcb_protocol_for_scalar_in_out() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeIoioOps::default();
        let mut out_insn = EmulatedInsn::from_opcode(0, 0xe7);
        out_insn.immediate = 0x80;
        out_insn.opnd_bytes = 4;
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                ax: 0xffff_ffff_1234_5678,
                ..Default::default()
            },
            SVM_EXIT_IOIO,
            out_insn,
        );

        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_IOIO);
        assert_eq!(ops.exit_info_2, 0);
        assert_eq!(ghcb.save.rax, 0x1234_5678);
        assert_eq!((ops.exit_info_1 >> 16) & 0xffff, 0x80);

        let mut in_insn = EmulatedInsn::from_opcode(0, 0xe4);
        in_insn.immediate = 0x64;
        in_insn.opnd_bytes = 4;
        let mut ctxt = vc_init_em_ctxt_with_regs(VcRegs::default(), SVM_EXIT_IOIO, in_insn);
        ops.out_rax = Some(0x1234);
        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ctxt.regs.ax, 0x34);

        assert!(ghcb_rax_is_valid(&ghcb));
    }

    #[test]
    fn ioio_handler_models_linux_string_updates_and_shared_buffer() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeIoioOps {
            segment_base: Some(0x1000),
            ..Default::default()
        };
        let mut outs = EmulatedInsn::from_opcode(0, 0x6e);
        outs.rep_prefix = true;
        outs.opnd_bytes = 1;
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                si: 0x20,
                cx: 3,
                flags: X86_EFLAGS_DF,
                ..Default::default()
            },
            SVM_EXIT_IOIO,
            outs,
        );

        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_info_2, 3);
        assert_eq!(ops.read_addr, 0x1020);
        assert_eq!(ops.read_count, 3);
        assert!(ops.read_df);
        assert_eq!(&ghcb.shared_buffer[..3], &[1, 2, 3]);
        assert_eq!(ghcb.save.sw_scratch, 0x7000);
        assert_eq!(ctxt.regs.si, 0x20u64.wrapping_sub(3));
        assert_eq!(ctxt.regs.cx, 0);

        let mut ins = EmulatedInsn::from_opcode(0, 0x6d);
        ins.rep_prefix = true;
        ins.opnd_bytes = 4;
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                di: 0x40,
                cx: (GHCB_SHARED_BUFFER_SIZE / 4 + 2) as u64,
                ..Default::default()
            },
            SVM_EXIT_IOIO,
            ins,
        );
        ghcb.shared_buffer[..4].copy_from_slice(&[9, 8, 7, 6]);
        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Retry
        );
        assert_eq!(ops.exit_info_2, (GHCB_SHARED_BUFFER_SIZE / 4) as u64);
        assert_eq!(ghcb.save.sw_scratch, 0x7000);
        assert_eq!(ops.write_addr, 0x1040);
        assert_eq!(ops.write_count, (GHCB_SHARED_BUFFER_SIZE / 4) as u64);
        assert_eq!(ctxt.regs.di, 0x40 + GHCB_SHARED_BUFFER_SIZE as u64);
        assert_eq!(ctxt.regs.cx, 2);
    }

    #[test]
    fn string_io_rejects_user_mode_kernel_addresses_before_copy_or_vmgexit() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeIoioOps {
            segment_base: Some(TASK_SIZE_MAX),
            ..Default::default()
        };
        let mut outs = EmulatedInsn::from_opcode(0, 0x6e);
        outs.opnd_bytes = 1;
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                si: 0x20,
                cx: 1,
                user_mode: true,
                ..Default::default()
            },
            SVM_EXIT_IOIO,
            outs,
        );

        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_PF);
        assert_eq!(ctxt.fi.error_code, X86_PF_USER);
        assert_eq!(ctxt.fi.cr2, TASK_SIZE_MAX + 0x20);
        assert_eq!(ops.read_count, 0);
        assert_eq!(ops.call_count, 0);

        let mut ins = EmulatedInsn::from_opcode(0, 0x6c);
        ins.opnd_bytes = 1;
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                di: 0x30,
                cx: 1,
                user_mode: true,
                ..Default::default()
            },
            SVM_EXIT_IOIO,
            ins,
        );
        assert_eq!(
            vc_handle_ioio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_PF);
        assert_eq!(ctxt.fi.error_code, X86_PF_USER | X86_PF_WRITE);
        assert_eq!(ctxt.fi.cr2, TASK_SIZE_MAX + 0x30);
        assert_eq!(ops.write_count, 0);
        assert_eq!(ops.call_count, 1);
    }

    #[test]
    fn string_io_kernel_space_check_keeps_linux_vsyscall_exception() {
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                user_mode: true,
                ..Default::default()
            },
            ..Default::default()
        };

        assert!(fault_in_kernel_space(TASK_SIZE_MAX));
        assert!(!fault_in_kernel_space(VSYSCALL_ADDR));
        assert_eq!(
            vc_insn_string_check(&mut ctxt, VSYSCALL_ADDR + 0x800, true),
            EsResult::Continue
        );
        assert_eq!(ctxt.fi, FaultInfo::default());
    }

    #[test]
    fn ghcb_call_populates_protocol_and_exit_fields() {
        let _version_guard = ghcb_version_test_guard(0);
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt(SVM_VMGEXIT_PSC, 0, 0);
        assert_eq!(
            sev_es_ghcb_hv_call(&mut ghcb, &mut ctxt, SVM_VMGEXIT_PSC, 0, 2),
            EsResult::Continue
        );
        assert_eq!(ghcb.protocol_version, 0);
        assert_eq!(ghcb.ghcb_usage, GHCB_DEFAULT_USAGE);
        assert_eq!(ghcb.save.sw_exit_code, SVM_VMGEXIT_PSC);
        assert_eq!(ghcb.save.sw_exit_info_1, 0);
        assert_eq!(ghcb.save.sw_exit_info_2, 2);
    }

    #[test]
    fn ghcb_call_uses_negotiated_protocol_version_like_linux() {
        let _version_guard = ghcb_version_test_guard(0);
        assert!(sev_es_apply_negotiated_protocol(ghcb_msr_sev_info(1, 1, 0)));
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt(SVM_VMGEXIT_PSC, 0, 0);
        assert_eq!(
            sev_es_ghcb_hv_call(&mut ghcb, &mut ctxt, SVM_VMGEXIT_PSC, 0, 2),
            EsResult::Continue
        );
        assert_eq!(ghcb.protocol_version, 1);
        assert_eq!(ghcb.ghcb_usage, GHCB_DEFAULT_USAGE);
        assert_eq!(ghcb.save.sw_exit_code, SVM_VMGEXIT_PSC);
        assert_eq!(ghcb.save.sw_exit_info_1, 0);
        assert_eq!(ghcb.save.sw_exit_info_2, 2);
    }

    #[test]
    fn exception_info_accepts_only_gp_or_ud_exception_injection() {
        let mut ctxt = vc_init_em_ctxt(SVM_EXIT_CPUID, 0, 0);
        let mut ghcb = Ghcb::default();
        ghcb.save.sw_exit_info_1 = 1;
        ghcb.save.sw_exit_info_2 = SVM_EVTINJ_VALID
            | SVM_EVTINJ_VALID_ERR
            | SVM_EVTINJ_TYPE_EXEPT
            | X86_TRAP_GP
            | (7 << 32);
        assert_eq!(verify_exception_info(&ghcb, &mut ctxt), EsResult::Exception);
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);
        assert_eq!(ctxt.fi.error_code, 7);

        ctxt.fi = FaultInfo::default();
        ghcb.save.sw_exit_info_2 = SVM_EVTINJ_VALID | SVM_EVTINJ_TYPE_EXEPT | X86_TRAP_PF;
        assert_eq!(verify_exception_info(&ghcb, &mut ctxt), EsResult::VmmError);
    }

    #[test]
    fn cpuid_hv_path_requires_all_result_registers_valid() {
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt(SVM_EXIT_CPUID, 0, 0);
        let mut leaf = CpuidLeaf {
            fn_: 0,
            subfn: 1,
            ..Default::default()
        };
        assert_eq!(
            sev_cpuid_hv_ghcb_with(&mut ghcb, &mut ctxt, &mut leaf, false, 0, |ghcb| {
                ghcb_set_rax(ghcb, 1);
                ghcb_set_rbx(ghcb, 2);
                ghcb_set_rcx(ghcb, 3);
                ghcb_set_rdx(ghcb, 4);
            }),
            EsResult::Continue
        );
        assert!(ghcb_xcr0_is_valid(&ghcb));
        assert_eq!(ghcb.save.xcr0, 1);
        assert_eq!((leaf.eax, leaf.ebx, leaf.ecx, leaf.edx), (1, 2, 3, 4));

        let mut osxsave = Ghcb::default();
        assert_eq!(
            sev_cpuid_hv_ghcb_with(&mut osxsave, &mut ctxt, &mut leaf, true, 0x37, |ghcb| {
                ghcb_set_rax(ghcb, 1);
                ghcb_set_rbx(ghcb, 2);
                ghcb_set_rcx(ghcb, 3);
                ghcb_set_rdx(ghcb, 4);
            }),
            EsResult::Continue
        );
        assert_eq!(osxsave.save.xcr0, 0x37);

        let mut bad = Ghcb::default();
        assert_eq!(
            sev_cpuid_hv_ghcb_with(&mut bad, &mut ctxt, &mut leaf, false, 0, |ghcb| {
                ghcb_set_rax(ghcb, 1);
            }),
            EsResult::VmmError
        );
    }

    #[test]
    fn snp_cpuid_hv_ghcb_plan_terminates_on_callback_failure() {
        assert_eq!(
            snp_cpuid_hv_ghcb_plan(EsResult::Continue),
            SnpCpuidHvGhcbPlan {
                result: EsResult::Continue,
                termination: None,
            }
        );
        assert_eq!(
            snp_cpuid_hv_ghcb_plan(EsResult::VmmError),
            SnpCpuidHvGhcbPlan {
                result: EsResult::VmmError,
                termination: Some(SevTermination {
                    set: SEV_TERM_SET_LINUX,
                    reason: GHCB_TERM_CPUID_HV,
                }),
            }
        );
    }

    #[test]
    fn cpuid_handler_copies_regs_through_linux_ghcb_path() {
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                ax: 0x8000_0001,
                cx: 3,
                ..Default::default()
            },
            SVM_EXIT_CPUID,
            EmulatedInsn::from_opcode(0xa20f, 0x0f),
        );
        let mut ops = FakeCpuidOps {
            xcr0: 0x37,
            cr4_osxsave: true,
            out_rax: Some(1),
            out_rbx: Some(2),
            out_rcx: Some(3),
            out_rdx: Some(4),
            ..Default::default()
        };

        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, false),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_CPUID);
        assert_eq!(ops.call_count, 1);
        assert_eq!(ops.requested_rax, 0x8000_0001);
        assert_eq!(ops.requested_rcx, 3);
        assert_eq!(ops.requested_xcr0, 0x37);
        assert!(ghcb_xcr0_is_valid(&ghcb));
        assert!(!ghcb_xss_is_valid(&ghcb));
        assert_eq!(
            (ctxt.regs.ax, ctxt.regs.bx, ctxt.regs.cx, ctxt.regs.dx),
            (1, 2, 3, 4)
        );

        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                ax: 0x8000_0001,
                cx: 3,
                ..Default::default()
            },
            SVM_EXIT_CPUID,
            EmulatedInsn::from_opcode(0xa20f, 0x0f),
        );
        let mut ops = FakeCpuidOps {
            xcr0: 0x37,
            cr4_osxsave: false,
            out_rax: Some(1),
            out_rbx: Some(2),
            out_rcx: Some(3),
            out_rdx: Some(4),
            ..Default::default()
        };

        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, false),
            EsResult::Continue
        );
        assert_eq!(ops.requested_xcr0, 1);
    }

    #[test]
    fn cpuid_handler_sets_linux_xss_for_shstk_leaf_0xd_subleaf_1() {
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                ax: 0xd,
                cx: 1,
                ..Default::default()
            },
            SVM_EXIT_CPUID,
            EmulatedInsn::from_opcode(0xa20f, 0x0f),
        );
        let mut ops = FakeCpuidOps {
            shstk_supported: true,
            xss: 0xaa55,
            out_rax: Some(1),
            out_rbx: Some(2),
            out_rcx: Some(3),
            out_rdx: Some(4),
            ..Default::default()
        };

        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, false),
            EsResult::Continue
        );
        assert!(ghcb_xss_is_valid(&ghcb));
        assert_eq!(ghcb.save.xss, 0xaa55);

        let mut ghcb = Ghcb::default();
        ctxt.regs.cx = 0;
        let mut ops = FakeCpuidOps {
            shstk_supported: true,
            xss: 0xaa55,
            out_rax: Some(1),
            out_rbx: Some(2),
            out_rcx: Some(3),
            out_rdx: Some(4),
            ..Default::default()
        };
        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, false),
            EsResult::Continue
        );
        assert!(!ghcb_xss_is_valid(&ghcb));
    }

    #[test]
    fn cpuid_handler_falls_back_from_snp_eopnotsupp_like_linux() {
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                ax: 0xd,
                cx: 1,
                ..Default::default()
            },
            SVM_EXIT_CPUID,
            EmulatedInsn::from_opcode(0xa20f, 0x0f),
        );
        let mut ops = FakeCpuidOps {
            snp_result: VcSnpCpuidResult::Unsupported,
            xcr0: 0x55,
            cr4_osxsave: true,
            out_rax: Some(0x11),
            out_rbx: Some(0x22),
            out_rcx: Some(0x33),
            out_rdx: Some(0x44),
            ..Default::default()
        };

        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, true),
            EsResult::Continue
        );
        assert_eq!(ops.call_count, 1);
        assert_eq!(ops.requested_rax, 0xd);
        assert_eq!(ops.requested_rcx, 1);
        assert_eq!(ops.requested_xcr0, 0x55);
        assert_eq!(
            (ctxt.regs.ax, ctxt.regs.bx, ctxt.regs.cx, ctxt.regs.dx),
            (0x11, 0x22, 0x33, 0x44)
        );
    }

    #[test]
    fn cpuid_handler_models_linux_snp_table_or_vmm_error() {
        let mut ghcb = Ghcb::default();
        let mut ctxt = vc_init_em_ctxt_with_regs(
            VcRegs {
                ax: 0x1,
                cx: 0,
                ..Default::default()
            },
            SVM_EXIT_CPUID,
            EmulatedInsn::from_opcode(0xa20f, 0x0f),
        );
        let mut ops = FakeCpuidOps {
            snp_ok: true,
            snp_leaf: CpuidLeaf {
                eax: 0xa,
                ebx: 0xb,
                ecx: 0xc,
                edx: 0xd,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, true),
            EsResult::Continue
        );
        assert_eq!(ops.call_count, 0);
        assert_eq!(
            (ctxt.regs.ax, ctxt.regs.bx, ctxt.regs.cx, ctxt.regs.dx),
            (0xa, 0xb, 0xc, 0xd)
        );

        ops.snp_ok = false;
        assert_eq!(
            vc_handle_cpuid_with(&mut ops, &mut ghcb, &mut ctxt, true),
            EsResult::VmmError
        );
    }

    #[test]
    fn rdtsc_rejects_secure_tsc_and_requires_valid_outputs() {
        let mut ghcb = Ghcb::default();
        ghcb_set_rax(&mut ghcb, 0xaaaa);
        ghcb_set_rdx(&mut ghcb, 0xbbbb);
        ghcb_set_rcx(&mut ghcb, 0xcccc);
        let mut ctxt = vc_init_em_ctxt(SVM_EXIT_RDTSCP, 0, 0);
        assert_eq!(
            vc_handle_rdtsc(&mut ghcb, &mut ctxt, SVM_EXIT_RDTSCP, false),
            EsResult::Continue
        );
        assert_eq!(
            (ctxt.regs.ax, ctxt.regs.dx, ctxt.regs.cx),
            (0xaaaa, 0xbbbb, 0xcccc)
        );
        assert_eq!(
            vc_handle_rdtsc(&mut ghcb, &mut ctxt, SVM_EXIT_RDTSC, true),
            EsResult::VmmError
        );
    }

    #[test]
    fn ghcb_msr_requests_keep_opcode_in_low_bits() {
        let cpuid = ghcb_msr_cpuid_req(0x8000_0001, 2);
        assert_eq!(ghcb_msr_info(cpuid), GHCB_MSR_CPUID_REQ);
        assert_eq!((cpuid >> 30) & 0x3, 2);

        let psc = ghcb_msr_psc_req(0x1234, 2);
        assert_eq!(ghcb_msr_info(psc), GHCB_MSR_PSC_REQ);
        assert_eq!((psc >> 52) & 0xf, 2);
    }

    #[test]
    fn ghcb_registration_and_protocol_negotiation_follow_msr_layout() {
        let paddr = 0x1234_5000;
        let req = snp_register_ghcb_early_request(paddr);
        assert_eq!(ghcb_msr_info(req), GHCB_MSR_REG_GPA_REQ);
        let resp = ((paddr >> 12) << 12) | GHCB_MSR_REG_GPA_RESP;
        assert!(snp_register_ghcb_early_response_matches(paddr, resp));
        assert_eq!(
            snp_register_ghcb_early_plan(paddr, resp),
            SnpRegisterGhcbEarlyPlan {
                paddr,
                pfn: paddr >> 12,
                request_msr: req,
                response_msr: resp,
                ok: true,
                termination: None,
            }
        );
        assert!(!snp_register_ghcb_early_response_matches(
            paddr + 0x1000,
            resp
        ));
        assert_eq!(
            snp_register_ghcb_early_plan(paddr + 0x1000, resp).termination,
            Some(SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_REGISTER,
            })
        );

        let info = ghcb_msr_sev_info(3, 1, 47);
        assert_eq!(
            sev_es_negotiate_protocol_response(info),
            Some(GHCB_PROTOCOL_MAX)
        );
        assert_eq!(
            sev_es_negotiate_protocol_response(GHCB_MSR_SEV_INFO_REQ),
            None
        );
    }

    #[test]
    fn sev_es_cpu_feature_check_requires_linux_rdrand_gate() {
        assert_eq!(
            sev_es_check_cpu_features_plan(true),
            SevEsCpuFeaturePlan {
                has_rdrand: true,
                ok: true,
                error: None,
            }
        );
        assert_eq!(
            sev_es_check_cpu_features_plan(false),
            SevEsCpuFeaturePlan {
                has_rdrand: false,
                ok: false,
                error: Some(SEV_ES_RDRAND_ERROR),
            }
        );
    }

    #[test]
    fn sev_es_protocol_negotiation_plan_matches_linux_msr_sequence_and_ranges() {
        let response = ghcb_msr_sev_info(3, 1, 47);
        assert_eq!(
            sev_es_negotiate_protocol_plan(response),
            SevEsProtocolNegotiationPlan {
                request_msr: GHCB_MSR_SEV_INFO_REQ,
                wrote_request: true,
                vmgexit: true,
                read_response: true,
                response_msr: response,
                negotiated_version: Some(GHCB_PROTOCOL_MAX),
                failure: None,
            }
        );

        let lower_max = ghcb_msr_sev_info(1, 1, 0);
        assert_eq!(
            sev_es_negotiate_protocol_plan(lower_max).negotiated_version,
            Some(1)
        );

        assert_eq!(
            sev_es_negotiate_protocol_plan(GHCB_MSR_SEV_INFO_REQ).failure,
            Some(SevEsProtocolFailure::BadResponseCode)
        );
        assert_eq!(
            sev_es_negotiate_protocol_plan(ghcb_msr_sev_info(0, 0, 0)).failure,
            Some(SevEsProtocolFailure::UnsupportedRange)
        );
        assert_eq!(
            sev_es_negotiate_protocol_plan(ghcb_msr_sev_info(3, 3, 0)).failure,
            Some(SevEsProtocolFailure::UnsupportedRange)
        );
    }
}
