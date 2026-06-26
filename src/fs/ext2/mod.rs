//! linux-parity: partial
//! linux-source: vendor/linux/fs/ext2
//! ext2 filesystem source coverage.

pub mod symlink;
pub mod trace;
pub mod xattr_security;
pub mod xattr_trusted;
pub mod xattr_user;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ext2XattrListGate {
    MountOptionXattrUser,
    CapSysAdmin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ext2XattrHandler {
    pub symbol: &'static str,
    pub prefix: &'static str,
    pub index: u8,
    pub list_function: &'static str,
    pub get_function: &'static str,
    pub set_function: &'static str,
    pub list_gate: Ext2XattrListGate,
}
