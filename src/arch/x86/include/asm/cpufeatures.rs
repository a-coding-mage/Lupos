// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/cpufeatures.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000496

/* SPDX-License-Identifier: GPL-2.0 */

/*
 * Defines x86 CPU feature bits
 */
pub const NCAPINTS: u32 = 22; /* N 32-bit words worth of info */
pub const NBUGINTS: u32 = 2; /* N 32-bit bug flags */

/*
 * Note: If the comment begins with a quoted string, that string is used
 * in /proc/cpuinfo instead of the macro name.  Otherwise, this feature
 * bit is not displayed in /proc/cpuinfo at all.
 *
 * When adding new features here that depend on other features,
 * please update the table in kernel/cpu/cpuid-deps.c as well.
 */

/* Intel-defined CPU features, CPUID level 0x00000001 (EDX), word 0 */
pub const X86_FEATURE_FPU: u32 = ( 0*32+ 0); /* "fpu" Onboard FPU */
pub const X86_FEATURE_VME: u32 = ( 0*32+ 1); /* "vme" Virtual Mode Extensions */
pub const X86_FEATURE_DE: u32 = ( 0*32+ 2); /* "de" Debugging Extensions */
pub const X86_FEATURE_PSE: u32 = ( 0*32+ 3); /* "pse" Page Size Extensions */
pub const X86_FEATURE_TSC: u32 = ( 0*32+ 4); /* "tsc" Time Stamp Counter */
pub const X86_FEATURE_MSR: u32 = ( 0*32+ 5); /* "msr" Model-Specific Registers */
pub const X86_FEATURE_PAE: u32 = ( 0*32+ 6); /* "pae" Physical Address Extensions */
pub const X86_FEATURE_MCE: u32 = ( 0*32+ 7); /* "mce" Machine Check Exception */
pub const X86_FEATURE_CX8: u32 = ( 0*32+ 8); /* "cx8" CMPXCHG8 instruction */
pub const X86_FEATURE_APIC: u32 = ( 0*32+ 9); /* "apic" Onboard APIC */
pub const X86_FEATURE_SEP: u32 = ( 0*32+11); /* "sep" SYSENTER/SYSEXIT */
pub const X86_FEATURE_MTRR: u32 = ( 0*32+12); /* "mtrr" Memory Type Range Registers */
pub const X86_FEATURE_PGE: u32 = ( 0*32+13); /* "pge" Page Global Enable */
pub const X86_FEATURE_MCA: u32 = ( 0*32+14); /* "mca" Machine Check Architecture */
pub const X86_FEATURE_CMOV: u32 = ( 0*32+15); /* "cmov" CMOV instructions (plus FCMOVcc, FCOMI with FPU) */
pub const X86_FEATURE_PAT: u32 = ( 0*32+16); /* "pat" Page Attribute Table */
pub const X86_FEATURE_PSE36: u32 = ( 0*32+17); /* "pse36" 36-bit PSEs */
pub const X86_FEATURE_PN: u32 = ( 0*32+18); /* "pn" Processor serial number */
pub const X86_FEATURE_CLFLUSH: u32 = ( 0*32+19); /* "clflush" CLFLUSH instruction */
pub const X86_FEATURE_DS: u32 = ( 0*32+21); /* "dts" Debug Store */
pub const X86_FEATURE_ACPI: u32 = ( 0*32+22); /* "acpi" ACPI via MSR */
pub const X86_FEATURE_MMX: u32 = ( 0*32+23); /* "mmx" Multimedia Extensions */
pub const X86_FEATURE_FXSR: u32 = ( 0*32+24); /* "fxsr" FXSAVE/FXRSTOR, CR4.OSFXSR */
pub const X86_FEATURE_XMM: u32 = ( 0*32+25); /* "sse" */
pub const X86_FEATURE_XMM2: u32 = ( 0*32+26); /* "sse2" */
pub const X86_FEATURE_SELFSNOOP: u32 = ( 0*32+27); /* "ss" CPU self snoop */
pub const X86_FEATURE_HT: u32 = ( 0*32+28); /* "ht" Hyper-Threading */
pub const X86_FEATURE_ACC: u32 = ( 0*32+29); /* "tm" Automatic clock control */
pub const X86_FEATURE_IA64: u32 = ( 0*32+30); /* "ia64" IA-64 processor */
pub const X86_FEATURE_PBE: u32 = ( 0*32+31); /* "pbe" Pending Break Enable */

/* AMD-defined CPU features, CPUID level 0x80000001, word 1 */
/* Don't duplicate feature flags which are redundant with Intel! */
pub const X86_FEATURE_SYSCALL: u32 = ( 1*32+11); /* "syscall" SYSCALL/SYSRET */
pub const X86_FEATURE_MP: u32 = ( 1*32+19); /* "mp" MP Capable */
pub const X86_FEATURE_NX: u32 = ( 1*32+20); /* "nx" Execute Disable */
pub const X86_FEATURE_MMXEXT: u32 = ( 1*32+22); /* "mmxext" AMD MMX extensions */
pub const X86_FEATURE_FXSR_OPT: u32 = ( 1*32+25); /* "fxsr_opt" FXSAVE/FXRSTOR optimizations */
pub const X86_FEATURE_GBPAGES: u32 = ( 1*32+26); /* "pdpe1gb" GB pages */
pub const X86_FEATURE_RDTSCP: u32 = ( 1*32+27); /* "rdtscp" RDTSCP */
pub const X86_FEATURE_LM: u32 = ( 1*32+29); /* "lm" Long Mode (x86-64, 64-bit support) */
pub const X86_FEATURE_3DNOWEXT: u32 = ( 1*32+30); /* "3dnowext" AMD 3DNow extensions */
pub const X86_FEATURE_3DNOW: u32 = ( 1*32+31); /* "3dnow" 3DNow */

/* Transmeta-defined CPU features, CPUID level 0x80860001, word 2 */
pub const X86_FEATURE_RECOVERY: u32 = ( 2*32+ 0); /* "recovery" CPU in recovery mode */
pub const X86_FEATURE_LONGRUN: u32 = ( 2*32+ 1); /* "longrun" Longrun power control */
pub const X86_FEATURE_LRTI: u32 = ( 2*32+ 3); /* "lrti" LongRun table interface */

/* Other features, Linux-defined mapping, word 3 */
/* This range is used for feature bits which conflict or are synthesized */
pub const X86_FEATURE_CXMMX: u32 = ( 3*32+ 0); /* "cxmmx" Cyrix MMX extensions */
pub const X86_FEATURE_K6_MTRR: u32 = ( 3*32+ 1); /* "k6_mtrr" AMD K6 nonstandard MTRRs */
pub const X86_FEATURE_CYRIX_ARR: u32 = ( 3*32+ 2); /* "cyrix_arr" Cyrix ARRs (= MTRRs) */
pub const X86_FEATURE_CENTAUR_MCR: u32 = ( 3*32+ 3); /* "centaur_mcr" Centaur MCRs (= MTRRs) */
pub const X86_FEATURE_K8: u32 = ( 3*32+ 4); /* Opteron, Athlon64 */
pub const X86_FEATURE_ZEN5: u32 = ( 3*32+ 5); /* CPU based on Zen5 microarchitecture */
pub const X86_FEATURE_ZEN6: u32 = ( 3*32+ 6); /* CPU based on Zen6 microarchitecture */
/* Free                                 ( 3*32+ 7) */
pub const X86_FEATURE_CONSTANT_TSC: u32 = ( 3*32+ 8); /* "constant_tsc" TSC ticks at a constant rate */
pub const X86_FEATURE_UP: u32 = ( 3*32+ 9); /* "up" SMP kernel running on UP */
pub const X86_FEATURE_ART: u32 = ( 3*32+10); /* "art" Always running timer (ART) */
pub const X86_FEATURE_ARCH_PERFMON: u32 = ( 3*32+11); /* "arch_perfmon" Intel Architectural PerfMon */
pub const X86_FEATURE_PEBS: u32 = ( 3*32+12); /* "pebs" Precise-Event Based Sampling */
pub const X86_FEATURE_BTS: u32 = ( 3*32+13); /* "bts" Branch Trace Store */
pub const X86_FEATURE_SYSCALL32: u32 = ( 3*32+14); /* syscall in IA32 userspace */
pub const X86_FEATURE_SYSFAST32: u32 = ( 3*32+15); /* sysenter/syscall in IA32 userspace */
pub const X86_FEATURE_REP_GOOD: u32 = ( 3*32+16); /* "rep_good" REP microcode works well */
pub const X86_FEATURE_AMD_LBR_V2: u32 = ( 3*32+17); /* "amd_lbr_v2" AMD Last Branch Record Extension Version 2 */
pub const X86_FEATURE_CLEAR_CPU_BUF: u32 = ( 3*32+18); /* Clear CPU buffers using VERW */
pub const X86_FEATURE_ACC_POWER: u32 = ( 3*32+19); /* "acc_power" AMD Accumulated Power Mechanism */
pub const X86_FEATURE_NOPL: u32 = ( 3*32+20); /* "nopl" The NOPL (0F 1F) instructions */
pub const X86_FEATURE_ALWAYS: u32 = ( 3*32+21); /* Always-present feature */
pub const X86_FEATURE_XTOPOLOGY: u32 = ( 3*32+22); /* "xtopology" CPU topology enum extensions */
pub const X86_FEATURE_TSC_RELIABLE: u32 = ( 3*32+23); /* "tsc_reliable" TSC is known to be reliable */
pub const X86_FEATURE_NONSTOP_TSC: u32 = ( 3*32+24); /* "nonstop_tsc" TSC does not stop in C states */
pub const X86_FEATURE_CPUID: u32 = ( 3*32+25); /* "cpuid" CPU has CPUID instruction itself */
pub const X86_FEATURE_EXTD_APICID: u32 = ( 3*32+26); /* "extd_apicid" Extended APICID (8 bits) */
pub const X86_FEATURE_AMD_DCM: u32 = ( 3*32+27); /* "amd_dcm" AMD multi-node processor */
pub const X86_FEATURE_APERFMPERF: u32 = ( 3*32+28); /* "aperfmperf" P-State hardware coordination feedback capability (APERF/MPERF MSRs) */
pub const X86_FEATURE_RAPL: u32 = ( 3*32+29); /* "rapl" AMD/Hygon RAPL interface */
pub const X86_FEATURE_NONSTOP_TSC_S3: u32 = ( 3*32+30); /* "nonstop_tsc_s3" TSC doesn't stop in S3 state */
pub const X86_FEATURE_TSC_KNOWN_FREQ: u32 = ( 3*32+31); /* "tsc_known_freq" TSC has known frequency */

/* Intel-defined CPU features, CPUID level 0x00000001 (ECX), word 4 */
pub const X86_FEATURE_XMM3: u32 = ( 4*32+ 0); /* "pni" SSE-3 */
pub const X86_FEATURE_PCLMULQDQ: u32 = ( 4*32+ 1); /* "pclmulqdq" PCLMULQDQ instruction */
pub const X86_FEATURE_DTES64: u32 = ( 4*32+ 2); /* "dtes64" 64-bit Debug Store */
pub const X86_FEATURE_MWAIT: u32 = ( 4*32+ 3); /* "monitor" MONITOR/MWAIT support */
pub const X86_FEATURE_DSCPL: u32 = ( 4*32+ 4); /* "ds_cpl" CPL-qualified (filtered) Debug Store */
pub const X86_FEATURE_VMX: u32 = ( 4*32+ 5); /* "vmx" Hardware virtualization */
pub const X86_FEATURE_SMX: u32 = ( 4*32+ 6); /* "smx" Safer Mode eXtensions */
pub const X86_FEATURE_EST: u32 = ( 4*32+ 7); /* "est" Enhanced SpeedStep */
pub const X86_FEATURE_TM2: u32 = ( 4*32+ 8); /* "tm2" Thermal Monitor 2 */
pub const X86_FEATURE_SSSE3: u32 = ( 4*32+ 9); /* "ssse3" Supplemental SSE-3 */
pub const X86_FEATURE_CID: u32 = ( 4*32+10); /* "cid" Context ID */
pub const X86_FEATURE_SDBG: u32 = ( 4*32+11); /* "sdbg" Silicon Debug */
pub const X86_FEATURE_FMA: u32 = ( 4*32+12); /* "fma" Fused multiply-add */
pub const X86_FEATURE_CX16: u32 = ( 4*32+13); /* "cx16" CMPXCHG16B instruction */
pub const X86_FEATURE_XTPR: u32 = ( 4*32+14); /* "xtpr" Send Task Priority Messages */
pub const X86_FEATURE_PDCM: u32 = ( 4*32+15); /* "pdcm" Perf/Debug Capabilities MSR */
pub const X86_FEATURE_PCID: u32 = ( 4*32+17); /* "pcid" Process Context Identifiers */
pub const X86_FEATURE_DCA: u32 = ( 4*32+18); /* "dca" Direct Cache Access */
pub const X86_FEATURE_XMM4_1: u32 = ( 4*32+19); /* "sse4_1" SSE-4.1 */
pub const X86_FEATURE_XMM4_2: u32 = ( 4*32+20); /* "sse4_2" SSE-4.2 */
pub const X86_FEATURE_X2APIC: u32 = ( 4*32+21); /* "x2apic" X2APIC */
pub const X86_FEATURE_MOVBE: u32 = ( 4*32+22); /* "movbe" MOVBE instruction */
pub const X86_FEATURE_POPCNT: u32 = ( 4*32+23); /* "popcnt" POPCNT instruction */
pub const X86_FEATURE_TSC_DEADLINE_TIMER: u32 = ( 4*32+24); /* "tsc_deadline_timer" TSC deadline timer */
pub const X86_FEATURE_AES: u32 = ( 4*32+25); /* "aes" AES instructions */
pub const X86_FEATURE_XSAVE: u32 = ( 4*32+26); /* "xsave" XSAVE/XRSTOR/XSETBV/XGETBV instructions */
pub const X86_FEATURE_OSXSAVE: u32 = ( 4*32+27); /* XSAVE instruction enabled in the OS */
pub const X86_FEATURE_AVX: u32 = ( 4*32+28); /* "avx" Advanced Vector Extensions */
pub const X86_FEATURE_F16C: u32 = ( 4*32+29); /* "f16c" 16-bit FP conversions */
pub const X86_FEATURE_RDRAND: u32 = ( 4*32+30); /* "rdrand" RDRAND instruction */
pub const X86_FEATURE_HYPERVISOR: u32 = ( 4*32+31); /* "hypervisor" Running on a hypervisor */

/* VIA/Cyrix/Centaur-defined CPU features, CPUID level 0xC0000001, word 5 */
pub const X86_FEATURE_XSTORE: u32 = ( 5*32+ 2); /* "rng" RNG present (xstore) */
pub const X86_FEATURE_XSTORE_EN: u32 = ( 5*32+ 3); /* "rng_en" RNG enabled */
pub const X86_FEATURE_XCRYPT: u32 = ( 5*32+ 6); /* "ace" on-CPU crypto (xcrypt) */
pub const X86_FEATURE_XCRYPT_EN: u32 = ( 5*32+ 7); /* "ace_en" on-CPU crypto enabled */
pub const X86_FEATURE_ACE2: u32 = ( 5*32+ 8); /* "ace2" Advanced Cryptography Engine v2 */
pub const X86_FEATURE_ACE2_EN: u32 = ( 5*32+ 9); /* "ace2_en" ACE v2 enabled */
pub const X86_FEATURE_PHE: u32 = ( 5*32+10); /* "phe" PadLock Hash Engine */
pub const X86_FEATURE_PHE_EN: u32 = ( 5*32+11); /* "phe_en" PHE enabled */
pub const X86_FEATURE_PMM: u32 = ( 5*32+12); /* "pmm" PadLock Montgomery Multiplier */
pub const X86_FEATURE_PMM_EN: u32 = ( 5*32+13); /* "pmm_en" PMM enabled */

/* More extended AMD flags: CPUID level 0x80000001, ECX, word 6 */
pub const X86_FEATURE_LAHF_LM: u32 = ( 6*32+ 0); /* "lahf_lm" LAHF/SAHF in long mode */
pub const X86_FEATURE_CMP_LEGACY: u32 = ( 6*32+ 1); /* "cmp_legacy" If yes HyperThreading not valid */
pub const X86_FEATURE_SVM: u32 = ( 6*32+ 2); /* "svm" Secure Virtual Machine */
pub const X86_FEATURE_EXTAPIC: u32 = ( 6*32+ 3); /* "extapic" Extended APIC space */
pub const X86_FEATURE_CR8_LEGACY: u32 = ( 6*32+ 4); /* "cr8_legacy" CR8 in 32-bit mode */
pub const X86_FEATURE_ABM: u32 = ( 6*32+ 5); /* "abm" Advanced bit manipulation */
pub const X86_FEATURE_SSE4A: u32 = ( 6*32+ 6); /* "sse4a" SSE-4A */
pub const X86_FEATURE_MISALIGNSSE: u32 = ( 6*32+ 7); /* "misalignsse" Misaligned SSE mode */
pub const X86_FEATURE_3DNOWPREFETCH: u32 = ( 6*32+ 8); /* "3dnowprefetch" 3DNow prefetch instructions */
pub const X86_FEATURE_OSVW: u32 = ( 6*32+ 9); /* "osvw" OS Visible Workaround */
pub const X86_FEATURE_IBS: u32 = ( 6*32+10); /* "ibs" Instruction Based Sampling */
pub const X86_FEATURE_XOP: u32 = ( 6*32+11); /* "xop" Extended AVX instructions */
pub const X86_FEATURE_SKINIT: u32 = ( 6*32+12); /* "skinit" SKINIT/STGI instructions */
pub const X86_FEATURE_WDT: u32 = ( 6*32+13); /* "wdt" Watchdog timer */
pub const X86_FEATURE_LWP: u32 = ( 6*32+15); /* "lwp" Light Weight Profiling */
pub const X86_FEATURE_FMA4: u32 = ( 6*32+16); /* "fma4" 4 operands MAC instructions */
pub const X86_FEATURE_TCE: u32 = ( 6*32+17); /* "tce" Translation Cache Extension */
pub const X86_FEATURE_NODEID_MSR: u32 = ( 6*32+19); /* "nodeid_msr" NodeId MSR */
pub const X86_FEATURE_TBM: u32 = ( 6*32+21); /* "tbm" Trailing Bit Manipulations */
pub const X86_FEATURE_TOPOEXT: u32 = ( 6*32+22); /* "topoext" Topology extensions CPUID leafs */
pub const X86_FEATURE_PERFCTR_CORE: u32 = ( 6*32+23); /* "perfctr_core" Core performance counter extensions */
pub const X86_FEATURE_PERFCTR_NB: u32 = ( 6*32+24); /* "perfctr_nb" NB performance counter extensions */
pub const X86_FEATURE_BPEXT: u32 = ( 6*32+26); /* "bpext" Data breakpoint extension */
pub const X86_FEATURE_PTSC: u32 = ( 6*32+27); /* "ptsc" Performance time-stamp counter */
pub const X86_FEATURE_PERFCTR_LLC: u32 = ( 6*32+28); /* "perfctr_llc" Last Level Cache performance counter extensions */
pub const X86_FEATURE_MWAITX: u32 = ( 6*32+29); /* "mwaitx" MWAIT extension (MONITORX/MWAITX instructions) */

/*
 * Auxiliary flags: Linux defined - For features scattered in various
 * CPUID levels like 0x6, 0xA etc, word 7.
 *
 * Reuse free bits when adding new feature flags!
 */
pub const X86_FEATURE_RING3MWAIT: u32 = ( 7*32+ 0); /* "ring3mwait" Ring 3 MONITOR/MWAIT instructions */
pub const X86_FEATURE_CPUID_FAULT: u32 = ( 7*32+ 1); /* "cpuid_fault" Intel CPUID faulting */
pub const X86_FEATURE_CPB: u32 = ( 7*32+ 2); /* "cpb" AMD Core Performance Boost */
pub const X86_FEATURE_EPB: u32 = ( 7*32+ 3); /* "epb" IA32_ENERGY_PERF_BIAS support */
pub const X86_FEATURE_CAT_L3: u32 = ( 7*32+ 4); /* "cat_l3" Cache Allocation Technology L3 */
pub const X86_FEATURE_CAT_L2: u32 = ( 7*32+ 5); /* "cat_l2" Cache Allocation Technology L2 */
pub const X86_FEATURE_CDP_L3: u32 = ( 7*32+ 6); /* "cdp_l3" Code and Data Prioritization L3 */
pub const X86_FEATURE_TDX_HOST_PLATFORM: u32 = ( 7*32+ 7); /* "tdx_host_platform" Platform supports being a TDX host */
pub const X86_FEATURE_HW_PSTATE: u32 = ( 7*32+ 8); /* "hw_pstate" AMD HW-PState */
pub const X86_FEATURE_PROC_FEEDBACK: u32 = ( 7*32+ 9); /* "proc_feedback" AMD ProcFeedbackInterface */
pub const X86_FEATURE_XCOMPACTED: u32 = ( 7*32+10); /* Use compacted XSTATE (XSAVES or XSAVEC) */
pub const X86_FEATURE_PTI: u32 = ( 7*32+11); /* "pti" Kernel Page Table Isolation enabled */
pub const X86_FEATURE_KERNEL_IBRS: u32 = ( 7*32+12); /* Set/clear IBRS on kernel entry/exit */
pub const X86_FEATURE_RSB_VMEXIT: u32 = ( 7*32+13); /* Fill RSB on VM-Exit */
pub const X86_FEATURE_INTEL_PPIN: u32 = ( 7*32+14); /* "intel_ppin" Intel Processor Inventory Number */
pub const X86_FEATURE_CDP_L2: u32 = ( 7*32+15); /* "cdp_l2" Code and Data Prioritization L2 */
pub const X86_FEATURE_MSR_SPEC_CTRL: u32 = ( 7*32+16); /* MSR SPEC_CTRL is implemented */
pub const X86_FEATURE_SSBD: u32 = ( 7*32+17); /* "ssbd" Speculative Store Bypass Disable */
pub const X86_FEATURE_MBA: u32 = ( 7*32+18); /* "mba" Memory Bandwidth Allocation */
pub const X86_FEATURE_RSB_CTXSW: u32 = ( 7*32+19); /* Fill RSB on context switches */
pub const X86_FEATURE_PERFMON_V2: u32 = ( 7*32+20); /* "perfmon_v2" AMD Performance Monitoring Version 2 */
pub const X86_FEATURE_USE_IBRS_FW: u32 = ( 7*32+22); /* Use IBRS during runtime firmware calls */
pub const X86_FEATURE_SPEC_STORE_BYPASS_DISABLE: u32 = ( 7*32+23); /* Disable Speculative Store Bypass. */
pub const X86_FEATURE_LS_CFG_SSBD: u32 = ( 7*32+24); /* AMD SSBD implementation via LS_CFG MSR */
pub const X86_FEATURE_IBRS: u32 = ( 7*32+25); /* "ibrs" Indirect Branch Restricted Speculation */
pub const X86_FEATURE_IBPB: u32 = ( 7*32+26); /* "ibpb" Indirect Branch Prediction Barrier without a guaranteed RSB flush */
pub const X86_FEATURE_STIBP: u32 = ( 7*32+27); /* "stibp" Single Thread Indirect Branch Predictors */
pub const X86_FEATURE_ZEN: u32 = ( 7*32+28); /* Generic flag for all Zen and newer */
pub const X86_FEATURE_L1TF_PTEINV: u32 = ( 7*32+29); /* L1TF workaround PTE inversion */
pub const X86_FEATURE_IBRS_ENHANCED: u32 = ( 7*32+30); /* "ibrs_enhanced" Enhanced IBRS */
pub const X86_FEATURE_MSR_IA32_FEAT_CTL: u32 = ( 7*32+31); /* MSR IA32_FEAT_CTL configured */

/* Virtualization flags: Linux defined, word 8 */
pub const X86_FEATURE_TPR_SHADOW: u32 = ( 8*32+ 0); /* "tpr_shadow" Intel TPR Shadow */
pub const X86_FEATURE_FLEXPRIORITY: u32 = ( 8*32+ 1); /* "flexpriority" Intel FlexPriority */
pub const X86_FEATURE_EPT: u32 = ( 8*32+ 2); /* "ept" Intel Extended Page Table */
pub const X86_FEATURE_VPID: u32 = ( 8*32+ 3); /* "vpid" Intel Virtual Processor ID */
pub const X86_FEATURE_COHERENCY_SFW_NO: u32 = ( 8*32+ 4); /* SNP cache coherency software work around not needed */

pub const X86_FEATURE_VMMCALL: u32 = ( 8*32+15); /* "vmmcall" Prefer VMMCALL to VMCALL */
pub const X86_FEATURE_XENPV: u32 = ( 8*32+16); /* Xen paravirtual guest */
pub const X86_FEATURE_EPT_AD: u32 = ( 8*32+17); /* "ept_ad" Intel Extended Page Table access-dirty bit */
pub const X86_FEATURE_VMCALL: u32 = ( 8*32+18); /* Hypervisor supports the VMCALL instruction */
pub const X86_FEATURE_VMW_VMMCALL: u32 = ( 8*32+19); /* VMware prefers VMMCALL hypercall instruction */
pub const X86_FEATURE_PVUNLOCK: u32 = ( 8*32+20); /* PV unlock function */
pub const X86_FEATURE_VCPUPREEMPT: u32 = ( 8*32+21); /* PV vcpu_is_preempted function */
pub const X86_FEATURE_TDX_GUEST: u32 = ( 8*32+22); /* "tdx_guest" Intel Trust Domain Extensions Guest */

/* Intel-defined CPU features, CPUID level 0x00000007:0 (EBX), word 9 */
pub const X86_FEATURE_FSGSBASE: u32 = ( 9*32+ 0); /* "fsgsbase" RDFSBASE, WRFSBASE, RDGSBASE, WRGSBASE instructions*/
pub const X86_FEATURE_TSC_ADJUST: u32 = ( 9*32+ 1); /* "tsc_adjust" TSC adjustment MSR 0x3B */
pub const X86_FEATURE_SGX: u32 = ( 9*32+ 2); /* "sgx" Software Guard Extensions */
pub const X86_FEATURE_BMI1: u32 = ( 9*32+ 3); /* "bmi1" 1st group bit manipulation extensions */
pub const X86_FEATURE_HLE: u32 = ( 9*32+ 4); /* "hle" Hardware Lock Elision */
pub const X86_FEATURE_AVX2: u32 = ( 9*32+ 5); /* "avx2" AVX2 instructions */
pub const X86_FEATURE_FDP_EXCPTN_ONLY: u32 = ( 9*32+ 6); /* FPU data pointer updated only on x87 exceptions */
pub const X86_FEATURE_SMEP: u32 = ( 9*32+ 7); /* "smep" Supervisor Mode Execution Protection */
pub const X86_FEATURE_BMI2: u32 = ( 9*32+ 8); /* "bmi2" 2nd group bit manipulation extensions */
pub const X86_FEATURE_ERMS: u32 = ( 9*32+ 9); /* "erms" Enhanced REP MOVSB/STOSB instructions */
pub const X86_FEATURE_INVPCID: u32 = ( 9*32+10); /* "invpcid" Invalidate Processor Context ID */
pub const X86_FEATURE_RTM: u32 = ( 9*32+11); /* "rtm" Restricted Transactional Memory */
pub const X86_FEATURE_CQM: u32 = ( 9*32+12); /* "cqm" Cache QoS Monitoring */
pub const X86_FEATURE_ZERO_FCS_FDS: u32 = ( 9*32+13); /* Zero out FPU CS and FPU DS */
pub const X86_FEATURE_MPX: u32 = ( 9*32+14); /* "mpx" Memory Protection Extension */
pub const X86_FEATURE_RDT_A: u32 = ( 9*32+15); /* "rdt_a" Resource Director Technology Allocation */
pub const X86_FEATURE_AVX512F: u32 = ( 9*32+16); /* "avx512f" AVX-512 Foundation */
pub const X86_FEATURE_AVX512DQ: u32 = ( 9*32+17); /* "avx512dq" AVX-512 DQ (Double/Quad granular) Instructions */
pub const X86_FEATURE_RDSEED: u32 = ( 9*32+18); /* "rdseed" RDSEED instruction */
pub const X86_FEATURE_ADX: u32 = ( 9*32+19); /* "adx" ADCX and ADOX instructions */
pub const X86_FEATURE_SMAP: u32 = ( 9*32+20); /* "smap" Supervisor Mode Access Prevention */
pub const X86_FEATURE_AVX512IFMA: u32 = ( 9*32+21); /* "avx512ifma" AVX-512 Integer Fused Multiply-Add instructions */
pub const X86_FEATURE_CLFLUSHOPT: u32 = ( 9*32+23); /* "clflushopt" CLFLUSHOPT instruction */
pub const X86_FEATURE_CLWB: u32 = ( 9*32+24); /* "clwb" CLWB instruction */
pub const X86_FEATURE_INTEL_PT: u32 = ( 9*32+25); /* "intel_pt" Intel Processor Trace */
pub const X86_FEATURE_AVX512PF: u32 = ( 9*32+26); /* "avx512pf" AVX-512 Prefetch */
pub const X86_FEATURE_AVX512ER: u32 = ( 9*32+27); /* "avx512er" AVX-512 Exponential and Reciprocal */
pub const X86_FEATURE_AVX512CD: u32 = ( 9*32+28); /* "avx512cd" AVX-512 Conflict Detection */
pub const X86_FEATURE_SHA_NI: u32 = ( 9*32+29); /* "sha_ni" SHA1/SHA256 Instruction Extensions */
pub const X86_FEATURE_AVX512BW: u32 = ( 9*32+30); /* "avx512bw" AVX-512 BW (Byte/Word granular) Instructions */
pub const X86_FEATURE_AVX512VL: u32 = ( 9*32+31); /* "avx512vl" AVX-512 VL (128/256 Vector Length) Extensions */

/* Extended state features, CPUID level 0x0000000d:1 (EAX), word 10 */
pub const X86_FEATURE_XSAVEOPT: u32 = (10*32+ 0); /* "xsaveopt" XSAVEOPT instruction */
pub const X86_FEATURE_XSAVEC: u32 = (10*32+ 1); /* "xsavec" XSAVEC instruction */
pub const X86_FEATURE_XGETBV1: u32 = (10*32+ 2); /* "xgetbv1" XGETBV with ECX = 1 instruction */
pub const X86_FEATURE_XSAVES: u32 = (10*32+ 3); /* "xsaves" XSAVES/XRSTORS instructions */
pub const X86_FEATURE_XFD: u32 = (10*32+ 4); /* eXtended Feature Disabling */

/*
 * Extended auxiliary flags: Linux defined - for features scattered in various
 * CPUID levels like 0xf, etc.
 *
 * Reuse free bits when adding new feature flags!
 */
pub const X86_FEATURE_CQM_LLC: u32 = (11*32+ 0); /* "cqm_llc" LLC QoS if 1 */
pub const X86_FEATURE_CQM_OCCUP_LLC: u32 = (11*32+ 1); /* "cqm_occup_llc" LLC occupancy monitoring */
pub const X86_FEATURE_CQM_MBM_TOTAL: u32 = (11*32+ 2); /* "cqm_mbm_total" LLC Total MBM monitoring */
pub const X86_FEATURE_CQM_MBM_LOCAL: u32 = (11*32+ 3); /* "cqm_mbm_local" LLC Local MBM monitoring */
pub const X86_FEATURE_FENCE_SWAPGS_USER: u32 = (11*32+ 4); /* LFENCE in user entry SWAPGS path */
pub const X86_FEATURE_FENCE_SWAPGS_KERNEL: u32 = (11*32+ 5); /* LFENCE in kernel entry SWAPGS path */
pub const X86_FEATURE_SPLIT_LOCK_DETECT: u32 = (11*32+ 6); /* "split_lock_detect" #AC for split lock */
pub const X86_FEATURE_PER_THREAD_MBA: u32 = (11*32+ 7); /* Per-thread Memory Bandwidth Allocation */
pub const X86_FEATURE_SGX1: u32 = (11*32+ 8); /* Basic SGX */
pub const X86_FEATURE_SGX2: u32 = (11*32+ 9); /* SGX Enclave Dynamic Memory Management (EDMM) */
pub const X86_FEATURE_ENTRY_IBPB: u32 = (11*32+10); /* Issue an IBPB on kernel entry */
pub const X86_FEATURE_RRSBA_CTRL: u32 = (11*32+11); /* RET prediction control */
pub const X86_FEATURE_RETPOLINE: u32 = (11*32+12); /* Generic Retpoline mitigation for Spectre variant 2 */
pub const X86_FEATURE_RETPOLINE_LFENCE: u32 = (11*32+13); /* Use LFENCE for Spectre variant 2 */
pub const X86_FEATURE_RETHUNK: u32 = (11*32+14); /* Use REturn THUNK */
pub const X86_FEATURE_UNRET: u32 = (11*32+15); /* AMD BTB untrain return */
pub const X86_FEATURE_USE_IBPB_FW: u32 = (11*32+16); /* Use IBPB during runtime firmware calls */
pub const X86_FEATURE_RSB_VMEXIT_LITE: u32 = (11*32+17); /* Fill RSB on VM exit when EIBRS is enabled */
pub const X86_FEATURE_SGX_EDECCSSA: u32 = (11*32+18); /* SGX EDECCSSA user leaf function */
pub const X86_FEATURE_CALL_DEPTH: u32 = (11*32+19); /* Call depth tracking for RSB stuffing */
pub const X86_FEATURE_MSR_TSX_CTRL: u32 = (11*32+20); /* MSR IA32_TSX_CTRL (Intel) implemented */
pub const X86_FEATURE_SMBA: u32 = (11*32+21); /* Slow Memory Bandwidth Allocation */
pub const X86_FEATURE_BMEC: u32 = (11*32+22); /* Bandwidth Monitoring Event Configuration */
pub const X86_FEATURE_USER_SHSTK: u32 = (11*32+23); /* "user_shstk" Shadow stack support for user mode applications */
pub const X86_FEATURE_SRSO: u32 = (11*32+24); /* AMD BTB untrain RETs */
pub const X86_FEATURE_SRSO_ALIAS: u32 = (11*32+25); /* AMD BTB untrain RETs through aliasing */
pub const X86_FEATURE_IBPB_ON_VMEXIT: u32 = (11*32+26); /* Issue an IBPB only on VMEXIT */
pub const X86_FEATURE_APIC_MSRS_FENCE: u32 = (11*32+27); /* IA32_TSC_DEADLINE and X2APIC MSRs need fencing */
pub const X86_FEATURE_ZEN2: u32 = (11*32+28); /* CPU based on Zen2 microarchitecture */
pub const X86_FEATURE_ZEN3: u32 = (11*32+29); /* CPU based on Zen3 microarchitecture */
pub const X86_FEATURE_ZEN4: u32 = (11*32+30); /* CPU based on Zen4 microarchitecture */
pub const X86_FEATURE_ZEN1: u32 = (11*32+31); /* CPU based on Zen1 microarchitecture */

/* Intel-defined CPU features, CPUID level 0x00000007:1 (EAX), word 12 */
pub const X86_FEATURE_SHA512: u32 = (12*32+ 0); /* SHA512 instructions */
pub const X86_FEATURE_SM3: u32 = (12*32+ 1); /* SM3 instructions */
pub const X86_FEATURE_SM4: u32 = (12*32+ 2); /* SM4 instructions */
pub const X86_FEATURE_AVX_VNNI: u32 = (12*32+ 4); /* "avx_vnni" AVX VNNI instructions */
pub const X86_FEATURE_AVX512_BF16: u32 = (12*32+ 5); /* "avx512_bf16" AVX512 BFLOAT16 instructions */
pub const X86_FEATURE_LASS: u32 = (12*32+ 6); /* "lass" Linear Address Space Separation */
pub const X86_FEATURE_CMPCCXADD: u32 = (12*32+ 7); /* CMPccXADD instructions */
pub const X86_FEATURE_ARCH_PERFMON_EXT: u32 = (12*32+ 8); /* Intel Architectural PerfMon Extension */
pub const X86_FEATURE_FZRM: u32 = (12*32+10); /* Fast zero-length REP MOVSB */
pub const X86_FEATURE_FSRS: u32 = (12*32+11); /* Fast short REP STOSB */
pub const X86_FEATURE_FSRC: u32 = (12*32+12); /* Fast short REP {CMPSB,SCASB} */
pub const X86_FEATURE_FRED: u32 = (12*32+17); /* "fred" Flexible Return and Event Delivery */
pub const X86_FEATURE_LKGS: u32 = (12*32+18); /* Like MOV_GS except MSR_KERNEL_GS_BASE = GS.base */
pub const X86_FEATURE_WRMSRNS: u32 = (12*32+19); /* Non-serializing WRMSR */
pub const X86_FEATURE_AMX_FP16: u32 = (12*32+21); /* AMX fp16 Support */
pub const X86_FEATURE_AVX_IFMA: u32 = (12*32+23); /* Support for VPMADD52[H,L]UQ */
pub const X86_FEATURE_LAM: u32 = (12*32+26); /* "lam" Linear Address Masking */
pub const X86_FEATURE_MOVRS: u32 = (12*32+31); /* MOVRS instructions */

/* AMD-defined CPU features, CPUID level 0x80000008 (EBX), word 13 */
pub const X86_FEATURE_CLZERO: u32 = (13*32+ 0); /* "clzero" CLZERO instruction */
pub const X86_FEATURE_IRPERF: u32 = (13*32+ 1); /* "irperf" Instructions Retired Count */
pub const X86_FEATURE_XSAVEERPTR: u32 = (13*32+ 2); /* "xsaveerptr" Always save/restore FP error pointers */
pub const X86_FEATURE_INVLPGB: u32 = (13*32+ 3); /* INVLPGB and TLBSYNC instructions supported */
pub const X86_FEATURE_RDPRU: u32 = (13*32+ 4); /* "rdpru" Read processor register at user level */
pub const X86_FEATURE_WBNOINVD: u32 = (13*32+ 9); /* "wbnoinvd" WBNOINVD instruction */
pub const X86_FEATURE_AMD_IBPB: u32 = (13*32+12); /* Indirect Branch Prediction Barrier */
pub const X86_FEATURE_AMD_IBRS: u32 = (13*32+14); /* Indirect Branch Restricted Speculation */
pub const X86_FEATURE_AMD_STIBP: u32 = (13*32+15); /* Single Thread Indirect Branch Predictors */
pub const X86_FEATURE_AMD_STIBP_ALWAYS_ON: u32 = (13*32+17); /* Single Thread Indirect Branch Predictors always-on preferred */
pub const X86_FEATURE_AMD_IBRS_SAME_MODE: u32 = (13*32+19); /* Indirect Branch Restricted Speculation same mode protection*/
pub const X86_FEATURE_EFER_LMSLE_MBZ: u32 = (13*32+20); /* EFER.LMSLE must be zero */
pub const X86_FEATURE_AMD_PPIN: u32 = (13*32+23); /* "amd_ppin" Protected Processor Inventory Number */
pub const X86_FEATURE_AMD_SSBD: u32 = (13*32+24); /* Speculative Store Bypass Disable */
pub const X86_FEATURE_VIRT_SSBD: u32 = (13*32+25); /* "virt_ssbd" Virtualized Speculative Store Bypass Disable */
pub const X86_FEATURE_AMD_SSB_NO: u32 = (13*32+26); /* Speculative Store Bypass is fixed in hardware. */
pub const X86_FEATURE_CPPC: u32 = (13*32+27); /* "cppc" Collaborative Processor Performance Control */
pub const X86_FEATURE_AMD_PSFD: u32 = (13*32+28); /* Predictive Store Forwarding Disable */
pub const X86_FEATURE_BTC_NO: u32 = (13*32+29); /* Not vulnerable to Branch Type Confusion */
pub const X86_FEATURE_AMD_IBPB_RET: u32 = (13*32+30); /* IBPB clears return address predictor */
pub const X86_FEATURE_BRS: u32 = (13*32+31); /* "brs" Branch Sampling available */

/* Thermal and Power Management Leaf, CPUID level 0x00000006 (EAX), word 14 */
pub const X86_FEATURE_DTHERM: u32 = (14*32+ 0); /* "dtherm" Digital Thermal Sensor */
pub const X86_FEATURE_IDA: u32 = (14*32+ 1); /* "ida" Intel Dynamic Acceleration */
pub const X86_FEATURE_ARAT: u32 = (14*32+ 2); /* "arat" Always Running APIC Timer */
pub const X86_FEATURE_PLN: u32 = (14*32+ 4); /* "pln" Intel Power Limit Notification */
pub const X86_FEATURE_PTS: u32 = (14*32+ 6); /* "pts" Intel Package Thermal Status */
pub const X86_FEATURE_HWP: u32 = (14*32+ 7); /* "hwp" Intel Hardware P-states */
pub const X86_FEATURE_HWP_NOTIFY: u32 = (14*32+ 8); /* "hwp_notify" HWP Notification */
pub const X86_FEATURE_HWP_ACT_WINDOW: u32 = (14*32+ 9); /* "hwp_act_window" HWP Activity Window */
pub const X86_FEATURE_HWP_EPP: u32 = (14*32+10); /* "hwp_epp" HWP Energy Perf. Preference */
pub const X86_FEATURE_HWP_PKG_REQ: u32 = (14*32+11); /* "hwp_pkg_req" HWP Package Level Request */
pub const X86_FEATURE_HWP_HIGHEST_PERF_CHANGE: u32 = (14*32+15); /* HWP Highest perf change */
pub const X86_FEATURE_HFI: u32 = (14*32+19); /* "hfi" Hardware Feedback Interface */

/* AMD SVM Feature Identification, CPUID level 0x8000000a (EDX), word 15 */
pub const X86_FEATURE_NPT: u32 = (15*32+ 0); /* "npt" Nested Page Table support */
pub const X86_FEATURE_LBRV: u32 = (15*32+ 1); /* "lbrv" LBR Virtualization support */
pub const X86_FEATURE_SVML: u32 = (15*32+ 2); /* "svm_lock" SVM locking MSR */
pub const X86_FEATURE_NRIPS: u32 = (15*32+ 3); /* "nrip_save" SVM next_rip save */
pub const X86_FEATURE_TSCRATEMSR: u32 = (15*32+ 4); /* "tsc_scale" TSC scaling support */
pub const X86_FEATURE_VMCBCLEAN: u32 = (15*32+ 5); /* "vmcb_clean" VMCB clean bits support */
pub const X86_FEATURE_FLUSHBYASID: u32 = (15*32+ 6); /* "flushbyasid" Flush-by-ASID support */
pub const X86_FEATURE_DECODEASSISTS: u32 = (15*32+ 7); /* "decodeassists" Decode Assists support */
pub const X86_FEATURE_PAUSEFILTER: u32 = (15*32+10); /* "pausefilter" Filtered pause intercept */
pub const X86_FEATURE_PFTHRESHOLD: u32 = (15*32+12); /* "pfthreshold" Pause filter threshold */
pub const X86_FEATURE_AVIC: u32 = (15*32+13); /* "avic" Virtual Interrupt Controller */
pub const X86_FEATURE_V_VMSAVE_VMLOAD: u32 = (15*32+15); /* "v_vmsave_vmload" Virtual VMSAVE VMLOAD */
pub const X86_FEATURE_VGIF: u32 = (15*32+16); /* "vgif" Virtual GIF */
pub const X86_FEATURE_GMET: u32 = (15*32+17); /* Guest Mode Execution Trap */
pub const X86_FEATURE_X2AVIC: u32 = (15*32+18); /* "x2avic" Virtual x2apic */
pub const X86_FEATURE_V_SPEC_CTRL: u32 = (15*32+20); /* "v_spec_ctrl" Virtual SPEC_CTRL */
pub const X86_FEATURE_VNMI: u32 = (15*32+25); /* "vnmi" Virtual NMI */
pub const X86_FEATURE_SVME_ADDR_CHK: u32 = (15*32+28); /* SVME addr check */
pub const X86_FEATURE_BUS_LOCK_THRESHOLD: u32 = (15*32+29); /* Bus lock threshold */
pub const X86_FEATURE_IDLE_HLT: u32 = (15*32+30); /* IDLE HLT intercept */

/* Intel-defined CPU features, CPUID level 0x00000007:0 (ECX), word 16 */
pub const X86_FEATURE_AVX512VBMI: u32 = (16*32+ 1); /* "avx512vbmi" AVX512 Vector Bit Manipulation instructions*/
pub const X86_FEATURE_UMIP: u32 = (16*32+ 2); /* "umip" User Mode Instruction Protection */
pub const X86_FEATURE_PKU: u32 = (16*32+ 3); /* "pku" Protection Keys for Userspace */
pub const X86_FEATURE_OSPKE: u32 = (16*32+ 4); /* "ospke" OS Protection Keys Enable */
pub const X86_FEATURE_WAITPKG: u32 = (16*32+ 5); /* "waitpkg" UMONITOR/UMWAIT/TPAUSE Instructions */
pub const X86_FEATURE_AVX512_VBMI2: u32 = (16*32+ 6); /* "avx512_vbmi2" Additional AVX512 Vector Bit Manipulation Instructions */
pub const X86_FEATURE_SHSTK: u32 = (16*32+ 7); /* Shadow stack */
pub const X86_FEATURE_GFNI: u32 = (16*32+ 8); /* "gfni" Galois Field New Instructions */
pub const X86_FEATURE_VAES: u32 = (16*32+ 9); /* "vaes" Vector AES */
pub const X86_FEATURE_VPCLMULQDQ: u32 = (16*32+10); /* "vpclmulqdq" Carry-Less Multiplication Double Quadword */
pub const X86_FEATURE_AVX512_VNNI: u32 = (16*32+11); /* "avx512_vnni" Vector Neural Network Instructions */
pub const X86_FEATURE_AVX512_BITALG: u32 = (16*32+12); /* "avx512_bitalg" Support for VPOPCNT[B,W] and VPSHUF-BITQMB instructions */
pub const X86_FEATURE_TME: u32 = (16*32+13); /* "tme" Intel Total Memory Encryption */
pub const X86_FEATURE_AVX512_VPOPCNTDQ: u32 = (16*32+14); /* "avx512_vpopcntdq" POPCNT for vectors of DW/QW */
pub const X86_FEATURE_LA57: u32 = (16*32+16); /* "la57" 5-level page tables */
pub const X86_FEATURE_RDPID: u32 = (16*32+22); /* "rdpid" RDPID instruction */
pub const X86_FEATURE_BUS_LOCK_DETECT: u32 = (16*32+24); /* "bus_lock_detect" Bus Lock detect */
pub const X86_FEATURE_CLDEMOTE: u32 = (16*32+25); /* "cldemote" CLDEMOTE instruction */
pub const X86_FEATURE_MOVDIRI: u32 = (16*32+27); /* "movdiri" MOVDIRI instruction */
pub const X86_FEATURE_MOVDIR64B: u32 = (16*32+28); /* "movdir64b" MOVDIR64B instruction */
pub const X86_FEATURE_ENQCMD: u32 = (16*32+29); /* "enqcmd" ENQCMD and ENQCMDS instructions */
pub const X86_FEATURE_SGX_LC: u32 = (16*32+30); /* "sgx_lc" Software Guard Extensions Launch Control */

/*
 * Linux-defined word for use with scattered/synthetic bits.
 */
pub const X86_FEATURE_OVERFLOW_RECOV: u32 = (17*32+ 0); /* "overflow_recov" MCA overflow recovery support */
pub const X86_FEATURE_SUCCOR: u32 = (17*32+ 1); /* "succor" Uncorrectable error containment and recovery */
pub const X86_FEATURE_CPPC_PERF_PRIO: u32 = (17*32+ 2); /* CPPC Floor Perf support */
pub const X86_FEATURE_SMCA: u32 = (17*32+ 3); /* "smca" Scalable MCA */

/* Intel-defined CPU features, CPUID level 0x00000007:0 (EDX), word 18 */
pub const X86_FEATURE_AVX512_4VNNIW: u32 = (18*32+ 2); /* "avx512_4vnniw" AVX-512 Neural Network Instructions */
pub const X86_FEATURE_AVX512_4FMAPS: u32 = (18*32+ 3); /* "avx512_4fmaps" AVX-512 Multiply Accumulation Single precision */
pub const X86_FEATURE_FSRM: u32 = (18*32+ 4); /* "fsrm" Fast Short Rep Mov */
pub const X86_FEATURE_AVX512_VP2INTERSECT: u32 = (18*32+ 8); /* "avx512_vp2intersect" AVX-512 Intersect for D/Q */
pub const X86_FEATURE_SRBDS_CTRL: u32 = (18*32+ 9); /* SRBDS mitigation MSR available */
pub const X86_FEATURE_MD_CLEAR: u32 = (18*32+10); /* "md_clear" VERW clears CPU buffers */
pub const X86_FEATURE_RTM_ALWAYS_ABORT: u32 = (18*32+11); /* RTM transaction always aborts */
pub const X86_FEATURE_TSX_FORCE_ABORT: u32 = (18*32+13); /* TSX_FORCE_ABORT */
pub const X86_FEATURE_SERIALIZE: u32 = (18*32+14); /* "serialize" SERIALIZE instruction */
pub const X86_FEATURE_HYBRID_CPU: u32 = (18*32+15); /* This part has CPUs of more than one type */
pub const X86_FEATURE_TSXLDTRK: u32 = (18*32+16); /* "tsxldtrk" TSX Suspend Load Address Tracking */
pub const X86_FEATURE_PCONFIG: u32 = (18*32+18); /* "pconfig" Intel PCONFIG */
pub const X86_FEATURE_ARCH_LBR: u32 = (18*32+19); /* "arch_lbr" Intel ARCH LBR */
pub const X86_FEATURE_IBT: u32 = (18*32+20); /* "ibt" Indirect Branch Tracking */
pub const X86_FEATURE_AMX_BF16: u32 = (18*32+22); /* "amx_bf16" AMX bf16 Support */
pub const X86_FEATURE_AVX512_FP16: u32 = (18*32+23); /* "avx512_fp16" AVX512 FP16 */
pub const X86_FEATURE_AMX_TILE: u32 = (18*32+24); /* "amx_tile" AMX tile Support */
pub const X86_FEATURE_AMX_INT8: u32 = (18*32+25); /* "amx_int8" AMX int8 Support */
pub const X86_FEATURE_SPEC_CTRL: u32 = (18*32+26); /* Speculation Control (IBRS + IBPB) */
pub const X86_FEATURE_INTEL_STIBP: u32 = (18*32+27); /* Single Thread Indirect Branch Predictors */
pub const X86_FEATURE_FLUSH_L1D: u32 = (18*32+28); /* "flush_l1d" Flush L1D cache */
pub const X86_FEATURE_ARCH_CAPABILITIES: u32 = (18*32+29); /* "arch_capabilities" IA32_ARCH_CAPABILITIES MSR (Intel) */
pub const X86_FEATURE_CORE_CAPABILITIES: u32 = (18*32+30); /* IA32_CORE_CAPABILITIES MSR */
pub const X86_FEATURE_SPEC_CTRL_SSBD: u32 = (18*32+31); /* Speculative Store Bypass Disable */

/* AMD-defined memory encryption features, CPUID level 0x8000001f (EAX), word 19 */
pub const X86_FEATURE_SME: u32 = (19*32+ 0); /* "sme" Secure Memory Encryption */
pub const X86_FEATURE_SEV: u32 = (19*32+ 1); /* "sev" Secure Encrypted Virtualization */
pub const X86_FEATURE_VM_PAGE_FLUSH: u32 = (19*32+ 2); /* VM Page Flush MSR is supported */
pub const X86_FEATURE_SEV_ES: u32 = (19*32+ 3); /* "sev_es" Secure Encrypted Virtualization - Encrypted State */
pub const X86_FEATURE_SEV_SNP: u32 = (19*32+ 4); /* "sev_snp" Secure Encrypted Virtualization - Secure Nested Paging */
pub const X86_FEATURE_SNP_SECURE_TSC: u32 = (19*32+ 8); /* SEV-SNP Secure TSC */
pub const X86_FEATURE_V_TSC_AUX: u32 = (19*32+ 9); /* Virtual TSC_AUX */
pub const X86_FEATURE_SME_COHERENT: u32 = (19*32+10); /* hardware-enforced cache coherency */
pub const X86_FEATURE_DEBUG_SWAP: u32 = (19*32+14); /* "debug_swap" SEV-ES full debug state swap support */
pub const X86_FEATURE_RMPREAD: u32 = (19*32+21); /* RMPREAD instruction */
pub const X86_FEATURE_SEGMENTED_RMP: u32 = (19*32+23); /* Segmented RMP support */
pub const X86_FEATURE_ALLOWED_SEV_FEATURES: u32 = (19*32+27); /* Allowed SEV Features */
pub const X86_FEATURE_SVSM: u32 = (19*32+28); /* "svsm" SVSM present */
pub const X86_FEATURE_HV_INUSE_WR_ALLOWED: u32 = (19*32+30); /* Allow Write to in-use hypervisor-owned pages */

/* AMD-defined Extended Feature 2 EAX, CPUID level 0x80000021 (EAX), word 20 */
pub const X86_FEATURE_NO_NESTED_DATA_BP: u32 = (20*32+ 0); /* No Nested Data Breakpoints */
pub const X86_FEATURE_WRMSR_XX_BASE_NS: u32 = (20*32+ 1); /* WRMSR to {FS,GS,KERNEL_GS}_BASE is non-serializing */
pub const X86_FEATURE_LFENCE_RDTSC: u32 = (20*32+ 2); /* LFENCE always serializing / synchronizes RDTSC */
pub const X86_FEATURE_VERW_CLEAR: u32 = (20*32+ 5); /* The memory form of VERW mitigates TSA */
pub const X86_FEATURE_NULL_SEL_CLR_BASE: u32 = (20*32+ 6); /* Null Selector Clears Base */

pub const X86_FEATURE_AUTOIBRS: u32 = (20*32+ 8); /* Automatic IBRS */
pub const X86_FEATURE_NO_SMM_CTL_MSR: u32 = (20*32+ 9); /* SMM_CTL MSR is not present */

pub const X86_FEATURE_GP_ON_USER_CPUID: u32 = (20*32+17); /* User CPUID faulting */

pub const X86_FEATURE_PREFETCHI: u32 = (20*32+20); /* Prefetch Data/Instruction to Cache Level */
pub const X86_FEATURE_AVX512_BMM: u32 = (20*32+23); /* AVX512 Bit Matrix Multiply instructions */
pub const X86_FEATURE_ERAPS: u32 = (20*32+24); /* Enhanced Return Address Predictor Security */
pub const X86_FEATURE_SBPB: u32 = (20*32+27); /* Selective Branch Prediction Barrier */
pub const X86_FEATURE_IBPB_BRTYPE: u32 = (20*32+28); /* MSR_PRED_CMD[IBPB] flushes all branch type predictions */
pub const X86_FEATURE_SRSO_NO: u32 = (20*32+29); /* CPU is not affected by SRSO */
pub const X86_FEATURE_SRSO_USER_KERNEL_NO: u32 = (20*32+30); /* CPU is not affected by SRSO across user/kernel boundaries */
pub const X86_FEATURE_SRSO_BP_SPEC_REDUCE: u32 = (20*32+31); /*
						    * BP_CFG[BpSpecReduce] can be used to mitigate SRSO for VMs.
						    * (SRSO_MSR_FIX in the official doc).
						    */

/*
 * Extended auxiliary flags: Linux defined - for features scattered in various
 * CPUID levels like 0x80000022, etc and Linux defined features.
 *
 * Reuse free bits when adding new feature flags!
 */
pub const X86_FEATURE_AMD_LBR_PMC_FREEZE: u32 = (21*32+ 0); /* "amd_lbr_pmc_freeze" AMD LBR and PMC Freeze */
pub const X86_FEATURE_CLEAR_BHB_LOOP: u32 = (21*32+ 1); /* Clear branch history at syscall entry using SW loop */
pub const X86_FEATURE_BHI_CTRL: u32 = (21*32+ 2); /* BHI_DIS_S HW control available */
pub const X86_FEATURE_CLEAR_BHB_HW: u32 = (21*32+ 3); /* BHI_DIS_S HW control enabled */
pub const X86_FEATURE_CLEAR_BHB_VMEXIT: u32 = (21*32+ 4); /* Clear branch history at vmexit using SW loop */
pub const X86_FEATURE_AMD_FAST_CPPC: u32 = (21*32+ 5); /* Fast CPPC */
pub const X86_FEATURE_AMD_HTR_CORES: u32 = (21*32+ 6); /* Heterogeneous Core Topology */
pub const X86_FEATURE_AMD_WORKLOAD_CLASS: u32 = (21*32+ 7); /* Workload Classification */
pub const X86_FEATURE_PREFER_YMM: u32 = (21*32+ 8); /* Avoid ZMM registers due to downclocking */
pub const X86_FEATURE_APX: u32 = (21*32+ 9); /* Advanced Performance Extensions */
pub const X86_FEATURE_INDIRECT_THUNK_ITS: u32 = (21*32+10); /* Use thunk for indirect branches in lower half of cacheline */
pub const X86_FEATURE_TSA_SQ_NO: u32 = (21*32+11); /* AMD CPU not vulnerable to TSA-SQ */
pub const X86_FEATURE_TSA_L1_NO: u32 = (21*32+12); /* AMD CPU not vulnerable to TSA-L1 */
pub const X86_FEATURE_CLEAR_CPU_BUF_VM: u32 = (21*32+13); /* Clear CPU buffers using VERW before VMRUN */
pub const X86_FEATURE_IBPB_EXIT_TO_USER: u32 = (21*32+14); /* Use IBPB on exit-to-userspace, see VMSCAPE bug */
pub const X86_FEATURE_ABMC: u32 = (21*32+15); /* Assignable Bandwidth Monitoring Counters */
pub const X86_FEATURE_MSR_IMM: u32 = (21*32+16); /* MSR immediate form instructions */
pub const X86_FEATURE_SGX_EUPDATESVN: u32 = (21*32+17); /* Support for ENCLS[EUPDATESVN] instruction */

pub const X86_FEATURE_SDCIAE: u32 = (21*32+18); /* L3 Smart Data Cache Injection Allocation Enforcement */
pub const X86_FEATURE_CLEAR_CPU_BUF_VM_MMIO: u32 = (21*32+19); /*
						      * Clear CPU buffers before VM-Enter if the vCPU
						      * can access host MMIO (ignored for all intents
						      * and purposes if CLEAR_CPU_BUF_VM is set).
						      */
pub const X86_FEATURE_X2AVIC_EXT: u32 = (21*32+20); /* AMD SVM x2AVIC support for 4k vCPUs */

/*
 * BUG word(s)
 */
#[inline]
pub const fn X86_BUG(x: u32) -> u32 {
    NCAPINTS * 32 + x
}

pub const X86_BUG_F00F: u32 = X86_BUG(0); /* "f00f" Intel F00F */
pub const X86_BUG_FDIV: u32 = X86_BUG(1); /* "fdiv" FPU FDIV */
pub const X86_BUG_COMA: u32 = X86_BUG(2); /* "coma" Cyrix 6x86 coma */
pub const X86_BUG_AMD_TLB_MMATCH: u32 = X86_BUG(3); /* "tlb_mmatch" AMD Erratum 383 */
pub const X86_BUG_AMD_APIC_C1E: u32 = X86_BUG(4); /* "apic_c1e" AMD Erratum 400 */
pub const X86_BUG_11AP: u32 = X86_BUG(5); /* "11ap" Bad local APIC aka 11AP */
pub const X86_BUG_FXSAVE_LEAK: u32 = X86_BUG(6); /* "fxsave_leak" FXSAVE leaks FOP/FIP/FOP */
pub const X86_BUG_CLFLUSH_MONITOR: u32 = X86_BUG(7); /* "clflush_monitor" AAI65, CLFLUSH required before MONITOR */
pub const X86_BUG_SYSRET_SS_ATTRS: u32 = X86_BUG(8); /* "sysret_ss_attrs" SYSRET doesn't fix up SS attrs */
pub const X86_BUG_NULL_SEG: u32 = X86_BUG(10); /* "null_seg" Nulling a selector preserves the base */
pub const X86_BUG_SWAPGS_FENCE: u32 = X86_BUG(11); /* "swapgs_fence" SWAPGS without input dep on GS */
pub const X86_BUG_MONITOR: u32 = X86_BUG(12); /* "monitor" IPI required to wake up remote CPU */
pub const X86_BUG_AMD_E400: u32 = X86_BUG(13); /* "amd_e400" CPU is among the affected by Erratum 400 */
pub const X86_BUG_CPU_MELTDOWN: u32 = X86_BUG(14); /* "cpu_meltdown" CPU is affected by meltdown attack and needs kernel page table isolation */
pub const X86_BUG_SPECTRE_V1: u32 = X86_BUG(15); /* "spectre_v1" CPU is affected by Spectre variant 1 attack with conditional branches */
pub const X86_BUG_SPECTRE_V2: u32 = X86_BUG(16); /* "spectre_v2" CPU is affected by Spectre variant 2 attack with indirect branches */
pub const X86_BUG_SPEC_STORE_BYPASS: u32 = X86_BUG(17); /* "spec_store_bypass" CPU is affected by speculative store bypass attack */
pub const X86_BUG_L1TF: u32 = X86_BUG(18); /* "l1tf" CPU is affected by L1 Terminal Fault */
pub const X86_BUG_MDS: u32 = X86_BUG(19); /* "mds" CPU is affected by Microarchitectural data sampling */
pub const X86_BUG_MSBDS_ONLY: u32 = X86_BUG(20); /* "msbds_only" CPU is only affected by the  MSDBS variant of BUG_MDS */
pub const X86_BUG_SWAPGS: u32 = X86_BUG(21); /* "swapgs" CPU is affected by speculation through SWAPGS */
pub const X86_BUG_TAA: u32 = X86_BUG(22); /* "taa" CPU is affected by TSX Async Abort(TAA) */
pub const X86_BUG_ITLB_MULTIHIT: u32 = X86_BUG(23); /* "itlb_multihit" CPU may incur MCE during certain page attribute changes */
pub const X86_BUG_SRBDS: u32 = X86_BUG(24); /* "srbds" CPU may leak RNG bits if not mitigated */
pub const X86_BUG_MMIO_STALE_DATA: u32 = X86_BUG(25); /* "mmio_stale_data" CPU is affected by Processor MMIO Stale Data vulnerabilities */
/* unused, was #define X86_BUG_MMIO_UNKNOWN		X86_BUG(26) "mmio_unknown" CPU is too old and its MMIO Stale Data status is unknown */
pub const X86_BUG_RETBLEED: u32 = X86_BUG(27); /* "retbleed" CPU is affected by RETBleed */
pub const X86_BUG_EIBRS_PBRSB: u32 = X86_BUG(28); /* "eibrs_pbrsb" EIBRS is vulnerable to Post Barrier RSB Predictions */
pub const X86_BUG_SMT_RSB: u32 = X86_BUG(29); /* "smt_rsb" CPU is vulnerable to Cross-Thread Return Address Predictions */
pub const X86_BUG_GDS: u32 = X86_BUG(30); /* "gds" CPU is affected by Gather Data Sampling */
pub const X86_BUG_TDX_PW_MCE: u32 = X86_BUG(31); /* "tdx_pw_mce" CPU may incur #MC if non-TD software does partial write to TDX private memory */

/* BUG word 2 */
pub const X86_BUG_SRSO: u32 = X86_BUG( 1*32+ 0); /* "srso" AMD SRSO bug */
pub const X86_BUG_DIV0: u32 = X86_BUG( 1*32+ 1); /* "div0" AMD DIV0 speculation bug */
pub const X86_BUG_RFDS: u32 = X86_BUG( 1*32+ 2); /* "rfds" CPU is vulnerable to Register File Data Sampling */
pub const X86_BUG_BHI: u32 = X86_BUG( 1*32+ 3); /* "bhi" CPU is affected by Branch History Injection */
pub const X86_BUG_IBPB_NO_RET: u32 = X86_BUG( 1*32+ 4); /* "ibpb_no_ret" IBPB omits return target predictions */
pub const X86_BUG_SPECTRE_V2_USER: u32 = X86_BUG( 1*32+ 5); /* "spectre_v2_user" CPU is affected by Spectre variant 2 attack between user processes */
pub const X86_BUG_OLD_MICROCODE: u32 = X86_BUG( 1*32+ 6); /* "old_microcode" CPU has old microcode, it is surely vulnerable to something */
pub const X86_BUG_ITS: u32 = X86_BUG( 1*32+ 7); /* "its" CPU is affected by Indirect Target Selection */
pub const X86_BUG_ITS_NATIVE_ONLY: u32 = X86_BUG( 1*32+ 8); /* "its_native_only" CPU is affected by ITS, VMX is not affected */
pub const X86_BUG_TSA: u32 = X86_BUG( 1*32+ 9); /* "tsa" CPU is affected by Transient Scheduler Attacks */
pub const X86_BUG_VMSCAPE: u32 = X86_BUG( 1*32+10); /* "vmscape" CPU is affected by VMSCAPE attacks from guests */
pub const X86_BUG_SEAMRET_INVD_VMCS: u32 = X86_BUG( 1*32+11); /* "seamret_invd_vmcs" SEAMRET from P-SEAMLDR clears the current VMCS */
