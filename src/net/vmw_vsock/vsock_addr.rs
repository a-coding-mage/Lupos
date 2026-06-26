//! linux-parity: complete
//! linux-source: vendor/linux/net/vmw_vsock/vsock_addr.c
//! test-origin: linux:vendor/linux/net/vmw_vsock/vsock_addr.c
//! VMware vSockets address helpers.

use crate::include::uapi::errno::{EAFNOSUPPORT, EFAULT, EINVAL};

pub const AF_VSOCK: u16 = 40;
pub const VMADDR_CID_ANY: u32 = u32::MAX;
pub const VMADDR_PORT_ANY: u32 = u32::MAX;
pub const VMADDR_FLAG_TO_HOST: u8 = 0x01;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SockAddrVm {
    pub svm_family: u16,
    pub svm_reserved1: u16,
    pub svm_port: u32,
    pub svm_cid: u32,
    pub svm_flags: u8,
}

pub const fn vsock_addr_init(cid: u32, port: u32) -> SockAddrVm {
    SockAddrVm {
        svm_family: AF_VSOCK,
        svm_reserved1: 0,
        svm_port: port,
        svm_cid: cid,
        svm_flags: 0,
    }
}

pub const fn vsock_addr_validate(addr: Option<&SockAddrVm>) -> Result<(), i32> {
    let Some(addr) = addr else {
        return Err(-EFAULT);
    };
    if addr.svm_family != AF_VSOCK {
        return Err(-EAFNOSUPPORT);
    }
    if addr.svm_flags & !VMADDR_FLAG_TO_HOST != 0 {
        return Err(-EINVAL);
    }
    Ok(())
}

pub const fn vsock_addr_bound(addr: &SockAddrVm) -> bool {
    addr.svm_port != VMADDR_PORT_ANY
}

pub const fn vsock_addr_unbind() -> SockAddrVm {
    vsock_addr_init(VMADDR_CID_ANY, VMADDR_PORT_ANY)
}

pub const fn vsock_addr_equals_addr(addr: &SockAddrVm, other: &SockAddrVm) -> bool {
    addr.svm_cid == other.svm_cid && addr.svm_port == other.svm_port
}

pub const fn vsock_addr_cast<'a>(
    addr: Option<&'a SockAddrVm>,
    len: usize,
) -> Result<&'a SockAddrVm, i32> {
    if len < core::mem::size_of::<SockAddrVm>() {
        return Err(-EFAULT);
    }
    let Some(addr) = addr else {
        return Err(-EFAULT);
    };
    match vsock_addr_validate(Some(addr)) {
        Ok(()) => Ok(addr),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsock_addr_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/vmw_vsock/vsock_addr.c"
        ));
        assert!(source.contains("void vsock_addr_init"));
        assert!(source.contains("memset(addr, 0, sizeof(*addr));"));
        assert!(source.contains("addr->svm_family = AF_VSOCK;"));
        assert!(source.contains("addr->svm_cid = cid;"));
        assert!(source.contains("addr->svm_port = port;"));
        assert!(source.contains("__u8 svm_valid_flags = VMADDR_FLAG_TO_HOST;"));
        assert!(source.contains("return -EFAULT;"));
        assert!(source.contains("return -EAFNOSUPPORT;"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("return addr->svm_port != VMADDR_PORT_ANY;"));
        assert!(source.contains("vsock_addr_init(addr, VMADDR_CID_ANY, VMADDR_PORT_ANY);"));
        assert!(source.contains("addr->svm_cid == other->svm_cid"));
        assert!(source.contains("if (len < sizeof(**out_addr))"));

        let addr = vsock_addr_init(3, 1024);
        assert_eq!(addr.svm_family, AF_VSOCK);
        assert_eq!(addr.svm_cid, 3);
        assert_eq!(addr.svm_port, 1024);
    }

    #[test]
    fn validate_bound_unbind_equals_and_cast_follow_linux_rules() {
        let addr = vsock_addr_init(4, 2048);
        assert_eq!(vsock_addr_validate(Some(&addr)), Ok(()));
        assert!(vsock_addr_bound(&addr));
        assert!(vsock_addr_equals_addr(&addr, &vsock_addr_init(4, 2048)));
        assert!(!vsock_addr_equals_addr(&addr, &vsock_addr_init(5, 2048)));

        let unbound = vsock_addr_unbind();
        assert_eq!(unbound.svm_cid, VMADDR_CID_ANY);
        assert_eq!(unbound.svm_port, VMADDR_PORT_ANY);
        assert!(!vsock_addr_bound(&unbound));

        assert_eq!(vsock_addr_validate(None), Err(-EFAULT));
        assert_eq!(
            vsock_addr_validate(Some(&SockAddrVm {
                svm_family: 1,
                ..addr
            })),
            Err(-EAFNOSUPPORT)
        );
        assert_eq!(
            vsock_addr_validate(Some(&SockAddrVm {
                svm_flags: 0x80,
                ..addr
            })),
            Err(-EINVAL)
        );
        assert_eq!(
            vsock_addr_cast(Some(&addr), core::mem::size_of::<SockAddrVm>()),
            Ok(&addr)
        );
        assert_eq!(vsock_addr_cast(Some(&addr), 1), Err(-EFAULT));
    }
}
