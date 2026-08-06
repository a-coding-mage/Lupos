# Resolution — S013681 / P01

## Reopened evidence

I reopened the complete pinned header
`vendor/linux/include/linux/decompress/unlzma.h:1-13` at Linux revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the operative implementation
`vendor/linux/lib/decompress_unlzma.c:69-78, 93-109, 539-669`, and the shared
decompressor interface and calling context in
`vendor/linux/include/linux/decompress/generic.h:5-34` and
`vendor/linux/lib/decompress.c:51-87`.  The frozen header-consumer commands in
`rewrite/FILE_MAP.tsv:16459,21462` select respectively
`--target=aarch64-linux-gnu` and `--target=x86_64-linux-gnu`, and both contain
`-funsigned-char`.  The task remains `common`, with the source-to-destination
mapping and provenance checked against `rewrite/SCOPE.tsv` row `S013681` and
`vendor/linux.SHA`.

## Findings and dispositions

| Finding | Disposition |
| --- | --- |
| R1: the candidate used target-native `c_char` for `void (*error)(char *)`, despite both frozen consumers compiling C `char` with `-funsigned-char`. | Applied. `error` now has the non-null C-ABI type `unsafe extern "C" fn(*mut c_uchar)`. This retains raw-pointer provenance and the frozen unsigned-byte pointee contract without introducing a Rust reference or ownership transfer. |
| Parity review: no findings. | Confirmed. The corrected declaration retains symbol `unlzma`, C ABI, `int` return, raw mutable `unsigned char *` / `long *` parameters, LP64 `long`/`unsigned long`, and all callbacks' original argument and return types. |

The `error` function pointer must remain non-null: the implementation stores it
without a check and calls it on allocation, header, EOF, and corrupt-input
paths. `fill` and `flush` must remain nullable `Option<unsafe extern "C" fn>`
values: the implementation tests and conditionally calls each (`rc_init` and
the final flush path). `buf`, `output`, and `posp` stay raw nullable pointers;
the source branches on all three, and `posp` is written only when non-null.

## Pending-review closure

The only S013681 `PENDING_REVIEW` entries in `rewrite/SYMBOLS.tsv` are the
two architecture instances of the include-guard conditional pair and
`DECOMPRESS_UNLZMA_H` macro. They are now resolved as non-operative C
multiple-inclusion machinery represented by this one Rust module; they impose
no emitted Rust symbol, layout, ownership, ABI, locking, or lifetime behavior.
The task has no S013681 rows in `rewrite/LIFETIMES.tsv`, `rewrite/ABI.tsv`,
`rewrite/DRIVER_ABI.tsv`, or `rewrite/BLOCKERS.tsv`. The function ABI and all
pointer/callback lifetime decisions are closed above from pinned source and
the frozen commands; no semantic PENDING_REVIEW remains for this task.

No compiler, formatter, rust-analyzer diagnostic, build, link, test, runtime,
debugger, or benchmark was invoked. This is a source-review closure only.
