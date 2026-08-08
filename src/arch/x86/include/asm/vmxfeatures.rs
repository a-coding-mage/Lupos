// SPDX-License-Identifier: GPL-2.0
//! linux-source: arch/x86/include/asm/vmxfeatures.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000758

/* Defines VMX CPU feature bits. */
pub const NVMXINTS: i32 = 5;

/* Pin-Based VM-Execution Controls, EPT/VPID, APIC and VM-Functions, word 0 */
pub const VMX_FEATURE_INTR_EXITING: i32 = 0i32 * 32 + 0;
pub const VMX_FEATURE_NMI_EXITING: i32 = 0i32 * 32 + 3;
pub const VMX_FEATURE_VIRTUAL_NMIS: i32 = 0i32 * 32 + 5;
pub const VMX_FEATURE_PREEMPTION_TIMER: i32 = 0i32 * 32 + 6;
pub const VMX_FEATURE_POSTED_INTR: i32 = 0i32 * 32 + 7;

/* EPT/VPID features, scattered to bits 16-23 */
pub const VMX_FEATURE_INVVPID: i32 = 0i32 * 32 + 16;
pub const VMX_FEATURE_EPT_EXECUTE_ONLY: i32 = 0i32 * 32 + 17;
pub const VMX_FEATURE_EPT_AD: i32 = 0i32 * 32 + 18;
pub const VMX_FEATURE_EPT_1GB: i32 = 0i32 * 32 + 19;
pub const VMX_FEATURE_EPT_5LEVEL: i32 = 0i32 * 32 + 20;

/* Aggregated APIC features 24-27 */
pub const VMX_FEATURE_FLEXPRIORITY: i32 = 0i32 * 32 + 24;
pub const VMX_FEATURE_APICV: i32 = 0i32 * 32 + 25;

/* VM-Functions, shifted to bits 28-31 */
pub const VMX_FEATURE_EPTP_SWITCHING: i32 = 0i32 * 32 + 28;

/* Primary Processor-Based VM-Execution Controls, word 1 */
pub const VMX_FEATURE_INTR_WINDOW_EXITING: i32 = 1i32 * 32 + 2;
pub const VMX_FEATURE_USE_TSC_OFFSETTING: i32 = 1i32 * 32 + 3;
pub const VMX_FEATURE_HLT_EXITING: i32 = 1i32 * 32 + 7;
pub const VMX_FEATURE_INVLPG_EXITING: i32 = 1i32 * 32 + 9;
pub const VMX_FEATURE_MWAIT_EXITING: i32 = 1i32 * 32 + 10;
pub const VMX_FEATURE_RDPMC_EXITING: i32 = 1i32 * 32 + 11;
pub const VMX_FEATURE_RDTSC_EXITING: i32 = 1i32 * 32 + 12;
pub const VMX_FEATURE_CR3_LOAD_EXITING: i32 = 1i32 * 32 + 15;
pub const VMX_FEATURE_CR3_STORE_EXITING: i32 = 1i32 * 32 + 16;
pub const VMX_FEATURE_TERTIARY_CONTROLS: i32 = 1i32 * 32 + 17;
pub const VMX_FEATURE_CR8_LOAD_EXITING: i32 = 1i32 * 32 + 19;
pub const VMX_FEATURE_CR8_STORE_EXITING: i32 = 1i32 * 32 + 20;
pub const VMX_FEATURE_VIRTUAL_TPR: i32 = 1i32 * 32 + 21;
pub const VMX_FEATURE_NMI_WINDOW_EXITING: i32 = 1i32 * 32 + 22;
pub const VMX_FEATURE_MOV_DR_EXITING: i32 = 1i32 * 32 + 23;
pub const VMX_FEATURE_UNCOND_IO_EXITING: i32 = 1i32 * 32 + 24;
pub const VMX_FEATURE_USE_IO_BITMAPS: i32 = 1i32 * 32 + 25;
pub const VMX_FEATURE_MONITOR_TRAP_FLAG: i32 = 1i32 * 32 + 27;
pub const VMX_FEATURE_USE_MSR_BITMAPS: i32 = 1i32 * 32 + 28;
pub const VMX_FEATURE_MONITOR_EXITING: i32 = 1i32 * 32 + 29;
pub const VMX_FEATURE_PAUSE_EXITING: i32 = 1i32 * 32 + 30;
pub const VMX_FEATURE_SEC_CONTROLS: i32 = 1i32 * 32 + 31;

/* Secondary Processor-Based VM-Execution Controls, word 2 */
pub const VMX_FEATURE_VIRT_APIC_ACCESSES: i32 = 2i32 * 32 + 0;
pub const VMX_FEATURE_EPT: i32 = 2i32 * 32 + 1;
pub const VMX_FEATURE_DESC_EXITING: i32 = 2i32 * 32 + 2;
pub const VMX_FEATURE_RDTSCP: i32 = 2i32 * 32 + 3;
pub const VMX_FEATURE_VIRTUAL_X2APIC: i32 = 2i32 * 32 + 4;
pub const VMX_FEATURE_VPID: i32 = 2i32 * 32 + 5;
pub const VMX_FEATURE_WBINVD_EXITING: i32 = 2i32 * 32 + 6;
pub const VMX_FEATURE_UNRESTRICTED_GUEST: i32 = 2i32 * 32 + 7;
pub const VMX_FEATURE_APIC_REGISTER_VIRT: i32 = 2i32 * 32 + 8;
pub const VMX_FEATURE_VIRT_INTR_DELIVERY: i32 = 2i32 * 32 + 9;
pub const VMX_FEATURE_PAUSE_LOOP_EXITING: i32 = 2i32 * 32 + 10;
pub const VMX_FEATURE_RDRAND_EXITING: i32 = 2i32 * 32 + 11;
pub const VMX_FEATURE_INVPCID: i32 = 2i32 * 32 + 12;
pub const VMX_FEATURE_VMFUNC: i32 = 2i32 * 32 + 13;
pub const VMX_FEATURE_SHADOW_VMCS: i32 = 2i32 * 32 + 14;
pub const VMX_FEATURE_ENCLS_EXITING: i32 = 2i32 * 32 + 15;
pub const VMX_FEATURE_RDSEED_EXITING: i32 = 2i32 * 32 + 16;
pub const VMX_FEATURE_PAGE_MOD_LOGGING: i32 = 2i32 * 32 + 17;
pub const VMX_FEATURE_EPT_VIOLATION_VE: i32 = 2i32 * 32 + 18;
pub const VMX_FEATURE_PT_CONCEAL_VMX: i32 = 2i32 * 32 + 19;
pub const VMX_FEATURE_XSAVES: i32 = 2i32 * 32 + 20;
pub const VMX_FEATURE_MODE_BASED_EPT_EXEC: i32 = 2i32 * 32 + 22;
pub const VMX_FEATURE_PT_USE_GPA: i32 = 2i32 * 32 + 24;
pub const VMX_FEATURE_TSC_SCALING: i32 = 2i32 * 32 + 25;
pub const VMX_FEATURE_USR_WAIT_PAUSE: i32 = 2i32 * 32 + 26;
pub const VMX_FEATURE_ENCLV_EXITING: i32 = 2i32 * 32 + 28;
pub const VMX_FEATURE_BUS_LOCK_DETECTION: i32 = 2i32 * 32 + 30;
pub const VMX_FEATURE_NOTIFY_VM_EXITING: i32 = 2i32 * 32 + 31;

/* Tertiary Processor-Based VM-Execution Controls, word 3 */
pub const VMX_FEATURE_IPI_VIRT: i32 = 3i32 * 32 + 4;
