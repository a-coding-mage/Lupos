//! linux-parity: complete
//! linux-source: vendor/linux/kernel/trace/undefsyms_base.c
//! test-origin: linux:vendor/linux/kernel/trace/undefsyms_base.c
//! Undefined-symbol exerciser for simple ring buffer builds.

use core::sync::atomic::{AtomicU32, Ordering};

use spin::Mutex;

use crate::arch::x86::mm::paging::PAGE_SIZE;

pub const PAGE_SIZE_BYTES: usize = PAGE_SIZE as usize;
pub const BUFFER_SIZE: usize = 256;
pub const FILL_BYTE: u8 = 8;
pub const CMPXCHG_EXPECTED: u32 = 0;
pub const CMPXCHG_NEW: u32 = 8;
pub const PAGE_BUFFER: &str = "static char page[PAGE_SIZE] __aligned(PAGE_SIZE);";
pub const WARN_SENTINEL: i32 = 0xdeadbeef_u32 as i32;

static PAGE: Mutex<[u8; PAGE_SIZE_BYTES]> = Mutex::new([0; PAGE_SIZE_BYTES]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UndefsymsBaseReport {
    pub page_len: usize,
    pub buffer_len: usize,
    pub copied_len: usize,
    pub cmpxchg_previous: u32,
    pub cmpxchg_final: u32,
    pub warned: bool,
}

pub const fn undefsyms_base_warns(n: i32) -> bool {
    n == WARN_SENTINEL
}

pub fn undefsyms_base(p: &mut [u8], n: i32) -> UndefsymsBaseReport {
    assert!(p.len() >= BUFFER_SIZE);

    let mut buffer = [0u8; BUFFER_SIZE];
    PAGE.lock().fill(FILL_BYTE);
    buffer.fill(FILL_BYTE);
    p[..BUFFER_SIZE].copy_from_slice(&buffer);

    let u = AtomicU32::new(CMPXCHG_EXPECTED);
    let cmpxchg_previous = match u.compare_exchange(
        CMPXCHG_EXPECTED,
        CMPXCHG_NEW,
        Ordering::SeqCst,
        Ordering::SeqCst,
    ) {
        Ok(previous) | Err(previous) => previous,
    };

    UndefsymsBaseReport {
        page_len: PAGE_SIZE_BYTES,
        buffer_len: BUFFER_SIZE,
        copied_len: BUFFER_SIZE,
        cmpxchg_previous,
        cmpxchg_final: u.load(Ordering::SeqCst),
        warned: undefsyms_base_warns(n),
    }
}

pub fn page_byte(index: usize) -> Option<u8> {
    PAGE.lock().get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undefsyms_base_source_contract_matches_linux() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/trace/undefsyms_base.c"
        ));
        assert!(source.contains("#include <linux/atomic.h>"));
        assert!(source.contains("#include <linux/string.h>"));
        assert!(source.contains("#include <asm/page.h>"));
        assert!(source.contains("char buffer[256] = { 0 };"));
        assert!(source.contains("u32 u = 0;"));
        assert!(source.contains(PAGE_BUFFER));
        assert!(source.contains("memset((char * volatile)page, 8, PAGE_SIZE);"));
        assert!(source.contains("memset((char * volatile)buffer, 8, sizeof(buffer));"));
        assert!(source.contains("memcpy((void * volatile)p, buffer, sizeof(buffer));"));
        assert!(source.contains("cmpxchg((u32 * volatile)&u, 0, 8);"));
        assert!(source.contains("WARN_ON(n == 0xdeadbeef);"));
        assert_eq!(BUFFER_SIZE, 256);
        assert_eq!(FILL_BYTE, 8);
        assert_eq!(CMPXCHG_EXPECTED, 0);
        assert_eq!(CMPXCHG_NEW, 8);
        assert!(undefsyms_base_warns(WARN_SENTINEL));
        assert!(!undefsyms_base_warns(0));
    }

    #[test]
    fn undefsyms_base_performs_linux_operations() {
        let mut destination = [0u8; BUFFER_SIZE + 8];
        let report = undefsyms_base(&mut destination, WARN_SENTINEL);

        assert_eq!(
            report,
            UndefsymsBaseReport {
                page_len: PAGE_SIZE_BYTES,
                buffer_len: BUFFER_SIZE,
                copied_len: BUFFER_SIZE,
                cmpxchg_previous: 0,
                cmpxchg_final: 8,
                warned: true,
            }
        );
        assert_eq!(page_byte(0), Some(FILL_BYTE));
        assert_eq!(page_byte(PAGE_SIZE_BYTES - 1), Some(FILL_BYTE));
        assert!(
            destination[..BUFFER_SIZE]
                .iter()
                .all(|byte| *byte == FILL_BYTE)
        );
        assert!(destination[BUFFER_SIZE..].iter().all(|byte| *byte == 0));
    }
}
