//! linux-parity: complete
//! linux-source: vendor/linux/lib/crypto/hash_info.c
//! test-origin: linux:vendor/linux/lib/crypto/hash_info.c
//! Hash algorithm names and digest sizes.

use crate::kernel::module::{export_symbol, find_symbol};

pub const HASH_ALGO_LAST: usize = 23;

#[repr(usize)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HashAlgo {
    Md4 = 0,
    Md5 = 1,
    Sha1 = 2,
    RipeMd160 = 3,
    Sha256 = 4,
    Sha384 = 5,
    Sha512 = 6,
    Sha224 = 7,
    RipeMd128 = 8,
    RipeMd256 = 9,
    RipeMd320 = 10,
    Wp256 = 11,
    Wp384 = 12,
    Wp512 = 13,
    Tgr128 = 14,
    Tgr160 = 15,
    Tgr192 = 16,
    Sm3_256 = 17,
    Streebog256 = 18,
    Streebog512 = 19,
    Sha3_256 = 20,
    Sha3_384 = 21,
    Sha3_512 = 22,
}

pub static HASH_ALGO_NAMES: [&str; HASH_ALGO_LAST] = [
    "md4",
    "md5",
    "sha1",
    "rmd160",
    "sha256",
    "sha384",
    "sha512",
    "sha224",
    "rmd128",
    "rmd256",
    "rmd320",
    "wp256",
    "wp384",
    "wp512",
    "tgr128",
    "tgr160",
    "tgr192",
    "sm3",
    "streebog256",
    "streebog512",
    "sha3-256",
    "sha3-384",
    "sha3-512",
];

pub static HASH_DIGEST_SIZES: [usize; HASH_ALGO_LAST] = [
    16, 16, 20, 20, 32, 48, 64, 28, 16, 32, 40, 32, 48, 64, 16, 20, 24, 32, 32, 64, 32, 48, 64,
];

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

pub fn register_module_exports() {
    export_symbol_once("hash_algo_name", HASH_ALGO_NAMES.as_ptr() as usize, true);
    export_symbol_once(
        "hash_digest_size",
        HASH_DIGEST_SIZES.as_ptr() as usize,
        true,
    );
}

pub fn hash_algo_name(algo: HashAlgo) -> &'static str {
    HASH_ALGO_NAMES[algo as usize]
}

pub fn hash_digest_size(algo: HashAlgo) -> usize {
    HASH_DIGEST_SIZES[algo as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_info_tables_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/crypto/hash_info.c"
        ));
        assert!(source.contains("const char *const hash_algo_name[HASH_ALGO__LAST]"));
        assert!(source.contains("const int hash_digest_size[HASH_ALGO__LAST]"));
        assert!(source.contains("[HASH_ALGO_MD4]\t\t= \"md4\""));
        assert!(source.contains("[HASH_ALGO_SHA3_512]    = \"sha3-512\""));
        assert!(source.contains("[HASH_ALGO_MD4]\t\t= MD5_DIGEST_SIZE"));
        assert!(source.contains("[HASH_ALGO_STREEBOG_512] = STREEBOG512_DIGEST_SIZE"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(hash_algo_name);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(hash_digest_size);"));

        assert_eq!(HASH_ALGO_NAMES.len(), HASH_ALGO_LAST);
        assert_eq!(HASH_DIGEST_SIZES.len(), HASH_ALGO_LAST);
        assert_eq!(hash_algo_name(HashAlgo::Md5), "md5");
        assert_eq!(hash_algo_name(HashAlgo::Sha3_384), "sha3-384");
        assert_eq!(hash_digest_size(HashAlgo::Sha1), 20);
        assert_eq!(hash_digest_size(HashAlgo::Sha512), 64);
        assert_eq!(hash_digest_size(HashAlgo::Tgr192), 24);
    }

    #[test]
    fn hash_info_exports_register_for_modules() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("hash_algo_name"),
            Some(HASH_ALGO_NAMES.as_ptr() as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("hash_digest_size"),
            Some(HASH_DIGEST_SIZES.as_ptr() as usize)
        );
    }
}
