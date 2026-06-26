//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-mul1.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-mul1.c
//! Generic MPI helper for multiplying a limb vector by one limb.

use super::mpih_cmp::{MpiLimb, MpiSize};

pub fn mpihelp_mul_1_slice(res: &mut [MpiLimb], s1: &[MpiLimb], s2_limb: MpiLimb) -> MpiLimb {
    assert!(res.len() >= s1.len());
    let mut cy_limb: MpiLimb = 0;
    for (out, input) in res.iter_mut().zip(s1.iter()).take(s1.len()) {
        let product = (*input as u128) * (s2_limb as u128) + (cy_limb as u128);
        *out = product as MpiLimb;
        cy_limb = (product >> MpiLimb::BITS) as MpiLimb;
    }
    cy_limb
}

pub unsafe extern "C" fn mpihelp_mul_1(
    res_ptr: *mut MpiLimb,
    s1_ptr: *const MpiLimb,
    s1_size: MpiSize,
    s2_limb: MpiLimb,
) -> MpiLimb {
    if s1_size <= 0 {
        return 0;
    }
    if res_ptr.is_null() || s1_ptr.is_null() {
        return 0;
    }

    let size = s1_size as usize;
    let res = unsafe { core::slice::from_raw_parts_mut(res_ptr, size) };
    let s1 = unsafe { core::slice::from_raw_parts(s1_ptr, size) };
    mpihelp_mul_1_slice(res, s1, s2_limb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_mul_1_matches_linux_limb_carry_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-mul1.c"
        ));
        assert!(source.contains("mpihelp_mul_1"));
        assert!(source.contains("umul_ppmm(prod_high, prod_low, s1_ptr[j], s2_limb);"));
        assert!(source.contains("prod_low += cy_limb;"));
        assert!(source.contains("cy_limb = (prod_low < cy_limb ? 1 : 0) + prod_high;"));
        assert!(source.contains("res_ptr[j] = prod_low;"));

        let input = [MpiLimb::MAX, 2, 3];
        let mut output = [0; 3];
        let carry = mpihelp_mul_1_slice(&mut output, &input, 2);
        assert_eq!(output, [MpiLimb::MAX.wrapping_mul(2), 5, 6]);
        assert_eq!(carry, 0);
    }

    #[test]
    fn raw_mpihelp_mul_1_writes_result_and_returns_carry() {
        let input = [MpiLimb::MAX, MpiLimb::MAX];
        let mut output = [0; 2];
        let carry = unsafe {
            mpihelp_mul_1(
                output.as_mut_ptr(),
                input.as_ptr(),
                input.len() as MpiSize,
                MpiLimb::MAX,
            )
        };
        assert_eq!(output[0], 1);
        assert_eq!(output[1], MpiLimb::MAX);
        assert_eq!(carry, MpiLimb::MAX - 1);
    }
}
