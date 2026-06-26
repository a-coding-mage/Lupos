//! linux-parity: complete
//! linux-source: vendor/linux/tools/testing/selftests/mm
//! test-origin: linux:vendor/linux/tools/testing/selftests/mm
//!
//! KUnit-compatible ports of Linux mm kselftests.

use crate::kernel::kunit::KunitCase;
use crate::mm::mm_types::MmStruct;

pub mod madv_populate;
pub mod map_fixed_noreplace;
pub mod mremap_dontunmap;

pub const CASE_GROUPS: &[&[KunitCase]] = &[
    map_fixed_noreplace::CASES,
    mremap_dontunmap::CASES,
    madv_populate::CASES,
];

#[cfg(test)]
pub fn with_global_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = crate::mm::test_lock::GLOBAL_HW_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f()
}

#[cfg(all(
    not(test),
    any(
        feature = "test-kunit",
        feature = "test-mm-kselftests",
        feature = "test-entry-kselftests",
        feature = "test-futex-kselftests",
        feature = "test-rcu-kselftests",
        feature = "test-fs-kselftests",
        feature = "test-ipc-kselftests",
        feature = "test-cgroup-kselftests",
        feature = "test-net-kselftests",
        feature = "test-drivers-kselftests",
        feature = "test-security-kselftests",
        feature = "test-block-kselftests",
        feature = "test-userspace-kselftests",
    )
))]
pub fn with_global_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = crate::mm::test_lock::GLOBAL_HW_TEST_LOCK.lock();
    f()
}

#[cfg(test)]
pub fn make_mm() -> MmStruct {
    MmStruct::new(0)
}

#[cfg(all(
    not(test),
    any(
        feature = "test-kunit",
        feature = "test-mm-kselftests",
        feature = "test-entry-kselftests",
        feature = "test-futex-kselftests",
        feature = "test-rcu-kselftests",
        feature = "test-fs-kselftests",
        feature = "test-ipc-kselftests",
        feature = "test-cgroup-kselftests",
        feature = "test-net-kselftests",
        feature = "test-drivers-kselftests",
        feature = "test-security-kselftests",
        feature = "test-block-kselftests",
        feature = "test-userspace-kselftests",
    )
))]
pub fn make_mm() -> MmStruct {
    use crate::arch::x86::mm::paging::phys_to_virt;
    use crate::mm::buddy::{page_to_pfn, with_global_buddy};
    use crate::mm::frame::PAGE_SIZE;
    use crate::mm::page_flags::GFP_KERNEL;

    let pgd_page = with_global_buddy(|b| b.alloc_pages(0, GFP_KERNEL))
        .expect("kunit: failed to allocate page table root");
    let pgd_pfn = unsafe { page_to_pfn(pgd_page) } as u64;
    let pgd_virt = unsafe { phys_to_virt(pgd_pfn << 12) as *mut u64 };
    unsafe {
        core::ptr::write_bytes(pgd_virt as *mut u8, 0, PAGE_SIZE);
    }
    MmStruct::new(pgd_virt as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mm_kunit_cases_pass() {
        for cases in CASE_GROUPS {
            for case in *cases {
                assert!((case.run)(), "{}.{}", case.suite, case.name);
            }
        }
    }
}
