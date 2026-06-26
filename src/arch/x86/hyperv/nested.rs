//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/hyperv/nested.c
//! test-origin: linux:vendor/linux/arch/x86/hyperv/nested.c
//! Hyper-V nested guest mapping flush helpers.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOSPC;

pub const ENOTSUPP: i32 = 524;
pub const HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_SPACE: u16 = 0x00af;
pub const HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_LIST: u16 = 0x00b0;
pub const HV_MAX_FLUSH_PAGES: u64 = 2048;
pub const HV_MAX_FLUSH_REP_COUNT: usize = (4096 - 2 * core::mem::size_of::<u64>()) / 8;
pub const HV_HYPERCALL_RESULT_MASK: u64 = 0xffff;
pub const HV_STATUS_SUCCESS: i32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvGuestMappingFlush {
    pub address_space: u64,
    pub flags: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HvGpaPageRange {
    pub additional_pages: u64,
    pub largepage: bool,
    pub basepfn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HvGuestMappingFlushList {
    pub address_space: u64,
    pub flags: u64,
    pub gpa_list: Vec<HvGpaPageRange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypervFlushTrace {
    pub address_space: u64,
    pub ret: i32,
    pub hypercall_code: Option<u16>,
    pub reps: usize,
}

pub const fn hv_result(status: u64) -> i32 {
    (status & HV_HYPERCALL_RESULT_MASK) as i32
}

pub const fn hv_result_success(status: u64) -> bool {
    hv_result(status) == HV_STATUS_SUCCESS
}

pub fn hyperv_flush_guest_mapping(
    address_space: u64,
    hypercall_page_present: bool,
    pcpu_input_arg_present: bool,
    hypercall_status: u64,
) -> (i32, Option<HvGuestMappingFlush>, HypervFlushTrace) {
    let mut ret = -ENOTSUPP;
    let mut flush = None;
    let mut code = None;

    if hypercall_page_present && pcpu_input_arg_present {
        flush = Some(HvGuestMappingFlush {
            address_space,
            flags: 0,
        });
        code = Some(HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_SPACE);
        if hv_result_success(hypercall_status) {
            ret = 0;
        }
    }

    (
        ret,
        flush,
        HypervFlushTrace {
            address_space,
            ret,
            hypercall_code: code,
            reps: 0,
        },
    )
}

pub fn hyperv_fill_flush_guest_mapping_list(
    flush: &mut HvGuestMappingFlushList,
    start_gfn: u64,
    mut pages: u64,
) -> i32 {
    let mut cur = start_gfn;
    let mut gpa_n = 0i32;

    loop {
        if gpa_n as usize >= HV_MAX_FLUSH_REP_COUNT {
            return -ENOSPC;
        }

        let additional_pages = pages.min(HV_MAX_FLUSH_PAGES) - 1;
        flush.gpa_list.push(HvGpaPageRange {
            additional_pages,
            largepage: false,
            basepfn: cur,
        });
        pages -= additional_pages + 1;
        cur += additional_pages + 1;
        gpa_n += 1;

        if pages == 0 {
            return gpa_n;
        }
    }
}

pub fn hyperv_flush_guest_mapping_range<F>(
    address_space: u64,
    hypercall_page_present: bool,
    pcpu_input_arg_present: bool,
    hypercall_status: u64,
    fill_flush_list_func: Option<F>,
) -> (i32, Option<HvGuestMappingFlushList>, HypervFlushTrace)
where
    F: FnOnce(&mut HvGuestMappingFlushList) -> i32,
{
    let mut ret = -ENOTSUPP;
    let mut flush = None;
    let mut reps = 0usize;
    let mut code = None;

    if hypercall_page_present
        && pcpu_input_arg_present
        && let Some(fill) = fill_flush_list_func
    {
        let mut list = HvGuestMappingFlushList {
            address_space,
            flags: 0,
            gpa_list: Vec::new(),
        };
        let gpa_n = fill(&mut list);
        if gpa_n >= 0 {
            reps = gpa_n as usize;
            code = Some(HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_LIST);
            ret = if hv_result_success(hypercall_status) {
                0
            } else {
                hv_result(hypercall_status)
            };
            flush = Some(list);
        }
    }

    (
        ret,
        flush,
        HypervFlushTrace {
            address_space,
            ret,
            hypercall_code: code,
            reps,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_hyperv_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/hyperv/nested.c"
        ));
        let hvgdk = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/hyperv/hvgdk_mini.h"
        ));
        let mshyperv = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/asm-generic/mshyperv.h"
        ));
        assert!(source.contains("int hyperv_flush_guest_mapping(u64 as)"));
        assert!(source.contains("int ret = -ENOTSUPP;"));
        assert!(source.contains("if (!hv_hypercall_pg)"));
        assert!(source.contains("flush = *this_cpu_ptr(hyperv_pcpu_input_arg);"));
        assert!(source.contains("HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_SPACE"));
        assert!(source.contains("trace_hyperv_nested_flush_guest_mapping(as, ret);"));
        assert!(source.contains("hyperv_fill_flush_guest_mapping_list"));
        assert!(source.contains("if (gpa_n >= HV_MAX_FLUSH_REP_COUNT)"));
        assert!(source.contains("return -ENOSPC;"));
        assert!(source.contains("additional_pages = min_t(u64, pages, HV_MAX_FLUSH_PAGES) - 1;"));
        assert!(source.contains("hyperv_flush_guest_mapping_range"));
        assert!(source.contains("HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_LIST"));
        assert!(source.contains("ret = hv_result(status);"));
        assert!(hvgdk.contains("#define HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_SPACE\t0x00af"));
        assert!(hvgdk.contains("#define HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_LIST\t0x00b0"));
        assert!(hvgdk.contains("#define HV_MAX_FLUSH_PAGES (2048)"));
        assert!(mshyperv.contains("return status & HV_HYPERCALL_RESULT_MASK;"));

        assert_eq!(HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_SPACE, 0x00af);
        assert_eq!(HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_LIST, 0x00b0);
        assert_eq!(HV_MAX_FLUSH_REP_COUNT, 510);
    }

    #[test]
    fn single_address_space_flush_matches_fault_and_success_paths() {
        let (ret, flush, trace) = hyperv_flush_guest_mapping(7, false, true, 0);
        assert_eq!(ret, -ENOTSUPP);
        assert_eq!(flush, None);
        assert_eq!(trace.ret, -ENOTSUPP);
        assert_eq!(trace.hypercall_code, None);

        let (ret, flush, trace) = hyperv_flush_guest_mapping(9, true, true, 0);
        assert_eq!(ret, 0);
        assert_eq!(
            flush,
            Some(HvGuestMappingFlush {
                address_space: 9,
                flags: 0,
            })
        );
        assert_eq!(
            trace.hypercall_code,
            Some(HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_SPACE)
        );

        let (ret, _flush, _trace) = hyperv_flush_guest_mapping(9, true, true, 3);
        assert_eq!(ret, -ENOTSUPP);
    }

    #[test]
    fn fill_flush_guest_mapping_list_chunks_ranges_like_linux() {
        let mut flush = HvGuestMappingFlushList {
            address_space: 0,
            flags: 0,
            gpa_list: Vec::new(),
        };
        assert_eq!(
            hyperv_fill_flush_guest_mapping_list(&mut flush, 10, HV_MAX_FLUSH_PAGES + 5),
            2
        );
        assert_eq!(
            flush.gpa_list,
            alloc::vec![
                HvGpaPageRange {
                    additional_pages: HV_MAX_FLUSH_PAGES - 1,
                    largepage: false,
                    basepfn: 10,
                },
                HvGpaPageRange {
                    additional_pages: 4,
                    largepage: false,
                    basepfn: 10 + HV_MAX_FLUSH_PAGES,
                },
            ]
        );

        let mut too_many = HvGuestMappingFlushList {
            address_space: 0,
            flags: 0,
            gpa_list: Vec::new(),
        };
        assert_eq!(
            hyperv_fill_flush_guest_mapping_list(
                &mut too_many,
                0,
                (HV_MAX_FLUSH_REP_COUNT as u64 + 1) * HV_MAX_FLUSH_PAGES,
            ),
            -ENOSPC
        );
    }

    #[test]
    fn range_flush_uses_fill_callback_and_status_result() {
        let (ret, flush, trace) = hyperv_flush_guest_mapping_range(
            17,
            true,
            true,
            0,
            Some(|flush: &mut HvGuestMappingFlushList| {
                hyperv_fill_flush_guest_mapping_list(flush, 4, 2)
            }),
        );
        assert_eq!(ret, 0);
        let flush = flush.expect("flush list");
        assert_eq!(flush.address_space, 17);
        assert_eq!(flush.flags, 0);
        assert_eq!(flush.gpa_list[0].basepfn, 4);
        assert_eq!(trace.reps, 1);
        assert_eq!(
            trace.hypercall_code,
            Some(HVCALL_FLUSH_GUEST_PHYSICAL_ADDRESS_LIST)
        );

        let (ret, flush, trace) = hyperv_flush_guest_mapping_range(
            17,
            true,
            true,
            5,
            Some(|_flush: &mut HvGuestMappingFlushList| -ENOSPC),
        );
        assert_eq!(ret, -ENOTSUPP);
        assert_eq!(flush, None);
        assert_eq!(trace.hypercall_code, None);

        let (ret, _flush, _trace) = hyperv_flush_guest_mapping_range(
            17,
            true,
            true,
            5,
            Some(|flush: &mut HvGuestMappingFlushList| {
                hyperv_fill_flush_guest_mapping_list(flush, 4, 2)
            }),
        );
        assert_eq!(ret, 5);
    }
}
