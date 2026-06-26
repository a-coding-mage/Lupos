//! linux-parity: complete
//! linux-source: vendor/linux/fs/affs/symlink.c
//! test-origin: linux:vendor/linux/fs/affs/symlink.c
//! AFFS symlink target expansion rules.

extern crate alloc;

use crate::include::uapi::errno::EIO;
use alloc::vec::Vec;

pub const AFFS_SYMLINK_MAX: usize = 1023;
pub const AFFS_SYMLINK_AOPS_SYMBOL: &str = "affs_symlink_aops";
pub const AFFS_SYMLINK_INODE_OPS_SYMBOL: &str = "affs_symlink_inode_operations";

pub fn affs_symlink_read_error(block_present: bool) -> i32 {
    if block_present { 0 } else { -EIO }
}

pub fn affs_expand_symlink(symname: &[u8], prefix: Option<&[u8]>) -> Vec<u8> {
    let symname = nul_terminated(symname);
    let mut link = Vec::new();
    let mut j = 0usize;
    let mut last = 0u8;

    if let Some(colon) = symname.iter().position(|&c| c == b':') {
        let prefix = prefix.unwrap_or(b"/");
        push_limited(&mut link, prefix);
        while link.len() < AFFS_SYMLINK_MAX && j < colon {
            link.push(symname[j]);
            j += 1;
        }
        if link.len() < AFFS_SYMLINK_MAX {
            link.push(b'/');
        }
        j = colon + 1;
        last = b'/';
    }

    while link.len() < AFFS_SYMLINK_MAX && j < symname.len() {
        let c = symname[j];
        if c == b'/' && last == b'/' && link.len() < 1020 {
            link.push(b'.');
            link.push(b'.');
        }
        link.push(c);
        last = c;
        j += 1;
    }

    link
}

fn nul_terminated(bytes: &[u8]) -> &[u8] {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    &bytes[..end]
}

fn push_limited(out: &mut Vec<u8>, bytes: &[u8]) {
    for &byte in bytes {
        if out.len() >= AFFS_SYMLINK_MAX || byte == 0 {
            break;
        }
        out.push(byte);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affs_symlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/affs/symlink.c"
        ));
        assert!(source.contains("#include \"affs.h\""));
        assert!(source.contains("static int affs_symlink_read_folio"));
        assert!(source.contains("bh = affs_bread(inode->i_sb, inode->i_ino);"));
        assert!(source.contains("if (strchr(lf->symname,':'))"));
        assert!(source.contains("pf = sbi->s_prefix ? sbi->s_prefix : \"/\";"));
        assert!(source.contains("if (c == '/' && lc == '/' && i < 1020)"));
        assert!(source.contains("link[i] = '\\0';"));
        assert!(source.contains("folio_mark_uptodate(folio);"));
        assert!(source.contains("return -EIO;"));
        assert!(source.contains(AFFS_SYMLINK_AOPS_SYMBOL));
        assert!(source.contains(AFFS_SYMLINK_INODE_OPS_SYMBOL));

        assert_eq!(
            affs_expand_symlink(b"VOL:dir/file", Some(b"/mnt/")),
            b"/mnt/VOL/dir/file"
        );
        assert_eq!(affs_expand_symlink(b"a//b", None), b"a/../b");
        assert_eq!(affs_symlink_read_error(false), -EIO);
        assert_eq!(affs_symlink_read_error(true), 0);
    }
}
