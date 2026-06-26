//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/mpi-cmp.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/mpi-cmp.c
//! High-level MPI comparison helpers.

extern crate alloc;

use alloc::vec::Vec;

use super::mpih_cmp::{MpiLimb, mpihelp_cmp_slice};
use crate::kernel::module::{export_symbol, find_symbol};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mpi {
    pub limbs: Vec<MpiLimb>,
    pub sign: bool,
}

impl Mpi {
    pub fn new(sign: bool, mut limbs: Vec<MpiLimb>) -> Self {
        while limbs.last().copied() == Some(0) {
            limbs.pop();
        }
        Self {
            sign: sign && !limbs.is_empty(),
            limbs,
        }
    }

    pub fn from_ui(value: MpiLimb) -> Self {
        if value == 0 {
            Self::new(false, Vec::new())
        } else {
            Self::new(false, alloc::vec![value])
        }
    }

    pub fn normalize(&mut self) {
        while self.limbs.last().copied() == Some(0) {
            self.limbs.pop();
        }
        if self.limbs.is_empty() {
            self.sign = false;
        }
    }
}

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("mpi_cmp_ui", mpi_cmp_ui_raw as usize, true);
    export_symbol_once("mpi_cmp", mpi_cmp_raw as usize, true);
}

pub fn mpi_cmp_ui(u: &mut Mpi, v: MpiLimb) -> i32 {
    u.normalize();
    if u.limbs.is_empty() {
        return if v == 0 { 0 } else { -1 };
    }
    if u.sign {
        return -1;
    }
    if u.limbs.len() > 1 {
        return 1;
    }
    match u.limbs[0].cmp(&v) {
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
        core::cmp::Ordering::Less => -1,
    }
}

pub fn mpi_cmp(u: &mut Mpi, v: &mut Mpi) -> i32 {
    u.normalize();
    v.normalize();

    let usize = u.limbs.len() as i32;
    let vsize = v.limbs.len() as i32;
    if !u.sign && v.sign {
        return 1;
    }
    if u.sign && !v.sign {
        return -1;
    }
    if usize != vsize && !u.sign && !v.sign {
        return usize - vsize;
    }
    if usize != vsize && u.sign && v.sign {
        return vsize - usize;
    }
    if usize == 0 {
        return 0;
    }

    let cmp = mpihelp_cmp_slice(&u.limbs, &v.limbs);
    if u.sign { -cmp } else { cmp }
}

pub unsafe extern "C" fn mpi_cmp_ui_raw(u: *mut Mpi, v: MpiLimb) -> i32 {
    if u.is_null() {
        return if v == 0 { 0 } else { -1 };
    }
    unsafe { mpi_cmp_ui(&mut *u, v) }
}

pub unsafe extern "C" fn mpi_cmp_raw(u: *mut Mpi, v: *mut Mpi) -> i32 {
    if u.is_null() || v.is_null() {
        return 0;
    }
    unsafe { mpi_cmp(&mut *u, &mut *v) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpi_cmp_matches_linux_sign_size_and_limb_order() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-cmp.c"
        ));
        assert!(source.contains("mpi_normalize(u);"));
        assert!(source.contains("if (u->nlimbs == 0)"));
        assert!(source.contains("if (u->sign)"));
        assert!(source.contains("return usize - vsize;"));
        assert!(source.contains("cmp = mpihelp_cmp(u->d, v->d, usize);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_cmp);"));

        let mut zero = Mpi::from_ui(0);
        assert_eq!(mpi_cmp_ui(&mut zero, 0), 0);
        assert_eq!(mpi_cmp_ui(&mut zero, 7), -1);

        let mut big = Mpi::new(false, alloc::vec![0, 1]);
        assert_eq!(mpi_cmp_ui(&mut big, MpiLimb::MAX), 1);

        let mut neg = Mpi::new(true, alloc::vec![99]);
        let mut pos = Mpi::new(false, alloc::vec![1]);
        assert_eq!(mpi_cmp(&mut neg, &mut pos), -1);

        let mut left = Mpi::new(false, alloc::vec![usize::MAX, 2]);
        let mut right = Mpi::new(false, alloc::vec![0, 3]);
        assert_eq!(mpi_cmp(&mut left, &mut right), -1);
    }
}
