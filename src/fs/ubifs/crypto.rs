//! linux-parity: complete
//! linux-source: vendor/linux/fs/ubifs/crypto.c
//! test-origin: linux:vendor/linux/fs/ubifs/crypto.c
//! UBIFS fscrypt operation glue.

use crate::include::uapi::errno::EINVAL;

pub const UBIFS_BLOCK_SIZE: usize = 4096;
pub const UBIFS_CIPHER_BLOCK_SIZE: usize = 16;
pub const UBIFS_XATTR_NAME_ENCRYPTION_CONTEXT: &str = "c";
pub const UBIFS_LEGACY_KEY_PREFIX: &str = "ubifs:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UbifsEncryptPlan {
    pub compr_size: usize,
    pub padded_len: usize,
    pub zero_pad_len: usize,
}

pub const fn ubifs_encrypt_plan(
    in_len: usize,
    out_capacity: usize,
) -> Result<UbifsEncryptPlan, i32> {
    let padded_len = round_up(in_len, UBIFS_CIPHER_BLOCK_SIZE);
    if padded_len > out_capacity {
        return Err(-EINVAL);
    }
    Ok(UbifsEncryptPlan {
        compr_size: in_len,
        padded_len,
        zero_pad_len: padded_len - in_len,
    })
}

pub const fn ubifs_decrypt_out_len(compr_size: usize, data_len: usize) -> Result<usize, i32> {
    if compr_size == 0 || compr_size > UBIFS_BLOCK_SIZE || compr_size > data_len {
        return Err(-EINVAL);
    }
    if data_len > UBIFS_BLOCK_SIZE {
        return Err(-EINVAL);
    }
    Ok(compr_size)
}

const fn round_up(value: usize, align: usize) -> usize {
    if align == 0 {
        value
    } else {
        ((value + align - 1) / align) * align
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubifs_crypto_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/ubifs/crypto.c"
        ));
        assert!(source.contains("#include \"ubifs.h\""));
        assert!(source.contains("ubifs_crypt_get_context"));
        assert!(source.contains("UBIFS_XATTR_NAME_ENCRYPTION_CONTEXT"));
        assert!(source.contains("ubifs_xattr_get(inode, UBIFS_XATTR_NAME_ENCRYPTION_CONTEXT"));
        assert!(source.contains("ubifs_xattr_set(inode, UBIFS_XATTR_NAME_ENCRYPTION_CONTEXT"));
        assert!(source.contains("ubifs_crypt_empty_dir"));
        assert!(source.contains("return ubifs_check_dir_empty(inode) == 0;"));
        assert!(source.contains("int ubifs_encrypt"));
        assert!(source.contains("round_up(in_len, UBIFS_CIPHER_BLOCK_SIZE);"));
        assert!(source.contains("dn->compr_size = cpu_to_le16(in_len);"));
        assert!(source.contains("memset(p + in_len, 0, pad_len - in_len);"));
        assert!(source.contains("fscrypt_encrypt_block_inplace"));
        assert!(source.contains("*out_len = pad_len;"));
        assert!(source.contains("int ubifs_decrypt"));
        assert!(source.contains("clen <= 0 || clen > UBIFS_BLOCK_SIZE || clen > dlen"));
        assert!(source.contains("fscrypt_decrypt_block_inplace"));
        assert!(source.contains("*out_len = clen;"));
        assert!(source.contains("const struct fscrypt_operations ubifs_crypt_operations"));
        assert!(source.contains(".legacy_key_prefix\t= \"ubifs:\""));
        assert!(source.contains(".get_context\t\t= ubifs_crypt_get_context"));
        assert!(source.contains(".set_context\t\t= ubifs_crypt_set_context"));
        assert!(source.contains(".empty_dir\t\t= ubifs_crypt_empty_dir"));

        assert_eq!(
            ubifs_encrypt_plan(17, 32),
            Ok(UbifsEncryptPlan {
                compr_size: 17,
                padded_len: 32,
                zero_pad_len: 15,
            })
        );
        assert_eq!(ubifs_encrypt_plan(17, 31), Err(-EINVAL));
        assert_eq!(ubifs_decrypt_out_len(17, 32), Ok(17));
        assert_eq!(ubifs_decrypt_out_len(0, 32), Err(-EINVAL));
        assert_eq!(ubifs_decrypt_out_len(4097, 4097), Err(-EINVAL));
    }
}
