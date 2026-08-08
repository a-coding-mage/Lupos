# Implementation evidence — S016150

- Task: `S016150`
- Pipeline/attempt: `P02` / `1`
- Source: `vendor/linux/include/uapi/linux/hsr_netlink.h`
- Destination: `src/include/uapi/linux/hsr_netlink.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Architecture: `aarch64`

The fresh translation preserves the UAPI SPDX expression and copyright-relevant
header identity through provenance, then maps both anonymous C enums to public
signed C-int constants with the exact ordinal values from the pinned header.
The `HSR_A_MAX` and `HSR_C_MAX` macros remain expressions over their internal
`__HSR_*_MAX` constants, preserving their source dependency and value domain.
No configuration branch, structure, function, test, driver, or placeholder is
present in the pinned source file.
