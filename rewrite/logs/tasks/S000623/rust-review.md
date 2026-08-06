# Rust semantics review — S000623

Scope: source-only review of `src/arch/x86/include/asm/orc_lookup.rs` against pinned `arch/x86/include/asm/orc_lookup.h`, its selected x86_64 consumer `arch/x86/kernel/unwind_orc.c`, `include/asm-generic/sections.h`, and the `ORC_UNWIND_TABLE` linker-script definition in `include/asm-generic/vmlinux.lds.h`. No compiler, formatter, linker, test, or runtime diagnostic was used.

## Finding R1 — foreign scalar declarations overstate linker-boundary storage contract (must resolve)

The candidate represents all four linker symbols as foreign scalar statics.  That makes `orc_lookup` and `orc_lookup_end` readable/writable Rust `u32` objects, and `_stext`/`_etext` readable Rust `u8` objects.  This differs from the C interface: `orc_lookup[]`, `orc_lookup_end[]`, `_stext[]`, and `_etext[]` are incomplete-array address anchors.  In particular, `ORC_UNWIND_TABLE` sets `orc_lookup_end = .` immediately after reserving the final table byte; it is a one-past-end boundary, not a `u32` object that may be read or written.

Although the current macros take only `_stext`/`_etext` addresses and the intended future caller can use `addr_of!` for the lookup table, the public `static mut u32` declarations expose direct scalar reads/writes that C's array-to-pointer expression cannot perform.  A direct Rust access to `orc_lookup_end` would load from the endpoint rather than yield its address.  The declaration should be replaced by an address-only/opaque linker-anchor representation, or its visibility and API constrained so every possible consumer obtains only raw addresses and performs explicitly justified pointer arithmetic.  The resolution must document the no-dereference invariant for both table-boundary symbols and the synchronization contract for writes to the lookup table.

## Checks with no additional finding

- Foreign `extern "C"` names match the linker-script symbol spellings; no Rust definition or `no_mangle` export is appropriate because the linker script owns the symbols.
- The candidate's `usize` result for `LOOKUP_START_IP`/`LOOKUP_STOP_IP` matches `unsigned long` on this task's sole x86_64 architecture.  `addr_of!` avoids forming references to linker-owned storage and each zero-argument macro expansion evaluates the address expression once, matching the C macros' observable use.
- `LOOKUP_BLOCK_ORDER` and `LOOKUP_BLOCK_SIZE` are correctly represented as `i32` values for the C signed-`int` macro expressions.  Future translated expressions must retain the original conversion/overflow behavior rather than freely mixing these constants with `usize`.
- Rust macro re-export is crate-restricted and the macro body resolves its private `_stext`/`_etext` names at its definition site; this adds no caller-side evaluation or aliasing behavior.
- `LINKER_SCRIPT` is defined only while preprocessing the original `vmlinux.lds.S`; that script continues to include the C header, where the branch intentionally omits the C declarations while retaining the block-size macros.  The Rust file is not a linker-script input, so its lack of a Rust `LINKER_SCRIPT` cfg branch does not itself change the selected script behavior.  The applier should record this disposition when closing the manifest's pending conditional entry.

Outcome: revision required for R1 before source acceptance.
