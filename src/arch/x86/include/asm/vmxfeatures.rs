// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/vmxfeatures.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000758

// VMX capability bit indices.  Each index is its 32-bit capability word
// multiplied by 32, plus its bit position within that word.  The C macros
// contain unsuffixed decimal literals, and therefore have signed `int`
// semantics on the frozen x86_64 target.
pub const NVMXINTS: i32 = 5;

// Pin-Based VM-Execution Controls, EPT/VPID, APIC and VM-Functions, word 0.
pub const VMX_FEATURE_INTR_EXITING: i32 = 0 * 32 + 0;
pub const VMX_FEATURE_NMI_EXITING: i32 = 0 * 32 + 3;
pub const VMX_FEATURE_VIRTUAL_NMIS: i32 = 0 * 32 + 5;
pub const VMX_FEATURE_PREEMPTION_TIMER: i32 = 0 * 32 + 6;
pub const VMX_FEATURE_POSTED_INTR: i32 = 0 * 32 + 7;
pub const VMX_FEATURE_INVVPID: i32 = 0 * 32 + 16;
pub const VMX_FEATURE_EPT_EXECUTE_ONLY: i32 = 0 * 32 + 17;
pub const VMX_FEATURE_EPT_AD: i32 = 0 * 32 + 18;
pub const VMX_FEATURE_EPT_1GB: i32 = 0 * 32 + 19;
pub const VMX_FEATURE_EPT_5LEVEL: i32 = 0 * 32 + 20;
pub const VMX_FEATURE_FLEXPRIORITY: i32 = 0 * 32 + 24;
pub const VMX_FEATURE_APICV: i32 = 0 * 32 + 25;
pub const VMX_FEATURE_EPTP_SWITCHING: i32 = 0 * 32 + 28;

// Primary Processor-Based VM-Execution Controls, word 1.
pub const VMX_FEATURE_INTR_WINDOW_EXITING: i32 = 1 * 32 + 2;
pub const VMX_FEATURE_USE_TSC_OFFSETTING: i32 = 1 * 32 + 3;
pub const VMX_FEATURE_HLT_EXITING: i32 = 1 * 32 + 7;
pub const VMX_FEATURE_INVLPG_EXITING: i32 = 1 * 32 + 9;
pub const VMX_FEATURE_MWAIT_EXITING: i32 = 1 * 32 + 10;
pub const VMX_FEATURE_RDPMC_EXITING: i32 = 1 * 32 + 11;
pub const VMX_FEATURE_RDTSC_EXITING: i32 = 1 * 32 + 12;
pub const VMX_FEATURE_CR3_LOAD_EXITING: i32 = 1 * 32 + 15;
pub const VMX_FEATURE_CR3_STORE_EXITING: i32 = 1 * 32 + 16;
pub const VMX_FEATURE_TERTIARY_CONTROLS: i32 = 1 * 32 + 17;
pub const VMX_FEATURE_CR8_LOAD_EXITING: i32 = 1 * 32 + 19;
pub const VMX_FEATURE_CR8_STORE_EXITING: i32 = 1 * 32 + 20;
pub const VMX_FEATURE_VIRTUAL_TPR: i32 = 1 * 32 + 21;
pub const VMX_FEATURE_NMI_WINDOW_EXITING: i32 = 1 * 32 + 22;
pub const VMX_FEATURE_MOV_DR_EXITING: i32 = 1 * 32 + 23;
pub const VMX_FEATURE_UNCOND_IO_EXITING: i32 = 1 * 32 + 24;
pub const VMX_FEATURE_USE_IO_BITMAPS: i32 = 1 * 32 + 25;
pub const VMX_FEATURE_MONITOR_TRAP_FLAG: i32 = 1 * 32 + 27;
pub const VMX_FEATURE_USE_MSR_BITMAPS: i32 = 1 * 32 + 28;
pub const VMX_FEATURE_MONITOR_EXITING: i32 = 1 * 32 + 29;
pub const VMX_FEATURE_PAUSE_EXITING: i32 = 1 * 32 + 30;
pub const VMX_FEATURE_SEC_CONTROLS: i32 = 1 * 32 + 31;

// Secondary Processor-Based VM-Execution Controls, word 2.
pub const VMX_FEATURE_VIRT_APIC_ACCESSES: i32 = 2 * 32 + 0;
pub const VMX_FEATURE_EPT: i32 = 2 * 32 + 1;
pub const VMX_FEATURE_DESC_EXITING: i32 = 2 * 32 + 2;
pub const VMX_FEATURE_RDTSCP: i32 = 2 * 32 + 3;
pub const VMX_FEATURE_VIRTUAL_X2APIC: i32 = 2 * 32 + 4;
pub const VMX_FEATURE_VPID: i32 = 2 * 32 + 5;
pub const VMX_FEATURE_WBINVD_EXITING: i32 = 2 * 32 + 6;
pub const VMX_FEATURE_UNRESTRICTED_GUEST: i32 = 2 * 32 + 7;
pub const VMX_FEATURE_APIC_REGISTER_VIRT: i32 = 2 * 32 + 8;
pub const VMX_FEATURE_VIRT_INTR_DELIVERY: i32 = 2 * 32 + 9;
pub const VMX_FEATURE_PAUSE_LOOP_EXITING: i32 = 2 * 32 + 10;
pub const VMX_FEATURE_RDRAND_EXITING: i32 = 2 * 32 + 11;
pub const VMX_FEATURE_INVPCID: i32 = 2 * 32 + 12;
pub const VMX_FEATURE_VMFUNC: i32 = 2 * 32 + 13;
pub const VMX_FEATURE_SHADOW_VMCS: i32 = 2 * 32 + 14;
pub const VMX_FEATURE_ENCLS_EXITING: i32 = 2 * 32 + 15;
pub const VMX_FEATURE_RDSEED_EXITING: i32 = 2 * 32 + 16;
pub const VMX_FEATURE_PAGE_MOD_LOGGING: i32 = 2 * 32 + 17;
pub const VMX_FEATURE_EPT_VIOLATION_VE: i32 = 2 * 32 + 18;
pub const VMX_FEATURE_PT_CONCEAL_VMX: i32 = 2 * 32 + 19;
pub const VMX_FEATURE_XSAVES: i32 = 2 * 32 + 20;
pub const VMX_FEATURE_MODE_BASED_EPT_EXEC: i32 = 2 * 32 + 22;
pub const VMX_FEATURE_PT_USE_GPA: i32 = 2 * 32 + 24;
pub const VMX_FEATURE_TSC_SCALING: i32 = 2 * 32 + 25;
pub const VMX_FEATURE_USR_WAIT_PAUSE: i32 = 2 * 32 + 26;
pub const VMX_FEATURE_ENCLV_EXITING: i32 = 2 * 32 + 28;
pub const VMX_FEATURE_BUS_LOCK_DETECTION: i32 = 2 * 32 + 30;
pub const VMX_FEATURE_NOTIFY_VM_EXITING: i32 = 2 * 32 + 31;

// Tertiary Processor-Based VM-Execution Controls, word 3.
pub const VMX_FEATURE_IPI_VIRT: i32 = 3 * 32 + 4;

// `mkcapflags.sh` turns only source comments beginning with a quoted string
// into the selected CONFIG_X86_VMX_FEATURE_NAMES /proc/cpuinfo flag table.
// `None` exactly represents the generated C table's null entry for every
// feature whose source comment does not begin with such a string.
pub const VMX_FEATURE_NAMES: [Option<&str>; (NVMXINTS * 32) as usize] = {
    let mut names = [None; (NVMXINTS * 32) as usize];
    names[VMX_FEATURE_VIRTUAL_NMIS as usize] = Some("vnmi");
    names[VMX_FEATURE_PREEMPTION_TIMER as usize] = Some("preemption_timer");
    names[VMX_FEATURE_POSTED_INTR as usize] = Some("posted_intr");
    names[VMX_FEATURE_INVVPID as usize] = Some("invvpid");
    names[VMX_FEATURE_EPT_EXECUTE_ONLY as usize] = Some("ept_x_only");
    names[VMX_FEATURE_EPT_AD as usize] = Some("ept_ad");
    names[VMX_FEATURE_EPT_1GB as usize] = Some("ept_1gb");
    names[VMX_FEATURE_EPT_5LEVEL as usize] = Some("ept_5level");
    names[VMX_FEATURE_FLEXPRIORITY as usize] = Some("flexpriority");
    names[VMX_FEATURE_APICV as usize] = Some("apicv");
    names[VMX_FEATURE_EPTP_SWITCHING as usize] = Some("eptp_switching");
    names[VMX_FEATURE_USE_TSC_OFFSETTING as usize] = Some("tsc_offset");
    names[VMX_FEATURE_VIRTUAL_TPR as usize] = Some("vtpr");
    names[VMX_FEATURE_MONITOR_TRAP_FLAG as usize] = Some("mtf");
    names[VMX_FEATURE_VIRT_APIC_ACCESSES as usize] = Some("vapic");
    names[VMX_FEATURE_EPT as usize] = Some("ept");
    names[VMX_FEATURE_VPID as usize] = Some("vpid");
    names[VMX_FEATURE_UNRESTRICTED_GUEST as usize] = Some("unrestricted_guest");
    names[VMX_FEATURE_APIC_REGISTER_VIRT as usize] = Some("vapic_reg");
    names[VMX_FEATURE_VIRT_INTR_DELIVERY as usize] = Some("vid");
    names[VMX_FEATURE_PAUSE_LOOP_EXITING as usize] = Some("ple");
    names[VMX_FEATURE_SHADOW_VMCS as usize] = Some("shadow_vmcs");
    names[VMX_FEATURE_PAGE_MOD_LOGGING as usize] = Some("pml");
    names[VMX_FEATURE_EPT_VIOLATION_VE as usize] = Some("ept_violation_ve");
    names[VMX_FEATURE_MODE_BASED_EPT_EXEC as usize] = Some("ept_mode_based_exec");
    names[VMX_FEATURE_TSC_SCALING as usize] = Some("tsc_scaling");
    names[VMX_FEATURE_USR_WAIT_PAUSE as usize] = Some("usr_wait_pause");
    names[VMX_FEATURE_NOTIFY_VM_EXITING as usize] = Some("notify_vm_exiting");
    names[VMX_FEATURE_IPI_VIRT as usize] = Some("ipi_virt");
    names
};
