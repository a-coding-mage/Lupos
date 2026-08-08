# Parity review — S016417 (attempt 1, slot 1)

Status: FINDINGS

Reviewed only `vendor/linux/include/uapi/linux/thermal.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, the frozen S016417 rows, the
candidate diff, and the current candidate. No compiler, formatter, test, or
historical source was used.

## Findings

1. **F1 — C string-literal macro representation is changed (blocking).**
   Linux symbols `THERMAL_GENL_FAMILY_NAME`,
   `THERMAL_GENL_SAMPLING_GROUP_NAME`, and `THERMAL_GENL_EVENT_GROUP_NAME`
   are preprocessor macros whose replacements are respectively the C string
   literals `"thermal"`, `"sampling"`, and `"event"`
   (`thermal.h:22,24-25`).  A C string literal provides character storage with
   a terminating NUL and is usable in a C `char *` initializer; the direct
   pinned caller `drivers/thermal/thermal_netlink.c:19-20,906` uses these
   exact macro expansions for `genl_multicast_group.name` and
   `genl_family.name`.  The candidate instead declares each as `&str`.
   `&str` is a Rust string slice (data pointer plus length), has no C-string
   ABI, and its value does not include the required NUL terminator.  This
   changes the macro expression/value and cannot preserve the Linux caller
   contract.  Represent the macros with NUL-terminated byte storage and an
   explicitly appropriate C-facing expression/contract.

2. **F2 — all selected C enum constants lost their unscoped public names
   (blocking).**  Linux declares every enumerator in
   `thermal_device_mode`, `thermal_trip_type`, `thermal_genl_attr`,
   `thermal_genl_sampling`, `thermal_genl_event`, and `thermal_genl_cmd` as a
   header-scope C identifier (for example `THERMAL_DEVICE_DISABLED` at
   `thermal.h:10` and `THERMAL_TRIP_CRITICAL` at `thermal.h:18`).  The frozen
   symbol inventory selects each of those `enum_constant` rows for both
   x86_64 and aarch64.  The candidate makes them Rust enum variants, so every
   name is available only through a type-qualified path such as
   `thermal_device_mode::THERMAL_DEVICE_DISABLED`; it exports no module-scope
   `THERMAL_*` constant matching the Linux identifier.  This is an observable
   source/linkage API change, not merely style: direct pinned caller
   `drivers/thermal/thermal_core.c:229,628,634` uses
   `THERMAL_DEVICE_DISABLED` and `THERMAL_DEVICE_ENABLED` bare, and the same
   file uses `THERMAL_TRIP_CRITICAL` and related values bare at
   `:233,350,403,418`.  Provide public Linux-named constants with the same
   values and preserve their usable C-style expression behavior for every
   selected enumerator (including the leading-double-underscore sentinel
   names).

## Checked without a finding

The candidate retains all six enum declarations, their selected enumerator
spellings and ordinal values, the four derived `*_MAX` arithmetic values, and
the integer macro values from `thermal.h:5-7,23,59,65,91,108`.  No selected
conditional branch, implementation body, lock/refcount/error path, or
allowlisted branding delta exists in this header.
