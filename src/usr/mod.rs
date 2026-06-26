//! linux-parity: partial
//! linux-source: vendor/linux/usr
//! Linux usr/initramfs build root.
//!
//! Runtime initramfs handling lives under init/. The small staged PID 1 helper
//! crate lives in usr/init because it is a user-visible initramfs payload.

pub mod gen_init_cpio;
