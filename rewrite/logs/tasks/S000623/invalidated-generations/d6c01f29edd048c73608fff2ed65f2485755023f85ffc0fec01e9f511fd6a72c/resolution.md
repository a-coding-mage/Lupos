# Resolution — S000623

## Result

Accepted after correcting Rust-review finding R1.  `src/arch/x86/include/asm/orc_lookup.rs` remains the path-preserving translation of pinned `arch/x86/include/asm/orc_lookup.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df` for x86_64.  No compiler, formatter, linker, test, runtime command, or diagnostic was used.

## Review dispositions

| Review | Finding | Disposition |
| --- | --- | --- |
| Parity | No findings | Confirmed.  The candidate covers the two numeric macros, two text-boundary address expressions, both table-boundary linker symbols, and the `LINKER_SCRIPT` conditional behavior. |
| Rust | R1: scalar foreign statics made linker anchors appear readable/writable Rust objects | Fixed.  The four private foreign bindings are now `[u8; 0]` anchors with exact linker names.  `orc_lookup()` and `orc_lookup_end()` return only raw `*mut u32` addresses; none of the bindings is a public scalar object. |

## Source recheck and R1 adjudication

`orc_lookup.h:26-27` declares `orc_lookup[]` and `orc_lookup_end[]` as incomplete `unsigned int` arrays.  `asm-generic/vmlinux.lds.h:875-881` computes `text_size`, aligns the lookup section to four bytes, sets `orc_lookup = .`, reserves `(((text_size + LOOKUP_BLOCK_SIZE - 1) / LOOKUP_BLOCK_SIZE) + 1) * 4` bytes, then sets `orc_lookup_end = .`.  Thus the end symbol is a one-past-end boundary and is not an `unsigned int` object.

The corrected private zero-size bindings preserve link-name/address identity without providing a scalar load/store interface.  The accessors form their results with `addr_of!`, never a Rust reference.  The end accessor is explicitly documented as non-dereferenceable.  The start accessor represents the array base only; callers may use raw-pointer address arithmetic and may dereference only elements strictly below the end, exactly as the original array expressions allow.  The linker script's four-byte alignment and its integral multiple-of-four reservation establish the required `u32` alignment for both address boundaries.

`asm-generic/sections.h:35` likewise declares `_stext` and `_etext` as `char[]`; `arch/x86/kernel/vmlinux.lds.S:137,167` defines those address boundaries.  The corrected private anchors and `LOOKUP_START_IP`/`LOOKUP_STOP_IP` macros retain their address-only form.  On this x86_64 task, casting those addresses to `usize` is the same-width representation as the source macros' `unsigned long` casts.

The selected consumer confirms the ownership and ordering contract.  `arch/x86/kernel/unwind_orc.c:333-377` makes `unwind_init()` the sole writer: it derives the block count from the two boundaries, fills each table entry, then sets `orc_init`.  Its normal lookup path reads entries at lines 217-229, and `__unwind_start()` refuses to use the unwinder before `orc_init` at lines 718-719.  The header introduces no lock, atomic, RCU, refcount, allocation, or cleanup mechanism.  Therefore the Rust accessors do not manufacture references, `Send`/`Sync`, or aliasing guarantees; future callers must preserve this existing init-before-read ordering and any source-level synchronization at their call site.

## Final semantic-record closure (recorded here; frozen manifests unchanged)

- `SYMBOLS.tsv` rows 28476-28480: include guard and `LINKER_SCRIPT` condition are preprocessor-only.  The original `vmlinux.lds.S` continues to include the C header while `LINKER_SCRIPT` is defined, leaving only the numeric macros available to that script; the Rust module is not a linker-script input.
- `SYMBOLS.tsv` rows 28481-28482: `LOOKUP_BLOCK_ORDER` and `LOOKUP_BLOCK_SIZE` are C `int` expressions (`8` and `1 << 8`) and are represented as `i32` values with the same values.
- `SYMBOLS.tsv` rows 28483-28484: the two macros are x86_64 `unsigned long` address expressions, represented by hygienic `addr_of!`-based `usize` macros without a Rust reference.
- `SYMBOLS.tsv` rows 28485-28486 and `ABI.tsv` rows 14840-14841: both names are externally linker-defined, incomplete-array boundaries.  They have no Rust object layout or scalar access contract; their exact link names are private zero-size anchors, and their Rust interface is raw address only.  `orc_lookup_end` is one-past-end and must never be dereferenced.
- `LIFETIMES.tsv` rows 14749-14750: both boundaries have static link-image lifetime and no Rust owner or destructor.  The lookup-table storage belongs to the linker-defined `.orc_lookup` region.  `unwind_init()` writes it before `orc_init`; post-init lookup reads it under the original ordering.  No additional ownership, lock, RCU, refcount, or `Drop` rule applies to this header.

The corrected candidate retains exact provenance, contains no Rust test configuration or placeholder, and no unresolved semantic record remains for S000623.
