# S000686 implementation

- Task: `S000686`
- Pipeline/attempt: `P01` / `1`
- Linux source: `vendor/linux/arch/x86/include/asm/shared/tdx_errno.h`
- Destination: `src/arch/x86/include/asm/shared/tdx_errno.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `x86_64`

The fresh translation preserves the header's complete macro set. ULL status
constants are represented as `u64`; the four unsuffixed operand-ID literals
retain C `int` width as `i32`. The C include guard has no emitted Rust value and
is represented by the module boundary. No conditional implementation branch
is selected beyond the guard recorded in `SYMBOLS.tsv`.

The destination contains all one status-mask macro, nineteen status-code
macros, and four operand-ID macros from the pinned header, with their original
names and values. No compiler, formatter, test, or runtime command was used.
