// SPDX-License-Identifier: GPL-2.0 WITH Linux-syscall-note
/*
 * vmx.h: VMX Architecture related definitions
 * Copyright (c) 2004, Intel Corporation.
 *
 * This program is free software; you can redistribute it and/or modify it
 * under the terms and conditions of the GNU General Public License,
 * version 2, as published by the Free Software Foundation.
 *
 * This program is distributed in the hope it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
 * more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * this program; if not, write to the Free Software Foundation, Inc., 59 Temple
 * Place - Suite 330, Boston, MA 02111-1307 USA.
 *
 * A few random additions are:
 * Copyright (C) 2006 Qumranet
 *    Avi Kivity <avi@qumranet.com>
 *    Yaniv Kamay <yaniv@qumranet.com>
 *
 */
//! linux-source: arch/x86/include/uapi/asm/vmx.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000805

pub const VMX_EXIT_REASONS_FAILED_VMENTRY: u32 = 0x8000_0000;
pub const VMX_EXIT_REASONS_SGX_ENCLAVE_MODE: i32 = 0x0800_0000;

pub const EXIT_REASON_EXCEPTION_NMI: i32 = 0;
pub const EXIT_REASON_EXTERNAL_INTERRUPT: i32 = 1;
pub const EXIT_REASON_TRIPLE_FAULT: i32 = 2;
pub const EXIT_REASON_INIT_SIGNAL: i32 = 3;
pub const EXIT_REASON_SIPI_SIGNAL: i32 = 4;
pub const EXIT_REASON_OTHER_SMI: i32 = 6;
pub const EXIT_REASON_INTERRUPT_WINDOW: i32 = 7;
pub const EXIT_REASON_NMI_WINDOW: i32 = 8;
pub const EXIT_REASON_TASK_SWITCH: i32 = 9;
pub const EXIT_REASON_CPUID: i32 = 10;
pub const EXIT_REASON_HLT: i32 = 12;
pub const EXIT_REASON_INVD: i32 = 13;
pub const EXIT_REASON_INVLPG: i32 = 14;
pub const EXIT_REASON_RDPMC: i32 = 15;
pub const EXIT_REASON_RDTSC: i32 = 16;
pub const EXIT_REASON_VMCALL: i32 = 18;
pub const EXIT_REASON_VMCLEAR: i32 = 19;
pub const EXIT_REASON_VMLAUNCH: i32 = 20;
pub const EXIT_REASON_VMPTRLD: i32 = 21;
pub const EXIT_REASON_VMPTRST: i32 = 22;
pub const EXIT_REASON_VMREAD: i32 = 23;
pub const EXIT_REASON_VMRESUME: i32 = 24;
pub const EXIT_REASON_VMWRITE: i32 = 25;
pub const EXIT_REASON_VMOFF: i32 = 26;
pub const EXIT_REASON_VMON: i32 = 27;
pub const EXIT_REASON_CR_ACCESS: i32 = 28;
pub const EXIT_REASON_DR_ACCESS: i32 = 29;
pub const EXIT_REASON_IO_INSTRUCTION: i32 = 30;
pub const EXIT_REASON_MSR_READ: i32 = 31;
pub const EXIT_REASON_MSR_WRITE: i32 = 32;
pub const EXIT_REASON_INVALID_STATE: i32 = 33;
pub const EXIT_REASON_MSR_LOAD_FAIL: i32 = 34;
pub const EXIT_REASON_MWAIT_INSTRUCTION: i32 = 36;
pub const EXIT_REASON_MONITOR_TRAP_FLAG: i32 = 37;
pub const EXIT_REASON_MONITOR_INSTRUCTION: i32 = 39;
pub const EXIT_REASON_PAUSE_INSTRUCTION: i32 = 40;
pub const EXIT_REASON_MCE_DURING_VMENTRY: i32 = 41;
pub const EXIT_REASON_TPR_BELOW_THRESHOLD: i32 = 43;
pub const EXIT_REASON_APIC_ACCESS: i32 = 44;
pub const EXIT_REASON_EOI_INDUCED: i32 = 45;
pub const EXIT_REASON_GDTR_IDTR: i32 = 46;
pub const EXIT_REASON_LDTR_TR: i32 = 47;
pub const EXIT_REASON_EPT_VIOLATION: i32 = 48;
pub const EXIT_REASON_EPT_MISCONFIG: i32 = 49;
pub const EXIT_REASON_INVEPT: i32 = 50;
pub const EXIT_REASON_RDTSCP: i32 = 51;
pub const EXIT_REASON_PREEMPTION_TIMER: i32 = 52;
pub const EXIT_REASON_INVVPID: i32 = 53;
pub const EXIT_REASON_WBINVD: i32 = 54;
pub const EXIT_REASON_XSETBV: i32 = 55;
pub const EXIT_REASON_APIC_WRITE: i32 = 56;
pub const EXIT_REASON_RDRAND: i32 = 57;
pub const EXIT_REASON_INVPCID: i32 = 58;
pub const EXIT_REASON_VMFUNC: i32 = 59;
pub const EXIT_REASON_ENCLS: i32 = 60;
pub const EXIT_REASON_RDSEED: i32 = 61;
pub const EXIT_REASON_PML_FULL: i32 = 62;
pub const EXIT_REASON_XSAVES: i32 = 63;
pub const EXIT_REASON_XRSTORS: i32 = 64;
pub const EXIT_REASON_UMWAIT: i32 = 67;
pub const EXIT_REASON_TPAUSE: i32 = 68;
pub const EXIT_REASON_BUS_LOCK: i32 = 74;
pub const EXIT_REASON_NOTIFY: i32 = 75;
pub const EXIT_REASON_SEAMCALL: i32 = 76;
pub const EXIT_REASON_TDCALL: i32 = 77;
pub const EXIT_REASON_MSR_READ_IMM: i32 = 84;
pub const EXIT_REASON_MSR_WRITE_IMM: i32 = 85;

/// Expands the upstream `VMX_EXIT_REASONS` initializer fragment through the
/// supplied consumer macro.  The consumer selects the concrete table type and
/// materializes the static C-string pointers, exactly as each C call site does.
#[macro_export]
macro_rules! VMX_EXIT_REASONS {
    ($consumer:ident) => {
        $consumer! {
            (EXIT_REASON_EXCEPTION_NMI as ::core::ffi::c_ulong, c"EXCEPTION_NMI".as_ptr()),
            (EXIT_REASON_EXTERNAL_INTERRUPT as ::core::ffi::c_ulong, c"EXTERNAL_INTERRUPT".as_ptr()),
            (EXIT_REASON_TRIPLE_FAULT as ::core::ffi::c_ulong, c"TRIPLE_FAULT".as_ptr()),
            (EXIT_REASON_INIT_SIGNAL as ::core::ffi::c_ulong, c"INIT_SIGNAL".as_ptr()),
            (EXIT_REASON_SIPI_SIGNAL as ::core::ffi::c_ulong, c"SIPI_SIGNAL".as_ptr()),
            (EXIT_REASON_INTERRUPT_WINDOW as ::core::ffi::c_ulong, c"INTERRUPT_WINDOW".as_ptr()),
            (EXIT_REASON_NMI_WINDOW as ::core::ffi::c_ulong, c"NMI_WINDOW".as_ptr()),
            (EXIT_REASON_TASK_SWITCH as ::core::ffi::c_ulong, c"TASK_SWITCH".as_ptr()),
            (EXIT_REASON_CPUID as ::core::ffi::c_ulong, c"CPUID".as_ptr()),
            (EXIT_REASON_HLT as ::core::ffi::c_ulong, c"HLT".as_ptr()),
            (EXIT_REASON_INVD as ::core::ffi::c_ulong, c"INVD".as_ptr()),
            (EXIT_REASON_INVLPG as ::core::ffi::c_ulong, c"INVLPG".as_ptr()),
            (EXIT_REASON_RDPMC as ::core::ffi::c_ulong, c"RDPMC".as_ptr()),
            (EXIT_REASON_RDTSC as ::core::ffi::c_ulong, c"RDTSC".as_ptr()),
            (EXIT_REASON_VMCALL as ::core::ffi::c_ulong, c"VMCALL".as_ptr()),
            (EXIT_REASON_VMCLEAR as ::core::ffi::c_ulong, c"VMCLEAR".as_ptr()),
            (EXIT_REASON_VMLAUNCH as ::core::ffi::c_ulong, c"VMLAUNCH".as_ptr()),
            (EXIT_REASON_VMPTRLD as ::core::ffi::c_ulong, c"VMPTRLD".as_ptr()),
            (EXIT_REASON_VMPTRST as ::core::ffi::c_ulong, c"VMPTRST".as_ptr()),
            (EXIT_REASON_VMREAD as ::core::ffi::c_ulong, c"VMREAD".as_ptr()),
            (EXIT_REASON_VMRESUME as ::core::ffi::c_ulong, c"VMRESUME".as_ptr()),
            (EXIT_REASON_VMWRITE as ::core::ffi::c_ulong, c"VMWRITE".as_ptr()),
            (EXIT_REASON_VMOFF as ::core::ffi::c_ulong, c"VMOFF".as_ptr()),
            (EXIT_REASON_VMON as ::core::ffi::c_ulong, c"VMON".as_ptr()),
            (EXIT_REASON_CR_ACCESS as ::core::ffi::c_ulong, c"CR_ACCESS".as_ptr()),
            (EXIT_REASON_DR_ACCESS as ::core::ffi::c_ulong, c"DR_ACCESS".as_ptr()),
            (EXIT_REASON_IO_INSTRUCTION as ::core::ffi::c_ulong, c"IO_INSTRUCTION".as_ptr()),
            (EXIT_REASON_MSR_READ as ::core::ffi::c_ulong, c"MSR_READ".as_ptr()),
            (EXIT_REASON_MSR_WRITE as ::core::ffi::c_ulong, c"MSR_WRITE".as_ptr()),
            (EXIT_REASON_INVALID_STATE as ::core::ffi::c_ulong, c"INVALID_STATE".as_ptr()),
            (EXIT_REASON_MSR_LOAD_FAIL as ::core::ffi::c_ulong, c"MSR_LOAD_FAIL".as_ptr()),
            (EXIT_REASON_MWAIT_INSTRUCTION as ::core::ffi::c_ulong, c"MWAIT_INSTRUCTION".as_ptr()),
            (EXIT_REASON_MONITOR_TRAP_FLAG as ::core::ffi::c_ulong, c"MONITOR_TRAP_FLAG".as_ptr()),
            (EXIT_REASON_MONITOR_INSTRUCTION as ::core::ffi::c_ulong, c"MONITOR_INSTRUCTION".as_ptr()),
            (EXIT_REASON_PAUSE_INSTRUCTION as ::core::ffi::c_ulong, c"PAUSE_INSTRUCTION".as_ptr()),
            (EXIT_REASON_MCE_DURING_VMENTRY as ::core::ffi::c_ulong, c"MCE_DURING_VMENTRY".as_ptr()),
            (EXIT_REASON_TPR_BELOW_THRESHOLD as ::core::ffi::c_ulong, c"TPR_BELOW_THRESHOLD".as_ptr()),
            (EXIT_REASON_APIC_ACCESS as ::core::ffi::c_ulong, c"APIC_ACCESS".as_ptr()),
            (EXIT_REASON_EOI_INDUCED as ::core::ffi::c_ulong, c"EOI_INDUCED".as_ptr()),
            (EXIT_REASON_GDTR_IDTR as ::core::ffi::c_ulong, c"GDTR_IDTR".as_ptr()),
            (EXIT_REASON_LDTR_TR as ::core::ffi::c_ulong, c"LDTR_TR".as_ptr()),
            (EXIT_REASON_EPT_VIOLATION as ::core::ffi::c_ulong, c"EPT_VIOLATION".as_ptr()),
            (EXIT_REASON_EPT_MISCONFIG as ::core::ffi::c_ulong, c"EPT_MISCONFIG".as_ptr()),
            (EXIT_REASON_INVEPT as ::core::ffi::c_ulong, c"INVEPT".as_ptr()),
            (EXIT_REASON_RDTSCP as ::core::ffi::c_ulong, c"RDTSCP".as_ptr()),
            (EXIT_REASON_PREEMPTION_TIMER as ::core::ffi::c_ulong, c"PREEMPTION_TIMER".as_ptr()),
            (EXIT_REASON_INVVPID as ::core::ffi::c_ulong, c"INVVPID".as_ptr()),
            (EXIT_REASON_WBINVD as ::core::ffi::c_ulong, c"WBINVD".as_ptr()),
            (EXIT_REASON_XSETBV as ::core::ffi::c_ulong, c"XSETBV".as_ptr()),
            (EXIT_REASON_APIC_WRITE as ::core::ffi::c_ulong, c"APIC_WRITE".as_ptr()),
            (EXIT_REASON_RDRAND as ::core::ffi::c_ulong, c"RDRAND".as_ptr()),
            (EXIT_REASON_INVPCID as ::core::ffi::c_ulong, c"INVPCID".as_ptr()),
            (EXIT_REASON_VMFUNC as ::core::ffi::c_ulong, c"VMFUNC".as_ptr()),
            (EXIT_REASON_ENCLS as ::core::ffi::c_ulong, c"ENCLS".as_ptr()),
            (EXIT_REASON_RDSEED as ::core::ffi::c_ulong, c"RDSEED".as_ptr()),
            (EXIT_REASON_PML_FULL as ::core::ffi::c_ulong, c"PML_FULL".as_ptr()),
            (EXIT_REASON_XSAVES as ::core::ffi::c_ulong, c"XSAVES".as_ptr()),
            (EXIT_REASON_XRSTORS as ::core::ffi::c_ulong, c"XRSTORS".as_ptr()),
            (EXIT_REASON_UMWAIT as ::core::ffi::c_ulong, c"UMWAIT".as_ptr()),
            (EXIT_REASON_TPAUSE as ::core::ffi::c_ulong, c"TPAUSE".as_ptr()),
            (EXIT_REASON_BUS_LOCK as ::core::ffi::c_ulong, c"BUS_LOCK".as_ptr()),
            (EXIT_REASON_NOTIFY as ::core::ffi::c_ulong, c"NOTIFY".as_ptr()),
            (EXIT_REASON_TDCALL as ::core::ffi::c_ulong, c"TDCALL".as_ptr()),
            (EXIT_REASON_MSR_READ_IMM as ::core::ffi::c_ulong, c"MSR_READ_IMM".as_ptr()),
            (EXIT_REASON_MSR_WRITE_IMM as ::core::ffi::c_ulong, c"MSR_WRITE_IMM".as_ptr()),
        }
    };
}

/// Expands the upstream `VMX_EXIT_REASON_FLAGS` initializer fragment through
/// the supplied consumer macro, leaving the caller to select its C-compatible
/// table layout and static storage.
#[macro_export]
macro_rules! VMX_EXIT_REASON_FLAGS {
    ($consumer:ident) => {
        $consumer! {
            (VMX_EXIT_REASONS_FAILED_VMENTRY as ::core::ffi::c_ulong, c"FAILED_VMENTRY".as_ptr()),
        }
    };
}

pub const VMX_ABORT_SAVE_GUEST_MSR_FAIL: i32 = 1;
pub const VMX_ABORT_LOAD_HOST_PDPTE_FAIL: i32 = 2;
pub const VMX_ABORT_LOAD_HOST_MSR_FAIL: i32 = 4;
