//! linux-parity: complete
//! linux-source: vendor/linux/fs/hpfs/name.c
//! test-origin: linux:vendor/linux/fs/hpfs/name.c
//! HPFS filename validation and ordering helpers.

use crate::include::uapi::errno::{EINVAL, ENAMETOOLONG};

pub fn hpfs_not_allowed_char(c: u8) -> bool {
    c < b' '
        || c == b'"'
        || c == b'*'
        || c == b'/'
        || c == b':'
        || c == b'<'
        || c == b'>'
        || c == b'?'
        || c == b'\\'
        || c == b'|'
}

pub fn hpfs_no_dos_char(c: u8) -> bool {
    c == b'+' || c == b',' || c == b';' || c == b'=' || c == b'[' || c == b']'
}

pub fn hpfs_upcase(cp_table: Option<&[u8]>, a: u8) -> u8 {
    if a < 128 || a == 255 {
        if a.is_ascii_lowercase() { a - 0x20 } else { a }
    } else if let Some(table) = cp_table {
        table.get((a - 128) as usize).copied().unwrap_or(a)
    } else {
        a
    }
}

pub fn hpfs_locase(cp_table: Option<&[u8]>, a: u8) -> u8 {
    if a < 128 || a == 255 {
        if a.is_ascii_uppercase() { a + 0x20 } else { a }
    } else if let Some(table) = cp_table {
        table.get(a as usize).copied().unwrap_or(a)
    } else {
        a
    }
}

pub fn hpfs_adjust_length(name: &[u8], len: &mut usize) {
    if *len == 0 {
        return;
    }
    if *len == 1 && name.first() == Some(&b'.') {
        return;
    }
    if *len == 2 && name.first() == Some(&b'.') && name.get(1) == Some(&b'.') {
        return;
    }
    while *len != 0 && (name[*len - 1] == b'.' || name[*len - 1] == b' ') {
        *len -= 1;
    }
}

pub fn hpfs_chk_name(name: &[u8], len: &mut usize) -> Result<(), i32> {
    if *len > 254 {
        return Err(-ENAMETOOLONG);
    }
    hpfs_adjust_length(name, len);
    if *len == 0 {
        return Err(-EINVAL);
    }
    for &c in name.iter().take(*len) {
        if hpfs_not_allowed_char(c) {
            return Err(-EINVAL);
        }
    }
    if *len == 1 && name[0] == b'.' {
        return Err(-EINVAL);
    }
    if *len == 2 && name[0] == b'.' && name[1] == b'.' {
        return Err(-EINVAL);
    }
    Ok(())
}

pub fn hpfs_compare_names(cp_table: Option<&[u8]>, n1: &[u8], n2: &[u8], last: bool) -> i32 {
    if last {
        return -1;
    }

    for (&a, &b) in n1.iter().zip(n2.iter()) {
        let c1 = hpfs_upcase(cp_table, a);
        let c2 = hpfs_upcase(cp_table, b);
        if c1 < c2 {
            return -1;
        }
        if c1 > c2 {
            return 1;
        }
    }
    if n1.len() < n2.len() {
        -1
    } else if n1.len() > n2.len() {
        1
    } else {
        0
    }
}

pub fn hpfs_is_name_long(name: &[u8]) -> bool {
    let mut i = 0usize;
    while i < name.len() && name[i] != b'.' {
        if hpfs_no_dos_char(name[i]) {
            return true;
        }
        i += 1;
    }
    if i == 0 || i > 8 {
        return true;
    }
    if i == name.len() {
        return false;
    }
    let mut j = i + 1;
    while j < name.len() {
        if name[j] == b'.' || hpfs_no_dos_char(name[i]) {
            return true;
        }
        j += 1;
    }
    j - i > 4
}

pub fn hpfs_locase_name(cp_table: Option<&[u8]>, name: &[u8], out: &mut [u8]) -> usize {
    let len = core::cmp::min(name.len(), out.len());
    for i in 0..len {
        out[i] = hpfs_locase(cp_table, name[i]);
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hpfs_name_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/hpfs/name.c"
        ));
        assert!(source.contains("#include \"hpfs_fn.h\""));
        assert!(source.contains("static inline int not_allowed_char(unsigned char c)"));
        assert!(source.contains("return c<' ' || c=='\"' || c=='*' || c=='/' || c==':'"));
        assert!(source.contains("static inline int no_dos_char(unsigned char c)"));
        assert!(
            source.contains(
                "static inline unsigned char upcase(unsigned char *dir, unsigned char a)"
            )
        );
        assert!(source.contains("unsigned char hpfs_upcase(unsigned char *dir, unsigned char a)"));
        assert!(
            source.contains(
                "static inline unsigned char locase(unsigned char *dir, unsigned char a)"
            )
        );
        assert!(source.contains("int hpfs_chk_name(const unsigned char *name, unsigned *len)"));
        assert!(source.contains("if (*len > 254) return -ENAMETOOLONG;"));
        assert!(source.contains("hpfs_adjust_length(name, len);"));
        assert!(source.contains("if (!*len) return -EINVAL;"));
        assert!(source.contains("unsigned char *hpfs_translate_name"));
        assert!(source.contains("int hpfs_compare_names(struct super_block *s,"));
        assert!(source.contains("if (last) return -1;"));
        assert!(source.contains("int hpfs_is_name_long(const unsigned char *name, unsigned len)"));
        assert!(
            source.contains("void hpfs_adjust_length(const unsigned char *name, unsigned *len)")
        );

        assert!(hpfs_not_allowed_char(b':'));
        assert!(!hpfs_not_allowed_char(b'A'));
        assert!(hpfs_no_dos_char(b'+'));
        assert_eq!(hpfs_upcase(None, b'a'), b'A');
        assert_eq!(hpfs_locase(None, b'Z'), b'z');

        let mut len = b"file.txt.  ".len();
        hpfs_adjust_length(b"file.txt.  ", &mut len);
        assert_eq!(len, b"file.txt".len());
        assert_eq!(hpfs_chk_name(b"file.txt", &mut 8), Ok(()));
        assert_eq!(hpfs_chk_name(b".", &mut 1), Err(-EINVAL));
        assert_eq!(hpfs_chk_name(b"bad:name", &mut 8), Err(-EINVAL));
        assert_eq!(hpfs_chk_name(&[b'a'; 255], &mut 255), Err(-ENAMETOOLONG));

        assert_eq!(hpfs_compare_names(None, b"abc", b"ABC", false), 0);
        assert_eq!(hpfs_compare_names(None, b"abc", b"abd", false), -1);
        assert_eq!(hpfs_compare_names(None, b"abcd", b"abc", false), 1);
        assert_eq!(hpfs_compare_names(None, b"abc", b"abc", true), -1);
        assert!(!hpfs_is_name_long(b"FILE.TXT"));
        assert!(hpfs_is_name_long(b"TOOLONGNAM.TXT"));
        assert!(hpfs_is_name_long(b"FILE+ONE"));

        let mut out = [0u8; 3];
        assert_eq!(hpfs_locase_name(None, b"ABC", &mut out), 3);
        assert_eq!(&out, b"abc");
    }
}
