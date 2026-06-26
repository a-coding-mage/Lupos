//! linux-parity: partial
//! linux-source: vendor/linux/kernel/bpf
//! BPF subsystem.
//!
//! - `cbpf` — classic BPF interpreter (M27 seccomp).  Re-exported at the
//!   top level so existing imports `crate::kernel::bpf::SockFilter`,
//!   `crate::kernel::bpf::bpf_run_filter`, etc. keep working.
//! - `insn`, `uapi`, `interp`, `verifier`, `maps`, `helpers`, `syscall`,
//!   `attach` — eBPF subsystem (M63).

pub mod cbpf;
pub use cbpf::*;

pub mod attach;
pub mod bpf_lsm_proto;
pub mod btf_iter;
pub mod btf_relocate;
pub mod helpers;
pub mod insn;
pub mod interp;
pub mod link_iter;
pub mod linux_sources;
pub mod maps;
pub mod preload;
pub mod prog_iter;
pub mod relo_core;
pub mod syscall;
pub mod sysfs_btf;
pub mod uapi;
pub mod verifier;
