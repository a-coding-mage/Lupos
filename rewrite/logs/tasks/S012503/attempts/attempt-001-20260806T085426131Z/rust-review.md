# S012503 Rust review

Reviewed `src/include/asm-generic/audit_write.rs` against the pinned
`include/asm-generic/audit_write.h`, its selected direct C inclusion sites, the
frozen x86_64/AArch64 configurations, and the syscall-table sources. This was
a source-only review; no compiler, formatter, linker, or test command was run.

## Finding R1 — exported macros do not preserve the C fragment's inclusion scope or bind a selected caller to its architecture

**Severity: major.**

The upstream file is an unguarded, reincludable token fragment. Each selected C
translation unit includes it directly inside a particular `unsigned` array
initializer: `write_class` in `lib/audit.c:17-20` and
`arch/x86/kernel/audit_64.c:18-21`, `ia32_write_class` in
`arch/x86/ia32/audit.c:16-19`, and `compat_write_class` in
`lib/compat_audit.c:17-20`. Its `#ifdef` decisions consequently come from that
translation unit's selected `<asm/unistd...>` environment.

Lines 11-68 instead create four `#[macro_export]` root-level macros, each
accepting an arbitrary `$consumer:ident`. This adds globally visible macro
names that the C fragment did not export, does not tie a macro to one of the
four selected owning arrays, and allows any future caller on either target to
instantiate any of the four architecture/ABI sequences. The candidate has no
architecture/config gate or caller-side binding that prevents, for example, an
AArch64 native owner from using the IA32 sequence. The required consumer macro
also becomes a new syntactic and macro-hygiene contract absent from the C
initializer fragment.

Resolve by representing the fragment through a mechanism whose use is fixed
at each selected owning translation unit (including its native/compat ABI), or
otherwise make the selected caller/architecture binding explicit and
non-exported. Do not leave four unconstrained crate-root macro entry points as
the file's public interface.

## Checked items

- The retained sequence order and values match the frozen syscall-table
  contexts: x86_64 native, x86 IA32, AArch64 native, and AArch32 compatibility.
  In particular, the AArch64 native table deliberately has only the 64-bit
  `truncate`/`ftruncate` names, while the AArch32 table supplies its distinct
  `truncate64`/`ftruncate64` entries.
- `u32` is the correct width for the selected C `unsigned` array elements on
  both approved architectures; this header owns neither the array storage nor
  the following `~0U` sentinel. There is no `unsafe`, FFI layout, or
  evaluation-order issue within the literal sequences themselves.
- SPDX (`GPL-2.0`) and immutable provenance identify the pinned source,
  revision, architecture scope, and task correctly. No unauthorized branding
  was found.
