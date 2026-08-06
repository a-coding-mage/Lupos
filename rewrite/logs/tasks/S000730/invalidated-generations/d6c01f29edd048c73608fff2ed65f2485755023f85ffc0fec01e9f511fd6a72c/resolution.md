# Applier resolution — S000730

## Evidence reopened

I independently reopened the complete pinned source
`vendor/linux/arch/x86/include/asm/trapnr.h` at revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate, both
independent reviews, the frozen x86_64 configuration, the Phase 0 identity,
the S000730 scope and symbol records, and the header-closure/include-edge
metadata.  The selected source is an unconditionally defined, macro-only
header: it has eight event-type literals (lines 8--15), 24 trap-number literals
(lines 19--42), and only a C include guard (lines 2--3 and 44).

`CONFIG_X86_64=y` and `CONFIG_X86_FRED` is unset in the frozen configuration.
That disabled configuration is a property of particular FRED consumers, not a
conditional in this header.  The candidate correctly retains every event-type
constant, including `EVENT_TYPE_OTHER`; no source selection may remove it.

## Review dispositions

1. **Parity review, accepted.**  Its exhaustive macro comparison is correct:
   `EVENT_TYPE_EXTINT` through `EVENT_TYPE_OTHER` map one-for-one to `0..=7`;
   `X86_TRAP_DE` through `X86_TRAP_CP` map one-for-one to `0..=21`; and the
   non-contiguous `X86_TRAP_VC=29` and `X86_TRAP_IRET=32` are retained.  No
   conditional, linkage, layout, allocation, locking, cleanup, or runtime
   behavior was omitted.  The original C header remains the immediate token
   provider for preserved Linux assembly; this Rust header creates no new
   exported object or replacement ABI.
2. **Rust review, accepted.**  The source replacement lists are unsuffixed
   decimal literals whose values fit signed C `int` on the frozen
   `x86_64-linux-gnu`, `-m64` target.  `pub const ...: i32` preserves that
   source width and signedness for the translated constant surface.  The
   header has no expression evaluation, pointer, ownership, FFI object,
   unsafe, allocation, panic, `Drop`, or synchronization contract; translated
   consumers must still express their own C conversion rules where applicable.
3. **Independent applier check, accepted without source change.**  The current
   Rust file has exactly the required immutable provenance and all 32 public
   constants with matching names and values.  It adds no test, stub, panic,
   unsafe block, branding change, or configuration-dependent substitute.

## Semantic-record closure

The S000730 scope row and all symbol rows are resolved.  The two conditional
rows plus `_ASM_X86_TRAPNR_H` are C preprocessing-only include-guard mechanics
and have no Rust item, linkage, or ABI counterpart.  Every remaining symbol
row records its unconditional signed-C-`int` literal value and is mapped to the
candidate `i32` constant.  `ABI.tsv` and `LIFETIMES.tsv` contain no S000730
rows: this header defines no ABI-bearing function/type/object and has no
storage, ownership, lifetime, locking, RCU, refcount, or cleanup family to
record.  This is an explicit N/A closure, not an omitted pending record.

No source amendment is required.  S000730 is ready for `DONE` as a
translation-pipeline result only; no compile, link, test, formatting, runtime,
or parity-proof claim is made.
