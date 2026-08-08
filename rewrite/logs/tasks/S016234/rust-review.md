# Rust semantics review — S016234 attempt 2, slot 2

## Result

APPROVE — no Rust-semantics findings.

Reviewed the sealed current-attempt proposal
`d8976d8c877bd25099551e653623d8a36fe524bd47bd962b95fd22c4ba1d094a`,
the current candidate summary digest
`3e1dbcffdc528f998e98ebd455723af10e20231caede3d4091c9ff69d7ea4b32`,
and `src/include/uapi/linux/major.rs`
(`5654a7ecce976fb0b02ff96c61208b26676bbeadfba41deec158ad6c6753e525`).

## Source-to-Rust semantics

- The pinned source is an object-like macro-only UAPI header:
  `vendor/linux/include/uapi/linux/major.h:10-176`.  Its 139 non-guard
  definitions have direct same-name public Rust constants; source/candidate
  accounting leaves only the C include-guard macro `_LINUX_MAJOR_H` unmapped.
  The guard has no Rust item, matching its C preprocessing-only role.  The
  selected common scope and both architecture-specific guard records are
  closed by proposal keys
  `SC1-b6d127d264de9711a181c9115ae5da495aa9402f047fad1a2269e23d92aee22f`,
  `SC1-5a7398bbccd7a70836df244538704903ba9e2da7524cd6193fe4848a01603389`,
  `SC1-d8ca9332a74eb553b0632c0a1f94388fb48a9c966229d6592c6974ad2c1eeed6`,
  `SC1-740102c75dff911b18f7e8a0953f0d10da4e7015bf6efe19b070a3d823f5b019`,
  and `SC1-6c95eeaa7294e78bbb04ce477b26cedb6eb134a13600a9d018df73fc6d5461f5`.
- All C numeric replacements are unsuffixed decimal integer constants no
  larger than 260.  On each frozen target the C category is `int`; Rust `i32`
  therefore preserves the value and signed integer category with neither
  truncation nor overflow/panic path.  The sealed rows cover each selection
  expression for `aarch64` and `x86_64`; the task scope records both frozen
  configurations as header-closure consumers, so `//! architectures: common`
  is supported rather than architecture-specific.
- `HD_MAJOR` remains a named alias of `IDE0_MAJOR`, exactly as pinned source
  line 16, not a duplicated literal.  Its corresponding selected value/status
  proposal keys are
  `SC1-1163d4cfadf7858826fa90b6311c39788f754a2c4e2d395da98783b04c7088f1`,
  `SC1-83701e7fc4e85ebfd220c9386996d1f491d6ca03e237fc9b1ab6370bed28e1d6`,
  `SC1-4062ab02bbd7a8349e12b56b86ed81e7c5cad1219866f96ba478df076406016a`,
  and `SC1-7e737d22382fa0a298260e83f00dd61d32f829c651973cab93b259dfab1a512d`.
- `UNIX98_PTY_SLAVE_MAJOR` remains the named addition of
  `UNIX98_PTY_MASTER_MAJOR` and `UNIX98_PTY_MAJOR_COUNT`, matching source
  line 147.  Both inputs and the sum are positive `int` values (128 and 8),
  hence the Rust `i32` constant expression has the same evaluated value, 136,
  with no changed promotion, wrapping, or panic behavior.  Its proposal keys
  are `SC1-385b4a04f2704c8e1b82a0faa53be41c0558f48cda1df84b647f7fb342a8b1d5`,
  `SC1-1959b9773c900541fe3ea06b778f620cf4e6437ab545fa3d3188285b716e77c0`,
  `SC1-95526247a5a6988f07d11465e1f82ad319d7f07a1f97bc37c6975454eaf39cac`,
  and `SC1-c80c4eebf75e0ac76e38bf15c743fd7bb84a90d2340d6cf8dc067b0a81cfff8e`.
- This header declares no objects, functions, layouts, FFI symbols, mutable
  state, ownership, or unsafe operation.  `pub const` introduces no runtime
  allocation, linkage symbol, `Drop`, bounds check, or panic path.  The
  candidate preserves the UAPI identifiers and SPDX expression, plus exact
  pinned-source/revision/task/common-architecture provenance.

No compiler, formatter, linker, test, rust-analyzer diagnostic, or runtime
command was used in this source-only review.
