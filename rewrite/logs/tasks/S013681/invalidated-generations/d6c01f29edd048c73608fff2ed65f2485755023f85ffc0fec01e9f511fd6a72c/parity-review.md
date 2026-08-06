# Parity review — S013681 / P01 / slot 1

## Scope and evidence

Reviewed only `src/include/linux/decompress/unlzma.rs` against pinned
`vendor/linux/include/linux/decompress/unlzma.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, plus the pinned implementation
entry point in `vendor/linux/lib/decompress_unlzma.c:539-674` and the generic
decompressor contract in `vendor/linux/include/linux/decompress/generic.h:5-34`
and `vendor/linux/lib/decompress.c:51-87`.  The frozen ABI/lifetime records for
the common `decompress_fn` contract (S013677, both architectures) were also
consulted.  Branch was `feat/bun-like-rewrite-test`; the queue row was
`REVIEWING`, task path and Linux path matched S013681, and `vendor/linux.SHA`
matched the checked-out vendor tree.

## Result

No parity findings.

`unlzma` retains the exact C symbol name, C calling convention, return `int`,
and the LP64 `long`/`unsigned long` widths required on both x86_64 and AArch64.
Its `unsigned char *`, `long *`, and `void *` parameters remain raw mutable
pointers.  `fill` and `flush` correctly use nullable `Option<unsafe extern
"C" fn(*mut c_void, c_ulong) -> c_long>` values, matching the generic contract
that permits NULL callbacks.  The `error` callback retains the C `char *`
argument through target-correct `c_char`, preserving architecture-specific C
`char` signedness; the implementation invokes it directly on its error paths,
so its non-null callable contract is correctly represented.  The declaration
does not transfer ownership, add allocation, or extend any buffer/callback
lifetime, and it leaves the implementation's conditional `posp` write,
position value, error return, and cleanup behavior untouched.

No compiler, formatter, rust-analyzer, build, test, or runtime diagnostic was
run.
