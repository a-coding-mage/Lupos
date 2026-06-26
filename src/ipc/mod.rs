//! linux-parity: partial
//! linux-source: vendor/linux/ipc
//! Linux IPC subsystem root. Implementation files land here as IPC parity grows.

pub mod compat;
pub mod ipc_sysctl;
pub mod mq_sysctl;
pub mod msg;
pub mod msgutil;
pub mod namespace;
pub mod sem;
pub mod shm;
pub mod syscall;
pub mod util;
