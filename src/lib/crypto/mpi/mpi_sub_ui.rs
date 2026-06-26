//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/mpi-sub-ui.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/mpi-sub-ui.c
//! Subtract an unsigned integer from an MPI.

extern crate alloc;

use super::generic_mpih_add1::mpihelp_add_n_slices;
use super::generic_mpih_sub1::mpihelp_sub_n_slice;
use super::mpi_cmp::Mpi;
use super::mpih_cmp::MpiLimb;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("mpi_sub_ui", mpi_sub_ui_raw as usize, true);
}

fn add_limb(input: &[MpiLimb], limb: MpiLimb) -> alloc::vec::Vec<MpiLimb> {
    let mut out = alloc::vec![0; input.len()];
    let carry = mpihelp_add_n_slices(&mut out, input, &single_limb_vec(input.len(), limb));
    if carry != 0 {
        out.push(carry);
    }
    out
}

fn single_limb_vec(len: usize, limb: MpiLimb) -> alloc::vec::Vec<MpiLimb> {
    let mut v = alloc::vec![0; len];
    if !v.is_empty() {
        v[0] = limb;
    }
    v
}

fn sub_limb(input: &[MpiLimb], limb: MpiLimb) -> alloc::vec::Vec<MpiLimb> {
    let mut out = alloc::vec![0; input.len()];
    mpihelp_sub_n_slice(&mut out, input, &single_limb_vec(input.len(), limb));
    out
}

pub fn mpi_sub_ui(w: &mut Mpi, u: &Mpi, vval: MpiLimb) -> Result<(), i32> {
    if u.limbs.is_empty() {
        *w = Mpi::new(
            vval != 0,
            if vval == 0 {
                alloc::vec![]
            } else {
                alloc::vec![vval]
            },
        );
        return Ok(());
    }

    if u.sign {
        *w = Mpi::new(true, add_limb(&u.limbs, vval));
    } else if u.limbs.len() == 1 && u.limbs[0] < vval {
        *w = Mpi::new(true, alloc::vec![vval - u.limbs[0]]);
    } else {
        *w = Mpi::new(false, sub_limb(&u.limbs, vval));
    }

    w.normalize();
    Ok(())
}

pub unsafe extern "C" fn mpi_sub_ui_raw(w: *mut Mpi, u: *const Mpi, vval: MpiLimb) -> i32 {
    if w.is_null() || u.is_null() {
        return 0;
    }
    match unsafe { mpi_sub_ui(&mut *w, &*u, vval) } {
        Ok(()) => 0,
        Err(err) => err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpi_sub_ui_matches_linux_sign_cases() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-sub-ui.c"
        ));
        assert!(source.contains("if (u->nlimbs == 0)"));
        assert!(source.contains("w->sign = (vval != 0);"));
        assert!(source.contains("if (u->sign)"));
        assert!(source.contains("mpihelp_add_1"));
        assert!(source.contains("u->nlimbs == 1 && u->d[0] < vval"));
        assert!(source.contains("mpihelp_sub_1"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_sub_ui);"));

        let mut out = Mpi::from_ui(0);
        mpi_sub_ui(&mut out, &Mpi::from_ui(0), 7).expect("sub");
        assert_eq!(out, Mpi::new(true, alloc::vec![7]));

        mpi_sub_ui(&mut out, &Mpi::new(true, alloc::vec![5]), 7).expect("sub");
        assert_eq!(out, Mpi::new(true, alloc::vec![12]));

        mpi_sub_ui(&mut out, &Mpi::new(false, alloc::vec![5]), 7).expect("sub");
        assert_eq!(out, Mpi::new(true, alloc::vec![2]));

        mpi_sub_ui(&mut out, &Mpi::new(false, alloc::vec![0, 1]), 1).expect("sub");
        assert_eq!(out, Mpi::new(false, alloc::vec![usize::MAX]));
    }
}
