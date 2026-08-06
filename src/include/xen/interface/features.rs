// SPDX-License-Identifier: MIT
// Copyright (c) 2006, Keir Fraser <keir@xensource.com>
//! linux-source: include/xen/interface/features.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016567

//! Feature flags reported by `XENVER_get_features`.

/// If set, the guest does not need to write-protect its page tables and can
/// update them through direct writes.
pub const XENFEAT_writable_page_tables: i32 = 0;

/// If set, the guest does not need to write-protect its segment descriptor
/// tables and can update them through direct writes.
pub const XENFEAT_writable_descriptor_tables: i32 = 1;

/// If set, the hypervisor handles translation between the guest's
/// pseudo-physical address space and the host's machine address space. In
/// this mode the guest does not need to perform phys-to/from-machine
/// translations while performing page-table operations.
pub const XENFEAT_auto_translated_physmap: i32 = 2;

/// If set, the guest is running in supervisor mode (for example, x86 ring 0).
pub const XENFEAT_supervisor_mode_kernel: i32 = 3;

/// If set, the guest does not need to allocate x86 PAE page directories below
/// 4 GiB. This flag is usually implied by `XENFEAT_auto_translated_physmap`.
pub const XENFEAT_pae_pgdir_above_4gb: i32 = 4;

/// x86: Does this Xen host support the `MMU_PT_UPDATE_PRESERVE_AD` hypercall?
pub const XENFEAT_mmu_pt_update_preserve_ad: i32 = 5;

/// x86: Does this Xen host support the `MMU_{CLEAR,COPY}_PAGE` hypercall?
pub const XENFEAT_highmem_assist: i32 = 6;

/// If set, `GNTTABOP_map_grant_ref` honors flags to be placed in guest-kernel
/// available PTE bits.
pub const XENFEAT_gnttab_map_avail_bits: i32 = 7;

/// x86: Does this Xen host support the HVM callback vector type?
pub const XENFEAT_hvm_callback_vector: i32 = 8;

/// x86: Is the pvclock algorithm safe to use on HVM?
pub const XENFEAT_hvm_safe_pvclock: i32 = 9;

/// x86: Can HVM guests use pirqs?
pub const XENFEAT_hvm_pirqs: i32 = 10;

/// Is operation as Dom0 supported?
pub const XENFEAT_dom0: i32 = 11;

/* Xen also maps grant references at pfn = mfn. This feature flag is
 * deprecated and must not be used. The upstream apparent
 * XENFEAT_grant_map_identity definition is in a block comment, so it has no
 * Rust constant.
 */

/// A guest can use `XENMEMF_vnode` to specify a virtual node for a memory op.
pub const XENFEAT_memory_op_vnode_supported: i32 = 13;

/// arm: The hypervisor supports the ARM SMC calling convention.
pub const XENFEAT_ARM_SMCCC_supported: i32 = 14;

/// x86/PVH: If set, ACPI RSDP can be placed at any address. Otherwise, RSDP
/// must be located in the lower 1 MiB, as required by the ACPI Specification
/// for IA-PC systems. This feature is consulted only if
/// `XEN_ELFNOTE_GUEST_OS` contains the `"linux"` string.
pub const XENFEAT_linux_rsdp_unrestricted: i32 = 15;

/// A direct-mapped (or 1:1 mapped) domain has local pages with `gfn == mfn`.
/// A direct-mapped domain sets `XENFEAT_direct_mapped`; otherwise it sets
/// `XENFEAT_not_direct_mapped`.
///
/// If neither flag is set (for example, with older Xen releases), the
/// assumptions are:
/// - non-auto-translated domains (x86 only) are always direct-mapped;
/// - on x86, auto-translated domains are not direct-mapped;
/// - on ARM, Dom0 is direct-mapped and DomUs are not.
pub const XENFEAT_not_direct_mapped: i32 = 16;

pub const XENFEAT_direct_mapped: i32 = 17;

pub const XENFEAT_NR_SUBMAPS: i32 = 1;
