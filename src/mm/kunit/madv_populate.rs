//! linux-parity: complete
//! linux-source: vendor/linux/tools/testing/selftests/mm/madv_populate.c

use crate::kernel::kunit::KunitCase;
use crate::mm::kunit::{make_mm, with_global_lock};
use crate::mm::madvise::{MADV_POPULATE_READ, MADV_POPULATE_WRITE, do_madvise};
use crate::mm::mmap::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, do_mmap};

pub const CASES: &[KunitCase] = &[
    KunitCase {
        domain: crate::kernel::kunit::DOMAIN_MM,
        suite: "mm.madv_populate",
        name: "write_requires_write_vma",
        source: "vendor/linux/tools/testing/selftests/mm/madv_populate.c",
        run: madv_populate_write_requires_write_vma,
    },
    KunitCase {
        domain: crate::kernel::kunit::DOMAIN_MM,
        suite: "mm.madv_populate",
        name: "read_on_hole_returns_enomem",
        source: "vendor/linux/tools/testing/selftests/mm/madv_populate.c",
        run: madv_populate_read_on_hole_returns_enomem,
    },
];

fn madv_populate_write_requires_write_vma() -> bool {
    with_global_lock(|| {
        let mut mm = make_mm();
        let mapped = unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                0,
                0,
            )
        };
        crate::kunit_expect!(mapped == Ok(0x10000));

        let result = unsafe { do_madvise(&mut mm, 0x10000, 0x10000, MADV_POPULATE_WRITE) };
        crate::kunit_expect!(result == Err(-22));
        true
    })
}

fn madv_populate_read_on_hole_returns_enomem() -> bool {
    with_global_lock(|| {
        let mut mm = make_mm();
        let result = unsafe { do_madvise(&mut mm, 0x10000, 0x10000, MADV_POPULATE_READ) };
        crate::kunit_expect!(result == Err(-12));
        true
    })
}
