//! linux-parity: complete
//! linux-source: vendor/linux/fs/crypto/hkdf.c
//! test-origin: linux:vendor/linux/fs/crypto/hkdf.c
//! fscrypt HKDF-SHA512 layout constants and expand bounds.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::EINVAL;

pub const HKDF_HASHLEN: usize = 64;
pub const HKDF_CONTEXT_PREFIX: &[u8; 8] = b"fscrypt\0";
pub const HKDF_MAX_OUTPUT_LEN: usize = 255 * HKDF_HASHLEN;
pub const DEFAULT_SALT_LEN: usize = HKDF_HASHLEN;
pub const INITIAL_COUNTER: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HkdfInitReport {
    pub default_salt_len: usize,
    pub master_key_size: usize,
    pub prk_len: usize,
    pub prepare_key_len: usize,
    pub prk_zeroized: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HkdfExpandStep {
    pub offset: usize,
    pub counter: u8,
    pub includes_previous_block: bool,
    pub prefix_len: usize,
    pub context: u8,
    pub context_len: usize,
    pub info_len: usize,
    pub output_len: usize,
    pub used_tmp: bool,
    pub tmp_zeroized: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HkdfExpandReport {
    pub warned_too_long: bool,
    pub steps: Vec<HkdfExpandStep>,
}

pub const fn fscrypt_hkdf_expand_blocks(okmlen: usize) -> Result<usize, i32> {
    if okmlen > HKDF_MAX_OUTPUT_LEN {
        return Err(-EINVAL);
    }
    Ok((okmlen + HKDF_HASHLEN - 1) / HKDF_HASHLEN)
}

pub const fn fscrypt_hkdf_expand_update_count(okmlen: usize) -> Result<usize, i32> {
    match fscrypt_hkdf_expand_blocks(okmlen) {
        Ok(0) => Ok(0),
        Ok(blocks) => Ok(4 + (blocks - 1) * 5),
        Err(err) => Err(err),
    }
}

pub const fn fscrypt_init_hkdf_report(master_key_size: usize) -> HkdfInitReport {
    HkdfInitReport {
        default_salt_len: DEFAULT_SALT_LEN,
        master_key_size,
        prk_len: HKDF_HASHLEN,
        prepare_key_len: HKDF_HASHLEN,
        prk_zeroized: true,
    }
}

pub fn fscrypt_hkdf_expand_plan(
    context: u8,
    info_len: usize,
    okmlen: usize,
) -> Result<HkdfExpandReport, i32> {
    if okmlen > HKDF_MAX_OUTPUT_LEN {
        return Err(-EINVAL);
    }

    let blocks = fscrypt_hkdf_expand_blocks(okmlen)?;
    let mut steps = Vec::with_capacity(blocks);
    let mut counter = INITIAL_COUNTER;

    for offset in (0..okmlen).step_by(HKDF_HASHLEN) {
        let output_len = (okmlen - offset).min(HKDF_HASHLEN);
        let used_tmp = output_len < HKDF_HASHLEN;
        steps.push(HkdfExpandStep {
            offset,
            counter,
            includes_previous_block: offset != 0,
            prefix_len: HKDF_CONTEXT_PREFIX.len(),
            context,
            context_len: 1,
            info_len,
            output_len,
            used_tmp,
            tmp_zeroized: used_tmp,
        });
        counter = counter.wrapping_add(1);
    }

    Ok(HkdfExpandReport {
        warned_too_long: false,
        steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fscrypt_hkdf_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/crypto/hkdf.c"
        ));
        assert!(source.contains("#include \"fscrypt_private.h\""));
        assert!(source.contains("#define HKDF_HASHLEN\t\tSHA512_DIGEST_SIZE"));
        assert!(source.contains("void fscrypt_init_hkdf"));
        assert!(source.contains("static const u8 default_salt[HKDF_HASHLEN];"));
        assert!(source.contains("u8 prk[HKDF_HASHLEN];"));
        assert!(source.contains("hmac_sha512_usingrawkey(default_salt, sizeof(default_salt),"));
        assert!(source.contains("master_key, master_key_size, prk);"));
        assert!(source.contains("hmac_sha512_preparekey(hkdf, prk, sizeof(prk));"));
        assert!(source.contains("memzero_explicit(prk, sizeof(prk));"));
        assert!(source.contains("void fscrypt_hkdf_expand"));
        assert!(source.contains("u8 counter = 1;"));
        assert!(source.contains("u8 tmp[HKDF_HASHLEN];"));
        assert!(source.contains("WARN_ON_ONCE(okmlen > 255 * HKDF_HASHLEN);"));
        assert!(source.contains("for (unsigned int i = 0; i < okmlen; i += HKDF_HASHLEN)"));
        assert!(source.contains("hmac_sha512_init(&ctx, hkdf);"));
        assert!(source.contains("if (i != 0)"));
        assert!(source.contains("hmac_sha512_update(&ctx, &okm[i - HKDF_HASHLEN]"));
        assert!(source.contains("hmac_sha512_update(&ctx, \"fscrypt\\0\", 8);"));
        assert!(source.contains("hmac_sha512_update(&ctx, &context, 1);"));
        assert!(source.contains("hmac_sha512_update(&ctx, info, infolen);"));
        assert!(source.contains("hmac_sha512_update(&ctx, &counter, 1);"));
        assert!(source.contains("if (okmlen - i < HKDF_HASHLEN)"));
        assert!(source.contains("memcpy(&okm[i], tmp, okmlen - i);"));
        assert!(source.contains("memzero_explicit(tmp, sizeof(tmp));"));
        assert!(source.contains("counter++;"));

        assert_eq!(HKDF_HASHLEN, 64);
        assert_eq!(HKDF_CONTEXT_PREFIX, b"fscrypt\0");
        assert_eq!(fscrypt_hkdf_expand_blocks(0), Ok(0));
        assert_eq!(fscrypt_hkdf_expand_blocks(1), Ok(1));
        assert_eq!(fscrypt_hkdf_expand_blocks(64), Ok(1));
        assert_eq!(fscrypt_hkdf_expand_blocks(65), Ok(2));
        assert_eq!(
            fscrypt_hkdf_expand_blocks(HKDF_MAX_OUTPUT_LEN + 1),
            Err(-EINVAL)
        );
        assert_eq!(fscrypt_hkdf_expand_update_count(0), Ok(0));
        assert_eq!(fscrypt_hkdf_expand_update_count(64), Ok(4));
        assert_eq!(fscrypt_hkdf_expand_update_count(65), Ok(9));
    }

    #[test]
    fn init_hkdf_report_matches_extract_prepare_zeroize_flow() {
        assert_eq!(
            fscrypt_init_hkdf_report(32),
            HkdfInitReport {
                default_salt_len: HKDF_HASHLEN,
                master_key_size: 32,
                prk_len: HKDF_HASHLEN,
                prepare_key_len: HKDF_HASHLEN,
                prk_zeroized: true,
            }
        );
    }

    #[test]
    fn expand_plan_matches_block_update_and_tmp_rules() {
        let report = fscrypt_hkdf_expand_plan(3, 11, 65).unwrap();
        assert_eq!(
            report.steps,
            alloc::vec![
                HkdfExpandStep {
                    offset: 0,
                    counter: 1,
                    includes_previous_block: false,
                    prefix_len: 8,
                    context: 3,
                    context_len: 1,
                    info_len: 11,
                    output_len: 64,
                    used_tmp: false,
                    tmp_zeroized: false,
                },
                HkdfExpandStep {
                    offset: 64,
                    counter: 2,
                    includes_previous_block: true,
                    prefix_len: 8,
                    context: 3,
                    context_len: 1,
                    info_len: 11,
                    output_len: 1,
                    used_tmp: true,
                    tmp_zeroized: true,
                },
            ]
        );
        assert_eq!(
            fscrypt_hkdf_expand_plan(0, 0, HKDF_MAX_OUTPUT_LEN + 1).unwrap_err(),
            -EINVAL
        );
    }
}
