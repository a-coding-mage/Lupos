#![no_std]

//! lupos kernel library - testable Linux-shaped implementation modules.
//!
//! Runtime code lives under `src/` in Linux-shaped directories (`src/arch/`,
//! `src/include/`, `src/kernel/`, `src/mm/`, `src/fs/`, ...).

extern crate alloc;

#[path = "arch/mod.rs"]
pub mod arch;
#[path = "block/mod.rs"]
pub mod block;
#[path = "certs/mod.rs"]
pub mod certs;
#[path = "crypto/mod.rs"]
pub mod crypto;
#[path = "efi/mod.rs"]
pub mod efi;
#[path = "fs/mod.rs"]
pub mod fs;
#[path = "include/mod.rs"]
pub mod include;
#[path = "init/mod.rs"]
pub mod init;
#[path = "io_uring/mod.rs"]
pub mod io_uring;
#[path = "ipc/mod.rs"]
pub mod ipc;
#[path = "kernel/mod.rs"]
pub mod kernel;
#[path = "lib/mod.rs"]
pub mod lib;
#[path = "linux_driver_abi/mod.rs"]
pub mod linux_driver_abi;
#[path = "mm/mod.rs"]
pub mod mm;
#[path = "net/mod.rs"]
pub mod net;
#[path = "rust/mod.rs"]
pub mod rust;
#[path = "security/mod.rs"]
pub mod security;
#[path = "usr/mod.rs"]
pub mod usr;
#[path = "virt/mod.rs"]
pub mod virt;

// Re-export Milestone 5 modules so tests and downstream code can use short paths.
pub use arch::x86::kernel::acpi;
pub use arch::x86::kernel::apic;
pub use arch::x86::kernel::smp;
