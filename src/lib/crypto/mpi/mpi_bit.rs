//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/mpi-bit.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/mpi-bit.c
//! MPI bit-level helpers.

extern crate alloc;

use super::generic_mpih_rshift::mpihelp_rshift_slice;
use super::mpi_cmp::Mpi;
use super::mpih_cmp::MpiLimb;
use crate::include::uapi::errno::{EINVAL, ENOMEM};
use crate::kernel::module::{export_symbol, find_symbol};

const A_LIMB_1: MpiLimb = 1;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("mpi_get_nbits", mpi_get_nbits_raw as usize, true);
    export_symbol_once("mpi_test_bit", mpi_test_bit_raw as usize, true);
    export_symbol_once("mpi_set_bit", mpi_set_bit_raw as usize, true);
    export_symbol_once("mpi_rshift", mpi_rshift_raw as usize, true);
}

pub fn mpi_normalize(a: &mut Mpi) {
    while a.limbs.last().copied() == Some(0) {
        a.limbs.pop();
    }
}

pub fn mpi_get_nbits(a: &mut Mpi) -> u32 {
    mpi_normalize(a);

    if let Some(&alimb) = a.limbs.last() {
        let n = if alimb != 0 {
            alimb.leading_zeros()
        } else {
            MpiLimb::BITS
        };
        MpiLimb::BITS - n + (a.limbs.len() as u32 - 1) * MpiLimb::BITS
    } else {
        0
    }
}

pub fn mpi_test_bit(a: &Mpi, n: u32) -> i32 {
    let limbno = n / MpiLimb::BITS;
    let bitno = n % MpiLimb::BITS;

    if limbno as usize >= a.limbs.len() {
        return 0;
    }
    let limb = a.limbs[limbno as usize];
    if (limb & (A_LIMB_1 << bitno)) != 0 {
        1
    } else {
        0
    }
}

pub fn mpi_set_bit(a: &mut Mpi, n: u32) -> Result<(), i32> {
    let limbno = n / MpiLimb::BITS;
    let bitno = n % MpiLimb::BITS;
    let needed = limbno as usize + 1;

    if needed > a.limbs.len() {
        a.limbs
            .try_reserve(needed - a.limbs.len())
            .map_err(|_| -ENOMEM)?;
        a.limbs.resize(needed, 0);
    }
    a.limbs[limbno as usize] |= A_LIMB_1 << bitno;
    Ok(())
}

pub fn mpi_rshift(x: &mut Mpi, a: &Mpi, n: u32) -> Result<(), i32> {
    let nlimbs = (n / MpiLimb::BITS) as usize;
    let nbits = n % MpiLimb::BITS;
    let sign = a.sign;
    let mut limbs = a.limbs.clone();

    if nlimbs >= limbs.len() {
        x.limbs.clear();
        x.sign = sign;
        return Ok(());
    }

    if nlimbs != 0 {
        limbs.copy_within(nlimbs.., 0);
        let new_len = limbs.len() - nlimbs;
        limbs.truncate(new_len);
    }

    if !limbs.is_empty() && nbits != 0 {
        let input = limbs.clone();
        mpihelp_rshift_slice(&mut limbs, &input, nbits);
    }

    while limbs.last().copied() == Some(0) {
        limbs.pop();
    }
    x.limbs = limbs;
    x.sign = sign;
    Ok(())
}

pub unsafe extern "C" fn mpi_get_nbits_raw(a: *mut Mpi) -> u32 {
    if a.is_null() {
        return 0;
    }
    unsafe { mpi_get_nbits(&mut *a) }
}

pub unsafe extern "C" fn mpi_test_bit_raw(a: *const Mpi, n: u32) -> i32 {
    if a.is_null() {
        return 0;
    }
    unsafe { mpi_test_bit(&*a, n) }
}

pub unsafe extern "C" fn mpi_set_bit_raw(a: *mut Mpi, n: u32) -> i32 {
    if a.is_null() {
        return -EINVAL;
    }
    match unsafe { mpi_set_bit(&mut *a, n) } {
        Ok(()) => 0,
        Err(err) => err,
    }
}

pub unsafe extern "C" fn mpi_rshift_raw(x: *mut Mpi, a: *const Mpi, n: u32) -> i32 {
    if x.is_null() || a.is_null() {
        return -EINVAL;
    }
    let input = unsafe { (*a).clone() };
    match unsafe { mpi_rshift(&mut *x, &input, n) } {
        Ok(()) => 0,
        Err(err) => err,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpi_normalize_and_get_nbits_match_linux_limb_rules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-bit.c"
        ));
        assert!(source.contains("void mpi_normalize(MPI a)"));
        assert!(source.contains("for (; a->nlimbs && !a->d[a->nlimbs - 1]; a->nlimbs--)"));
        assert!(source.contains("n = count_leading_zeros(alimb);"));
        assert!(source.contains("BITS_PER_MPI_LIMB - n + (a->nlimbs - 1) * BITS_PER_MPI_LIMB"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_get_nbits);"));

        let mut negative_zero = Mpi {
            limbs: alloc::vec![0, 0],
            sign: true,
        };
        assert_eq!(mpi_get_nbits(&mut negative_zero), 0);
        assert!(negative_zero.sign);
        assert!(negative_zero.limbs.is_empty());

        let mut value = Mpi::new(false, alloc::vec![0, 0x8000]);
        assert_eq!(mpi_get_nbits(&mut value), MpiLimb::BITS + 16);
    }

    #[test]
    fn mpi_test_and_set_bit_match_linux_indexing() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-bit.c"
        ));
        assert!(source.contains("limbno = n / BITS_PER_MPI_LIMB;"));
        assert!(source.contains("bitno  = n % BITS_PER_MPI_LIMB;"));
        assert!(source.contains("return (limb & (A_LIMB_1 << bitno)) ? 1 : 0;"));
        assert!(source.contains("a->d[limbno] |= (A_LIMB_1<<bitno);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_set_bit);"));

        let mut value = Mpi::from_ui(0);
        assert_eq!(mpi_test_bit(&value, MpiLimb::BITS), 0);
        mpi_set_bit(&mut value, MpiLimb::BITS).expect("set bit");
        assert_eq!(value.limbs, alloc::vec![0, 1]);
        assert_eq!(mpi_test_bit(&value, MpiLimb::BITS), 1);
        assert_eq!(mpi_test_bit(&value, MpiLimb::BITS - 1), 0);
    }

    #[test]
    fn mpi_rshift_matches_linux_copy_in_place_and_normalize_cases() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-bit.c"
        ));
        assert!(source.contains("if (x == a)"));
        assert!(source.contains("if (nlimbs >= x->nlimbs)"));
        assert!(source.contains("x->d[i] = x->d[i+nlimbs];"));
        assert!(source.contains("mpihelp_rshift(x->d, x->d, x->nlimbs, nbits);"));
        assert!(source.contains("MPN_NORMALIZE(x->d, x->nlimbs);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_rshift);"));

        let input = Mpi::new(false, alloc::vec![0, 1]);
        let mut out = Mpi::from_ui(99);
        mpi_rshift(&mut out, &input, MpiLimb::BITS).expect("whole-limb shift");
        assert_eq!(out, Mpi::from_ui(1));

        let input = Mpi::new(true, alloc::vec![0b1000, 0]);
        mpi_rshift(&mut out, &input, 2).expect("bit shift");
        assert_eq!(
            out,
            Mpi {
                limbs: alloc::vec![0b10],
                sign: true,
            }
        );

        let input = Mpi::new(true, alloc::vec![1]);
        mpi_rshift(&mut out, &input, MpiLimb::BITS).expect("shift beyond size");
        assert_eq!(out.limbs, alloc::vec![]);
        assert!(out.sign);
    }

    #[test]
    fn raw_mpi_rshift_supports_same_input_and_output_pointer() {
        let mut value = Mpi::new(false, alloc::vec![0, 4]);
        let ret = unsafe { mpi_rshift_raw(&mut value, &raw const value, MpiLimb::BITS + 1) };
        assert_eq!(ret, 0);
        assert_eq!(value, Mpi::from_ui(2));
    }
}
