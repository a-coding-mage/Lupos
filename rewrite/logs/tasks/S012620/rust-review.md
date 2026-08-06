# Rust review — S012620

Reviewed `src/include/crypto/dh.rs` against pinned
`vendor/linux/include/crypto/dh.h` and the concrete decode contract in
`vendor/linux/crypto/dh_helper.c`.  Review scope was Rust ownership, raw
pointer/FFI ABI, layout, and unsafe boundaries only.  No source, build, test,
or formatting command was run.

## Finding RUST-001 — blocking: Rust 2024 requires an unsafe extern block

`Cargo.toml` selects `edition = "2024"`, but lines 26–43 use `extern "C" {`
without the required `unsafe` qualifier.  Rust 2024 makes a bare extern block
an error, so this header cannot be compiled as written.  The declarations also
accept raw pointers whose validity, pointed-to byte ranges, and lifetime cannot
be checked by Rust.  The declaration boundary must therefore be marked unsafe,
and the individual function declarations must not provide a safe call surface
unless their respective C preconditions have been represented and enforced.

Required resolution: use the Rust-2024-valid unsafe extern-block syntax and
preserve the C caller obligations for all four raw-pointer functions.

Evidence: `Cargo.toml:4`; `src/include/crypto/dh.rs:26-43`;
`vendor/linux/include/crypto/dh.h:51,66,80,95-96`; and
`vendor/linux/crypto/dh_helper.c:40-120`.

## Checks with no finding

- `#[repr(C)]`, three `*const c_void` fields, and three `c_uint` fields retain
  the AArch64 C declaration order, pointer constness, and integer widths.
- The `c_int` returns and `c_char` buffer pointers retain the declared C ABI;
  the frozen AArch64 compile command uses unsigned `char`, and the pointer
  representation is unchanged by the signedness of the byte element type.
- Raw pointers intentionally encode no Rust ownership, exclusivity, or
  lifetime.  This correctly permits the C decode behavior where `key`, `p`,
  and `g` borrow/alias ranges inside `buf`, including aliasing with other raw
  arguments permitted by C.
- No broad unsafe block, Rust reference conversion, ownership transfer, or
  `Send`/`Sync` assertion was introduced.

Disposition: reject pending resolution of RUST-001.
