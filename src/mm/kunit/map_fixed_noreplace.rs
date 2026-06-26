//! linux-parity: complete
//! linux-source: vendor/linux/tools/testing/selftests/mm/map_fixed_noreplace.c

use crate::kernel::kunit::KunitCase;
use crate::mm::kunit::{make_mm, with_global_lock};
use crate::mm::mmap::{MAP_ANONYMOUS, MAP_FIXED_NOREPLACE, MAP_PRIVATE, PROT_READ, do_mmap};

pub const CASES: &[KunitCase] = &[
    KunitCase {
        domain: crate::kernel::kunit::DOMAIN_MM,
        suite: "mm.map_fixed_noreplace",
        name: "rejects_overlap",
        source: "vendor/linux/tools/testing/selftests/mm/map_fixed_noreplace.c",
        run: map_fixed_noreplace_rejects_overlap,
    },
    KunitCase {
        domain: crate::kernel::kunit::DOMAIN_MM,
        suite: "mm.map_fixed_noreplace",
        name: "allows_adjacent",
        source: "vendor/linux/tools/testing/selftests/mm/map_fixed_noreplace.c",
        run: map_fixed_noreplace_allows_adjacent,
    },
];

fn map_fixed_noreplace_rejects_overlap() -> bool {
    with_global_lock(|| {
        let mut mm = make_mm();
        let flags = MAP_PRIVATE | MAP_ANONYMOUS;
        let noreplace = flags | MAP_FIXED_NOREPLACE;

        let anchor = unsafe { do_mmap(&mut mm, 0x10000, 0x10000, PROT_READ, flags, 0, 0) };
        crate::kunit_expect!(anchor == Ok(0x10000));

        let exact = unsafe { do_mmap(&mut mm, 0x10000, 0x10000, PROT_READ, noreplace, 0, 0) };
        crate::kunit_expect!(exact == Err(-17));

        let overlap_start =
            unsafe { do_mmap(&mut mm, 0x0f000, 0x2000, PROT_READ, noreplace, 0, 0) };
        crate::kunit_expect!(overlap_start == Err(-17));

        let overlap_end = unsafe { do_mmap(&mut mm, 0x1f000, 0x2000, PROT_READ, noreplace, 0, 0) };
        crate::kunit_expect!(overlap_end == Err(-17));
        true
    })
}

fn map_fixed_noreplace_allows_adjacent() -> bool {
    with_global_lock(|| {
        let mut mm = make_mm();
        let flags = MAP_PRIVATE | MAP_ANONYMOUS;
        let noreplace = flags | MAP_FIXED_NOREPLACE;

        let anchor = unsafe { do_mmap(&mut mm, 0x10000, 0x10000, PROT_READ, flags, 0, 0) };
        crate::kunit_expect!(anchor == Ok(0x10000));

        let before = unsafe { do_mmap(&mut mm, 0x8000, 0x8000, PROT_READ, noreplace, 0, 0) };
        crate::kunit_expect!(before == Ok(0x8000));

        let after = unsafe { do_mmap(&mut mm, 0x20000, 0x10000, PROT_READ, noreplace, 0, 0) };
        crate::kunit_expect!(after == Ok(0x20000));
        true
    })
}
