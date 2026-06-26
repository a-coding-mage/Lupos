//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/mpi/mpi-mul.c
//! test-origin: linux:vendor/linux/lib/crypto/mpi/mpi-mul.c
//! High-level MPI multiplication helpers.

extern crate alloc;

use alloc::vec::Vec;

use super::mpi_cmp::Mpi;
use super::mpih_cmp::MpiLimb;
use crate::include::uapi::errno::EINVAL;
use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("mpi_mul", mpi_mul_raw as usize, true);
    export_symbol_once("mpi_mulm", mpi_mulm_raw as usize, true);
}

pub fn mpi_mul(w: &mut Mpi, u: &Mpi, v: &Mpi) -> Result<(), i32> {
    let (up, vp) = if u.limbs.len() < v.limbs.len() {
        (&v.limbs, &u.limbs)
    } else {
        (&u.limbs, &v.limbs)
    };
    let limbs = mul_abs(up, vp);
    *w = Mpi::new(u.sign ^ v.sign, limbs);
    Ok(())
}

pub fn mpi_mulm(w: &mut Mpi, u: &Mpi, v: &Mpi, m: &Mpi) -> Result<(), i32> {
    let mut product = Mpi::from_ui(0);
    mpi_mul(&mut product, u, v)?;
    let remainder = mod_abs(&product.limbs, &m.limbs)?;
    *w = Mpi::new(false, remainder);
    Ok(())
}

pub unsafe extern "C" fn mpi_mul_raw(w: *mut Mpi, u: *const Mpi, v: *const Mpi) -> i32 {
    if w.is_null() || u.is_null() || v.is_null() {
        return -EINVAL;
    }
    match unsafe { mpi_mul(&mut *w, &*u, &*v) } {
        Ok(()) => 0,
        Err(err) => err,
    }
}

pub unsafe extern "C" fn mpi_mulm_raw(
    w: *mut Mpi,
    u: *const Mpi,
    v: *const Mpi,
    m: *const Mpi,
) -> i32 {
    if w.is_null() || u.is_null() || v.is_null() || m.is_null() {
        return -EINVAL;
    }
    match unsafe { mpi_mulm(&mut *w, &*u, &*v, &*m) } {
        Ok(()) => 0,
        Err(err) => err,
    }
}

fn mul_abs(up: &[MpiLimb], vp: &[MpiLimb]) -> Vec<MpiLimb> {
    if up.is_empty() || vp.is_empty() {
        return Vec::new();
    }

    let mut out = alloc::vec![0; up.len() + vp.len()];
    for (i, &u_limb) in up.iter().enumerate() {
        let mut carry = 0u128;
        for (j, &v_limb) in vp.iter().enumerate() {
            let index = i + j;
            let total = out[index] as u128 + (u_limb as u128) * (v_limb as u128) + carry;
            out[index] = total as MpiLimb;
            carry = total >> MpiLimb::BITS;
        }
        let mut index = i + vp.len();
        while carry != 0 {
            if index == out.len() {
                out.push(0);
            }
            let total = out[index] as u128 + carry;
            out[index] = total as MpiLimb;
            carry = total >> MpiLimb::BITS;
            index += 1;
        }
    }
    normalize(out)
}

fn mod_abs(dividend: &[MpiLimb], divisor: &[MpiLimb]) -> Result<Vec<MpiLimb>, i32> {
    let divisor = normalize(divisor.to_vec());
    if divisor.is_empty() {
        return Err(-EINVAL);
    }
    let mut remainder = Vec::new();
    for bit in (0..bit_len(dividend)).rev() {
        shl1(&mut remainder);
        if get_bit(dividend, bit) {
            if remainder.is_empty() {
                remainder.push(1);
            } else {
                remainder[0] |= 1;
            }
        }
        if cmp_abs(&remainder, &divisor) != core::cmp::Ordering::Less {
            sub_abs_assign(&mut remainder, &divisor);
        }
    }
    Ok(normalize(remainder))
}

fn bit_len(limbs: &[MpiLimb]) -> usize {
    let limbs = normalize(limbs.to_vec());
    if let Some(&last) = limbs.last() {
        (limbs.len() - 1) * MpiLimb::BITS as usize + (MpiLimb::BITS - last.leading_zeros()) as usize
    } else {
        0
    }
}

fn get_bit(limbs: &[MpiLimb], bit: usize) -> bool {
    let limb = bit / MpiLimb::BITS as usize;
    let offset = bit % MpiLimb::BITS as usize;
    limbs
        .get(limb)
        .map(|value| ((value >> offset) & 1) != 0)
        .unwrap_or(false)
}

fn shl1(limbs: &mut Vec<MpiLimb>) {
    let mut carry = 0usize;
    for limb in limbs.iter_mut() {
        let next = *limb >> (MpiLimb::BITS as usize - 1);
        *limb = (*limb << 1) | carry;
        carry = next;
    }
    if carry != 0 {
        limbs.push(carry);
    }
}

fn cmp_abs(left: &[MpiLimb], right: &[MpiLimb]) -> core::cmp::Ordering {
    let left = normalize(left.to_vec());
    let right = normalize(right.to_vec());
    match left.len().cmp(&right.len()) {
        core::cmp::Ordering::Equal => {
            for index in (0..left.len()).rev() {
                match left[index].cmp(&right[index]) {
                    core::cmp::Ordering::Equal => {}
                    ordering => return ordering,
                }
            }
            core::cmp::Ordering::Equal
        }
        ordering => ordering,
    }
}

fn sub_abs_assign(left: &mut Vec<MpiLimb>, right: &[MpiLimb]) {
    let mut borrow = 0usize;
    for index in 0..left.len() {
        let rhs = right.get(index).copied().unwrap_or(0);
        let (tmp, b1) = left[index].overflowing_sub(rhs);
        let (tmp, b2) = tmp.overflowing_sub(borrow);
        left[index] = tmp;
        borrow = (b1 || b2) as usize;
    }
    while left.last().copied() == Some(0) {
        left.pop();
    }
}

fn normalize(mut limbs: Vec<MpiLimb>) -> Vec<MpiLimb> {
    while limbs.last().copied() == Some(0) {
        limbs.pop();
    }
    limbs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpi_mul_matches_linux_alias_and_sign_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/mpi/mpi-mul.c"
        ));
        assert!(source.contains("if (u->nlimbs < v->nlimbs)"));
        assert!(source.contains("sign_product = usign ^ vsign;"));
        assert!(source.contains("wsize = usize + vsize;"));
        assert!(source.contains("if (wp == up || wp == vp)"));
        assert!(source.contains("err = mpihelp_mul(wp, up, usize, vp, vsize, &cy);"));
        assert!(source.contains("wsize -= cy ? 0:1;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_mul);"));
        assert!(source.contains("return mpi_mul(w, u, v) ?:"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(mpi_mulm);"));

        let mut out = Mpi::from_ui(0);
        mpi_mul(
            &mut out,
            &Mpi::new(false, alloc::vec![MpiLimb::MAX, 1]),
            &Mpi::new(true, alloc::vec![2]),
        )
        .expect("mul");
        assert_eq!(out, Mpi::new(true, alloc::vec![MpiLimb::MAX - 1, 3]));

        mpi_mul(&mut out, &Mpi::from_ui(0), &Mpi::from_ui(99)).expect("mul zero");
        assert_eq!(out, Mpi::from_ui(0));

        mpi_mulm(
            &mut out,
            &Mpi::new(false, alloc::vec![17]),
            &Mpi::new(false, alloc::vec![19]),
            &Mpi::new(false, alloc::vec![23]),
        )
        .expect("mulm");
        assert_eq!(out, Mpi::from_ui((17 * 19) % 23));
    }
}
