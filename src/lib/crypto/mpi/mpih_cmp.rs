//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/mpih-cmp.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/mpih-cmp.c
//! MPI limb comparison helper.

pub type MpiLimb = usize;
pub type MpiSize = i32;

pub fn mpihelp_cmp_slice(op1: &[MpiLimb], op2: &[MpiLimb]) -> i32 {
    assert_eq!(op1.len(), op2.len());
    for index in (0..op1.len()).rev() {
        let op1_word = op1[index];
        let op2_word = op2[index];
        if op1_word != op2_word {
            return if op1_word > op2_word { 1 } else { -1 };
        }
    }
    0
}

pub unsafe extern "C" fn mpihelp_cmp(
    op1_ptr: *const MpiLimb,
    op2_ptr: *const MpiLimb,
    size: MpiSize,
) -> i32 {
    if size <= 0 {
        return 0;
    }
    if op1_ptr.is_null() || op2_ptr.is_null() {
        return 0;
    }

    let size = size as usize;
    let op1 = unsafe { core::slice::from_raw_parts(op1_ptr, size) };
    let op2 = unsafe { core::slice::from_raw_parts(op2_ptr, size) };
    mpihelp_cmp_slice(op1, op2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpihelp_cmp_walks_limbs_from_most_significant_to_least() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpih-cmp.c"
        ));
        assert!(source.contains("#include \"mpi-internal.h\""));
        assert!(source.contains("int mpihelp_cmp(mpi_ptr_t op1_ptr, mpi_ptr_t op2_ptr"));
        assert!(source.contains("for (i = size - 1; i >= 0; i--)"));
        assert!(source.contains("op1_word = op1_ptr[i];"));
        assert!(source.contains("op2_word = op2_ptr[i];"));
        assert!(source.contains("return (op1_word > op2_word) ? 1 : -1;"));

        assert_eq!(mpihelp_cmp_slice(&[1, 2, 3], &[1, 2, 3]), 0);
        assert_eq!(mpihelp_cmp_slice(&[usize::MAX, 1], &[0, 2]), -1);
        assert_eq!(mpihelp_cmp_slice(&[0, 2], &[usize::MAX, 1]), 1);
        assert_eq!(mpihelp_cmp_slice(&[7, 9], &[8, 9]), -1);
        assert_eq!(mpihelp_cmp_slice(&[9, 9], &[8, 9]), 1);
    }

    #[test]
    fn raw_mpihelp_cmp_matches_slice_helper() {
        let left = [0usize, 10, 99];
        let right = [usize::MAX, 10, 98];
        let result = unsafe { mpihelp_cmp(left.as_ptr(), right.as_ptr(), left.len() as MpiSize) };
        assert_eq!(result, 1);
        assert_eq!(
            unsafe { mpihelp_cmp(left.as_ptr(), left.as_ptr(), left.len() as MpiSize) },
            0
        );
    }
}
