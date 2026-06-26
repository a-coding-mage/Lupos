//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-add1.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-add1.c
//! MPI limb vector addition.

pub type MpiLimb = usize;
pub type MpiSize = i32;

pub fn mpihelp_add_n_slices(res: &mut [MpiLimb], s1: &[MpiLimb], s2: &[MpiLimb]) -> MpiLimb {
    assert_eq!(res.len(), s1.len());
    assert_eq!(s1.len(), s2.len());

    let mut carry = false;
    for index in 0..s1.len() {
        let (with_carry, carry_from_carry) = s2[index].overflowing_add(carry as MpiLimb);
        let (sum, carry_from_sum) = with_carry.overflowing_add(s1[index]);
        carry = carry_from_carry || carry_from_sum;
        res[index] = sum;
    }
    carry as MpiLimb
}

pub unsafe extern "C" fn mpihelp_add_n(
    res_ptr: *mut MpiLimb,
    s1_ptr: *const MpiLimb,
    s2_ptr: *const MpiLimb,
    size: MpiSize,
) -> MpiLimb {
    if size <= 0 || res_ptr.is_null() || s1_ptr.is_null() || s2_ptr.is_null() {
        return 0;
    }

    let size = size as usize;
    let res = unsafe { core::slice::from_raw_parts_mut(res_ptr, size) };
    let s1 = unsafe { core::slice::from_raw_parts(s1_ptr, size) };
    let s2 = unsafe { core::slice::from_raw_parts(s2_ptr, size) };
    mpihelp_add_n_slices(res, s1, s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_add_n_matches_linux_carry_chain() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-add1.c"
        ));
        assert!(source.contains("mpihelp_add_n(mpi_ptr_t res_ptr"));
        assert!(source.contains("j = -size;"));
        assert!(source.contains("y += cy;"));
        assert!(source.contains("cy = y < cy;"));
        assert!(source.contains("cy += y < x;"));

        let mut out = [0; 3];
        let carry = mpihelp_add_n_slices(&mut out, &[usize::MAX, 0, 9], &[1, usize::MAX, 1]);
        assert_eq!(out, [0, 0, 11]);
        assert_eq!(carry, 0);

        let mut overflow = [0; 1];
        assert_eq!(
            mpihelp_add_n_slices(&mut overflow, &[usize::MAX], &[usize::MAX]),
            1
        );
        assert_eq!(overflow, [usize::MAX - 1]);
    }

    #[test]
    fn raw_mpihelp_add_n_uses_limb_slices() {
        let left = [usize::MAX, 7];
        let right = [2usize, 4];
        let mut out = [0usize; 2];
        let carry = unsafe { mpihelp_add_n(out.as_mut_ptr(), left.as_ptr(), right.as_ptr(), 2) };
        assert_eq!(out, [1, 12]);
        assert_eq!(carry, 0);
    }
}
