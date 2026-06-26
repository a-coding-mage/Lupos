//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/um
//! User-Mode Linux x86 architecture helpers.

pub mod bugs_32;
pub mod bugs_64;
pub mod delay;
pub mod fault;
pub mod mem_64;
pub mod os_linux_registers;
pub mod os_linux_tls;
pub mod ptrace_user;
pub mod stub_segv;
pub mod sys_call_table_32;
pub mod sys_call_table_64;
pub mod syscalls_32;
pub mod syscalls_64;
pub mod sysrq_32;
pub mod sysrq_64;
pub mod tls_64;
pub mod user_offsets;
pub mod vdso;
