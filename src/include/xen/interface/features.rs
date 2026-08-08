// SPDX-License-Identifier: MIT
//! linux-source: include/xen/interface/features.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: aarch64
//! rewrite-task: S016567

//! Xen feature-bit indices returned by `XENVER_get_features`.

/*
 * Copyright (c) 2006, Keir Fraser <keir@xensource.com>
 */

/* These are bit indices, not masks; C numeric macro expressions are `int`. */

/// The guest can directly update writable page tables.
pub const XENFEAT_writable_page_tables: i32 = 0;
/// The guest can directly update segment descriptor tables.
pub const XENFEAT_writable_descriptor_tables: i32 = 1;
/// The hypervisor translates pseudo-physical and machine addresses.
pub const XENFEAT_auto_translated_physmap: i32 = 2;
/// The guest is running in supervisor mode.
pub const XENFEAT_supervisor_mode_kernel: i32 = 3;
/// x86 PAE page directories may be allocated above 4 GiB.
pub const XENFEAT_pae_pgdir_above_4gb: i32 = 4;
/// x86 supports the `MMU_PT_UPDATE_PRESERVE_AD` hypercall.
pub const XENFEAT_mmu_pt_update_preserve_ad: i32 = 5;
/// x86 supports `MMU_{CLEAR,COPY}_PAGE` hypercalls.
pub const XENFEAT_highmem_assist: i32 = 6;
/// `GNTTABOP_map_grant_ref` honors guest-kernel-available PTE bits.
pub const XENFEAT_gnttab_map_avail_bits: i32 = 7;
/// x86 supports the HVM callback-vector type.
pub const XENFEAT_hvm_callback_vector: i32 = 8;
/// The pvclock algorithm is safe on HVM.
pub const XENFEAT_hvm_safe_pvclock: i32 = 9;
/// x86 HVM guests may use PIRQs.
pub const XENFEAT_hvm_pirqs: i32 = 10;
/// Dom0 operation is supported.
pub const XENFEAT_dom0: i32 = 11;
/// `XENMEMF_vnode` may specify a virtual node for a memory operation.
pub const XENFEAT_memory_op_vnode_supported: i32 = 13;
/// The hypervisor supports the ARM SMC calling convention.
pub const XENFEAT_ARM_SMCCC_supported: i32 = 14;
/// An x86/PVH guest may place its ACPI RSDP at any address.
pub const XENFEAT_linux_rsdp_unrestricted: i32 = 15;
/// The domain is not direct-mapped.
pub const XENFEAT_not_direct_mapped: i32 = 16;
/// The domain is direct-mapped.
pub const XENFEAT_direct_mapped: i32 = 17;
/// Number of Xen feature submaps.
pub const XENFEAT_NR_SUBMAPS: i32 = 1;
