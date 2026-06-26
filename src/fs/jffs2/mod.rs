//! linux-parity: complete
//! linux-source: vendor/linux/fs/jffs2
//! JFFS2 small source units.

pub mod compr_lzo;
pub mod ioctl;
pub mod security;
pub mod symlink;
pub mod writev;
pub mod xattr_trusted;
pub mod xattr_user;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Jffs2XattrListGate {
    CapSysAdmin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Jffs2XattrHandler {
    pub symbol: &'static str,
    pub prefix: &'static str,
    pub xprefix: u8,
    pub list_function: Option<&'static str>,
    pub get_function: &'static str,
    pub set_function: &'static str,
    pub list_gate: Option<Jffs2XattrListGate>,
}
