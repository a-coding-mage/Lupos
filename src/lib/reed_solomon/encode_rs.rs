//! linux-parity: complete
//! linux-source: vendor/linux/lib/reed_solomon/encode_rs.c
//! test-origin: linux:vendor/linux/lib/reed_solomon/encode_rs.c
//! Generic Reed-Solomon parity encoder body.

use crate::include::uapi::errno::ERANGE;

#[derive(Clone, Copy)]
pub struct RsCodecTables<'a> {
    pub mm: usize,
    pub nn: usize,
    pub nroots: usize,
    pub alpha_to: &'a [u16],
    pub index_of: &'a [u16],
    pub genpoly: &'a [u16],
}

pub fn rs_modnn(rs: &RsCodecTables<'_>, mut x: usize) -> usize {
    while x >= rs.nn {
        x -= rs.nn;
        x = (x >> rs.mm) + (x & rs.nn);
    }
    x
}

pub fn encode_rs<T>(
    rs: &RsCodecTables<'_>,
    data: &[T],
    par: &mut [u16],
    invmsk: u16,
) -> Result<(), i32>
where
    T: Copy + Into<u16>,
{
    if par.len() < rs.nroots
        || rs.alpha_to.len() <= rs.nn
        || rs.index_of.len() <= rs.nn
        || rs.genpoly.len() <= rs.nroots
    {
        return Err(-ERANGE);
    }

    let pad = rs.nn as isize - rs.nroots as isize - data.len() as isize;
    if pad < 0 || pad >= rs.nn as isize {
        return Err(-ERANGE);
    }

    let msk = rs.nn as u16;
    for value in data {
        let feedback_input = ((((*value).into() ^ invmsk) & msk) ^ par[0]) as usize;
        let fb = rs.index_of[feedback_input];
        if fb != rs.nn as u16 {
            for j in 1..rs.nroots {
                let exponent = fb as usize + rs.genpoly[rs.nroots - j] as usize;
                par[j] ^= rs.alpha_to[rs_modnn(rs, exponent)];
            }
        }

        if rs.nroots > 1 {
            par.copy_within(1..rs.nroots, 0);
        }
        par[rs.nroots - 1] = if fb != rs.nn as u16 {
            let exponent = fb as usize + rs.genpoly[0] as usize;
            rs.alpha_to[rs_modnn(rs, exponent)]
        } else {
            0
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_rs_matches_linux_feedback_and_shift_body() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/reed_solomon/encode_rs.c"
        ));
        assert!(source.contains("struct rs_codec *rs = rsc->codec;"));
        assert!(source.contains("pad = nn - nroots - len;"));
        assert!(source.contains("return -ERANGE;"));
        assert!(source.contains("fb = index_of"));
        assert!(source.contains("par[j] ^= alpha_to[rs_modnn"));
        assert!(source.contains("memmove(&par[0], &par[1]"));
        assert!(source.contains("par[nroots - 1] = alpha_to[rs_modnn"));

        let alpha_to: [u16; 16] = core::array::from_fn(|index| index as u16);
        let mut index_of: [u16; 16] = core::array::from_fn(|index| index as u16);
        index_of[0] = 15;
        let genpoly = [1u16, 2, 15];
        let codec = RsCodecTables {
            mm: 4,
            nn: 15,
            nroots: 2,
            alpha_to: &alpha_to,
            index_of: &index_of,
            genpoly: &genpoly,
        };
        let mut parity = [0u16; 2];
        assert_eq!(encode_rs(&codec, &[1u8, 2], &mut parity, 0), Ok(()));
        assert_eq!(parity, [1, 2]);
        assert_eq!(encode_rs(&codec, &[0u8; 14], &mut parity, 0), Err(-ERANGE));
    }

    #[test]
    fn rs_modnn_matches_header_reduction_shape() {
        let alpha_to = [0u16; 16];
        let index_of = [0u16; 16];
        let genpoly = [0u16; 3];
        let codec = RsCodecTables {
            mm: 4,
            nn: 15,
            nroots: 2,
            alpha_to: &alpha_to,
            index_of: &index_of,
            genpoly: &genpoly,
        };
        assert_eq!(rs_modnn(&codec, 15), 0);
        assert_eq!(rs_modnn(&codec, 31), 1);
        assert_eq!(rs_modnn(&codec, 47), 2);
    }
}
