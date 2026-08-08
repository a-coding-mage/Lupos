# Resolution — S013591 / attempt 1

Result: `BLOCKED`.

The sealed candidate was rechecked against the complete pinned
`vendor/linux/include/linux/circ_buf.h`, direct selected consumers
`vendor/linux/drivers/input/serio/userio.c` and
`vendor/linux/kernel/events/ring_buffer.c`, the candidate and candidate
snapshot, both independent reports and attestations, and the frozen
`SCOPE.tsv`, `SYMBOLS.tsv`, `ABI.tsv`, `LIFETIMES.tsv`, and header-closure
records.  No compiler, formatter, test, analyzer, historical Lupos source, or
runtime evidence was used.  The candidate is sealed; no source edit was made.

## P1-INTEGER and R1/R2 — unresolved C arithmetic and fixed `int` semantics

Accepted.  `circ_buf.h:16,21,26-35` is a macro interface whose ordinary
arithmetic uses C's usual arithmetic conversions; the two GNU statement
expressions additionally declare `int end` and `int n`.  The sealed Rust
macros instead call receiver-typed `wrapping_sub`/`wrapping_add` and infer the
temporary types.  They do not establish the required C promotions,
signed/unsigned balancing, fixed `int` temporary conversions, resulting signed
comparison domain, or C operand-evaluation contract.

This is reached by selected consumers with incompatible source domains:
`userio.c:35-36,122-123` passes `u8` head/tail and the integer
`USERIO_BUFSIZE` to `CIRC_CNT_TO_END`, while
`ring_buffer.c:142-149,438` uses `unsigned long` state and an `unsigned int`
size with `CIRC_SPACE`.  The frozen records enumerate the macro text and these
selected header consumers, but provide no frozen Rust representation or
conversion boundary that can preserve all of those C domains.  Without that
bridge, changing the sealed source to an ad hoc `i32`, `u32`, or generic Rust
contract would be an unreviewed semantic design.  The affected macro closure
records therefore cannot be committed as `COMPLETE`.

## P1-GUARD — include guard has no established Rust equivalent

Accepted.  `circ_buf.h:6-7,37` uses `_LINUX_CIRC_BUF_H` to suppress repeated
C declarations and macro definitions in one translation unit.  `SYMBOLS.tsv`
retains both architecture-specific conditional and operative-macro records as
`PENDING_REVIEW`.  The sealed candidate exports global macros but the frozen
scope, file map, and consumer metadata do not establish a Rust module/import
or macro-export mechanism with the selected C duplicate-include behavior.
Treating Rust's current module behavior as equivalent would be an unsupported
semantic decision, so the associated guard records remain unresolved.

## R3 — `struct circ_buf.buf` character representation and derived ABI/lifetime fields

Accepted.  The pinned declaration at `circ_buf.h:9-13` is `char *buf`, while
the sealed candidate uses `*mut core::ffi::c_char`.  The frozen ABI and
lifetime rows for both architectures retain layout, alignment, export kind,
ownership, lifetime contract, and locking/refcount fields as
`PENDING_REVIEW`; they do not establish the required target character
signedness/element representation or authorize the `c_char` alias as the
Linux field's exact mapping.  `#[repr(C)]` and `i32` fields are necessary but
do not independently close that contract.  No source-backed replacement is
available in the frozen records, so the related ABI and lifetime closure keys
remain unresolved.

The task must remain blocked pending a frozen, source-derived Rust bridge for
the macro arithmetic/evaluation and import semantics, and an ABI decision for
the character-pointer field.  No placeholder or simplified replacement is
permitted.
