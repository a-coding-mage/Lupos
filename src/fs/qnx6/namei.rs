//! linux-parity: complete
//! linux-source: vendor/linux/fs/qnx6/namei.c
//! test-origin: linux:vendor/linux/fs/qnx6/namei.c
//! QNX6 lookup name-length guard and inode lookup flow.

use crate::include::uapi::errno::ENAMETOOLONG;

pub const QNX6_LONG_NAME_MAX: usize = 510;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Qnx6LookupOutcome {
    pub error: i32,
    pub find_ino_called: bool,
    pub iget_ino: Option<u32>,
    pub splice_alias_called: bool,
}

pub const fn qnx6_lookup_outcome(name_len: usize, found_ino: u32) -> Qnx6LookupOutcome {
    if name_len > QNX6_LONG_NAME_MAX {
        return Qnx6LookupOutcome {
            error: -ENAMETOOLONG,
            find_ino_called: false,
            iget_ino: None,
            splice_alias_called: false,
        };
    }

    Qnx6LookupOutcome {
        error: 0,
        find_ino_called: true,
        iget_ino: if found_ino == 0 {
            None
        } else {
            Some(found_ino)
        },
        splice_alias_called: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qnx6_lookup_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/qnx6/namei.c"
        ));
        assert!(source.contains("#include \"qnx6.h\""));
        assert!(source.contains("len > QNX6_LONG_NAME_MAX"));
        assert!(source.contains("return ERR_PTR(-ENAMETOOLONG);"));
        assert!(source.contains("ino = qnx6_find_ino(len, dir, name);"));
        assert!(source.contains("foundinode = qnx6_iget(dir->i_sb, ino);"));
        assert!(source.contains("return d_splice_alias(foundinode, dentry);"));

        assert_eq!(QNX6_LONG_NAME_MAX, 510);
        assert_eq!(
            qnx6_lookup_outcome(QNX6_LONG_NAME_MAX + 1, 7),
            Qnx6LookupOutcome {
                error: -ENAMETOOLONG,
                find_ino_called: false,
                iget_ino: None,
                splice_alias_called: false,
            }
        );
        assert_eq!(
            qnx6_lookup_outcome(4, 0),
            Qnx6LookupOutcome {
                error: 0,
                find_ino_called: true,
                iget_ino: None,
                splice_alias_called: true,
            }
        );
        assert_eq!(qnx6_lookup_outcome(4, 42).iget_ino, Some(42));
    }
}
