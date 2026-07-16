//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/lib/retpoline.S
//! linux-source: vendor/linux/arch/x86/include/asm/GEN-for-each-reg.h
//! Linux module-visible x86 indirect and return thunks.
//!
//! The thunk array is deliberately emitted in machine-register order and at
//! `RETPOLINE_THUNK_SIZE` spacing. `apply_retpolines()` in
//! `vendor/linux/arch/x86/kernel/alternative.c` derives the encoded register
//! from exactly this address relationship.

use crate::kernel::module::{export_symbol, find_symbol};

pub const RETPOLINE_THUNK_SIZE: usize = 32;
pub const RETPOLINE_THUNK_COUNT: usize = 16;

core::arch::global_asm!(
    r#"
    .pushsection .text..__x86.indirect_thunk,"ax"

    .balign 64, 0xcc
    .global call_depth_return_thunk
    .type call_depth_return_thunk,@function
call_depth_return_thunk:
        shl qword ptr gs:[rip + {percpu_base} + {depth_offset}], 5
        jz .Lcall_depth_stuff
        ret
        int3
.Lcall_depth_stuff:
        .rept 16
        call 771f
        int3
771:
        .endr
        add rsp, 128
        mov qword ptr gs:[rip + {percpu_base} + {depth_offset}], -1
        ret
        int3
    .size call_depth_return_thunk, .-call_depth_return_thunk

    .macro LUPOS_RETPOLINE_THUNK name, reg
        .balign 32, 0xcc
        .global __x86_indirect_thunk_\name
        .type __x86_indirect_thunk_\name,@function
__x86_indirect_thunk_\name:
        call .Ldo_rop_\@
        int3
.Ldo_rop_\@:
        mov [rsp], \reg
        jmp __x86_return_thunk
        .size __x86_indirect_thunk_\name, .-__x86_indirect_thunk_\name
    .endm

    .balign 32, 0xcc
    .global __x86_indirect_thunk_array
__x86_indirect_thunk_array:
    LUPOS_RETPOLINE_THUNK rax, rax
    LUPOS_RETPOLINE_THUNK rcx, rcx
    LUPOS_RETPOLINE_THUNK rdx, rdx
    LUPOS_RETPOLINE_THUNK rbx, rbx
    LUPOS_RETPOLINE_THUNK rsp, rsp
    LUPOS_RETPOLINE_THUNK rbp, rbp
    LUPOS_RETPOLINE_THUNK rsi, rsi
    LUPOS_RETPOLINE_THUNK rdi, rdi
    LUPOS_RETPOLINE_THUNK r8, r8
    LUPOS_RETPOLINE_THUNK r9, r9
    LUPOS_RETPOLINE_THUNK r10, r10
    LUPOS_RETPOLINE_THUNK r11, r11
    LUPOS_RETPOLINE_THUNK r12, r12
    LUPOS_RETPOLINE_THUNK r13, r13
    LUPOS_RETPOLINE_THUNK r14, r14
    LUPOS_RETPOLINE_THUNK r15, r15
    .balign 32, 0xcc
    .global __x86_indirect_thunk_array_end
__x86_indirect_thunk_array_end:

    .macro LUPOS_CALL_DEPTH_CALL_THUNK name, reg
        .balign 32, 0xcc
        .global __x86_indirect_call_thunk_\name
        .type __x86_indirect_call_thunk_\name,@function
__x86_indirect_call_thunk_\name:
        sar qword ptr gs:[rip + {percpu_base} + {depth_offset}], 5
        call .Lcall_depth_rop_\@
        int3
.Lcall_depth_rop_\@:
        mov [rsp], \reg
        ret
        int3
        .size __x86_indirect_call_thunk_\name, .-__x86_indirect_call_thunk_\name
    .endm

    .balign 32, 0xcc
    .global __x86_indirect_call_thunk_array
__x86_indirect_call_thunk_array:
    LUPOS_CALL_DEPTH_CALL_THUNK rax, rax
    LUPOS_CALL_DEPTH_CALL_THUNK rcx, rcx
    LUPOS_CALL_DEPTH_CALL_THUNK rdx, rdx
    LUPOS_CALL_DEPTH_CALL_THUNK rbx, rbx
    LUPOS_CALL_DEPTH_CALL_THUNK rsp, rsp
    LUPOS_CALL_DEPTH_CALL_THUNK rbp, rbp
    LUPOS_CALL_DEPTH_CALL_THUNK rsi, rsi
    LUPOS_CALL_DEPTH_CALL_THUNK rdi, rdi
    LUPOS_CALL_DEPTH_CALL_THUNK r8, r8
    LUPOS_CALL_DEPTH_CALL_THUNK r9, r9
    LUPOS_CALL_DEPTH_CALL_THUNK r10, r10
    LUPOS_CALL_DEPTH_CALL_THUNK r11, r11
    LUPOS_CALL_DEPTH_CALL_THUNK r12, r12
    LUPOS_CALL_DEPTH_CALL_THUNK r13, r13
    LUPOS_CALL_DEPTH_CALL_THUNK r14, r14
    LUPOS_CALL_DEPTH_CALL_THUNK r15, r15
    .balign 32, 0xcc
    .global __x86_indirect_call_thunk_array_end
__x86_indirect_call_thunk_array_end:

    .macro LUPOS_CALL_DEPTH_JUMP_THUNK name, reg
        .balign 32, 0xcc
        .global __x86_indirect_jump_thunk_\name
        .type __x86_indirect_jump_thunk_\name,@function
__x86_indirect_jump_thunk_\name:
        call .Ljump_depth_rop_\@
        int3
.Ljump_depth_rop_\@:
        mov [rsp], \reg
        ret
        int3
        .size __x86_indirect_jump_thunk_\name, .-__x86_indirect_jump_thunk_\name
    .endm

    .balign 32, 0xcc
    .global __x86_indirect_jump_thunk_array
__x86_indirect_jump_thunk_array:
    LUPOS_CALL_DEPTH_JUMP_THUNK rax, rax
    LUPOS_CALL_DEPTH_JUMP_THUNK rcx, rcx
    LUPOS_CALL_DEPTH_JUMP_THUNK rdx, rdx
    LUPOS_CALL_DEPTH_JUMP_THUNK rbx, rbx
    LUPOS_CALL_DEPTH_JUMP_THUNK rsp, rsp
    LUPOS_CALL_DEPTH_JUMP_THUNK rbp, rbp
    LUPOS_CALL_DEPTH_JUMP_THUNK rsi, rsi
    LUPOS_CALL_DEPTH_JUMP_THUNK rdi, rdi
    LUPOS_CALL_DEPTH_JUMP_THUNK r8, r8
    LUPOS_CALL_DEPTH_JUMP_THUNK r9, r9
    LUPOS_CALL_DEPTH_JUMP_THUNK r10, r10
    LUPOS_CALL_DEPTH_JUMP_THUNK r11, r11
    LUPOS_CALL_DEPTH_JUMP_THUNK r12, r12
    LUPOS_CALL_DEPTH_JUMP_THUNK r13, r13
    LUPOS_CALL_DEPTH_JUMP_THUNK r14, r14
    LUPOS_CALL_DEPTH_JUMP_THUNK r15, r15
    .balign 32, 0xcc
    .global __x86_indirect_jump_thunk_array_end
__x86_indirect_jump_thunk_array_end:

    .global __x86_return_thunk
    .type __x86_return_thunk,@function
__x86_return_thunk:
    ret
    int3
    .size __x86_return_thunk, .-__x86_return_thunk

    .popsection
"#,
    percpu_base = sym crate::arch::x86::kernel::setup_percpu::LINUX_PER_CPU_AREAS,
    depth_offset = const crate::arch::x86::kernel::setup_percpu::X86_CALL_DEPTH_OFFSET,
);

unsafe extern "C" {
    static __x86_indirect_thunk_array: u8;
    static __x86_indirect_thunk_array_end: u8;
    static __x86_indirect_call_thunk_array: u8;
    static __x86_indirect_call_thunk_array_end: u8;
    static __x86_indirect_jump_thunk_array: u8;
    static __x86_indirect_jump_thunk_array_end: u8;
    pub fn call_depth_return_thunk();
    pub fn __x86_indirect_thunk_rax();
    pub fn __x86_indirect_thunk_rcx();
    pub fn __x86_indirect_thunk_rdx();
    pub fn __x86_indirect_thunk_rbx();
    pub fn __x86_indirect_thunk_rsp();
    pub fn __x86_indirect_thunk_rbp();
    pub fn __x86_indirect_thunk_rsi();
    pub fn __x86_indirect_thunk_rdi();
    pub fn __x86_indirect_thunk_r8();
    pub fn __x86_indirect_thunk_r9();
    pub fn __x86_indirect_thunk_r10();
    pub fn __x86_indirect_thunk_r11();
    pub fn __x86_indirect_thunk_r12();
    pub fn __x86_indirect_thunk_r13();
    pub fn __x86_indirect_thunk_r14();
    pub fn __x86_indirect_thunk_r15();
    pub fn __x86_return_thunk();
}

pub fn indirect_thunk_array_addr() -> usize {
    core::ptr::addr_of!(__x86_indirect_thunk_array) as usize
}

pub fn indirect_thunk_array_end() -> usize {
    core::ptr::addr_of!(__x86_indirect_thunk_array_end) as usize
}

pub fn return_thunk_addr() -> usize {
    if crate::arch::x86::kernel::cpu::common::boot_cpu_has(
        crate::arch::x86::kernel::cpu::common::X86_FEATURE_CALL_DEPTH,
    ) {
        call_depth_return_thunk as usize
    } else {
        __x86_return_thunk as usize
    }
}

pub fn compiler_return_thunk_addr() -> usize {
    __x86_return_thunk as usize
}

pub fn call_depth_retpoline_thunk_addr(register: u8, call: bool) -> Option<usize> {
    if register as usize >= RETPOLINE_THUNK_COUNT || register == 4 {
        return None;
    }
    let base = if call {
        core::ptr::addr_of!(__x86_indirect_call_thunk_array) as usize
    } else {
        core::ptr::addr_of!(__x86_indirect_jump_thunk_array) as usize
    };
    Some(base + register as usize * RETPOLINE_THUNK_SIZE)
}

/// Register number encoded by an address in Linux's thunk array.
pub fn retpoline_register(target: usize) -> Option<u8> {
    let offset = target.checked_sub(indirect_thunk_array_addr())?;
    if offset % RETPOLINE_THUNK_SIZE != 0 || offset >= RETPOLINE_THUNK_COUNT * RETPOLINE_THUNK_SIZE
    {
        return None;
    }
    Some((offset / RETPOLINE_THUNK_SIZE) as u8)
}

fn export_once(name: &'static str, addr: usize) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, false);
    }
}

/// Export the symbols emitted by `retpoline.S` which compiler-generated
/// module relocations reference before their site metadata is finalized.
pub fn register_module_exports() {
    let thunks = [
        (
            "__x86_indirect_thunk_rax",
            __x86_indirect_thunk_rax as usize,
        ),
        (
            "__x86_indirect_thunk_rcx",
            __x86_indirect_thunk_rcx as usize,
        ),
        (
            "__x86_indirect_thunk_rdx",
            __x86_indirect_thunk_rdx as usize,
        ),
        (
            "__x86_indirect_thunk_rbx",
            __x86_indirect_thunk_rbx as usize,
        ),
        (
            "__x86_indirect_thunk_rsp",
            __x86_indirect_thunk_rsp as usize,
        ),
        (
            "__x86_indirect_thunk_rbp",
            __x86_indirect_thunk_rbp as usize,
        ),
        (
            "__x86_indirect_thunk_rsi",
            __x86_indirect_thunk_rsi as usize,
        ),
        (
            "__x86_indirect_thunk_rdi",
            __x86_indirect_thunk_rdi as usize,
        ),
        ("__x86_indirect_thunk_r8", __x86_indirect_thunk_r8 as usize),
        ("__x86_indirect_thunk_r9", __x86_indirect_thunk_r9 as usize),
        (
            "__x86_indirect_thunk_r10",
            __x86_indirect_thunk_r10 as usize,
        ),
        (
            "__x86_indirect_thunk_r11",
            __x86_indirect_thunk_r11 as usize,
        ),
        (
            "__x86_indirect_thunk_r12",
            __x86_indirect_thunk_r12 as usize,
        ),
        (
            "__x86_indirect_thunk_r13",
            __x86_indirect_thunk_r13 as usize,
        ),
        (
            "__x86_indirect_thunk_r14",
            __x86_indirect_thunk_r14 as usize,
        ),
        (
            "__x86_indirect_thunk_r15",
            __x86_indirect_thunk_r15 as usize,
        ),
    ];
    for (name, addr) in thunks {
        export_once(name, addr);
    }
    export_once("__x86_return_thunk", __x86_return_thunk as usize);
    let call_base = core::ptr::addr_of!(__x86_indirect_call_thunk_array) as usize;
    let jump_base = core::ptr::addr_of!(__x86_indirect_jump_thunk_array) as usize;
    let call_names = [
        "__x86_indirect_call_thunk_rax", "__x86_indirect_call_thunk_rcx",
        "__x86_indirect_call_thunk_rdx", "__x86_indirect_call_thunk_rbx",
        "__x86_indirect_call_thunk_rsp", "__x86_indirect_call_thunk_rbp",
        "__x86_indirect_call_thunk_rsi", "__x86_indirect_call_thunk_rdi",
        "__x86_indirect_call_thunk_r8", "__x86_indirect_call_thunk_r9",
        "__x86_indirect_call_thunk_r10", "__x86_indirect_call_thunk_r11",
        "__x86_indirect_call_thunk_r12", "__x86_indirect_call_thunk_r13",
        "__x86_indirect_call_thunk_r14", "__x86_indirect_call_thunk_r15",
    ];
    let jump_names = [
        "__x86_indirect_jump_thunk_rax", "__x86_indirect_jump_thunk_rcx",
        "__x86_indirect_jump_thunk_rdx", "__x86_indirect_jump_thunk_rbx",
        "__x86_indirect_jump_thunk_rsp", "__x86_indirect_jump_thunk_rbp",
        "__x86_indirect_jump_thunk_rsi", "__x86_indirect_jump_thunk_rdi",
        "__x86_indirect_jump_thunk_r8", "__x86_indirect_jump_thunk_r9",
        "__x86_indirect_jump_thunk_r10", "__x86_indirect_jump_thunk_r11",
        "__x86_indirect_jump_thunk_r12", "__x86_indirect_jump_thunk_r13",
        "__x86_indirect_jump_thunk_r14", "__x86_indirect_jump_thunk_r15",
    ];
    for index in 0..RETPOLINE_THUNK_COUNT {
        export_once(call_names[index], call_base + index * RETPOLINE_THUNK_SIZE);
        export_once(jump_names[index], jump_base + index * RETPOLINE_THUNK_SIZE);
    }
}
