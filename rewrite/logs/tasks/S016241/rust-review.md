# S016241 Rust review — slot 2

Reviewed independently against the pinned source
`vendor/linux/include/uapi/linux/membarrier.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the current candidate
`src/include/uapi/linux/membarrier.rs`, and sealed semantic proposal
`39d3d3f156de79b106f4470053ea7f5cf99feaa031f7331eb97f69cc6764edf2`.
The bound candidate evidence digest is
`fab668e88976ede3b404e700a2adc1209992b2f258e1828724279ca4dc56aca3`.

## Result

APPROVE — no findings.

## Rust-semantics audit

- The header declares only the two named enum categories and their constants;
  it declares no aggregate, pointer, lifetime, callback, ownership, or
  synchronization contract.  The candidate therefore introduces no `unsafe`,
  FFI function, reference, `Drop`, `Send`/`Sync`, allocation, panic, or
  bounds-check path.
- Both enum member value expressions are C `int` expressions: `0` or `1 << n`
  for `0 <= n <= 9`.  Every result is representable in `i32`, and the Rust
  typed constants retain the exact values without signed overflow or a
  shift-count hazard.  This covers the command records rooted at
  `SC1-fc441458b3c6640a83669f2a96b8354bb6d7ff63010861b877854db631613e0a`
  (aarch64) and
  `SC1-21d2e5c7671cda0e6ff3323866e7c692ebd59f5a2a4235adf8f67154963bbddf`
  (x86_64), plus the flag records rooted at
  `SC1-fffd9423cc3e65978aa2580f5f7ddc5a5ca55892023ae3df4c8a729f970fd7ce`
  and
  `SC1-3e65b5986bd297fbd266bc4513c277892223aca2c808dd5f2c9ce23c9773bd1d`.
- `MEMBARRIER_CMD_SHARED` is a value alias of `MEMBARRIER_CMD_GLOBAL` in the
  C enum (pinned source line 162), not an independent discriminant.  A Rust
  constant alias preserves that compatibility identity without a duplicate
  Rust enum discriminant restriction.
- The source values fit the normal 32-bit syscall command domain; representing
  the command constants as `i32` is consistent with their C `int` expressions.
  No candidate type is emitted as a by-value FFI aggregate, so there is no
  additional layout, alignment, or calling-convention mechanism to audit in
  this header.  The associated ABI proposal keys for both architectures
  (including `SC1-53f288e3a7fda14263b092e5c07fec59a53c1d595df3f038e6019f89e67d7366`,
  `SC1-be2adb34f22b1d4632bf7741ccf01c0c4e0f869acbc4e78fc060da10621d02fa`,
  `SC1-b71f14744e3f0ff023d5b9ccda3f1507b30bb4ac6523ba0afe29c8c7e0d0ba2b`, and
  `SC1-2c303111cc9cc6c59cdf34d1a88794a14d95909d30081257b2cceae03466bea3`)
  remain correctly scoped to those named integer categories.

No compiler, formatter, linker, test, emulator, debugger, or rust-analyzer
diagnostic was run or used.
