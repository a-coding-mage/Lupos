# Parity review — S016214 (slot 1)

Reviewed source-only against pinned `vendor/linux` revision
`425f94c2954b1fe80ebdbf9b29854e89750355df`:

- Linux source: `include/uapi/linux/kdev_t.h` (complete 14-line header)
- Candidate: `src/include/uapi/linux/kdev_t.rs`
- Scope: common, x86_64 and aarch64 frozen configurations; task row was
  `REVIEWING` in pipeline `P02` when reviewed.
- Context read: `include/linux/kdev_t.h`, frozen header-closure records, and
  selected consumers through the internal header (`fs/char_dev.c`,
  `block/genhd.c`, `include/linux/fs.h`, and `include/linux/root_dev.h`).

No compiler, formatter, rust-analyzer, build, test, debugger, or runtime tool
was used. No implementation/candidate evidence, Rust review, resolution, other
report, archive, incident, or historical translation was read.

## Findings

### P1 — `__KERNEL__` conditional is omitted, activating legacy UAPI macros in kernel context

Linux lines 4–13 define `MAJOR`, `MINOR`, and `MKDEV` only under
`#ifndef __KERNEL__`. Every frozen kernel compilation context defines
`__KERNEL__`; consequently those 8-bit UAPI macros are absent there. The
immediately relevant internal header, `include/linux/kdev_t.h`, includes this
header and then defines the kernel versions using `MINORBITS` 20 and
`MINORMASK`. Its consumers include `fs/char_dev.c` and `block/genhd.c`, where
the 20-bit `MAJOR`, `MINOR`, and `MKDEV` operations are operative.

The candidate has no equivalent conditional boundary: it always defines and
re-exports the 8-bit forms. That changes the selected kernel configuration
branch from no UAPI macro definitions to active legacy definitions and can
shadow or collide with the internal 20-bit definitions. It is not an include
guard issue: the source condition carries an observable UAPI-versus-kernel
semantic distinction.

### P1 — macro bodies do not preserve C integer promotions, widths, or shift behavior

The candidate comment claims that operand integer width and signedness follow
the UAPI definitions, but its `macro_rules!` bodies apply Rust operators to the
original operand type. C applies the integer promotions before the operators.
For example, with an unsigned-char major operand, Linux
`MKDEV(ma, mi)` promotes `ma` to `int` before `<< 8` and yields an `int`-width
result. `MKDEV!(ma_u8, mi_u8)` instead shifts a `u8` by 8, which cannot express
the C result and has Rust shift-overflow behavior. Likewise `MAJOR` and
`MINOR` retain Rust operand-result widths rather than C's promoted result
width; signedness, negative values, and overflowing/out-of-range shifts also
follow Rust rules rather than the C expression rules.

This applies to all three operative source macros (Linux lines 10–12). Each
candidate macro evaluates each operand once and retains the source grouping,
shift counts, and mask value for already-compatible operand types, but that is
insufficient to preserve the macro contract for the allowed integral operand
types.

### P1 — UAPI macros are crate-private rather than externally visible

Linux explicitly states that these are the externally visible definitions for
programs using the kernel sources. The candidate declares each macro and its
re-export with crate-private visibility (`pub(crate) use`). Thus an external
consumer cannot access `MAJOR`, `MINOR`, or `MKDEV` through this UAPI module.
This narrows the intended UAPI surface independently of the conditional issue.

## Items verified without finding a divergence

- The SPDX identifier is exactly `GPL-2.0 WITH Linux-syscall-note`.
- The immutable provenance records the correct Linux source, pinned revision,
  common architecture membership, and task ID.
- No unauthorized branding is present.
- `MAJOR` and `MINOR` retain the source's `>> 8` and `& 0xff`; `MKDEV` retains
  the source's `<< 8 |` grouping, and every source macro operand is evaluated
  once by the candidate expansion.

## Verdict

Reject pending resolution of all three P1 findings. The candidate is not a
source-parity translation of the selected UAPI conditional branch or its C
integer-expression semantics.
