//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-lshift.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-lshift.c
//! MPI limb vector left shift.

use super::mpih_cmp::{MpiLimb, MpiSize};

pub fn mpihelp_lshift_slice(wp: &mut [MpiLimb], up: &[MpiLimb], cnt: u32) -> MpiLimb {
    assert!(wp.len() >= up.len());
    assert!(cnt > 0 && cnt < MpiLimb::BITS);

    let sh_1 = cnt;
    let sh_2 = MpiLimb::BITS - sh_1;
    let mut high_limb = up[up.len() - 1];
    let retval = high_limb >> sh_2;

    for index in (0..up.len() - 1).rev() {
        let low_limb = up[index];
        wp[index + 1] = (high_limb << sh_1) | (low_limb >> sh_2);
        high_limb = low_limb;
    }
    wp[0] = high_limb << sh_1;

    retval
}

pub unsafe extern "C" fn mpihelp_lshift(
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
    let mut high_limb = unsafe { *up.add(size - 1) };
    let retval = high_limb >> sh_2;

    for index in (0..size - 1).rev() {
        let low_limb = unsafe { *up.add(index) };
        unsafe {
            *wp.add(index + 1) = (high_limb << sh_1) | (low_limb >> sh_2);
        }
        high_limb = low_limb;
    }
    unsafe {
        *wp = high_limb << sh_1;
    }

    retval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_lshift_matches_linux_reverse_limb_walk() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-lshift.c"
        ));
        assert!(source.contains("mpihelp_lshift(mpi_ptr_t wp"));
        assert!(source.contains("wp += 1;"));
        assert!(source.contains("retval = low_limb >> sh_2;"));
        assert!(source.contains("while (--i >= 0)"));
        assert!(source.contains("wp[i] = (high_limb << sh_1) | (low_limb >> sh_2);"));
        assert!(source.contains("wp[i] = high_limb << sh_1;"));

        let input = [0x0123_4567_89ab_cdefusize, 0xfedc_ba98_7654_3210];
        let mut output = [0usize; 2];
        let carry = mpihelp_lshift_slice(&mut output, &input, 4);
        assert_eq!(output, [0x1234_5678_9abc_def0, 0xedcb_a987_6543_2100]);
        assert_eq!(carry, 0xf);
    }

    #[test]
    fn raw_mpihelp_lshift_uses_limb_pointers() {
        let input = [1usize, 2];
        let mut output = [0usize; 2];
        let carry = unsafe { mpihelp_lshift(output.as_mut_ptr(), input.as_ptr(), 2, 1) };
        assert_eq!(output, [2, 4]);
        assert_eq!(carry, 0);
    }

    #[test]
    fn raw_mpihelp_lshift_supports_in_place_buffers() {
        let mut limbs = [0x0123_4567_89ab_cdefusize, 0xfedc_ba98_7654_3210];
        let carry = unsafe {
            mpihelp_lshift(
                limbs.as_mut_ptr(),
                limbs.as_ptr(),
                limbs.len() as MpiSize,
                4,
            )
        };
        assert_eq!(limbs, [0x1234_5678_9abc_def0, 0xedcb_a987_6543_2100]);
        assert_eq!(carry, 0xf);
    }
}
