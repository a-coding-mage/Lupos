//! linux-parity: complete
//! linux-source: vendor/linux/fs/coda/symlink.c
//! test-origin: linux:vendor/linux/fs/coda/symlink.c
//! Coda symlink folio filler behavior.

pub const CODA_SYMLINK_AOPS_SYMBOL: &str = "coda_symlink_aops";
pub const CODA_SYMLINK_READ_FOLIO: &str = "coda_symlink_filler";
pub const CODA_SYMLINK_AOPS_SLOT: &str = "read_folio";
pub const CODA_READLINK_LEN: usize = 4096;
pub const CODA_SYMLINK_INODE_EXPR: &str = "folio->mapping->host";
pub const CODA_SYMLINK_CII_EXPR: &str = "ITOC(inode)";
pub const CODA_SYMLINK_BUFFER_EXPR: &str = "folio_address(folio)";
pub const CODA_SYMLINK_UPCALL: &str = "venus_readlink";
pub const CODA_SYMLINK_COMPLETE_READ: &str = "folio_end_read";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodaSymlinkFillerModel {
    pub file_arg: &'static str,
    pub folio_arg: &'static str,
    pub inode_expr: &'static str,
    pub cii_expr: &'static str,
    pub initial_len: usize,
    pub buffer_expr: &'static str,
    pub upcall: &'static str,
    pub complete_read: &'static str,
    pub success_when_error_zero: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodaAddressSpaceOperations {
    pub symbol: &'static str,
    pub read_folio: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodaSymlinkReadOutcome {
    pub requested_len: usize,
    pub folio_end_read_success: bool,
    pub returned_error: i32,
}

pub const CODA_SYMLINK_FILLER_MODEL: CodaSymlinkFillerModel = CodaSymlinkFillerModel {
    file_arg: "file",
    folio_arg: "folio",
    inode_expr: CODA_SYMLINK_INODE_EXPR,
    cii_expr: CODA_SYMLINK_CII_EXPR,
    initial_len: CODA_READLINK_LEN,
    buffer_expr: CODA_SYMLINK_BUFFER_EXPR,
    upcall: CODA_SYMLINK_UPCALL,
    complete_read: CODA_SYMLINK_COMPLETE_READ,
    success_when_error_zero: true,
};

pub const CODA_SYMLINK_AOPS: CodaAddressSpaceOperations = CodaAddressSpaceOperations {
    symbol: CODA_SYMLINK_AOPS_SYMBOL,
    read_folio: CODA_SYMLINK_READ_FOLIO,
};

pub const fn coda_symlink_aops() -> CodaAddressSpaceOperations {
    CODA_SYMLINK_AOPS
}

pub const fn coda_symlink_filler_outcome(venus_readlink_ret: i32) -> CodaSymlinkReadOutcome {
    CodaSymlinkReadOutcome {
        requested_len: CODA_READLINK_LEN,
        folio_end_read_success: venus_readlink_ret == 0,
        returned_error: venus_readlink_ret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coda_symlink_filler_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/coda/symlink.c"
        ));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("#include <linux/coda.h>"));
        assert!(source.contains("#include \"coda_psdev.h\""));
        assert!(source.contains("#include \"coda_linux.h\""));
        assert!(source.contains("static int coda_symlink_filler"));
        assert!(source.contains("struct inode *inode = folio->mapping->host;"));
        assert!(source.contains("int error;"));
        assert!(source.contains("struct coda_inode_info *cii;"));
        assert!(source.contains("unsigned int len = PAGE_SIZE;"));
        assert!(source.contains("char *p = folio_address(folio);"));
        assert!(source.contains("cii = ITOC(inode);"));
        assert!(source.contains("venus_readlink(inode->i_sb, &cii->c_fid, p, &len);"));
        assert!(source.contains("folio_end_read(folio, error == 0);"));
        assert!(source.contains("return error;"));
        assert!(source.contains("const struct address_space_operations coda_symlink_aops"));
        assert!(source.contains(".read_folio\t= coda_symlink_filler"));
        assert_eq!(
            CODA_SYMLINK_FILLER_MODEL,
            CodaSymlinkFillerModel {
                file_arg: "file",
                folio_arg: "folio",
                inode_expr: "folio->mapping->host",
                cii_expr: "ITOC(inode)",
                initial_len: CODA_READLINK_LEN,
                buffer_expr: "folio_address(folio)",
                upcall: "venus_readlink",
                complete_read: "folio_end_read",
                success_when_error_zero: true,
            }
        );
        assert_eq!(
            coda_symlink_aops(),
            CodaAddressSpaceOperations {
                symbol: "coda_symlink_aops",
                read_folio: "coda_symlink_filler",
            }
        );

        assert_eq!(
            coda_symlink_filler_outcome(0),
            CodaSymlinkReadOutcome {
                requested_len: CODA_READLINK_LEN,
                folio_end_read_success: true,
                returned_error: 0,
            }
        );
        assert_eq!(
            coda_symlink_filler_outcome(-5),
            CodaSymlinkReadOutcome {
                requested_len: CODA_READLINK_LEN,
                folio_end_read_success: false,
                returned_error: -5,
            }
        );
    }
}
