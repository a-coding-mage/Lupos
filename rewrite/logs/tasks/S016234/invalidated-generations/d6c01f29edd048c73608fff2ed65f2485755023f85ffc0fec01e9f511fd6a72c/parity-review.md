# Parity review — S016234

Reviewed `vendor/linux/include/uapi/linux/major.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/major.rs`.

## Scope checked

- SPDX identifier and immutable source/revision/architecture/task provenance.
- All 139 UAPI device-major macro identifiers, their `int`-representable values,
  aliases, and the `UNIX98_PTY_SLAVE_MAJOR` expression.
- The source header's `_LINUX_MAJOR_H` include guard was correctly not emitted
  as a Rust public constant.
- No extra public major-number constants, substitutions, placeholders, or Rust
  test configuration are present.

## Result

No findings. The candidate contains exactly the 139 UAPI device-major macro
definitions from the Linux header. Each name and value/expression matches after
syntax-only normalization; `HD_MAJOR` retains its alias to `IDE0_MAJOR`, and
`UNIX98_PTY_SLAVE_MAJOR` retains the source expression over the two preceding
constants. All literal values are C `int`-representable and are faithfully
declared as `i32`.
