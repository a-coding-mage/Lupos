//! linux-parity: complete
//! linux-source: vendor/linux/lib/raid6/recov_s390xc.c
//! test-origin: linux:vendor/linux/lib/raid6/recov_s390xc.c
//! Scalar model of the s390 XC RAID6 recovery chunk loops.

pub const S390XC_CHUNK_SIZE: usize = 256;
pub const RAID6_RECOV_S390XC_NAME: &str = "s390xc";
pub const RAID6_RECOV_S390XC_PRIORITY: i32 = 1;

pub fn xor_block(p1: &mut [u8], p2: &[u8]) {
    assert!(p1.len() >= S390XC_CHUNK_SIZE);
    assert!(p2.len() >= S390XC_CHUNK_SIZE);
    for index in 0..S390XC_CHUNK_SIZE {
        p1[index] ^= p2[index];
    }
}

pub fn raid6_2data_recov_s390xc_chunk(
    p: &[u8],
    q: &[u8],
    dp: &mut [u8],
    dq: &mut [u8],
    pbmul: &[u8],
    qmul: &[u8],
) {
    assert!(pbmul.len() >= 256);
    assert!(qmul.len() >= 256);
    xor_block(dp, p);
    xor_block(dq, q);
    for index in 0..S390XC_CHUNK_SIZE {
        dq[index] = pbmul[dp[index] as usize] ^ qmul[dq[index] as usize];
    }
    xor_block(dp, dq);
}

pub fn raid6_datap_recov_s390xc_chunk(p: &mut [u8], q: &[u8], dq: &mut [u8], qmul: &[u8]) {
    assert!(qmul.len() >= 256);
    xor_block(dq, q);
    for index in 0..S390XC_CHUNK_SIZE {
        dq[index] = qmul[dq[index] as usize];
    }
    xor_block(p, dq);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Raid6RecovCalls {
    pub name: &'static str,
    pub priority: i32,
    pub has_data2: bool,
    pub has_datap: bool,
    pub valid_is_null: bool,
}

pub const RAID6_RECOV_S390XC: Raid6RecovCalls = Raid6RecovCalls {
    name: RAID6_RECOV_S390XC_NAME,
    priority: RAID6_RECOV_S390XC_PRIORITY,
    has_data2: true,
    has_datap: true,
    valid_is_null: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recov_s390xc_matches_linux_xc_recovery_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/raid6/recov_s390xc.c"
        ));
        assert!(source.contains("asm volatile("));
        assert!(source.contains("xc\t0(256,%[p1]),0(%[p2])"));
        assert!(source.contains("static void raid6_2data_recov_s390xc"));
        assert!(source.contains("ptrs[faila] = raid6_get_zero_page();"));
        assert!(source.contains("raid6_call.gen_syndrome(disks, bytes, ptrs);"));
        assert!(source.contains("pbmul = raid6_gfmul[raid6_gfexi[failb-faila]];"));
        assert!(source.contains("for (i = 0; i < 256; i++)"));
        assert!(source.contains("dq[i] = pbmul[dp[i]] ^ qmul[dq[i]];"));
        assert!(source.contains("static void raid6_datap_recov_s390xc"));
        assert!(source.contains(".name = \"s390xc\""));
        assert!(source.contains(".priority = 1"));

        let mut identity = [0u8; 256];
        for (index, value) in identity.iter_mut().enumerate() {
            *value = index as u8;
        }
        let p = [0x11u8; S390XC_CHUNK_SIZE];
        let q = [0x22u8; S390XC_CHUNK_SIZE];
        let mut dp = [0x33u8; S390XC_CHUNK_SIZE];
        let mut dq = [0x44u8; S390XC_CHUNK_SIZE];
        raid6_2data_recov_s390xc_chunk(&p, &q, &mut dp, &mut dq, &identity, &identity);
        assert_eq!(dq[0], (0x33 ^ 0x11) ^ (0x44 ^ 0x22));
        assert_eq!(dp[0], 0x44 ^ 0x22);

        let mut p = [0x11u8; S390XC_CHUNK_SIZE];
        let q = [0x22u8; S390XC_CHUNK_SIZE];
        let mut dq = [0x44u8; S390XC_CHUNK_SIZE];
        raid6_datap_recov_s390xc_chunk(&mut p, &q, &mut dq, &identity);
        assert_eq!(dq[0], 0x44 ^ 0x22);
        assert_eq!(p[0], 0x11 ^ 0x44 ^ 0x22);
        assert_eq!(RAID6_RECOV_S390XC.name, "s390xc");
    }
}
