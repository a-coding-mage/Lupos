// SPDX-License-Identifier: GPL-2.0
//! linux-source: arch/x86/include/asm/audit.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000472

//! x86 IA32 audit declarations.
//!
//! The C declarations for the class tables have incomplete array type.  Rust
//! cannot declare an external object with an incomplete array type, so each
//! binding denotes the first `u32` element at the same externally linked
//! address.  C consumers use these declarations through array-to-pointer
//! decay; Rust consumers must likewise take the address of the binding before
//! indexing the table.

unsafe extern "C" {
    pub fn ia32_classify_syscall(syscall: u32) -> i32;

    pub static mut ia32_dir_class: u32;
    pub static mut ia32_write_class: u32;
    pub static mut ia32_read_class: u32;
    pub static mut ia32_chattr_class: u32;
    pub static mut ia32_signal_class: u32;
}
