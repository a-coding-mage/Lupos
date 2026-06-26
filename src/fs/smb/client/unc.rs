//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/unc.c
//! test-origin: linux:vendor/linux/fs/smb/client/unc.c
//! SMB UNC host and share extraction.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UncAllocationKind {
    Kmalloc,
    Kstrdup,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UncCopyPlan<'a> {
    pub value: &'a str,
    pub allocation: UncAllocationKind,
    pub allocated_len: usize,
    pub copied_len: usize,
    pub nul_terminated: bool,
}

pub fn extract_hostname_plan(
    unc: &str,
    allocation_available: bool,
) -> Result<UncCopyPlan<'_>, i32> {
    if unc.len() < 3 {
        return Err(-EINVAL);
    }

    let src = unc.trim_start_matches('\\');
    if src.is_empty() {
        return Err(-EINVAL);
    }

    let Some(delim) = src.find('\\') else {
        return Err(-EINVAL);
    };

    if !allocation_available {
        return Err(-ENOMEM);
    }

    let value = &src[..delim];
    Ok(UncCopyPlan {
        value,
        allocation: UncAllocationKind::Kmalloc,
        allocated_len: value.len() + 1,
        copied_len: value.len(),
        nul_terminated: true,
    })
}

pub fn extract_hostname(unc: &str) -> Result<&str, i32> {
    extract_hostname_plan(unc, true).map(|plan| plan.value)
}

pub fn extract_sharename_plan(
    unc: &str,
    allocation_available: bool,
) -> Result<UncCopyPlan<'_>, i32> {
    let Some(src) = unc.get(2..) else {
        return Err(-EINVAL);
    };

    let Some(delim) = src.find('\\') else {
        return Err(-EINVAL);
    };

    if !allocation_available {
        return Err(-ENOMEM);
    }

    let value = &src[delim + 1..];
    Ok(UncCopyPlan {
        value,
        allocation: UncAllocationKind::Kstrdup,
        allocated_len: value.len() + 1,
        copied_len: value.len(),
        nul_terminated: true,
    })
}

pub fn extract_sharename(unc: &str) -> Result<&str, i32> {
    extract_sharename_plan(unc, true).map(|plan| plan.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unc_extractors_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/unc.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include <linux/slab.h>"));
        assert!(source.contains("#include \"cifsglob.h\""));
        assert!(source.contains("#include \"cifsproto.h\""));
        assert!(source.contains("char *extract_hostname"));
        assert!(source.contains("strlen(unc) < 3"));
        assert!(source.contains("for (src = unc; *src && *src == '\\\\'; src++)"));
        assert!(source.contains("delim = strchr(src, '\\\\');"));
        assert!(source.contains("dst = kmalloc((len + 1), GFP_KERNEL);"));
        assert!(source.contains("if (dst == NULL)"));
        assert!(source.contains("return ERR_PTR(-ENOMEM);"));
        assert!(source.contains("memcpy(dst, src, len);"));
        assert!(source.contains("dst[len] = '\\0';"));
        assert!(source.contains("char *extract_sharename"));
        assert!(source.contains("src = unc + 2;"));
        assert!(source.contains("delim++;"));
        assert!(source.contains("dst = kstrdup(delim, GFP_KERNEL);"));
        assert!(source.contains("if (!dst)"));

        assert_eq!(extract_hostname(r"\\server\share\path"), Ok("server"));
        assert_eq!(extract_hostname(r"\\\\server\share"), Ok("server"));
        assert_eq!(extract_hostname(r"\\server"), Err(-EINVAL));
        assert_eq!(extract_sharename(r"\\server\share\path"), Ok("share\\path"));
        assert_eq!(extract_sharename(r"\\server"), Err(-EINVAL));
    }

    #[test]
    fn hostname_plan_matches_kmalloc_copy_and_terminator() {
        assert_eq!(
            extract_hostname_plan(r"\\server\share", true),
            Ok(UncCopyPlan {
                value: "server",
                allocation: UncAllocationKind::Kmalloc,
                allocated_len: 7,
                copied_len: 6,
                nul_terminated: true,
            })
        );
        assert_eq!(
            extract_hostname_plan(r"\\server\share", false),
            Err(-ENOMEM)
        );
        assert_eq!(extract_hostname_plan(r"\\\\", true), Err(-EINVAL));
    }

    #[test]
    fn sharename_plan_matches_kstrdup_after_delimiter_increment() {
        assert_eq!(
            extract_sharename_plan(r"\\server\share\path", true),
            Ok(UncCopyPlan {
                value: "share\\path",
                allocation: UncAllocationKind::Kstrdup,
                allocated_len: 11,
                copied_len: 10,
                nul_terminated: true,
            })
        );
        assert_eq!(
            extract_sharename_plan(r"\\server\share", false),
            Err(-ENOMEM)
        );
        assert_eq!(extract_sharename_plan(r"\\server", true), Err(-EINVAL));
    }
}
