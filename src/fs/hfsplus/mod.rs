//! linux-parity: partial
//! linux-source: vendor/linux/fs/hfsplus
//! HFS+ filesystem source coverage.

pub mod ioctl;
pub mod xattr_security;
pub mod xattr_trusted;
pub mod xattr_user;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HfsplusXattrHandler {
    pub symbol: &'static str,
    pub prefix: &'static str,
    pub prefix_len: usize,
    pub get_function: &'static str,
    pub set_function: &'static str,
}
