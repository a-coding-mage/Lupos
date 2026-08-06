// SPDX-License-Identifier: GPL-2.0-only
//! linux-source: arch/x86/include/asm/xen/trace_types.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000767

#[repr(C)]
#[allow(non_camel_case_types)]
pub enum xen_mc_flush_reason {
    XEN_MC_FL_NONE = 0,
    XEN_MC_FL_BATCH = 1,
    XEN_MC_FL_ARGS = 2,
    XEN_MC_FL_CALLBACK = 3,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub enum xen_mc_extend_args {
    XEN_MC_XE_OK = 0,
    XEN_MC_XE_BAD_OP = 1,
    XEN_MC_XE_NO_SPACE = 2,
}

#[allow(non_camel_case_types)]
pub type xen_mc_callback_fn_t = Option<unsafe extern "C" fn(*mut core::ffi::c_void)>;
