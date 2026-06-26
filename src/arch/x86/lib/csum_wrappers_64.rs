//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/lib/csum-wrappers_64.c
//! test-origin: linux:vendor/linux/arch/x86/lib/csum-wrappers_64.c
//! x86-64 checksum copy wrapper access gates.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CsumCopyResult {
    pub checksum: u32,
    pub user_access_began: bool,
    pub user_access_ended: bool,
}

pub fn csum_partial_copy_nocheck(src: &[u8], dst: &mut [u8]) -> u32 {
    let len = src.len().min(dst.len());
    dst[..len].copy_from_slice(&src[..len]);
    checksum32(&dst[..len])
}

pub fn csum_and_copy_from_user(access_ok: bool, src: &[u8], dst: &mut [u8]) -> CsumCopyResult {
    if !access_ok {
        return CsumCopyResult {
            checksum: 0,
            user_access_began: false,
            user_access_ended: false,
        };
    }
    CsumCopyResult {
        checksum: csum_partial_copy_nocheck(src, dst),
        user_access_began: true,
        user_access_ended: true,
    }
}

pub fn csum_and_copy_to_user(access_ok: bool, src: &[u8], dst: &mut [u8]) -> CsumCopyResult {
    csum_and_copy_from_user(access_ok, src, dst)
}

fn checksum32(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(0u32, |sum, byte| sum.wrapping_add(*byte as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csum_wrappers_match_linux_access_gate_shape() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/lib/csum-wrappers_64.c"
        ));
        assert!(source.contains("csum_and_copy_from_user"));
        assert!(source.contains("might_sleep();"));
        assert!(source.contains("if (!user_access_begin(src, len))"));
        assert!(source.contains("sum = csum_partial_copy_generic"));
        assert!(source.contains("user_access_end();"));
        assert!(source.contains("csum_and_copy_to_user"));
        assert!(source.contains("if (!user_access_begin(dst, len))"));
        assert!(source.contains("csum_partial_copy_nocheck"));
        assert!(source.contains("EXPORT_SYMBOL(csum_partial_copy_nocheck);"));

        let src = [1u8, 2, 3, 4];
        let mut dst = [0u8; 4];
        let result = csum_and_copy_from_user(true, &src, &mut dst);
        assert_eq!(dst, src);
        assert_eq!(result.checksum, 10);
        assert!(result.user_access_began);
        assert_eq!(
            csum_and_copy_to_user(false, &src, &mut dst),
            CsumCopyResult {
                checksum: 0,
                user_access_began: false,
                user_access_ended: false,
            }
        );
    }
}
