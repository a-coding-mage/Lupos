# Application resolution — S016214

Pinned source reopened: `vendor/linux/include/uapi/linux/kdev_t.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`.

Frozen x86_64 and AArch64 Kbuild command records each carry `-D__KERNEL__`.
Therefore, for every selected kernel context, source lines 4--13 are excluded:
this UAPI header contributes no `MAJOR`, `MINOR`, or `MKDEV` definitions.  The
immediately following kernel provider, `include/linux/kdev_t.h`, includes this
header and supplies distinct `MINORBITS == 20` / `MINORMASK` forms.  Its
in-kernel consumers consequently must not receive the UAPI 8-bit macros.

## Finding dispositions

1. **Parity P1 / Rust RUST-1 (`__KERNEL__` condition): accepted.**  The
   candidate unconditionally defines all three macros, contrary to both frozen
   architecture contexts.  No authoritative Rust configuration mechanism or
   module-boundary mapping was recorded that can faithfully select the UAPI
   non-kernel branch while leaving every selected kernel inclusion empty.

2. **Parity P1 / Rust RUST-2 (C integer domain): accepted.**  The three C
   replacement lists accept the C integer operand domain and apply C integer
   promotions; `MKDEV` also applies the usual arithmetic conversions before
   `|`.  The proposed `macro_rules!` expansions instead use Rust operator
   traits and operand types.  In particular, a C `unsigned char` operand is
   promoted to `int` before `<< 8`, whereas a direct Rust `u8 << 8` cannot
   preserve that expression.  No header-local type domain, conversion rule,
   overflow/shift policy, or ABI record authorizes choosing a Rust numeric
   type or cast.  Exposing the macros only for selected compatible types would
   narrow the UAPI contract; a trait-based form would broaden it and still not
   reproduce C promotions.

3. **Parity P1 / Rust RUST-3 (visibility): accepted.**  The source documents
   these as externally visible non-kernel definitions.  `pub(crate)` narrows
   that surface, while `pub` would incorrectly expose it in the selected
   kernel context unless an authoritative condition mapping exists.

## Outcome

No source amendment is applied.  Replacing the candidate with an empty
kernel-only module would correctly model the selected `__KERNEL__` branch but
would silently discard the source header's externally visible non-kernel UAPI
contract; retaining or publicizing the candidate macros changes branch
selection and integer semantics.  This task is therefore **BLOCKED** pending
an authoritative, project-wide UAPI boundary and C-integer-expression mapping.

The S016214 `PENDING_REVIEW` symbol records remain unresolved by design: the
blocker is exactly the semantic mapping required to close them.  No ABI,
lifetime, or source manifest was hand-edited, and no compiler, formatter,
rust-analyzer, build, test, or runtime tool was used.
