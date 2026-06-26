//! linux-parity: complete
//! linux-source: vendor/linux/fs/lockd/procfs.c
//! test-origin: linux:vendor/linux/fs/lockd/procfs.c
//! lockd procfs `nlm_end_grace` control file.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const LOCKD_PROC_DIR: &str = "fs/lockd";
pub const LOCKD_END_GRACE_FILE: &str = "nlm_end_grace";
pub const LOCKD_END_GRACE_MODE: u16 = 0o644;

pub fn nlm_end_grace_write_accepts(data: &[u8]) -> Result<(), i32> {
    match data.first().copied() {
        Some(b'Y' | b'y' | b'1') => Ok(()),
        Some(_) => Err(-EINVAL),
        None => Err(-EINVAL),
    }
}

pub const fn nlm_end_grace_read_response(grace_list_empty: bool) -> [u8; 3] {
    [if grace_list_empty { b'Y' } else { b'N' }, b'\n', b'\0']
}

pub const fn lockd_create_procfs_result(mkdir_ok: bool, proc_create_ok: bool) -> Result<(), i32> {
    if !mkdir_ok || !proc_create_ok {
        Err(-ENOMEM)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockd_procfs_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/lockd/procfs.c"
        ));
        assert!(source.contains("#include <linux/proc_fs.h>"));
        assert!(source.contains("#include \"netns.h\""));
        assert!(source.contains("#include \"procfs.h\""));
        assert!(source.contains("nlm_end_grace_write"));
        assert!(source.contains("if (size < 1)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("case 'Y':"));
        assert!(source.contains("case 'y':"));
        assert!(source.contains("case '1':"));
        assert!(source.contains("locks_end_grace(&ln->lockd_manager);"));
        assert!(source.contains("resp[0] = list_empty(&ln->lockd_manager.list) ? 'Y' : 'N';"));
        assert!(source.contains("proc_mkdir(\"fs/lockd\", NULL);"));
        assert!(source.contains("proc_create(\"nlm_end_grace\", S_IRUGO|S_IWUSR, entry,"));
        assert!(source.contains("remove_proc_entry(\"fs/lockd/nlm_end_grace\", NULL);"));
        assert!(source.contains("remove_proc_entry(\"fs/lockd\", NULL);"));

        assert_eq!(nlm_end_grace_write_accepts(b"Y"), Ok(()));
        assert_eq!(nlm_end_grace_write_accepts(b"1\n"), Ok(()));
        assert_eq!(nlm_end_grace_write_accepts(b"N"), Err(-EINVAL));
        assert_eq!(nlm_end_grace_read_response(true), [b'Y', b'\n', b'\0']);
        assert_eq!(nlm_end_grace_read_response(false), [b'N', b'\n', b'\0']);
        assert_eq!(lockd_create_procfs_result(false, true), Err(-ENOMEM));
        assert_eq!(lockd_create_procfs_result(true, false), Err(-ENOMEM));
        assert_eq!(lockd_create_procfs_result(true, true), Ok(()));
    }
}
