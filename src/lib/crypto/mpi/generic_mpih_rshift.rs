//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-rshift.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-rshift.c
//! MPI limb vector right shift.

use super::mpih_cmp::{MpiLimb, MpiSize};

pub fn mpihelp_rshift_slice(wp: &mut [MpiLimb], up: &[MpiLimb], cnt: u32) -> MpiLimb {
    assert!(wp.len() >= up.len());
    assert!(cnt > 0 && cnt < MpiLimb::BITS);

    let sh_1 = cnt;
    let sh_2 = MpiLimb::BITS - sh_1;
    let mut low_limb = up[0];
    let retval = low_limb << sh_2;

    for index in 1..up.len() {
        let high_limb = up[index];
        wp[index - 1] = (low_limb >> sh_1) | (high_limb << sh_2);
        low_limb = high_limb;
    }
    wp[up.len() - 1] = low_limb >> sh_1;

    retval
}

pub unsafe extern "C" fn mpihelp_rshift(
    wp: *mut MpiLimb,
    up: *const MpiLimb,
    usize: MpiSize,
    cnt: u32,
) -> MpiLimb {
    if usize <= 0 || wp.is_null() || up.is_null() || cnt == 0 || cnt >= MpiLimb::BITS {
        return 0;
    }

    let size = usize as usize;
    let sh_1 = cnt;
    let sh_2 = MpiLimb::BITS - sh_1;
    let mut low_limb = unsafe { *up };
    let retval = low_limb << sh_2;

    for index in 1..size {
        let high_limb = unsafe { *up.add(index) };
        unsafe {
            *wp.add(index - 1) = (low_limb >> sh_1) | (high_limb << sh_2);
        }
        low_limb = high_limb;
    }
    unsafe {
        *wp.add(size - 1) = low_limb >> sh_1;
    }

    retval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_rshift_matches_linux_forward_limb_walk() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-rshift.c"
        ));
        assert!(source.contains("mpihelp_rshift(mpi_ptr_t wp"));
        assert!(source.contains("wp -= 1;"));
        assert!(source.contains("retval = high_limb << sh_2;"));
        assert!(source.contains("for (i = 1; i < usize; i++)"));
        assert!(source.contains("wp[i] = (low_limb >> sh_1) | (high_limb << sh_2);"));
        assert!(source.contains("wp[i] = low_limb >> sh_1;"));

        let input = [0x0123_4567_89ab_cdefusize, 0xfedc_ba98_7654_3210];
        let mut output = [0usize; 2];
        let carry = mpihelp_rshift_slice(&mut output, &input, 4);
        assert_eq!(output, [0x0012_3456_789a_bcde, 0x0fed_cba9_8765_4321]);
        assert_eq!(carry, 0xf000_0000_0000_0000);
    }

    #[test]
    fn raw_mpihelp_rshift_uses_limb_pointers() {
        let input = [2usize, 4];
        let mut output = [0usize; 2];
        let carry = unsafe { mpihelp_rshift(output.as_mut_ptr(), input.as_ptr(), 2, 1) };
        assert_eq!(output, [1, 2]);
        assert_eq!(carry, 0);
    }

    #[test]
    fn raw_mpihelp_rshift_supports_in_place_buffers() {
        let mut limbs = [0x0123_4567_89ab_cdefusize, 0xfedc_ba98_7654_3210];
        let carry = unsafe {
            mpihelp_rshift(
                limbs.as_mut_ptr(),
                limbs.as_ptr(),
                limbs.len() as MpiSize,
                4,
            )
        };
        assert_eq!(limbs, [0x0012_3456_789a_bcde, 0x0fed_cba9_8765_4321]);
        assert_eq!(carry, 0xf000_0000_0000_0000);
    }
}
