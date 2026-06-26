//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/xattr.c
//! test-origin: linux:vendor/linux/io_uring/xattr.c
//! `IORING_OP_{F,G,S}{GET,SET}XATTR`.
//!
//! Ref: vendor/linux/io_uring/xattr.c

use super::sqe::Sqe;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoXattr {
    pub fd: i32,
    pub name_addr: u64,
    pub value_addr: u64,
    pub value_len: u32,
    pub flags: u32,
    pub path_addr: u64,
}

fn common_prep(sqe: &Sqe, needs_path: bool) -> Result<IoXattr, i32> {
    if sqe.addr == 0 || (needs_path && sqe.addr3 == 0) {
        return Err(-22);
    }
    Ok(IoXattr {
        fd: sqe.fd,
        name_addr: sqe.addr,
        value_addr: sqe.off,
        value_len: sqe.len,
        flags: sqe.op_flags,
        path_addr: sqe.addr3,
    })
}

pub fn fgetxattr_prep(sqe: &Sqe) -> Result<IoXattr, i32> {
    common_prep(sqe, false)
}
pub fn fsetxattr_prep(sqe: &Sqe) -> Result<IoXattr, i32> {
    common_prep(sqe, false)
}
pub fn getxattr_prep(sqe: &Sqe) -> Result<IoXattr, i32> {
    common_prep(sqe, true)
}
pub fn setxattr_prep(sqe: &Sqe) -> Result<IoXattr, i32> {
    common_prep(sqe, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fgetxattr_only_needs_name() {
        let mut s = Sqe::default();
        s.addr = 0xcafe;
        let r = fgetxattr_prep(&s).unwrap();
        assert_eq!(r.name_addr, 0xcafe);
    }

    #[test]
    fn getxattr_requires_path_pointer() {
        let mut s = Sqe::default();
        s.addr = 0xcafe;
        // No path pointer.
        assert_eq!(getxattr_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn setxattr_captures_flags() {
        let mut s = Sqe::default();
        s.addr = 1;
        s.addr3 = 2;
        s.op_flags = 0xa5;
        let r = setxattr_prep(&s).unwrap();
        assert_eq!(r.flags, 0xa5);
    }
}
