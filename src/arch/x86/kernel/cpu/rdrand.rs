//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/kernel/cpu/rdrand.c
//! test-origin: linux:vendor/linux/arch/x86/kernel/cpu/rdrand.c
//! RDRAND startup sanity check.
//!
//! Port / mirror:
//! - vendor/linux/arch/x86/kernel/cpu/rdrand.c

// `rdrand.c` runs 8 RDRAND draws at boot and disables the feature if the
// sequence is constant (CPU returns the same value, which indicates a
// stuck hardware RNG). We model the sanity predicate over an arbitrary
// sample slice.

pub const RDRAND_SANITY_DRAW_COUNT: usize = 8;

pub fn rdrand_passes_sanity(samples: &[u64]) -> bool {
    if samples.len() < RDRAND_SANITY_DRAW_COUNT {
        return false;
    }
    let first = samples[0];
    samples
        .iter()
        .take(RDRAND_SANITY_DRAW_COUNT)
        .any(|s| *s != first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_stream_fails_sanity_check() {
        let stuck = [0xdead_beef_dead_beef_u64; 8];
        assert!(!rdrand_passes_sanity(&stuck));
    }

    #[test]
    fn varying_stream_passes_sanity_check() {
        let varied = [1u64, 2, 3, 4, 5, 6, 7, 8];
        assert!(rdrand_passes_sanity(&varied));
    }

    #[test]
    fn too_few_samples_fail_check() {
        let short = [1u64, 2, 3];
        assert!(!rdrand_passes_sanity(&short));
    }
}
