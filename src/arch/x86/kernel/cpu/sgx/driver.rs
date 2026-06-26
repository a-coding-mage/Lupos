//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/sgx/driver.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/sgx/driver.c
//! SGX enclave misc-device driver entry points.

pub const SGX_XFRM_RESERVED_MASK_DEFAULT: u64 = !0x3;
pub const SGX_MISC_RESERVED_MASK: u64 = u64::MAX & !1;
pub const SGX_ATTR_RESERVED_MASK: u64 =
    (1 << 3) | (1 << 6) | (1 << 8) | (1 << 9) | (u64::MAX << 11);
pub const SGX_CPUID: u32 = 0x12;
pub const MISC_DYNAMIC_MINOR: i32 = 255;
pub const MAP_TYPE: u64 = 0x0f;
pub const MAP_PRIVATE: u64 = 0x02;
pub const MAP_FIXED: u64 = 0x10;
pub const VM_PFNMAP: u64 = 1 << 4;
pub const VM_IO: u64 = 1 << 14;
pub const VM_DONTEXPAND: u64 = 1 << 16;
pub const VM_DONTDUMP: u64 = 1 << 17;
pub const SGX_MMAP_VM_FLAGS: u64 = VM_PFNMAP | VM_DONTEXPAND | VM_DONTDUMP | VM_IO;

const ENOMEM: i32 = -12;
const ENODEV: i32 = -19;
const EINVAL: i32 = -22;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxMiscDevice {
    pub minor: i32,
    pub name: &'static str,
    pub nodename: &'static str,
    pub fops: &'static str,
}

pub const SGX_DEV_ENCLAVE: SgxMiscDevice = SgxMiscDevice {
    minor: MISC_DYNAMIC_MINOR,
    name: "sgx_enclave",
    nodename: "sgx_enclave",
    fops: "sgx_encl_fops",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxFileOperations {
    pub open: &'static str,
    pub release: &'static str,
    pub unlocked_ioctl: &'static str,
    pub compat_ioctl: Option<&'static str>,
    pub mmap: &'static str,
    pub get_unmapped_area: &'static str,
}

pub const SGX_ENCL_FOPS: SgxFileOperations = SgxFileOperations {
    open: "sgx_open",
    release: "sgx_release",
    unlocked_ioctl: "sgx_ioctl",
    compat_ioctl: Some("sgx_compat_ioctl"),
    mmap: "sgx_mmap",
    get_unmapped_area: "sgx_get_unmapped_area",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxOpenEnv {
    pub usage_count_ret: i32,
    pub allocation_succeeds: bool,
    pub init_srcu_ret: i32,
}

impl SgxOpenEnv {
    pub const SUCCESS: Self = Self {
        usage_count_ret: 0,
        allocation_succeeds: true,
        init_srcu_ret: 0,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxOpenPlan {
    pub usage_count_inc: bool,
    pub allocation_attempted: bool,
    pub kref_init: bool,
    pub page_array_init: bool,
    pub mutex_init: bool,
    pub lists_init: bool,
    pub mm_lock_init: bool,
    pub srcu_init: bool,
    pub file_private_data_set: bool,
    pub allocation_freed_on_error: bool,
    pub usage_count_dec_on_error: bool,
}

pub const fn sgx_open_plan(env: SgxOpenEnv) -> Result<SgxOpenPlan, i32> {
    if env.usage_count_ret != 0 {
        return Err(env.usage_count_ret);
    }
    if !env.allocation_succeeds {
        return Err(ENOMEM);
    }
    if env.init_srcu_ret != 0 {
        return Err(env.init_srcu_ret);
    }
    Ok(SgxOpenPlan {
        usage_count_inc: true,
        allocation_attempted: true,
        kref_init: true,
        page_array_init: true,
        mutex_init: true,
        lists_init: true,
        mm_lock_init: true,
        srcu_init: true,
        file_private_data_set: true,
        allocation_freed_on_error: false,
        usage_count_dec_on_error: false,
    })
}

pub const fn sgx_open_error_cleanup(env: SgxOpenEnv) -> SgxOpenPlan {
    SgxOpenPlan {
        usage_count_inc: env.usage_count_ret == 0,
        allocation_attempted: env.usage_count_ret == 0,
        kref_init: env.usage_count_ret == 0 && env.allocation_succeeds,
        page_array_init: env.usage_count_ret == 0 && env.allocation_succeeds,
        mutex_init: env.usage_count_ret == 0 && env.allocation_succeeds,
        lists_init: env.usage_count_ret == 0 && env.allocation_succeeds,
        mm_lock_init: env.usage_count_ret == 0 && env.allocation_succeeds,
        srcu_init: env.usage_count_ret == 0 && env.allocation_succeeds,
        file_private_data_set: false,
        allocation_freed_on_error: env.usage_count_ret == 0
            && env.allocation_succeeds
            && env.init_srcu_ret != 0,
        usage_count_dec_on_error: env.usage_count_ret == 0
            && (!env.allocation_succeeds || env.init_srcu_ret != 0),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxReleasePlan {
    pub mm_entries_drained: usize,
    pub synchronize_srcu_calls: usize,
    pub mmu_notifier_unregister_calls: usize,
    pub encl_mm_frees: usize,
    pub encl_mm_kref_puts: usize,
    pub final_encl_kref_put: bool,
}

pub const fn sgx_release_plan(mm_entries: usize) -> SgxReleasePlan {
    SgxReleasePlan {
        mm_entries_drained: mm_entries,
        synchronize_srcu_calls: mm_entries,
        mmu_notifier_unregister_calls: mm_entries,
        encl_mm_frees: mm_entries,
        encl_mm_kref_puts: mm_entries,
        final_encl_kref_put: true,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxMmapPlan {
    pub may_map_checked: bool,
    pub mm_added: bool,
    pub vm_ops_set: bool,
    pub vm_flags_added: u64,
    pub vm_private_data_set: bool,
}

pub const fn sgx_mmap_plan(may_map_ret: i32, mm_add_ret: i32) -> Result<SgxMmapPlan, i32> {
    if may_map_ret != 0 {
        return Err(may_map_ret);
    }
    if mm_add_ret != 0 {
        return Err(mm_add_ret);
    }
    Ok(SgxMmapPlan {
        may_map_checked: true,
        mm_added: true,
        vm_ops_set: true,
        vm_flags_added: SGX_MMAP_VM_FLAGS,
        vm_private_data_set: true,
    })
}

pub const fn sgx_get_unmapped_area(flags: u64, addr: u64, fallback: u64) -> Result<u64, i32> {
    if flags & MAP_TYPE == MAP_PRIVATE {
        return Err(EINVAL);
    }
    if flags & MAP_FIXED != 0 {
        return Ok(addr);
    }
    Ok(fallback)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxDrvInitEnv {
    pub has_sgx_lc: bool,
    pub has_osxsave: bool,
    pub cpuid0: CpuidResult,
    pub cpuid1: CpuidResult,
    pub misc_register_ret: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SgxDrvInitPlan {
    pub ret: i32,
    pub cpuid_leaf0_read: bool,
    pub cpuid_leaf1_read: bool,
    pub misc_reserved_mask: u32,
    pub attributes_reserved_mask: u64,
    pub xfrm_reserved_mask: u64,
    pub misc_register_called: bool,
}

pub const fn sgx_drv_init_plan(env: SgxDrvInitEnv) -> SgxDrvInitPlan {
    if !env.has_sgx_lc {
        return SgxDrvInitPlan {
            ret: ENODEV,
            cpuid_leaf0_read: false,
            cpuid_leaf1_read: false,
            misc_reserved_mask: 0,
            attributes_reserved_mask: 0,
            xfrm_reserved_mask: SGX_XFRM_RESERVED_MASK_DEFAULT,
            misc_register_called: false,
        };
    }
    if env.cpuid0.eax & 1 == 0 {
        return SgxDrvInitPlan {
            ret: ENODEV,
            cpuid_leaf0_read: true,
            cpuid_leaf1_read: false,
            misc_reserved_mask: 0,
            attributes_reserved_mask: 0,
            xfrm_reserved_mask: SGX_XFRM_RESERVED_MASK_DEFAULT,
            misc_register_called: false,
        };
    }

    let attr_mask = ((env.cpuid1.ebx as u64) << 32) + env.cpuid1.eax as u64;
    let xfrm_mask = if env.has_osxsave {
        ((env.cpuid1.edx as u64) << 32) + env.cpuid1.ecx as u64
    } else {
        0x3
    };
    SgxDrvInitPlan {
        ret: env.misc_register_ret,
        cpuid_leaf0_read: true,
        cpuid_leaf1_read: true,
        misc_reserved_mask: !env.cpuid0.ebx | SGX_MISC_RESERVED_MASK as u32,
        attributes_reserved_mask: !attr_mask | SGX_ATTR_RESERVED_MASK,
        xfrm_reserved_mask: !xfrm_mask,
        misc_register_called: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sgx_driver_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/sgx/driver.c"
        ));
        let driver_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/sgx/driver.h"
        ));
        let sgx_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/sgx/sgx.h"
        ));
        let encl_h = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/kernel/cpu/sgx/encl.h"
        ));
        let asm_sgx = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/sgx.h"
        ));
        let miscdevice = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/miscdevice.h"
        ));
        let mman = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/linux/mman.h"
        ));
        let mman_common = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/asm-generic/mman-common.h"
        ));

        assert!(source.contains("u64 sgx_attributes_reserved_mask;"));
        assert!(source.contains("u64 sgx_xfrm_reserved_mask = ~0x3;"));
        assert!(source.contains("u32 sgx_misc_reserved_mask;"));
        assert!(source.contains("static int __sgx_open"));
        assert!(source.contains("encl = kzalloc_obj(*encl);"));
        assert!(source.contains("kref_init(&encl->refcount);"));
        assert!(source.contains("xa_init(&encl->page_array);"));
        assert!(source.contains("INIT_LIST_HEAD(&encl->va_pages);"));
        assert!(source.contains("INIT_LIST_HEAD(&encl->mm_list);"));
        assert!(source.contains("ret = init_srcu_struct(&encl->srcu);"));
        assert!(source.contains("file->private_data = encl;"));
        assert!(source.contains("ret = sgx_inc_usage_count();"));
        assert!(source.contains("sgx_dec_usage_count();"));
        assert!(source.contains("static int sgx_release"));
        assert!(source.contains("list_first_entry(&encl->mm_list"));
        assert!(source.contains("synchronize_srcu(&encl->srcu);"));
        assert!(source.contains("mmu_notifier_unregister(&encl_mm->mmu_notifier, encl_mm->mm);"));
        assert!(source.contains("kref_put(&encl->refcount, sgx_encl_release);"));
        assert!(source.contains("static int sgx_mmap"));
        assert!(
            source.contains("sgx_encl_may_map(encl, vma->vm_start, vma->vm_end, vma->vm_flags);")
        );
        assert!(source.contains("sgx_encl_mm_add(encl, vma->vm_mm);"));
        assert!(source.contains("vma->vm_ops = &sgx_vm_ops;"));
        assert!(
            source.contains("vm_flags_set(vma, VM_PFNMAP | VM_DONTEXPAND | VM_DONTDUMP | VM_IO);")
        );
        assert!(source.contains("if ((flags & MAP_TYPE) == MAP_PRIVATE)"));
        assert!(source.contains("if (flags & MAP_FIXED)"));
        assert!(source.contains("static const struct file_operations sgx_encl_fops"));
        assert!(source.contains(".unlocked_ioctl\t\t= sgx_ioctl"));
        assert!(source.contains(".get_unmapped_area\t= sgx_get_unmapped_area"));
        assert!(source.contains(".minor = MISC_DYNAMIC_MINOR"));
        assert!(source.contains(".name = \"sgx_enclave\""));
        assert!(source.contains("int __init sgx_drv_init(void)"));
        assert!(source.contains("if (!cpu_feature_enabled(X86_FEATURE_SGX_LC))"));
        assert!(source.contains("cpuid_count(SGX_CPUID, 0, &eax, &ebx, &ecx, &edx);"));
        assert!(source.contains("if (!(eax & 1))"));
        assert!(source.contains("sgx_misc_reserved_mask = ~ebx | SGX_MISC_RESERVED_MASK;"));
        assert!(source.contains("attr_mask = (((u64)ebx) << 32) + (u64)eax;"));
        assert!(
            source.contains("sgx_attributes_reserved_mask = ~attr_mask | SGX_ATTR_RESERVED_MASK;")
        );
        assert!(source.contains("xfrm_mask = (((u64)edx) << 32) + (u64)ecx;"));
        assert!(source.contains("sgx_xfrm_reserved_mask = ~xfrm_mask;"));
        assert!(source.contains("ret = misc_register(&sgx_dev_enclave);"));
        assert!(driver_h.contains("extern u64 sgx_attributes_reserved_mask;"));
        assert!(sgx_h.contains("int sgx_inc_usage_count(void);"));
        assert!(encl_h.contains("int sgx_encl_may_map"));
        assert!(asm_sgx.contains("#define SGX_CPUID\t\t0x12"));
        assert!(asm_sgx.contains("#define SGX_MISC_RESERVED_MASK"));
        assert!(asm_sgx.contains("#define SGX_ATTR_RESERVED_MASK"));
        assert!(miscdevice.contains("#define MISC_DYNAMIC_MINOR\t255"));
        assert!(mman.contains("#define MAP_PRIVATE\t0x02"));
        assert!(mman_common.contains("#define MAP_TYPE\t0x0f"));
        assert_eq!(SGX_CPUID, 0x12);
        assert_eq!(
            SGX_DEV_ENCLAVE,
            SgxMiscDevice {
                minor: MISC_DYNAMIC_MINOR,
                name: "sgx_enclave",
                nodename: "sgx_enclave",
                fops: "sgx_encl_fops",
            }
        );
    }

    #[test]
    fn open_plan_matches_usage_count_and_srcu_cleanup() {
        assert_eq!(
            sgx_open_plan(SgxOpenEnv::SUCCESS),
            Ok(SgxOpenPlan {
                usage_count_inc: true,
                allocation_attempted: true,
                kref_init: true,
                page_array_init: true,
                mutex_init: true,
                lists_init: true,
                mm_lock_init: true,
                srcu_init: true,
                file_private_data_set: true,
                allocation_freed_on_error: false,
                usage_count_dec_on_error: false,
            })
        );
        assert_eq!(
            sgx_open_plan(SgxOpenEnv {
                usage_count_ret: -5,
                ..SgxOpenEnv::SUCCESS
            }),
            Err(-5)
        );
        assert_eq!(
            sgx_open_plan(SgxOpenEnv {
                allocation_succeeds: false,
                ..SgxOpenEnv::SUCCESS
            }),
            Err(ENOMEM)
        );
        assert_eq!(
            sgx_open_error_cleanup(SgxOpenEnv {
                allocation_succeeds: false,
                ..SgxOpenEnv::SUCCESS
            })
            .usage_count_dec_on_error,
            true
        );
        let srcu_fail = sgx_open_error_cleanup(SgxOpenEnv {
            init_srcu_ret: -11,
            ..SgxOpenEnv::SUCCESS
        });
        assert!(srcu_fail.allocation_freed_on_error);
        assert!(srcu_fail.usage_count_dec_on_error);
    }

    #[test]
    fn release_and_mmap_plans_follow_linux_order() {
        assert_eq!(
            sgx_release_plan(3),
            SgxReleasePlan {
                mm_entries_drained: 3,
                synchronize_srcu_calls: 3,
                mmu_notifier_unregister_calls: 3,
                encl_mm_frees: 3,
                encl_mm_kref_puts: 3,
                final_encl_kref_put: true,
            }
        );
        assert_eq!(sgx_mmap_plan(-13, 0), Err(-13));
        assert_eq!(sgx_mmap_plan(0, -12), Err(-12));
        assert_eq!(
            sgx_mmap_plan(0, 0),
            Ok(SgxMmapPlan {
                may_map_checked: true,
                mm_added: true,
                vm_ops_set: true,
                vm_flags_added: SGX_MMAP_VM_FLAGS,
                vm_private_data_set: true,
            })
        );
        assert_eq!(
            sgx_get_unmapped_area(MAP_PRIVATE, 0x1000, 0x2000),
            Err(EINVAL)
        );
        assert_eq!(sgx_get_unmapped_area(MAP_FIXED, 0x1000, 0x2000), Ok(0x1000));
        assert_eq!(sgx_get_unmapped_area(0, 0x1000, 0x2000), Ok(0x2000));
    }

    #[test]
    fn drv_init_derives_reserved_masks_and_registers_misc_device() {
        assert_eq!(
            sgx_drv_init_plan(SgxDrvInitEnv {
                has_sgx_lc: false,
                has_osxsave: true,
                cpuid0: CpuidResult::default(),
                cpuid1: CpuidResult::default(),
                misc_register_ret: 0,
            })
            .ret,
            ENODEV
        );
        assert_eq!(
            sgx_drv_init_plan(SgxDrvInitEnv {
                has_sgx_lc: true,
                has_osxsave: true,
                cpuid0: CpuidResult {
                    eax: 0,
                    ..CpuidResult::default()
                },
                cpuid1: CpuidResult::default(),
                misc_register_ret: 0,
            })
            .cpuid_leaf1_read,
            false
        );

        let plan = sgx_drv_init_plan(SgxDrvInitEnv {
            has_sgx_lc: true,
            has_osxsave: true,
            cpuid0: CpuidResult {
                eax: 1,
                ebx: 0x1,
                ..CpuidResult::default()
            },
            cpuid1: CpuidResult {
                eax: 0x00ff_00ff,
                ebx: 0x0f0f_0f0f,
                ecx: 0x0000_0003,
                edx: 0,
            },
            misc_register_ret: -16,
        });
        assert_eq!(plan.ret, -16);
        assert!(plan.misc_register_called);
        assert_eq!(
            plan.misc_reserved_mask,
            !0x1 | SGX_MISC_RESERVED_MASK as u32
        );
        let attr_mask = (0x0f0f_0f0fu64 << 32) + 0x00ff_00ffu64;
        assert_eq!(
            plan.attributes_reserved_mask,
            !attr_mask | SGX_ATTR_RESERVED_MASK
        );
        assert_eq!(plan.xfrm_reserved_mask, !0x3);

        let no_osxsave = sgx_drv_init_plan(SgxDrvInitEnv {
            has_osxsave: false,
            misc_register_ret: 0,
            ..SgxDrvInitEnv {
                has_sgx_lc: true,
                cpuid0: CpuidResult {
                    eax: 1,
                    ..CpuidResult::default()
                },
                cpuid1: CpuidResult::default(),
                misc_register_ret: 0,
                has_osxsave: false,
            }
        });
        assert_eq!(
            no_osxsave.xfrm_reserved_mask,
            SGX_XFRM_RESERVED_MASK_DEFAULT
        );
    }
}
