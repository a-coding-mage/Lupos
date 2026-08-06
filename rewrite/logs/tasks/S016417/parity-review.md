# S016417 parity review (slot 1)

Reviewed `vendor/linux/include/uapi/linux/thermal.h` at pinned revision
`425f94c2954b1fe80ebdbf9b29854e89750355df` against
`src/include/uapi/linux/thermal.rs`.

## Result

Accepted: no parity findings.

## Checked source surface

- The candidate retains the exact UAPI SPDX identifier and immutable provenance
  for `include/uapi/linux/thermal.h`, the pinned revision, `common`, and
  `S016417`.  No branding delta is present or allowlisted.
- The only source conditional is the C multiple-inclusion guard; the source
  has no configuration- or architecture-dependent branch.  Its absence from
  the Rust module has no runtime/API counterpart.  The candidate introduces
  no configuration branch.
- All six source enum tags are represented with transparent `c_int` storage:
  `thermal_device_mode`, `thermal_trip_type`, `thermal_genl_attr`,
  `thermal_genl_sampling`, `thermal_genl_event`, and `thermal_genl_cmd`.
  The candidate retains every 70 enumerator with its sequential C value,
  including all four double-underscore sentinels.
- All eleven value macros are retained: `THERMAL_NAME_LENGTH`, both threshold
  direction flags, `THERMAL_GENL_VERSION`, and the four `*_MAX` expressions.
  Each derived maximum still resolves to its immediately preceding sentinel
  minus one (27, 0, 19, and 10 respectively).
- The three string-literal macros retain their exact character sequences and
  terminating NULs: `"thermal"` (8 bytes), `"sampling"` (9 bytes), and
  `"event"` (6 bytes).  Immutable static `c_char` arrays correctly supply
  the string-literal storage/decay role without changing their byte values.

No source, build, formatting, test, or runtime command was run during this
review.
