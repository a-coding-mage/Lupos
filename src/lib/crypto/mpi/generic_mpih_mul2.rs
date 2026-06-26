//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-mul2.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-mul2.c
//! Generic MPI helper for add-multiply by one limb.

use super::mpih_cmp::{MpiLimb, MpiSize};

pub fn mpihelp_addmul_1_slice(res: &mut [MpiLimb], s1: &[MpiLimb], s2_limb: MpiLimb) -> MpiLimb {
    assert!(res.len() >= s1.len());
    let mut cy_limb: MpiLimb = 0;
    for (out, input) in res.iter_mut().zip(s1.iter()).take(s1.len()) {
        let product = (*input as u128) * (s2_limb as u128);
        let prod_high = (product >> MpiLimb::BITS) as MpiLimb;
        let (prod_low, low_carry) = (product as MpiLimb).overflowing_add(cy_limb);
        cy_limb = prod_high.wrapping_add(low_carry as MpiLimb);

        let x = *out;
        let (sum, res_carry) = x.overflowing_add(prod_low);
        cy_limb = cy_limb.wrapping_add(res_carry as MpiLimb);
        *out = sum;
    }
    cy_limb
}

pub unsafe extern "C" fn mpihelp_addmul_1(
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
    mpihelp_addmul_1_slice(res, s1, s2_limb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_addmul_1_adds_product_into_destination() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-mul2.c"
        ));
        assert!(source.contains("mpihelp_addmul_1"));
        assert!(source.contains("umul_ppmm(prod_high, prod_low, s1_ptr[j], s2_limb);"));
        assert!(source.contains("x = res_ptr[j];"));
        assert!(source.contains("prod_low = x + prod_low;"));
        assert!(source.contains("cy_limb += prod_low < x ? 1 : 0;"));
        assert!(source.contains("res_ptr[j] = prod_low;"));

        let input = [MpiLimb::MAX, 1];
        let mut output = [1, MpiLimb::MAX];
        let carry = mpihelp_addmul_1_slice(&mut output, &input, 2);
        assert_eq!(output, [MpiLimb::MAX, 2]);
        assert_eq!(carry, 1);
    }

    #[test]
    fn raw_mpihelp_addmul_1_matches_slice_helper() {
        let input = [3, 4, 5];
        let mut output = [10, 20, 30];
        let carry = unsafe {
            mpihelp_addmul_1(
                output.as_mut_ptr(),
                input.as_ptr(),
                input.len() as MpiSize,
                7,
            )
        };
        assert_eq!(output, [31, 48, 65]);
        assert_eq!(carry, 0);
    }
}
