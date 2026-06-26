//! linux-parity: partial
//! linux-source: vendor/linux/include/uapi
//! Linux UAPI constants — single source of truth.
//!
//! Ports values verbatim from `vendor/linux/include/uapi/`.  Per-subsystem
//! modules (futex, sched, time, irq, …) historically redefined the errnos
//! they touched; new code should `pub use crate::include::uapi::errno::*;` instead.

pub mod errno;
pub mod fcntl;
pub mod mount;
pub mod openat2;
pub mod stat;
