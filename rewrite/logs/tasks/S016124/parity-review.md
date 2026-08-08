# Parity review — S016124 / slot 1

Candidate: `src/include/uapi/linux/falloc.rs`  
Pinned Linux source: `vendor/linux/include/uapi/linux/falloc.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`  
Scope: common (`x86_64`, `aarch64`); attempt 1, pipeline P02.

## Result

APPROVE — no parity findings.

The complete pinned header has only its single preprocessing include guard and
nine unconditional object-like UAPI macros.  The candidate has exactly the
nine corresponding same-named public constants at
`src/include/uapi/linux/falloc.rs:7-32`:

- `FALLOC_FL_ALLOCATE_RANGE`, `FALLOC_FL_KEEP_SIZE`,
  `FALLOC_FL_PUNCH_HOLE`, and `FALLOC_FL_NO_HIDE_STALE` retain the Linux
  values `0x00`, `0x01`, `0x02`, and `0x04` from
  `include/uapi/linux/falloc.h:5-8`.
- `FALLOC_FL_COLLAPSE_RANGE`, `FALLOC_FL_ZERO_RANGE`,
  `FALLOC_FL_INSERT_RANGE`, `FALLOC_FL_UNSHARE_RANGE`, and
  `FALLOC_FL_WRITE_ZEROES` retain `0x08`, `0x10`, `0x20`, `0x40`, and `0x80`
  from `include/uapi/linux/falloc.h:30,44,61,79,96`.

Each source literal is an unsuffixed hexadecimal integer whose C type is
`int` on both selected architectures; the candidate's `i32` constants retain
the required 32-bit signed integer value and introduce no linkage symbol.
There are no functions, types, storage objects, configuration branches,
allocation paths, locking/refcount/RCU mechanisms, or error paths in the
pinned header.

Linux's `_UAPI_FALLOC_H_` guard (`include/uapi/linux/falloc.h:2-3,98`) only
controls repeated C preprocessor inclusion.  The Rust module boundary supplies
the corresponding one-definition property; it creates no public UAPI item and
does not change any of the nine constants.  The frozen `SYMBOLS.tsv` selection
for both x86_64 and aarch64 contains exactly that guard/endif and those nine
macros, all unconditional.  Candidate provenance matches the pinned source,
revision, common architecture scope, and task ID; no branding delta appears.

No compiler, formatter, linker, test, runtime, diagnostic, historical-source,
or prior-review evidence was used.
