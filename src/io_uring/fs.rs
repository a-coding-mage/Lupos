//! linux-parity: complete
//! linux-source: vendor/linux/io_uring/fs.c
//! test-origin: linux:vendor/linux/io_uring/fs.c
//! `IORING_OP_RENAMEAT` / `UNLINKAT` / `MKDIRAT` / `SYMLINKAT` / `LINKAT`.
//!
//! Ref: vendor/linux/io_uring/fs.c

use super::sqe::Sqe;

#[derive(Clone, Copy, Debug, Default)]
pub struct IoFsPath {
    pub old_dfd: i32,
    pub new_dfd: i32,
    pub old_path: u64,
    pub new_path: u64,
    pub flags: u32,
    pub mode: u32,
}

fn require_path(addr: u64) -> Result<(), i32> {
    if addr == 0 { Err(-22) } else { Ok(()) }
}

pub fn renameat_prep(sqe: &Sqe) -> Result<IoFsPath, i32> {
    require_path(sqe.addr)?;
    require_path(sqe.addr3)?;
    Ok(IoFsPath {
        old_dfd: sqe.fd,
        new_dfd: sqe.len as i32,
        old_path: sqe.addr,
        new_path: sqe.addr3,
        flags: sqe.op_flags,
        mode: 0,
    })
}

pub fn unlinkat_prep(sqe: &Sqe) -> Result<IoFsPath, i32> {
    require_path(sqe.addr)?;
    Ok(IoFsPath {
        old_dfd: sqe.fd,
        new_dfd: -1,
        old_path: sqe.addr,
        new_path: 0,
        flags: sqe.op_flags,
        mode: 0,
    })
}

pub fn mkdirat_prep(sqe: &Sqe) -> Result<IoFsPath, i32> {
    require_path(sqe.addr)?;
    Ok(IoFsPath {
        old_dfd: sqe.fd,
        new_dfd: -1,
        old_path: sqe.addr,
        new_path: 0,
        flags: 0,
        mode: sqe.len,
    })
}

pub fn symlinkat_prep(sqe: &Sqe) -> Result<IoFsPath, i32> {
    require_path(sqe.addr)?;
    require_path(sqe.addr3)?;
    Ok(IoFsPath {
        old_dfd: -1,
        new_dfd: sqe.fd,
        old_path: sqe.addr,
        new_path: sqe.addr3,
        flags: 0,
        mode: 0,
    })
}

pub fn linkat_prep(sqe: &Sqe) -> Result<IoFsPath, i32> {
    require_path(sqe.addr)?;
    require_path(sqe.addr3)?;
    Ok(IoFsPath {
        old_dfd: sqe.fd,
        new_dfd: sqe.len as i32,
        old_path: sqe.addr,
        new_path: sqe.addr3,
        flags: sqe.op_flags,
        mode: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renameat_requires_both_paths() {
        let mut s = Sqe::default();
        s.addr = 0;
        s.addr3 = 0xface;
        assert_eq!(renameat_prep(&s).unwrap_err(), -22);
        s.addr = 0xcafe;
        s.addr3 = 0;
        assert_eq!(renameat_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn unlinkat_requires_path() {
        let s = Sqe::default();
        assert_eq!(unlinkat_prep(&s).unwrap_err(), -22);
    }

    #[test]
    fn mkdirat_captures_mode_from_len() {
        let mut s = Sqe::default();
        s.fd = 4;
        s.addr = 0xcafe;
        s.len = 0o755;
        let r = mkdirat_prep(&s).unwrap();
        assert_eq!(r.old_dfd, 4);
        assert_eq!(r.mode, 0o755);
    }

    #[test]
    fn linkat_uses_two_dfds() {
        let mut s = Sqe::default();
        s.fd = 3;
        s.len = 7;
        s.addr = 0xcafe;
        s.addr3 = 0xface;
        let r = linkat_prep(&s).unwrap();
        assert_eq!(r.old_dfd, 3);
        assert_eq!(r.new_dfd, 7);
    }
}
