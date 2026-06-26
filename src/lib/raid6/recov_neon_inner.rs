//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid6/recov_neon_inner.c
//! test-origin: linux:vendor/linux/lib/raid6/recov_neon_inner.c
//! Scalar model of the ARM NEON RAID6 recovery inner loops.

pub const NEON_RECOVERY_STRIDE: usize = 16;
pub const NIBBLE_TABLE_LEN: usize = 32;

pub fn raid6_2data_recov_neon_inner(
    p: &[u8],
    q: &[u8],
    dp: &mut [u8],
    dq: &mut [u8],
    pbmul: &[u8],
    qmul: &[u8],
) {
    assert_eq!(p.len(), q.len());
    assert_eq!(p.len(), dp.len());
    assert_eq!(p.len(), dq.len());
    assert!(pbmul.len() >= NIBBLE_TABLE_LEN);
    assert!(qmul.len() >= NIBBLE_TABLE_LEN);

    for index in 0..p.len() {
        let px = p[index] ^ dp[index];
        let qx = raid6_neon_table_lookup(qmul, q[index] ^ dq[index]);
        let db = raid6_neon_table_lookup(pbmul, px) ^ qx;
        dq[index] = db;
        dp[index] = db ^ px;
    }
}

pub fn raid6_datap_recov_neon_inner(p: &mut [u8], q: &[u8], dq: &mut [u8], qmul: &[u8]) {
    assert_eq!(p.len(), q.len());
    assert_eq!(p.len(), dq.len());
    assert!(qmul.len() >= NIBBLE_TABLE_LEN);

    for index in 0..p.len() {
        let recovered = raid6_neon_table_lookup(qmul, q[index] ^ dq[index]);
        dq[index] = recovered;
        p[index] ^= recovered;
    }
}

pub fn raid6_neon_table_lookup(table: &[u8], value: u8) -> u8 {
    table[(value & 0x0f) as usize] ^ table[16 + (value >> 4) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_nibble_table() -> [u8; NIBBLE_TABLE_LEN] {
        let mut table = [0u8; NIBBLE_TABLE_LEN];
        for i in 0..16 {
            table[i] = i as u8;
            table[16 + i] = (i as u8) << 4;
        }
        table
    }

    #[test]
    fn recov_neon_inner_matches_linux_scalar_comments() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid6/recov_neon_inner.c"
        ));
        assert!(source.contains("#include <arm_neon.h>"));
        assert!(source.contains("static uint8x16_t vqtbl1q_u8"));
        assert!(source.contains("void __raid6_2data_recov_neon"));
        assert!(source.contains("px    = *p ^ *dp;"));
        assert!(source.contains("qx    = qmul[*q ^ *dq];"));
        assert!(source.contains("*dq++ = db = pbmul[px] ^ qx;"));
        assert!(source.contains("*dp++ = db ^ px;"));
        assert!(source.contains("void __raid6_datap_recov_neon"));
        assert!(source.contains("*p++ ^= *dq = qmul[*q ^ *dq];"));
        assert!(source.contains("bytes -= 16;"));

        let table = identity_nibble_table();
        let p = [1u8, 2, 3, 4];
        let q = [10u8, 20, 30, 40];
        let mut dp = [4u8, 5, 6, 7];
        let mut dq = [8u8, 9, 10, 11];
        let px0 = p[0] ^ dp[0];
        let qx0 = q[0] ^ dq[0];
        raid6_2data_recov_neon_inner(&p, &q, &mut dp, &mut dq, &table, &table);
        assert_eq!(dq[0], px0 ^ qx0);
        assert_eq!(dp[0], qx0);

        let mut p = [1u8, 2, 3, 4];
        let q = [10u8, 20, 30, 40];
        let mut dq = [8u8, 9, 10, 11];
        let recovered = q[2] ^ dq[2];
        raid6_datap_recov_neon_inner(&mut p, &q, &mut dq, &table);
        assert_eq!(dq[2], recovered);
        assert_eq!(p[2], 3 ^ recovered);
    }
}
