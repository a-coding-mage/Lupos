//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/entry
//! x86 entry-path helpers outside the native syscall_64 file.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/entry/entry_fred.c
//! - vendor/linux/arch/x86/entry/syscall_32.c
//! - vendor/linux/arch/x86/entry/vsyscall/vsyscall_64.c
//! - vendor/linux/arch/x86/entry/vdso/common/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/common/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/extable.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32-setup.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/vdso32/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vclock_gettime.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetcpu.c
//! - vendor/linux/arch/x86/entry/vdso/vdso64/vgetrandom.c

pub mod common;
pub mod entry_fred;
pub mod syscall_32;
pub mod thunk;
pub mod vdso;
pub mod vsyscall;

pub mod sys_ia32;
pub mod sys_ni;
pub mod syscall;
pub mod syscall_table;
pub mod syscall_wrappers;
