//! linux-parity: complete
//! linux-source: vendor/linux/tools/testing/selftests/mm/mremap_dontunmap.c

use crate::kernel::kunit::KunitCase;
use crate::mm::kunit::{make_mm, with_global_lock};
use crate::mm::mmap::{MAP_ANONYMOUS, MAP_FIXED, MAP_PRIVATE, PROT_READ, PROT_WRITE, do_mmap};
use crate::mm::mremap::{MREMAP_DONTUNMAP, MREMAP_MAYMOVE, do_mremap};
use crate::mm::vma::find_vma;

pub const CASES: &[KunitCase] = &[KunitCase {
    domain: crate::kernel::kunit::DOMAIN_MM,
    suite: "mm.mremap_dontunmap",
    name: "leaves_source_vma",
    source: "vendor/linux/tools/testing/selftests/mm/mremap_dontunmap.c",
    run: mremap_dontunmap_leaves_source_vma,
}];

fn mremap_dontunmap_leaves_source_vma() -> bool {
    with_global_lock(|| {
        let mut mm = make_mm();
        let source = unsafe {
            do_mmap(
                &mut mm,
                0x10000,
                0x10000,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_FIXED,
                0,
                0,
            )
        };
        crate::kunit_expect!(source == Ok(0x10000));

        let dest = unsafe {
            do_mremap(
                &mut mm,
                0x10000,
                0x10000,
                0x10000,
                MREMAP_MAYMOVE | MREMAP_DONTUNMAP,
                0,
            )
        };
        crate::kunit_expect!(dest.is_ok());
        let dest = dest.unwrap_or(0);

        crate::kunit_expect!(dest != 0x10000);
        crate::kunit_expect!(find_vma(&mm, 0x10000).is_some());
        crate::kunit_expect!(find_vma(&mm, dest).is_some());
        true
    })
}
