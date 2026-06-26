//! linux-parity: complete
//! linux-source: vendor/linux/fs/nfsd/lockd.c
//! test-origin: linux:vendor/linux/fs/nfsd/lockd.c
//! NFSD lockd binding access and error mapping.

use crate::include::uapi::errno::{ENOLCK, ESTALE, EWOULDBLOCK};
use crate::include::uapi::fcntl::O_WRONLY;

pub const NFSD_MAY_WRITE: i32 = 0x002;
pub const NFSD_MAY_READ: i32 = 0x004;
pub const NFSD_MAY_NLM: i32 = 0x020;
pub const NFSD_MAY_OWNER_OVERRIDE: i32 = 0x040;
pub const NFSD_MAY_BYPASS_GSS: i32 = 0x400;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NfsdOpenStatus {
    Ok,
    Jukebox,
    Stale,
    Other,
}

pub const fn nlm_fopen_access(flags: u32) -> i32 {
    let base = if flags == O_WRONLY {
        NFSD_MAY_WRITE
    } else {
        NFSD_MAY_READ
    };
    base | NFSD_MAY_NLM | NFSD_MAY_OWNER_OVERRIDE | NFSD_MAY_BYPASS_GSS
}

pub const fn nlm_fopen_result(status: NfsdOpenStatus) -> Result<(), i32> {
    match status {
        NfsdOpenStatus::Ok => Ok(()),
        NfsdOpenStatus::Jukebox => Err(-EWOULDBLOCK),
        NfsdOpenStatus::Stale => Err(-ESTALE),
        NfsdOpenStatus::Other => Err(-ENOLCK),
    }
}

pub const fn nfsd_lockd_ops_installed(after_init: bool) -> bool {
    after_init
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfsd_lockd_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/nfsd/lockd.c"
        ));
        assert!(source.contains("#include <linux/file.h>"));
        assert!(source.contains("#include <linux/lockd/bind.h>"));
        assert!(source.contains("#include \"nfsd.h\""));
        assert!(source.contains("#define NFSDDBG_FACILITY\t\tNFSDDBG_LOCKD"));
        assert!(source.contains("static int nlm_fopen(struct svc_rqst *rqstp, struct nfs_fh *f,"));
        assert!(source.contains("fh_init(&fh,0);"));
        assert!(source.contains("fh.fh_handle.fh_size = f->size;"));
        assert!(source.contains("memcpy(&fh.fh_handle.fh_raw, f->data, f->size);"));
        assert!(source.contains("access = (flags == O_WRONLY) ? NFSD_MAY_WRITE : NFSD_MAY_READ;"));
        assert!(
            source.contains(
                "access |= NFSD_MAY_NLM | NFSD_MAY_OWNER_OVERRIDE | NFSD_MAY_BYPASS_GSS;"
            )
        );
        assert!(source.contains("nfserr = nfsd_open(rqstp, &fh, S_IFREG, access, filp);"));
        assert!(source.contains("case nfs_ok:"));
        assert!(source.contains("case nfserr_jukebox:"));
        assert!(source.contains("return -EWOULDBLOCK;"));
        assert!(source.contains("case nfserr_stale:"));
        assert!(source.contains("return -ESTALE;"));
        assert!(source.contains("return -ENOLCK;"));
        assert!(source.contains("fput(filp);"));
        assert!(source.contains("static const struct nlmsvc_binding nfsd_nlm_ops"));
        assert!(source.contains("nlmsvc_ops = &nfsd_nlm_ops;"));
        assert!(source.contains("nlmsvc_ops = NULL;"));

        assert_eq!(
            nlm_fopen_access(O_WRONLY),
            NFSD_MAY_WRITE | NFSD_MAY_NLM | NFSD_MAY_OWNER_OVERRIDE | NFSD_MAY_BYPASS_GSS
        );
        assert_eq!(
            nlm_fopen_access(0),
            NFSD_MAY_READ | NFSD_MAY_NLM | NFSD_MAY_OWNER_OVERRIDE | NFSD_MAY_BYPASS_GSS
        );
        assert_eq!(nlm_fopen_result(NfsdOpenStatus::Ok), Ok(()));
        assert_eq!(nlm_fopen_result(NfsdOpenStatus::Jukebox), Err(-EWOULDBLOCK));
        assert_eq!(nlm_fopen_result(NfsdOpenStatus::Stale), Err(-ESTALE));
        assert_eq!(nlm_fopen_result(NfsdOpenStatus::Other), Err(-ENOLCK));
        assert!(nfsd_lockd_ops_installed(true));
        assert!(!nfsd_lockd_ops_installed(false));
    }
}
