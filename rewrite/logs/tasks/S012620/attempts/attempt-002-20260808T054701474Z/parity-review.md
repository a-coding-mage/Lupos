# Parity review — S012620 (slot 1)

Result: **CLEAN — no SC1 findings.**

Frozen inputs: Phase 0 identity `0123...40d2`; immutable queue fingerprint
`cfa8...ee3f`; scope fingerprint `b833...e0a`. Reviewed only the pinned
`include/crypto/dh.h`, its direct pinned helper definitions and callers, the
S012620 queue/scope/symbol/file-map rows, current `src/include/crypto/dh.rs`,
and this task's candidate diff. This was a manual source review; no compiler,
formatter, linker, test, runtime tool, or historical Lupos source was used.

## Coverage and evidence

- **`struct dh` — `include/crypto/dh.h:32-39`:** the candidate's
  `#[repr(C)] struct dh` preserves the exact six-field declaration order:
  `key`, `p`, `g` as `const void *`, followed by `key_size`, `p_size`, and
  `g_size` as `unsigned int`. The corresponding Rust types are `*const
  c_void` and `c_uint`; no replacement container, allocation, padding field,
  or layout-changing representation was introduced. `Copy, Clone` retains
  the C record's ordinary by-value/bitwise-copy semantics and does not own or
  free the pointed-to storage.
- **`struct dh` pointer lifetime/aliasing contract —
  `include/crypto/dh.h:69-80`; `crypto/dh_helper.c:66-92`:** the header says
  decode makes the fields point into the supplied packet buffer, and the
  pinned helper assigns `key`, `p`, and `g` directly from that buffer. The
  candidate preserves these as non-owning raw pointers and accurately records
  the caller-owned-buffer contract; it adds no reference lifetime, drop, or
  allocation behavior.
- **`crypto_dh_key_len` — `include/crypto/dh.h:51`; 
  `crypto/dh_helper.c:34-38`:** declared as `extern "C"` with the exact symbol
  name, `const struct dh *` parameter (`*const dh`), and `unsigned int`
  result (`c_uint`). The GPL-exported linkage in the helper is a definition
  property; the candidate correctly declares, rather than redefines, it.
- **`crypto_dh_encode_key` — `include/crypto/dh.h:66`; 
  `crypto/dh_helper.c:40-64`:** `char *`, `unsigned int`, and `const struct
  dh *` map exactly to `*mut c_char`, `c_uint`, and `*const dh`, with `int`
  return (`c_int`). Direct consumers in `crypto/dh.c:483-490` and
  `security/keys/dh.c:200-206` use this caller-provided mutable buffer
  contract; the candidate preserves it.
- **`crypto_dh_decode_key` — `include/crypto/dh.h:80`; 
  `crypto/dh_helper.c:94-120`:** `const char *`, `unsigned int`, and `struct
  dh *` map exactly to `*const c_char`, `c_uint`, and `*mut dh`, returning
  `c_int`. Direct callers in `crypto/dh.c:75-81`,
  `qat_asym_algs.c:480-483`, and `hpre_crypto.c:639-642` require the mutable
  output record and are consistent with this declaration.
- **`__crypto_dh_decode_key` — `include/crypto/dh.h:95-96`; 
  `crypto/dh_helper.c:66-92`:** the candidate preserves the exact private
  helper symbol and ABI. Unlike the three exported helpers, the pinned
  definition has no `EXPORT_SYMBOL_GPL`; the Rust `pub` declaration creates
  no C definition or export and therefore does not alter that linkage.
- **Copyright, SPDX, and branding:** the candidate retains the exact
  `GPL-2.0-or-later` SPDX identifier and the Intel/Salvatore Benedetto
  copyright/authorship text from `include/crypto/dh.h:1-6`. No branding or
  behavioral text change is present.

`include/crypto/dh.h` contains no executable branch, error path, allocation,
locking, RCU/refcount, or cleanup mechanism beyond the four ABI declarations
and the record above. All selected header content is represented; no shell,
stub, mock, or replacement mechanism was found.
