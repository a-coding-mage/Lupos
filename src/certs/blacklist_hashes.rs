//! linux-parity: complete
//! linux-source: vendor/linux/certs/blacklist_hashes.c
//! test-origin: linux:vendor/linux/certs/blacklist_hashes.c
//! Built-in certificate blacklist hash include list.

pub const LINUX_SOURCE: &str = "vendor/linux/certs/blacklist_hashes.c";
pub const HEADER: &str = "blacklist.h";
pub const HASH_LIST_INCLUDE: &str = "blacklist_hash_list";

pub fn blacklist_hash_includes() -> [&'static str; 2] {
    [HEADER, HASH_LIST_INCLUDE]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blacklist_hashes_source_includes_generated_list() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/certs/blacklist_hashes.c"
        ));
        assert!(source.contains("#include \"blacklist.h\""));
        assert!(source.contains("const char __initconst *const blacklist_hashes[]"));
        assert!(source.contains("#include \"blacklist_hash_list\""));
        assert_eq!(blacklist_hash_includes(), [HEADER, HASH_LIST_INCLUDE]);
    }
}
