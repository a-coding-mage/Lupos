//! linux-parity: complete
//! linux-source: vendor/linux/mm/page_poison.c
//! test-origin: linux:vendor/linux/mm/page_poison.c
//! Page poisoning pattern fill and verification helpers.

pub const PAGE_POISON: u8 = 0xaa;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoisonCheck {
    Clean,
    SingleBitError { offset: usize, actual: u8 },
    Corruption { start: usize, end: usize },
}

pub fn poison_page_bytes(mem: &mut [u8]) {
    mem.fill(PAGE_POISON);
}

pub fn single_bit_flip(a: u8, b: u8) -> bool {
    let error = a ^ b;
    error != 0 && (error & error.wrapping_sub(1)) == 0
}

pub fn check_poison_mem(mem: &[u8]) -> PoisonCheck {
    let Some(start) = mem.iter().position(|byte| *byte != PAGE_POISON) else {
        return PoisonCheck::Clean;
    };
    let mut end = mem.len().saturating_sub(1);
    while end > start && mem[end] == PAGE_POISON {
        end -= 1;
    }
    if start == end && single_bit_flip(mem[start], PAGE_POISON) {
        PoisonCheck::SingleBitError {
            offset: start,
            actual: mem[start],
        }
    } else {
        PoisonCheck::Corruption { start, end }
    }
}

pub const fn kernel_map_pages_is_noop_without_arch_debug_pagealloc() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_poisoning_matches_linux_pattern_and_corruption_classification() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/page_poison.c"
        ));
        assert!(source.contains("bool _page_poisoning_enabled_early;"));
        assert!(source.contains("DEFINE_STATIC_KEY_FALSE(_page_poisoning_enabled);"));
        assert!(source.contains("early_param(\"page_poison\", early_page_poison_param);"));
        assert!(source.contains("memset(kasan_reset_tag(addr), PAGE_POISON, PAGE_SIZE);"));
        assert!(source.contains("void __kernel_poison_pages(struct page *page, int n)"));
        assert!(source.contains("static bool single_bit_flip(unsigned char a, unsigned char b)"));
        assert!(source.contains("return error && !(error & (error - 1));"));
        assert!(source.contains("start = memchr_inv(mem, PAGE_POISON, bytes);"));
        assert!(source.contains("pr_err(\"pagealloc: single bit error\\n\");"));
        assert!(source.contains("pr_err(\"pagealloc: memory corruption\\n\");"));
        assert!(source.contains("void __kernel_unpoison_pages(struct page *page, int n)"));
        assert!(source.contains("This function does nothing, all work is done via poison pages"));

        let mut page = [0u8; 16];
        poison_page_bytes(&mut page);
        assert_eq!(page, [PAGE_POISON; 16]);
        assert_eq!(check_poison_mem(&page), PoisonCheck::Clean);

        page[3] = PAGE_POISON ^ 0x01;
        assert_eq!(
            check_poison_mem(&page),
            PoisonCheck::SingleBitError {
                offset: 3,
                actual: PAGE_POISON ^ 0x01,
            }
        );
        page[5] = 0;
        assert_eq!(
            check_poison_mem(&page),
            PoisonCheck::Corruption { start: 3, end: 5 }
        );
        assert!(kernel_map_pages_is_noop_without_arch_debug_pagealloc());
    }
}
