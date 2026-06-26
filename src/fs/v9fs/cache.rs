//! linux-parity: complete
//! linux-source: vendor/linux/fs/9p/cache.c
//! test-origin: linux:vendor/linux/fs/9p/cache.c
//! V9FS fscache key construction and cookie acquisition gates.

extern crate alloc;

use alloc::{format, string::String};

use crate::include::uapi::errno::{EBUSY, ENOMEM};

pub const CACHE_VOLUME_PREFIX: &str = "9p";
pub const DEBUG_FLAG: &str = "P9_DEBUG_FSC";
pub const V9FS_PATH_KEY_LEN: usize = core::mem::size_of::<u64>();
pub const V9FS_VERSION_KEY_LEN: usize = core::mem::size_of::<u32>();

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V9fsSessionCookieReport {
    pub name: Option<String>,
    pub debug_logged: bool,
    pub error_logged: bool,
    pub stored_volume_cookie: bool,
    pub name_freed: bool,
    pub returned: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct V9fsInodeCookieReport {
    pub regular_checked: bool,
    pub warned_existing_cookie: bool,
    pub path_key_len: usize,
    pub version_key_len: usize,
    pub object_size: u64,
    pub acquired_cookie: bool,
    pub mapping_release_always: bool,
    pub debug_logged: bool,
}

pub fn session_cache_name(dev_name: &str, cachetag: Option<&str>, aname: &str) -> String {
    let selected = cachetag.unwrap_or(aname);
    format!("{CACHE_VOLUME_PREFIX},{dev_name},{selected}").replace('/', ";")
}

pub fn session_cookie_report(
    dev_name: &str,
    cachetag: Option<&str>,
    aname: &str,
    allocation_failed: bool,
    acquire_errno: Option<i32>,
) -> V9fsSessionCookieReport {
    if allocation_failed {
        return V9fsSessionCookieReport {
            name: None,
            debug_logged: false,
            error_logged: false,
            stored_volume_cookie: false,
            name_freed: false,
            returned: -ENOMEM,
        };
    }

    let name = session_cache_name(dev_name, cachetag, aname);
    match acquire_errno {
        Some(errno) if errno != -EBUSY => V9fsSessionCookieReport {
            name: Some(name),
            debug_logged: true,
            error_logged: false,
            stored_volume_cookie: false,
            name_freed: true,
            returned: errno,
        },
        Some(_) => V9fsSessionCookieReport {
            name: Some(name),
            debug_logged: true,
            error_logged: true,
            stored_volume_cookie: false,
            name_freed: true,
            returned: 0,
        },
        None => V9fsSessionCookieReport {
            name: Some(name),
            debug_logged: true,
            error_logged: false,
            stored_volume_cookie: true,
            name_freed: true,
            returned: 0,
        },
    }
}

pub const fn session_cookie_result(
    allocation_failed: bool,
    acquire_errno: Option<i32>,
) -> Result<bool, i32> {
    if allocation_failed {
        return Err(-ENOMEM);
    }
    match acquire_errno {
        Some(errno) if errno == -EBUSY => Ok(false),
        Some(errno) => Err(errno),
        None => Ok(true),
    }
}

pub const fn inode_cookie_acquire_allowed(is_regular: bool, has_cookie: bool) -> bool {
    is_regular && !has_cookie
}

pub const fn mapping_release_always_after_cookie(cookie_acquired: bool) -> bool {
    cookie_acquired
}

pub const fn inode_cookie_report(
    is_regular: bool,
    has_cookie: bool,
    cookie_acquired: bool,
    object_size: u64,
) -> V9fsInodeCookieReport {
    if !is_regular {
        return V9fsInodeCookieReport {
            regular_checked: true,
            warned_existing_cookie: false,
            path_key_len: 0,
            version_key_len: 0,
            object_size: 0,
            acquired_cookie: false,
            mapping_release_always: false,
            debug_logged: false,
        };
    }

    if has_cookie {
        return V9fsInodeCookieReport {
            regular_checked: true,
            warned_existing_cookie: true,
            path_key_len: 0,
            version_key_len: 0,
            object_size: 0,
            acquired_cookie: false,
            mapping_release_always: false,
            debug_logged: false,
        };
    }

    V9fsInodeCookieReport {
        regular_checked: true,
        warned_existing_cookie: false,
        path_key_len: V9FS_PATH_KEY_LEN,
        version_key_len: V9FS_VERSION_KEY_LEN,
        object_size,
        acquired_cookie: cookie_acquired,
        mapping_release_always: cookie_acquired,
        debug_logged: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v9fs_cache_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/cache.c"
        ));
        assert!(source.contains("#include <linux/jiffies.h>"));
        assert!(source.contains("#include <linux/file.h>"));
        assert!(source.contains("#include <net/9p/9p.h>"));
        assert!(source.contains("#include \"v9fs.h\""));
        assert!(source.contains("#include \"cache.h\""));
        assert!(source.contains("kasprintf(GFP_KERNEL, \"9p,%s,%s\""));
        assert!(source.contains("v9ses->cachetag ?: v9ses->aname"));
        assert!(source.contains("if (*p == '/')"));
        assert!(source.contains("*p = ';';"));
        assert!(source.contains("fscache_acquire_volume"));
        assert!(source.contains("p9_debug(P9_DEBUG_FSC, \"session %p get volume %p (%s)\\n\""));
        assert!(source.contains("if (IS_ERR(vcookie))"));
        assert!(source.contains("vcookie != ERR_PTR(-EBUSY)"));
        assert!(source.contains("kfree(name);"));
        assert!(source.contains("return PTR_ERR(vcookie);"));
        assert!(source.contains("pr_err(\"Cache volume key already in use (%s)\\n\", name);"));
        assert!(source.contains("vcookie = NULL;"));
        assert!(source.contains("v9ses->fscache = vcookie;"));
        assert!(source.contains("fscache_acquire_cookie"));
        assert!(source.contains("if (!S_ISREG(inode->i_mode))"));
        assert!(source.contains("if (WARN_ON(v9fs_inode_cookie(v9inode)))"));
        assert!(source.contains("version = cpu_to_le32(v9inode->qid.version);"));
        assert!(source.contains("path = cpu_to_le64(v9inode->qid.path);"));
        assert!(source.contains("&path, sizeof(path),"));
        assert!(source.contains("&version, sizeof(version),"));
        assert!(source.contains("i_size_read(&v9inode->netfs.inode)"));
        assert!(source.contains("mapping_set_release_always"));
        assert!(source.contains(DEBUG_FLAG));

        assert_eq!(
            session_cache_name("dev/name", Some("tag/path"), "aname"),
            "9p,dev;name,tag;path"
        );
        assert_eq!(
            session_cache_name("dev", None, "root/path"),
            "9p,dev,root;path"
        );
        assert_eq!(session_cookie_result(true, None), Err(-12));
        assert_eq!(session_cookie_result(false, Some(-16)), Ok(false));
        assert_eq!(session_cookie_result(false, Some(-5)), Err(-5));
        assert_eq!(session_cookie_result(false, None), Ok(true));
        assert!(inode_cookie_acquire_allowed(true, false));
        assert!(!inode_cookie_acquire_allowed(false, false));
        assert!(!inode_cookie_acquire_allowed(true, true));
        assert!(mapping_release_always_after_cookie(true));
    }

    #[test]
    fn session_cookie_report_matches_success_busy_and_error_cleanup() {
        assert_eq!(
            session_cookie_report("dev/name", Some("tag/path"), "aname", false, None),
            V9fsSessionCookieReport {
                name: Some(String::from("9p,dev;name,tag;path")),
                debug_logged: true,
                error_logged: false,
                stored_volume_cookie: true,
                name_freed: true,
                returned: 0,
            }
        );
        assert_eq!(
            session_cookie_report("dev", None, "root/path", false, Some(-EBUSY)),
            V9fsSessionCookieReport {
                name: Some(String::from("9p,dev,root;path")),
                debug_logged: true,
                error_logged: true,
                stored_volume_cookie: false,
                name_freed: true,
                returned: 0,
            }
        );
        assert_eq!(
            session_cookie_report("dev", None, "root", false, Some(-5)).returned,
            -5
        );
        assert_eq!(
            session_cookie_report("dev", None, "root", true, None),
            V9fsSessionCookieReport {
                name: None,
                debug_logged: false,
                error_logged: false,
                stored_volume_cookie: false,
                name_freed: false,
                returned: -ENOMEM,
            }
        );
    }

    #[test]
    fn inode_cookie_report_matches_regular_warn_and_mapping_paths() {
        assert_eq!(
            inode_cookie_report(false, false, true, 4096),
            V9fsInodeCookieReport {
                regular_checked: true,
                warned_existing_cookie: false,
                path_key_len: 0,
                version_key_len: 0,
                object_size: 0,
                acquired_cookie: false,
                mapping_release_always: false,
                debug_logged: false,
            }
        );
        assert!(inode_cookie_report(true, true, false, 0).warned_existing_cookie);
        assert_eq!(
            inode_cookie_report(true, false, true, 4096),
            V9fsInodeCookieReport {
                regular_checked: true,
                warned_existing_cookie: false,
                path_key_len: 8,
                version_key_len: 4,
                object_size: 4096,
                acquired_cookie: true,
                mapping_release_always: true,
                debug_logged: true,
            }
        );
    }
}
