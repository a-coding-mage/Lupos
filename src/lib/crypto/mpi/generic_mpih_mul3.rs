//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-mul3.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-mul3.c
//! MPI subtract-multiply by one limb.

pub type MpiLimb = usize;
pub type MpiSize = i32;

fn mul_limb(lhs: MpiLimb, rhs: MpiLimb) -> (MpiLimb, MpiLimb) {
    let product = (lhs as u128) * (rhs as u128);
    let low = product as MpiLimb;
    let high = (product >> usize::BITS) as MpiLimb;
    (high, low)
}

pub fn mpihelp_submul_1_slice(res: &mut [MpiLimb], s1: &[MpiLimb], s2_limb: MpiLimb) -> MpiLimb {
    assert_eq!(res.len(), s1.len());

    let mut cy_limb = 0;
    for index in 0..s1.len() {
        let (prod_high, prod_low) = mul_limb(s1[index], s2_limb);
        let (prod_low, carry_from_low) = prod_low.overflowing_add(cy_limb);
        cy_limb = prod_high.wrapping_add(carry_from_low as MpiLimb);

        let x = res[index];
        let (difference, borrow_from_res) = x.overflowing_sub(prod_low);
        cy_limb = cy_limb.wrapping_add(borrow_from_res as MpiLimb);
        res[index] = difference;
    }
    cy_limb
}

pub unsafe extern "C" fn mpihelp_submul_1(
    res_ptr: *mut MpiLimb,
    s1_ptr: *const MpiLimb,
    s1_size: MpiSize,
    s2_limb: MpiLimb,
) -> MpiLimb {
    if s1_size <= 0 || res_ptr.is_null() || s1_ptr.is_null() {
        return 0;
    }

    let size = s1_size as usize;
    let res = unsafe { core::slice::from_raw_parts_mut(res_ptr, size) };
    let s1 = unsafe { core::slice::from_raw_parts(s1_ptr, size) };
    mpihelp_submul_1_slice(res, s1, s2_limb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_submul_1_matches_linux_submul_carry_chain() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-mul3.c"
        ));
        assert!(source.contains("mpihelp_submul_1(mpi_ptr_t res_ptr"));
        assert!(source.contains("umul_ppmm(prod_high, prod_low"));
        assert!(source.contains("prod_low = x - prod_low;"));
        assert!(source.contains("cy_limb += prod_low > x ? 1 : 0;"));

        let mut out = [20usize, 0, 20];
        let carry = mpihelp_submul_1_slice(&mut out, &[3, 1, 2], 5);
        assert_eq!(out, [5, usize::MAX - 4, 9]);
        assert_eq!(carry, 0);

        let mut overflow = [0usize];
        assert_eq!(
            mpihelp_submul_1_slice(&mut overflow, &[usize::MAX], usize::MAX),
            usize::MAX
        );
        assert_eq!(overflow, [usize::MAX]);
    }

    #[test]
    fn raw_mpihelp_submul_1_uses_limb_slices() {
        let input = [3usize, 4];
        let mut out = [20usize, 30];
        let carry = unsafe { mpihelp_submul_1(out.as_mut_ptr(), input.as_ptr(), 2, 5) };
        assert_eq!(out, [5, 10]);
        assert_eq!(carry, 0);
    }
}
