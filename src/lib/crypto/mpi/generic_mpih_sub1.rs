//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/generic_mpih-sub1.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/generic_mpih-sub1.c
//! Generic MPI helper for subtracting equal-length limb vectors.

use super::mpih_cmp::{MpiLimb, MpiSize};

pub fn mpihelp_sub_n_slice(res: &mut [MpiLimb], s1: &[MpiLimb], s2: &[MpiLimb]) -> MpiLimb {
    assert!(res.len() >= s1.len());
    assert_eq!(s1.len(), s2.len());

    let mut cy: MpiLimb = 0;
    for ((out, x), y) in res.iter_mut().zip(s1.iter()).zip(s2.iter()) {
        let (subtrahend, add_carry) = y.overflowing_add(cy);
        let (diff, sub_carry) = x.overflowing_sub(subtrahend);
        cy = (add_carry as MpiLimb).wrapping_add(sub_carry as MpiLimb);
        *out = diff;
    }
    cy
}

pub unsafe extern "C" fn mpihelp_sub_n(
    res_ptr: *mut MpiLimb,
    s1_ptr: *const MpiLimb,
    s2_ptr: *const MpiLimb,
    size: MpiSize,
) -> MpiLimb {
    if size <= 0 {
        return 0;
    }
    if res_ptr.is_null() || s1_ptr.is_null() || s2_ptr.is_null() {
        return 0;
    }

    let size = size as usize;
    let res = unsafe { core::slice::from_raw_parts_mut(res_ptr, size) };
    let s1 = unsafe { core::slice::from_raw_parts(s1_ptr, size) };
    let s2 = unsafe { core::slice::from_raw_parts(s2_ptr, size) };
    mpihelp_sub_n_slice(res, s1, s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_sub_n_propagates_linux_borrow_carry() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/generic_mpih-sub1.c"
        ));
        assert!(source.contains("mpihelp_sub_n"));
        assert!(source.contains("y += cy"));
        assert!(source.contains("cy = y < cy"));
        assert!(source.contains("y = x - y"));
        assert!(source.contains("cy += y > x"));
        assert!(source.contains("res_ptr[j] = y;"));

        let left = [0, 0, 2];
        let right = [1, 0, 1];
        let mut output = [0; 3];
        let borrow = mpihelp_sub_n_slice(&mut output, &left, &right);
        assert_eq!(output, [MpiLimb::MAX, MpiLimb::MAX, 0]);
        assert_eq!(borrow, 0);
    }

    #[test]
    fn raw_mpihelp_sub_n_returns_final_borrow() {
        let left = [0, 0];
        let right = [1, 0];
        let mut output = [0; 2];
        let borrow = unsafe {
            mpihelp_sub_n(
                output.as_mut_ptr(),
                left.as_ptr(),
                right.as_ptr(),
                left.len() as MpiSize,
            )
        };
        assert_eq!(output, [MpiLimb::MAX, MpiLimb::MAX]);
        assert_eq!(borrow, 1);
    }
}
