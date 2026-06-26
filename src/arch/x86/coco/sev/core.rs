//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/coco/sev/core.c
//! test-origin: linux:vendor/linux/arch/x86/coco/sev/core.c
//! AMD SEV/SEV-SNP core runtime state.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/coco/sev/core.c

use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use crate::arch::x86::coco::core::{CcAttr, CcPlatformState, cc_platform_has_state};
use crate::include::uapi::errno::{
    EAGAIN, EBADMSG, EINVAL, EIO, ENODEV, ENOMEM, ENOSPC, ENOTTY, EOPNOTSUPP, ETIMEDOUT,
};
use crate::lib::crypto::aesgcm::{
    AesGcmCtx as LinuxAesGcmCtx, GCM_AES_IV_SIZE, aesgcm_decrypt, aesgcm_encrypt, aesgcm_expandkey,
};
use crate::mm::page_flags::{__GFP_ZERO, GFP_KERNEL_ACCOUNT, GfpFlags};

pub use crate::arch::x86::boot::compressed::sev::{
    GHCB_SIZE, MSR_AMD64_SEV, MSR_AMD64_SEV_ES_GHCB, SEV_ENABLED, SEV_ES_ENABLED, SEV_SNP_ENABLED,
    SnpPageState, snp_page_state_msr,
};

pub const VMGEXIT_PSC_MAX_ENTRY: usize = 64;
pub const VMGEXIT_PSC_MAX_COUNT: usize = 253;
pub const VMGEXIT_PSC_OP_PRIVATE: u32 = 1;
pub const VMGEXIT_PSC_OP_SHARED: u32 = 2;
pub const RMP_PG_SIZE_4K: u32 = 0;
pub const RMP_PG_SIZE_2M: u32 = 1;
pub const PVALIDATE_FAIL_SIZEMISMATCH: i32 = 6;
pub const PVALIDATE_FAIL_NOUPDATE: i32 = 255;
pub const AP_INIT_CS_LIMIT: u32 = 0xffff;
pub const AP_INIT_DS_LIMIT: u32 = 0xffff;
pub const AP_INIT_LDTR_LIMIT: u32 = 0xffff;
pub const AP_INIT_GDTR_LIMIT: u32 = 0xffff;
pub const AP_INIT_IDTR_LIMIT: u32 = 0xffff;
pub const AP_INIT_TR_LIMIT: u32 = 0xffff;
pub const AP_INIT_RFLAGS_DEFAULT: u64 = 0x2;
pub const AP_INIT_DR6_DEFAULT: u64 = 0xffff_0ff0;
pub const AP_INIT_GPAT_DEFAULT: u64 = 0x0007_0406_0007_0406;
pub const AP_INIT_XCR0_DEFAULT: u64 = 0x1;
pub const AP_INIT_X87_FTW_DEFAULT: u64 = 0x5555;
pub const AP_INIT_X87_FCW_DEFAULT: u64 = 0x0040;
pub const AP_INIT_CR0_DEFAULT: u64 = 0x6000_0010;
pub const AP_INIT_MXCSR_DEFAULT: u32 = 0x1f80;
const PAGE_SHIFT: u64 = 12;
const PAGE_SIZE: u64 = 1 << PAGE_SHIFT;
const PAGE_MASK: u64 = !(PAGE_SIZE - 1);
const PMD_SIZE: u64 = 2 * 1024 * 1024;
const PMD_PAGES: usize = (PMD_SIZE / PAGE_SIZE) as usize;
pub const SVM_VMGEXIT_AP_HLT_LOOP: u64 = 0x8000_0004;
pub const SVM_VMGEXIT_AP_JUMP_TABLE: u64 = 0x8000_0005;
pub const SVM_VMGEXIT_SET_AP_JUMP_TABLE: u64 = 0;
pub const SVM_VMGEXIT_GET_AP_JUMP_TABLE: u64 = 1;
pub const SVM_VMGEXIT_AP_CREATION: u64 = 0x8000_0013;
pub const SVM_VMGEXIT_AP_CREATE_ON_INIT: u64 = 0;
pub const SVM_VMGEXIT_AP_CREATE: u64 = 1;
pub const SVM_VMGEXIT_AP_DESTROY: u64 = 2;
pub const SVM_VMGEXIT_SAVIC: u64 = 0x8000_001a;
pub const SVM_VMGEXIT_SAVIC_REGISTER_GPA: u64 = 0;
pub const SVM_VMGEXIT_SAVIC_UNREGISTER_GPA: u64 = 1;
pub const SVM_VMGEXIT_SAVIC_SELF_GPA: u64 = !0;
pub const APIC_BASE_MSR: u64 = 0x800;
pub const GHCB_HV_FT_SNP: u64 = 1 << 0;
pub const GHCB_HV_FT_SNP_AP_CREATION: u64 = 1 << 1;
pub const RMPADJUST_VMSA_PAGE_BIT: u64 = 1 << 16;
pub const X86_CR4_MCE: u64 = 1 << 6;
pub const EFER_SVME: u64 = 1 << 12;
pub const V_GIF_MASK: u64 = 1 << 9;
pub const V_NMI_ENABLE_MASK: u64 = 1 << 26;
pub const SVM_SELECTOR_WRITE_MASK: u16 = 1 << 1;
pub const SVM_SELECTOR_READ_MASK: u16 = SVM_SELECTOR_WRITE_MASK;
pub const SVM_SELECTOR_CODE_MASK: u16 = 1 << 3;
pub const SVM_SELECTOR_S_MASK: u16 = 1 << 4;
pub const SVM_SELECTOR_P_MASK: u16 = 1 << 7;
pub const INIT_CS_ATTRIBS: u16 =
    SVM_SELECTOR_P_MASK | SVM_SELECTOR_S_MASK | SVM_SELECTOR_READ_MASK | SVM_SELECTOR_CODE_MASK;
pub const INIT_DS_ATTRIBS: u16 =
    SVM_SELECTOR_P_MASK | SVM_SELECTOR_S_MASK | SVM_SELECTOR_WRITE_MASK;
pub const INIT_LDTR_ATTRIBS: u16 = SVM_SELECTOR_P_MASK | 2;
pub const INIT_TR_ATTRIBS: u16 = SVM_SELECTOR_P_MASK | 3;
pub const MAX_AUTHTAG_LEN: usize = 32;
pub const AUTHTAG_LEN: usize = 16;
pub const AAD_LEN: usize = 48;
pub const MSG_HDR_VER: u8 = 1;
pub const VMPCK_KEY_LEN: usize = 32;
pub const SNP_MSG_SEQNO_OVERFLOW_LIMIT: u64 = u32::MAX as u64;
pub const ENOSR: i32 = 63;
pub const SNP_AEAD_AES_256_GCM: u8 = 1;
pub const SNP_GUEST_MSG_HDR_SIZE: usize = MAX_AUTHTAG_LEN + 8 + 8 + AAD_LEN;
pub const SNP_GUEST_MSG_PAYLOAD_SIZE: usize = PAGE_SIZE as usize - SNP_GUEST_MSG_HDR_SIZE;
pub const SNP_GUEST_MSG_SIZE: usize = SNP_GUEST_MSG_HDR_SIZE + SNP_GUEST_MSG_PAYLOAD_SIZE;
pub const SNP_TSC_INFO_REQ_SZ: usize = 128;
pub const SNP_TSC_INFO_RESP_SZ: usize = 128;
pub const SNP_TSC_INFO_RESP_BUF_SZ: usize = SNP_TSC_INFO_RESP_SZ + AUTHTAG_LEN;
pub const SNP_MSG_TSC_INFO_REQ: u8 = 17;
pub const SNP_MSG_TSC_INFO_RSP: u8 = 18;
pub const SVM_VMGEXIT_GUEST_REQUEST: u64 = 0x8000_0011;
pub const SVM_VMGEXIT_EXT_GUEST_REQUEST: u64 = 0x8000_0012;
pub const SNP_GUEST_VMM_ERR_SHIFT: u64 = 32;
pub const SNP_GUEST_VMM_ERR_INVALID_LEN: u64 = 1;
pub const SNP_GUEST_VMM_ERR_BUSY: u64 = 2;
pub const SEV_RET_NO_FW_CALL: u64 = u64::MAX;
pub const SEV_TERM_SET_GEN: u8 = 0;
pub const SEV_TERM_SET_LINUX: u8 = 1;
pub const GHCB_SEV_ES_GEN_REQ: u8 = 0;
pub const GHCB_TERM_REGISTER: u8 = 0;
pub const GHCB_TERM_PSC: u8 = 1;
pub const GHCB_TERM_PVALIDATE: u8 = 2;
pub const GHCB_SNP_UNSUPPORTED: u8 = 2;
pub const GHCB_TERM_CPUID_HV: u8 = 5;
pub const GHCB_TERM_SECURE_TSC: u8 = 10;
pub const GHCB_TERM_SAVIC_FAIL: u8 = 12;
pub const SNP_GUEST_TSC_FREQ_MASK: u64 = (1 << 18) - 1;
pub const SNP_KEXEC_MAX_ACTIONS: usize = 64;
pub const SEV_EFI_MAP_MAX_ENTRIES: usize = 64;
pub const SNP_MSG_ALLOC_MAX_ACTIONS: usize = 8;
pub const SNP_MSG_FREE_MAX_ACTIONS: usize = 5;
pub const SEV_VC_INIT_MAX_CPUS: usize = 64;
pub const SEV_VC_INIT_MAX_ACTIONS: usize = SEV_VC_INIT_MAX_CPUS * 2 + 5;
pub const MSR_AMD64_SEV_ENABLED_BIT: usize = 0;
pub const MSR_AMD64_SEV_ES_ENABLED_BIT: usize = 1;
pub const MSR_AMD64_SEV_SNP_ENABLED_BIT: usize = 2;
pub const MSR_AMD64_SNP_VTOM_BIT: usize = 3;
pub const MSR_AMD64_SNP_REFLECT_VC_BIT: usize = 4;
pub const MSR_AMD64_SNP_RESTRICTED_INJ_BIT: usize = 5;
pub const MSR_AMD64_SNP_ALT_INJ_BIT: usize = 6;
pub const MSR_AMD64_SNP_DEBUG_SWAP_BIT: usize = 7;
pub const MSR_AMD64_SNP_PREVENT_HOST_IBS_BIT: usize = 8;
pub const MSR_AMD64_SNP_BTB_ISOLATION_BIT: usize = 9;
pub const MSR_AMD64_SNP_VMPL_SSS_BIT: usize = 10;
pub const MSR_AMD64_SNP_SECURE_TSC_BIT: usize = 11;
pub const MSR_AMD64_SNP_VMGEXIT_PARAM_BIT: usize = 12;
pub const MSR_AMD64_SNP_IBS_VIRT_BIT: usize = 14;
pub const MSR_AMD64_SNP_VMSA_REG_PROT_BIT: usize = 16;
pub const MSR_AMD64_SNP_SMT_PROT_BIT: usize = 17;
pub const MSR_AMD64_SNP_SECURE_AVIC_BIT: usize = 18;
pub const MSR_AMD64_SNP_IBPB_ON_ENTRY_BIT: usize = 23;
pub const MSR_AMD64_SNP_RESV_BIT: usize = 24;
pub const SEV_STATUS_FEATURE_NAMES: [Option<&'static str>; MSR_AMD64_SNP_RESV_BIT] = [
    Some("SEV"),
    Some("SEV-ES"),
    Some("SEV-SNP"),
    Some("vTom"),
    Some("ReflectVC"),
    Some("RI"),
    Some("AI"),
    Some("DebugSwap"),
    Some("NoHostIBS"),
    Some("BTBIsol"),
    Some("VmplSSS"),
    Some("SecureTSC"),
    Some("VMGExitParam"),
    None,
    Some("IBSVirt"),
    None,
    Some("VMSARegProt"),
    Some("SMTProt"),
    Some("SecureAVIC"),
    None,
    None,
    None,
    None,
    Some("IBPBOnEntry"),
];

pub const fn snp_guest_vmm_err(err: u64) -> u64 {
    err << SNP_GUEST_VMM_ERR_SHIFT
}

static SEV_HV_FEATURES: AtomicU64 = AtomicU64::new(0);
static SEV_SECRETS_PA: AtomicU64 = AtomicU64::new(0);
static SNP_TSC_SCALE: AtomicU64 = AtomicU64::new(0);
static SNP_TSC_OFFSET: AtomicU64 = AtomicU64::new(0);
static SNP_TSC_FREQ_KHZ: AtomicU64 = AtomicU64::new(0);
static SNP_VMPL: AtomicU8 = AtomicU8::new(0);

#[cfg(test)]
const PSC_ISSUE_LOG_CAP: usize = 16;
#[cfg(test)]
const PSC_EVENT_PVALIDATE_BEFORE: u8 = 1;
#[cfg(test)]
const PSC_EVENT_VMGEXIT: u8 = 2;
#[cfg(test)]
const PSC_EVENT_PVALIDATE_AFTER: u8 = 3;
#[cfg(test)]
static PSC_ISSUE_LOG_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static PSC_ISSUE_LOG: spin::Mutex<[(u8, u32, usize); PSC_ISSUE_LOG_CAP]> =
    spin::Mutex::new([(0, 0, 0); PSC_ISSUE_LOG_CAP]);
#[cfg(test)]
const PVALIDATE_LOG_CAP: usize = 16;
#[cfg(test)]
static PVALIDATE_LOG_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static PVALIDATE_LOG: spin::Mutex<[(u64, u32, bool); PVALIDATE_LOG_CAP]> =
    spin::Mutex::new([(0, 0, false); PVALIDATE_LOG_CAP]);
#[cfg(test)]
static TEST_PVALIDATE_2M_SIZE_MISMATCH: core::sync::atomic::AtomicBool =
    core::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
const GHCB_PSC_LOG_CAP: usize = 16;
#[cfg(test)]
const TEST_GHCB_PSC_COMPLETE: u8 = 0;
#[cfg(test)]
const TEST_GHCB_PSC_EXIT_INFO: u8 = 1;
#[cfg(test)]
const TEST_GHCB_PSC_RESERVED: u8 = 2;
#[cfg(test)]
const TEST_GHCB_PSC_END_GROWTH: u8 = 3;
#[cfg(test)]
static TEST_GHCB_PSC_MODE: core::sync::atomic::AtomicU8 =
    core::sync::atomic::AtomicU8::new(TEST_GHCB_PSC_COMPLETE);
#[cfg(test)]
static GHCB_PSC_LOG_LEN: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
static GHCB_PSC_LOG: spin::Mutex<[GhcbPscLogEntry; GHCB_PSC_LOG_CAP]> =
    spin::Mutex::new([GhcbPscLogEntry::empty(); GHCB_PSC_LOG_CAP]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PscEntry {
    pub gfn: u64,
    pub pagesize: u32,
    pub operation: u32,
    pub current_page: bool,
}

impl PscEntry {
    pub const fn new(gfn: u64, operation: u32) -> Self {
        Self {
            gfn,
            pagesize: 0,
            operation,
            current_page: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PscDesc {
    pub entries: [Option<PscEntry>; VMGEXIT_PSC_MAX_ENTRY],
    pub count: usize,
}

impl PscDesc {
    pub const fn empty() -> Self {
        Self {
            entries: [None; VMGEXIT_PSC_MAX_ENTRY],
            count: 0,
        }
    }

    pub fn push(&mut self, entry: PscEntry) -> Result<(), i32> {
        if self.count == VMGEXIT_PSC_MAX_ENTRY {
            return Err(EOPNOTSUPP);
        }
        self.entries[self.count] = Some(entry);
        self.count += 1;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GhcbPscSharedBuffer {
    desc: PscDesc,
    cur_entry: usize,
    end_entry: usize,
    reserved: bool,
    sw_exit_info_2: u64,
}

impl GhcbPscSharedBuffer {
    fn from_desc(desc: &PscDesc) -> Self {
        Self {
            desc: *desc,
            cur_entry: 0,
            end_entry: desc.count.saturating_sub(1),
            reserved: false,
            sw_exit_info_2: 0,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GhcbPscLogEntry {
    copied_count: usize,
    cur_entry: usize,
    end_entry: usize,
    sw_scratch: u64,
}

#[cfg(test)]
impl GhcbPscLogEntry {
    const fn empty() -> Self {
        Self {
            copied_count: 0,
            cur_entry: 0,
            end_entry: 0,
            sw_scratch: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PscBatchPlan {
    pub first: PscDesc,
    pub desc_count: usize,
    pub entry_count: usize,
    pub pages: usize,
    pub pvalidate_before: bool,
    pub pvalidate_after: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpPvalidateCacheEvictEntry {
    pub pfn: u64,
    pub vaddr: u64,
    pub pages: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpPvalidateCacheEvictPlan {
    pub skipped_for_coherency: bool,
    pub entries: [Option<SnpPvalidateCacheEvictEntry>; VMGEXIT_PSC_MAX_ENTRY],
    pub count: usize,
}

impl SnpPvalidateCacheEvictPlan {
    pub const fn empty(skipped_for_coherency: bool) -> Self {
        Self {
            skipped_for_coherency,
            entries: [None; VMGEXIT_PSC_MAX_ENTRY],
            count: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpAcceptMemoryPlan {
    pub vaddr: u64,
    pub npages: usize,
    pub private_plan: PscBatchPlan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpEarlySetPagesStatePlan {
    pub vaddr: u64,
    pub paddr: u64,
    pub npages: usize,
    pub op: u32,
    pub ca: u64,
    pub caa_pa: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpSetPagesStatePath {
    EarlyMsr(SnpEarlySetPagesStatePlan),
    Ghcb(PscBatchPlan),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpMemoryStatePlan {
    SkippedNotSnp,
    SetPages(SnpSetPagesStatePath),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SevRealModeHeader {
    pub trampoline_start: u64,
    pub sev_es_trampoline_start: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevApJumpTablePlan {
    pub jump_table_pa: u64,
    pub startup_ip: u16,
    pub startup_cs: u16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SevVmsaSegment {
    pub base: u64,
    pub limit: u32,
    pub attrib: u16,
    pub selector: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevVmsaPlan {
    pub apic_id: u32,
    pub cpu: u32,
    pub start_ip: u64,
    pub sipi_vector: u8,
    pub cs: SevVmsaSegment,
    pub ds: SevVmsaSegment,
    pub gdtr_limit: u32,
    pub ldtr_limit: u32,
    pub ldtr_attrib: u16,
    pub idtr_limit: u32,
    pub tr_limit: u32,
    pub tr_attrib: u16,
    pub rip: u64,
    pub cr0: u64,
    pub cr4: u64,
    pub dr6: u64,
    pub dr7: u64,
    pub rflags: u64,
    pub g_pat: u64,
    pub xcr0: u64,
    pub mxcsr: u32,
    pub x87_ftw: u64,
    pub x87_fcw: u64,
    pub vintr_ctrl: u64,
    pub efer: u64,
    pub vmpl: u8,
    pub sev_features: u64,
    pub tsc_scale: u64,
    pub tsc_offset: u64,
    pub ap_create_exit_info_1: u64,
    pub ap_create_event: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevVmsaInputs {
    pub hv_features: u64,
    pub real_mode: SevRealModeHeader,
    pub start_ip: u64,
    pub apic_id: u32,
    pub cpu: u32,
    pub vmsa_page_available: bool,
    pub set_vmsa_ok: bool,
    pub ap_create_ok: bool,
    pub cr4: u64,
    pub secure_avic: bool,
    pub snp_secure_tsc: bool,
    pub snp_vmpl: u8,
    pub sev_status: u64,
    pub tsc_scale: u64,
    pub tsc_offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpAllocVmsaPagePlan {
    pub cpu: u32,
    pub node: Option<usize>,
    pub gfp: GfpFlags,
    pub order: u32,
    pub allocation_size: u64,
    pub allocation_alignment: u64,
    pub allocated_order_page: bool,
    pub split_page_order: Option<u32>,
    pub freed_first_page: bool,
    pub returned_second_page: bool,
    pub returned_page_offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpWakeupCpuInputs {
    pub vmsa: SevVmsaInputs,
    pub new_vmsa_pa: u64,
    pub current_vmsa_pa: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpWakeupCpuPlan {
    pub rc: i32,
    pub vmsa: Option<SevVmsaPlan>,
    pub allocated_new_vmsa: bool,
    pub freed_new_vmsa_on_set_failure: bool,
    pub cleaned_new_vmsa_on_ap_failure: bool,
    pub cleaned_previous_vmsa: bool,
    pub recorded_vmsa_pa: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpWakeupCallback {
    WakeupCpuViaVmgexit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpWakeupSecondaryCpuPlan {
    pub sev_snp: bool,
    pub callback_installed: bool,
    pub callback: Option<SnpWakeupCallback>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevApControlPlan {
    pub event: u64,
    pub apic_id: u32,
    pub snp_vmpl: u8,
    pub vmsa_pa: u64,
    pub sw_exit_code: u64,
    pub sw_exit_info_1: u64,
    pub sw_exit_info_2: u64,
    pub ghcb_rax: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpSetVmsaPlan {
    Svsm {
        call_rax: u64,
        rcx_vmsa_pa: u64,
        rdx_caa_pa: u64,
        r8_apic_id: u64,
    },
    Rmpadjust {
        vmsa_va: u64,
        page_size: u32,
        attrs: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpSavicMsrPlan {
    pub reg: u32,
    pub msr: u64,
    pub write: bool,
    pub cx: u64,
    pub ax: u64,
    pub dx: u64,
    pub handler_result: crate::arch::x86::coco::sev::vc_shared::EsResult,
    pub termination: Option<SevTermination>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpSavicGpaPlan {
    pub operation: u64,
    pub gpa_argument: Option<u64>,
    pub sw_exit_code: u64,
    pub sw_exit_info_1: u64,
    pub sw_exit_info_2: u64,
    pub rax: u64,
    pub rbx: Option<u64>,
    pub hv_result: crate::arch::x86::coco::sev::vc_shared::EsResult,
    pub returned_gpa: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevStatusReport {
    pub names: [Option<&'static str>; MSR_AMD64_SNP_RESV_BIT],
    pub count: usize,
}

impl SevStatusReport {
    pub const fn empty() -> Self {
        Self {
            names: [None; MSR_AMD64_SNP_RESV_BIT],
            count: 0,
        }
    }

    fn push(&mut self, name: &'static str) {
        if self.count < self.names.len() {
            self.names[self.count] = Some(name);
            self.count += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpInfoReport {
    pub rc: i32,
    pub cpuid_table_count: u32,
    pub announced_cpuid_table: bool,
    pub dumped_cpuid_table: bool,
    pub sev_snp: bool,
    pub vmpl: u8,
    pub announced_vmpl: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpPlatformDevicePlan {
    pub rc: i32,
    pub sev_guest_registered: bool,
    pub vtpm_probe: bool,
    pub tpm_svsm_registered: bool,
    pub info_printed: bool,
}

impl SnpPlatformDevicePlan {
    pub const fn new(rc: i32) -> Self {
        Self {
            rc,
            sev_guest_registered: false,
            vtpm_probe: false,
            tpm_svsm_registered: false,
            info_printed: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmplShow {
    pub bytes: [u8; 4],
    pub len: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevSysfsInitPlan {
    pub rc: i32,
    pub got_dev_root: bool,
    pub created_kobject: bool,
    pub created_group: bool,
    pub put_dev_root: bool,
    pub put_kobject_on_group_error: bool,
}

impl SevSysfsInitPlan {
    pub const fn new(rc: i32) -> Self {
        Self {
            rc,
            got_dev_root: false,
            created_kobject: false,
            created_group: false,
            put_dev_root: false,
            put_kobject_on_group_error: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevEfiMapCpu {
    pub cpu: u32,
    pub ghcb_pa: u64,
    pub svsm_caa_pa: u64,
    pub ghcb_map_ok: bool,
    pub ca_map_ok: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SevEfiMapKind {
    Ghcb,
    SvsmCaa,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevEfiMapEntry {
    pub cpu: u32,
    pub kind: SevEfiMapKind,
    pub pfn: u64,
    pub address: u64,
    pub pages: usize,
    pub pflags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevEfiMapPlan {
    pub skipped_no_guest_state_encrypt: bool,
    pub rc: i32,
    pub entries: [Option<SevEfiMapEntry>; SEV_EFI_MAP_MAX_ENTRIES],
    pub entry_count: usize,
}

impl SevEfiMapPlan {
    pub const fn new(skipped_no_guest_state_encrypt: bool) -> Self {
        Self {
            skipped_no_guest_state_encrypt,
            rc: 0,
            entries: [None; SEV_EFI_MAP_MAX_ENTRIES],
            entry_count: 0,
        }
    }

    fn push(&mut self, entry: SevEfiMapEntry) -> bool {
        if self.entry_count == SEV_EFI_MAP_MAX_ENTRIES {
            self.rc = 1;
            return false;
        }
        self.entries[self.entry_count] = Some(entry);
        self.entry_count += 1;
        true
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GhcbSetupAction {
    RegisterPerCpuGhcb {
        paddr: u64,
        request_msr: u64,
        response_ok: bool,
    },
    SetGhcbsInitialized(bool),
    Terminate {
        termination: SevTermination,
    },
    SetGhcbProtocolVersion {
        version: u16,
    },
    ClearBootGhcbPage {
        paddr: u64,
    },
    SelectBootGhcb {
        paddr: u64,
    },
    RegisterBootGhcb {
        paddr: u64,
        request_msr: u64,
        response_ok: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GhcbSetupPlan {
    pub skipped_no_guest_state_encrypt: bool,
    pub used_runtime_handler: bool,
    pub actions: [Option<GhcbSetupAction>; 8],
    pub action_count: usize,
}

impl GhcbSetupPlan {
    pub const fn new(skipped_no_guest_state_encrypt: bool, used_runtime_handler: bool) -> Self {
        Self {
            skipped_no_guest_state_encrypt,
            used_runtime_handler,
            actions: [None; 8],
            action_count: 0,
        }
    }

    fn push(&mut self, action: GhcbSetupAction) {
        if self.action_count < self.actions.len() {
            self.actions[self.action_count] = Some(action);
            self.action_count += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevApHltExit {
    pub sw_exit_info_2_valid: bool,
    pub sw_exit_info_2: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevApHltLoopPlan {
    pub ghcb_acquired: bool,
    pub ghcb_released: bool,
    pub iterations: usize,
    pub sw_exit_code: u64,
    pub sw_exit_info_1: u64,
    pub sw_exit_info_2: u64,
    pub woke: bool,
    pub truncated_without_wakeup: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevPlayDeadPlan {
    pub play_dead_common_called: bool,
    pub hlt_loop: SevApHltLoopPlan,
    pub soft_restart_cpu_called: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SevVcInitPanic {
    MissingCpuFeatures,
    RuntimeDataAlloc { cpu: u32 },
    SvsmCaaAlloc { cpu: u32 },
    GhcbDecrypt { cpu: u32, rc: i32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevRuntimeCpuInput {
    pub cpu: u32,
    pub runtime_alloc_ok: bool,
    pub svsm_caa_alloc_ok: bool,
    pub svsm_caa_pa: u64,
    pub ghcb_decrypt_rc: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevRuntimeCpuPlan {
    pub cpu: u32,
    pub runtime_data_set: bool,
    pub svsm_caa_set: bool,
    pub svsm_caa_uses_boot_page: bool,
    pub svsm_caa_pa: u64,
    pub ghcb_decrypted: bool,
    pub ghcb_zeroed: bool,
    pub ghcb_active: bool,
    pub backup_ghcb_active: bool,
    pub panic: Option<SevVcInitPanic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevVcInitInputs<'a> {
    pub guest_state_encrypt: bool,
    pub cpu_features_ok: bool,
    pub sev_snp: bool,
    pub hv_features: u64,
    pub snp_vmpl: u8,
    pub boot_svsm_caa_pa: u64,
    pub smp_enabled: bool,
    pub cpus: &'a [SevRuntimeCpuInput],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SevVcInitAction {
    LoadHvFeatures { hv_features: u64 },
    AllocRuntimeData { cpu: u32 },
    InitGhcb { cpu: u32 },
    EnableSvsmCas,
    SetupPlayDead,
    SetRuntimeVcHandler,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevVcHandlingPlan {
    pub skipped_no_guest_state_encrypt: bool,
    pub panic: Option<SevVcInitPanic>,
    pub termination: Option<SevTermination>,
    pub use_cas: bool,
    pub play_dead_setup: bool,
    pub runtime_vc_handler_set: bool,
    pub cpu_plans: [Option<SevRuntimeCpuPlan>; SEV_VC_INIT_MAX_CPUS],
    pub cpu_count: usize,
    pub actions: [Option<SevVcInitAction>; SEV_VC_INIT_MAX_ACTIONS],
    pub action_count: usize,
}

impl SevVcHandlingPlan {
    pub const fn new(skipped_no_guest_state_encrypt: bool) -> Self {
        Self {
            skipped_no_guest_state_encrypt,
            panic: None,
            termination: None,
            use_cas: false,
            play_dead_setup: false,
            runtime_vc_handler_set: false,
            cpu_plans: [None; SEV_VC_INIT_MAX_CPUS],
            cpu_count: 0,
            actions: [None; SEV_VC_INIT_MAX_ACTIONS],
            action_count: 0,
        }
    }

    fn push_action(&mut self, action: SevVcInitAction) {
        if self.action_count < self.actions.len() {
            self.actions[self.action_count] = Some(action);
            self.action_count += 1;
        }
    }

    fn push_cpu(&mut self, cpu: SevRuntimeCpuPlan) {
        if self.cpu_count < self.cpu_plans.len() {
            self.cpu_plans[self.cpu_count] = Some(cpu);
            self.cpu_count += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpKexecBeginOutcome {
    SkippedNotSnp,
    SkippedKexecDisabled,
    ConversionsStopped,
    StopConversionWarned,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpKexecFinishSkip {
    None,
    NotSnp,
    KexecDisabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpKexecCpu {
    pub cpu: u32,
    pub present: bool,
    pub apic_id: u32,
    pub vmsa_pa: u64,
    pub ghcb_pa: u64,
    pub ghcb_mapping_size: u64,
    pub ghcb_mapping_level: u32,
    pub online_page_present: bool,
    pub clear_vmsa_ok: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpKexecMemoryRange {
    pub addr: u64,
    pub size: u64,
    pub level: u32,
    pub present: bool,
    pub decrypted: bool,
}

impl SnpKexecMemoryRange {
    pub const fn pages(self) -> usize {
        (self.size / PAGE_SIZE) as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpKexecFinishInputs<'a> {
    pub sev_snp: bool,
    pub kexec_core_enabled: bool,
    pub this_cpu: u32,
    pub snp_vmpl: u8,
    pub sev_features: u64,
    pub direct_map: &'a [SnpKexecMemoryRange],
    pub bss_decrypted: &'a [SnpKexecMemoryRange],
    pub bss_start: u64,
    pub bss_end: u64,
    pub possible_cpus: &'a [SnpKexecCpu],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpKexecAction {
    MarkCurrentVmsaOffline {
        cpu: u32,
        vmsa_pa: u64,
        online_page_found: bool,
    },
    DestroyAp {
        cpu: u32,
        apic_id: u32,
        vmsa_pa: u64,
        plan: SevApControlPlan,
    },
    ClearVmsa {
        cpu: u32,
        apic_id: u32,
        vmsa_pa: u64,
        plan: SnpSetVmsaPlan,
    },
    FreeVmsa {
        cpu: u32,
        vmsa_pa: u64,
    },
    LeakVmsa {
        cpu: u32,
        vmsa_pa: u64,
    },
    SetPteEncrypted {
        addr: u64,
        size: u64,
        level: u32,
    },
    SetMemoryPrivate {
        addr: u64,
        npages: usize,
    },
    FlushTlbAll,
    DisableGhcbProtocol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpKexecFinishPlan {
    pub skipped: SnpKexecFinishSkip,
    pub actions: [Option<SnpKexecAction>; SNP_KEXEC_MAX_ACTIONS],
    pub action_count: usize,
}

impl SnpKexecFinishPlan {
    pub const fn skipped(skipped: SnpKexecFinishSkip) -> Self {
        Self {
            skipped,
            actions: [None; SNP_KEXEC_MAX_ACTIONS],
            action_count: 0,
        }
    }

    pub const fn new() -> Self {
        Self::skipped(SnpKexecFinishSkip::None)
    }

    fn push(&mut self, action: SnpKexecAction) -> Result<(), i32> {
        if self.action_count == SNP_KEXEC_MAX_ACTIONS {
            return Err(EOPNOTSUPP);
        }
        self.actions[self.action_count] = Some(action);
        self.action_count += 1;
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SecretsOsArea {
    pub msg_seqno_0: u32,
    pub msg_seqno_1: u32,
    pub msg_seqno_2: u32,
    pub msg_seqno_3: u32,
    pub ap_jump_table_pa: u64,
    pub rsvd: [u8; 40],
    pub guest_usage: [u8; 32],
}

impl Default for SecretsOsArea {
    fn default() -> Self {
        Self {
            msg_seqno_0: 0,
            msg_seqno_1: 0,
            msg_seqno_2: 0,
            msg_seqno_3: 0,
            ap_jump_table_pa: 0,
            rsvd: [0; 40],
            guest_usage: [0; 32],
        }
    }
}

impl SecretsOsArea {
    pub const fn msg_seqno(&self, index: usize) -> Option<u32> {
        match index {
            0 => Some(self.msg_seqno_0),
            1 => Some(self.msg_seqno_1),
            2 => Some(self.msg_seqno_2),
            3 => Some(self.msg_seqno_3),
            _ => None,
        }
    }

    pub fn msg_seqno_mut(&mut self, index: usize) -> Option<&mut u32> {
        match index {
            0 => Some(&mut self.msg_seqno_0),
            1 => Some(&mut self.msg_seqno_1),
            2 => Some(&mut self.msg_seqno_2),
            3 => Some(&mut self.msg_seqno_3),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpSecretsPage {
    pub version: u32,
    pub imien_rsvd1: u32,
    pub fms: u32,
    pub rsvd2: u32,
    pub gosvw: [u8; 16],
    pub vmpck0: [u8; VMPCK_KEY_LEN],
    pub vmpck1: [u8; VMPCK_KEY_LEN],
    pub vmpck2: [u8; VMPCK_KEY_LEN],
    pub vmpck3: [u8; VMPCK_KEY_LEN],
    pub os_area: SecretsOsArea,
    pub vmsa_tweak_bitmap: [u8; 64],
    pub svsm_base: u64,
    pub svsm_size: u64,
    pub svsm_caa: u64,
    pub svsm_max_version: u32,
    pub svsm_guest_vmpl: u8,
    pub rsvd3: [u8; 3],
    pub tsc_factor: u32,
    pub rsvd4: [u8; 3740],
}

impl Default for SnpSecretsPage {
    fn default() -> Self {
        Self {
            version: 0,
            imien_rsvd1: 0,
            fms: 0,
            rsvd2: 0,
            gosvw: [0; 16],
            vmpck0: [0; VMPCK_KEY_LEN],
            vmpck1: [0; VMPCK_KEY_LEN],
            vmpck2: [0; VMPCK_KEY_LEN],
            vmpck3: [0; VMPCK_KEY_LEN],
            os_area: SecretsOsArea::default(),
            vmsa_tweak_bitmap: [0; 64],
            svsm_base: 0,
            svsm_size: 0,
            svsm_caa: 0,
            svsm_max_version: 0,
            svsm_guest_vmpl: 0,
            rsvd3: [0; 3],
            tsc_factor: 0,
            rsvd4: [0; 3740],
        }
    }
}

impl SnpSecretsPage {
    pub fn vmpck(&self, index: usize) -> Option<[u8; VMPCK_KEY_LEN]> {
        match index {
            0 => Some(self.vmpck0),
            1 => Some(self.vmpck1),
            2 => Some(self.vmpck2),
            3 => Some(self.vmpck3),
            _ => None,
        }
    }

    pub fn set_vmpck(&mut self, index: usize, key: [u8; VMPCK_KEY_LEN]) -> Result<(), i32> {
        match index {
            0 => self.vmpck0 = key,
            1 => self.vmpck1 = key,
            2 => self.vmpck2 = key,
            3 => self.vmpck3 = key,
            _ => return Err(EINVAL),
        }
        Ok(())
    }

    pub fn zero_vmpck(&mut self, index: usize) -> Result<(), i32> {
        self.set_vmpck(index, [0; VMPCK_KEY_LEN])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpVmpckSelection {
    pub key: [u8; VMPCK_KEY_LEN],
    pub seqno_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpAesGcmCtx {
    pub key_len: usize,
    pub authtag_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpGuestMsgHeader {
    pub authtag: [u8; MAX_AUTHTAG_LEN],
    pub msg_seqno: u64,
    pub rsvd1: [u8; 8],
    pub algo: u8,
    pub hdr_version: u8,
    pub hdr_sz: u16,
    pub msg_type: u8,
    pub msg_version: u8,
    pub msg_sz: u16,
    pub rsvd2: u32,
    pub msg_vmpck: u8,
    pub rsvd3: [u8; 35],
}

impl Default for SnpGuestMsgHeader {
    fn default() -> Self {
        Self {
            authtag: [0; MAX_AUTHTAG_LEN],
            msg_seqno: 0,
            rsvd1: [0; 8],
            algo: 0,
            hdr_version: 0,
            hdr_sz: 0,
            msg_type: 0,
            msg_version: 0,
            msg_sz: 0,
            rsvd2: 0,
            msg_vmpck: 0,
            rsvd3: [0; 35],
        }
    }
}

impl SnpGuestMsgHeader {
    pub fn aad_bytes(&self) -> [u8; AAD_LEN] {
        let mut aad = [0u8; AAD_LEN];
        aad[0] = self.algo;
        aad[1] = self.hdr_version;
        aad[2..4].copy_from_slice(&self.hdr_sz.to_le_bytes());
        aad[4] = self.msg_type;
        aad[5] = self.msg_version;
        aad[6..8].copy_from_slice(&self.msg_sz.to_le_bytes());
        aad[8..12].copy_from_slice(&self.rsvd2.to_le_bytes());
        aad[12] = self.msg_vmpck;
        aad[13..48].copy_from_slice(&self.rsvd3);
        aad
    }

    pub fn iv_from_seqno(&self) -> [u8; GCM_AES_IV_SIZE] {
        let mut iv = [0u8; GCM_AES_IV_SIZE];
        iv[..8].copy_from_slice(&self.msg_seqno.to_le_bytes());
        iv
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpGuestMsg {
    pub hdr: SnpGuestMsgHeader,
    pub payload: [u8; SNP_GUEST_MSG_PAYLOAD_SIZE],
}

impl Default for SnpGuestMsg {
    fn default() -> Self {
        Self {
            hdr: SnpGuestMsgHeader::default(),
            payload: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnpReqData {
    pub req_gpa: u64,
    pub resp_gpa: u64,
    pub data_gpa: u64,
    pub data_npages: u32,
}

impl Default for SnpReqData {
    fn default() -> Self {
        Self {
            req_gpa: 0,
            resp_gpa: 0,
            data_gpa: 0,
            data_npages: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnpGuestReq {
    pub req_buf: [u8; SNP_GUEST_MSG_PAYLOAD_SIZE],
    pub req_sz: usize,
    pub resp_buf: [u8; SNP_GUEST_MSG_PAYLOAD_SIZE],
    pub resp_sz: usize,
    pub req_buf_valid: bool,
    pub resp_buf_valid: bool,
    pub exit_code: u64,
    pub exitinfo2: u64,
    pub vmpck_id: u32,
    pub msg_version: u8,
    pub msg_type: u8,
    pub input: SnpReqData,
    pub certs_data_present: bool,
    pub certs_data_gpa: u64,
}

impl Default for SnpGuestReq {
    fn default() -> Self {
        Self {
            req_buf: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
            req_sz: 0,
            resp_buf: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
            resp_sz: 0,
            req_buf_valid: true,
            resp_buf_valid: true,
            exit_code: 0,
            exitinfo2: 0,
            vmpck_id: 0,
            msg_version: 0,
            msg_type: 0,
            input: SnpReqData::default(),
            certs_data_present: false,
            certs_data_gpa: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpTscInfoReq {
    pub rsvd: [u8; SNP_TSC_INFO_REQ_SZ],
}

impl Default for SnpTscInfoReq {
    fn default() -> Self {
        Self {
            rsvd: [0; SNP_TSC_INFO_REQ_SZ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpTscInfoResp {
    pub status: u32,
    pub rsvd1: u32,
    pub tsc_scale: u64,
    pub tsc_offset: u64,
    pub tsc_factor: u32,
    pub rsvd2: [u8; 100],
}

impl Default for SnpTscInfoResp {
    fn default() -> Self {
        Self {
            status: 0,
            rsvd1: 0,
            tsc_scale: 0,
            tsc_offset: 0,
            tsc_factor: 0,
            rsvd2: [0; 100],
        }
    }
}

impl SnpTscInfoResp {
    pub fn from_payload(payload: &[u8]) -> Self {
        let mut padded = [0u8; SNP_TSC_INFO_RESP_SZ];
        let len = payload.len().min(SNP_TSC_INFO_RESP_SZ);
        padded[..len].copy_from_slice(&payload[..len]);

        let mut status = [0u8; 4];
        status.copy_from_slice(&padded[0..4]);
        let mut rsvd1 = [0u8; 4];
        rsvd1.copy_from_slice(&padded[4..8]);
        let mut tsc_scale = [0u8; 8];
        tsc_scale.copy_from_slice(&padded[8..16]);
        let mut tsc_offset = [0u8; 8];
        tsc_offset.copy_from_slice(&padded[16..24]);
        let mut tsc_factor = [0u8; 4];
        tsc_factor.copy_from_slice(&padded[24..28]);
        let mut rsvd2 = [0u8; 100];
        rsvd2.copy_from_slice(&padded[28..128]);

        Self {
            status: u32::from_le_bytes(status),
            rsvd1: u32::from_le_bytes(rsvd1),
            tsc_scale: u64::from_le_bytes(tsc_scale),
            tsc_offset: u64::from_le_bytes(tsc_offset),
            tsc_factor: u32::from_le_bytes(tsc_factor),
            rsvd2,
        }
    }

    pub fn to_payload(self) -> [u8; SNP_TSC_INFO_RESP_SZ] {
        let mut payload = [0u8; SNP_TSC_INFO_RESP_SZ];
        payload[0..4].copy_from_slice(&self.status.to_le_bytes());
        payload[4..8].copy_from_slice(&self.rsvd1.to_le_bytes());
        payload[8..16].copy_from_slice(&self.tsc_scale.to_le_bytes());
        payload[16..24].copy_from_slice(&self.tsc_offset.to_le_bytes());
        payload[24..28].copy_from_slice(&self.tsc_factor.to_le_bytes());
        payload[28..128].copy_from_slice(&self.rsvd2);
        payload
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpTscInfoResources {
    pub tsc_req_available: bool,
    pub tsc_resp_available: bool,
    pub msg_desc_available: bool,
    pub crypto_available: bool,
    pub secrets: SnpSecretsPage,
}

impl SnpTscInfoResources {
    pub const fn new(secrets: SnpSecretsPage) -> Self {
        Self {
            tsc_req_available: true,
            tsc_resp_available: true,
            msg_desc_available: true,
            crypto_available: true,
            secrets,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SevTermination {
    pub set: u8,
    pub reason: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpSecureTscPrepareOutcome {
    Skipped,
    Enabled,
    Terminated {
        termination: SevTermination,
        rc: i32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpSecureTscInitOutcome {
    Skipped,
    Initialized { freq_mhz: u64, freq_khz: u64 },
    Terminated { termination: SevTermination },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpIssueGuestRequestGhcb {
    pub ghcb_available: bool,
    pub hv_call_result: Result<(), i32>,
    pub sw_exit_info_2: u64,
    pub rbx: u64,
}

impl Default for SnpIssueGuestRequestGhcb {
    fn default() -> Self {
        Self {
            ghcb_available: true,
            hv_call_result: Ok(()),
            sw_exit_info_2: 0,
            rbx: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpIssueGuestRequestPlan {
    pub exit_code: u64,
    pub req_gpa: u64,
    pub resp_gpa: u64,
    pub rax: Option<u64>,
    pub rbx: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreeSharedPagesPlan {
    pub size: usize,
    pub npages: usize,
    pub order: u32,
    pub buf_present: bool,
    pub set_memory_encrypted: bool,
    pub set_memory_encrypted_rc: i32,
    pub freed_pages: bool,
    pub leaked_on_encrypt_error: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AllocSharedPagesPlan {
    pub size: usize,
    pub npages: usize,
    pub order: u32,
    pub page_allocated: bool,
    pub set_memory_decrypted: bool,
    pub set_memory_decrypted_rc: i32,
    pub freed_pages_on_decrypt_error: bool,
    pub returned_page: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpMsgAllocAction {
    KzallocDesc,
    IoremapSecrets,
    AllocRequestShared,
    AllocResponseShared,
    FreeRequestShared,
    IounmapSecrets,
    KfreeDesc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpMsgAllocInputs {
    pub mdesc_alloc_ok: bool,
    pub secrets_ioremap_ok: bool,
    pub request_page_alloc_ok: bool,
    pub request_set_decrypted_rc: i32,
    pub response_page_alloc_ok: bool,
    pub response_set_decrypted_rc: i32,
}

impl SnpMsgAllocInputs {
    pub const fn success() -> Self {
        Self {
            mdesc_alloc_ok: true,
            secrets_ioremap_ok: true,
            request_page_alloc_ok: true,
            request_set_decrypted_rc: 0,
            response_page_alloc_ok: true,
            response_set_decrypted_rc: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpMsgAllocPlan {
    pub rc: i32,
    pub success: bool,
    pub request_shared: Option<AllocSharedPagesPlan>,
    pub response_shared: Option<AllocSharedPagesPlan>,
    pub actions: [Option<SnpMsgAllocAction>; SNP_MSG_ALLOC_MAX_ACTIONS],
    pub action_count: usize,
}

impl SnpMsgAllocPlan {
    pub const fn new() -> Self {
        Self {
            rc: 0,
            success: false,
            request_shared: None,
            response_shared: None,
            actions: [None; SNP_MSG_ALLOC_MAX_ACTIONS],
            action_count: 0,
        }
    }

    fn push(&mut self, action: SnpMsgAllocAction) {
        if self.action_count < self.actions.len() {
            self.actions[self.action_count] = Some(action);
            self.action_count += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpMsgFreeAction {
    KfreeCtx,
    FreeResponseShared,
    FreeRequestShared,
    IounmapSecrets,
    KfreeSensitiveDesc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpMsgFreePlan {
    pub desc_present: bool,
    pub ctx_present: bool,
    pub response_shared: Option<FreeSharedPagesPlan>,
    pub request_shared: Option<FreeSharedPagesPlan>,
    pub actions: [Option<SnpMsgFreeAction>; SNP_MSG_FREE_MAX_ACTIONS],
    pub action_count: usize,
}

impl SnpMsgFreePlan {
    pub const fn new(desc_present: bool, ctx_present: bool) -> Self {
        Self {
            desc_present,
            ctx_present,
            response_shared: None,
            request_shared: None,
            actions: [None; SNP_MSG_FREE_MAX_ACTIONS],
            action_count: 0,
        }
    }

    fn push(&mut self, action: SnpMsgFreeAction) {
        if self.action_count < self.actions.len() {
            self.actions[self.action_count] = Some(action);
            self.action_count += 1;
        }
    }
}

pub trait SnpGuestRequestBackend {
    fn issue_guest_request(
        &mut self,
        mdesc: &mut SnpMsgDesc,
        req: &mut SnpGuestReq,
    ) -> Result<(), i32>;

    fn retry_timed_out(&mut self, _attempts: usize) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnpMsgDesc {
    pub request: SnpGuestMsg,
    pub response: SnpGuestMsg,
    pub secret_request: SnpGuestMsg,
    pub secret_response: SnpGuestMsg,
    pub secrets: SnpSecretsPage,
    pub ctx: Option<SnpAesGcmCtx>,
    pub os_area_msg_seqno: Option<usize>,
    pub vmpck: Option<[u8; VMPCK_KEY_LEN]>,
    pub vmpck_id: i32,
}

impl SnpMsgDesc {
    pub const fn new(secrets: SnpSecretsPage) -> Self {
        Self {
            request: SnpGuestMsg {
                hdr: SnpGuestMsgHeader {
                    authtag: [0; MAX_AUTHTAG_LEN],
                    msg_seqno: 0,
                    rsvd1: [0; 8],
                    algo: 0,
                    hdr_version: 0,
                    hdr_sz: 0,
                    msg_type: 0,
                    msg_version: 0,
                    msg_sz: 0,
                    rsvd2: 0,
                    msg_vmpck: 0,
                    rsvd3: [0; 35],
                },
                payload: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
            },
            response: SnpGuestMsg {
                hdr: SnpGuestMsgHeader {
                    authtag: [0; MAX_AUTHTAG_LEN],
                    msg_seqno: 0,
                    rsvd1: [0; 8],
                    algo: 0,
                    hdr_version: 0,
                    hdr_sz: 0,
                    msg_type: 0,
                    msg_version: 0,
                    msg_sz: 0,
                    rsvd2: 0,
                    msg_vmpck: 0,
                    rsvd3: [0; 35],
                },
                payload: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
            },
            secret_request: SnpGuestMsg {
                hdr: SnpGuestMsgHeader {
                    authtag: [0; MAX_AUTHTAG_LEN],
                    msg_seqno: 0,
                    rsvd1: [0; 8],
                    algo: 0,
                    hdr_version: 0,
                    hdr_sz: 0,
                    msg_type: 0,
                    msg_version: 0,
                    msg_sz: 0,
                    rsvd2: 0,
                    msg_vmpck: 0,
                    rsvd3: [0; 35],
                },
                payload: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
            },
            secret_response: SnpGuestMsg {
                hdr: SnpGuestMsgHeader {
                    authtag: [0; MAX_AUTHTAG_LEN],
                    msg_seqno: 0,
                    rsvd1: [0; 8],
                    algo: 0,
                    hdr_version: 0,
                    hdr_sz: 0,
                    msg_type: 0,
                    msg_version: 0,
                    msg_sz: 0,
                    rsvd2: 0,
                    msg_vmpck: 0,
                    rsvd3: [0; 35],
                },
                payload: [0; SNP_GUEST_MSG_PAYLOAD_SIZE],
            },
            secrets,
            ctx: None,
            os_area_msg_seqno: None,
            vmpck: None,
            vmpck_id: -1,
        }
    }
}

pub const fn get_jump_table_addr_from_sources(
    snp_enabled: bool,
    snp_ap_jump_table_pa: u64,
    ghcb_exit_info_valid: bool,
    ghcb_sw_exit_info_2: u64,
) -> u64 {
    if snp_enabled {
        snp_ap_jump_table_pa
    } else if ghcb_exit_info_valid {
        ghcb_sw_exit_info_2
    } else {
        0
    }
}

pub fn sev_es_setup_ap_jump_table_from_addr(
    rmh: &SevRealModeHeader,
    jump_table_addr: u64,
    jump_table: Option<&mut [u16; 2]>,
) -> Result<Option<SevApJumpTablePlan>, i32> {
    if jump_table_addr == 0 {
        return Ok(None);
    }

    if jump_table_addr & !PAGE_MASK != 0 {
        return Err(EINVAL);
    }

    let Some(jump_table) = jump_table else {
        return Err(EIO);
    };

    let plan = SevApJumpTablePlan {
        jump_table_pa: jump_table_addr & PAGE_MASK,
        startup_ip: rmh
            .sev_es_trampoline_start
            .wrapping_sub(rmh.trampoline_start) as u16,
        startup_cs: (rmh.trampoline_start >> 4) as u16,
    };
    jump_table[0] = plan.startup_ip;
    jump_table[1] = plan.startup_cs;

    Ok(Some(plan))
}

pub fn sev_es_setup_ap_jump_table(
    rmh: &SevRealModeHeader,
    snp_enabled: bool,
    snp_ap_jump_table_pa: u64,
    ghcb_exit_info_valid: bool,
    ghcb_sw_exit_info_2: u64,
    jump_table: Option<&mut [u16; 2]>,
) -> Result<Option<SevApJumpTablePlan>, i32> {
    let jump_table_addr = get_jump_table_addr_from_sources(
        snp_enabled,
        snp_ap_jump_table_pa,
        ghcb_exit_info_valid,
        ghcb_sw_exit_info_2,
    );
    sev_es_setup_ap_jump_table_from_addr(rmh, jump_table_addr, jump_table)
}

pub fn snp_alloc_vmsa_page_plan(cpu: u32, page_alloc_ok: bool) -> SnpAllocVmsaPagePlan {
    let mut plan = SnpAllocVmsaPagePlan {
        cpu,
        node: crate::arch::x86::mm::numa::cpu_to_node(cpu as usize).ok(),
        gfp: GFP_KERNEL_ACCOUNT | __GFP_ZERO,
        order: 1,
        allocation_size: PAGE_SIZE * 2,
        allocation_alignment: PAGE_SIZE * 2,
        allocated_order_page: page_alloc_ok,
        split_page_order: None,
        freed_first_page: false,
        returned_second_page: false,
        returned_page_offset: 0,
    };

    if !page_alloc_ok {
        return plan;
    }

    plan.split_page_order = Some(1);
    plan.freed_first_page = true;
    plan.returned_second_page = true;
    plan.returned_page_offset = PAGE_SIZE;
    plan
}

pub fn snp_wakeup_cpu_vmsa_plan(inputs: SevVmsaInputs) -> Result<SevVmsaPlan, i32> {
    if inputs.hv_features & GHCB_HV_FT_SNP_AP_CREATION == 0 {
        return Err(EOPNOTSUPP);
    }

    if inputs.start_ip != inputs.real_mode.trampoline_start {
        return Err(EINVAL);
    }

    if !inputs.vmsa_page_available {
        return Err(ENOMEM);
    }

    let start_ip = inputs.real_mode.sev_es_trampoline_start;
    let sipi_vector = (start_ip >> PAGE_SHIFT) as u8;
    let ds = SevVmsaSegment {
        base: 0,
        limit: AP_INIT_DS_LIMIT,
        attrib: INIT_DS_ATTRIBS,
        selector: 0,
    };
    let mut plan = SevVmsaPlan {
        apic_id: inputs.apic_id,
        cpu: inputs.cpu,
        start_ip,
        sipi_vector,
        cs: SevVmsaSegment {
            base: (sipi_vector as u64) << PAGE_SHIFT,
            limit: AP_INIT_CS_LIMIT,
            attrib: INIT_CS_ATTRIBS,
            selector: (sipi_vector as u16) << 8,
        },
        ds,
        gdtr_limit: AP_INIT_GDTR_LIMIT,
        ldtr_limit: AP_INIT_LDTR_LIMIT,
        ldtr_attrib: INIT_LDTR_ATTRIBS,
        idtr_limit: AP_INIT_IDTR_LIMIT,
        tr_limit: AP_INIT_TR_LIMIT,
        tr_attrib: INIT_TR_ATTRIBS,
        rip: start_ip & (PAGE_SIZE - 1),
        cr0: AP_INIT_CR0_DEFAULT,
        cr4: inputs.cr4 & X86_CR4_MCE,
        dr6: AP_INIT_DR6_DEFAULT,
        dr7: crate::arch::x86::coco::sev::vc_handle::DR7_RESET_VALUE,
        rflags: AP_INIT_RFLAGS_DEFAULT,
        g_pat: AP_INIT_GPAT_DEFAULT,
        xcr0: AP_INIT_XCR0_DEFAULT,
        mxcsr: AP_INIT_MXCSR_DEFAULT,
        x87_ftw: AP_INIT_X87_FTW_DEFAULT,
        x87_fcw: AP_INIT_X87_FCW_DEFAULT,
        vintr_ctrl: 0,
        efer: EFER_SVME,
        vmpl: inputs.snp_vmpl,
        sev_features: inputs.sev_status >> 2,
        tsc_scale: 0,
        tsc_offset: 0,
        ap_create_exit_info_1: ((inputs.apic_id as u64) << 32)
            | ((inputs.snp_vmpl as u64) << 16)
            | SVM_VMGEXIT_AP_CREATE,
        ap_create_event: SVM_VMGEXIT_AP_CREATE,
    };

    if inputs.secure_avic {
        plan.vintr_ctrl |= V_GIF_MASK | V_NMI_ENABLE_MASK;
    }

    if inputs.snp_secure_tsc {
        plan.tsc_scale = inputs.tsc_scale;
        plan.tsc_offset = inputs.tsc_offset;
    }

    if !inputs.set_vmsa_ok || !inputs.ap_create_ok {
        return Err(EINVAL);
    }

    Ok(plan)
}

pub fn wakeup_cpu_via_vmgexit_plan(inputs: SnpWakeupCpuInputs) -> SnpWakeupCpuPlan {
    let mut plan = SnpWakeupCpuPlan {
        rc: 0,
        vmsa: None,
        allocated_new_vmsa: false,
        freed_new_vmsa_on_set_failure: false,
        cleaned_new_vmsa_on_ap_failure: false,
        cleaned_previous_vmsa: false,
        recorded_vmsa_pa: 0,
    };

    if inputs.vmsa.hv_features & GHCB_HV_FT_SNP_AP_CREATION == 0 {
        plan.rc = -EOPNOTSUPP;
        return plan;
    }

    if inputs.vmsa.start_ip != inputs.vmsa.real_mode.trampoline_start {
        plan.rc = -EINVAL;
        return plan;
    }

    if !inputs.vmsa.vmsa_page_available {
        plan.rc = -ENOMEM;
        return plan;
    }
    plan.allocated_new_vmsa = true;

    if !inputs.vmsa.set_vmsa_ok {
        plan.freed_new_vmsa_on_set_failure = true;
        plan.rc = -EINVAL;
        return plan;
    }

    if !inputs.vmsa.ap_create_ok {
        plan.cleaned_new_vmsa_on_ap_failure = true;
        plan.cleaned_previous_vmsa = inputs.current_vmsa_pa != 0;
        plan.rc = -EINVAL;
        return plan;
    }

    match snp_wakeup_cpu_vmsa_plan(inputs.vmsa) {
        Ok(vmsa) => {
            plan.vmsa = Some(vmsa);
            plan.cleaned_previous_vmsa = inputs.current_vmsa_pa != 0;
            plan.recorded_vmsa_pa = inputs.new_vmsa_pa;
        }
        Err(err) => {
            plan.rc = -err;
        }
    }

    plan
}

pub const fn snp_set_wakeup_secondary_cpu_plan(sev_snp: bool) -> SnpWakeupSecondaryCpuPlan {
    SnpWakeupSecondaryCpuPlan {
        sev_snp,
        callback_installed: sev_snp,
        callback: if sev_snp {
            Some(SnpWakeupCallback::WakeupCpuViaVmgexit)
        } else {
            None
        },
    }
}

fn valid_ap_creation_event(event: u64) -> bool {
    matches!(
        event,
        SVM_VMGEXIT_AP_CREATE_ON_INIT | SVM_VMGEXIT_AP_CREATE | SVM_VMGEXIT_AP_DESTROY
    )
}

pub fn vmgexit_ap_control_request(
    event: u64,
    apic_id: u32,
    snp_vmpl: u8,
    vmsa_pa: u64,
    sev_features: u64,
) -> Result<SevApControlPlan, i32> {
    if !valid_ap_creation_event(event) {
        return Err(EOPNOTSUPP);
    }

    Ok(SevApControlPlan {
        event,
        apic_id,
        snp_vmpl,
        vmsa_pa,
        sw_exit_code: SVM_VMGEXIT_AP_CREATION,
        sw_exit_info_1: ((apic_id as u64) << 32) | ((snp_vmpl as u64) << 16) | event,
        sw_exit_info_2: vmsa_pa,
        ghcb_rax: if event != SVM_VMGEXIT_AP_DESTROY {
            Some(sev_features)
        } else {
            None
        },
    })
}

pub fn vmgexit_ap_control_with_response(
    event: u64,
    apic_id: u32,
    snp_vmpl: u8,
    vmsa_pa: u64,
    sev_features: u64,
    response_exit_info_1_valid: bool,
    response_sw_exit_info_1: u64,
) -> Result<SevApControlPlan, i32> {
    let plan = vmgexit_ap_control_request(event, apic_id, snp_vmpl, vmsa_pa, sev_features)?;
    if !response_exit_info_1_valid || response_sw_exit_info_1 & 0xffff_ffff != 0 {
        return Err(EINVAL);
    }
    Ok(plan)
}

pub fn snp_set_vmsa_plan(
    vmsa_pa: u64,
    caa_pa: u64,
    apic_id: u32,
    make_vmsa: bool,
    snp_vmpl: u8,
) -> SnpSetVmsaPlan {
    if snp_vmpl != 0 {
        if make_vmsa {
            SnpSetVmsaPlan::Svsm {
                call_rax: crate::arch::x86::coco::sev::svsm::svsm_core_call(
                    crate::arch::x86::coco::sev::svsm::SVSM_CORE_CREATE_VCPU as u64,
                ),
                rcx_vmsa_pa: vmsa_pa,
                rdx_caa_pa: caa_pa,
                r8_apic_id: apic_id as u64,
            }
        } else {
            SnpSetVmsaPlan::Svsm {
                call_rax: crate::arch::x86::coco::sev::svsm::svsm_core_call(
                    crate::arch::x86::coco::sev::svsm::SVSM_CORE_DELETE_VCPU as u64,
                ),
                rcx_vmsa_pa: vmsa_pa,
                rdx_caa_pa: 0,
                r8_apic_id: 0,
            }
        }
    } else {
        let mut attrs = 1u64;
        if make_vmsa {
            attrs |= RMPADJUST_VMSA_PAGE_BIT;
        }
        SnpSetVmsaPlan::Rmpadjust {
            vmsa_va: vmsa_pa,
            page_size: RMP_PG_SIZE_4K,
            attrs,
        }
    }
}

pub const fn snp_kexec_begin_plan(
    sev_snp: bool,
    kexec_core_enabled: bool,
    stop_conversion_ok: bool,
) -> SnpKexecBeginOutcome {
    if !sev_snp {
        return SnpKexecBeginOutcome::SkippedNotSnp;
    }

    if !kexec_core_enabled {
        return SnpKexecBeginOutcome::SkippedKexecDisabled;
    }

    if stop_conversion_ok {
        SnpKexecBeginOutcome::ConversionsStopped
    } else {
        SnpKexecBeginOutcome::StopConversionWarned
    }
}

fn page_level_mask_from_size(size: u64) -> u64 {
    if size == 0 { 0 } else { !(size - 1) }
}

fn snp_kexec_cleanup_vmsa_plan(
    plan: &mut SnpKexecFinishPlan,
    cpu: &SnpKexecCpu,
    snp_vmpl: u8,
) -> Result<(), i32> {
    plan.push(SnpKexecAction::ClearVmsa {
        cpu: cpu.cpu,
        apic_id: cpu.apic_id,
        vmsa_pa: cpu.vmsa_pa,
        plan: snp_set_vmsa_plan(cpu.vmsa_pa, 0, cpu.apic_id, false, snp_vmpl),
    })?;

    if cpu.clear_vmsa_ok {
        plan.push(SnpKexecAction::FreeVmsa {
            cpu: cpu.cpu,
            vmsa_pa: cpu.vmsa_pa,
        })
    } else {
        plan.push(SnpKexecAction::LeakVmsa {
            cpu: cpu.cpu,
            vmsa_pa: cpu.vmsa_pa,
        })
    }
}

fn snp_kexec_shutdown_all_aps(
    plan: &mut SnpKexecFinishPlan,
    inputs: &SnpKexecFinishInputs<'_>,
) -> Result<(), i32> {
    for cpu in inputs.possible_cpus {
        if !cpu.present {
            continue;
        }

        if cpu.vmsa_pa == 0 {
            continue;
        }

        if cpu.cpu == inputs.this_cpu {
            plan.push(SnpKexecAction::MarkCurrentVmsaOffline {
                cpu: cpu.cpu,
                vmsa_pa: cpu.vmsa_pa,
                online_page_found: cpu.online_page_present,
            })?;
            continue;
        }

        let ap_plan = vmgexit_ap_control_request(
            SVM_VMGEXIT_AP_DESTROY,
            cpu.apic_id,
            inputs.snp_vmpl,
            cpu.vmsa_pa,
            inputs.sev_features,
        )?;
        plan.push(SnpKexecAction::DestroyAp {
            cpu: cpu.cpu,
            apic_id: cpu.apic_id,
            vmsa_pa: cpu.vmsa_pa,
            plan: ap_plan,
        })?;
        snp_kexec_cleanup_vmsa_plan(plan, cpu, inputs.snp_vmpl)?;
    }

    Ok(())
}

fn snp_kexec_range_contains_ghcb(range: SnpKexecMemoryRange, cpus: &[SnpKexecCpu]) -> bool {
    cpus.iter().any(|cpu| {
        cpu.ghcb_pa != 0
            && range.addr <= cpu.ghcb_pa
            && cpu.ghcb_pa < range.addr.wrapping_add(range.size)
    })
}

fn snp_kexec_unshare_all_memory(
    plan: &mut SnpKexecFinishPlan,
    inputs: &SnpKexecFinishInputs<'_>,
) -> Result<(), i32> {
    for range in inputs.direct_map {
        if !range.present || !range.decrypted {
            continue;
        }

        if snp_kexec_range_contains_ghcb(*range, inputs.possible_cpus) {
            continue;
        }

        plan.push(SnpKexecAction::SetPteEncrypted {
            addr: range.addr,
            size: range.size,
            level: range.level,
        })?;
        plan.push(SnpKexecAction::SetMemoryPrivate {
            addr: range.addr,
            npages: range.pages(),
        })?;
    }

    for range in inputs.bss_decrypted {
        if !range.present || !range.decrypted {
            continue;
        }

        plan.push(SnpKexecAction::SetPteEncrypted {
            addr: range.addr,
            size: range.size,
            level: range.level,
        })?;
    }

    plan.push(SnpKexecAction::SetMemoryPrivate {
        addr: inputs.bss_start,
        npages: (inputs.bss_end.wrapping_sub(inputs.bss_start) >> PAGE_SHIFT) as usize,
    })?;
    plan.push(SnpKexecAction::FlushTlbAll)?;

    Ok(())
}

pub fn snp_kexec_finish_plan(inputs: SnpKexecFinishInputs<'_>) -> Result<SnpKexecFinishPlan, i32> {
    if !inputs.sev_snp {
        return Ok(SnpKexecFinishPlan::skipped(SnpKexecFinishSkip::NotSnp));
    }

    if !inputs.kexec_core_enabled {
        return Ok(SnpKexecFinishPlan::skipped(
            SnpKexecFinishSkip::KexecDisabled,
        ));
    }

    let mut plan = SnpKexecFinishPlan::new();
    snp_kexec_shutdown_all_aps(&mut plan, &inputs)?;
    snp_kexec_unshare_all_memory(&mut plan, &inputs)?;

    plan.push(SnpKexecAction::DisableGhcbProtocol)?;

    for cpu in inputs.possible_cpus {
        let size = cpu.ghcb_mapping_size;
        let addr = cpu.ghcb_pa & page_level_mask_from_size(size);
        plan.push(SnpKexecAction::SetPteEncrypted {
            addr,
            size,
            level: cpu.ghcb_mapping_level,
        })?;
        plan.push(SnpKexecAction::SetMemoryPrivate {
            addr,
            npages: (size / PAGE_SIZE) as usize,
        })?;
    }

    Ok(plan)
}

pub fn sev_es_efi_map_ghcbs_cas_plan(
    guest_state_encrypt: bool,
    snp_vmpl: u8,
    encrypted_mask: u64,
    cpus: &[SevEfiMapCpu],
) -> SevEfiMapPlan {
    let mut plan = SevEfiMapPlan::new(!guest_state_encrypt);
    if !guest_state_encrypt {
        return plan;
    }

    let pflags = crate::arch::x86::mm::paging::_PAGE_NX | crate::arch::x86::mm::paging::_PAGE_RW;
    let pflags_enc = pflags | encrypted_mask;

    for cpu in cpus {
        let ghcb_entry = SevEfiMapEntry {
            cpu: cpu.cpu,
            kind: SevEfiMapKind::Ghcb,
            pfn: cpu.ghcb_pa >> PAGE_SHIFT,
            address: cpu.ghcb_pa,
            pages: 1,
            pflags,
        };
        if !plan.push(ghcb_entry) || !cpu.ghcb_map_ok {
            plan.rc = 1;
            return plan;
        }

        if snp_vmpl != 0 {
            if cpu.svsm_caa_pa == 0 {
                plan.rc = 1;
                return plan;
            }

            let ca_entry = SevEfiMapEntry {
                cpu: cpu.cpu,
                kind: SevEfiMapKind::SvsmCaa,
                pfn: cpu.svsm_caa_pa >> PAGE_SHIFT,
                address: cpu.svsm_caa_pa,
                pages: 1,
                pflags: pflags_enc,
            };
            if !plan.push(ca_entry) || !cpu.ca_map_ok {
                plan.rc = 1;
                return plan;
            }
        }
    }

    plan
}

fn ghcb_register_action_response_ok(paddr: u64, response_msr: Option<u64>) -> bool {
    response_msr
        .map(|response| {
            crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_response_matches(
                paddr, response,
            )
        })
        .unwrap_or(false)
}

const fn ghcb_registration_failure_termination() -> GhcbSetupAction {
    GhcbSetupAction::Terminate {
        termination: SevTermination {
            set: SEV_TERM_SET_LINUX,
            reason: GHCB_TERM_REGISTER,
        },
    }
}

pub fn snp_register_per_cpu_ghcb_plan(
    ghcb_paddr: u64,
    response_msr: Option<u64>,
) -> GhcbSetupAction {
    GhcbSetupAction::RegisterPerCpuGhcb {
        paddr: ghcb_paddr,
        request_msr: crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(
            ghcb_paddr,
        ),
        response_ok: ghcb_register_action_response_ok(ghcb_paddr, response_msr),
    }
}

pub fn setup_ghcb_plan(
    guest_state_encrypt: bool,
    sev_snp: bool,
    runtime_vc_handler_active: bool,
    negotiated_protocol_version: Option<u16>,
    per_cpu_ghcb_paddr: u64,
    boot_ghcb_paddr: u64,
    per_cpu_response_msr: Option<u64>,
    boot_response_msr: Option<u64>,
) -> GhcbSetupPlan {
    let mut plan = GhcbSetupPlan::new(!guest_state_encrypt, runtime_vc_handler_active);
    if !guest_state_encrypt {
        return plan;
    }

    if runtime_vc_handler_active {
        if sev_snp {
            let response_ok =
                ghcb_register_action_response_ok(per_cpu_ghcb_paddr, per_cpu_response_msr);
            plan.push(GhcbSetupAction::RegisterPerCpuGhcb {
                paddr: per_cpu_ghcb_paddr,
                request_msr:
                    crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(
                        per_cpu_ghcb_paddr,
                    ),
                response_ok,
            });
            if !response_ok {
                plan.push(ghcb_registration_failure_termination());
                return plan;
            }
        }
        plan.push(GhcbSetupAction::SetGhcbsInitialized(true));
        return plan;
    }

    let Some(protocol_version) = negotiated_protocol_version else {
        plan.push(GhcbSetupAction::Terminate {
            termination: SevTermination {
                set: SEV_TERM_SET_GEN,
                reason: GHCB_SEV_ES_GEN_REQ,
            },
        });
        return plan;
    };

    plan.push(GhcbSetupAction::SetGhcbProtocolVersion {
        version: protocol_version,
    });
    plan.push(GhcbSetupAction::ClearBootGhcbPage {
        paddr: boot_ghcb_paddr,
    });
    plan.push(GhcbSetupAction::SelectBootGhcb {
        paddr: boot_ghcb_paddr,
    });

    if sev_snp {
        let response_ok = ghcb_register_action_response_ok(boot_ghcb_paddr, boot_response_msr);
        plan.push(GhcbSetupAction::RegisterBootGhcb {
            paddr: boot_ghcb_paddr,
            request_msr: crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(
                boot_ghcb_paddr,
            ),
            response_ok,
        });
        if !response_ok {
            plan.push(ghcb_registration_failure_termination());
        }
    }

    plan
}

pub fn sev_es_ap_hlt_loop_plan(exits: &[SevApHltExit]) -> SevApHltLoopPlan {
    let mut plan = SevApHltLoopPlan {
        ghcb_acquired: true,
        ghcb_released: false,
        iterations: 0,
        sw_exit_code: SVM_VMGEXIT_AP_HLT_LOOP,
        sw_exit_info_1: 0,
        sw_exit_info_2: 0,
        woke: false,
        truncated_without_wakeup: false,
    };

    for exit in exits {
        plan.iterations += 1;
        if exit.sw_exit_info_2_valid && exit.sw_exit_info_2 != 0 {
            plan.woke = true;
            plan.ghcb_released = true;
            return plan;
        }
    }

    plan.truncated_without_wakeup = true;
    plan
}

pub fn sev_es_play_dead_plan(exits: &[SevApHltExit]) -> SevPlayDeadPlan {
    let hlt_loop = sev_es_ap_hlt_loop_plan(exits);
    SevPlayDeadPlan {
        play_dead_common_called: true,
        soft_restart_cpu_called: hlt_loop.woke,
        hlt_loop,
    }
}

pub const fn sev_es_setup_play_dead_plan(smp_enabled: bool) -> bool {
    smp_enabled
}

pub const fn alloc_runtime_data_plan(
    input: SevRuntimeCpuInput,
    snp_vmpl: u8,
    boot_svsm_caa_pa: u64,
) -> SevRuntimeCpuPlan {
    let mut plan = SevRuntimeCpuPlan {
        cpu: input.cpu,
        runtime_data_set: false,
        svsm_caa_set: false,
        svsm_caa_uses_boot_page: false,
        svsm_caa_pa: 0,
        ghcb_decrypted: false,
        ghcb_zeroed: false,
        ghcb_active: true,
        backup_ghcb_active: true,
        panic: None,
    };

    if !input.runtime_alloc_ok {
        plan.panic = Some(SevVcInitPanic::RuntimeDataAlloc { cpu: input.cpu });
        return plan;
    }
    plan.runtime_data_set = true;

    if snp_vmpl != 0 {
        if input.cpu == 0 {
            plan.svsm_caa_set = true;
            plan.svsm_caa_uses_boot_page = true;
            plan.svsm_caa_pa = boot_svsm_caa_pa;
        } else if !input.svsm_caa_alloc_ok {
            plan.panic = Some(SevVcInitPanic::SvsmCaaAlloc { cpu: input.cpu });
        } else {
            plan.svsm_caa_set = true;
            plan.svsm_caa_pa = input.svsm_caa_pa;
        }
    }

    plan
}

pub const fn init_ghcb_plan(
    mut cpu_plan: SevRuntimeCpuPlan,
    ghcb_decrypt_rc: i32,
) -> SevRuntimeCpuPlan {
    if cpu_plan.panic.is_some() {
        return cpu_plan;
    }

    if ghcb_decrypt_rc != 0 {
        cpu_plan.panic = Some(SevVcInitPanic::GhcbDecrypt {
            cpu: cpu_plan.cpu,
            rc: ghcb_decrypt_rc,
        });
        return cpu_plan;
    }

    cpu_plan.ghcb_decrypted = true;
    cpu_plan.ghcb_zeroed = true;
    cpu_plan.ghcb_active = false;
    cpu_plan.backup_ghcb_active = false;
    cpu_plan
}

pub fn sev_es_init_vc_handling_plan(inputs: SevVcInitInputs<'_>) -> SevVcHandlingPlan {
    let mut plan = SevVcHandlingPlan::new(!inputs.guest_state_encrypt);
    if !inputs.guest_state_encrypt {
        return plan;
    }

    if !inputs.cpu_features_ok {
        plan.panic = Some(SevVcInitPanic::MissingCpuFeatures);
        return plan;
    }

    if inputs.sev_snp {
        plan.push_action(SevVcInitAction::LoadHvFeatures {
            hv_features: inputs.hv_features,
        });
        if inputs.hv_features & GHCB_HV_FT_SNP == 0 {
            plan.termination = Some(SevTermination {
                set: SEV_TERM_SET_GEN,
                reason: GHCB_SNP_UNSUPPORTED,
            });
            return plan;
        }
    }

    for input in inputs.cpus {
        plan.push_action(SevVcInitAction::AllocRuntimeData { cpu: input.cpu });
        let mut cpu_plan =
            alloc_runtime_data_plan(*input, inputs.snp_vmpl, inputs.boot_svsm_caa_pa);
        if let Some(panic) = cpu_plan.panic {
            plan.panic = Some(panic);
            plan.push_cpu(cpu_plan);
            return plan;
        }

        plan.push_action(SevVcInitAction::InitGhcb { cpu: input.cpu });
        cpu_plan = init_ghcb_plan(cpu_plan, input.ghcb_decrypt_rc);
        if let Some(panic) = cpu_plan.panic {
            plan.panic = Some(panic);
            plan.push_cpu(cpu_plan);
            return plan;
        }
        plan.push_cpu(cpu_plan);
    }

    if inputs.snp_vmpl != 0 {
        plan.use_cas = true;
        plan.push_action(SevVcInitAction::EnableSvsmCas);
    }

    if sev_es_setup_play_dead_plan(inputs.smp_enabled) {
        plan.play_dead_setup = true;
        plan.push_action(SevVcInitAction::SetupPlayDead);
    }

    plan.runtime_vc_handler_set = true;
    plan.push_action(SevVcInitAction::SetRuntimeVcHandler);

    plan
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnpPageOp {
    Private,
    Shared,
}

impl SnpPageOp {
    pub const fn psc_op(self) -> u32 {
        match self {
            Self::Private => VMGEXIT_PSC_OP_PRIVATE,
            Self::Shared => VMGEXIT_PSC_OP_SHARED,
        }
    }

    pub const fn msr_state(self) -> SnpPageState {
        match self {
            Self::Private => SnpPageState::Private,
            Self::Shared => SnpPageState::Shared,
        }
    }

    pub const fn pvalidate_before(self) -> bool {
        matches!(self, Self::Shared)
    }

    pub const fn pvalidate_after(self) -> bool {
        matches!(self, Self::Private)
    }
}

pub fn build_psc_desc(start_gfn: u64, pages: usize, op: SnpPageOp) -> Result<PscDesc, i32> {
    if pages > VMGEXIT_PSC_MAX_ENTRY {
        return Err(EOPNOTSUPP);
    }
    let mut desc = PscDesc::empty();
    let mut i = 0;
    while i < pages {
        desc.push(PscEntry::new(start_gfn + i as u64, op.psc_op()))?;
        i += 1;
    }
    Ok(desc)
}

fn psc_pfn_for_vaddr(vaddr: u64) -> u64 {
    if let Some(phys) = crate::arch::x86::mm::paging::virt_to_phys(vaddr) {
        phys >> PAGE_SHIFT
    } else if vaddr >= crate::arch::x86::mm::paging::PAGE_OFFSET {
        (vaddr - crate::arch::x86::mm::paging::PAGE_OFFSET) >> PAGE_SHIFT
    } else {
        vaddr >> PAGE_SHIFT
    }
}

fn psc_paddr_for_vaddr(vaddr: u64) -> u64 {
    psc_pfn_for_vaddr(vaddr) << PAGE_SHIFT
}

fn psc_range(vaddr: u64, npages: usize) -> Result<(u64, u64), i32> {
    let bytes = (npages as u64).wrapping_mul(PAGE_SIZE);
    let start = vaddr & !(PAGE_SIZE - 1);
    let end = start.wrapping_add(bytes);
    Ok((start, end))
}

pub fn set_pages_state_plan(
    vaddr: u64,
    npages: usize,
    op: SnpPageOp,
    boot_ghcb_available: bool,
    svsm_caa: u64,
    svsm_caa_pa: u64,
) -> Result<SnpSetPagesStatePath, i32> {
    if !boot_ghcb_available {
        return Ok(SnpSetPagesStatePath::EarlyMsr(SnpEarlySetPagesStatePlan {
            vaddr: vaddr & PAGE_MASK,
            paddr: psc_paddr_for_vaddr(vaddr) & PAGE_MASK,
            npages,
            op: op.psc_op(),
            ca: svsm_caa,
            caa_pa: svsm_caa_pa,
        }));
    }

    Ok(SnpSetPagesStatePath::Ghcb(build_psc_plan(
        vaddr, npages, op,
    )?))
}

pub fn snp_set_memory_shared_linux_plan(
    state: CcPlatformState,
    vaddr: u64,
    npages: usize,
    boot_ghcb_available: bool,
    svsm_caa: u64,
    svsm_caa_pa: u64,
) -> Result<SnpMemoryStatePlan, i32> {
    if !cc_platform_has_state(state, CcAttr::GuestSevSnp) {
        return Ok(SnpMemoryStatePlan::SkippedNotSnp);
    }

    Ok(SnpMemoryStatePlan::SetPages(set_pages_state_plan(
        vaddr,
        npages,
        SnpPageOp::Shared,
        boot_ghcb_available,
        svsm_caa,
        svsm_caa_pa,
    )?))
}

pub fn snp_set_memory_private_linux_plan(
    state: CcPlatformState,
    vaddr: u64,
    npages: usize,
    boot_ghcb_available: bool,
    svsm_caa: u64,
    svsm_caa_pa: u64,
) -> Result<SnpMemoryStatePlan, i32> {
    if !cc_platform_has_state(state, CcAttr::GuestSevSnp) {
        return Ok(SnpMemoryStatePlan::SkippedNotSnp);
    }

    Ok(SnpMemoryStatePlan::SetPages(set_pages_state_plan(
        vaddr,
        npages,
        SnpPageOp::Private,
        boot_ghcb_available,
        svsm_caa,
        svsm_caa_pa,
    )?))
}

fn build_psc_desc_batch(
    mut cur: u64,
    end: u64,
    op: SnpPageOp,
) -> Result<(PscDesc, u64, usize), i32> {
    let mut desc = PscDesc::empty();
    let mut pages = 0usize;

    while cur < end && desc.count < VMGEXIT_PSC_MAX_ENTRY {
        let remaining = end - cur;
        let pfn = psc_pfn_for_vaddr(cur);
        let phys = pfn << PAGE_SHIFT;
        let use_2m =
            cur & (PMD_SIZE - 1) == 0 && phys & (PMD_SIZE - 1) == 0 && remaining >= PMD_SIZE;
        let (pagesize, step, covered_pages) = if use_2m {
            (RMP_PG_SIZE_2M, PMD_SIZE, PMD_PAGES)
        } else {
            (RMP_PG_SIZE_4K, PAGE_SIZE, 1)
        };

        desc.push(PscEntry {
            gfn: pfn,
            pagesize,
            operation: op.psc_op(),
            current_page: true,
        })?;
        pages += covered_pages;
        cur += step;
    }

    Ok((desc, cur, pages))
}

fn build_psc_plan(vaddr: u64, npages: usize, op: SnpPageOp) -> Result<PscBatchPlan, i32> {
    let (start, end) = psc_range(vaddr, npages)?;
    let mut first = PscDesc::empty();
    let mut cur = start;
    let mut desc_count = 0usize;
    let mut entry_count = 0usize;
    let mut pages = 0usize;

    while cur < end {
        let (desc, next, batch_pages) = build_psc_desc_batch(cur, end, op)?;
        if desc_count == 0 {
            first = desc;
        }
        desc_count += 1;
        entry_count += desc.count;
        pages += batch_pages;
        cur = next;
    }

    Ok(PscBatchPlan {
        first,
        desc_count,
        entry_count,
        pages,
        pvalidate_before: op.pvalidate_before(),
        pvalidate_after: op.pvalidate_after(),
    })
}

fn issue_psc_batches(vaddr: u64, npages: usize, op: SnpPageOp) -> Result<(), i32> {
    let (start, end) = psc_range(vaddr, npages)?;
    let mut cur = start;
    let mut desc_count = 0usize;

    while cur < end {
        let (desc, next, _) = build_psc_desc_batch(cur, end, op)?;
        issue_psc_desc(&desc, op)?;
        cur = next;
        desc_count += 1;
    }

    Ok(())
}

fn issue_psc_desc(desc: &PscDesc, op: SnpPageOp) -> Result<(), i32> {
    if op.pvalidate_before() {
        pvalidate_pages(desc, op, true)?;
    }
    vmgexit_psc(desc, op)?;
    if op.pvalidate_after() {
        pvalidate_pages(desc, op, false)?;
    }
    Ok(())
}

fn pvalidate_pages(desc: &PscDesc, op: SnpPageOp, before: bool) -> Result<(), i32> {
    #[cfg(test)]
    record_psc_issue_event(
        if before {
            PSC_EVENT_PVALIDATE_BEFORE
        } else {
            PSC_EVENT_PVALIDATE_AFTER
        },
        op.psc_op(),
        desc.count,
    );

    if snp_vmpl() != 0 {
        return crate::arch::x86::coco::sev::svsm::svsm_pval_pages(desc);
    }

    pvalidate_desc_entries(desc, op)
}

pub fn pvalidate_pages_cache_evict_plan(
    desc: &PscDesc,
    coherency_sfw_no: bool,
) -> Result<SnpPvalidateCacheEvictPlan, i32> {
    let mut plan = SnpPvalidateCacheEvictPlan::empty(coherency_sfw_no);
    if coherency_sfw_no {
        return Ok(plan);
    }

    let mut i = 0usize;
    while i < desc.count {
        let Some(entry) = desc.entries[i] else {
            return Err(EOPNOTSUPP);
        };

        if entry.operation == VMGEXIT_PSC_OP_PRIVATE {
            plan.entries[plan.count] = Some(SnpPvalidateCacheEvictEntry {
                pfn: entry.gfn,
                vaddr: crate::arch::x86::mm::paging::pfn_to_virt(entry.gfn as usize) as u64,
                pages: if entry.pagesize == RMP_PG_SIZE_2M {
                    PMD_PAGES
                } else {
                    1
                },
            });
            plan.count += 1;
        }

        i += 1;
    }

    Ok(plan)
}

fn pvalidate_desc_entries(desc: &PscDesc, op: SnpPageOp) -> Result<(), i32> {
    let validate = matches!(op, SnpPageOp::Private);
    let mut i = 0;

    while i < desc.count {
        let Some(entry) = desc.entries[i] else {
            return Err(EOPNOTSUPP);
        };
        match pvalidate_entry(entry.gfn, entry.pagesize, validate) {
            Ok(()) => {}
            Err(PVALIDATE_FAIL_SIZEMISMATCH) if entry.pagesize == RMP_PG_SIZE_2M => {
                let mut pfn = entry.gfn;
                let pfn_end = entry.gfn + PMD_PAGES as u64;
                while pfn < pfn_end {
                    pvalidate_entry(pfn, RMP_PG_SIZE_4K, validate)?;
                    pfn += 1;
                }
            }
            Err(err) => return Err(err),
        }
        i += 1;
    }

    Ok(())
}

fn pvalidate_entry(pfn: u64, pagesize: u32, validate: bool) -> Result<(), i32> {
    let vaddr = crate::arch::x86::mm::paging::pfn_to_virt(pfn as usize) as u64;
    let rc = unsafe { pvalidate_instruction(vaddr, pfn, pagesize, validate) };
    if rc == 0 { Ok(()) } else { Err(rc) }
}

#[cfg(test)]
unsafe fn pvalidate_instruction(_vaddr: u64, pfn: u64, pagesize: u32, validate: bool) -> i32 {
    record_pvalidate_call(pfn, pagesize, validate);
    if pagesize == RMP_PG_SIZE_2M && TEST_PVALIDATE_2M_SIZE_MISMATCH.load(Ordering::Acquire) {
        PVALIDATE_FAIL_SIZEMISMATCH
    } else {
        0
    }
}

#[cfg(not(test))]
unsafe fn pvalidate_instruction(vaddr: u64, _pfn: u64, pagesize: u32, validate: bool) -> i32 {
    let mut rax = vaddr;
    let no_rmpupdate: u8;
    let rmp_psize = (pagesize == RMP_PG_SIZE_2M) as u64;
    let validate = validate as u64;

    unsafe {
        core::arch::asm!(
            ".byte 0xF2, 0x0F, 0x01, 0xFF",
            "setc {no_rmpupdate}",
            inlateout("rax") rax,
            in("rcx") rmp_psize,
            in("rdx") validate,
            no_rmpupdate = lateout(reg_byte) no_rmpupdate,
            options(nostack)
        );
    }

    if no_rmpupdate != 0 {
        PVALIDATE_FAIL_NOUPDATE
    } else {
        rax as i32
    }
}

fn vmgexit_psc(desc: &PscDesc, op: SnpPageOp) -> Result<(), i32> {
    #[cfg(test)]
    record_psc_issue_event(PSC_EVENT_VMGEXIT, op.psc_op(), desc.count);

    let mut buffer = GhcbPscSharedBuffer::from_desc(desc);
    ghcb_psc_exchange(&mut buffer)
}

fn ghcb_psc_exchange(buffer: &mut GhcbPscSharedBuffer) -> Result<(), i32> {
    if buffer.desc.count == 0 {
        return Ok(());
    }
    let mut cur_entry = buffer.cur_entry;
    let end_entry = buffer.end_entry;

    while buffer.cur_entry <= buffer.end_entry {
        let sw_scratch = ghcb_psc_shared_buffer_scratch(buffer);
        #[cfg(test)]
        record_ghcb_psc_log(buffer, sw_scratch);

        match ghcb_psc_hv_call(buffer) {
            Ok(()) => {}
            Err(err) => return Err(err),
        }

        if buffer.sw_exit_info_2 != 0 || buffer.reserved {
            return Err(EOPNOTSUPP);
        }
        if buffer.end_entry > end_entry || cur_entry > buffer.cur_entry {
            return Err(EOPNOTSUPP);
        }
        cur_entry = buffer.cur_entry;
    }

    Ok(())
}

fn ghcb_psc_shared_buffer_scratch(buffer: &GhcbPscSharedBuffer) -> u64 {
    core::ptr::addr_of!(*buffer) as u64
}

fn ghcb_psc_hv_call(buffer: &mut GhcbPscSharedBuffer) -> Result<(), i32> {
    let mut ghcb = crate::arch::x86::coco::sev::vc_shared::Ghcb::default();
    let mut ctxt = crate::arch::x86::coco::sev::vc_shared::vc_init_em_ctxt(
        crate::arch::x86::coco::sev::vc_shared::SVM_VMGEXIT_PSC,
        0,
        0,
    );
    crate::arch::x86::coco::sev::vc_shared::ghcb_set_sw_scratch(
        &mut ghcb,
        ghcb_psc_shared_buffer_scratch(buffer),
    );

    match crate::arch::x86::coco::sev::vc_shared::sev_es_ghcb_hv_call(
        &mut ghcb,
        &mut ctxt,
        crate::arch::x86::coco::sev::vc_shared::SVM_VMGEXIT_PSC,
        0,
        0,
    ) {
        crate::arch::x86::coco::sev::vc_shared::EsResult::Continue => {
            ghcb_psc_apply_hypervisor_progress(buffer);
            Ok(())
        }
        _ => Err(EOPNOTSUPP),
    }
}

fn ghcb_psc_apply_hypervisor_progress(buffer: &mut GhcbPscSharedBuffer) {
    #[cfg(test)]
    match TEST_GHCB_PSC_MODE.load(Ordering::Acquire) {
        TEST_GHCB_PSC_EXIT_INFO => {
            buffer.sw_exit_info_2 = 1;
            return;
        }
        TEST_GHCB_PSC_RESERVED => {
            buffer.reserved = true;
            return;
        }
        TEST_GHCB_PSC_END_GROWTH => {
            buffer.end_entry = buffer.end_entry.saturating_add(1);
            return;
        }
        _ => {}
    }

    buffer.cur_entry = buffer.end_entry.saturating_add(1);
}

#[cfg(test)]
fn record_psc_issue_event(kind: u8, op: u32, count: usize) {
    let idx = PSC_ISSUE_LOG_LEN.fetch_add(1, Ordering::AcqRel);
    if idx < PSC_ISSUE_LOG_CAP {
        PSC_ISSUE_LOG.lock()[idx] = (kind, op, count);
    }
}

#[cfg(test)]
fn reset_psc_issue_log() {
    PSC_ISSUE_LOG_LEN.store(0, Ordering::Release);
    *PSC_ISSUE_LOG.lock() = [(0, 0, 0); PSC_ISSUE_LOG_CAP];
}

#[cfg(test)]
fn psc_issue_log() -> (usize, [(u8, u32, usize); PSC_ISSUE_LOG_CAP]) {
    (
        PSC_ISSUE_LOG_LEN
            .load(Ordering::Acquire)
            .min(PSC_ISSUE_LOG_CAP),
        *PSC_ISSUE_LOG.lock(),
    )
}

#[cfg(test)]
fn record_pvalidate_call(pfn: u64, pagesize: u32, validate: bool) {
    let idx = PVALIDATE_LOG_LEN.fetch_add(1, Ordering::AcqRel);
    if idx < PVALIDATE_LOG_CAP {
        PVALIDATE_LOG.lock()[idx] = (pfn, pagesize, validate);
    }
}

#[cfg(test)]
fn reset_pvalidate_log() {
    PVALIDATE_LOG_LEN.store(0, Ordering::Release);
    *PVALIDATE_LOG.lock() = [(0, 0, false); PVALIDATE_LOG_CAP];
}

#[cfg(test)]
fn pvalidate_log() -> (usize, [(u64, u32, bool); PVALIDATE_LOG_CAP]) {
    (
        PVALIDATE_LOG_LEN.load(Ordering::Acquire),
        *PVALIDATE_LOG.lock(),
    )
}

#[cfg(test)]
fn set_test_pvalidate_2m_size_mismatch(enabled: bool) {
    TEST_PVALIDATE_2M_SIZE_MISMATCH.store(enabled, Ordering::Release);
}

#[cfg(test)]
fn record_ghcb_psc_log(buffer: &GhcbPscSharedBuffer, sw_scratch: u64) {
    let idx = GHCB_PSC_LOG_LEN.fetch_add(1, Ordering::AcqRel);
    if idx < GHCB_PSC_LOG_CAP {
        GHCB_PSC_LOG.lock()[idx] = GhcbPscLogEntry {
            copied_count: buffer.desc.count,
            cur_entry: buffer.cur_entry,
            end_entry: buffer.end_entry,
            sw_scratch,
        };
    }
}

#[cfg(test)]
fn reset_ghcb_psc_log() {
    GHCB_PSC_LOG_LEN.store(0, Ordering::Release);
    *GHCB_PSC_LOG.lock() = [GhcbPscLogEntry::empty(); GHCB_PSC_LOG_CAP];
}

#[cfg(test)]
fn ghcb_psc_log() -> (usize, [GhcbPscLogEntry; GHCB_PSC_LOG_CAP]) {
    (
        GHCB_PSC_LOG_LEN
            .load(Ordering::Acquire)
            .min(GHCB_PSC_LOG_CAP),
        *GHCB_PSC_LOG.lock(),
    )
}

#[cfg(test)]
fn set_test_ghcb_psc_mode(mode: u8) {
    TEST_GHCB_PSC_MODE.store(mode, Ordering::Release);
}

pub fn snp_set_memory_shared(vaddr: u64, npages: usize) -> Result<PscBatchPlan, i32> {
    let plan = build_psc_plan(vaddr, npages, SnpPageOp::Shared)?;
    issue_psc_batches(vaddr, npages, SnpPageOp::Shared)?;
    Ok(plan)
}

pub fn snp_set_memory_private(vaddr: u64, npages: usize) -> Result<PscBatchPlan, i32> {
    let plan = build_psc_plan(vaddr, npages, SnpPageOp::Private)?;
    issue_psc_batches(vaddr, npages, SnpPageOp::Private)?;
    Ok(plan)
}

pub fn snp_accept_memory_plan(start: u64, end: u64) -> Result<SnpAcceptMemoryPlan, i32> {
    let npages = (end.wrapping_sub(start) >> PAGE_SHIFT) as usize;
    let vaddr = crate::arch::x86::mm::paging::PAGE_OFFSET.wrapping_add(start);
    let private_plan = build_psc_plan(vaddr, npages, SnpPageOp::Private)?;

    Ok(SnpAcceptMemoryPlan {
        vaddr,
        npages,
        private_plan,
    })
}

pub fn snp_accept_memory(start: u64, end: u64) -> Result<usize, i32> {
    let plan = snp_accept_memory_plan(start, end)?;
    issue_psc_batches(plan.vaddr, plan.npages, SnpPageOp::Private)?;
    Ok(plan.npages)
}

pub fn vmgexit_ap_control(event: u64, apic_id: u32) -> Result<(u64, u32), i32> {
    let plan = vmgexit_ap_control_request(event, apic_id, snp_vmpl(), 0, 0)?;
    Ok((plan.event, plan.apic_id))
}

fn savic_msr(reg: u32) -> u64 {
    APIC_BASE_MSR + ((reg as u64) >> 4)
}

fn savic_failure_termination(
    result: crate::arch::x86::coco::sev::vc_shared::EsResult,
) -> Option<SevTermination> {
    if result == crate::arch::x86::coco::sev::vc_shared::EsResult::Continue {
        None
    } else {
        Some(SevTermination {
            set: SEV_TERM_SET_LINUX,
            reason: GHCB_TERM_SAVIC_FAIL,
        })
    }
}

pub fn savic_ghcb_msr_read_plan(
    reg: u32,
    handler_result: crate::arch::x86::coco::sev::vc_shared::EsResult,
    returned_ax: u32,
    returned_dx: u32,
) -> (SnpSavicMsrPlan, Option<u64>) {
    let value = (returned_ax as u64) | ((returned_dx as u64) << 32);
    let termination = savic_failure_termination(handler_result);
    (
        SnpSavicMsrPlan {
            reg,
            msr: savic_msr(reg),
            write: false,
            cx: savic_msr(reg),
            ax: returned_ax as u64,
            dx: returned_dx as u64,
            handler_result,
            termination,
        },
        if termination.is_none() {
            Some(value)
        } else {
            None
        },
    )
}

pub fn savic_ghcb_msr_write_plan(
    reg: u32,
    value: u64,
    handler_result: crate::arch::x86::coco::sev::vc_shared::EsResult,
) -> SnpSavicMsrPlan {
    SnpSavicMsrPlan {
        reg,
        msr: savic_msr(reg),
        write: true,
        cx: savic_msr(reg),
        ax: value & 0xffff_ffff,
        dx: value >> 32,
        handler_result,
        termination: savic_failure_termination(handler_result),
    }
}

pub fn savic_register_gpa_plan(
    gpa: u64,
    hv_result: crate::arch::x86::coco::sev::vc_shared::EsResult,
) -> SnpSavicGpaPlan {
    SnpSavicGpaPlan {
        operation: SVM_VMGEXIT_SAVIC_REGISTER_GPA,
        gpa_argument: Some(gpa),
        sw_exit_code: SVM_VMGEXIT_SAVIC,
        sw_exit_info_1: SVM_VMGEXIT_SAVIC_REGISTER_GPA,
        sw_exit_info_2: 0,
        rax: SVM_VMGEXIT_SAVIC_SELF_GPA,
        rbx: Some(gpa),
        hv_result,
        returned_gpa: None,
    }
}

pub fn savic_unregister_gpa_plan(
    copy_gpa: bool,
    returned_rbx: u64,
    hv_result: crate::arch::x86::coco::sev::vc_shared::EsResult,
) -> SnpSavicGpaPlan {
    SnpSavicGpaPlan {
        operation: SVM_VMGEXIT_SAVIC_UNREGISTER_GPA,
        gpa_argument: None,
        sw_exit_code: SVM_VMGEXIT_SAVIC,
        sw_exit_info_1: SVM_VMGEXIT_SAVIC_UNREGISTER_GPA,
        sw_exit_info_2: 0,
        rax: SVM_VMGEXIT_SAVIC_SELF_GPA,
        rbx: None,
        hv_result,
        returned_gpa: if copy_gpa
            && hv_result == crate::arch::x86::coco::sev::vc_shared::EsResult::Continue
        {
            Some(returned_rbx)
        } else {
            None
        },
    }
}

pub fn savic_register_gpa(gpa: u64) -> Result<(u64, u64), i32> {
    let plan = savic_register_gpa_plan(
        gpa,
        crate::arch::x86::coco::sev::vc_shared::EsResult::Continue,
    );
    Ok((plan.operation, plan.gpa_argument.unwrap_or(0)))
}

pub fn savic_unregister_gpa(gpa: u64) -> Result<(u64, u64), i32> {
    let plan = savic_unregister_gpa_plan(
        true,
        gpa,
        crate::arch::x86::coco::sev::vc_shared::EsResult::Continue,
    );
    Ok((plan.operation, plan.returned_gpa.unwrap_or(0)))
}

pub fn sev_status_feature_name(bit: usize) -> Option<&'static str> {
    if bit < MSR_AMD64_SNP_RESV_BIT {
        SEV_STATUS_FEATURE_NAMES[bit]
    } else {
        None
    }
}

pub fn sev_show_status_report(sev_status: u64) -> SevStatusReport {
    let mut report = SevStatusReport::empty();

    for bit in 0..MSR_AMD64_SNP_RESV_BIT {
        if sev_status & (1u64 << bit) == 0 {
            continue;
        }

        if let Some(name) = sev_status_feature_name(bit) {
            report.push(name);
        }
    }

    report
}

pub const fn snp_dmi_setup_plan(efi_config_tables_enabled: bool) -> bool {
    efi_config_tables_enabled
}

pub const fn report_snp_info_plan(
    cpuid_table_count: u32,
    sev_cfg_debug: bool,
    sev_snp: bool,
    vmpl: u8,
) -> SnpInfoReport {
    SnpInfoReport {
        rc: 0,
        cpuid_table_count,
        announced_cpuid_table: cpuid_table_count != 0,
        dumped_cpuid_table: cpuid_table_count != 0 && sev_cfg_debug,
        sev_snp,
        vmpl,
        announced_vmpl: sev_snp,
    }
}

pub const fn snp_init_platform_device_plan(
    sev_snp: bool,
    sev_guest_register_ok: bool,
    vtpm_probe: bool,
    tpm_svsm_register_ok: bool,
) -> SnpPlatformDevicePlan {
    let mut plan = SnpPlatformDevicePlan::new(0);

    if !sev_snp {
        plan.rc = -ENODEV;
        return plan;
    }

    if !sev_guest_register_ok {
        plan.rc = -ENODEV;
        return plan;
    }
    plan.sev_guest_registered = true;

    if vtpm_probe {
        plan.vtpm_probe = true;
        if !tpm_svsm_register_ok {
            plan.rc = -ENODEV;
            return plan;
        }
        plan.tpm_svsm_registered = true;
    }

    plan.info_printed = true;
    plan
}

pub const fn vmpl_show_bytes(vmpl: u8) -> VmplShow {
    let mut show = VmplShow {
        bytes: [0; 4],
        len: 0,
    };

    if vmpl >= 100 {
        show.bytes[0] = b'0' + (vmpl / 100);
        show.bytes[1] = b'0' + ((vmpl / 10) % 10);
        show.bytes[2] = b'0' + (vmpl % 10);
        show.bytes[3] = b'\n';
        show.len = 4;
    } else if vmpl >= 10 {
        show.bytes[0] = b'0' + (vmpl / 10);
        show.bytes[1] = b'0' + (vmpl % 10);
        show.bytes[2] = b'\n';
        show.len = 3;
    } else {
        show.bytes[0] = b'0' + vmpl;
        show.bytes[1] = b'\n';
        show.len = 2;
    }

    show
}

pub const fn sev_sysfs_init_plan(
    sev_snp: bool,
    dev_root_available: bool,
    kobject_create_ok: bool,
    sysfs_create_group_rc: i32,
) -> SevSysfsInitPlan {
    let mut plan = SevSysfsInitPlan::new(0);

    if !sev_snp {
        plan.rc = -ENODEV;
        return plan;
    }

    if !dev_root_available {
        plan.rc = -ENODEV;
        return plan;
    }
    plan.got_dev_root = true;
    plan.put_dev_root = true;

    if !kobject_create_ok {
        plan.rc = -ENOMEM;
        return plan;
    }
    plan.created_kobject = true;

    if sysfs_create_group_rc != 0 {
        plan.rc = sysfs_create_group_rc;
        plan.put_kobject_on_group_error = true;
        return plan;
    }

    plan.created_group = true;
    plan
}

pub fn setup_ghcb(hv_features: u64, secrets_pa: u64) {
    SEV_HV_FEATURES.store(hv_features, Ordering::Release);
    SEV_SECRETS_PA.store(secrets_pa, Ordering::Release);
}

pub fn get_hv_features() -> u64 {
    SEV_HV_FEATURES.load(Ordering::Acquire)
}

pub fn sev_secrets_pa() -> u64 {
    SEV_SECRETS_PA.load(Ordering::Acquire)
}

pub fn publish_secure_tsc(scale: u64, offset: u64, freq_khz: u64) {
    SNP_TSC_SCALE.store(scale, Ordering::Release);
    SNP_TSC_OFFSET.store(offset, Ordering::Release);
    SNP_TSC_FREQ_KHZ.store(freq_khz, Ordering::Release);
}

fn publish_secure_tsc_scale_offset(scale: u64, offset: u64) {
    SNP_TSC_SCALE.store(scale, Ordering::Release);
    SNP_TSC_OFFSET.store(offset, Ordering::Release);
}

pub fn publish_snp_vmpl(vmpl: u8) {
    SNP_VMPL.store(vmpl, Ordering::Release);
}

pub fn snp_vmpl() -> u8 {
    SNP_VMPL.load(Ordering::Acquire)
}

pub fn snp_secure_tsc_info() -> (u64, u64, u64) {
    (
        SNP_TSC_SCALE.load(Ordering::Acquire),
        SNP_TSC_OFFSET.load(Ordering::Acquire),
        SNP_TSC_FREQ_KHZ.load(Ordering::Acquire),
    )
}

pub const fn snp_scale_tsc_freq(freq: u64, factor: u32) -> u64 {
    freq - (freq * factor as u64) / 100_000
}

pub fn securetsc_get_tsc_khz() -> u64 {
    SNP_TSC_FREQ_KHZ.load(Ordering::Acquire)
}

pub const fn linux_page_align_size(size: usize) -> usize {
    let mask = PAGE_SIZE as usize - 1;
    (size + mask) & !mask
}

pub const fn linux_page_count(size: usize) -> usize {
    linux_page_align_size(size) >> PAGE_SHIFT
}

pub const fn linux_get_order(size: usize) -> u32 {
    let pages = linux_page_count(size);
    if pages <= 1 {
        return 0;
    }

    let mut order = 0;
    let mut capacity = 1usize;
    while capacity < pages {
        capacity <<= 1;
        order += 1;
    }
    order
}

pub const fn free_shared_pages_plan(
    buf_present: bool,
    size: usize,
    set_memory_encrypted_rc: i32,
) -> FreeSharedPagesPlan {
    let mut plan = FreeSharedPagesPlan {
        size,
        npages: linux_page_count(size),
        order: linux_get_order(size),
        buf_present,
        set_memory_encrypted: false,
        set_memory_encrypted_rc: 0,
        freed_pages: false,
        leaked_on_encrypt_error: false,
    };

    if !buf_present {
        return plan;
    }

    plan.set_memory_encrypted = true;
    plan.set_memory_encrypted_rc = set_memory_encrypted_rc;

    if set_memory_encrypted_rc != 0 {
        plan.leaked_on_encrypt_error = true;
        return plan;
    }

    plan.freed_pages = true;
    plan
}

pub const fn alloc_shared_pages_plan(
    size: usize,
    page_alloc_ok: bool,
    set_memory_decrypted_rc: i32,
) -> AllocSharedPagesPlan {
    let mut plan = AllocSharedPagesPlan {
        size,
        npages: linux_page_count(size),
        order: linux_get_order(size),
        page_allocated: page_alloc_ok,
        set_memory_decrypted: false,
        set_memory_decrypted_rc: 0,
        freed_pages_on_decrypt_error: false,
        returned_page: false,
    };

    if !page_alloc_ok {
        return plan;
    }

    plan.set_memory_decrypted = true;
    plan.set_memory_decrypted_rc = set_memory_decrypted_rc;

    if set_memory_decrypted_rc != 0 {
        plan.freed_pages_on_decrypt_error = true;
        return plan;
    }

    plan.returned_page = true;
    plan
}

pub fn snp_msg_alloc_plan(inputs: SnpMsgAllocInputs) -> SnpMsgAllocPlan {
    let mut plan = SnpMsgAllocPlan::new();

    plan.push(SnpMsgAllocAction::KzallocDesc);
    if !inputs.mdesc_alloc_ok {
        plan.rc = -ENOMEM;
        return plan;
    }

    plan.push(SnpMsgAllocAction::IoremapSecrets);
    if !inputs.secrets_ioremap_ok {
        plan.push(SnpMsgAllocAction::KfreeDesc);
        plan.rc = -ENOMEM;
        return plan;
    }

    plan.push(SnpMsgAllocAction::AllocRequestShared);
    let request = alloc_shared_pages_plan(
        SNP_GUEST_MSG_SIZE,
        inputs.request_page_alloc_ok,
        inputs.request_set_decrypted_rc,
    );
    plan.request_shared = Some(request);
    if !request.returned_page {
        plan.push(SnpMsgAllocAction::IounmapSecrets);
        plan.push(SnpMsgAllocAction::KfreeDesc);
        plan.rc = -ENOMEM;
        return plan;
    }

    plan.push(SnpMsgAllocAction::AllocResponseShared);
    let response = alloc_shared_pages_plan(
        SNP_GUEST_MSG_SIZE,
        inputs.response_page_alloc_ok,
        inputs.response_set_decrypted_rc,
    );
    plan.response_shared = Some(response);
    if !response.returned_page {
        plan.push(SnpMsgAllocAction::FreeRequestShared);
        plan.push(SnpMsgAllocAction::IounmapSecrets);
        plan.push(SnpMsgAllocAction::KfreeDesc);
        plan.rc = -ENOMEM;
        return plan;
    }

    plan.success = true;
    plan
}

pub fn snp_msg_free_plan(
    desc_present: bool,
    ctx_present: bool,
    response_present: bool,
    request_present: bool,
    response_set_encrypted_rc: i32,
    request_set_encrypted_rc: i32,
) -> SnpMsgFreePlan {
    let mut plan = SnpMsgFreePlan::new(desc_present, ctx_present);

    if !desc_present {
        return plan;
    }

    plan.push(SnpMsgFreeAction::KfreeCtx);
    plan.push(SnpMsgFreeAction::FreeResponseShared);
    plan.response_shared = Some(free_shared_pages_plan(
        response_present,
        SNP_GUEST_MSG_SIZE,
        response_set_encrypted_rc,
    ));
    plan.push(SnpMsgFreeAction::FreeRequestShared);
    plan.request_shared = Some(free_shared_pages_plan(
        request_present,
        SNP_GUEST_MSG_SIZE,
        request_set_encrypted_rc,
    ));
    plan.push(SnpMsgFreeAction::IounmapSecrets);
    plan.push(SnpMsgFreeAction::KfreeSensitiveDesc);

    plan
}

pub fn get_vmpck_from_secrets(secrets: &SnpSecretsPage, id: i32) -> Option<SnpVmpckSelection> {
    let index = usize::try_from(id).ok()?;
    let key = secrets.vmpck(index)?;
    Some(SnpVmpckSelection {
        key,
        seqno_index: index,
    })
}

pub const fn snp_init_crypto(key_len: usize, crypto_available: bool) -> Option<SnpAesGcmCtx> {
    if crypto_available {
        Some(SnpAesGcmCtx {
            key_len,
            authtag_len: AUTHTAG_LEN,
        })
    } else {
        None
    }
}

pub fn snp_msg_init_desc(
    mdesc: &mut SnpMsgDesc,
    mut vmpck_id: i32,
    current_vmpl: u8,
    crypto_available: bool,
) -> Result<(), i32> {
    if vmpck_id == -1 {
        vmpck_id = current_vmpl as i32;
    }

    let selection = get_vmpck_from_secrets(&mdesc.secrets, vmpck_id).ok_or(EINVAL)?;
    mdesc.vmpck = Some(selection.key);
    mdesc.os_area_msg_seqno = Some(selection.seqno_index);

    if !selection.key.iter().any(|byte| *byte != 0) {
        return Err(EINVAL);
    }

    mdesc.vmpck_id = vmpck_id;
    mdesc.ctx = snp_init_crypto(VMPCK_KEY_LEN, crypto_available);
    if mdesc.ctx.is_none() {
        return Err(ENOMEM);
    }

    Ok(())
}

pub fn snp_msg_init_with_secrets(
    secrets: SnpSecretsPage,
    vmpck_id: i32,
    current_vmpl: u8,
    crypto_available: bool,
) -> Result<SnpMsgDesc, i32> {
    let mut desc = SnpMsgDesc::new(secrets);
    snp_msg_init_desc(&mut desc, vmpck_id, current_vmpl, crypto_available)?;
    Ok(desc)
}

pub fn snp_disable_vmpck_desc(mdesc: &mut SnpMsgDesc) -> Result<(), i32> {
    let index = usize::try_from(mdesc.vmpck_id).map_err(|_| EINVAL)?;
    mdesc.secrets.zero_vmpck(index)?;
    mdesc.vmpck = None;
    Ok(())
}

pub fn __snp_get_msg_seqno_desc(mdesc: &SnpMsgDesc) -> Result<u64, i32> {
    let index = mdesc.os_area_msg_seqno.ok_or(EINVAL)?;
    let count = mdesc.secrets.os_area.msg_seqno(index).ok_or(EINVAL)?;
    Ok(count as u64 + 1)
}

pub fn snp_get_msg_seqno_desc(mdesc: &SnpMsgDesc) -> Result<u64, i32> {
    let count = __snp_get_msg_seqno_desc(mdesc)?;
    if count >= SNP_MSG_SEQNO_OVERFLOW_LIMIT {
        Ok(0)
    } else {
        Ok(count)
    }
}

pub fn snp_inc_msg_seqno_desc(mdesc: &mut SnpMsgDesc) -> Result<(), i32> {
    let index = mdesc.os_area_msg_seqno.ok_or(EINVAL)?;
    let seqno = mdesc.secrets.os_area.msg_seqno_mut(index).ok_or(EINVAL)?;
    *seqno = seqno.wrapping_add(2);
    Ok(())
}

fn snp_linux_aesgcm_ctx(mdesc: &SnpMsgDesc) -> Result<(LinuxAesGcmCtx, usize), i32> {
    let snp_ctx = mdesc.ctx.ok_or(EINVAL)?;
    let key = mdesc.vmpck.ok_or(EINVAL)?;
    let mut ctx = LinuxAesGcmCtx::default();
    if aesgcm_expandkey(&mut ctx, &key, snp_ctx.authtag_len) != 0 {
        return Err(EINVAL);
    }
    Ok((ctx, snp_ctx.authtag_len))
}

const fn snp_guest_msg_size_with_auth_fits(msg_sz: usize, authsize: usize, limit: usize) -> bool {
    msg_sz.wrapping_add(authsize) <= limit
}

pub fn enc_payload_desc(mdesc: &mut SnpMsgDesc, seqno: u64, req: &SnpGuestReq) -> Result<(), i32> {
    let (ctx, authsize) = snp_linux_aesgcm_ctx(mdesc)?;
    mdesc.secret_request = SnpGuestMsg::default();

    let hdr = &mut mdesc.secret_request.hdr;
    hdr.algo = SNP_AEAD_AES_256_GCM;
    hdr.hdr_version = MSG_HDR_VER;
    hdr.hdr_sz = SNP_GUEST_MSG_HDR_SIZE as u16;
    hdr.msg_type = req.msg_type;
    hdr.msg_version = req.msg_version;
    hdr.msg_seqno = seqno;
    hdr.msg_vmpck = req.vmpck_id as u8;
    hdr.msg_sz = req.req_sz as u16;

    if hdr.msg_seqno == 0 {
        return Err(ENOSR);
    }

    if !snp_guest_msg_size_with_auth_fits(req.req_sz, authsize, SNP_GUEST_MSG_PAYLOAD_SIZE) {
        return Err(EBADMSG);
    }

    let aad = hdr.aad_bytes();
    let iv = hdr.iv_from_seqno();
    aesgcm_encrypt(
        &ctx,
        &mut mdesc.secret_request.payload[..req.req_sz],
        &req.req_buf[..req.req_sz],
        &aad,
        &iv,
        &mut hdr.authtag,
    );

    Ok(())
}

pub fn verify_and_dec_payload_desc(
    mdesc: &mut SnpMsgDesc,
    req: &mut SnpGuestReq,
) -> Result<(), i32> {
    let (ctx, authsize) = snp_linux_aesgcm_ctx(mdesc)?;
    mdesc.secret_response = mdesc.response;

    let req_hdr = &mdesc.secret_request.hdr;
    let resp_hdr = &mdesc.secret_response.hdr;
    if resp_hdr.msg_seqno != req_hdr.msg_seqno.wrapping_add(1) {
        return Err(EBADMSG);
    }

    if resp_hdr.msg_type != req_hdr.msg_type.wrapping_add(1)
        || resp_hdr.msg_version != req_hdr.msg_version
    {
        return Err(EBADMSG);
    }

    let msg_sz = resp_hdr.msg_sz as usize;
    if !snp_guest_msg_size_with_auth_fits(msg_sz, authsize, req.resp_sz)
        || msg_sz > SNP_GUEST_MSG_PAYLOAD_SIZE
    {
        return Err(EBADMSG);
    }

    let aad = resp_hdr.aad_bytes();
    let iv = resp_hdr.iv_from_seqno();
    if !aesgcm_decrypt(
        &ctx,
        &mut req.resp_buf[..msg_sz],
        &mdesc.secret_response.payload[..msg_sz],
        &aad,
        &iv,
        &resp_hdr.authtag,
    ) {
        return Err(EBADMSG);
    }

    Ok(())
}

pub fn snp_issue_guest_request_from_ghcb(
    req: &mut SnpGuestReq,
    ghcb: SnpIssueGuestRequestGhcb,
) -> (SnpIssueGuestRequestPlan, Result<(), i32>) {
    let plan = SnpIssueGuestRequestPlan {
        exit_code: req.exit_code,
        req_gpa: req.input.req_gpa,
        resp_gpa: req.input.resp_gpa,
        rax: if req.exit_code == SVM_VMGEXIT_EXT_GUEST_REQUEST {
            Some(req.input.data_gpa)
        } else {
            None
        },
        rbx: if req.exit_code == SVM_VMGEXIT_EXT_GUEST_REQUEST {
            Some(req.input.data_npages as u64)
        } else {
            None
        },
    };

    req.exitinfo2 = SEV_RET_NO_FW_CALL;

    if !ghcb.ghcb_available {
        return (plan, Err(EIO));
    }

    if let Err(err) = ghcb.hv_call_result {
        return (plan, Err(err));
    }

    req.exitinfo2 = ghcb.sw_exit_info_2;
    let rc = match req.exitinfo2 {
        0 => Ok(()),
        err if err == snp_guest_vmm_err(SNP_GUEST_VMM_ERR_BUSY) => Err(EAGAIN),
        err if err == snp_guest_vmm_err(SNP_GUEST_VMM_ERR_INVALID_LEN)
            && req.exit_code == SVM_VMGEXIT_EXT_GUEST_REQUEST =>
        {
            req.input.data_npages = ghcb.rbx as u32;
            Err(ENOSPC)
        }
        _ => Err(EIO),
    };

    (plan, rc)
}

pub fn __handle_guest_request_with<B: SnpGuestRequestBackend>(
    backend: &mut B,
    mdesc: &mut SnpMsgDesc,
    req: &mut SnpGuestReq,
) -> Result<(), i32> {
    let mut override_npages = 0u32;
    let mut override_err = 0u64;
    let mut attempts = 0usize;

    let rc = loop {
        attempts += 1;
        match backend.issue_guest_request(mdesc, req) {
            Ok(()) => break Ok(()),
            Err(ENOSPC) => {
                override_npages = req.input.data_npages;
                req.exit_code = SVM_VMGEXIT_GUEST_REQUEST;
                override_err = snp_guest_vmm_err(SNP_GUEST_VMM_ERR_INVALID_LEN);
            }
            Err(EAGAIN) => {
                if backend.retry_timed_out(attempts) {
                    break Err(ETIMEDOUT);
                }
            }
            Err(err) => break Err(err),
        }
    };

    snp_inc_msg_seqno_desc(mdesc)?;

    let mut rc = rc;
    if override_err != 0 {
        req.exitinfo2 = override_err;
        if rc.is_ok() && override_err == snp_guest_vmm_err(SNP_GUEST_VMM_ERR_INVALID_LEN) {
            rc = Err(EIO);
        }
    }

    if override_npages != 0 {
        req.input.data_npages = override_npages;
    }

    rc
}

pub fn snp_send_guest_request_with<B: SnpGuestRequestBackend>(
    backend: &mut B,
    mdesc: &mut SnpMsgDesc,
    req: &mut SnpGuestReq,
) -> Result<(), i32> {
    if !req.req_buf_valid || !req.resp_buf_valid {
        return Err(EINVAL);
    }

    let Some(vmpck) = mdesc.vmpck else {
        return Err(ENOTTY);
    };
    if !vmpck.iter().any(|byte| *byte != 0) {
        return Err(ENOTTY);
    }

    let seqno = snp_get_msg_seqno_desc(mdesc)?;
    if seqno == 0 {
        return Err(EIO);
    }

    mdesc.response = SnpGuestMsg::default();
    enc_payload_desc(mdesc, seqno, req)?;
    mdesc.request = mdesc.secret_request;

    req.input.req_gpa = core::ptr::addr_of!(mdesc.request) as u64;
    req.input.resp_gpa = core::ptr::addr_of!(mdesc.response) as u64;
    req.input.data_gpa = if req.certs_data_present {
        req.certs_data_gpa
    } else {
        0
    };

    match __handle_guest_request_with(backend, mdesc, req) {
        Ok(()) => {}
        Err(err) => {
            if err == EIO && req.exitinfo2 == snp_guest_vmm_err(SNP_GUEST_VMM_ERR_INVALID_LEN) {
                return Err(err);
            }
            let _ = snp_disable_vmpck_desc(mdesc);
            return Err(err);
        }
    }

    if let Err(err) = verify_and_dec_payload_desc(mdesc, req) {
        let _ = snp_disable_vmpck_desc(mdesc);
        return Err(err);
    }

    Ok(())
}

pub fn snp_get_tsc_info_with<B: SnpGuestRequestBackend>(
    backend: &mut B,
    resources: SnpTscInfoResources,
) -> Result<SnpTscInfoResp, i32> {
    if !resources.tsc_req_available {
        return Err(ENOMEM);
    }

    if !resources.tsc_resp_available {
        return Err(ENOMEM);
    }

    if !resources.msg_desc_available {
        return Err(ENOMEM);
    }

    let current_vmpl = snp_vmpl();
    let mut mdesc = SnpMsgDesc::new(resources.secrets);
    snp_msg_init_desc(
        &mut mdesc,
        current_vmpl as i32,
        current_vmpl,
        resources.crypto_available,
    )?;

    let mut req = SnpGuestReq {
        req_sz: core::mem::size_of::<SnpTscInfoReq>(),
        resp_sz: core::mem::size_of::<SnpTscInfoResp>() + AUTHTAG_LEN,
        exit_code: SVM_VMGEXIT_GUEST_REQUEST,
        vmpck_id: current_vmpl as u32,
        msg_version: MSG_HDR_VER,
        msg_type: SNP_MSG_TSC_INFO_REQ,
        ..Default::default()
    };

    snp_send_guest_request_with(backend, &mut mdesc, &mut req)?;

    let tsc_resp = SnpTscInfoResp::from_payload(&req.resp_buf);
    if tsc_resp.status != 0 {
        return Err(EIO);
    }

    publish_secure_tsc_scale_offset(tsc_resp.tsc_scale, tsc_resp.tsc_offset);
    Ok(tsc_resp)
}

pub fn snp_secure_tsc_prepare_with<B: SnpGuestRequestBackend>(
    secure_tsc: bool,
    backend: &mut B,
    resources: SnpTscInfoResources,
) -> SnpSecureTscPrepareOutcome {
    if !secure_tsc {
        return SnpSecureTscPrepareOutcome::Skipped;
    }

    match snp_get_tsc_info_with(backend, resources) {
        Ok(_) => SnpSecureTscPrepareOutcome::Enabled,
        Err(rc) => SnpSecureTscPrepareOutcome::Terminated {
            termination: SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_SECURE_TSC,
            },
            rc,
        },
    }
}

pub fn snp_secure_tsc_init_with(
    secure_tsc: bool,
    secrets: Option<&SnpSecretsPage>,
    guest_tsc_freq_msr: u64,
) -> SnpSecureTscInitOutcome {
    if !secure_tsc {
        return SnpSecureTscInitOutcome::Skipped;
    }

    let Some(secrets) = secrets else {
        return SnpSecureTscInitOutcome::Terminated {
            termination: SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_SECURE_TSC,
            },
        };
    };

    let freq_mhz = guest_tsc_freq_msr & SNP_GUEST_TSC_FREQ_MASK;
    let freq_khz = snp_scale_tsc_freq(freq_mhz * 1000, secrets.tsc_factor);
    SNP_TSC_FREQ_KHZ.store(freq_khz, Ordering::Release);

    SnpSecureTscInitOutcome::Initialized { freq_mhz, freq_khz }
}

pub fn snp_msg_init(vmpck_id: usize, secrets_present: bool) -> Result<usize, i32> {
    if !secrets_present {
        return Err(EOPNOTSUPP);
    }

    let mut secrets = SnpSecretsPage::default();
    if vmpck_id < 4 {
        let mut key = [0u8; VMPCK_KEY_LEN];
        key[0] = 1;
        secrets.set_vmpck(vmpck_id, key)?;
    }

    snp_msg_init_with_secrets(secrets, vmpck_id as i32, snp_vmpl(), true)
        .map(|desc| usize::try_from(desc.vmpck_id).expect("validated VMPCK id is non-negative"))
}

#[cfg(test)]
mod tests {
    use super::*;

    static SEV_CORE_TEST_LOCK: spin::Mutex<()> = spin::Mutex::new(());

    fn sev_core_test_guard() -> spin::MutexGuard<'static, ()> {
        SEV_CORE_TEST_LOCK.lock()
    }

    #[test]
    fn ap_jump_table_source_matches_linux_snp_or_ghcb_rules() {
        let _guard = sev_core_test_guard();
        assert_eq!(
            get_jump_table_addr_from_sources(true, 0x4000, true, 0x8000),
            0x4000
        );
        assert_eq!(
            get_jump_table_addr_from_sources(false, 0x4000, true, 0x8000),
            0x8000
        );
        assert_eq!(
            get_jump_table_addr_from_sources(false, 0x4000, false, 0x8000),
            0
        );
    }

    #[test]
    fn ap_jump_table_setup_writes_linux_startup_words() {
        let _guard = sev_core_test_guard();
        let rmh = SevRealModeHeader {
            trampoline_start: 0x7000,
            sev_es_trampoline_start: 0x7420,
        };
        let mut jump_table = [0u16; 2];

        let plan =
            sev_es_setup_ap_jump_table_from_addr(&rmh, 0x12000, Some(&mut jump_table)).unwrap();
        assert_eq!(
            plan,
            Some(SevApJumpTablePlan {
                jump_table_pa: 0x12000,
                startup_ip: 0x420,
                startup_cs: 0x700,
            })
        );
        assert_eq!(jump_table, [0x420, 0x700]);
    }

    #[test]
    fn ap_jump_table_setup_matches_linux_error_paths() {
        let _guard = sev_core_test_guard();
        let rmh = SevRealModeHeader {
            trampoline_start: 0x7000,
            sev_es_trampoline_start: 0x7420,
        };
        let mut jump_table = [0u16; 2];

        assert_eq!(
            sev_es_setup_ap_jump_table_from_addr(&rmh, 0, Some(&mut jump_table)),
            Ok(None)
        );
        assert_eq!(
            sev_es_setup_ap_jump_table_from_addr(&rmh, 0x12008, Some(&mut jump_table)),
            Err(EINVAL)
        );
        assert_eq!(
            sev_es_setup_ap_jump_table_from_addr(&rmh, 0x12000, None),
            Err(EIO)
        );
    }

    #[test]
    fn ap_jump_table_setup_uses_non_snp_ghcb_source_when_valid() {
        let _guard = sev_core_test_guard();
        let rmh = SevRealModeHeader {
            trampoline_start: 0x9000,
            sev_es_trampoline_start: 0x9abc,
        };
        let mut jump_table = [0u16; 2];

        let plan =
            sev_es_setup_ap_jump_table(&rmh, false, 0x4000, true, 0x13000, Some(&mut jump_table))
                .unwrap();
        assert_eq!(plan.unwrap().jump_table_pa, 0x13000);
        assert_eq!(jump_table, [0xabc, 0x900]);
    }

    fn vmsa_inputs() -> SevVmsaInputs {
        SevVmsaInputs {
            hv_features: GHCB_HV_FT_SNP_AP_CREATION,
            real_mode: SevRealModeHeader {
                trampoline_start: 0x7000,
                sev_es_trampoline_start: 0x8420,
            },
            start_ip: 0x7000,
            apic_id: 0x22,
            cpu: 3,
            vmsa_page_available: true,
            set_vmsa_ok: true,
            ap_create_ok: true,
            cr4: X86_CR4_MCE | (1 << 5),
            secure_avic: true,
            snp_secure_tsc: true,
            snp_vmpl: 2,
            sev_status: 0b1011_1100,
            tsc_scale: 0x1111_2222,
            tsc_offset: 0x3333_4444,
        }
    }

    #[test]
    fn snp_alloc_vmsa_page_plan_models_linux_erratum_workaround() {
        let _guard = sev_core_test_guard();
        let plan = snp_alloc_vmsa_page_plan(3, true);

        assert_eq!(plan.cpu, 3);
        assert_eq!(plan.node, Some(0));
        assert_eq!(plan.gfp, GFP_KERNEL_ACCOUNT | __GFP_ZERO);
        assert_eq!(plan.order, 1);
        assert_eq!(plan.allocation_size, 2 * PAGE_SIZE);
        assert_eq!(plan.allocation_alignment, 2 * PAGE_SIZE);
        assert!(plan.allocated_order_page);
        assert_eq!(plan.split_page_order, Some(1));
        assert!(plan.freed_first_page);
        assert!(plan.returned_second_page);
        assert_eq!(plan.returned_page_offset, PAGE_SIZE);
    }

    #[test]
    fn snp_alloc_vmsa_page_plan_returns_null_equivalent_on_alloc_failure() {
        let _guard = sev_core_test_guard();
        let plan = snp_alloc_vmsa_page_plan(3, false);

        assert_eq!(plan.gfp, GFP_KERNEL_ACCOUNT | __GFP_ZERO);
        assert_eq!(plan.order, 1);
        assert!(!plan.allocated_order_page);
        assert_eq!(plan.split_page_order, None);
        assert!(!plan.freed_first_page);
        assert!(!plan.returned_second_page);
        assert_eq!(plan.returned_page_offset, 0);
    }

    #[test]
    fn snp_wakeup_vmsa_plan_matches_linux_init_defaults() {
        let _guard = sev_core_test_guard();
        let plan = snp_wakeup_cpu_vmsa_plan(vmsa_inputs()).unwrap();

        assert_eq!(plan.start_ip, 0x8420);
        assert_eq!(plan.sipi_vector, 0x8);
        assert_eq!(plan.cs.base, 0x8000);
        assert_eq!(plan.cs.limit, AP_INIT_CS_LIMIT);
        assert_eq!(plan.cs.attrib, INIT_CS_ATTRIBS);
        assert_eq!(plan.cs.selector, 0x800);
        assert_eq!(
            plan.ds,
            SevVmsaSegment {
                base: 0,
                limit: AP_INIT_DS_LIMIT,
                attrib: INIT_DS_ATTRIBS,
                selector: 0,
            }
        );
        assert_eq!(plan.rip, 0x420);
        assert_eq!(plan.cr4, X86_CR4_MCE);
        assert_eq!(plan.cr0, AP_INIT_CR0_DEFAULT);
        assert_eq!(plan.dr6, AP_INIT_DR6_DEFAULT);
        assert_eq!(
            plan.dr7,
            crate::arch::x86::coco::sev::vc_handle::DR7_RESET_VALUE
        );
        assert_eq!(plan.rflags, AP_INIT_RFLAGS_DEFAULT);
        assert_eq!(plan.g_pat, AP_INIT_GPAT_DEFAULT);
        assert_eq!(plan.xcr0, AP_INIT_XCR0_DEFAULT);
        assert_eq!(plan.mxcsr, AP_INIT_MXCSR_DEFAULT);
        assert_eq!(plan.x87_ftw, AP_INIT_X87_FTW_DEFAULT);
        assert_eq!(plan.x87_fcw, AP_INIT_X87_FCW_DEFAULT);
        assert_eq!(plan.efer, EFER_SVME);
        assert_eq!(plan.vmpl, 2);
        assert_eq!(plan.sev_features, 0b10_1111);
        assert_eq!(plan.vintr_ctrl, V_GIF_MASK | V_NMI_ENABLE_MASK);
        assert_eq!(plan.tsc_scale, 0x1111_2222);
        assert_eq!(plan.tsc_offset, 0x3333_4444);
        assert_eq!(
            plan.ap_create_exit_info_1,
            (0x22u64 << 32) | (2u64 << 16) | SVM_VMGEXIT_AP_CREATE
        );
    }

    #[test]
    fn snp_wakeup_vmsa_plan_matches_linux_failure_paths() {
        let _guard = sev_core_test_guard();
        let mut inputs = vmsa_inputs();

        inputs.hv_features = 0;
        assert_eq!(snp_wakeup_cpu_vmsa_plan(inputs), Err(EOPNOTSUPP));

        inputs = vmsa_inputs();
        inputs.start_ip = 0x8000;
        assert_eq!(snp_wakeup_cpu_vmsa_plan(inputs), Err(EINVAL));

        inputs = vmsa_inputs();
        inputs.vmsa_page_available = false;
        assert_eq!(snp_wakeup_cpu_vmsa_plan(inputs), Err(ENOMEM));

        inputs = vmsa_inputs();
        inputs.set_vmsa_ok = false;
        assert_eq!(snp_wakeup_cpu_vmsa_plan(inputs), Err(EINVAL));

        inputs = vmsa_inputs();
        inputs.ap_create_ok = false;
        assert_eq!(snp_wakeup_cpu_vmsa_plan(inputs), Err(EINVAL));
    }

    #[test]
    fn snp_wakeup_vmsa_plan_skips_optional_secure_bits_when_disabled() {
        let _guard = sev_core_test_guard();
        let mut inputs = vmsa_inputs();
        inputs.secure_avic = false;
        inputs.snp_secure_tsc = false;
        let plan = snp_wakeup_cpu_vmsa_plan(inputs).unwrap();
        assert_eq!(plan.vintr_ctrl, 0);
        assert_eq!(plan.tsc_scale, 0);
        assert_eq!(plan.tsc_offset, 0);
    }

    #[test]
    fn snp_set_wakeup_secondary_cpu_installs_vmgexit_callback_only_for_snp() {
        let _guard = sev_core_test_guard();
        assert_eq!(
            snp_set_wakeup_secondary_cpu_plan(false),
            SnpWakeupSecondaryCpuPlan {
                sev_snp: false,
                callback_installed: false,
                callback: None,
            }
        );
        assert_eq!(
            snp_set_wakeup_secondary_cpu_plan(true),
            SnpWakeupSecondaryCpuPlan {
                sev_snp: true,
                callback_installed: true,
                callback: Some(SnpWakeupCallback::WakeupCpuViaVmgexit),
            }
        );
    }

    #[test]
    fn wakeup_cpu_via_vmgexit_records_new_vmsa_and_cleans_previous_like_linux() {
        let _guard = sev_core_test_guard();
        let plan = wakeup_cpu_via_vmgexit_plan(SnpWakeupCpuInputs {
            vmsa: vmsa_inputs(),
            new_vmsa_pa: 0x7000,
            current_vmsa_pa: 0x5000,
        });

        assert_eq!(plan.rc, 0);
        assert!(plan.allocated_new_vmsa);
        assert!(plan.cleaned_previous_vmsa);
        assert_eq!(plan.recorded_vmsa_pa, 0x7000);
        assert!(plan.vmsa.is_some());
        assert!(!plan.freed_new_vmsa_on_set_failure);
        assert!(!plan.cleaned_new_vmsa_on_ap_failure);
    }

    #[test]
    fn wakeup_cpu_via_vmgexit_preserves_linux_failure_cleanup_order() {
        let _guard = sev_core_test_guard();
        let mut inputs = vmsa_inputs();
        inputs.hv_features = 0;
        let plan = wakeup_cpu_via_vmgexit_plan(SnpWakeupCpuInputs {
            vmsa: inputs,
            new_vmsa_pa: 0x7000,
            current_vmsa_pa: 0x5000,
        });
        assert_eq!(plan.rc, -EOPNOTSUPP);
        assert!(!plan.allocated_new_vmsa);
        assert!(!plan.cleaned_previous_vmsa);

        inputs = vmsa_inputs();
        inputs.set_vmsa_ok = false;
        let plan = wakeup_cpu_via_vmgexit_plan(SnpWakeupCpuInputs {
            vmsa: inputs,
            new_vmsa_pa: 0x7000,
            current_vmsa_pa: 0x5000,
        });
        assert_eq!(plan.rc, -EINVAL);
        assert!(plan.allocated_new_vmsa);
        assert!(plan.freed_new_vmsa_on_set_failure);
        assert!(!plan.cleaned_previous_vmsa);

        inputs = vmsa_inputs();
        inputs.ap_create_ok = false;
        let plan = wakeup_cpu_via_vmgexit_plan(SnpWakeupCpuInputs {
            vmsa: inputs,
            new_vmsa_pa: 0x7000,
            current_vmsa_pa: 0x5000,
        });
        assert_eq!(plan.rc, -EINVAL);
        assert!(plan.allocated_new_vmsa);
        assert!(plan.cleaned_new_vmsa_on_ap_failure);
        assert!(plan.cleaned_previous_vmsa);
        assert_eq!(plan.recorded_vmsa_pa, 0);
    }

    #[test]
    fn ap_control_request_packs_linux_ghcb_fields() {
        let _guard = sev_core_test_guard();
        let plan =
            vmgexit_ap_control_request(SVM_VMGEXIT_AP_CREATE, 0x33, 2, 0x1234_5000, 0xfeed_beef)
                .unwrap();

        assert_eq!(plan.sw_exit_code, SVM_VMGEXIT_AP_CREATION);
        assert_eq!(
            plan.sw_exit_info_1,
            (0x33u64 << 32) | (2u64 << 16) | SVM_VMGEXIT_AP_CREATE
        );
        assert_eq!(plan.sw_exit_info_2, 0x1234_5000);
        assert_eq!(plan.ghcb_rax, Some(0xfeed_beef));

        let destroy =
            vmgexit_ap_control_request(SVM_VMGEXIT_AP_DESTROY, 0x33, 2, 0x1234_5000, 0).unwrap();
        assert_eq!(destroy.ghcb_rax, None);
        assert_eq!(
            destroy.sw_exit_info_1,
            (0x33u64 << 32) | (2u64 << 16) | SVM_VMGEXIT_AP_DESTROY
        );
        assert_eq!(vmgexit_ap_control_request(99, 0, 0, 0, 0), Err(EOPNOTSUPP));
    }

    #[test]
    fn ap_control_response_matches_linux_error_check() {
        let _guard = sev_core_test_guard();
        assert!(
            vmgexit_ap_control_with_response(SVM_VMGEXIT_AP_CREATE, 1, 0, 0x1000, 0x20, true, 0,)
                .is_ok()
        );
        assert_eq!(
            vmgexit_ap_control_with_response(SVM_VMGEXIT_AP_CREATE, 1, 0, 0x1000, 0x20, false, 0,),
            Err(EINVAL)
        );
        assert_eq!(
            vmgexit_ap_control_with_response(SVM_VMGEXIT_AP_CREATE, 1, 0, 0x1000, 0x20, true, 1,),
            Err(EINVAL)
        );
    }

    #[test]
    fn snp_set_vmsa_plan_matches_linux_svsm_or_rmpadjust_paths() {
        let _guard = sev_core_test_guard();
        assert_eq!(
            snp_set_vmsa_plan(0x2000, 0x3000, 7, true, 1),
            SnpSetVmsaPlan::Svsm {
                call_rax: crate::arch::x86::coco::sev::svsm::SVSM_CORE_CREATE_VCPU as u64,
                rcx_vmsa_pa: 0x2000,
                rdx_caa_pa: 0x3000,
                r8_apic_id: 7,
            }
        );
        assert_eq!(
            snp_set_vmsa_plan(0x2000, 0x3000, 7, false, 1),
            SnpSetVmsaPlan::Svsm {
                call_rax: crate::arch::x86::coco::sev::svsm::SVSM_CORE_DELETE_VCPU as u64,
                rcx_vmsa_pa: 0x2000,
                rdx_caa_pa: 0,
                r8_apic_id: 0,
            }
        );
        assert_eq!(
            snp_set_vmsa_plan(0x4000, 0, 7, true, 0),
            SnpSetVmsaPlan::Rmpadjust {
                vmsa_va: 0x4000,
                page_size: RMP_PG_SIZE_4K,
                attrs: 1 | RMPADJUST_VMSA_PAGE_BIT,
            }
        );
        assert_eq!(
            snp_set_vmsa_plan(0x4000, 0, 7, false, 0),
            SnpSetVmsaPlan::Rmpadjust {
                vmsa_va: 0x4000,
                page_size: RMP_PG_SIZE_4K,
                attrs: 1,
            }
        );
    }

    #[test]
    fn savic_msr_helpers_match_linux_msr_register_packing_and_fatal_failures() {
        let _guard = sev_core_test_guard();
        use crate::arch::x86::coco::sev::vc_shared::EsResult;

        let (read, value) =
            savic_ghcb_msr_read_plan(0x300, EsResult::Continue, 0x5566_7788, 0x1122_3344);
        assert_eq!(read.msr, APIC_BASE_MSR + (0x300 >> 4));
        assert!(!read.write);
        assert_eq!(read.cx, read.msr);
        assert_eq!(read.ax, 0x5566_7788);
        assert_eq!(read.dx, 0x1122_3344);
        assert_eq!(read.termination, None);
        assert_eq!(value, Some(0x1122_3344_5566_7788));

        let (failed_read, failed_value) = savic_ghcb_msr_read_plan(0x30, EsResult::VmmError, 0, 0);
        assert_eq!(failed_value, None);
        assert_eq!(
            failed_read.termination,
            Some(SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_SAVIC_FAIL,
            })
        );

        let write = savic_ghcb_msr_write_plan(0x310, 0xaabb_ccdd_1122_3344, EsResult::Continue);
        assert!(write.write);
        assert_eq!(write.msr, APIC_BASE_MSR + (0x310 >> 4));
        assert_eq!(write.ax, 0x1122_3344);
        assert_eq!(write.dx, 0xaabb_ccdd);
        assert_eq!(write.termination, None);

        let failed_write = savic_ghcb_msr_write_plan(0x310, 0, EsResult::Exception);
        assert_eq!(
            failed_write.termination,
            Some(SevTermination {
                set: SEV_TERM_SET_LINUX,
                reason: GHCB_TERM_SAVIC_FAIL,
            })
        );
    }

    #[test]
    fn savic_register_and_unregister_gpa_match_linux_ghcb_fields() {
        let _guard = sev_core_test_guard();
        use crate::arch::x86::coco::sev::vc_shared::EsResult;

        let register = savic_register_gpa_plan(0xfeed_cafe, EsResult::Continue);
        assert_eq!(
            register,
            SnpSavicGpaPlan {
                operation: SVM_VMGEXIT_SAVIC_REGISTER_GPA,
                gpa_argument: Some(0xfeed_cafe),
                sw_exit_code: SVM_VMGEXIT_SAVIC,
                sw_exit_info_1: SVM_VMGEXIT_SAVIC_REGISTER_GPA,
                sw_exit_info_2: 0,
                rax: SVM_VMGEXIT_SAVIC_SELF_GPA,
                rbx: Some(0xfeed_cafe),
                hv_result: EsResult::Continue,
                returned_gpa: None,
            }
        );
        assert_eq!(
            savic_register_gpa(0xfeed_cafe),
            Ok((SVM_VMGEXIT_SAVIC_REGISTER_GPA, 0xfeed_cafe))
        );

        let unregister = savic_unregister_gpa_plan(true, 0x1234_5000, EsResult::Continue);
        assert_eq!(unregister.rax, SVM_VMGEXIT_SAVIC_SELF_GPA);
        assert_eq!(unregister.rbx, None);
        assert_eq!(unregister.returned_gpa, Some(0x1234_5000));
        assert_eq!(
            savic_unregister_gpa(0x1234_5000),
            Ok((SVM_VMGEXIT_SAVIC_UNREGISTER_GPA, 0x1234_5000))
        );

        let no_output = savic_unregister_gpa_plan(false, 0x1234_5000, EsResult::Continue);
        assert_eq!(no_output.returned_gpa, None);

        let failed = savic_unregister_gpa_plan(true, 0x1234_5000, EsResult::VmmError);
        assert_eq!(failed.returned_gpa, None);
    }

    #[test]
    fn efi_map_ghcbs_cas_skips_without_guest_state_encryption() {
        let _guard = sev_core_test_guard();
        let cpus = [SevEfiMapCpu {
            cpu: 0,
            ghcb_pa: 0x1000,
            svsm_caa_pa: 0x2000,
            ghcb_map_ok: false,
            ca_map_ok: false,
        }];

        let plan = sev_es_efi_map_ghcbs_cas_plan(false, 1, 0x8000, &cpus);
        assert!(plan.skipped_no_guest_state_encrypt);
        assert_eq!(plan.rc, 0);
        assert_eq!(plan.entry_count, 0);
    }

    #[test]
    fn efi_map_ghcbs_cas_maps_each_ghcb_and_svsm_caa_like_linux() {
        let _guard = sev_core_test_guard();
        let pflags =
            crate::arch::x86::mm::paging::_PAGE_NX | crate::arch::x86::mm::paging::_PAGE_RW;
        let cpus = [
            SevEfiMapCpu {
                cpu: 0,
                ghcb_pa: 0x4000,
                svsm_caa_pa: 0x9000,
                ghcb_map_ok: true,
                ca_map_ok: true,
            },
            SevEfiMapCpu {
                cpu: 1,
                ghcb_pa: 0x8000,
                svsm_caa_pa: 0xa000,
                ghcb_map_ok: true,
                ca_map_ok: true,
            },
        ];

        let ghcb_only = sev_es_efi_map_ghcbs_cas_plan(true, 0, 0x1000_0000, &cpus);
        assert_eq!(ghcb_only.rc, 0);
        assert_eq!(ghcb_only.entry_count, 2);
        assert_eq!(
            ghcb_only.entries[0],
            Some(SevEfiMapEntry {
                cpu: 0,
                kind: SevEfiMapKind::Ghcb,
                pfn: 4,
                address: 0x4000,
                pages: 1,
                pflags,
            })
        );
        assert_eq!(ghcb_only.entries[1].unwrap().kind, SevEfiMapKind::Ghcb);

        let with_caa = sev_es_efi_map_ghcbs_cas_plan(true, 2, 0x1000_0000, &cpus);
        assert_eq!(with_caa.rc, 0);
        assert_eq!(with_caa.entry_count, 4);
        assert_eq!(
            with_caa.entries[0],
            Some(SevEfiMapEntry {
                cpu: 0,
                kind: SevEfiMapKind::Ghcb,
                pfn: 4,
                address: 0x4000,
                pages: 1,
                pflags,
            })
        );
        assert_eq!(
            with_caa.entries[1],
            Some(SevEfiMapEntry {
                cpu: 0,
                kind: SevEfiMapKind::SvsmCaa,
                pfn: 9,
                address: 0x9000,
                pages: 1,
                pflags: pflags | 0x1000_0000,
            })
        );
        assert_eq!(with_caa.entries[2].unwrap().cpu, 1);
        assert_eq!(with_caa.entries[3].unwrap().kind, SevEfiMapKind::SvsmCaa);
    }

    #[test]
    fn efi_map_ghcbs_cas_stops_at_linux_failure_points() {
        let _guard = sev_core_test_guard();
        let cpus = [
            SevEfiMapCpu {
                cpu: 0,
                ghcb_pa: 0x4000,
                svsm_caa_pa: 0x9000,
                ghcb_map_ok: true,
                ca_map_ok: true,
            },
            SevEfiMapCpu {
                cpu: 1,
                ghcb_pa: 0x8000,
                svsm_caa_pa: 0,
                ghcb_map_ok: true,
                ca_map_ok: true,
            },
        ];

        let missing_caa = sev_es_efi_map_ghcbs_cas_plan(true, 1, 0x100, &cpus);
        assert_eq!(missing_caa.rc, 1);
        assert_eq!(missing_caa.entry_count, 3);
        assert_eq!(missing_caa.entries[2].unwrap().kind, SevEfiMapKind::Ghcb);

        let ghcb_fail = sev_es_efi_map_ghcbs_cas_plan(
            true,
            1,
            0x100,
            &[SevEfiMapCpu {
                cpu: 0,
                ghcb_pa: 0x4000,
                svsm_caa_pa: 0x9000,
                ghcb_map_ok: false,
                ca_map_ok: true,
            }],
        );
        assert_eq!(ghcb_fail.rc, 1);
        assert_eq!(ghcb_fail.entry_count, 1);

        let ca_fail = sev_es_efi_map_ghcbs_cas_plan(
            true,
            1,
            0x100,
            &[SevEfiMapCpu {
                cpu: 0,
                ghcb_pa: 0x4000,
                svsm_caa_pa: 0x9000,
                ghcb_map_ok: true,
                ca_map_ok: false,
            }],
        );
        assert_eq!(ca_fail.rc, 1);
        assert_eq!(ca_fail.entry_count, 2);
    }

    fn reg_gpa_response(paddr: u64) -> u64 {
        ((paddr >> 12) << 12) | crate::arch::x86::coco::sev::vc_shared::GHCB_MSR_REG_GPA_RESP
    }

    #[test]
    fn setup_ghcb_skips_when_guest_state_encryption_is_absent() {
        let _guard = sev_core_test_guard();
        let plan = setup_ghcb_plan(false, true, true, None, 0x1000, 0x2000, None, None);

        assert!(plan.skipped_no_guest_state_encrypt);
        assert!(plan.used_runtime_handler);
        assert_eq!(plan.action_count, 0);
    }

    #[test]
    fn setup_ghcb_runtime_handler_registers_per_cpu_ghcb_for_snp_then_marks_ready() {
        let _guard = sev_core_test_guard();
        let paddr = 0x45_000;
        let plan = setup_ghcb_plan(
            true,
            true,
            true,
            None,
            paddr,
            0x99_000,
            Some(reg_gpa_response(paddr)),
            None,
        );

        assert!(!plan.skipped_no_guest_state_encrypt);
        assert!(plan.used_runtime_handler);
        assert_eq!(plan.action_count, 2);
        assert_eq!(
            plan.actions[0],
            Some(GhcbSetupAction::RegisterPerCpuGhcb {
                paddr,
                request_msr:
                    crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(paddr,),
                response_ok: true,
            })
        );
        assert_eq!(
            plan.actions[1],
            Some(GhcbSetupAction::SetGhcbsInitialized(true))
        );

        let failed = setup_ghcb_plan(true, true, true, None, paddr, 0x99_000, Some(0), None);
        assert_eq!(failed.action_count, 2);
        assert_eq!(
            failed.actions[0],
            Some(GhcbSetupAction::RegisterPerCpuGhcb {
                paddr,
                request_msr:
                    crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(paddr,),
                response_ok: false,
            })
        );
        assert_eq!(
            failed.actions[1],
            Some(GhcbSetupAction::Terminate {
                termination: SevTermination {
                    set: SEV_TERM_SET_LINUX,
                    reason: GHCB_TERM_REGISTER,
                },
            })
        );

        let non_snp = setup_ghcb_plan(true, false, true, Some(1), paddr, 0x99_000, None, None);
        assert_eq!(non_snp.action_count, 1);
        assert_eq!(
            non_snp.actions[0],
            Some(GhcbSetupAction::SetGhcbsInitialized(true))
        );
    }

    #[test]
    fn setup_ghcb_boot_path_negotiates_then_selects_and_registers_boot_ghcb() {
        let _guard = sev_core_test_guard();
        let boot = 0x80_000;
        let plan = setup_ghcb_plan(
            true,
            true,
            false,
            Some(1),
            0x40_000,
            boot,
            None,
            Some(reg_gpa_response(boot)),
        );

        assert!(!plan.used_runtime_handler);
        assert_eq!(plan.action_count, 4);
        assert_eq!(
            plan.actions[0],
            Some(GhcbSetupAction::SetGhcbProtocolVersion { version: 1 })
        );
        assert_eq!(
            plan.actions[1],
            Some(GhcbSetupAction::ClearBootGhcbPage { paddr: boot })
        );
        assert_eq!(
            plan.actions[2],
            Some(GhcbSetupAction::SelectBootGhcb { paddr: boot })
        );
        assert_eq!(
            plan.actions[3],
            Some(GhcbSetupAction::RegisterBootGhcb {
                paddr: boot,
                request_msr:
                    crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(boot,),
                response_ok: true,
            })
        );

        let failed = setup_ghcb_plan(true, true, false, Some(1), 0, boot, None, Some(0));
        assert_eq!(failed.action_count, 5);
        assert_eq!(
            failed.actions[3],
            Some(GhcbSetupAction::RegisterBootGhcb {
                paddr: boot,
                request_msr:
                    crate::arch::x86::coco::sev::vc_shared::snp_register_ghcb_early_request(boot,),
                response_ok: false,
            })
        );
        assert_eq!(
            failed.actions[4],
            Some(GhcbSetupAction::Terminate {
                termination: SevTermination {
                    set: SEV_TERM_SET_LINUX,
                    reason: GHCB_TERM_REGISTER,
                },
            })
        );

        let non_snp = setup_ghcb_plan(true, false, false, Some(1), 0, boot, None, None);
        assert_eq!(non_snp.action_count, 3);
        assert_eq!(
            non_snp.actions[0],
            Some(GhcbSetupAction::SetGhcbProtocolVersion { version: 1 })
        );
        assert_eq!(
            non_snp.actions[1],
            Some(GhcbSetupAction::ClearBootGhcbPage { paddr: boot })
        );
        assert_eq!(
            non_snp.actions[2],
            Some(GhcbSetupAction::SelectBootGhcb { paddr: boot })
        );
    }

    #[test]
    fn setup_ghcb_boot_path_terminates_on_protocol_negotiation_failure() {
        let _guard = sev_core_test_guard();
        let plan = setup_ghcb_plan(true, true, false, None, 0x1000, 0x2000, None, None);

        assert_eq!(plan.action_count, 1);
        assert_eq!(
            plan.actions[0],
            Some(GhcbSetupAction::Terminate {
                termination: SevTermination {
                    set: SEV_TERM_SET_GEN,
                    reason: GHCB_SEV_ES_GEN_REQ,
                },
            })
        );
    }

    #[test]
    fn sev_es_ap_hlt_loop_and_play_dead_follow_linux_wakeup_signal() {
        let _guard = sev_core_test_guard();
        let exits = [
            SevApHltExit {
                sw_exit_info_2_valid: false,
                sw_exit_info_2: 9,
            },
            SevApHltExit {
                sw_exit_info_2_valid: true,
                sw_exit_info_2: 0,
            },
            SevApHltExit {
                sw_exit_info_2_valid: true,
                sw_exit_info_2: 1,
            },
        ];

        let plan = sev_es_ap_hlt_loop_plan(&exits);
        assert!(plan.ghcb_acquired);
        assert!(plan.ghcb_released);
        assert_eq!(plan.iterations, 3);
        assert_eq!(plan.sw_exit_code, SVM_VMGEXIT_AP_HLT_LOOP);
        assert_eq!(plan.sw_exit_info_1, 0);
        assert_eq!(plan.sw_exit_info_2, 0);
        assert!(plan.woke);
        assert!(!plan.truncated_without_wakeup);

        let play_dead = sev_es_play_dead_plan(&exits);
        assert!(play_dead.play_dead_common_called);
        assert!(play_dead.soft_restart_cpu_called);

        let no_wake = sev_es_play_dead_plan(&exits[..2]);
        assert!(no_wake.hlt_loop.truncated_without_wakeup);
        assert!(!no_wake.hlt_loop.ghcb_released);
        assert!(!no_wake.soft_restart_cpu_called);
    }

    fn vc_init_cpu(cpu: u32) -> SevRuntimeCpuInput {
        SevRuntimeCpuInput {
            cpu,
            runtime_alloc_ok: true,
            svsm_caa_alloc_ok: true,
            svsm_caa_pa: 0x9000 + (cpu as u64 * PAGE_SIZE),
            ghcb_decrypt_rc: 0,
        }
    }

    #[test]
    fn sev_es_init_vc_handling_models_skip_feature_and_snp_terminate_paths() {
        let _guard = sev_core_test_guard();
        let cpus = [vc_init_cpu(0)];

        let skipped = sev_es_init_vc_handling_plan(SevVcInitInputs {
            guest_state_encrypt: false,
            cpu_features_ok: true,
            sev_snp: true,
            hv_features: GHCB_HV_FT_SNP,
            snp_vmpl: 0,
            boot_svsm_caa_pa: 0x8000,
            smp_enabled: true,
            cpus: &cpus,
        });
        assert!(skipped.skipped_no_guest_state_encrypt);
        assert_eq!(skipped.action_count, 0);

        let missing_features = sev_es_init_vc_handling_plan(SevVcInitInputs {
            guest_state_encrypt: true,
            cpu_features_ok: false,
            sev_snp: false,
            hv_features: 0,
            snp_vmpl: 0,
            boot_svsm_caa_pa: 0x8000,
            smp_enabled: true,
            cpus: &cpus,
        });
        assert_eq!(
            missing_features.panic,
            Some(SevVcInitPanic::MissingCpuFeatures)
        );
        assert_eq!(missing_features.action_count, 0);

        let unsupported_snp = sev_es_init_vc_handling_plan(SevVcInitInputs {
            guest_state_encrypt: true,
            cpu_features_ok: true,
            sev_snp: true,
            hv_features: 0,
            snp_vmpl: 0,
            boot_svsm_caa_pa: 0x8000,
            smp_enabled: true,
            cpus: &cpus,
        });
        assert_eq!(
            unsupported_snp.actions[0],
            Some(SevVcInitAction::LoadHvFeatures { hv_features: 0 })
        );
        assert_eq!(
            unsupported_snp.termination,
            Some(SevTermination {
                set: SEV_TERM_SET_GEN,
                reason: GHCB_SNP_UNSUPPORTED,
            })
        );
        assert_eq!(unsupported_snp.cpu_count, 0);
    }

    #[test]
    fn sev_es_init_vc_handling_allocates_runtime_ghcbs_and_svsm_cas_in_linux_order() {
        let _guard = sev_core_test_guard();
        let cpus = [vc_init_cpu(0), vc_init_cpu(1)];

        let plan = sev_es_init_vc_handling_plan(SevVcInitInputs {
            guest_state_encrypt: true,
            cpu_features_ok: true,
            sev_snp: true,
            hv_features: GHCB_HV_FT_SNP | GHCB_HV_FT_SNP_AP_CREATION,
            snp_vmpl: 2,
            boot_svsm_caa_pa: 0x8000,
            smp_enabled: true,
            cpus: &cpus,
        });

        assert_eq!(plan.panic, None);
        assert_eq!(plan.termination, None);
        assert!(plan.use_cas);
        assert!(plan.play_dead_setup);
        assert!(plan.runtime_vc_handler_set);
        assert_eq!(
            &plan.actions[..plan.action_count],
            &[
                Some(SevVcInitAction::LoadHvFeatures {
                    hv_features: GHCB_HV_FT_SNP | GHCB_HV_FT_SNP_AP_CREATION,
                }),
                Some(SevVcInitAction::AllocRuntimeData { cpu: 0 }),
                Some(SevVcInitAction::InitGhcb { cpu: 0 }),
                Some(SevVcInitAction::AllocRuntimeData { cpu: 1 }),
                Some(SevVcInitAction::InitGhcb { cpu: 1 }),
                Some(SevVcInitAction::EnableSvsmCas),
                Some(SevVcInitAction::SetupPlayDead),
                Some(SevVcInitAction::SetRuntimeVcHandler),
            ]
        );

        let bsp = plan.cpu_plans[0].unwrap();
        assert!(bsp.runtime_data_set);
        assert!(bsp.svsm_caa_set);
        assert!(bsp.svsm_caa_uses_boot_page);
        assert_eq!(bsp.svsm_caa_pa, 0x8000);
        assert!(bsp.ghcb_decrypted);
        assert!(bsp.ghcb_zeroed);
        assert!(!bsp.ghcb_active);
        assert!(!bsp.backup_ghcb_active);

        let ap = plan.cpu_plans[1].unwrap();
        assert!(ap.svsm_caa_set);
        assert!(!ap.svsm_caa_uses_boot_page);
        assert_eq!(ap.svsm_caa_pa, cpus[1].svsm_caa_pa);
    }

    #[test]
    fn sev_es_init_vc_handling_stops_on_runtime_svsm_or_ghcb_panic() {
        let _guard = sev_core_test_guard();
        let mut cpus = [vc_init_cpu(0), vc_init_cpu(1)];
        cpus[1].svsm_caa_alloc_ok = false;
        let plan = sev_es_init_vc_handling_plan(SevVcInitInputs {
            guest_state_encrypt: true,
            cpu_features_ok: true,
            sev_snp: false,
            hv_features: 0,
            snp_vmpl: 1,
            boot_svsm_caa_pa: 0x8000,
            smp_enabled: false,
            cpus: &cpus,
        });
        assert_eq!(plan.panic, Some(SevVcInitPanic::SvsmCaaAlloc { cpu: 1 }));
        assert_eq!(plan.cpu_count, 2);
        assert_eq!(
            plan.actions[2],
            Some(SevVcInitAction::AllocRuntimeData { cpu: 1 })
        );
        assert_eq!(plan.actions[3], None);

        cpus = [vc_init_cpu(0), vc_init_cpu(1)];
        cpus[1].ghcb_decrypt_rc = -EIO;
        let plan = sev_es_init_vc_handling_plan(SevVcInitInputs {
            guest_state_encrypt: true,
            cpu_features_ok: true,
            sev_snp: false,
            hv_features: 0,
            snp_vmpl: 0,
            boot_svsm_caa_pa: 0,
            smp_enabled: false,
            cpus: &cpus,
        });
        assert_eq!(
            plan.panic,
            Some(SevVcInitPanic::GhcbDecrypt { cpu: 1, rc: -EIO })
        );
        assert_eq!(plan.actions[3], Some(SevVcInitAction::InitGhcb { cpu: 1 }));
        assert!(!plan.runtime_vc_handler_set);
    }

    #[test]
    fn snp_kexec_begin_matches_linux_skip_and_warn_paths() {
        let _guard = sev_core_test_guard();
        assert_eq!(
            snp_kexec_begin_plan(false, true, true),
            SnpKexecBeginOutcome::SkippedNotSnp
        );
        assert_eq!(
            snp_kexec_begin_plan(true, false, true),
            SnpKexecBeginOutcome::SkippedKexecDisabled
        );
        assert_eq!(
            snp_kexec_begin_plan(true, true, true),
            SnpKexecBeginOutcome::ConversionsStopped
        );
        assert_eq!(
            snp_kexec_begin_plan(true, true, false),
            SnpKexecBeginOutcome::StopConversionWarned
        );
    }

    #[test]
    fn snp_kexec_finish_is_noop_unless_snp_and_kexec_are_enabled() {
        let _guard = sev_core_test_guard();
        let inputs = SnpKexecFinishInputs {
            sev_snp: false,
            kexec_core_enabled: true,
            this_cpu: 0,
            snp_vmpl: 0,
            sev_features: 0,
            direct_map: &[],
            bss_decrypted: &[],
            bss_start: 0,
            bss_end: 0,
            possible_cpus: &[],
        };

        assert_eq!(
            snp_kexec_finish_plan(inputs).unwrap(),
            SnpKexecFinishPlan::skipped(SnpKexecFinishSkip::NotSnp)
        );

        let disabled = SnpKexecFinishInputs {
            sev_snp: true,
            kexec_core_enabled: false,
            ..inputs
        };
        assert_eq!(
            snp_kexec_finish_plan(disabled).unwrap(),
            SnpKexecFinishPlan::skipped(SnpKexecFinishSkip::KexecDisabled)
        );
    }

    #[test]
    fn snp_kexec_finish_orders_ap_shutdown_unshare_and_ghcb_tail_like_linux() {
        let _guard = sev_core_test_guard();
        let base = crate::arch::x86::mm::paging::PAGE_OFFSET;
        let bss_start = base + 0x400000;
        let cpus = [
            SnpKexecCpu {
                cpu: 0,
                present: true,
                apic_id: 0x10,
                vmsa_pa: 0x10000,
                ghcb_pa: base + 0x1800,
                ghcb_mapping_size: PMD_SIZE,
                ghcb_mapping_level: 2,
                online_page_present: true,
                clear_vmsa_ok: true,
            },
            SnpKexecCpu {
                cpu: 1,
                present: true,
                apic_id: 0x11,
                vmsa_pa: 0,
                ghcb_pa: base + 0x220000,
                ghcb_mapping_size: PAGE_SIZE,
                ghcb_mapping_level: 1,
                online_page_present: false,
                clear_vmsa_ok: true,
            },
            SnpKexecCpu {
                cpu: 2,
                present: true,
                apic_id: 0x12,
                vmsa_pa: 0x30000,
                ghcb_pa: base + 0x230000,
                ghcb_mapping_size: PAGE_SIZE,
                ghcb_mapping_level: 1,
                online_page_present: false,
                clear_vmsa_ok: false,
            },
        ];
        let direct_map = [
            SnpKexecMemoryRange {
                addr: base,
                size: PMD_SIZE,
                level: 2,
                present: true,
                decrypted: true,
            },
            SnpKexecMemoryRange {
                addr: base + PMD_SIZE,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: true,
            },
            SnpKexecMemoryRange {
                addr: base + PMD_SIZE + PAGE_SIZE,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: false,
            },
        ];
        let bss = [
            SnpKexecMemoryRange {
                addr: bss_start,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: true,
            },
            SnpKexecMemoryRange {
                addr: bss_start + PAGE_SIZE,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: false,
            },
        ];

        let plan = snp_kexec_finish_plan(SnpKexecFinishInputs {
            sev_snp: true,
            kexec_core_enabled: true,
            this_cpu: 0,
            snp_vmpl: 0,
            sev_features: 0x55,
            direct_map: &direct_map,
            bss_decrypted: &bss,
            bss_start,
            bss_end: bss_start + 2 * PAGE_SIZE,
            possible_cpus: &cpus,
        })
        .unwrap();

        assert_eq!(plan.skipped, SnpKexecFinishSkip::None);
        assert_eq!(plan.action_count, 16);
        assert_eq!(
            plan.actions[0],
            Some(SnpKexecAction::MarkCurrentVmsaOffline {
                cpu: 0,
                vmsa_pa: 0x10000,
                online_page_found: true,
            })
        );
        assert_eq!(
            plan.actions[1],
            Some(SnpKexecAction::DestroyAp {
                cpu: 2,
                apic_id: 0x12,
                vmsa_pa: 0x30000,
                plan: SevApControlPlan {
                    event: SVM_VMGEXIT_AP_DESTROY,
                    apic_id: 0x12,
                    snp_vmpl: 0,
                    vmsa_pa: 0x30000,
                    sw_exit_code: SVM_VMGEXIT_AP_CREATION,
                    sw_exit_info_1: (0x12_u64 << 32) | SVM_VMGEXIT_AP_DESTROY,
                    sw_exit_info_2: 0x30000,
                    ghcb_rax: None,
                },
            })
        );
        assert_eq!(
            plan.actions[2],
            Some(SnpKexecAction::ClearVmsa {
                cpu: 2,
                apic_id: 0x12,
                vmsa_pa: 0x30000,
                plan: SnpSetVmsaPlan::Rmpadjust {
                    vmsa_va: 0x30000,
                    page_size: RMP_PG_SIZE_4K,
                    attrs: 1,
                },
            })
        );
        assert_eq!(
            plan.actions[3],
            Some(SnpKexecAction::LeakVmsa {
                cpu: 2,
                vmsa_pa: 0x30000,
            })
        );
        assert_eq!(
            plan.actions[4],
            Some(SnpKexecAction::SetPteEncrypted {
                addr: base + PMD_SIZE,
                size: PAGE_SIZE,
                level: 1,
            })
        );
        assert_eq!(
            plan.actions[5],
            Some(SnpKexecAction::SetMemoryPrivate {
                addr: base + PMD_SIZE,
                npages: 1,
            })
        );
        assert_eq!(
            plan.actions[6],
            Some(SnpKexecAction::SetPteEncrypted {
                addr: bss_start,
                size: PAGE_SIZE,
                level: 1,
            })
        );
        assert_eq!(
            plan.actions[7],
            Some(SnpKexecAction::SetMemoryPrivate {
                addr: bss_start,
                npages: 2,
            })
        );
        assert_eq!(plan.actions[8], Some(SnpKexecAction::FlushTlbAll));
        assert_eq!(plan.actions[9], Some(SnpKexecAction::DisableGhcbProtocol));
        assert_eq!(
            plan.actions[10],
            Some(SnpKexecAction::SetPteEncrypted {
                addr: base,
                size: PMD_SIZE,
                level: 2,
            })
        );
        assert_eq!(
            plan.actions[11],
            Some(SnpKexecAction::SetMemoryPrivate {
                addr: base,
                npages: PMD_PAGES,
            })
        );
    }

    #[test]
    fn snp_kexec_finish_shuts_down_present_cpus_but_tracks_possible_ghcbs() {
        let _guard = sev_core_test_guard();
        let base = crate::arch::x86::mm::paging::PAGE_OFFSET;
        let cpus = [
            SnpKexecCpu {
                cpu: 0,
                present: true,
                apic_id: 0x10,
                vmsa_pa: 0x10000,
                ghcb_pa: base + 0x800,
                ghcb_mapping_size: PAGE_SIZE,
                ghcb_mapping_level: 1,
                online_page_present: true,
                clear_vmsa_ok: true,
            },
            SnpKexecCpu {
                cpu: 1,
                present: false,
                apic_id: 0x11,
                vmsa_pa: 0x20000,
                ghcb_pa: base + PAGE_SIZE + 0x800,
                ghcb_mapping_size: PAGE_SIZE,
                ghcb_mapping_level: 1,
                online_page_present: false,
                clear_vmsa_ok: true,
            },
        ];
        let direct_map = [
            SnpKexecMemoryRange {
                addr: base,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: true,
            },
            SnpKexecMemoryRange {
                addr: base + PAGE_SIZE,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: true,
            },
            SnpKexecMemoryRange {
                addr: base + 2 * PAGE_SIZE,
                size: PAGE_SIZE,
                level: 1,
                present: true,
                decrypted: true,
            },
        ];

        let plan = snp_kexec_finish_plan(SnpKexecFinishInputs {
            sev_snp: true,
            kexec_core_enabled: true,
            this_cpu: 0,
            snp_vmpl: 0,
            sev_features: 0,
            direct_map: &direct_map,
            bss_decrypted: &[],
            bss_start: base + 4 * PAGE_SIZE,
            bss_end: base + 4 * PAGE_SIZE,
            possible_cpus: &cpus,
        })
        .unwrap();

        assert_eq!(
            plan.actions[0],
            Some(SnpKexecAction::MarkCurrentVmsaOffline {
                cpu: 0,
                vmsa_pa: 0x10000,
                online_page_found: true,
            })
        );
        assert!(
            !plan.actions[..plan.action_count]
                .iter()
                .any(|action| matches!(action, Some(SnpKexecAction::DestroyAp { cpu: 1, .. })))
        );
        assert_eq!(
            plan.actions[1],
            Some(SnpKexecAction::SetPteEncrypted {
                addr: base + 2 * PAGE_SIZE,
                size: PAGE_SIZE,
                level: 1,
            })
        );
        assert_eq!(
            plan.actions[2],
            Some(SnpKexecAction::SetMemoryPrivate {
                addr: base + 2 * PAGE_SIZE,
                npages: 1,
            })
        );
        assert_eq!(plan.actions[4], Some(SnpKexecAction::FlushTlbAll));
        assert_eq!(plan.actions[5], Some(SnpKexecAction::DisableGhcbProtocol));
        assert_eq!(plan.action_count, 10);
    }

    #[test]
    fn snp_kexec_finish_keeps_linux_wrapping_range_math() {
        let _guard = sev_core_test_guard();
        let direct_map = [SnpKexecMemoryRange {
            addr: u64::MAX - PAGE_SIZE + 1,
            size: 3 * PAGE_SIZE,
            level: 1,
            present: true,
            decrypted: true,
        }];
        let cpus = [SnpKexecCpu {
            cpu: 0,
            present: false,
            apic_id: 0,
            vmsa_pa: 0,
            ghcb_pa: u64::MAX - 0x800,
            ghcb_mapping_size: PAGE_SIZE,
            ghcb_mapping_level: 1,
            online_page_present: false,
            clear_vmsa_ok: true,
        }];

        let plan = snp_kexec_finish_plan(SnpKexecFinishInputs {
            sev_snp: true,
            kexec_core_enabled: true,
            this_cpu: 0,
            snp_vmpl: 0,
            sev_features: 0,
            direct_map: &direct_map,
            bss_decrypted: &[],
            bss_start: 0x4000,
            bss_end: 0x3000,
            possible_cpus: &cpus,
        })
        .unwrap();

        assert_eq!(
            plan.actions[0],
            Some(SnpKexecAction::SetPteEncrypted {
                addr: direct_map[0].addr,
                size: direct_map[0].size,
                level: 1,
            })
        );
        assert_eq!(
            plan.actions[1],
            Some(SnpKexecAction::SetMemoryPrivate {
                addr: direct_map[0].addr,
                npages: 3,
            })
        );
        assert_eq!(
            plan.actions[2],
            Some(SnpKexecAction::SetMemoryPrivate {
                addr: 0x4000,
                npages: (0x3000u64.wrapping_sub(0x4000) >> PAGE_SHIFT) as usize,
            })
        );
        assert_eq!(plan.actions[3], Some(SnpKexecAction::FlushTlbAll));
        assert_eq!(plan.actions[4], Some(SnpKexecAction::DisableGhcbProtocol));
    }

    #[test]
    fn psc_desc_caps_entries_like_linux_header() {
        let _guard = sev_core_test_guard();
        let desc = snp_set_memory_shared(0x2000, 2).unwrap();
        assert_eq!(desc.first.count, 2);
        assert_eq!(desc.first.entries[0].unwrap().gfn, 2);
        assert_eq!(
            desc.first.entries[0].unwrap().operation,
            VMGEXIT_PSC_OP_SHARED
        );
        assert!(build_psc_desc(0, VMGEXIT_PSC_MAX_ENTRY + 1, SnpPageOp::Shared).is_err());
    }

    #[test]
    fn private_and_shared_requests_use_linux_page_states() {
        let _guard = sev_core_test_guard();
        assert_eq!(SnpPageOp::Private.msr_state(), SnpPageState::Private);
        assert_eq!(SnpPageOp::Shared.msr_state(), SnpPageState::Shared);
        assert_eq!(
            snp_page_state_msr(0x123, SnpPageOp::Private.msr_state()) & 0xfff,
            0x14
        );
    }

    #[test]
    fn set_pages_state_plan_uses_linux_early_msr_fallback_without_boot_ghcb() {
        let _guard = sev_core_test_guard();
        let direct = crate::arch::x86::mm::paging::PAGE_OFFSET + 0x1234;

        assert_eq!(
            set_pages_state_plan(direct, 3, SnpPageOp::Shared, false, 0xdead, 0xbeef).unwrap(),
            SnpSetPagesStatePath::EarlyMsr(SnpEarlySetPagesStatePlan {
                vaddr: crate::arch::x86::mm::paging::PAGE_OFFSET + 0x1000,
                paddr: 0x1000,
                npages: 3,
                op: VMGEXIT_PSC_OP_SHARED,
                ca: 0xdead,
                caa_pa: 0xbeef,
            })
        );
    }

    #[test]
    fn set_pages_state_plan_uses_ghcb_psc_when_boot_ghcb_exists() {
        let _guard = sev_core_test_guard();

        let plan = set_pages_state_plan(0x2000, 2, SnpPageOp::Private, true, 0, 0).unwrap();

        let SnpSetPagesStatePath::Ghcb(plan) = plan else {
            panic!("expected GHCB PSC path");
        };
        assert_eq!(plan.first.count, 2);
        assert_eq!(plan.first.entries[0].unwrap().gfn, 2);
        assert_eq!(
            plan.first.entries[0].unwrap().operation,
            VMGEXIT_PSC_OP_PRIVATE
        );
        assert!(!plan.pvalidate_before);
        assert!(plan.pvalidate_after);
    }

    #[test]
    fn snp_set_memory_public_plans_preserve_linux_snp_feature_gate() {
        let _guard = sev_core_test_guard();
        let not_snp = CcPlatformState::default();
        assert_eq!(
            snp_set_memory_shared_linux_plan(not_snp, 0x1000, 1, true, 0, 0).unwrap(),
            SnpMemoryStatePlan::SkippedNotSnp
        );
        assert_eq!(
            snp_set_memory_private_linux_plan(not_snp, 0x1000, 1, false, 0, 0).unwrap(),
            SnpMemoryStatePlan::SkippedNotSnp
        );

        let snp = CcPlatformState {
            vendor: crate::arch::x86::coco::core::CcVendor::Amd,
            sev_status: crate::arch::x86::coco::core::MSR_AMD64_SEV_SNP_ENABLED,
            ..Default::default()
        };
        assert_eq!(
            snp_set_memory_shared_linux_plan(snp, 0x1000, 1, false, 0x44, 0x55).unwrap(),
            SnpMemoryStatePlan::SetPages(SnpSetPagesStatePath::EarlyMsr(
                SnpEarlySetPagesStatePlan {
                    vaddr: 0x1000,
                    paddr: 0x1000,
                    npages: 1,
                    op: VMGEXIT_PSC_OP_SHARED,
                    ca: 0x44,
                    caa_pa: 0x55,
                }
            ))
        );

        let private = snp_set_memory_private_linux_plan(snp, 0x2000, 1, true, 0, 0).unwrap();
        let SnpMemoryStatePlan::SetPages(SnpSetPagesStatePath::Ghcb(private)) = private else {
            panic!("expected GHCB PSC path");
        };
        assert_eq!(
            private.first.entries[0].unwrap().operation,
            VMGEXIT_PSC_OP_PRIVATE
        );
    }

    #[test]
    fn psc_plan_uses_physical_gfn_for_mapped_kernel_virtual_address() {
        use crate::arch::x86::mm::paging;
        use crate::mm::test_lock::GLOBAL_HW_TEST_LOCK as TEST_LOCK;

        let _sev_guard = sev_core_test_guard();
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe {
            paging::reset_test_pool();
        }
        let virt = 0xffff_ffff_82c0_0000;
        let phys = 0x0000_0000_0140_0000;
        unsafe {
            paging::map_kernel_page(virt, phys, paging::PAGE_KERNEL);
        }

        let plan = snp_set_memory_shared(virt, 1).unwrap();
        assert_eq!(plan.first.count, 1);
        assert_eq!(
            plan.first.entries[0].unwrap().gfn,
            phys >> paging::PAGE_SHIFT
        );
        assert_eq!(plan.first.entries[0].unwrap().pagesize, RMP_PG_SIZE_4K);
        assert!(plan.pvalidate_before);
        assert!(!plan.pvalidate_after);
    }

    #[test]
    fn psc_plan_coalesces_two_megabyte_aligned_spans() {
        let _guard = sev_core_test_guard();
        let plan = snp_set_memory_private(0x20_0000, 512).unwrap();
        assert_eq!(plan.first.count, 1);
        let entry = plan.first.entries[0].unwrap();
        assert_eq!(entry.gfn, 0x20_0000 >> 12);
        assert_eq!(entry.pagesize, RMP_PG_SIZE_2M);
        assert_eq!(plan.pages, 512);
        assert_eq!(plan.entry_count, 1);
        assert_eq!(plan.desc_count, 1);
        assert!(!plan.pvalidate_before);
        assert!(plan.pvalidate_after);
    }

    #[test]
    fn psc_plan_chunks_more_than_one_descriptor_like_linux_set_pages_state() {
        let _guard = sev_core_test_guard();
        let plan = snp_set_memory_shared(0x1000, VMGEXIT_PSC_MAX_ENTRY + 1).unwrap();
        assert_eq!(plan.first.count, VMGEXIT_PSC_MAX_ENTRY);
        assert_eq!(plan.entry_count, VMGEXIT_PSC_MAX_ENTRY + 1);
        assert_eq!(plan.desc_count, 2);
        assert_eq!(plan.pages, VMGEXIT_PSC_MAX_ENTRY + 1);
    }

    #[test]
    fn psc_plan_continues_past_ghcb_defined_max_count_like_linux_set_pages_state() {
        let _guard = sev_core_test_guard();
        let entries_before_tail = VMGEXIT_PSC_MAX_COUNT * VMGEXIT_PSC_MAX_ENTRY;
        let npages = entries_before_tail * PMD_PAGES + 1;

        let plan = snp_set_memory_shared(0, npages).unwrap();

        assert_eq!(plan.first.count, VMGEXIT_PSC_MAX_ENTRY);
        assert_eq!(plan.entry_count, entries_before_tail + 1);
        assert_eq!(plan.desc_count, VMGEXIT_PSC_MAX_COUNT + 1);
        assert_eq!(plan.pages, npages);
    }

    #[test]
    fn psc_issue_order_matches_linux_shared_and_private_validation() {
        let _guard = sev_core_test_guard();
        reset_psc_issue_log();
        assert_eq!(snp_set_memory_shared(0x1000, 2).unwrap().desc_count, 1);
        let (len, log) = psc_issue_log();
        assert_eq!(len, 2);
        assert_eq!(
            &log[..len],
            &[
                (PSC_EVENT_PVALIDATE_BEFORE, VMGEXIT_PSC_OP_SHARED, 2),
                (PSC_EVENT_VMGEXIT, VMGEXIT_PSC_OP_SHARED, 2),
            ]
        );

        reset_psc_issue_log();
        assert_eq!(snp_set_memory_private(0x1000, 2).unwrap().desc_count, 1);
        let (len, log) = psc_issue_log();
        assert_eq!(len, 2);
        assert_eq!(
            &log[..len],
            &[
                (PSC_EVENT_VMGEXIT, VMGEXIT_PSC_OP_PRIVATE, 2),
                (PSC_EVENT_PVALIDATE_AFTER, VMGEXIT_PSC_OP_PRIVATE, 2),
            ]
        );
    }

    #[test]
    fn psc_issue_invokes_one_vmgexit_per_descriptor_batch() {
        let _guard = sev_core_test_guard();
        reset_psc_issue_log();
        let plan = snp_set_memory_shared(0x1000, VMGEXIT_PSC_MAX_ENTRY + 1).unwrap();
        assert_eq!(plan.desc_count, 2);

        let (len, log) = psc_issue_log();
        assert_eq!(len, 4);
        assert_eq!(
            &log[..len],
            &[
                (
                    PSC_EVENT_PVALIDATE_BEFORE,
                    VMGEXIT_PSC_OP_SHARED,
                    VMGEXIT_PSC_MAX_ENTRY
                ),
                (
                    PSC_EVENT_VMGEXIT,
                    VMGEXIT_PSC_OP_SHARED,
                    VMGEXIT_PSC_MAX_ENTRY
                ),
                (PSC_EVENT_PVALIDATE_BEFORE, VMGEXIT_PSC_OP_SHARED, 1),
                (PSC_EVENT_VMGEXIT, VMGEXIT_PSC_OP_SHARED, 1),
            ]
        );
    }

    #[test]
    fn ghcb_psc_exchange_copies_descriptor_sets_scratch_and_completes() {
        let _guard = sev_core_test_guard();
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x500,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_SHARED,
            current_page: true,
        })
        .unwrap();
        desc.push(PscEntry {
            gfn: 0x501,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_SHARED,
            current_page: true,
        })
        .unwrap();

        reset_ghcb_psc_log();
        assert_eq!(vmgexit_psc(&desc, SnpPageOp::Shared), Ok(()));
        let (len, log) = ghcb_psc_log();
        assert_eq!(len, 1);
        assert_eq!(log[0].copied_count, 2);
        assert_eq!(log[0].cur_entry, 0);
        assert_eq!(log[0].end_entry, 1);
        assert_ne!(log[0].sw_scratch, 0);
    }

    #[test]
    fn ghcb_psc_exchange_rejects_hypervisor_error_reserved_and_end_growth() {
        let _guard = sev_core_test_guard();
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x600,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_SHARED,
            current_page: true,
        })
        .unwrap();

        set_test_ghcb_psc_mode(TEST_GHCB_PSC_EXIT_INFO);
        assert_eq!(vmgexit_psc(&desc, SnpPageOp::Shared), Err(EOPNOTSUPP));
        set_test_ghcb_psc_mode(TEST_GHCB_PSC_RESERVED);
        assert_eq!(vmgexit_psc(&desc, SnpPageOp::Shared), Err(EOPNOTSUPP));
        set_test_ghcb_psc_mode(TEST_GHCB_PSC_END_GROWTH);
        assert_eq!(vmgexit_psc(&desc, SnpPageOp::Shared), Err(EOPNOTSUPP));
        set_test_ghcb_psc_mode(TEST_GHCB_PSC_COMPLETE);
    }

    #[test]
    fn pvalidate_pages_uses_shared_false_and_private_true_actions() {
        let _guard = sev_core_test_guard();
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x200,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_SHARED,
            current_page: true,
        })
        .unwrap();

        reset_pvalidate_log();
        assert_eq!(pvalidate_pages(&desc, SnpPageOp::Shared, true), Ok(()));
        let (total, log) = pvalidate_log();
        assert_eq!(total, 1);
        assert_eq!(log[0], (0x200, RMP_PG_SIZE_4K, false));

        desc.entries[0].as_mut().unwrap().operation = VMGEXIT_PSC_OP_PRIVATE;
        reset_pvalidate_log();
        assert_eq!(pvalidate_pages(&desc, SnpPageOp::Private, false), Ok(()));
        let (total, log) = pvalidate_log();
        assert_eq!(total, 1);
        assert_eq!(log[0], (0x200, RMP_PG_SIZE_4K, true));
    }

    #[test]
    fn pvalidate_pages_retries_two_megabyte_size_mismatch_as_four_kib_pages() {
        let _guard = sev_core_test_guard();
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x400,
            pagesize: RMP_PG_SIZE_2M,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();

        reset_pvalidate_log();
        set_test_pvalidate_2m_size_mismatch(true);
        assert_eq!(pvalidate_pages(&desc, SnpPageOp::Private, false), Ok(()));
        set_test_pvalidate_2m_size_mismatch(false);

        let (total, log) = pvalidate_log();
        assert_eq!(total, PMD_PAGES + 1);
        assert_eq!(log[0], (0x400, RMP_PG_SIZE_2M, true));
        assert_eq!(log[1], (0x400, RMP_PG_SIZE_4K, true));
        assert_eq!(log[2], (0x401, RMP_PG_SIZE_4K, true));
    }

    #[test]
    fn pvalidate_pages_routes_to_svsm_when_running_above_vmpl0() {
        let _guard = sev_core_test_guard();
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x900,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();

        reset_pvalidate_log();
        crate::arch::x86::coco::sev::svsm::reset_svsm_pval_log();
        crate::arch::x86::coco::sev::svsm::set_test_svsm_pval_success(true);
        publish_snp_vmpl(1);

        assert_eq!(pvalidate_pages(&desc, SnpPageOp::Private, false), Ok(()));

        publish_snp_vmpl(0);
        crate::arch::x86::coco::sev::svsm::set_test_svsm_pval_success(false);

        let (total, _) = pvalidate_log();
        assert_eq!(total, 0);
        let (len, log) = crate::arch::x86::coco::sev::svsm::svsm_pval_log();
        assert_eq!(len, 1);
        assert_eq!(log[0], (1, 0x900, RMP_PG_SIZE_4K, true));
    }

    #[test]
    fn pvalidate_pages_cache_evict_plan_matches_linux_private_entries() {
        let _guard = sev_core_test_guard();
        let mut desc = PscDesc::empty();
        desc.push(PscEntry {
            gfn: 0x100,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_SHARED,
            current_page: true,
        })
        .unwrap();
        desc.push(PscEntry {
            gfn: 0x200,
            pagesize: RMP_PG_SIZE_4K,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: true,
        })
        .unwrap();
        desc.push(PscEntry {
            gfn: 0x400,
            pagesize: RMP_PG_SIZE_2M,
            operation: VMGEXIT_PSC_OP_PRIVATE,
            current_page: false,
        })
        .unwrap();

        let plan = pvalidate_pages_cache_evict_plan(&desc, false).unwrap();
        assert!(!plan.skipped_for_coherency);
        assert_eq!(plan.count, 2);
        assert_eq!(
            plan.entries[0],
            Some(SnpPvalidateCacheEvictEntry {
                pfn: 0x200,
                vaddr: crate::arch::x86::mm::paging::pfn_to_virt(0x200) as u64,
                pages: 1,
            })
        );
        assert_eq!(
            plan.entries[1],
            Some(SnpPvalidateCacheEvictEntry {
                pfn: 0x400,
                vaddr: crate::arch::x86::mm::paging::pfn_to_virt(0x400) as u64,
                pages: PMD_PAGES,
            })
        );

        let skipped = pvalidate_pages_cache_evict_plan(&desc, true).unwrap();
        assert!(skipped.skipped_for_coherency);
        assert_eq!(skipped.count, 0);
        assert!(skipped.entries.iter().all(Option::is_none));
    }

    fn secrets_with_keys() -> SnpSecretsPage {
        let mut secrets = SnpSecretsPage::default();
        let mut id = 0usize;
        while id < 4 {
            let mut key = [0u8; VMPCK_KEY_LEN];
            key[0] = id as u8 + 1;
            key[31] = 0xa0 | id as u8;
            secrets.set_vmpck(id, key).unwrap();
            id += 1;
        }
        secrets.os_area.msg_seqno_0 = 0x1000;
        secrets.os_area.msg_seqno_1 = 0x2000;
        secrets.os_area.msg_seqno_2 = 0x3000;
        secrets.os_area.msg_seqno_3 = 0x4000;
        secrets
    }

    #[test]
    fn snp_secrets_layout_matches_linux_header() {
        let _guard = sev_core_test_guard();
        let secrets = SnpSecretsPage::default();
        let base = core::ptr::addr_of!(secrets) as usize;
        let vmpck0_off = core::ptr::addr_of!(secrets.vmpck0) as usize - base;
        let os_area_off = core::ptr::addr_of!(secrets.os_area) as usize - base;
        let tsc_factor_off = core::ptr::addr_of!(secrets.tsc_factor) as usize - base;

        assert_eq!(MAX_AUTHTAG_LEN, 32);
        assert_eq!(AUTHTAG_LEN, 16);
        assert_eq!(AAD_LEN, 48);
        assert_eq!(MSG_HDR_VER, 1);
        assert_eq!(VMPCK_KEY_LEN, 32);
        assert_eq!(core::mem::size_of::<SecretsOsArea>(), 96);
        assert_eq!(core::mem::size_of::<SnpSecretsPage>(), PAGE_SIZE as usize);
        assert_eq!(vmpck0_off, 32);
        assert_eq!(os_area_off, 160);
        assert_eq!(tsc_factor_off, 352);
    }

    #[test]
    fn snp_guest_message_shared_page_sizing_matches_linux_helpers() {
        let _guard = sev_core_test_guard();
        assert_eq!(SNP_GUEST_MSG_SIZE, PAGE_SIZE as usize);
        assert!(SNP_GUEST_MSG_SIZE <= PAGE_SIZE as usize);

        assert_eq!(linux_page_count(1), 1);
        assert_eq!(linux_get_order(1), 0);
        assert_eq!(linux_page_count(PAGE_SIZE as usize + 1), 2);
        assert_eq!(linux_get_order(PAGE_SIZE as usize + 1), 1);
        assert_eq!(linux_page_count((PAGE_SIZE as usize * 2) + 1), 3);
        assert_eq!(linux_get_order((PAGE_SIZE as usize * 2) + 1), 2);
    }

    #[test]
    fn shared_page_alloc_and_free_plans_match_linux_memory_transitions() {
        let _guard = sev_core_test_guard();

        assert_eq!(
            alloc_shared_pages_plan(SNP_GUEST_MSG_SIZE, false, 0),
            AllocSharedPagesPlan {
                size: SNP_GUEST_MSG_SIZE,
                npages: 1,
                order: 0,
                page_allocated: false,
                set_memory_decrypted: false,
                set_memory_decrypted_rc: 0,
                freed_pages_on_decrypt_error: false,
                returned_page: false,
            }
        );

        assert_eq!(
            alloc_shared_pages_plan(PAGE_SIZE as usize + 1, true, -EIO),
            AllocSharedPagesPlan {
                size: PAGE_SIZE as usize + 1,
                npages: 2,
                order: 1,
                page_allocated: true,
                set_memory_decrypted: true,
                set_memory_decrypted_rc: -EIO,
                freed_pages_on_decrypt_error: true,
                returned_page: false,
            }
        );

        assert_eq!(
            free_shared_pages_plan(false, SNP_GUEST_MSG_SIZE, 0),
            FreeSharedPagesPlan {
                size: SNP_GUEST_MSG_SIZE,
                npages: 1,
                order: 0,
                buf_present: false,
                set_memory_encrypted: false,
                set_memory_encrypted_rc: 0,
                freed_pages: false,
                leaked_on_encrypt_error: false,
            }
        );

        assert_eq!(
            free_shared_pages_plan(true, SNP_GUEST_MSG_SIZE, -EIO),
            FreeSharedPagesPlan {
                size: SNP_GUEST_MSG_SIZE,
                npages: 1,
                order: 0,
                buf_present: true,
                set_memory_encrypted: true,
                set_memory_encrypted_rc: -EIO,
                freed_pages: false,
                leaked_on_encrypt_error: true,
            }
        );
    }

    #[test]
    fn snp_msg_alloc_plan_preserves_linux_success_and_unwind_labels() {
        let _guard = sev_core_test_guard();

        let plan = snp_msg_alloc_plan(SnpMsgAllocInputs::success());
        assert_eq!(plan.rc, 0);
        assert!(plan.success);
        assert_eq!(
            &plan.actions[..plan.action_count],
            &[
                Some(SnpMsgAllocAction::KzallocDesc),
                Some(SnpMsgAllocAction::IoremapSecrets),
                Some(SnpMsgAllocAction::AllocRequestShared),
                Some(SnpMsgAllocAction::AllocResponseShared),
            ]
        );
        assert!(plan.request_shared.unwrap().returned_page);
        assert!(plan.response_shared.unwrap().returned_page);

        let mut inputs = SnpMsgAllocInputs::success();
        inputs.request_page_alloc_ok = false;
        let plan = snp_msg_alloc_plan(inputs);
        assert_eq!(plan.rc, -ENOMEM);
        assert!(!plan.success);
        assert_eq!(
            &plan.actions[..plan.action_count],
            &[
                Some(SnpMsgAllocAction::KzallocDesc),
                Some(SnpMsgAllocAction::IoremapSecrets),
                Some(SnpMsgAllocAction::AllocRequestShared),
                Some(SnpMsgAllocAction::IounmapSecrets),
                Some(SnpMsgAllocAction::KfreeDesc),
            ]
        );
        assert_eq!(plan.response_shared, None);

        inputs = SnpMsgAllocInputs::success();
        inputs.response_set_decrypted_rc = -EIO;
        let plan = snp_msg_alloc_plan(inputs);
        assert_eq!(plan.rc, -ENOMEM);
        assert_eq!(
            &plan.actions[..plan.action_count],
            &[
                Some(SnpMsgAllocAction::KzallocDesc),
                Some(SnpMsgAllocAction::IoremapSecrets),
                Some(SnpMsgAllocAction::AllocRequestShared),
                Some(SnpMsgAllocAction::AllocResponseShared),
                Some(SnpMsgAllocAction::FreeRequestShared),
                Some(SnpMsgAllocAction::IounmapSecrets),
                Some(SnpMsgAllocAction::KfreeDesc),
            ]
        );
        assert!(plan.response_shared.unwrap().freed_pages_on_decrypt_error);
    }

    #[test]
    fn snp_msg_free_plan_matches_linux_release_order_even_on_leaked_response() {
        let _guard = sev_core_test_guard();

        let empty = snp_msg_free_plan(false, false, true, true, 0, 0);
        assert_eq!(empty.action_count, 0);
        assert_eq!(empty.response_shared, None);
        assert_eq!(empty.request_shared, None);

        let plan = snp_msg_free_plan(true, false, true, true, -EIO, 0);
        assert_eq!(
            &plan.actions[..plan.action_count],
            &[
                Some(SnpMsgFreeAction::KfreeCtx),
                Some(SnpMsgFreeAction::FreeResponseShared),
                Some(SnpMsgFreeAction::FreeRequestShared),
                Some(SnpMsgFreeAction::IounmapSecrets),
                Some(SnpMsgFreeAction::KfreeSensitiveDesc),
            ]
        );
        assert!(!plan.ctx_present);
        assert!(plan.response_shared.unwrap().leaked_on_encrypt_error);
        assert!(plan.request_shared.unwrap().freed_pages);
    }

    #[test]
    fn get_vmpck_selects_linux_key_and_seqno_slot() {
        let _guard = sev_core_test_guard();
        let secrets = secrets_with_keys();

        let selection = get_vmpck_from_secrets(&secrets, 2).unwrap();
        assert_eq!(selection.key[0], 3);
        assert_eq!(selection.key[31], 0xa2);
        assert_eq!(selection.seqno_index, 2);
        assert_eq!(
            secrets.os_area.msg_seqno(selection.seqno_index),
            Some(0x3000)
        );

        assert_eq!(get_vmpck_from_secrets(&secrets, -2), None);
        assert_eq!(get_vmpck_from_secrets(&secrets, 4), None);
    }

    #[test]
    fn snp_msg_init_uses_current_vmpl_for_default_key() {
        let _guard = sev_core_test_guard();
        let desc = snp_msg_init_with_secrets(secrets_with_keys(), -1, 3, true).unwrap();

        assert_eq!(desc.vmpck_id, 3);
        assert_eq!(desc.os_area_msg_seqno, Some(3));
        assert_eq!(desc.vmpck.unwrap()[0], 4);
        assert_eq!(
            desc.ctx,
            Some(SnpAesGcmCtx {
                key_len: VMPCK_KEY_LEN,
                authtag_len: AUTHTAG_LEN,
            })
        );
    }

    #[test]
    fn snp_msg_init_matches_linux_error_ordering() {
        let _guard = sev_core_test_guard();
        let mut invalid = SnpMsgDesc::new(secrets_with_keys());
        assert_eq!(snp_msg_init_desc(&mut invalid, 4, 0, true), Err(EINVAL));
        assert_eq!(invalid.vmpck, None);
        assert_eq!(invalid.os_area_msg_seqno, None);

        let mut zero = SnpMsgDesc::new(SnpSecretsPage::default());
        assert_eq!(snp_msg_init_desc(&mut zero, 1, 0, true), Err(EINVAL));
        assert_eq!(zero.os_area_msg_seqno, Some(1));
        assert_eq!(zero.vmpck, Some([0; VMPCK_KEY_LEN]));
        assert_eq!(zero.vmpck_id, -1);
        assert_eq!(zero.ctx, None);

        let mut no_crypto = SnpMsgDesc::new(secrets_with_keys());
        assert_eq!(snp_msg_init_desc(&mut no_crypto, 0, 0, false), Err(ENOMEM));
        assert_eq!(no_crypto.vmpck_id, 0);
        assert_eq!(no_crypto.os_area_msg_seqno, Some(0));
        assert_eq!(no_crypto.ctx, None);
    }

    #[test]
    fn snp_msg_init_compatibility_wrapper_uses_real_key_validation() {
        let _guard = sev_core_test_guard();
        publish_snp_vmpl(0);
        assert_eq!(snp_msg_init(0, true), Ok(0));
        assert_eq!(snp_msg_init(4, true), Err(EINVAL));
        assert_eq!(snp_msg_init(0, false), Err(EOPNOTSUPP));
    }

    #[test]
    fn snp_disable_vmpck_zeros_linux_secret_key_and_nulls_descriptor_key() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        assert!(desc.vmpck.unwrap().iter().any(|byte| *byte != 0));

        assert_eq!(snp_disable_vmpck_desc(&mut desc), Ok(()));

        assert_eq!(desc.vmpck, None);
        assert_eq!(desc.secrets.vmpck(1), Some([0; VMPCK_KEY_LEN]));
        assert_eq!(desc.secrets.vmpck(0).unwrap()[0], 1);
        assert_eq!(desc.vmpck_id, 1);
    }

    #[test]
    fn snp_msg_sequence_numbers_match_linux_storage_rules() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 2, 0, true).unwrap();

        assert_eq!(__snp_get_msg_seqno_desc(&desc), Ok(0x3001));
        assert_eq!(snp_get_msg_seqno_desc(&desc), Ok(0x3001));

        assert_eq!(snp_inc_msg_seqno_desc(&mut desc), Ok(()));
        assert_eq!(desc.secrets.os_area.msg_seqno(2), Some(0x3002));
        assert_eq!(snp_get_msg_seqno_desc(&desc), Ok(0x3003));
    }

    #[test]
    fn snp_msg_sequence_overflow_returns_zero_like_linux() {
        let _guard = sev_core_test_guard();
        let mut secrets = secrets_with_keys();
        secrets.os_area.msg_seqno_0 = u32::MAX - 2;
        let mut desc = snp_msg_init_with_secrets(secrets, 0, 0, true).unwrap();

        assert_eq!(
            snp_get_msg_seqno_desc(&desc),
            Ok(SNP_MSG_SEQNO_OVERFLOW_LIMIT - 1)
        );

        desc.secrets.os_area.msg_seqno_0 = u32::MAX - 1;
        assert_eq!(__snp_get_msg_seqno_desc(&desc), Ok(u32::MAX as u64));
        assert_eq!(snp_get_msg_seqno_desc(&desc), Ok(0));

        desc.secrets.os_area.msg_seqno_0 = u32::MAX;
        assert_eq!(snp_inc_msg_seqno_desc(&mut desc), Ok(()));
        assert_eq!(desc.secrets.os_area.msg_seqno_0, 1);
    }

    #[test]
    fn snp_msg_sequence_helpers_reject_uninitialized_descriptor_state() {
        let _guard = sev_core_test_guard();
        let mut desc = SnpMsgDesc::new(secrets_with_keys());

        assert_eq!(__snp_get_msg_seqno_desc(&desc), Err(EINVAL));
        assert_eq!(snp_get_msg_seqno_desc(&desc), Err(EINVAL));
        assert_eq!(snp_inc_msg_seqno_desc(&mut desc), Err(EINVAL));
        assert_eq!(snp_disable_vmpck_desc(&mut desc), Err(EINVAL));
    }

    fn guest_req(payload: &[u8], resp_sz: usize) -> SnpGuestReq {
        let mut req = SnpGuestReq {
            req_sz: payload.len(),
            resp_sz,
            vmpck_id: 1,
            msg_version: 2,
            msg_type: 5,
            ..Default::default()
        };
        req.req_buf[..payload.len()].copy_from_slice(payload);
        req
    }

    fn encrypt_response(desc: &mut SnpMsgDesc, plaintext: &[u8]) {
        let (ctx, _) = snp_linux_aesgcm_ctx(desc).unwrap();
        let req_hdr = desc.secret_request.hdr;
        desc.response = SnpGuestMsg::default();
        desc.response.hdr.algo = SNP_AEAD_AES_256_GCM;
        desc.response.hdr.hdr_version = MSG_HDR_VER;
        desc.response.hdr.hdr_sz = SNP_GUEST_MSG_HDR_SIZE as u16;
        desc.response.hdr.msg_type = req_hdr.msg_type + 1;
        desc.response.hdr.msg_version = req_hdr.msg_version;
        desc.response.hdr.msg_seqno = req_hdr.msg_seqno + 1;
        desc.response.hdr.msg_vmpck = req_hdr.msg_vmpck;
        desc.response.hdr.msg_sz = plaintext.len() as u16;

        let aad = desc.response.hdr.aad_bytes();
        let iv = desc.response.hdr.iv_from_seqno();
        crate::lib::crypto::aesgcm::aesgcm_encrypt(
            &ctx,
            &mut desc.response.payload[..plaintext.len()],
            plaintext,
            &aad,
            &iv,
            &mut desc.response.hdr.authtag,
        );
    }

    #[test]
    fn snp_guest_message_layout_matches_linux_header() {
        let _guard = sev_core_test_guard();
        let hdr = SnpGuestMsgHeader {
            algo: SNP_AEAD_AES_256_GCM,
            hdr_version: MSG_HDR_VER,
            hdr_sz: SNP_GUEST_MSG_HDR_SIZE as u16,
            msg_type: 7,
            msg_version: 3,
            msg_sz: 0x1234,
            rsvd2: 0x5566_7788,
            msg_vmpck: 2,
            rsvd3: [0xa5; 35],
            ..Default::default()
        };
        let aad = hdr.aad_bytes();

        assert_eq!(
            core::mem::size_of::<SnpGuestMsgHeader>(),
            SNP_GUEST_MSG_HDR_SIZE
        );
        assert_eq!(core::mem::size_of::<SnpGuestMsg>(), PAGE_SIZE as usize);
        assert_eq!(SNP_GUEST_MSG_PAYLOAD_SIZE, 4000);
        assert_eq!(SNP_MSG_TSC_INFO_REQ, 17);
        assert_eq!(SNP_MSG_TSC_INFO_RSP, 18);
        assert_eq!(core::mem::size_of::<SnpTscInfoReq>(), 128);
        assert_eq!(core::mem::size_of::<SnpTscInfoResp>(), 128);
        assert_eq!(SNP_TSC_INFO_RESP_BUF_SZ, 144);
        assert_eq!(
            &aad[..13],
            &[1, 1, 96, 0, 7, 3, 0x34, 0x12, 0x88, 0x77, 0x66, 0x55, 2]
        );
        assert_eq!(&aad[13..], &[0xa5; 35]);
        assert!(snp_guest_msg_size_with_auth_fits(32, 16, 48));
        assert!(!snp_guest_msg_size_with_auth_fits(33, 16, 48));
        assert!(snp_guest_msg_size_with_auth_fits(usize::MAX - 2, 16, 15));
    }

    #[test]
    fn enc_payload_builds_linux_header_and_encrypts_request_payload() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let req = guest_req(&[0x10, 0x20, 0x30], AUTHTAG_LEN + 4);

        assert_eq!(enc_payload_desc(&mut desc, 0x44, &req), Ok(()));

        let hdr = desc.secret_request.hdr;
        assert_eq!(hdr.algo, SNP_AEAD_AES_256_GCM);
        assert_eq!(hdr.hdr_version, MSG_HDR_VER);
        assert_eq!(hdr.hdr_sz as usize, SNP_GUEST_MSG_HDR_SIZE);
        assert_eq!(hdr.msg_type, req.msg_type);
        assert_eq!(hdr.msg_version, req.msg_version);
        assert_eq!(hdr.msg_seqno, 0x44);
        assert_eq!(hdr.msg_vmpck, 1);
        assert_eq!(hdr.msg_sz, 3);
        assert_ne!(&desc.secret_request.payload[..3], &req.req_buf[..3]);
        assert!(hdr.authtag[..AUTHTAG_LEN].iter().any(|byte| *byte != 0));

        assert_eq!(enc_payload_desc(&mut desc, 0, &req), Err(ENOSR));

        let mut too_large = guest_req(&[], AUTHTAG_LEN);
        too_large.req_sz = SNP_GUEST_MSG_PAYLOAD_SIZE - AUTHTAG_LEN + 1;
        assert_eq!(enc_payload_desc(&mut desc, 1, &too_large), Err(EBADMSG));
    }

    #[test]
    fn verify_and_dec_payload_checks_linux_response_header_then_decrypts() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[1, 2, 3, 4], AUTHTAG_LEN + 3);
        enc_payload_desc(&mut desc, 9, &req).unwrap();
        encrypt_response(&mut desc, &[0xaa, 0xbb, 0xcc]);

        assert_eq!(verify_and_dec_payload_desc(&mut desc, &mut req), Ok(()));
        assert_eq!(&req.resp_buf[..3], &[0xaa, 0xbb, 0xcc]);
        assert_eq!(desc.secret_response.hdr.msg_seqno, 10);
    }

    #[test]
    fn verify_and_dec_payload_rejects_linux_bad_response_cases() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[1, 2, 3, 4], AUTHTAG_LEN + 3);
        enc_payload_desc(&mut desc, 9, &req).unwrap();
        encrypt_response(&mut desc, &[0xaa, 0xbb, 0xcc]);

        desc.response.hdr.msg_seqno = 11;
        assert_eq!(
            verify_and_dec_payload_desc(&mut desc, &mut req),
            Err(EBADMSG)
        );

        encrypt_response(&mut desc, &[0xaa, 0xbb, 0xcc]);
        desc.response.hdr.msg_type = desc.secret_request.hdr.msg_type;
        assert_eq!(
            verify_and_dec_payload_desc(&mut desc, &mut req),
            Err(EBADMSG)
        );

        encrypt_response(&mut desc, &[0xaa, 0xbb, 0xcc]);
        req.resp_sz = AUTHTAG_LEN + 2;
        assert_eq!(
            verify_and_dec_payload_desc(&mut desc, &mut req),
            Err(EBADMSG)
        );

        req.resp_sz = AUTHTAG_LEN + 3;
        encrypt_response(&mut desc, &[0xaa, 0xbb, 0xcc]);
        desc.response.hdr.authtag[0] ^= 0xff;
        assert_eq!(
            verify_and_dec_payload_desc(&mut desc, &mut req),
            Err(EBADMSG)
        );
    }

    fn ghcb_req(exit_code: u64) -> SnpGuestReq {
        let mut req = guest_req(&[0x10], AUTHTAG_LEN + 1);
        req.exit_code = exit_code;
        req.input.req_gpa = 0x1111;
        req.input.resp_gpa = 0x2222;
        req.input.data_gpa = 0x3333;
        req.input.data_npages = 4;
        req
    }

    #[test]
    fn issue_guest_request_sets_linux_ghcb_register_plan_for_extended_only() {
        let _guard = sev_core_test_guard();
        let mut ext = ghcb_req(SVM_VMGEXIT_EXT_GUEST_REQUEST);
        let (plan, rc) =
            snp_issue_guest_request_from_ghcb(&mut ext, SnpIssueGuestRequestGhcb::default());
        assert_eq!(rc, Ok(()));
        assert_eq!(
            plan,
            SnpIssueGuestRequestPlan {
                exit_code: SVM_VMGEXIT_EXT_GUEST_REQUEST,
                req_gpa: 0x1111,
                resp_gpa: 0x2222,
                rax: Some(0x3333),
                rbx: Some(4),
            }
        );
        assert_eq!(ext.exitinfo2, 0);

        let mut standard = ghcb_req(SVM_VMGEXIT_GUEST_REQUEST);
        let (plan, rc) =
            snp_issue_guest_request_from_ghcb(&mut standard, SnpIssueGuestRequestGhcb::default());
        assert_eq!(rc, Ok(()));
        assert_eq!(plan.rax, None);
        assert_eq!(plan.rbx, None);
    }

    #[test]
    fn issue_guest_request_keeps_no_fw_call_on_missing_ghcb_or_hv_error() {
        let _guard = sev_core_test_guard();
        let mut req = ghcb_req(SVM_VMGEXIT_GUEST_REQUEST);
        let (_, rc) = snp_issue_guest_request_from_ghcb(
            &mut req,
            SnpIssueGuestRequestGhcb {
                ghcb_available: false,
                ..Default::default()
            },
        );
        assert_eq!(rc, Err(EIO));
        assert_eq!(req.exitinfo2, SEV_RET_NO_FW_CALL);

        let mut req = ghcb_req(SVM_VMGEXIT_GUEST_REQUEST);
        let (_, rc) = snp_issue_guest_request_from_ghcb(
            &mut req,
            SnpIssueGuestRequestGhcb {
                hv_call_result: Err(EINVAL),
                sw_exit_info_2: 0,
                ..Default::default()
            },
        );
        assert_eq!(rc, Err(EINVAL));
        assert_eq!(req.exitinfo2, SEV_RET_NO_FW_CALL);
    }

    #[test]
    fn issue_guest_request_maps_linux_sw_exit_info2_values() {
        let _guard = sev_core_test_guard();
        let mut req = ghcb_req(SVM_VMGEXIT_GUEST_REQUEST);
        let (_, rc) = snp_issue_guest_request_from_ghcb(
            &mut req,
            SnpIssueGuestRequestGhcb {
                sw_exit_info_2: snp_guest_vmm_err(SNP_GUEST_VMM_ERR_BUSY),
                ..Default::default()
            },
        );
        assert_eq!(rc, Err(EAGAIN));
        assert_eq!(req.exitinfo2, snp_guest_vmm_err(SNP_GUEST_VMM_ERR_BUSY));

        let mut req = ghcb_req(SVM_VMGEXIT_GUEST_REQUEST);
        let (_, rc) = snp_issue_guest_request_from_ghcb(
            &mut req,
            SnpIssueGuestRequestGhcb {
                sw_exit_info_2: 0xdead_beef,
                ..Default::default()
            },
        );
        assert_eq!(rc, Err(EIO));
        assert_eq!(req.exitinfo2, 0xdead_beef);
    }

    #[test]
    fn issue_guest_request_invalid_len_updates_pages_only_for_extended_request() {
        let _guard = sev_core_test_guard();
        let invalid_len = snp_guest_vmm_err(SNP_GUEST_VMM_ERR_INVALID_LEN);

        let mut ext = ghcb_req(SVM_VMGEXIT_EXT_GUEST_REQUEST);
        let (_, rc) = snp_issue_guest_request_from_ghcb(
            &mut ext,
            SnpIssueGuestRequestGhcb {
                sw_exit_info_2: invalid_len,
                rbx: 9,
                ..Default::default()
            },
        );
        assert_eq!(rc, Err(ENOSPC));
        assert_eq!(ext.exitinfo2, invalid_len);
        assert_eq!(ext.input.data_npages, 9);

        let mut standard = ghcb_req(SVM_VMGEXIT_GUEST_REQUEST);
        let (_, rc) = snp_issue_guest_request_from_ghcb(
            &mut standard,
            SnpIssueGuestRequestGhcb {
                sw_exit_info_2: invalid_len,
                rbx: 9,
                ..Default::default()
            },
        );
        assert_eq!(rc, Err(EIO));
        assert_eq!(standard.input.data_npages, 4);
    }

    struct SuccessBackend {
        calls: usize,
        response: [u8; 8],
        response_len: usize,
    }

    impl SnpGuestRequestBackend for SuccessBackend {
        fn issue_guest_request(
            &mut self,
            mdesc: &mut SnpMsgDesc,
            req: &mut SnpGuestReq,
        ) -> Result<(), i32> {
            self.calls += 1;
            req.exitinfo2 = 0;
            encrypt_response(mdesc, &self.response[..self.response_len]);
            Ok(())
        }
    }

    #[test]
    fn snp_send_guest_request_success_matches_linux_side_effects() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[0x51, 0x52, 0x53], AUTHTAG_LEN + 3);
        req.exit_code = SVM_VMGEXIT_GUEST_REQUEST;
        req.certs_data_present = true;
        req.certs_data_gpa = 0xfeed_cafe;
        let mut backend = SuccessBackend {
            calls: 0,
            response: [0xa1, 0xa2, 0xa3, 0, 0, 0, 0, 0],
            response_len: 3,
        };

        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Ok(())
        );

        assert_eq!(backend.calls, 1);
        assert_eq!(desc.secrets.os_area.msg_seqno(1), Some(0x2002));
        assert_eq!(req.input.req_gpa, core::ptr::addr_of!(desc.request) as u64);
        assert_eq!(
            req.input.resp_gpa,
            core::ptr::addr_of!(desc.response) as u64
        );
        assert_eq!(req.input.data_gpa, 0xfeed_cafe);
        assert_eq!(desc.request, desc.secret_request);
        assert_eq!(&req.resp_buf[..3], &[0xa1, 0xa2, 0xa3]);
        assert!(desc.vmpck.is_some());
    }

    struct EnospcThenSuccessBackend {
        calls: usize,
        expected_pages: u32,
    }

    impl SnpGuestRequestBackend for EnospcThenSuccessBackend {
        fn issue_guest_request(
            &mut self,
            mdesc: &mut SnpMsgDesc,
            req: &mut SnpGuestReq,
        ) -> Result<(), i32> {
            self.calls += 1;
            if self.calls == 1 {
                req.input.data_npages = self.expected_pages;
                return Err(ENOSPC);
            }

            assert_eq!(req.exit_code, SVM_VMGEXIT_GUEST_REQUEST);
            encrypt_response(mdesc, &[0x99]);
            Ok(())
        }
    }

    #[test]
    fn handle_guest_request_enospc_fallback_preserves_vmpck_and_returns_eio() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[0x10], AUTHTAG_LEN + 1);
        req.exit_code = SVM_VMGEXIT_EXT_GUEST_REQUEST;
        req.input.data_npages = 1;
        let mut backend = EnospcThenSuccessBackend {
            calls: 0,
            expected_pages: 9,
        };

        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(EIO)
        );

        assert_eq!(backend.calls, 2);
        assert_eq!(req.exit_code, SVM_VMGEXIT_GUEST_REQUEST);
        assert_eq!(
            req.exitinfo2,
            snp_guest_vmm_err(SNP_GUEST_VMM_ERR_INVALID_LEN)
        );
        assert_eq!(req.input.data_npages, 9);
        assert_eq!(desc.secrets.os_area.msg_seqno(1), Some(0x2002));
        assert!(desc.vmpck.is_some());
    }

    struct EagainTimeoutBackend {
        calls: usize,
    }

    impl SnpGuestRequestBackend for EagainTimeoutBackend {
        fn issue_guest_request(
            &mut self,
            _mdesc: &mut SnpMsgDesc,
            _req: &mut SnpGuestReq,
        ) -> Result<(), i32> {
            self.calls += 1;
            Err(EAGAIN)
        }

        fn retry_timed_out(&mut self, _attempts: usize) -> bool {
            true
        }
    }

    #[test]
    fn send_guest_request_timeout_increments_seqno_then_disables_vmpck() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[0x10], AUTHTAG_LEN + 1);
        req.exit_code = SVM_VMGEXIT_GUEST_REQUEST;
        let mut backend = EagainTimeoutBackend { calls: 0 };

        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(ETIMEDOUT)
        );

        assert_eq!(backend.calls, 1);
        assert_eq!(desc.secrets.os_area.msg_seqno(1), Some(0x2002));
        assert_eq!(desc.vmpck, None);
        assert_eq!(desc.secrets.vmpck(1), Some([0; VMPCK_KEY_LEN]));
    }

    struct AspErrorBackend;

    impl SnpGuestRequestBackend for AspErrorBackend {
        fn issue_guest_request(
            &mut self,
            _mdesc: &mut SnpMsgDesc,
            req: &mut SnpGuestReq,
        ) -> Result<(), i32> {
            req.exitinfo2 = 0xbeef;
            Err(EIO)
        }
    }

    #[test]
    fn send_guest_request_asp_error_disables_vmpck() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[0x10], AUTHTAG_LEN + 1);
        req.exit_code = SVM_VMGEXIT_GUEST_REQUEST;
        let mut backend = AspErrorBackend;

        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(EIO)
        );
        assert_eq!(desc.secrets.os_area.msg_seqno(1), Some(0x2002));
        assert_eq!(desc.vmpck, None);
    }

    struct BadResponseBackend;

    impl SnpGuestRequestBackend for BadResponseBackend {
        fn issue_guest_request(
            &mut self,
            mdesc: &mut SnpMsgDesc,
            _req: &mut SnpGuestReq,
        ) -> Result<(), i32> {
            encrypt_response(mdesc, &[0x42]);
            mdesc.response.hdr.msg_type = mdesc.secret_request.hdr.msg_type;
            Ok(())
        }
    }

    #[test]
    fn send_guest_request_decode_error_disables_vmpck() {
        let _guard = sev_core_test_guard();
        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[0x10], AUTHTAG_LEN + 1);
        req.exit_code = SVM_VMGEXIT_GUEST_REQUEST;
        let mut backend = BadResponseBackend;

        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(EBADMSG)
        );
        assert_eq!(desc.secrets.os_area.msg_seqno(1), Some(0x2002));
        assert_eq!(desc.vmpck, None);
    }

    #[test]
    fn send_guest_request_early_errors_do_not_disable_vmpck() {
        let _guard = sev_core_test_guard();
        let mut backend = SuccessBackend {
            calls: 0,
            response: [0; 8],
            response_len: 0,
        };

        let mut desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        let mut req = guest_req(&[0x10], AUTHTAG_LEN + 1);
        req.req_buf_valid = false;
        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(EINVAL)
        );
        assert!(desc.vmpck.is_some());

        req.req_buf_valid = true;
        desc.vmpck = Some([0; VMPCK_KEY_LEN]);
        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(ENOTTY)
        );
        assert_eq!(desc.vmpck, Some([0; VMPCK_KEY_LEN]));

        desc = snp_msg_init_with_secrets(secrets_with_keys(), 1, 0, true).unwrap();
        desc.secrets.os_area.msg_seqno_1 = u32::MAX - 1;
        assert_eq!(
            snp_send_guest_request_with(&mut backend, &mut desc, &mut req),
            Err(EIO)
        );
        assert!(desc.vmpck.is_some());
        assert_eq!(backend.calls, 0);
    }

    struct TscInfoBackend {
        calls: usize,
        response: SnpTscInfoResp,
        error: Option<i32>,
    }

    impl SnpGuestRequestBackend for TscInfoBackend {
        fn issue_guest_request(
            &mut self,
            mdesc: &mut SnpMsgDesc,
            req: &mut SnpGuestReq,
        ) -> Result<(), i32> {
            self.calls += 1;
            assert_eq!(req.msg_version, MSG_HDR_VER);
            assert_eq!(req.msg_type, SNP_MSG_TSC_INFO_REQ);
            assert_eq!(req.vmpck_id, snp_vmpl() as u32);
            assert_eq!(req.req_sz, SNP_TSC_INFO_REQ_SZ);
            assert_eq!(req.resp_sz, SNP_TSC_INFO_RESP_BUF_SZ);
            assert_eq!(req.exit_code, SVM_VMGEXIT_GUEST_REQUEST);
            assert_eq!(req.input.req_gpa, core::ptr::addr_of!(mdesc.request) as u64);
            assert_eq!(
                req.input.resp_gpa,
                core::ptr::addr_of!(mdesc.response) as u64
            );
            assert_eq!(req.input.data_gpa, 0);
            assert!(
                req.req_buf[..SNP_TSC_INFO_REQ_SZ]
                    .iter()
                    .all(|byte| *byte == 0)
            );

            if let Some(error) = self.error {
                return Err(error);
            }

            encrypt_response(mdesc, &self.response.to_payload());
            Ok(())
        }
    }

    #[test]
    fn snp_get_tsc_info_sends_linux_request_and_publishes_success() {
        let _guard = sev_core_test_guard();
        publish_snp_vmpl(2);
        publish_secure_tsc(0, 0, 1234);
        let resources = SnpTscInfoResources::new(secrets_with_keys());
        let mut backend = TscInfoBackend {
            calls: 0,
            response: SnpTscInfoResp {
                status: 0,
                tsc_scale: 0x0102_0304_0506_0708,
                tsc_offset: 0x1112_1314_1516_1718,
                tsc_factor: 77,
                ..Default::default()
            },
            error: None,
        };

        let resp = snp_get_tsc_info_with(&mut backend, resources).unwrap();

        assert_eq!(backend.calls, 1);
        assert_eq!(resp.status, 0);
        assert_eq!(
            snp_secure_tsc_info(),
            (0x0102_0304_0506_0708, 0x1112_1314_1516_1718, 1234)
        );
        publish_snp_vmpl(0);
    }

    #[test]
    fn snp_get_tsc_info_preserves_linux_error_ordering_and_status_failure() {
        let _guard = sev_core_test_guard();
        publish_snp_vmpl(1);
        publish_secure_tsc(0xaa, 0xbb, 0xcc);
        let mut backend = TscInfoBackend {
            calls: 0,
            response: SnpTscInfoResp {
                status: 5,
                tsc_scale: 0x10,
                tsc_offset: 0x20,
                ..Default::default()
            },
            error: None,
        };

        let mut resources = SnpTscInfoResources::new(secrets_with_keys());
        resources.tsc_req_available = false;
        assert_eq!(snp_get_tsc_info_with(&mut backend, resources), Err(ENOMEM));
        assert_eq!(backend.calls, 0);

        resources = SnpTscInfoResources::new(SnpSecretsPage::default());
        assert_eq!(snp_get_tsc_info_with(&mut backend, resources), Err(EINVAL));
        assert_eq!(backend.calls, 0);

        resources = SnpTscInfoResources::new(secrets_with_keys());
        assert_eq!(snp_get_tsc_info_with(&mut backend, resources), Err(EIO));
        assert_eq!(backend.calls, 1);
        assert_eq!(snp_secure_tsc_info(), (0xaa, 0xbb, 0xcc));
        publish_snp_vmpl(0);
    }

    #[test]
    fn secure_tsc_prepare_models_linux_skip_enable_and_terminate() {
        let _guard = sev_core_test_guard();
        publish_snp_vmpl(0);
        let resources = SnpTscInfoResources::new(secrets_with_keys());
        let mut backend = TscInfoBackend {
            calls: 0,
            response: SnpTscInfoResp {
                status: 0,
                tsc_scale: 3,
                tsc_offset: 4,
                ..Default::default()
            },
            error: None,
        };

        assert_eq!(
            snp_secure_tsc_prepare_with(false, &mut backend, resources),
            SnpSecureTscPrepareOutcome::Skipped
        );
        assert_eq!(backend.calls, 0);

        assert_eq!(
            snp_secure_tsc_prepare_with(true, &mut backend, resources),
            SnpSecureTscPrepareOutcome::Enabled
        );
        assert_eq!(backend.calls, 1);

        let mut missing_desc = resources;
        missing_desc.msg_desc_available = false;
        assert_eq!(
            snp_secure_tsc_prepare_with(true, &mut backend, missing_desc),
            SnpSecureTscPrepareOutcome::Terminated {
                termination: SevTermination {
                    set: SEV_TERM_SET_LINUX,
                    reason: GHCB_TERM_SECURE_TSC,
                },
                rc: ENOMEM,
            }
        );
    }

    #[test]
    fn secure_tsc_init_masks_msr_and_scales_frequency_from_secrets() {
        let _guard = sev_core_test_guard();
        publish_secure_tsc(7, 11, 0);
        let mut secrets = SnpSecretsPage::default();
        secrets.tsc_factor = 250;

        assert_eq!(
            snp_secure_tsc_init_with(false, Some(&secrets), u64::MAX),
            SnpSecureTscInitOutcome::Skipped
        );
        assert_eq!(securetsc_get_tsc_khz(), 0);

        assert_eq!(
            snp_secure_tsc_init_with(true, None, u64::MAX),
            SnpSecureTscInitOutcome::Terminated {
                termination: SevTermination {
                    set: SEV_TERM_SET_LINUX,
                    reason: GHCB_TERM_SECURE_TSC,
                },
            }
        );

        assert_eq!(snp_scale_tsc_freq(4_000_000, 250), 3_990_000);
        assert_eq!(
            snp_secure_tsc_init_with(true, Some(&secrets), (0xbeef << 18) | 4000),
            SnpSecureTscInitOutcome::Initialized {
                freq_mhz: 4000,
                freq_khz: 3_990_000,
            }
        );
        assert_eq!(snp_secure_tsc_info(), (7, 11, 3_990_000));
    }

    #[test]
    fn sev_status_names_match_linux_msr_indexes_and_skip_reserved_bits() {
        let _guard = sev_core_test_guard();

        assert_eq!(
            sev_status_feature_name(MSR_AMD64_SEV_ENABLED_BIT),
            Some("SEV")
        );
        assert_eq!(
            sev_status_feature_name(MSR_AMD64_SNP_VMGEXIT_PARAM_BIT),
            Some("VMGExitParam")
        );
        assert_eq!(
            sev_status_feature_name(MSR_AMD64_SNP_IBPB_ON_ENTRY_BIT),
            Some("IBPBOnEntry")
        );
        assert_eq!(sev_status_feature_name(13), None);
        assert_eq!(sev_status_feature_name(15), None);
        assert_eq!(sev_status_feature_name(MSR_AMD64_SNP_RESV_BIT), None);

        let report =
            sev_show_status_report((1 << 0) | (1 << 2) | (1 << 13) | (1 << 23) | (1 << 24));
        assert_eq!(report.count, 3);
        assert_eq!(report.names[0], Some("SEV"));
        assert_eq!(report.names[1], Some("SEV-SNP"));
        assert_eq!(report.names[2], Some("IBPBOnEntry"));
    }

    #[test]
    fn snp_dmi_setup_only_runs_with_efi_config_tables() {
        assert!(!snp_dmi_setup_plan(false));
        assert!(snp_dmi_setup_plan(true));
    }

    #[test]
    fn report_snp_info_models_cpuid_and_vmpl_print_conditions() {
        assert_eq!(
            report_snp_info_plan(0, true, false, 0),
            SnpInfoReport {
                rc: 0,
                cpuid_table_count: 0,
                announced_cpuid_table: false,
                dumped_cpuid_table: false,
                sev_snp: false,
                vmpl: 0,
                announced_vmpl: false,
            }
        );
        assert_eq!(
            report_snp_info_plan(7, true, true, 2),
            SnpInfoReport {
                rc: 0,
                cpuid_table_count: 7,
                announced_cpuid_table: true,
                dumped_cpuid_table: true,
                sev_snp: true,
                vmpl: 2,
                announced_vmpl: true,
            }
        );
        assert!(!report_snp_info_plan(7, false, true, 1).dumped_cpuid_table);
    }

    #[test]
    fn snp_platform_device_init_preserves_linux_registration_order() {
        assert_eq!(
            snp_init_platform_device_plan(false, true, false, true),
            SnpPlatformDevicePlan::new(-ENODEV)
        );
        assert_eq!(
            snp_init_platform_device_plan(true, false, false, true),
            SnpPlatformDevicePlan::new(-ENODEV)
        );
        assert_eq!(
            snp_init_platform_device_plan(true, true, false, false),
            SnpPlatformDevicePlan {
                rc: 0,
                sev_guest_registered: true,
                vtpm_probe: false,
                tpm_svsm_registered: false,
                info_printed: true,
            }
        );
        assert_eq!(
            snp_init_platform_device_plan(true, true, true, false),
            SnpPlatformDevicePlan {
                rc: -ENODEV,
                sev_guest_registered: true,
                vtpm_probe: true,
                tpm_svsm_registered: false,
                info_printed: false,
            }
        );
        assert_eq!(
            snp_init_platform_device_plan(true, true, true, true),
            SnpPlatformDevicePlan {
                rc: 0,
                sev_guest_registered: true,
                vtpm_probe: true,
                tpm_svsm_registered: true,
                info_printed: true,
            }
        );
    }

    #[test]
    fn vmpl_show_bytes_matches_sysfs_decimal_newline() {
        assert_eq!(
            vmpl_show_bytes(2),
            VmplShow {
                bytes: [b'2', b'\n', 0, 0],
                len: 2,
            }
        );
        assert_eq!(
            vmpl_show_bytes(255),
            VmplShow {
                bytes: [b'2', b'5', b'5', b'\n'],
                len: 4,
            }
        );
    }

    #[test]
    fn sev_sysfs_init_models_linux_error_cleanup_order() {
        assert_eq!(sev_sysfs_init_plan(false, true, true, 0).rc, -ENODEV);
        assert_eq!(
            sev_sysfs_init_plan(true, false, true, 0),
            SevSysfsInitPlan::new(-ENODEV)
        );
        assert_eq!(
            sev_sysfs_init_plan(true, true, false, 0),
            SevSysfsInitPlan {
                rc: -ENOMEM,
                got_dev_root: true,
                created_kobject: false,
                created_group: false,
                put_dev_root: true,
                put_kobject_on_group_error: false,
            }
        );
        assert_eq!(
            sev_sysfs_init_plan(true, true, true, -EIO),
            SevSysfsInitPlan {
                rc: -EIO,
                got_dev_root: true,
                created_kobject: true,
                created_group: false,
                put_dev_root: true,
                put_kobject_on_group_error: true,
            }
        );
        assert_eq!(
            sev_sysfs_init_plan(true, true, true, 0),
            SevSysfsInitPlan {
                rc: 0,
                got_dev_root: true,
                created_kobject: true,
                created_group: true,
                put_dev_root: true,
                put_kobject_on_group_error: false,
            }
        );
    }

    #[test]
    fn accept_memory_uses_linux_private_page_state_flow() {
        let _guard = sev_core_test_guard();
        let page_offset = crate::arch::x86::mm::paging::PAGE_OFFSET;
        let plan = snp_accept_memory_plan(0x1000, 0x3000).unwrap();
        assert_eq!(plan.vaddr, page_offset + 0x1000);
        assert_eq!(plan.npages, 2);
        assert_eq!(plan.private_plan.first.count, 2);
        assert_eq!(plan.private_plan.first.entries[0].unwrap().gfn, 1);
        assert_eq!(
            plan.private_plan.first.entries[0].unwrap().operation,
            VMGEXIT_PSC_OP_PRIVATE
        );
        assert!(!plan.private_plan.pvalidate_before);
        assert!(plan.private_plan.pvalidate_after);

        reset_psc_issue_log();
        assert_eq!(snp_accept_memory(0x1000, 0x3000), Ok(2));
        let (len, log) = psc_issue_log();
        assert_eq!(len, 2);
        assert_eq!(
            &log[..len],
            &[
                (PSC_EVENT_VMGEXIT, VMGEXIT_PSC_OP_PRIVATE, 2),
                (PSC_EVENT_PVALIDATE_AFTER, VMGEXIT_PSC_OP_PRIVATE, 2),
            ]
        );

        let misaligned = snp_accept_memory_plan(0x1001, 0x3000).unwrap();
        assert_eq!(misaligned.vaddr, page_offset + 0x1001);
        assert_eq!(misaligned.npages, 1);
        assert_eq!(misaligned.private_plan.first.count, 1);
        assert_eq!(misaligned.private_plan.first.entries[0].unwrap().gfn, 1);
        reset_psc_issue_log();
        assert_eq!(snp_accept_memory(0x1001, 0x3000), Ok(1));
        let (len, log) = psc_issue_log();
        assert_eq!(len, 2);
        assert_eq!(
            &log[..len],
            &[
                (PSC_EVENT_VMGEXIT, VMGEXIT_PSC_OP_PRIVATE, 1),
                (PSC_EVENT_PVALIDATE_AFTER, VMGEXIT_PSC_OP_PRIVATE, 1),
            ]
        );

        let wrapped_npages = (0x3000u64.wrapping_sub(0x4000) >> PAGE_SHIFT) as usize;
        let wrapped = snp_accept_memory_plan(0x4000, 0x3000).unwrap();
        assert_eq!(wrapped.vaddr, page_offset + 0x4000);
        assert_eq!(wrapped.npages, wrapped_npages);
        assert_eq!(wrapped.private_plan.desc_count, 0);
        assert_eq!(wrapped.private_plan.entry_count, 0);
        assert_eq!(wrapped.private_plan.pages, 0);
        reset_psc_issue_log();
        assert_eq!(snp_accept_memory(0x4000, 0x3000), Ok(wrapped_npages));
        let (len, _) = psc_issue_log();
        assert_eq!(len, 0);
    }

    #[test]
    fn secure_tsc_state_is_published_as_triplet() {
        let _guard = sev_core_test_guard();
        publish_secure_tsc(7, 11, 3000);
        assert_eq!(snp_secure_tsc_info(), (7, 11, 3000));
    }
}
