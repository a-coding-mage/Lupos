//! linux-parity: complete
//! linux-source: vendor/linux/fs/hpfs/dentry.c
//! test-origin: linux:vendor/linux/fs/hpfs/dentry.c
//! HPFS dentry hashing and comparison rules.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::{EINVAL, ENAMETOOLONG};

pub const HPFS_DENTRY_OPERATIONS_SYMBOL: &str = "hpfs_dentry_operations";

pub fn hpfs_hash_input(name: &[u8]) -> Vec<u8> {
    let len = hpfs_hash_len(name);
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    while i < len {
        out.push(hpfs_upcase_ascii(name[i]));
        i += 1;
    }
    out
}

pub fn hpfs_compare_dentry_mismatch(existing: &[u8], candidate: &[u8]) -> bool {
    let existing_len = hpfs_adjust_len(existing);
    let candidate_len = match hpfs_chk_name(candidate) {
        Ok(len) => len,
        Err(_) => return true,
    };
    hpfs_compare_names(existing, existing_len, candidate, candidate_len) != 0
}

pub const fn hpfs_hash_len(name: &[u8]) -> usize {
    let len = name.len();
    if len == 1 && name[0] == b'.' {
        return len;
    }
    if len == 2 && (name[0] == b'.' || name[1] == b'.') {
        return len;
    }
    hpfs_adjust_len(name)
}

pub const fn hpfs_adjust_len(name: &[u8]) -> usize {
    let mut len = name.len();
    if len == 0 {
        return 0;
    }
    if len == 1 && name[0] == b'.' {
        return len;
    }
    if len == 2 && name[0] == b'.' && name[1] == b'.' {
        return len;
    }
    while len != 0 && (name[len - 1] == b'.' || name[len - 1] == b' ') {
        len -= 1;
    }
    len
}

pub const fn hpfs_chk_name(name: &[u8]) -> Result<usize, i32> {
    if name.len() > 254 {
        return Err(-ENAMETOOLONG);
    }
    let len = hpfs_adjust_len(name);
    if len == 0 {
        return Err(-EINVAL);
    }
    let mut i = 0;
    while i < len {
        if hpfs_not_allowed_char(name[i]) {
            return Err(-EINVAL);
        }
        i += 1;
    }
    if len == 1 && name[0] == b'.' {
        return Err(-EINVAL);
    }
    if len == 2 && name[0] == b'.' && name[1] == b'.' {
        return Err(-EINVAL);
    }
    Ok(len)
}

pub const fn hpfs_upcase_ascii(byte: u8) -> u8 {
    if byte >= b'a' && byte <= b'z' {
        byte - 0x20
    } else {
        byte
    }
}

const fn hpfs_not_allowed_char(byte: u8) -> bool {
    byte < b' '
        || byte == b'"'
        || byte == b'*'
        || byte == b'/'
        || byte == b':'
        || byte == b'<'
        || byte == b'>'
        || byte == b'?'
        || byte == b'\\'
        || byte == b'|'
}

fn hpfs_compare_names(n1: &[u8], l1: usize, n2: &[u8], l2: usize) -> i32 {
    let l = if l1 < l2 { l1 } else { l2 };
    let mut i = 0;
    while i < l {
        let c1 = hpfs_upcase_ascii(n1[i]);
        let c2 = hpfs_upcase_ascii(n2[i]);
        if c1 < c2 {
            return -1;
        }
        if c1 > c2 {
            return 1;
        }
        i += 1;
    }
    if l1 < l2 {
        -1
    } else if l1 > l2 {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hpfs_dentry_rules_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hpfs/dentry.c"
        ));
        assert!(source.contains("#include \"hpfs_fn.h\""));
        assert!(source.contains("static int hpfs_hash_dentry"));
        assert!(source.contains("if (l == 1) if (qstr->name[0]=='.') goto x;"));
        assert!(
            source.contains("if (l == 2) if (qstr->name[0]=='.' || qstr->name[1]=='.') goto x;")
        );
        assert!(source.contains("hpfs_adjust_length(qstr->name, &l);"));
        assert!(source.contains("partial_name_hash(hpfs_upcase"));
        assert!(source.contains("static int hpfs_compare_dentry"));
        assert!(source.contains("hpfs_adjust_length(str, &al);"));
        assert!(source.contains("if (hpfs_chk_name(name->name, &bl))"));
        assert!(
            source.contains("if (hpfs_compare_names(dentry->d_sb, str, al, name->name, bl, 0))")
        );
        assert!(source.contains(HPFS_DENTRY_OPERATIONS_SYMBOL));
        assert!(source.contains(".d_hash\t\t= hpfs_hash_dentry"));
        assert!(source.contains(".d_compare\t= hpfs_compare_dentry"));

        assert_eq!(hpfs_hash_input(b"readme.txt.. "), b"README.TXT".to_vec());
        assert_eq!(hpfs_hash_input(b"."), b".".to_vec());
        assert_eq!(hpfs_hash_input(b"a."), b"A.".to_vec());
        assert_eq!(hpfs_chk_name(b"bad/name"), Err(-EINVAL));
        assert_eq!(hpfs_chk_name(b".."), Err(-EINVAL));
        assert!(!hpfs_compare_dentry_mismatch(
            b"Readme.TXT. ",
            b"readme.txt"
        ));
        assert!(hpfs_compare_dentry_mismatch(b"Readme.TXT", b"other"));
    }
}
