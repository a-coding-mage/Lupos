// SPDX-License-Identifier: GPL-2.0-or-later
/*
 * Copyright (C) 2017 Josh Poimboeuf <jpoimboe@redhat.com>
 */
//! linux-source: arch/x86/include/asm/orc_lookup.h
//! linux-revision: 425f94c2954b1fe80ebdbf9b29854e89750355df
//! architectures: x86_64
//! rewrite-task: S000623

/*
 * The linker script allocates the .orc_lookup section and defines its start
 * and one-past-end symbols.  C declares them as incomplete arrays, so neither
 * symbol is an object that may itself be loaded or stored.  Private zero-size
 * foreign anchors retain only their linker addresses; the address accessors
 * below are the sole Rust interface to those symbols.
 */
unsafe extern "C" {
    #[link_name = "orc_lookup"]
    static ORC_LOOKUP_ANCHOR: [u8; 0];
    #[link_name = "orc_lookup_end"]
    static ORC_LOOKUP_END_ANCHOR: [u8; 0];

    /* Linker-script text section boundary symbols, declared as char arrays in
     * asm-generic/sections.h.  Their addresses are the unsigned-long values
     * produced by LOOKUP_START_IP and LOOKUP_STOP_IP.
     */
    #[link_name = "_stext"]
    static STEXT_ANCHOR: [u8; 0];
    #[link_name = "_etext"]
    static ETEXT_ANCHOR: [u8; 0];
}

/*
 * The start and end are address-only array anchors.  ORC_UNWIND_TABLE aligns
 * the section to four bytes, gives the start an array of u32 elements, and
 * defines the end immediately one element past its last allocated byte.
 * Callers must not dereference `orc_lookup_end()` or create references/slices
 * from either result.  They may use raw-pointer address arithmetic exactly as
 * the C array expressions do, and may dereference elements below the end.
 *
 * unwind_init() is the sole writer in the selected C consumer; it fills the
 * table before setting orc_init.  The later lookup path is gated on orc_init.
 * This header adds no synchronization: Rust callers must preserve that same
 * initialization ordering and must not manufacture Rust aliasing guarantees.
 */
pub(crate) fn orc_lookup() -> *mut u32 {
    core::ptr::addr_of!(ORC_LOOKUP_ANCHOR).cast_mut().cast()
}

pub(crate) fn orc_lookup_end() -> *mut u32 {
    core::ptr::addr_of!(ORC_LOOKUP_END_ANCHOR).cast_mut().cast()
}

/*
 * Keep the C macro's signed-int values.  Callers performing address arithmetic
 * explicitly convert them as required by the corresponding C usual arithmetic
 * conversions.
 */
pub const LOOKUP_BLOCK_ORDER: i32 = 8;
pub const LOOKUP_BLOCK_SIZE: i32 = 1 << LOOKUP_BLOCK_ORDER;

/*
 * These expression macros retain the C definitions' symbol-address semantics.
 * `addr_of!` forms no Rust reference to linker-owned storage and so does not
 * impose Rust aliasing or lifetime guarantees beyond the original symbols.
 */
macro_rules! LOOKUP_START_IP {
    () => {
        core::ptr::addr_of!(STEXT_ANCHOR) as usize
    };
}
pub(crate) use LOOKUP_START_IP;

macro_rules! LOOKUP_STOP_IP {
    () => {
        core::ptr::addr_of!(ETEXT_ANCHOR) as usize
    };
}
pub(crate) use LOOKUP_STOP_IP;
