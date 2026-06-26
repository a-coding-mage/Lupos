//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev/vc-handle.c
//! test-origin: linux:vendor/linux/arch/x86/coco/sev/vc-handle.c
//! SEV-ES #VC handler dispatch.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/vc-handle.c

use crate::include::uapi::errno::{EINVAL, EOPNOTSUPP};

pub use super::internal::DR7_RESET_VALUE;

use super::core::{GHCB_SEV_ES_GEN_REQ, SEV_TERM_SET_GEN, SevTermination};
use super::vc_shared::{
    CpuidLeaf, EmulatedInsn, EsEmCtxt, EsResult, Ghcb, SVM_EXIT_CPUID, SVM_EXIT_EXCP_BASE,
    SVM_EXIT_INVD, SVM_EXIT_IOIO, SVM_EXIT_MONITOR, SVM_EXIT_MSR, SVM_EXIT_MWAIT, SVM_EXIT_NPF,
    SVM_EXIT_RDPMC, SVM_EXIT_RDTSC, SVM_EXIT_RDTSCP, SVM_EXIT_READ_DR7, SVM_EXIT_VMMCALL,
    SVM_EXIT_WBINVD, SVM_EXIT_WRITE_DR7, SVM_VMGEXIT_MMIO_READ, SVM_VMGEXIT_MMIO_WRITE,
    SVM_VMGEXIT_UNSUPPORTED_EVENT, VcCpuidOps, VcIoioOps, VcRegs, X86_EFLAGS_DF, X86_PF_INSTR,
    X86_PF_PROT, X86_PF_USER, X86_PF_WRITE, X86_TRAP_AC, X86_TRAP_GP, X86_TRAP_PF, X86_TRAP_UD,
    ghcb_cpl_is_valid, ghcb_rax_is_valid, ghcb_rcx_is_valid, ghcb_rdx_is_valid, ghcb_set_cpl,
    ghcb_set_rax, ghcb_set_rbx, ghcb_set_rcx, ghcb_set_rdx, ghcb_set_sw_exit_info_1,
    ghcb_set_sw_exit_info_2, ghcb_set_sw_scratch, sev_es_ghcb_hv_call, vc_check_opcode_bytes,
    vc_finish_insn, vc_ghcb_invalidate, vc_handle_cpuid_with, vc_handle_ioio_with,
};

pub const X86_TRAP_VC: u8 = 29;
pub const X86_TRAP_DB: u8 = 1;
pub const MSR_IA32_TSC: u32 = 0x0000_0010;
pub const MSR_AMD64_GUEST_TSC_FREQ: u32 = 0xc001_0134;
pub const MSR_AMD64_SAVIC_CONTROL: u32 = 0xc001_0138;
pub const MSR_SVSM_CAA: u32 = 0xc001_f000;
pub const MSR_AMD64_SNP_DEBUG_SWAP: u64 = 1 << 7;
pub const MSR_AMD64_SNP_SECURE_TSC: u64 = 1 << 11;
pub const MSR_AMD64_SEV_SNP_ENABLED: u64 = 1 << 2;
pub const DR7_RESERVED_CLEAR_MASK: u64 = 0xffff_23ff;
pub const PAGE_SHIFT: u64 = 12;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcPhysMapping {
    pub pfn: u64,
    pub page_level_mask: u64,
    pub encrypted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcMmioType {
    Write,
    WriteImm,
    Read,
    Movs,
    ReadZeroExtend,
    ReadSignExtend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcAction {
    Cpuid,
    Io,
    MsrRead,
    MsrWrite,
    ReadDr7,
    WriteDr7,
    TrapAc,
    Rdtsc,
    Rdpmc,
    NestedPageFaultMmio,
    Halt,
    NmiComplete,
    VmmCall,
    Wbinvd,
    Monitor,
    Mwait,
    ForwardException,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcStackBounds {
    pub bottom: u64,
    pub top: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcKernelEntryAction {
    PanicInvalidContext,
    DebugException,
    RawHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcUserEntryAction {
    DebugException,
    RawHandle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcBootGhcbAction {
    FinishInsn,
    FailTerminate,
    ForwardException,
    Retry,
    Bug,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcRawHandleResultAction {
    FinishInsn,
    FailUnsupported,
    FailVmmError,
    FailDecodeFailed,
    ForwardException,
    Retry,
    Bug,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcBootGhcbPlan {
    pub exit_code: u64,
    pub invalidate_boot_ghcb: bool,
    pub init_result: EsResult,
    pub handle_exitcode: bool,
    pub result: EsResult,
    pub action: VcBootGhcbAction,
    pub success: bool,
    pub termination: Option<SevTermination>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcDecodePath {
    User,
    Kernel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcForwardExceptionAction {
    GeneralProtection,
    InvalidOpcode,
    PageFault,
    AlignmentCheck,
    Bug,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcForwardExceptionPlan {
    pub trapnr: u64,
    pub error_code: u64,
    pub orig_ax: u64,
    pub write_cr2: bool,
    pub cr2: u64,
    pub action: VcForwardExceptionAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcEarlyForwardExceptionPlan {
    pub trapnr: u64,
    pub orig_ax: u64,
    pub write_cr2: bool,
    pub cr2: u64,
    pub do_early_exception: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VcHandlerState {
    pub sev_status: u64,
    pub secure_avic: bool,
    pub svsm_caa_pa: u64,
    pub secure_tsc_now: u64,
    pub runtime_dr7: Option<u64>,
}

impl Default for VcHandlerState {
    fn default() -> Self {
        Self {
            sev_status: 0,
            secure_avic: false,
            svsm_caa_pa: 0,
            secure_tsc_now: 0,
            runtime_dr7: None,
        }
    }
}

pub trait VcGhcbOps {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult;

    fn vmmcall_prepare(&mut self, _ghcb: &mut Ghcb, _regs: &mut VcRegs) {}

    fn vmmcall_finish(&mut self, _ghcb: &mut Ghcb, _regs: &mut VcRegs) -> bool {
        true
    }

    fn xcr0(&mut self) -> u64 {
        1
    }

    fn cr4_osxsave(&mut self) -> bool {
        false
    }

    fn snp_cpuid(&mut self, _leaf: &mut CpuidLeaf, _ghcb: &mut Ghcb, _ctxt: &mut EsEmCtxt) -> bool {
        false
    }

    fn io_bitmap_allows(&mut self, _port: u32) -> Option<bool> {
        None
    }

    fn lookup_address(&mut self, _vaddr: u64) -> Option<VcPhysMapping> {
        None
    }

    fn read_mem(&mut self, _addr: u64, _buf: &mut [u8]) -> bool {
        false
    }

    fn write_mem(&mut self, _addr: u64, _buf: &[u8]) -> bool {
        false
    }

    fn ghcb_shared_buffer_pa(&mut self, ghcb: &Ghcb) -> u64 {
        ghcb.shared_buffer.as_ptr() as u64
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultVcGhcbOps;

impl VcGhcbOps for DefaultVcGhcbOps {
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

struct VcIoioAdapter<'a, O>(&'a mut O);

impl<O: VcGhcbOps> VcIoioOps for VcIoioAdapter<'_, O> {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult {
        self.0
            .ghcb_hv_call(ghcb, ctxt, exit_code, exit_info_1, exit_info_2)
    }

    fn ghcb_shared_buffer_pa(&mut self, ghcb: &Ghcb) -> u64 {
        self.0.ghcb_shared_buffer_pa(ghcb)
    }

    fn ioio_check(&mut self, ctxt: &mut EsEmCtxt, port: u16, size: usize) -> EsResult {
        vc_ioio_check_with(self.0, ctxt, port, size)
    }

    fn es_segment_base(&mut self, ctxt: &EsEmCtxt) -> Option<u64> {
        ctxt.insn.es_base
    }

    fn string_read(
        &mut self,
        ctxt: &mut EsEmCtxt,
        addr: u64,
        buffer: &mut [u8],
        io_bytes: usize,
        count: u64,
        df: bool,
    ) -> EsResult {
        let mut index = 0u64;
        while index < count {
            let offset = index as usize * io_bytes;
            let step = index.wrapping_mul(io_bytes as u64);
            let src = if df {
                addr.wrapping_sub(step)
            } else {
                addr.wrapping_add(step)
            };
            let ret = vc_read_mem_with(self.0, ctxt, src, &mut buffer[offset..offset + io_bytes]);
            if ret != EsResult::Continue {
                return ret;
            }
            index += 1;
        }
        EsResult::Continue
    }

    fn string_write(
        &mut self,
        ctxt: &mut EsEmCtxt,
        addr: u64,
        buffer: &[u8],
        io_bytes: usize,
        count: u64,
        df: bool,
    ) -> EsResult {
        let mut index = 0u64;
        while index < count {
            let offset = index as usize * io_bytes;
            let step = index.wrapping_mul(io_bytes as u64);
            let dst = if df {
                addr.wrapping_sub(step)
            } else {
                addr.wrapping_add(step)
            };
            let ret = vc_write_mem_with(self.0, ctxt, dst, &buffer[offset..offset + io_bytes]);
            if ret != EsResult::Continue {
                return ret;
            }
            index += 1;
        }
        EsResult::Continue
    }
}

struct VcCpuidAdapter<'a, O>(&'a mut O);

impl<O: VcGhcbOps> VcCpuidOps for VcCpuidAdapter<'_, O> {
    fn ghcb_hv_call(
        &mut self,
        ghcb: &mut Ghcb,
        ctxt: &mut EsEmCtxt,
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
    ) -> EsResult {
        self.0
            .ghcb_hv_call(ghcb, ctxt, exit_code, exit_info_1, exit_info_2)
    }

    fn xcr0(&mut self) -> u64 {
        self.0.xcr0()
    }

    fn cr4_osxsave(&mut self) -> bool {
        self.0.cr4_osxsave()
    }

    fn snp_cpuid(&mut self, leaf: &mut CpuidLeaf, ghcb: &mut Ghcb, ctxt: &mut EsEmCtxt) -> bool {
        self.0.snp_cpuid(leaf, ghcb, ctxt)
    }
}

pub const fn vc_handle_exitcode(exit_code: u64, write: bool) -> VcAction {
    match exit_code {
        SVM_EXIT_READ_DR7 => VcAction::ReadDr7,
        SVM_EXIT_WRITE_DR7 => VcAction::WriteDr7,
        code if code == SVM_EXIT_EXCP_BASE + X86_TRAP_AC => VcAction::TrapAc,
        SVM_EXIT_RDTSC | SVM_EXIT_RDTSCP => VcAction::Rdtsc,
        SVM_EXIT_CPUID => VcAction::Cpuid,
        SVM_EXIT_INVD => VcAction::Unsupported,
        SVM_EXIT_IOIO => VcAction::Io,
        SVM_EXIT_MSR if write => VcAction::MsrWrite,
        SVM_EXIT_MSR => VcAction::MsrRead,
        SVM_EXIT_VMMCALL => VcAction::VmmCall,
        SVM_EXIT_RDPMC => VcAction::Rdpmc,
        SVM_EXIT_WBINVD => VcAction::Wbinvd,
        SVM_EXIT_MONITOR => VcAction::Monitor,
        SVM_EXIT_MWAIT => VcAction::Mwait,
        SVM_EXIT_NPF => VcAction::NestedPageFaultMmio,
        SVM_VMGEXIT_UNSUPPORTED_EVENT => VcAction::Unsupported,
        _ => VcAction::Unsupported,
    }
}

pub fn vc_handle_exitcode_with_ghcb(ghcb: &mut Ghcb, exit_code: u64, write: bool) -> VcAction {
    vc_ghcb_invalidate(ghcb);
    vc_handle_exitcode(exit_code, write)
}

pub fn vc_forward_exception(ctxt: &EsEmCtxt) -> VcForwardExceptionPlan {
    let action = match ctxt.fi.vector {
        X86_TRAP_GP => VcForwardExceptionAction::GeneralProtection,
        X86_TRAP_UD => VcForwardExceptionAction::InvalidOpcode,
        X86_TRAP_PF => VcForwardExceptionAction::PageFault,
        X86_TRAP_AC => VcForwardExceptionAction::AlignmentCheck,
        _ => VcForwardExceptionAction::Bug,
    };

    VcForwardExceptionPlan {
        trapnr: ctxt.fi.vector,
        error_code: ctxt.fi.error_code,
        orig_ax: ctxt.fi.error_code,
        write_cr2: ctxt.fi.vector == X86_TRAP_PF,
        cr2: if ctxt.fi.vector == X86_TRAP_PF {
            ctxt.fi.cr2
        } else {
            0
        },
        action,
    }
}

pub fn vc_early_forward_exception_plan(ctxt: &EsEmCtxt) -> VcEarlyForwardExceptionPlan {
    VcEarlyForwardExceptionPlan {
        trapnr: ctxt.fi.vector,
        orig_ax: ctxt.fi.error_code,
        write_cr2: ctxt.fi.vector == X86_TRAP_PF,
        cr2: if ctxt.fi.vector == X86_TRAP_PF {
            ctxt.fi.cr2
        } else {
            0
        },
        do_early_exception: true,
    }
}

pub const fn vc_decode_path(user_mode: bool, active_mm_is_efi: bool) -> VcDecodePath {
    if user_mode || active_mm_is_efi {
        VcDecodePath::User
    } else {
        VcDecodePath::Kernel
    }
}

pub fn vc_decode_user_insn_plan(
    ctxt: &mut EsEmCtxt,
    insn_bytes: i32,
    decode_ok: bool,
    immediate_got: bool,
) -> EsResult {
    if insn_bytes == 0 {
        ctxt.fi.vector = X86_TRAP_PF;
        ctxt.fi.error_code = X86_PF_INSTR | X86_PF_USER;
        ctxt.fi.cr2 = ctxt.regs.ip;
        return EsResult::Exception;
    }

    if insn_bytes == -EINVAL {
        ctxt.fi.vector = X86_TRAP_GP;
        ctxt.fi.error_code = 0;
        ctxt.fi.cr2 = 0;
        return EsResult::Exception;
    }

    if !decode_ok || !immediate_got {
        return EsResult::DecodeFailed;
    }

    EsResult::Continue
}

pub fn vc_decode_kern_insn_plan(ctxt: &mut EsEmCtxt, fetch_rc: i32, decode_ok: bool) -> EsResult {
    if fetch_rc != 0 {
        ctxt.fi.vector = X86_TRAP_PF;
        ctxt.fi.error_code = X86_PF_INSTR;
        ctxt.fi.cr2 = ctxt.regs.ip;
        return EsResult::Exception;
    }

    if !decode_ok {
        return EsResult::DecodeFailed;
    }

    EsResult::Continue
}

pub fn vc_decode_insn_plan(
    ctxt: &mut EsEmCtxt,
    active_mm_is_efi: bool,
    user_insn_bytes: i32,
    kernel_fetch_rc: i32,
    decode_ok: bool,
    immediate_got: bool,
) -> EsResult {
    match vc_decode_path(ctxt.regs.user_mode, active_mm_is_efi) {
        VcDecodePath::User => {
            vc_decode_user_insn_plan(ctxt, user_insn_bytes, decode_ok, immediate_got)
        }
        VcDecodePath::Kernel => vc_decode_kern_insn_plan(ctxt, kernel_fetch_rc, decode_ok),
    }
}

pub fn vc_slow_virt_to_phys_with<O: VcGhcbOps>(
    ops: &mut O,
    ctxt: &mut EsEmCtxt,
    vaddr: u64,
) -> Result<u64, EsResult> {
    let Some(mapping) = ops.lookup_address(vaddr) else {
        ctxt.fi.vector = X86_TRAP_PF;
        ctxt.fi.cr2 = vaddr;
        ctxt.fi.error_code = 0;
        if ctxt.regs.user_mode {
            ctxt.fi.error_code |= X86_PF_USER;
        }
        return Err(EsResult::Exception);
    };

    if mapping.encrypted {
        return Err(EsResult::Unsupported);
    }

    Ok((mapping.pfn << PAGE_SHIFT) | (vaddr & !mapping.page_level_mask))
}

fn valid_mem_access_size(size: usize) -> bool {
    matches!(size, 1 | 2 | 4 | 8)
}

fn set_page_fault(ctxt: &mut EsEmCtxt, addr: u64, error_code: u64) {
    ctxt.fi.vector = X86_TRAP_PF;
    ctxt.fi.error_code = error_code;
    ctxt.fi.cr2 = addr;
}

pub fn vc_write_mem_with<O: VcGhcbOps>(
    ops: &mut O,
    ctxt: &mut EsEmCtxt,
    dst: u64,
    buf: &[u8],
) -> EsResult {
    if !valid_mem_access_size(buf.len()) {
        return EsResult::Unsupported;
    }

    if ops.write_mem(dst, buf) {
        return EsResult::Continue;
    }

    let mut error_code = X86_PF_PROT | X86_PF_WRITE;
    if ctxt.regs.user_mode {
        error_code |= X86_PF_USER;
    }
    set_page_fault(ctxt, dst, error_code);
    EsResult::Exception
}

pub fn vc_read_mem_with<O: VcGhcbOps>(
    ops: &mut O,
    ctxt: &mut EsEmCtxt,
    src: u64,
    buf: &mut [u8],
) -> EsResult {
    if !valid_mem_access_size(buf.len()) {
        return EsResult::Unsupported;
    }

    if ops.read_mem(src, buf) {
        return EsResult::Continue;
    }

    let mut error_code = X86_PF_PROT;
    if ctxt.regs.user_mode {
        error_code |= X86_PF_USER;
    }
    set_page_fault(ctxt, src, error_code);
    EsResult::Exception
}

pub fn vc_decode_mmio(insn: &EmulatedInsn) -> Result<(VcMmioType, usize), EsResult> {
    if insn.opcode_nbytes == 0 {
        return Err(EsResult::DecodeFailed);
    }

    let mut bytes = 0usize;
    let mmio = match insn.opcode_bytes[0] {
        0x88 => {
            bytes = 1;
            VcMmioType::Write
        }
        0x89 => {
            bytes = insn.opnd_bytes as usize;
            VcMmioType::Write
        }
        0xc6 => {
            bytes = 1;
            VcMmioType::WriteImm
        }
        0xc7 => {
            bytes = insn.opnd_bytes as usize;
            VcMmioType::WriteImm
        }
        0x8a => {
            bytes = 1;
            VcMmioType::Read
        }
        0x8b => {
            bytes = insn.opnd_bytes as usize;
            VcMmioType::Read
        }
        0xa4 => {
            bytes = 1;
            VcMmioType::Movs
        }
        0xa5 => {
            bytes = insn.opnd_bytes as usize;
            VcMmioType::Movs
        }
        0x0f => match insn.opcode_bytes[1] {
            0xb6 => {
                bytes = 1;
                VcMmioType::ReadZeroExtend
            }
            0xb7 => {
                bytes = 2;
                VcMmioType::ReadZeroExtend
            }
            0xbe => {
                bytes = 1;
                VcMmioType::ReadSignExtend
            }
            0xbf => {
                bytes = 2;
                VcMmioType::ReadSignExtend
            }
            _ => return Err(EsResult::DecodeFailed),
        },
        _ => return Err(EsResult::DecodeFailed),
    };

    if !valid_mem_access_size(bytes) {
        return Err(EsResult::Unsupported);
    }

    Ok((mmio, bytes))
}

fn vc_insn_reg_value(ctxt: &EsEmCtxt) -> Option<u64> {
    match (ctxt.insn.modrm >> 3) & 7 {
        0 => Some(ctxt.regs.ax),
        1 => Some(ctxt.regs.cx),
        2 => Some(ctxt.regs.dx),
        3 => Some(ctxt.regs.bx),
        4 => Some(ctxt.regs.sp),
        5 => Some(ctxt.regs.bp),
        6 => Some(ctxt.regs.si),
        7 => Some(ctxt.regs.di),
        _ => None,
    }
}

fn vc_insn_reg_mut(ctxt: &mut EsEmCtxt) -> Option<&mut u64> {
    match (ctxt.insn.modrm >> 3) & 7 {
        0 => Some(&mut ctxt.regs.ax),
        1 => Some(&mut ctxt.regs.cx),
        2 => Some(&mut ctxt.regs.dx),
        3 => Some(&mut ctxt.regs.bx),
        4 => Some(&mut ctxt.regs.sp),
        5 => Some(&mut ctxt.regs.bp),
        6 => Some(&mut ctxt.regs.si),
        7 => Some(&mut ctxt.regs.di),
        _ => None,
    }
}

fn write_low_bytes(reg: &mut u64, bytes: &[u8]) {
    let mut reg_bytes = reg.to_le_bytes();
    reg_bytes[..bytes.len()].copy_from_slice(bytes);
    *reg = u64::from_le_bytes(reg_bytes);
}

fn fill_low_bytes(reg: &mut u64, byte: u8, count: usize) {
    let mut reg_bytes = reg.to_le_bytes();
    reg_bytes[..count].fill(byte);
    *reg = u64::from_le_bytes(reg_bytes);
}

pub fn vc_do_mmio_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    bytes: usize,
    read: bool,
) -> EsResult {
    if !valid_mem_access_size(bytes) {
        return EsResult::Unsupported;
    }

    let Some(vaddr) = ctxt.insn.addr_ref else {
        return EsResult::Unsupported;
    };

    let paddr = match vc_slow_virt_to_phys_with(ops, ctxt, vaddr) {
        Ok(paddr) => paddr,
        Err(ret) => {
            if ret == EsResult::Exception && !read {
                ctxt.fi.error_code |= X86_PF_WRITE;
            }
            return ret;
        }
    };

    let sw_scratch = ops.ghcb_shared_buffer_pa(ghcb);
    ghcb_set_sw_scratch(ghcb, sw_scratch);
    ops.ghcb_hv_call(
        ghcb,
        ctxt,
        if read {
            SVM_VMGEXIT_MMIO_READ
        } else {
            SVM_VMGEXIT_MMIO_WRITE
        },
        paddr,
        bytes as u64,
    )
}

pub fn vc_handle_mmio_movs_with<O: VcGhcbOps>(
    ops: &mut O,
    ctxt: &mut EsEmCtxt,
    bytes: usize,
) -> EsResult {
    if !valid_mem_access_size(bytes) {
        return EsResult::Unsupported;
    }

    let (Some(ds_base), Some(es_base)) = (ctxt.insn.ds_base, ctxt.insn.es_base) else {
        ctxt.fi.vector = X86_TRAP_GP;
        ctxt.fi.error_code = 0;
        return EsResult::Exception;
    };

    let src = ds_base.wrapping_add(ctxt.regs.si);
    let dst = es_base.wrapping_add(ctxt.regs.di);
    let mut buffer = [0u8; 8];

    let ret = vc_read_mem_with(ops, ctxt, src, &mut buffer[..bytes]);
    if ret != EsResult::Continue {
        return ret;
    }

    let ret = vc_write_mem_with(ops, ctxt, dst, &buffer[..bytes]);
    if ret != EsResult::Continue {
        return ret;
    }

    if ctxt.regs.flags & X86_EFLAGS_DF != 0 {
        ctxt.regs.si = ctxt.regs.si.wrapping_sub(bytes as u64);
        ctxt.regs.di = ctxt.regs.di.wrapping_sub(bytes as u64);
    } else {
        ctxt.regs.si = ctxt.regs.si.wrapping_add(bytes as u64);
        ctxt.regs.di = ctxt.regs.di.wrapping_add(bytes as u64);
    }

    if ctxt.insn.rep_prefix {
        ctxt.regs.cx = ctxt.regs.cx.wrapping_sub(1);
    }

    if !ctxt.insn.rep_prefix || ctxt.regs.cx == 0 {
        EsResult::Continue
    } else {
        EsResult::Retry
    }
}

pub fn vc_handle_mmio_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    let (mmio, bytes) = match vc_decode_mmio(&ctxt.insn) {
        Ok(decoded) => decoded,
        Err(ret) => return ret,
    };

    if !matches!(mmio, VcMmioType::WriteImm | VcMmioType::Movs) && vc_insn_reg_value(ctxt).is_none()
    {
        return EsResult::DecodeFailed;
    }

    if ctxt.regs.user_mode {
        return EsResult::Unsupported;
    }

    match mmio {
        VcMmioType::Write => {
            let Some(reg_data) = vc_insn_reg_value(ctxt) else {
                return EsResult::DecodeFailed;
            };
            ghcb.shared_buffer[..bytes].copy_from_slice(&reg_data.to_le_bytes()[..bytes]);
            vc_do_mmio_with(ops, ghcb, ctxt, bytes, false)
        }
        VcMmioType::WriteImm => {
            ghcb.shared_buffer[..bytes]
                .copy_from_slice(&ctxt.insn.immediate.to_le_bytes()[..bytes]);
            vc_do_mmio_with(ops, ghcb, ctxt, bytes, false)
        }
        VcMmioType::Read => {
            let ret = vc_do_mmio_with(ops, ghcb, ctxt, bytes, true);
            if ret != EsResult::Continue {
                return ret;
            }

            let mut data = [0u8; 8];
            data[..bytes].copy_from_slice(&ghcb.shared_buffer[..bytes]);
            let Some(reg_data) = vc_insn_reg_mut(ctxt) else {
                return EsResult::DecodeFailed;
            };
            if bytes == 4 {
                *reg_data = 0;
            }
            write_low_bytes(reg_data, &data[..bytes]);
            EsResult::Continue
        }
        VcMmioType::ReadZeroExtend => {
            let ret = vc_do_mmio_with(ops, ghcb, ctxt, bytes, true);
            if ret != EsResult::Continue {
                return ret;
            }

            let opnd_bytes = ctxt.insn.opnd_bytes as usize;
            if opnd_bytes > 8 {
                return EsResult::Unsupported;
            }
            let mut data = [0u8; 8];
            data[..bytes].copy_from_slice(&ghcb.shared_buffer[..bytes]);
            let Some(reg_data) = vc_insn_reg_mut(ctxt) else {
                return EsResult::DecodeFailed;
            };
            fill_low_bytes(reg_data, 0, opnd_bytes);
            write_low_bytes(reg_data, &data[..bytes]);
            EsResult::Continue
        }
        VcMmioType::ReadSignExtend => {
            let ret = vc_do_mmio_with(ops, ghcb, ctxt, bytes, true);
            if ret != EsResult::Continue {
                return ret;
            }

            let opnd_bytes = ctxt.insn.opnd_bytes as usize;
            if opnd_bytes > 8 {
                return EsResult::Unsupported;
            }
            let mut data = [0u8; 8];
            data[..bytes].copy_from_slice(&ghcb.shared_buffer[..bytes]);
            let sign_byte = if bytes == 1 {
                if data[0] & 0x80 != 0 { 0xff } else { 0x00 }
            } else if u16::from_le_bytes([data[0], data[1]]) & 0x8000 != 0 {
                0xff
            } else {
                0x00
            };
            let Some(reg_data) = vc_insn_reg_mut(ctxt) else {
                return EsResult::DecodeFailed;
            };
            fill_low_bytes(reg_data, sign_byte, opnd_bytes);
            write_low_bytes(reg_data, &data[..bytes]);
            EsResult::Continue
        }
        VcMmioType::Movs => vc_handle_mmio_movs_with(ops, ctxt, bytes),
    }
}

pub fn vc_handle_mmio(ghcb: &mut Ghcb, ctxt: &mut EsEmCtxt) -> EsResult {
    let mut ops = DefaultVcGhcbOps;
    vc_handle_mmio_with(&mut ops, ghcb, ctxt)
}

pub fn vc_ioio_check_with<O: VcGhcbOps>(
    ops: &mut O,
    ctxt: &mut EsEmCtxt,
    port: u16,
    size: usize,
) -> EsResult {
    assert!(size <= 4);

    if ctxt.regs.user_mode {
        let mut offset = 0usize;
        while offset < size {
            let port = port as u32 + offset as u32;
            match ops.io_bitmap_allows(port) {
                Some(true) => {}
                Some(false) | None => {
                    ctxt.fi.vector = X86_TRAP_GP;
                    ctxt.fi.error_code = 0;
                    return EsResult::Exception;
                }
            }
            offset += 1;
        }
    }

    EsResult::Continue
}

pub fn vc_handle_ioio_checked_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    let mut adapter = VcIoioAdapter(ops);
    vc_handle_ioio_with(&mut adapter, ghcb, ctxt)
}

const fn lower_32_bits(value: u64) -> u64 {
    value & 0xffff_ffff
}

const fn upper_32_bits(value: u64) -> u64 {
    value >> 32
}

pub const fn vc_insn_rm_reg_index(modrm: u8) -> Option<usize> {
    if modrm & 0xc0 != 0xc0 {
        return None;
    }

    Some((modrm & 7) as usize)
}

fn vc_insn_rm_reg_mut(ctxt: &mut EsEmCtxt) -> Option<&mut u64> {
    match vc_insn_rm_reg_index(ctxt.insn.modrm)? {
        0 => Some(&mut ctxt.regs.ax),
        1 => Some(&mut ctxt.regs.cx),
        2 => Some(&mut ctxt.regs.dx),
        3 => Some(&mut ctxt.regs.bx),
        4 => Some(&mut ctxt.regs.sp),
        5 => Some(&mut ctxt.regs.bp),
        6 => Some(&mut ctxt.regs.si),
        7 => Some(&mut ctxt.regs.di),
        _ => None,
    }
}

pub fn vc_handle_msr_caa(ctxt: &mut EsEmCtxt, write: bool, svsm_caa_pa: u64) -> EsResult {
    if write {
        return EsResult::Continue;
    }

    ctxt.regs.ax = lower_32_bits(svsm_caa_pa);
    ctxt.regs.dx = upper_32_bits(svsm_caa_pa);
    EsResult::Continue
}

pub fn vc_handle_secure_tsc_msr(ctxt: &mut EsEmCtxt, write: bool, tsc_now: u64) -> EsResult {
    if write {
        ctxt.fi.vector = X86_TRAP_GP;
        ctxt.fi.error_code = 0;
        return EsResult::Exception;
    }

    if ctxt.regs.cx == MSR_AMD64_GUEST_TSC_FREQ as u64 {
        return EsResult::VmmError;
    }

    ctxt.regs.ax = lower_32_bits(tsc_now);
    ctxt.regs.dx = upper_32_bits(tsc_now);
    EsResult::Continue
}

pub fn vc_handle_msr_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    write: bool,
    state: &VcHandlerState,
) -> EsResult {
    match ctxt.regs.cx {
        cx if cx == MSR_SVSM_CAA as u64 => {
            return vc_handle_msr_caa(ctxt, write, state.svsm_caa_pa);
        }
        cx if (cx == MSR_IA32_TSC as u64 || cx == MSR_AMD64_GUEST_TSC_FREQ as u64)
            && state.sev_status & MSR_AMD64_SNP_SECURE_TSC != 0 =>
        {
            return vc_handle_secure_tsc_msr(ctxt, write, state.secure_tsc_now);
        }
        cx if cx == MSR_AMD64_SAVIC_CONTROL as u64 && state.secure_avic => {
            return EsResult::VmmError;
        }
        _ => {}
    }

    ghcb_set_rcx(ghcb, ctxt.regs.cx);
    if write {
        ghcb_set_rax(ghcb, ctxt.regs.ax);
        ghcb_set_rdx(ghcb, ctxt.regs.dx);
    }

    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_MSR, write as u64, 0);
    if ret == EsResult::Continue && !write {
        ctxt.regs.ax = ghcb.save.rax;
        ctxt.regs.dx = ghcb.save.rdx;
    }
    ret
}

pub fn vc_handle_msr(ghcb: &mut Ghcb, ctxt: &mut EsEmCtxt, state: &VcHandlerState) -> EsResult {
    let mut ops = DefaultVcGhcbOps;
    vc_handle_msr_with(
        &mut ops,
        ghcb,
        ctxt,
        ctxt.insn.opcode_bytes[1] == 0x30,
        state,
    )
}

pub fn vc_handle_dr7_write_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    state: &mut VcHandlerState,
) -> EsResult {
    if state.sev_status & MSR_AMD64_SNP_DEBUG_SWAP != 0 {
        return EsResult::VmmError;
    }

    let Some(reg) = vc_insn_rm_reg_mut(ctxt) else {
        return EsResult::DecodeFailed;
    };
    let mut val = *reg;

    if val >> 32 != 0 {
        ctxt.fi.vector = X86_TRAP_GP;
        ctxt.fi.error_code = 0;
        return EsResult::Exception;
    }

    val = (val & DR7_RESERVED_CLEAR_MASK) | DR7_RESET_VALUE;
    if state.runtime_dr7.is_none() && (val & !DR7_RESET_VALUE) != 0 {
        return EsResult::Unsupported;
    }

    ghcb_set_rax(ghcb, val);
    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_WRITE_DR7, 0, 0);
    if ret != EsResult::Continue {
        return ret;
    }

    if let Some(dr7) = &mut state.runtime_dr7 {
        *dr7 = val;
    }
    EsResult::Continue
}

pub fn vc_handle_dr7_read(ctxt: &mut EsEmCtxt, state: &VcHandlerState) -> EsResult {
    if state.sev_status & MSR_AMD64_SNP_DEBUG_SWAP != 0 {
        return EsResult::VmmError;
    }

    let value = state.runtime_dr7.unwrap_or(DR7_RESET_VALUE);
    let Some(reg) = vc_insn_rm_reg_mut(ctxt) else {
        return EsResult::DecodeFailed;
    };
    *reg = value;
    EsResult::Continue
}

pub fn vc_handle_wbinvd_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_WBINVD, 0, 0)
}

pub fn vc_handle_rdpmc_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    ghcb_set_rcx(ghcb, ctxt.regs.cx);

    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_RDPMC, 0, 0);
    if ret != EsResult::Continue {
        return ret;
    }

    if !(ghcb_rax_is_valid(ghcb) && ghcb_rdx_is_valid(ghcb)) {
        return EsResult::VmmError;
    }

    ctxt.regs.ax = ghcb.save.rax;
    ctxt.regs.dx = ghcb.save.rdx;
    EsResult::Continue
}

pub fn vc_handle_rdtsc_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    exit_code: u64,
    secure_tsc: bool,
) -> EsResult {
    if secure_tsc {
        return EsResult::VmmError;
    }

    let rdtscp = exit_code == SVM_EXIT_RDTSCP;
    let ret = ops.ghcb_hv_call(ghcb, ctxt, exit_code, 0, 0);
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

pub const fn vc_handle_monitor() -> EsResult {
    EsResult::Continue
}

pub const fn vc_handle_mwait() -> EsResult {
    EsResult::Continue
}

pub fn vc_handle_vmmcall_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
) -> EsResult {
    ghcb_set_rax(ghcb, ctxt.regs.ax);
    ghcb_set_cpl(ghcb, if ctxt.regs.user_mode { 3 } else { 0 });

    let mut regs = ctxt.regs;
    ops.vmmcall_prepare(ghcb, &mut regs);
    ctxt.regs = regs;

    let ret = ops.ghcb_hv_call(ghcb, ctxt, SVM_EXIT_VMMCALL, 0, 0);
    if ret != EsResult::Continue {
        return ret;
    }
    if !ghcb_rax_is_valid(ghcb) {
        return EsResult::VmmError;
    }

    ctxt.regs.ax = ghcb.save.rax;
    let mut regs = ctxt.regs;
    if !ops.vmmcall_finish(ghcb, &mut regs) {
        return EsResult::VmmError;
    }
    ctxt.regs = regs;
    EsResult::Continue
}

pub fn vc_handle_trap_ac(ctxt: &mut EsEmCtxt) -> EsResult {
    ctxt.fi.vector = X86_TRAP_AC;
    ctxt.fi.error_code = 0;
    EsResult::Exception
}

pub fn vc_handle_exitcode_result_with<O: VcGhcbOps>(
    ops: &mut O,
    ghcb: &mut Ghcb,
    ctxt: &mut EsEmCtxt,
    state: &mut VcHandlerState,
    exit_code: u64,
) -> EsResult {
    let mut result = vc_check_opcode_bytes(ctxt, exit_code);
    if result != EsResult::Continue {
        return result;
    }

    result = match exit_code {
        SVM_EXIT_READ_DR7 => vc_handle_dr7_read(ctxt, state),
        SVM_EXIT_WRITE_DR7 => vc_handle_dr7_write_with(ops, ghcb, ctxt, state),
        code if code == SVM_EXIT_EXCP_BASE + X86_TRAP_AC => vc_handle_trap_ac(ctxt),
        SVM_EXIT_RDTSC | SVM_EXIT_RDTSCP => vc_handle_rdtsc_with(
            ops,
            ghcb,
            ctxt,
            exit_code,
            state.sev_status & MSR_AMD64_SNP_SECURE_TSC != 0,
        ),
        SVM_EXIT_RDPMC => vc_handle_rdpmc_with(ops, ghcb, ctxt),
        SVM_EXIT_INVD => EsResult::Unsupported,
        SVM_EXIT_CPUID => {
            let mut adapter = VcCpuidAdapter(ops);
            vc_handle_cpuid_with(
                &mut adapter,
                ghcb,
                ctxt,
                state.sev_status & MSR_AMD64_SEV_SNP_ENABLED != 0,
            )
        }
        SVM_EXIT_IOIO => vc_handle_ioio_checked_with(ops, ghcb, ctxt),
        SVM_EXIT_MSR => {
            vc_handle_msr_with(ops, ghcb, ctxt, ctxt.insn.opcode_bytes[1] == 0x30, state)
        }
        SVM_EXIT_VMMCALL => vc_handle_vmmcall_with(ops, ghcb, ctxt),
        SVM_EXIT_WBINVD => vc_handle_wbinvd_with(ops, ghcb, ctxt),
        SVM_EXIT_MONITOR => vc_handle_monitor(),
        SVM_EXIT_MWAIT => vc_handle_mwait(),
        SVM_EXIT_NPF => vc_handle_mmio_with(ops, ghcb, ctxt),
        _ => EsResult::Unsupported,
    };

    result
}

pub const fn is_vc2_stack(sp: u64, bounds: VcStackBounds) -> bool {
    sp >= bounds.bottom && sp < bounds.top
}

pub const fn vc_from_invalid_context(
    regs_addr: u64,
    previous_sp: u64,
    bounds: VcStackBounds,
) -> bool {
    is_vc2_stack(regs_addr, bounds) && !is_vc2_stack(previous_sp, bounds)
}

pub const fn vc_is_db(error_code: u64) -> bool {
    error_code == SVM_EXIT_EXCP_BASE + X86_TRAP_DB as u64
}

pub const fn vc_kernel_entry_action(
    regs_addr: u64,
    previous_sp: u64,
    error_code: u64,
    bounds: VcStackBounds,
) -> VcKernelEntryAction {
    if vc_from_invalid_context(regs_addr, previous_sp, bounds) {
        return VcKernelEntryAction::PanicInvalidContext;
    }

    if vc_is_db(error_code) {
        return VcKernelEntryAction::DebugException;
    }

    VcKernelEntryAction::RawHandle
}

pub const fn vc_user_entry_action(error_code: u64) -> VcUserEntryAction {
    if vc_is_db(error_code) {
        return VcUserEntryAction::DebugException;
    }

    VcUserEntryAction::RawHandle
}

pub const fn vc_raw_handle_result_action(result: EsResult) -> VcRawHandleResultAction {
    match result {
        EsResult::Continue => VcRawHandleResultAction::FinishInsn,
        EsResult::Unsupported => VcRawHandleResultAction::FailUnsupported,
        EsResult::VmmError => VcRawHandleResultAction::FailVmmError,
        EsResult::DecodeFailed => VcRawHandleResultAction::FailDecodeFailed,
        EsResult::Exception => VcRawHandleResultAction::ForwardException,
        EsResult::Retry => VcRawHandleResultAction::Retry,
        EsResult::Terminate => VcRawHandleResultAction::Bug,
    }
}

pub const fn vc_raw_handle_result_success(result: EsResult) -> bool {
    !matches!(
        vc_raw_handle_result_action(result),
        VcRawHandleResultAction::FailUnsupported
            | VcRawHandleResultAction::FailVmmError
            | VcRawHandleResultAction::FailDecodeFailed
            | VcRawHandleResultAction::Bug
    )
}

pub const fn vc_boot_ghcb_result_action(result: EsResult) -> VcBootGhcbAction {
    match result {
        EsResult::Continue => VcBootGhcbAction::FinishInsn,
        EsResult::Unsupported | EsResult::VmmError | EsResult::DecodeFailed => {
            VcBootGhcbAction::FailTerminate
        }
        EsResult::Exception => VcBootGhcbAction::ForwardException,
        EsResult::Retry => VcBootGhcbAction::Retry,
        EsResult::Terminate => VcBootGhcbAction::Bug,
    }
}

pub const fn vc_boot_ghcb_result_success(result: EsResult) -> bool {
    !matches!(
        vc_boot_ghcb_result_action(result),
        VcBootGhcbAction::FailTerminate | VcBootGhcbAction::Bug
    )
}

pub const fn vc_boot_ghcb_failure_termination(action: VcBootGhcbAction) -> Option<SevTermination> {
    match action {
        VcBootGhcbAction::FailTerminate => Some(SevTermination {
            set: SEV_TERM_SET_GEN,
            reason: GHCB_SEV_ES_GEN_REQ,
        }),
        _ => None,
    }
}

pub const fn handle_vc_boot_ghcb_plan(
    exit_code: u64,
    init_result: EsResult,
    handle_result: EsResult,
) -> VcBootGhcbPlan {
    let (handle_exitcode, result) = match init_result {
        EsResult::Continue => (true, handle_result),
        _ => (false, init_result),
    };
    let action = vc_boot_ghcb_result_action(result);

    VcBootGhcbPlan {
        exit_code,
        invalidate_boot_ghcb: true,
        init_result,
        handle_exitcode,
        result,
        action,
        success: vc_boot_ghcb_result_success(result),
        termination: vc_boot_ghcb_failure_termination(action),
    }
}

pub fn vc_raw_handle_exception(regs: &mut VcRegs, ctxt: &mut EsEmCtxt) -> Result<VcAction, i32> {
    let action = vc_handle_exitcode(ctxt.exit_code, ctxt.exit_info_1 & 1 != 0);
    match action {
        VcAction::Unsupported => Err(EOPNOTSUPP),
        VcAction::ForwardException => Ok(action),
        _ => {
            ctxt.regs.ip = regs.ip;
            vc_finish_insn(ctxt);
            regs.ip = ctxt.regs.ip;
            Ok(action)
        }
    }
}

pub fn handle_vc_boot_ghcb(exit_code: u64) -> bool {
    let handle_result = match vc_handle_exitcode(exit_code, false) {
        VcAction::Unsupported => EsResult::Unsupported,
        _ => EsResult::Continue,
    };

    handle_vc_boot_ghcb_plan(exit_code, EsResult::Continue, handle_result).success
}

pub fn vc_result_to_errno(result: EsResult) -> Result<(), i32> {
    match result {
        EsResult::Continue | EsResult::Retry => Ok(()),
        EsResult::Exception
        | EsResult::VmmError
        | EsResult::DecodeFailed
        | EsResult::Terminate
        | EsResult::Unsupported => Err(EOPNOTSUPP),
    }
}

#[cfg(test)]
mod tests {
    use super::super::vc_shared::FaultInfo;
    use super::*;

    struct FakeOps {
        exit_code: u64,
        exit_info_1: u64,
        exit_info_2: u64,
        call_count: usize,
        status: EsResult,
        out_rax: Option<u64>,
        out_rbx: Option<u64>,
        out_rcx: Option<u64>,
        out_rdx: Option<u64>,
        prepared: bool,
        finish_ok: bool,
        io_allowed: Option<bool>,
        mapping: Option<VcPhysMapping>,
        read_addr: u64,
        read_data: [u8; 8],
        read_ok: bool,
        write_addr: u64,
        write_data: [u8; 8],
        write_len: usize,
        write_ok: bool,
        shared_buffer_pa: u64,
    }

    impl Default for FakeOps {
        fn default() -> Self {
            Self {
                exit_code: 0,
                exit_info_1: 0,
                exit_info_2: 0,
                call_count: 0,
                status: EsResult::Continue,
                out_rax: None,
                out_rbx: None,
                out_rcx: None,
                out_rdx: None,
                prepared: false,
                finish_ok: false,
                io_allowed: None,
                mapping: None,
                read_addr: 0,
                read_data: [0; 8],
                read_ok: false,
                write_addr: 0,
                write_data: [0; 8],
                write_len: 0,
                write_ok: false,
                shared_buffer_pa: 0x8000,
            }
        }
    }

    impl VcGhcbOps for FakeOps {
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
            if let Some(rbx) = self.out_rbx {
                ghcb_set_rbx(ghcb, rbx);
            }
            if let Some(rcx) = self.out_rcx {
                ghcb_set_rcx(ghcb, rcx);
            }
            if let Some(rdx) = self.out_rdx {
                ghcb_set_rdx(ghcb, rdx);
            }
            if exit_code == SVM_VMGEXIT_MMIO_READ {
                let len = exit_info_2 as usize;
                ghcb.shared_buffer[..len].copy_from_slice(&self.read_data[..len]);
            }
            self.status
        }

        fn vmmcall_prepare(&mut self, ghcb: &mut Ghcb, regs: &mut VcRegs) {
            self.prepared = true;
            ghcb_set_rdx(ghcb, regs.dx);
        }

        fn vmmcall_finish(&mut self, _ghcb: &mut Ghcb, regs: &mut VcRegs) -> bool {
            regs.dx = 0xfeed;
            self.finish_ok
        }

        fn io_bitmap_allows(&mut self, _port: u32) -> Option<bool> {
            self.io_allowed
        }

        fn lookup_address(&mut self, _vaddr: u64) -> Option<VcPhysMapping> {
            self.mapping
        }

        fn read_mem(&mut self, addr: u64, buf: &mut [u8]) -> bool {
            self.read_addr = addr;
            if !self.read_ok {
                return false;
            }
            buf.copy_from_slice(&self.read_data[..buf.len()]);
            true
        }

        fn write_mem(&mut self, addr: u64, buf: &[u8]) -> bool {
            self.write_addr = addr;
            if !self.write_ok {
                return false;
            }
            self.write_len = buf.len();
            self.write_data[..buf.len()].copy_from_slice(buf);
            true
        }

        fn ghcb_shared_buffer_pa(&mut self, _ghcb: &Ghcb) -> u64 {
            self.shared_buffer_pa
        }
    }

    impl FakeOps {
        fn ok() -> Self {
            Self {
                status: EsResult::Continue,
                finish_ok: true,
                read_ok: true,
                write_ok: true,
                ..Default::default()
            }
        }
    }

    #[test]
    fn exitcode_dispatch_matches_linux_vc_handlers() {
        assert_eq!(vc_handle_exitcode(SVM_EXIT_CPUID, false), VcAction::Cpuid);
        assert_eq!(vc_handle_exitcode(SVM_EXIT_MSR, false), VcAction::MsrRead);
        assert_eq!(vc_handle_exitcode(SVM_EXIT_MSR, true), VcAction::MsrWrite);
        assert_eq!(
            vc_handle_exitcode(SVM_EXIT_READ_DR7, false),
            VcAction::ReadDr7
        );
        assert_eq!(
            vc_handle_exitcode(SVM_EXIT_EXCP_BASE + X86_TRAP_AC, false),
            VcAction::TrapAc
        );
        assert_eq!(vc_handle_exitcode(SVM_EXIT_RDTSC, false), VcAction::Rdtsc);
        assert_eq!(vc_handle_exitcode(SVM_EXIT_RDTSCP, false), VcAction::Rdtsc);
        assert_eq!(
            vc_handle_exitcode(SVM_EXIT_VMMCALL, false),
            VcAction::VmmCall
        );
        assert_eq!(vc_handle_exitcode(SVM_EXIT_RDPMC, false), VcAction::Rdpmc);
        assert_eq!(vc_handle_exitcode(SVM_EXIT_WBINVD, false), VcAction::Wbinvd);
        assert_eq!(
            vc_handle_exitcode(SVM_EXIT_MONITOR, false),
            VcAction::Monitor
        );
        assert_eq!(vc_handle_exitcode(SVM_EXIT_MWAIT, false), VcAction::Mwait);
        assert_eq!(
            vc_handle_exitcode(SVM_EXIT_NPF, false),
            VcAction::NestedPageFaultMmio
        );
        assert_eq!(
            vc_handle_exitcode(SVM_VMGEXIT_MMIO_READ, false),
            VcAction::Unsupported
        );
        assert_eq!(
            vc_handle_exitcode(SVM_VMGEXIT_MMIO_WRITE, false),
            VcAction::Unsupported
        );
        assert_eq!(
            vc_handle_exitcode(SVM_EXIT_INVD, false),
            VcAction::Unsupported
        );
        assert_eq!(
            vc_handle_exitcode(SVM_VMGEXIT_UNSUPPORTED_EVENT, false),
            VcAction::Unsupported
        );
        assert_eq!(vc_handle_exitcode(0xdead, false), VcAction::Unsupported);
    }

    #[test]
    fn exitcode_dispatch_invalidates_linux_ghcb_fields() {
        let mut ghcb = Ghcb::default();
        ghcb.save.sw_exit_code = 0x9999;
        ghcb.save.sw_exit_info_1 = 0xaaaa;
        ghcb.save.sw_exit_info_2 = 0xbbbb;
        ghcb_set_rax(&mut ghcb, 0x1234);
        assert!(ghcb_rax_is_valid(&ghcb));
        assert_eq!(
            vc_handle_exitcode_with_ghcb(&mut ghcb, SVM_EXIT_CPUID, false),
            VcAction::Cpuid
        );
        assert_eq!(ghcb.save.sw_exit_code, 0);
        assert_eq!(ghcb.save.sw_exit_info_1, 0xaaaa);
        assert_eq!(ghcb.save.sw_exit_info_2, 0xbbbb);
        assert_eq!(ghcb.save.rax, 0x1234);
        assert!(!ghcb_rax_is_valid(&ghcb));
    }

    #[test]
    fn runtime_exception_forwarding_matches_linux_switch_and_cr2_write() {
        let mut ctxt = EsEmCtxt {
            fi: FaultInfo {
                vector: X86_TRAP_GP,
                error_code: 0x55,
                cr2: 0xdead,
            },
            ..Default::default()
        };
        assert_eq!(
            vc_forward_exception(&ctxt),
            VcForwardExceptionPlan {
                trapnr: X86_TRAP_GP,
                error_code: 0x55,
                orig_ax: 0x55,
                write_cr2: false,
                cr2: 0,
                action: VcForwardExceptionAction::GeneralProtection,
            }
        );

        ctxt.fi.vector = X86_TRAP_UD;
        assert_eq!(
            vc_forward_exception(&ctxt).action,
            VcForwardExceptionAction::InvalidOpcode
        );

        ctxt.fi.vector = X86_TRAP_PF;
        assert_eq!(
            vc_forward_exception(&ctxt),
            VcForwardExceptionPlan {
                trapnr: X86_TRAP_PF,
                error_code: 0x55,
                orig_ax: 0x55,
                write_cr2: true,
                cr2: 0xdead,
                action: VcForwardExceptionAction::PageFault,
            }
        );

        ctxt.fi.vector = X86_TRAP_AC;
        assert_eq!(
            vc_forward_exception(&ctxt).action,
            VcForwardExceptionAction::AlignmentCheck
        );

        ctxt.fi.vector = X86_TRAP_VC as u64;
        assert_eq!(
            vc_forward_exception(&ctxt).action,
            VcForwardExceptionAction::Bug
        );
    }

    #[test]
    fn early_exception_forwarding_sets_orig_ax_and_only_writes_cr2_for_pf() {
        let mut ctxt = EsEmCtxt {
            fi: FaultInfo {
                vector: X86_TRAP_PF,
                error_code: X86_PF_INSTR | X86_PF_USER,
                cr2: 0x401000,
            },
            ..Default::default()
        };

        assert_eq!(
            vc_early_forward_exception_plan(&ctxt),
            VcEarlyForwardExceptionPlan {
                trapnr: X86_TRAP_PF,
                orig_ax: X86_PF_INSTR | X86_PF_USER,
                write_cr2: true,
                cr2: 0x401000,
                do_early_exception: true,
            }
        );

        ctxt.fi.vector = X86_TRAP_GP;
        ctxt.fi.error_code = 0;
        assert_eq!(
            vc_early_forward_exception_plan(&ctxt),
            VcEarlyForwardExceptionPlan {
                trapnr: X86_TRAP_GP,
                orig_ax: 0,
                write_cr2: false,
                cr2: 0,
                do_early_exception: true,
            }
        );
    }

    #[test]
    fn runtime_vc_context_checks_match_linux_vc2_stack_and_db_rules() {
        let bounds = VcStackBounds {
            bottom: 0x8000,
            top: 0x9000,
        };

        assert!(is_vc2_stack(0x8000, bounds));
        assert!(is_vc2_stack(0x8fff, bounds));
        assert!(!is_vc2_stack(0x7fff, bounds));
        assert!(!is_vc2_stack(0x9000, bounds));

        assert!(vc_from_invalid_context(0x8100, 0x7000, bounds));
        assert!(!vc_from_invalid_context(0x8100, 0x8200, bounds));
        assert!(!vc_from_invalid_context(0x7000, 0x8100, bounds));

        assert!(vc_is_db(SVM_EXIT_EXCP_BASE + X86_TRAP_DB as u64));
        assert!(!vc_is_db(SVM_EXIT_EXCP_BASE + X86_TRAP_GP));
    }

    #[test]
    fn decode_path_selects_user_for_user_mode_or_efi_like_linux() {
        assert_eq!(vc_decode_path(true, false), VcDecodePath::User);
        assert_eq!(vc_decode_path(false, true), VcDecodePath::User);
        assert_eq!(vc_decode_path(false, false), VcDecodePath::Kernel);
    }

    #[test]
    fn user_instruction_decode_faults_and_immediate_requirement_match_linux() {
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                ip: 0x401000,
                user_mode: true,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_decode_user_insn_plan(&mut ctxt, 0, true, true),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_PF);
        assert_eq!(ctxt.fi.error_code, X86_PF_INSTR | X86_PF_USER);
        assert_eq!(ctxt.fi.cr2, 0x401000);

        ctxt.fi = Default::default();
        assert_eq!(
            vc_decode_user_insn_plan(&mut ctxt, -EINVAL, true, true),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);
        assert_eq!(ctxt.fi.error_code, 0);
        assert_eq!(ctxt.fi.cr2, 0);

        assert_eq!(
            vc_decode_user_insn_plan(&mut ctxt, 4, false, true),
            EsResult::DecodeFailed
        );
        assert_eq!(
            vc_decode_user_insn_plan(&mut ctxt, 4, true, false),
            EsResult::DecodeFailed
        );
        assert_eq!(
            vc_decode_user_insn_plan(&mut ctxt, 4, true, true),
            EsResult::Continue
        );
    }

    #[test]
    fn kernel_instruction_decode_faults_without_user_bit_like_linux() {
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                ip: 0xffff_8000_0010_0000,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_decode_kern_insn_plan(&mut ctxt, -1, true),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_PF);
        assert_eq!(ctxt.fi.error_code, X86_PF_INSTR);
        assert_eq!(ctxt.fi.cr2, 0xffff_8000_0010_0000);

        assert_eq!(
            vc_decode_kern_insn_plan(&mut ctxt, 0, false),
            EsResult::DecodeFailed
        );
        assert_eq!(
            vc_decode_kern_insn_plan(&mut ctxt, 0, true),
            EsResult::Continue
        );
    }

    #[test]
    fn decode_insn_plan_routes_efi_kernel_context_through_user_decoder() {
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                ip: 0xfeed,
                user_mode: false,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_decode_insn_plan(&mut ctxt, true, 0, 0, true, true),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.error_code, X86_PF_INSTR | X86_PF_USER);

        ctxt.fi = Default::default();
        assert_eq!(
            vc_decode_insn_plan(&mut ctxt, false, 0, -1, true, true),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.error_code, X86_PF_INSTR);
    }

    #[test]
    fn runtime_vc_entry_actions_preserve_linux_kernel_user_ordering() {
        let bounds = VcStackBounds {
            bottom: 0x8000,
            top: 0x9000,
        };
        let db_exit = SVM_EXIT_EXCP_BASE + X86_TRAP_DB as u64;

        assert_eq!(
            vc_kernel_entry_action(0x8100, 0x7000, db_exit, bounds),
            VcKernelEntryAction::PanicInvalidContext
        );
        assert_eq!(
            vc_kernel_entry_action(0x8100, 0x8200, db_exit, bounds),
            VcKernelEntryAction::DebugException
        );
        assert_eq!(
            vc_kernel_entry_action(0x8100, 0x8200, SVM_EXIT_IOIO, bounds),
            VcKernelEntryAction::RawHandle
        );
        assert_eq!(
            vc_user_entry_action(db_exit),
            VcUserEntryAction::DebugException
        );
        assert_eq!(
            vc_user_entry_action(SVM_EXIT_IOIO),
            VcUserEntryAction::RawHandle
        );
    }

    #[test]
    fn boot_ghcb_result_actions_match_linux_switch_arms() {
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::Continue),
            VcBootGhcbAction::FinishInsn
        );
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::Unsupported),
            VcBootGhcbAction::FailTerminate
        );
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::VmmError),
            VcBootGhcbAction::FailTerminate
        );
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::DecodeFailed),
            VcBootGhcbAction::FailTerminate
        );
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::Exception),
            VcBootGhcbAction::ForwardException
        );
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::Retry),
            VcBootGhcbAction::Retry
        );
        assert_eq!(
            vc_boot_ghcb_result_action(EsResult::Terminate),
            VcBootGhcbAction::Bug
        );

        assert!(vc_boot_ghcb_result_success(EsResult::Continue));
        assert!(vc_boot_ghcb_result_success(EsResult::Exception));
        assert!(vc_boot_ghcb_result_success(EsResult::Retry));
        assert!(!vc_boot_ghcb_result_success(EsResult::Unsupported));
        assert!(!vc_boot_ghcb_result_success(EsResult::VmmError));
        assert!(!vc_boot_ghcb_result_success(EsResult::DecodeFailed));
        assert!(!vc_boot_ghcb_result_success(EsResult::Terminate));
    }

    #[test]
    fn boot_ghcb_plan_matches_linux_init_then_handle_sequence() {
        assert_eq!(
            handle_vc_boot_ghcb_plan(SVM_EXIT_CPUID, EsResult::Continue, EsResult::Continue),
            VcBootGhcbPlan {
                exit_code: SVM_EXIT_CPUID,
                invalidate_boot_ghcb: true,
                init_result: EsResult::Continue,
                handle_exitcode: true,
                result: EsResult::Continue,
                action: VcBootGhcbAction::FinishInsn,
                success: true,
                termination: None,
            }
        );

        assert_eq!(
            handle_vc_boot_ghcb_plan(SVM_EXIT_CPUID, EsResult::DecodeFailed, EsResult::Continue),
            VcBootGhcbPlan {
                exit_code: SVM_EXIT_CPUID,
                invalidate_boot_ghcb: true,
                init_result: EsResult::DecodeFailed,
                handle_exitcode: false,
                result: EsResult::DecodeFailed,
                action: VcBootGhcbAction::FailTerminate,
                success: false,
                termination: Some(SevTermination {
                    set: SEV_TERM_SET_GEN,
                    reason: GHCB_SEV_ES_GEN_REQ,
                }),
            }
        );
    }

    #[test]
    fn boot_ghcb_wrapper_accepts_any_supported_exit_like_linux_dispatch() {
        assert!(handle_vc_boot_ghcb(SVM_EXIT_CPUID));
        assert!(handle_vc_boot_ghcb(SVM_EXIT_READ_DR7));
        assert!(handle_vc_boot_ghcb(SVM_EXIT_RDTSCP));
        assert!(!handle_vc_boot_ghcb(0xdead));
    }

    #[test]
    fn raw_handle_result_actions_match_linux_runtime_switch_arms() {
        assert_eq!(
            vc_raw_handle_result_action(EsResult::Continue),
            VcRawHandleResultAction::FinishInsn
        );
        assert_eq!(
            vc_raw_handle_result_action(EsResult::Unsupported),
            VcRawHandleResultAction::FailUnsupported
        );
        assert_eq!(
            vc_raw_handle_result_action(EsResult::VmmError),
            VcRawHandleResultAction::FailVmmError
        );
        assert_eq!(
            vc_raw_handle_result_action(EsResult::DecodeFailed),
            VcRawHandleResultAction::FailDecodeFailed
        );
        assert_eq!(
            vc_raw_handle_result_action(EsResult::Exception),
            VcRawHandleResultAction::ForwardException
        );
        assert_eq!(
            vc_raw_handle_result_action(EsResult::Retry),
            VcRawHandleResultAction::Retry
        );
        assert_eq!(
            vc_raw_handle_result_action(EsResult::Terminate),
            VcRawHandleResultAction::Bug
        );

        assert!(vc_raw_handle_result_success(EsResult::Continue));
        assert!(vc_raw_handle_result_success(EsResult::Exception));
        assert!(vc_raw_handle_result_success(EsResult::Retry));
        assert!(!vc_raw_handle_result_success(EsResult::Unsupported));
        assert!(!vc_raw_handle_result_success(EsResult::VmmError));
        assert!(!vc_raw_handle_result_success(EsResult::DecodeFailed));
        assert!(!vc_raw_handle_result_success(EsResult::Terminate));
    }

    #[test]
    fn raw_handler_advances_rip_for_handled_exit() {
        let mut regs = VcRegs {
            ip: 0x1000,
            ..Default::default()
        };
        let mut ctxt = EsEmCtxt {
            exit_code: SVM_EXIT_CPUID,
            insn: EmulatedInsn {
                length: 2,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            vc_raw_handle_exception(&mut regs, &mut ctxt),
            Ok(VcAction::Cpuid)
        );
        assert_eq!(regs.ip, 0x1002);
    }

    #[test]
    fn raw_handler_rejects_unexpected_exit_code_like_linux_default() {
        let mut regs = VcRegs {
            ip: 0x1000,
            ..Default::default()
        };
        let mut ctxt = EsEmCtxt {
            exit_code: 0xdead,
            insn: EmulatedInsn {
                length: 2,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_raw_handle_exception(&mut regs, &mut ctxt),
            Err(EOPNOTSUPP)
        );
        assert_eq!(regs.ip, 0x1000);
    }

    #[test]
    fn slow_virt_to_phys_matches_linux_fault_and_encryption_rules() {
        let mut ops = FakeOps::ok();
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                user_mode: true,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_slow_virt_to_phys_with(&mut ops, &mut ctxt, 0xdead_beef),
            Err(EsResult::Exception)
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_PF);
        assert_eq!(ctxt.fi.cr2, 0xdead_beef);
        assert_eq!(ctxt.fi.error_code, X86_PF_USER);

        ops.mapping = Some(VcPhysMapping {
            pfn: 0x12345,
            page_level_mask: 0xffff_ffff_ffff_f000,
            encrypted: true,
        });
        assert_eq!(
            vc_slow_virt_to_phys_with(&mut ops, &mut ctxt, 0x1234_5abc),
            Err(EsResult::Unsupported)
        );

        ops.mapping = Some(VcPhysMapping {
            pfn: 0x12345,
            page_level_mask: 0xffff_ffff_ffff_f000,
            encrypted: false,
        });
        assert_eq!(
            vc_slow_virt_to_phys_with(&mut ops, &mut ctxt, 0x1234_5abc),
            Ok((0x12345 << PAGE_SHIFT) | 0xabc)
        );

        ops.mapping = Some(VcPhysMapping {
            pfn: 0x200,
            page_level_mask: 0xffff_ffff_ffe0_0000,
            encrypted: false,
        });
        assert_eq!(
            vc_slow_virt_to_phys_with(&mut ops, &mut ctxt, 0x2bad_cafe),
            Ok((0x200 << PAGE_SHIFT) | 0xdcafe)
        );
    }

    #[test]
    fn mmio_decode_matches_linux_instruction_table() {
        let mut insn = EmulatedInsn::from_opcode(0, 0x88);
        assert_eq!(vc_decode_mmio(&insn), Ok((VcMmioType::Write, 1)));

        insn = EmulatedInsn::from_opcode(0, 0x89);
        insn.opnd_bytes = 8;
        assert_eq!(vc_decode_mmio(&insn), Ok((VcMmioType::Write, 8)));

        insn = EmulatedInsn::from_opcode(0, 0xc7);
        insn.opnd_bytes = 4;
        assert_eq!(vc_decode_mmio(&insn), Ok((VcMmioType::WriteImm, 4)));

        insn = EmulatedInsn::from_opcode(0, 0x8b);
        insn.opnd_bytes = 2;
        assert_eq!(vc_decode_mmio(&insn), Ok((VcMmioType::Read, 2)));

        insn = EmulatedInsn::from_opcode(0, 0xa5);
        insn.opnd_bytes = 4;
        assert_eq!(vc_decode_mmio(&insn), Ok((VcMmioType::Movs, 4)));

        assert_eq!(
            vc_decode_mmio(&EmulatedInsn::from_opcode(0xb60f, 0x0f)),
            Ok((VcMmioType::ReadZeroExtend, 1))
        );
        assert_eq!(
            vc_decode_mmio(&EmulatedInsn::from_opcode(0xbf0f, 0x0f)),
            Ok((VcMmioType::ReadSignExtend, 2))
        );
        assert_eq!(
            vc_decode_mmio(&EmulatedInsn::from_opcode(0, 0x90)),
            Err(EsResult::DecodeFailed)
        );
    }

    #[test]
    fn mmio_write_uses_linux_ghcb_buffer_and_translated_address() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            mapping: Some(VcPhysMapping {
                pfn: 0x12345,
                page_level_mask: 0xffff_ffff_ffff_f000,
                encrypted: false,
            }),
            ..FakeOps::ok()
        };
        let mut insn = EmulatedInsn::from_opcode(0, 0x89);
        insn.opnd_bytes = 4;
        insn.modrm = 3 << 3;
        insn.addr_ref = Some(0x7fff_fabc);
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                bx: 0x1122_3344_5566_7788,
                ..Default::default()
            },
            insn,
            ..Default::default()
        };

        assert_eq!(
            vc_handle_mmio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_VMGEXIT_MMIO_WRITE);
        assert_eq!(ops.exit_info_1, (0x12345 << PAGE_SHIFT) | 0xabc);
        assert_eq!(ops.exit_info_2, 4);
        assert_eq!(&ghcb.shared_buffer[..4], &[0x88, 0x77, 0x66, 0x55]);
        assert_eq!(ghcb.save.sw_scratch, 0x8000);
    }

    #[test]
    fn mmio_read_zero_and_sign_extension_match_linux_register_writes() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            mapping: Some(VcPhysMapping {
                pfn: 0x222,
                page_level_mask: 0xffff_ffff_ffff_f000,
                encrypted: false,
            }),
            read_data: [1, 2, 3, 4, 0, 0, 0, 0],
            ..FakeOps::ok()
        };
        let mut insn = EmulatedInsn::from_opcode(0, 0x8b);
        insn.opnd_bytes = 4;
        insn.modrm = 3 << 3;
        insn.addr_ref = Some(0x1000);
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                bx: u64::MAX,
                ..Default::default()
            },
            insn,
            ..Default::default()
        };

        assert_eq!(
            vc_handle_mmio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ctxt.regs.bx, 0x0403_0201);

        ops.read_data = [0x7f, 0, 0, 0, 0, 0, 0, 0];
        let mut zero = EmulatedInsn::from_opcode(0xb60f, 0x0f);
        zero.opnd_bytes = 8;
        zero.modrm = 2 << 3;
        zero.addr_ref = Some(0x1000);
        ctxt.insn = zero;
        ctxt.regs.dx = u64::MAX;
        assert_eq!(
            vc_handle_mmio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ctxt.regs.dx, 0x7f);

        ops.read_data = [0x80, 0, 0, 0, 0, 0, 0, 0];
        let mut sign = EmulatedInsn::from_opcode(0xbe0f, 0x0f);
        sign.opnd_bytes = 8;
        sign.modrm = 0;
        sign.addr_ref = Some(0x1000);
        ctxt.insn = sign;
        ctxt.regs.ax = 0;
        assert_eq!(
            vc_handle_mmio_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ctxt.regs.ax, 0xffff_ffff_ffff_ff80);
    }

    #[test]
    fn mmio_movs_splits_read_and_write_and_updates_rep_registers() {
        let mut ops = FakeOps {
            read_data: [0xde, 0xad, 0xbe, 0xef, 0, 0, 0, 0],
            ..FakeOps::ok()
        };
        let mut insn = EmulatedInsn::from_opcode(0, 0xa5);
        insn.opnd_bytes = 4;
        insn.rep_prefix = true;
        insn.ds_base = Some(0x1000);
        insn.es_base = Some(0x2000);
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                si: 0x40,
                di: 0x80,
                cx: 2,
                flags: X86_EFLAGS_DF,
                ..Default::default()
            },
            insn,
            ..Default::default()
        };

        assert_eq!(
            vc_handle_mmio_movs_with(&mut ops, &mut ctxt, 4),
            EsResult::Retry
        );
        assert_eq!(ops.read_addr, 0x1040);
        assert_eq!(ops.write_addr, 0x2080);
        assert_eq!(ops.write_len, 4);
        assert_eq!(&ops.write_data[..4], &[0xde, 0xad, 0xbe, 0xef]);
        assert_eq!(ctxt.regs.si, 0x3c);
        assert_eq!(ctxt.regs.di, 0x7c);
        assert_eq!(ctxt.regs.cx, 1);

        assert_eq!(
            vc_handle_mmio_movs_with(&mut ops, &mut ctxt, 4),
            EsResult::Continue
        );
        assert_eq!(ctxt.regs.cx, 0);
    }

    #[test]
    fn mmio_faults_match_linux_pf_bits() {
        let mut ops = FakeOps::ok();
        let mut ghcb = Ghcb::default();
        let mut insn = EmulatedInsn::from_opcode(0, 0x89);
        insn.opnd_bytes = 4;
        insn.addr_ref = Some(0xdead_beef);
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                user_mode: true,
                ..Default::default()
            },
            insn,
            ..Default::default()
        };

        assert_eq!(
            vc_do_mmio_with(&mut ops, &mut ghcb, &mut ctxt, 4, false),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_PF);
        assert_eq!(ctxt.fi.cr2, 0xdead_beef);
        assert_eq!(ctxt.fi.error_code, X86_PF_USER | X86_PF_WRITE);

        ops.write_ok = false;
        let mut buffer = [0u8; 4];
        assert_eq!(
            vc_write_mem_with(&mut ops, &mut ctxt, 0xcafe, &buffer),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.cr2, 0xcafe);
        assert_eq!(ctxt.fi.error_code, X86_PF_PROT | X86_PF_WRITE | X86_PF_USER);

        ops.read_ok = false;
        assert_eq!(
            vc_read_mem_with(&mut ops, &mut ctxt, 0xbabe, &mut buffer),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.cr2, 0xbabe);
        assert_eq!(ctxt.fi.error_code, X86_PF_PROT | X86_PF_USER);
    }

    #[test]
    fn ioio_check_matches_linux_user_bitmap_faults() {
        let mut ops = FakeOps::ok();
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                user_mode: true,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_ioio_check_with(&mut ops, &mut ctxt, 0x3f8, 1),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);
        assert_eq!(ctxt.fi.error_code, 0);

        ops.io_allowed = Some(false);
        ctxt.fi = Default::default();
        assert_eq!(
            vc_ioio_check_with(&mut ops, &mut ctxt, 0x3f8, 2),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);

        ops.io_allowed = Some(true);
        ctxt.fi = Default::default();
        assert_eq!(
            vc_ioio_check_with(&mut ops, &mut ctxt, 0x3f8, 4),
            EsResult::Continue
        );

        ops.io_allowed = None;
        ctxt.regs.user_mode = false;
        assert_eq!(
            vc_ioio_check_with(&mut ops, &mut ctxt, 0x3f8, 4),
            EsResult::Continue
        );
    }

    #[test]
    fn checked_ioio_applies_linux_user_bitmap_check_before_vmgexit() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps::ok();
        let mut insn = EmulatedInsn::from_opcode(0, 0xe4);
        insn.immediate = 0x64;
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                user_mode: true,
                ..Default::default()
            },
            insn,
            ..Default::default()
        };

        assert_eq!(
            vc_handle_ioio_checked_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Exception
        );
        assert_eq!(ops.call_count, 0);
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);
        assert_eq!(ctxt.fi.error_code, 0);
    }

    #[test]
    fn checked_ioio_string_uses_linux_memory_helpers_and_shared_buffer_pa() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            read_data: [0xab, 0, 0, 0, 0, 0, 0, 0],
            ..FakeOps::ok()
        };
        let mut outs = EmulatedInsn::from_opcode(0, 0x6e);
        outs.rep_prefix = true;
        outs.es_base = Some(0x1000);
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                si: 0x20,
                cx: 2,
                flags: X86_EFLAGS_DF,
                ..Default::default()
            },
            insn: outs,
            ..Default::default()
        };

        assert_eq!(
            vc_handle_ioio_checked_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_IOIO);
        assert_eq!(ops.exit_info_2, 2);
        assert_eq!(ops.read_addr, 0x101f);
        assert_eq!(&ghcb.shared_buffer[..2], &[0xab, 0xab]);
        assert_eq!(ghcb.save.sw_scratch, 0x8000);
        assert_eq!(ctxt.regs.si, 0x1e);
        assert_eq!(ctxt.regs.cx, 0);

        ghcb.shared_buffer[..2].copy_from_slice(&[0x11, 0x22]);
        let mut ins = EmulatedInsn::from_opcode(0, 0x6d);
        ins.rep_prefix = true;
        ins.opnd_bytes = 2;
        ins.es_base = Some(0x2000);
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                di: 0x40,
                cx: 1,
                ..Default::default()
            },
            insn: ins,
            ..Default::default()
        };

        assert_eq!(
            vc_handle_ioio_checked_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_info_2, 1);
        assert_eq!(ops.write_addr, 0x2040);
        assert_eq!(ops.write_len, 2);
        assert_eq!(&ops.write_data[..2], &[0x11, 0x22]);
        assert_eq!(ghcb.save.sw_scratch, 0x8000);
        assert_eq!(ctxt.regs.di, 0x42);
        assert_eq!(ctxt.regs.cx, 0);
    }

    #[test]
    fn msr_handler_models_svsm_caa_and_secure_tsc_special_cases() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps::ok();
        let state = VcHandlerState {
            svsm_caa_pa: 0x1234_5678_9abc_def0,
            secure_tsc_now: 0x0102_0304_0506_0708,
            sev_status: MSR_AMD64_SNP_SECURE_TSC,
            ..Default::default()
        };

        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                cx: MSR_SVSM_CAA as u64,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, false, &state),
            EsResult::Continue
        );
        assert_eq!((ctxt.regs.ax, ctxt.regs.dx), (0x9abc_def0, 0x1234_5678));
        assert_eq!(ops.call_count, 0);

        ctxt.regs.cx = MSR_IA32_TSC as u64;
        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, false, &state),
            EsResult::Continue
        );
        assert_eq!((ctxt.regs.ax, ctxt.regs.dx), (0x0506_0708, 0x0102_0304));

        ctxt.regs.cx = MSR_AMD64_GUEST_TSC_FREQ as u64;
        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, false, &state),
            EsResult::VmmError
        );

        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, true, &state),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);

        ctxt.fi = Default::default();
        ctxt.regs.cx = (1u64 << 32) | MSR_SVSM_CAA as u64;
        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, false, &state),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_MSR);
        assert_eq!(ghcb.save.rcx, (1u64 << 32) | MSR_SVSM_CAA as u64);

        ctxt.regs.cx = (1u64 << 32) | MSR_IA32_TSC as u64;
        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, false, &state),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_MSR);
        assert_eq!(ghcb.save.rcx, (1u64 << 32) | MSR_IA32_TSC as u64);
    }

    #[test]
    fn msr_handler_uses_linux_ghcb_registers_for_regular_intercepts() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            out_rax: Some(0xaaaa),
            out_rdx: Some(0xbbbb),
            ..FakeOps::ok()
        };
        let state = VcHandlerState::default();
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                cx: 0x1b,
                ax: 0x1111,
                dx: 0x2222,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, false, &state),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_MSR);
        assert_eq!(ops.exit_info_1, 0);
        assert_eq!(ghcb.save.rcx, 0x1b);
        assert_eq!((ctxt.regs.ax, ctxt.regs.dx), (0xaaaa, 0xbbbb));

        assert_eq!(
            vc_handle_msr_with(&mut ops, &mut ghcb, &mut ctxt, true, &state),
            EsResult::Continue
        );
        assert_eq!(ops.exit_info_1, 1);
        assert_eq!((ghcb.save.rax, ghcb.save.rdx), (0xaaaa, 0xbbbb));
    }

    #[test]
    fn dr7_handlers_follow_linux_reserved_bit_and_runtime_rules() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps::ok();
        let mut state = VcHandlerState {
            runtime_dr7: Some(DR7_RESET_VALUE),
            ..Default::default()
        };
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                ax: 0xffff_ffff,
                ..Default::default()
            },
            insn: EmulatedInsn {
                modrm: 0xc0,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(vc_insn_rm_reg_index(0xc0), Some(0));
        assert_eq!(vc_insn_rm_reg_index(0xc1), Some(1));
        assert_eq!(vc_insn_rm_reg_index(0x38), None);

        assert_eq!(
            vc_handle_dr7_write_with(&mut ops, &mut ghcb, &mut ctxt, &mut state),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_WRITE_DR7);
        assert_eq!(
            ghcb.save.rax,
            (0xffff_ffff & DR7_RESERVED_CLEAR_MASK) | DR7_RESET_VALUE
        );
        assert_eq!(state.runtime_dr7, Some(ghcb.save.rax));

        ctxt.regs.cx = 0;
        ctxt.insn.modrm = 0xc1;
        assert_eq!(vc_handle_dr7_read(&mut ctxt, &state), EsResult::Continue);
        assert_eq!(ctxt.regs.cx, state.runtime_dr7.unwrap());

        ctxt.insn.modrm = 0x38;
        assert_eq!(
            vc_handle_dr7_read(&mut ctxt, &state),
            EsResult::DecodeFailed
        );

        state.runtime_dr7 = None;
        ctxt.regs.ax = 1;
        ctxt.insn.modrm = 0xc0;
        assert_eq!(
            vc_handle_dr7_write_with(&mut ops, &mut ghcb, &mut ctxt, &mut state),
            EsResult::Unsupported
        );

        state.sev_status = MSR_AMD64_SNP_DEBUG_SWAP;
        assert_eq!(vc_handle_dr7_read(&mut ctxt, &state), EsResult::VmmError);
    }

    #[test]
    fn rdpmc_wbinvd_monitor_mwait_and_vmmcall_match_linux_paths() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            out_rax: Some(0x44),
            out_rdx: Some(0x55),
            ..FakeOps::ok()
        };
        let mut ctxt = EsEmCtxt {
            regs: VcRegs {
                ax: 0x11,
                cx: 7,
                dx: 0x22,
                user_mode: true,
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_handle_rdpmc_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_RDPMC);
        assert_eq!(ghcb.save.rcx, 7);
        assert_eq!((ctxt.regs.ax, ctxt.regs.dx), (0x44, 0x55));

        assert_eq!(
            vc_handle_wbinvd_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_WBINVD);
        assert_eq!(vc_handle_monitor(), EsResult::Continue);
        assert_eq!(vc_handle_mwait(), EsResult::Continue);

        ops.out_rax = Some(0x99);
        assert_eq!(
            vc_handle_vmmcall_with(&mut ops, &mut ghcb, &mut ctxt),
            EsResult::Continue
        );
        assert!(ops.prepared);
        assert!(ghcb_cpl_is_valid(&ghcb));
        assert_eq!(ghcb.save.cpl, 3);
        assert_eq!(ctxt.regs.ax, 0x99);
        assert_eq!(ctxt.regs.dx, 0xfeed);
    }

    #[test]
    fn rdtsc_handler_requires_linux_valid_outputs_and_rejects_secure_tsc() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            out_rax: Some(0xaaaa),
            out_rcx: Some(0xcccc),
            out_rdx: Some(0xbbbb),
            ..FakeOps::ok()
        };
        let mut ctxt = EsEmCtxt::default();

        assert_eq!(
            vc_handle_rdtsc_with(&mut ops, &mut ghcb, &mut ctxt, SVM_EXIT_RDTSCP, false),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_RDTSCP);
        assert_eq!(
            (ctxt.regs.ax, ctxt.regs.dx, ctxt.regs.cx),
            (0xaaaa, 0xbbbb, 0xcccc)
        );

        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps {
            out_rax: Some(0xaaaa),
            out_rdx: Some(0xbbbb),
            ..FakeOps::ok()
        };
        assert_eq!(
            vc_handle_rdtsc_with(&mut ops, &mut ghcb, &mut ctxt, SVM_EXIT_RDTSCP, false),
            EsResult::VmmError
        );
        assert_eq!(
            vc_handle_rdtsc_with(&mut ops, &mut ghcb, &mut ctxt, SVM_EXIT_RDTSC, true),
            EsResult::VmmError
        );
    }

    #[test]
    fn exitcode_result_dispatch_checks_opcode_then_calls_modeled_handlers() {
        let mut ghcb = Ghcb::default();
        let mut ops = FakeOps::ok();
        let mut state = VcHandlerState::default();
        let mut ctxt = EsEmCtxt {
            insn: EmulatedInsn {
                opcode: 0x330f,
                opcode_bytes: [0x0f, 0x33],
                ..Default::default()
            },
            ..Default::default()
        };

        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_RDPMC,
            ),
            EsResult::VmmError
        );

        ctxt.insn.opcode = 0;
        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_RDPMC,
            ),
            EsResult::Unsupported
        );

        ctxt.insn = EmulatedInsn::from_opcode(0, 0xe6);
        ctxt.insn.immediate = 0x80;
        ctxt.regs.ax = 0xabcd;
        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_IOIO,
            ),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_IOIO);
        assert_eq!(ghcb.save.rax, 0xcd);

        ctxt.fi = Default::default();
        ctxt.regs.user_mode = true;
        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_IOIO,
            ),
            EsResult::Exception
        );
        assert_eq!(ctxt.fi.vector, X86_TRAP_GP);

        ops.io_allowed = Some(true);
        ctxt.fi = Default::default();
        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_IOIO,
            ),
            EsResult::Continue
        );
        ctxt.regs.user_mode = false;

        ops.out_rax = Some(0x10);
        ops.out_rdx = Some(0x20);
        ctxt.insn = EmulatedInsn::from_opcode(0x310f, 0x0f);
        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_RDTSC,
            ),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_RDTSC);
        assert_eq!((ctxt.regs.ax, ctxt.regs.dx), (0x10, 0x20));

        ops.out_rax = Some(1);
        ops.out_rbx = Some(2);
        ops.out_rcx = Some(3);
        ops.out_rdx = Some(4);
        ctxt.insn = EmulatedInsn::from_opcode(0xa20f, 0x0f);
        ctxt.regs.ax = 0x8000_0001;
        ctxt.regs.cx = 7;
        assert_eq!(
            vc_handle_exitcode_result_with(
                &mut ops,
                &mut ghcb,
                &mut ctxt,
                &mut state,
                SVM_EXIT_CPUID,
            ),
            EsResult::Continue
        );
        assert_eq!(ops.exit_code, SVM_EXIT_CPUID);
        assert_eq!(
            (ctxt.regs.ax, ctxt.regs.bx, ctxt.regs.cx, ctxt.regs.dx),
            (1, 2, 3, 4)
        );
    }
}
