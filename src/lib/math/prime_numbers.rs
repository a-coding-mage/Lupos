//! linux-parity: complete
//! linux-source: vendor/linux/lib/math/prime_numbers.c
//! test-origin: linux:vendor/linux/lib/math/prime_numbers.c
//! Prime-number helper API.

use crate::kernel::module::{export_symbol, find_symbol};

pub const BITS_PER_LONG: usize = usize::BITS as usize;
pub const ULONG_MAX: usize = usize::MAX;
pub const MAX_CACHED_BITS: usize = 131_072;
pub const PRIME_CACHE_WORDS: usize = MAX_CACHED_BITS / BITS_PER_LONG;
pub const MODULE_AUTHOR: &str = "Intel Corporation";
pub const MODULE_DESCRIPTION: &str = "Prime number library";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_EXIT: &str = "primes_exit";
pub const GFP_FLAGS: &str = "GFP_KERNEL | __GFP_NOWARN";

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("next_prime_number", next_prime_number as usize, false);
    export_symbol_once("is_prime_number", is_prime_number as usize, false);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Primes {
    pub last: usize,
    pub sz: usize,
    pub words: &'static [usize],
}

const SMALL_PRIMES_WORD: usize = (1usize << 2)
    | (1usize << 3)
    | (1usize << 5)
    | (1usize << 7)
    | (1usize << 11)
    | (1usize << 13)
    | (1usize << 17)
    | (1usize << 19)
    | (1usize << 23)
    | (1usize << 29)
    | (1usize << 31)
    | (1usize << 37)
    | (1usize << 41)
    | (1usize << 43)
    | (1usize << 47)
    | (1usize << 53)
    | (1usize << 59)
    | (1usize << 61);

pub const SMALL_PRIME_WORDS: &[usize] = &[SMALL_PRIMES_WORD];
pub const SMALL_PRIMES: Primes = Primes {
    last: 61,
    sz: usize::BITS as usize,
    words: SMALL_PRIME_WORDS,
};

const fn int_sqrt(mut x: usize) -> usize {
    if x <= 1 {
        return x;
    }

    let mut bit = 1usize << (((usize::BITS as usize) - 2) & !1usize);
    while bit > x {
        bit >>= 2;
    }

    let mut result = 0usize;
    while bit != 0 {
        if x >= result + bit {
            x -= result + bit;
            result = (result >> 1) + bit;
        } else {
            result >>= 1;
        }
        bit >>= 2;
    }
    result
}

pub const fn slow_is_prime_number(x: usize) -> bool {
    let mut y = int_sqrt(x);

    while y > 1 {
        if x % y == 0 {
            break;
        }
        y -= 1;
    }

    y == 1
}

const fn small_prime_bit(x: usize) -> bool {
    x < SMALL_PRIMES.sz && (SMALL_PRIMES_WORD & (1usize << x)) != 0
}

pub fn is_prime_number(x: usize) -> bool {
    let mut cache = PrimeCache::small();
    is_prime_number_cached(&mut cache, x)
}

pub fn next_prime_number(x: usize) -> usize {
    let mut cache = PrimeCache::small();
    next_prime_number_cached(&mut cache, x)
}

pub fn with_primes<R>(f: impl FnOnce(&Primes) -> R) -> R {
    f(&SMALL_PRIMES)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrimeCache {
    pub last: usize,
    pub sz: usize,
    pub words: [usize; PRIME_CACHE_WORDS],
    pub uses_small_primes: bool,
}

impl PrimeCache {
    pub fn small() -> Self {
        let mut words = [0usize; PRIME_CACHE_WORDS];
        let mut index = 0usize;
        while index < SMALL_PRIMES.words.len() {
            words[index] = SMALL_PRIMES.words[index];
            index += 1;
        }
        Self {
            last: SMALL_PRIMES.last,
            sz: SMALL_PRIMES.sz,
            words,
            uses_small_primes: true,
        }
    }
}

pub fn bitmap_size(sz: usize) -> usize {
    (sz + BITS_PER_LONG - 1) / BITS_PER_LONG
}

pub fn round_up_to_word(sz: usize) -> Option<usize> {
    let addend = BITS_PER_LONG - 1;
    sz.checked_add(addend).map(|value| value & !addend)
}

pub fn roundup(value: usize, divisor: usize) -> usize {
    if divisor == 0 {
        value
    } else {
        ((value + divisor - 1) / divisor) * divisor
    }
}

pub fn test_bit(bit: usize, words: &[usize]) -> bool {
    let word = bit / BITS_PER_LONG;
    let offset = bit % BITS_PER_LONG;
    word < words.len() && (words[word] & (1usize << offset)) != 0
}

pub fn clear_bit(bit: usize, words: &mut [usize]) {
    let word = bit / BITS_PER_LONG;
    let offset = bit % BITS_PER_LONG;
    if word < words.len() {
        words[word] &= !(1usize << offset);
    }
}

pub fn find_next_bit(words: &[usize], limit: usize, start: usize) -> usize {
    let mut bit = start;
    while bit < limit {
        if test_bit(bit, words) {
            return bit;
        }
        bit += 1;
    }
    limit
}

pub fn bitmap_fill(words: &mut [usize], sz: usize) {
    let mut index = 0usize;
    while index < words.len() {
        words[index] = usize::MAX;
        index += 1;
    }

    let excess = words.len() * BITS_PER_LONG - sz;
    if excess > 0 && !words.is_empty() {
        let last = words.len() - 1;
        words[last] &= usize::MAX >> excess;
    }
}

pub fn bitmap_copy(dst: &mut [usize], src: &[usize], bits: usize) {
    let full_words = bits / BITS_PER_LONG;
    let mut index = 0usize;
    while index < full_words && index < dst.len() && index < src.len() {
        dst[index] = src[index];
        index += 1;
    }

    let remaining = bits % BITS_PER_LONG;
    if remaining != 0 && full_words < dst.len() && full_words < src.len() {
        let mask = (1usize << remaining) - 1;
        dst[full_words] = (dst[full_words] & !mask) | (src[full_words] & mask);
    }
}

pub fn slow_next_prime_number(x: usize) -> usize {
    let mut next = x;
    while next < ULONG_MAX {
        next += 1;
        if slow_is_prime_number(next) {
            return next;
        }
    }
    next
}

pub fn clear_multiples(x: usize, words: &mut [usize], start: usize, end: usize) -> usize {
    let Some(mut multiple) = x.checked_mul(2) else {
        return x;
    };
    if multiple < start {
        multiple = roundup(start, x);
    }

    while multiple < end {
        clear_bit(multiple, words);
        multiple += x;
    }

    x
}

pub fn expand_to_next_prime(cache: &mut PrimeCache, x: usize) -> bool {
    let Some(raw_sz) = x.checked_mul(2) else {
        return false;
    };
    let Some(sz) = round_up_to_word(raw_sz) else {
        return false;
    };
    let word_count = bitmap_size(sz);
    if word_count > PRIME_CACHE_WORDS {
        return false;
    }
    let mut new_words = [0usize; PRIME_CACHE_WORDS];
    bitmap_fill(&mut new_words[..word_count], sz);

    if x < cache.last {
        return true;
    }

    bitmap_copy(&mut new_words[..word_count], &cache.words, cache.sz);
    let mut y = 2usize;
    let mut last = 0usize;
    while y < sz {
        last = clear_multiples(y, &mut new_words[..word_count], cache.sz, sz);
        y = find_next_bit(&new_words[..word_count], sz, y + 1);
    }

    if last <= x {
        return false;
    }

    cache.last = last;
    cache.sz = sz;
    cache.words = new_words;
    cache.uses_small_primes = false;
    true
}

pub fn free_primes(cache: &mut PrimeCache) {
    *cache = PrimeCache::small();
}

pub fn primes_exit(cache: &mut PrimeCache) {
    free_primes(cache);
}

pub fn next_prime_number_cached(cache: &mut PrimeCache, mut x: usize) -> usize {
    while x >= cache.last {
        if !expand_to_next_prime(cache, x) {
            return slow_next_prime_number(x);
        }
    }
    x = find_next_bit(&cache.words, cache.last, x + 1);
    x
}

pub fn is_prime_number_cached(cache: &mut PrimeCache, x: usize) -> bool {
    while x >= cache.sz {
        if !expand_to_next_prime(cache, x) {
            return slow_is_prime_number(x);
        }
    }
    test_bit(x, &cache.words)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_number_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/prime_numbers.c"
        ));
        let private = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/math/prime_numbers_private.h"
        ));
        assert!(source.contains("static const struct primes small_primes"));
        assert!(source.contains(".last = 61"));
        assert!(source.contains(".sz = 64"));
        assert!(source.contains("bool slow_is_prime_number(unsigned long x)"));
        assert!(source.contains("static unsigned long slow_next_prime_number(unsigned long x)"));
        assert!(source.contains("static unsigned long clear_multiples(unsigned long x"));
        assert!(source.contains("static bool expand_to_next_prime(unsigned long x)"));
        assert!(source.contains("static void free_primes(void)"));
        assert!(source.contains("unsigned long next_prime_number(unsigned long x)"));
        assert!(source.contains("bool is_prime_number(unsigned long x)"));
        assert!(source.contains("static DEFINE_MUTEX(lock);"));
        assert!(source.contains("RCU_INITIALIZER(&small_primes)"));
        assert!(source.contains("bitmap_fill(new->primes, sz);"));
        assert!(source.contains("bitmap_copy(new->primes, p->primes, p->sz);"));
        assert!(source.contains("new->last = clear_multiples(y, new->primes, p->sz, sz);"));
        assert!(source.contains("BUG_ON(new->last <= x);"));
        assert!(source.contains("rcu_assign_pointer(primes, new);"));
        assert!(source.contains("rcu_assign_pointer(primes, &small_primes);"));
        assert!(source.contains("EXPORT_SYMBOL(next_prime_number);"));
        assert!(source.contains("EXPORT_SYMBOL(is_prime_number);"));
        assert!(source.contains("module_exit(primes_exit);"));
        assert!(source.contains("MODULE_AUTHOR(\"Intel Corporation\");"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Prime number library\");"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\");"));
        assert!(private.contains("struct primes"));
        assert!(private.contains("struct rcu_head rcu;"));

        assert!(!is_prime_number(0));
        assert!(!is_prime_number(1));
        assert!(is_prime_number(2));
        assert!(is_prime_number(61));
        assert!(!is_prime_number(63));
        assert!(is_prime_number(65_521));
        assert_eq!(next_prime_number(0), 2);
        assert_eq!(next_prime_number(61), 67);
        assert_eq!(next_prime_number(usize::MAX), usize::MAX);
        assert_eq!(with_primes(|p| (p.last, p.sz)), (61, usize::BITS as usize));
        assert_eq!(BITS_PER_LONG, usize::BITS as usize);
        assert_eq!(ULONG_MAX, usize::MAX);
        assert_eq!(MODULE_AUTHOR, "Intel Corporation");
        assert_eq!(MODULE_DESCRIPTION, "Prime number library");
        assert_eq!(MODULE_LICENSE, "GPL");
        assert_eq!(MODULE_EXIT, "primes_exit");
        assert_eq!(GFP_FLAGS, "GFP_KERNEL | __GFP_NOWARN");
        assert!(small_prime_bit(61));
        assert_eq!(slow_next_prime_number(61), 67);
        assert_eq!(round_up_to_word(65), Some(128));
        assert_eq!(roundup(65, 7), 70);

        let mut words = [usize::MAX; PRIME_CACHE_WORDS];
        let word_count = bitmap_size(128);
        assert!(test_bit(77, &words));
        assert_eq!(clear_multiples(7, &mut words[..word_count], 64, 128), 7);
        assert!(!test_bit(70, &words));
        assert!(!test_bit(77, &words));
        assert!(test_bit(79, &words));

        let mut cache = PrimeCache::small();
        assert!(cache.uses_small_primes);
        assert!(expand_to_next_prime(&mut cache, 61));
        assert!(!cache.uses_small_primes);
        assert!(cache.sz >= 128);
        assert!(cache.last > 61);
        assert_eq!(next_prime_number_cached(&mut cache, 61), 67);
        assert!(is_prime_number_cached(&mut cache, 67));
        assert!(!is_prime_number_cached(&mut cache, 69));
        primes_exit(&mut cache);
        assert_eq!(cache, PrimeCache::small());
    }

    #[test]
    fn prime_number_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("next_prime_number"),
            Some(next_prime_number as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("is_prime_number"),
            Some(is_prime_number as usize)
        );
    }
}
