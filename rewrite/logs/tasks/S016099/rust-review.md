# Rust source review — S016099 (slot 2)

Reviewed the current candidate `src/include/uapi/linux/dev_energymodel.rs` against pinned Linux revision `425f94c2954b1fe80ebdbf9b29854e89750355df`, `include/uapi/linux/dev_energymodel.h`, its generated YNL specification, the aarch64 frozen context, and the current sealed task closure proposal. This was manual source inspection only; no compiler, formatter, linker, test, or rust-analyzer diagnostic was invoked.

## Result: FINDINGS

### RUST-001 — C string-literal macros became non-FFI Rust `&str` values

`DEV_ENERGYMODEL_FAMILY_NAME` and `DEV_ENERGYMODEL_MCGRP_EVENT` are C string-literal macros at upstream lines 10 and 80. Each expansion has a trailing NUL byte, decays to a thin `char *` where required, and retains C string-literal initialization semantics. The candidate instead exposes `&str`, a Rust fat reference (data pointer plus length) whose referenced bytes do not include the C terminator. This is not a representation-compatible replacement for the UAPI macros. The selected upstream consumer initializes `struct genl_family.name` from `DEV_ENERGYMODEL_FAMILY_NAME` (`kernel/power/em_netlink_autogen.c:52`); that destination is `char name[GENL_NAMSIZ]` (`include/net/genetlink.h:78-81`).

The string macro mappings must preserve their NUL-terminated byte content and thin C-character-array/pointer use, without relying on a Rust `&str` representation.

Affected SC1 records:

- `SC1-765c152908291fa365030afa9b47a9081f5795d0e5b1122f3c5ab34e2be019ed` — `SYMBOLS.tsv:357486`, `DEV_ENERGYMODEL_FAMILY_NAME`, `selection_expression`
- `SC1-4370bf84a0062b1c58ad6cd55686213db529f6bea741fdb78b1662bbe9b71ae4` — `SYMBOLS.tsv:357488`, `DEV_ENERGYMODEL_MCGRP_EVENT`, `selection_expression`

### RUST-002 — Named C flag enums were changed into scoped, closed Rust enums

Upstream lines 19-21 and 33-37 define C enum tags while placing their enumerators in the enclosing C identifier namespace. The corresponding YNL definitions are `flags`, and the generated protocol carries those flag sets in `u64` attributes (`Documentation/netlink/specs/dev-energymodel.yaml`; `kernel/power/em_netlink.c:59-60,230-232`). The candidate replaces the identifiers with variants scoped under differently named `DevEnergymodel*` Rust enums. Thus the required global identifiers such as `DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_MICROWATTS` no longer exist as the upstream header exposes them. It also substitutes a closed Rust-enum value domain for Linux integer flag values, which must permit combined bits and unrecognized extension bits when represented as the protocol's `u64` bitmask.

The applier must preserve the original public enumerator identifier scope and integer/bitmask semantics. If retaining named Rust type surfaces is necessary, they must not narrow the value domain or replace the public constants; their exact ABI/layout must also be established rather than inferred from an idiomatic Rust enum.

Affected SC1 records:

- `SC1-f7b0188adf2ae12788684d6d0bee8d49be81d618fe9fabd8d4bdb14f2b64aa95` — `SYMBOLS.tsv:357489`, `enum dev_energymodel_perf_state_flags`, `selection_expression`
- `SC1-4cad72babcfdcf2853ce59e89807501cbcd76a510bf434fdbe8a7ee11f1fa3d7` — `SYMBOLS.tsv:357490`, state flag enumerator, `selection_expression`
- `SC1-7e79bb18179c34d547b9e64dfa21720964330409d2e8c4031d44327ad53f4377` — `SYMBOLS.tsv:357491`, `enum dev_energymodel_perf_domain_flags`, `selection_expression`
- `SC1-bb0c154b10f75bd484baef174256e815ecd3818b57776b31cfc3a136c5333c94`, `SC1-0a62eaba114f745cccda38767cc64149fc47764e261f8a039e6226739d696cce`, `SC1-315b5fa8495d6e414681069796e440aabccbe56497b42cfd65d5f372e9165859` — `SYMBOLS.tsv:357492-357494`, domain-flag enumerators, `selection_expression`
- `SC1-4be922003ba74922eecf93a71a425e99978c1d7891bc4276030f3a51065ec5d2`, `SC1-9649ae1b894344d34a690836093c746eb3b2c67423a941499154b9e41705e4ce`, `SC1-8961f1d0bcd230797d1ae9303cede531f18a290945652a502a73d1c0fd05b8d4` — `ABI.tsv:190230`, state-flag enum alignment/export-kind/layout
- `SC1-1194e48c4dd892d71a77f9718cb73185eaaa651345bdc486034d734e61af3b45`, `SC1-d1eccea781ae6903949566a973fd355942ca5a448f7c6b9141c9a4a18d8f8665`, `SC1-682dff7ef6591f8003744f06ef0fe629879a3a5db5e1efc253e7920cf7b42f9d` — `ABI.tsv:190231`, domain-flag enum alignment/export-kind/layout

## Additional manual checks

- Provenance path, revision, architecture, task ID, SPDX expression, numeric values, anonymous-enum ordinal values, and max-sentinel arithmetic match the pinned header.
- The candidate contains no `unsafe`, raw-pointer operation, allocation, callback, synchronization primitive, `Drop`, panic/unwrap/expect path, FFI declaration, packed layout, or executable control flow to audit beyond the representation findings above.
- The header's selected definitions are unconditional after its C include guard; no frozen configuration conditional was omitted.

These findings reject the candidate as submitted. No source file was changed.
