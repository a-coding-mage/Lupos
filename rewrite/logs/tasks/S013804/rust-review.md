# Rust review — S013804

Reviewer: `rust_reviewer`  
Model/effort: `gpt-5.6-terra` / `high`  
Scope inspected: `vendor/linux/include/linux/dsa/brcm.h` in full; frozen
aarch64 configuration and selected use sites; `src/include/linux/dsa/brcm.rs`.
No compiler, formatter, rust-analyzer, build, test, or runtime tooling was
used.

## Result: reject pending application

### R1 — `BRCM_TAG_SET_PORT_QUEUE` changes the source macro's precedence

**Severity: high**

Linux defines the operative macro at
`vendor/linux/include/linux/dsa/brcm.h:12` as:

```c
((p) << 8 | q)
```

The `q` parameter is intentionally not parenthesized in that replacement
list. Therefore an expression supplied as `q` continues to participate in C
operator precedence: for example, a `q` expression containing `&` binds before
the macro's `|`. The candidate at `src/include/linux/dsa/brcm.rs:20` instead
expands `q` as `(($q) as u32)`, which necessarily groups it before the `|`.
That changes both the value and the macro interface for valid expression
arguments, despite evaluating the operand once in both versions.

The frozen selected call in `net/dsa/tag_brcm.c:130` currently supplies the
plain `u16 queue` local, but the task inventory records the macro itself as an
operative selected symbol. The fresh header translation must retain the pinned
macro's expression semantics rather than specialize its token interface to
this one call site.

### R2 — Rust's checked left shift can panic where the C unsigned shift wraps

**Severity: high**

`dp->index` is `unsigned int` (`vendor/linux/include/net/dsa.h:261`), so the
left operand of the source macro at `brcm.h:12` is an unsigned 32-bit value on
the frozen aarch64 target. C defines its left shift in that unsigned type,
with the result reduced modulo 2^32. The candidate's `(($p as u32) << 8)` at
`src/include/linux/dsa/brcm.rs:20` uses Rust's ordinary `u32` shift. Its
overflow checking is build-profile dependent and panics for an overflow in a
checked build; it therefore does not provide the source macro's total unsigned
wrapping operation.

The existing DSA topology ordinarily uses small port indices, but neither the
macro nor its Rust expansion encodes that as a type invariant. Exact
translation cannot introduce a profile-dependent panic for an unsigned C
operation. The application must express the 32-bit wrapping shift explicitly
while keeping the C expression/argument semantics from R1.

## Other Rust/FFI observations

The header declares no functions, data objects, layouts, ABI symbols, FFI
types, pointers, ownership transfer, or `unsafe` operations; consequently
there is no independent layout, aliasing, `Send`/`Sync`, or unsafe-boundary
finding. `BRCM_TAG_GET_PORT` and `BRCM_TAG_GET_QUEUE` receive the selected
`u16` queue mapping (`net/dsa/tag_brcm.c:94`,
`drivers/net/ethernet/broadcom/bcmsysport.c:2268`) and their source integer
promotions produce the same numerical fields for that concrete use. This does
not cure the two macro-definition mismatches above.
