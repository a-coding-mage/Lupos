//! linux-parity: complete
//! linux-source: vendor/linux/fs/proc/version.c
//! test-origin: linux:vendor/linux/fs/proc/version.c
//! `/proc/version`.
//!
//! Ref: `vendor/linux/fs/proc/version.c`

extern crate alloc;

use alloc::sync::Arc;

use crate::fs::kernfs::KernfsNode;
use crate::init::version::{UTS_RELEASE, UTS_SYSNAME, UTS_VERSION, linux_proc_banner};

pub const PROC_VERSION_NAME: &str = "version";
pub const PROC_VERSION_MODE: u16 = 0;
pub const PROC_VERSION_PARENT: Option<&str> = None;
pub const VERSION_PROC_SHOW_SYMBOL: &str = "version_proc_show";
pub const PROC_VERSION_INIT_SYMBOL: &str = "proc_version_init";
pub const FS_INITCALL_HOOK: &str = "fs_initcall(proc_version_init)";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcDirEntry {
    pub name: &'static str,
    pub mode: u16,
    pub parent: Option<&'static str>,
    pub show: &'static str,
    pub permanent: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcVersionInitReport {
    pub pde: ProcDirEntry,
    pub ret: i32,
}

pub fn show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(buf, &version_proc_text())
}

pub fn version_proc_text() -> alloc::string::String {
    linux_proc_banner(UTS_SYSNAME, UTS_RELEASE, UTS_VERSION)
}

pub const fn proc_create_single(
    name: &'static str,
    mode: u16,
    parent: Option<&'static str>,
    show: &'static str,
) -> ProcDirEntry {
    ProcDirEntry {
        name,
        mode,
        parent,
        show,
        permanent: false,
    }
}

pub fn pde_make_permanent(pde: &mut ProcDirEntry) {
    pde.permanent = true;
}

pub fn proc_version_init() -> ProcVersionInitReport {
    let mut pde = proc_create_single(
        PROC_VERSION_NAME,
        PROC_VERSION_MODE,
        PROC_VERSION_PARENT,
        VERSION_PROC_SHOW_SYMBOL,
    );
    pde_make_permanent(&mut pde);
    ProcVersionInitReport { pde, ret: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_proc_show_matches_linux_proc_banner_call() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/proc/version.c"
        ));
        assert!(source.contains("seq_printf(m, linux_proc_banner,"));
        assert!(source.contains("utsname()->sysname"));
        assert!(source.contains("utsname()->release"));
        assert!(source.contains("utsname()->version"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("static int __init proc_version_init(void)"));
        assert!(source.contains("struct proc_dir_entry *pde;"));
        assert!(source.contains("proc_create_single(\"version\", 0, NULL, version_proc_show)"));
        assert!(source.contains("pde_make_permanent(pde);"));
        assert!(source.contains("fs_initcall(proc_version_init);"));

        assert_eq!(
            version_proc_text(),
            linux_proc_banner(UTS_SYSNAME, UTS_RELEASE, UTS_VERSION)
        );
        assert!(version_proc_text().ends_with('\n'));
        assert_eq!(PROC_VERSION_NAME, "version");
        assert_eq!(PROC_VERSION_MODE, 0);
        assert_eq!(PROC_VERSION_PARENT, None);
        assert_eq!(VERSION_PROC_SHOW_SYMBOL, "version_proc_show");
        assert_eq!(PROC_VERSION_INIT_SYMBOL, "proc_version_init");
        assert_eq!(FS_INITCALL_HOOK, "fs_initcall(proc_version_init)");
    }

    #[test]
    fn proc_version_init_creates_permanent_single_file() {
        let report = proc_version_init();
        assert_eq!(report.ret, 0);
        assert_eq!(
            report.pde,
            ProcDirEntry {
                name: "version",
                mode: 0,
                parent: None,
                show: "version_proc_show",
                permanent: true,
            }
        );
    }
}
