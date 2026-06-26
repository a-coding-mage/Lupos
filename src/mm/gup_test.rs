//! linux-parity: partial
//! linux-source: vendor/linux/mm/gup_test.c
//! test-origin: linux:vendor/linux/mm/gup_test.c
//! GUP test ioctl ABI constants and deterministic test planning.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const PAGE_SIZE: u64 = 4096;
pub const FOLL_WRITE: u32 = 0x01;
pub const FOLL_LONGTERM: u32 = 1 << 8;
pub const GUP_TEST_MAX_PAGES_TO_DUMP: usize = 8;
pub const GUP_TEST_FLAG_DUMP_PAGES_USE_PIN: u32 = 0x1;
pub const PIN_LONGTERM_TEST_FLAG_USE_WRITE: u32 = 1;
pub const PIN_LONGTERM_TEST_FLAG_USE_FAST: u32 = 2;

const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_NRSHIFT: u32 = 0;
const IOC_TYPESHIFT: u32 = IOC_NRSHIFT + IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_TYPESHIFT + IOC_TYPEBITS;
const IOC_DIRSHIFT: u32 = IOC_SIZESHIFT + IOC_SIZEBITS;
const IOC_NONE: u32 = 0;
const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;
const GUP_IOCTL_TYPE: u32 = b'g' as u32;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GupTest {
    pub get_delta_usec: u64,
    pub put_delta_usec: u64,
    pub addr: u64,
    pub size: u64,
    pub nr_pages_per_call: u32,
    pub gup_flags: u32,
    pub test_flags: u32,
    pub which_pages: [u32; GUP_TEST_MAX_PAGES_TO_DUMP],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PinLongtermTest {
    pub addr: u64,
    pub size: u64,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GupCommandKind {
    GetFastBenchmark,
    PinFastBenchmark,
    PinLongtermBenchmark,
    GetBasicTest,
    PinBasicTest,
    DumpUserPagesTest,
    PinLongtermStart,
    PinLongtermStop,
    PinLongtermRead,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PutBackMode {
    PutPage,
    Unpin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GupCall {
    pub addr: u64,
    pub nr_pages: u64,
    pub flags: u32,
}

pub const GUP_FAST_BENCHMARK: u32 = iowr(GUP_IOCTL_TYPE, 1, core::mem::size_of::<GupTest>());
pub const PIN_FAST_BENCHMARK: u32 = iowr(GUP_IOCTL_TYPE, 2, core::mem::size_of::<GupTest>());
pub const PIN_LONGTERM_BENCHMARK: u32 = iowr(GUP_IOCTL_TYPE, 3, core::mem::size_of::<GupTest>());
pub const GUP_BASIC_TEST: u32 = iowr(GUP_IOCTL_TYPE, 4, core::mem::size_of::<GupTest>());
pub const PIN_BASIC_TEST: u32 = iowr(GUP_IOCTL_TYPE, 5, core::mem::size_of::<GupTest>());
pub const DUMP_USER_PAGES_TEST: u32 = iowr(GUP_IOCTL_TYPE, 6, core::mem::size_of::<GupTest>());
pub const PIN_LONGTERM_TEST_START: u32 =
    iow(GUP_IOCTL_TYPE, 7, core::mem::size_of::<PinLongtermTest>());
pub const PIN_LONGTERM_TEST_STOP: u32 = io(GUP_IOCTL_TYPE, 8);
pub const PIN_LONGTERM_TEST_READ: u32 = iow(GUP_IOCTL_TYPE, 9, core::mem::size_of::<u64>());

pub const fn ioc(dir: u32, typ: u32, nr: u32, size: usize) -> u32 {
    (dir << IOC_DIRSHIFT)
        | (typ << IOC_TYPESHIFT)
        | (nr << IOC_NRSHIFT)
        | ((size as u32) << IOC_SIZESHIFT)
}

pub const fn io(typ: u32, nr: u32) -> u32 {
    ioc(IOC_NONE, typ, nr, 0)
}

pub const fn iow(typ: u32, nr: u32, size: usize) -> u32 {
    ioc(IOC_WRITE, typ, nr, size)
}

pub const fn iowr(typ: u32, nr: u32, size: usize) -> u32 {
    ioc(IOC_READ | IOC_WRITE, typ, nr, size)
}

pub const fn command_kind(cmd: u32) -> Result<GupCommandKind, i32> {
    match cmd {
        GUP_FAST_BENCHMARK => Ok(GupCommandKind::GetFastBenchmark),
        PIN_FAST_BENCHMARK => Ok(GupCommandKind::PinFastBenchmark),
        PIN_LONGTERM_BENCHMARK => Ok(GupCommandKind::PinLongtermBenchmark),
        GUP_BASIC_TEST => Ok(GupCommandKind::GetBasicTest),
        PIN_BASIC_TEST => Ok(GupCommandKind::PinBasicTest),
        DUMP_USER_PAGES_TEST => Ok(GupCommandKind::DumpUserPagesTest),
        PIN_LONGTERM_TEST_START => Ok(GupCommandKind::PinLongtermStart),
        PIN_LONGTERM_TEST_STOP => Ok(GupCommandKind::PinLongtermStop),
        PIN_LONGTERM_TEST_READ => Ok(GupCommandKind::PinLongtermRead),
        _ => Err(-EINVAL),
    }
}

pub const fn cmd_to_str(cmd: u32) -> &'static str {
    match cmd {
        GUP_FAST_BENCHMARK => "GUP_FAST_BENCHMARK",
        PIN_FAST_BENCHMARK => "PIN_FAST_BENCHMARK",
        PIN_LONGTERM_BENCHMARK => "PIN_LONGTERM_BENCHMARK",
        GUP_BASIC_TEST => "GUP_BASIC_TEST",
        PIN_BASIC_TEST => "PIN_BASIC_TEST",
        DUMP_USER_PAGES_TEST => "DUMP_USER_PAGES_TEST",
        _ => "Unknown command",
    }
}

pub const fn put_back_mode(cmd: u32, gup_test_flags: u32) -> Result<PutBackMode, i32> {
    match cmd {
        GUP_FAST_BENCHMARK | GUP_BASIC_TEST => Ok(PutBackMode::PutPage),
        PIN_FAST_BENCHMARK | PIN_BASIC_TEST | PIN_LONGTERM_BENCHMARK => Ok(PutBackMode::Unpin),
        DUMP_USER_PAGES_TEST => {
            if gup_test_flags & GUP_TEST_FLAG_DUMP_PAGES_USE_PIN != 0 {
                Ok(PutBackMode::Unpin)
            } else {
                Ok(PutBackMode::PutPage)
            }
        }
        _ => Err(-EINVAL),
    }
}

pub fn dump_pages_test(gup: &mut GupTest, nr_pages: u64) -> Vec<usize> {
    let mut indexes = Vec::new();

    for page in &mut gup.which_pages {
        if *page as u64 > nr_pages {
            *page = 0;
        }
    }

    for page in gup.which_pages {
        if page != 0 {
            indexes.push(page as usize - 1);
        }
    }

    indexes
}

pub fn gup_call_plan(cmd: u32, gup: &GupTest) -> Result<Vec<GupCall>, i32> {
    command_kind(cmd)?;
    if gup.nr_pages_per_call == 0 || gup.size % PAGE_SIZE != 0 {
        return Err(-EINVAL);
    }

    let mut calls = Vec::new();
    let mut nr = gup.nr_pages_per_call as u64;
    let mut addr = gup.addr;
    let end = gup.addr.checked_add(gup.size).ok_or(-EINVAL)?;

    while addr < end {
        if nr != gup.nr_pages_per_call as u64 {
            break;
        }

        let mut next = addr
            .checked_add(nr.checked_mul(PAGE_SIZE).ok_or(-EINVAL)?)
            .ok_or(-EINVAL)?;
        if next > end {
            next = end;
            nr = (next - addr) / PAGE_SIZE;
        }

        let flags = if cmd == PIN_LONGTERM_BENCHMARK {
            gup.gup_flags | FOLL_LONGTERM
        } else {
            gup.gup_flags
        };
        calls.push(GupCall {
            addr,
            nr_pages: nr,
            flags,
        });
        addr = next;
    }

    Ok(calls)
}

pub const fn pin_longterm_start_valid(args: PinLongtermTest) -> bool {
    if args.flags & !(PIN_LONGTERM_TEST_FLAG_USE_WRITE | PIN_LONGTERM_TEST_FLAG_USE_FAST) != 0 {
        return false;
    }
    if (args.addr | args.size) & (PAGE_SIZE - 1) != 0 {
        return false;
    }
    args.size != 0 && args.size <= i64::MAX as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gup_test_ioctl_abi_matches_kernel_header_and_selftest() {
        let kernel = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/gup_test.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/gup_test.h"
        ));
        let selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/mm/gup_test.c"
        ));
        let longterm = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/mm/gup_longterm.c"
        ));
        let run_vmtests = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/mm/run_vmtests.sh"
        ));
        let ioctl = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/uapi/asm-generic/ioctl.h"
        ));

        assert!(header.contains("#define GUP_FAST_BENCHMARK\t_IOWR('g', 1, struct gup_test)"));
        assert!(
            header.contains(
                "#define PIN_LONGTERM_TEST_START\t_IOW('g', 7, struct pin_longterm_test)"
            )
        );
        assert!(header.contains("#define GUP_TEST_MAX_PAGES_TO_DUMP\t\t8"));
        assert!(kernel.contains("case GUP_FAST_BENCHMARK:"));
        assert!(kernel.contains("case PIN_LONGTERM_BENCHMARK:"));
        assert!(kernel.contains("gup->gup_flags | FOLL_LONGTERM"));
        assert!(kernel.contains("debugfs_create_file_unsafe(\"gup_test\", 0600"));
        assert!(selftest.contains("#define GUP_TEST_FILE \"/sys/kernel/debug/gup_test\""));
        assert!(selftest.contains("cmd = DUMP_USER_PAGES_TEST"));
        assert!(longterm.contains("PIN_LONGTERM_TEST_START"));
        assert!(
            run_vmtests
                .contains("CATEGORY=\"gup_test\" run_test ./gup_test -ct -F 0x1 0 19 0x1000")
        );
        assert!(ioctl.contains("#define _IOWR(type,nr,argtype)"));

        assert_eq!(GUP_TEST_MAX_PAGES_TO_DUMP, 8);
        assert_eq!(cmd_to_str(PIN_BASIC_TEST), "PIN_BASIC_TEST");
        assert_eq!(command_kind(0), Err(-EINVAL));
        assert_eq!(
            put_back_mode(DUMP_USER_PAGES_TEST, GUP_TEST_FLAG_DUMP_PAGES_USE_PIN),
            Ok(PutBackMode::Unpin)
        );
    }

    #[test]
    fn gup_test_plans_calls_and_dump_page_indexes() {
        let mut gup = GupTest {
            addr: 0x1000,
            size: PAGE_SIZE * 3,
            nr_pages_per_call: 2,
            gup_flags: FOLL_WRITE,
            which_pages: [1, 3, 4, 0, 0, 0, 0, 0],
            ..GupTest::default()
        };
        let calls = gup_call_plan(PIN_LONGTERM_BENCHMARK, &gup).unwrap();
        assert_eq!(
            calls,
            [
                GupCall {
                    addr: 0x1000,
                    nr_pages: 2,
                    flags: FOLL_WRITE | FOLL_LONGTERM,
                },
                GupCall {
                    addr: 0x3000,
                    nr_pages: 1,
                    flags: FOLL_WRITE | FOLL_LONGTERM,
                },
            ]
        );

        let indexes = dump_pages_test(&mut gup, 3);
        assert_eq!(indexes, [0, 2]);
        assert_eq!(gup.which_pages[2], 0);
        assert!(pin_longterm_start_valid(PinLongtermTest {
            addr: 0x2000,
            size: PAGE_SIZE,
            flags: PIN_LONGTERM_TEST_FLAG_USE_FAST | PIN_LONGTERM_TEST_FLAG_USE_WRITE,
        }));
        assert!(!pin_longterm_start_valid(PinLongtermTest {
            addr: 1,
            size: PAGE_SIZE,
            flags: 0,
        }));
    }
}
