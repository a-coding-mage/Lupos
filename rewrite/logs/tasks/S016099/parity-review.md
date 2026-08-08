# Parity review — S016099 / slot 1

Scope reviewed: the complete pinned `include/uapi/linux/dev_energymodel.h` at
`425f94c2954b1fe80ebdbf9b29854e89750355df`, frozen aarch64 selection, the
current candidate and candidate summary, and the direct local consumers
`kernel/power/em_netlink.c` and `kernel/power/em_netlink_autogen.c` plus the
generic-netlink declarations.  This was a manual source review only; no
compiler, formatter, test, or diagnostic output was used.

## Findings

### P1 — The two named C flag enums no longer provide their unscoped C symbols or open flag-value semantics

Linux symbols: `enum dev_energymodel_perf_state_flags`,
`DEV_ENERGYMODEL_PERF_STATE_FLAGS_PERF_STATE_INEFFICIENT`,
`enum dev_energymodel_perf_domain_flags`,
`DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_MICROWATTS`,
`DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_SKIP_INEFFICIENCIES`, and
`DEV_ENERGYMODEL_PERF_DOMAIN_FLAGS_PERF_DOMAIN_ARTIFICIAL`.

Local evidence: the pinned header declares those as C enum tags with ordinary,
unscoped enumerator identifiers at lines 19--21 and 33--37.  The candidate at
lines 15--25 instead places the four names only in the Rust namespaces
`DevEnergymodelPerfStateFlags` and `DevEnergymodelPerfDomainFlags`, and also
renames the two C tag identifiers.  Thus the original module-level UAPI names
are absent.  More importantly, the C domain values are bit flags (`1`, `2`,
and `4`) and remain freely composable integral values; a Rust fieldless enum
only admits its declared discriminants.  The direct EM implementation confirms
the flag mechanism: `kernel/power/energy_model.c:527` ORs the skip flag into
`pd->flags`, and lines 666--685 independently OR domain flags before retaining
them.  The candidate therefore substitutes a closed enum mechanism for the
source's open integer flag namespace.

Required correction: preserve the C-facing tag/name contract and expose all
four enumerators at their original unscoped names as composable integral flag
values, with any Rust nominal type unable to exclude valid combined values.

Semantic record keys:

- `SC1-f7b0188adf2ae12788684d6d0bee8d49be81d618fe9fabd8d4bdb14f2b64aa95`, `SC1-235269b6a35577f2d848e1ecc1414293b431ae2023580c8f8f6046dac42966b0`, `SC1-4cad72babcfdcf2853ce59e89807501cbcd76a510bf434fdbe8a7ee11f1fa3d7`, `SC1-b4e2ba688451d7c53bba8aba43a5b5358513c908a8f3151bc7ea42ae3e23ac28`
- `SC1-7e79bb18179c34d547b9e64dfa21720964330409d2e8c4031d44327ad53f4377`, `SC1-3ee531347b535eaed83a9e461d1f0c0df1cb8992270e9bb8446f12cce6bc7b54`, `SC1-bb0c154b10f75bd484baef174256e815ecd3818b57776b31cfc3a136c5333c94`, `SC1-b031732a26d933e5ab3a57cf006728be7bed8320b4a252e7f802efa09ff3211d`, `SC1-0a62eaba114f745cccda38767cc64149fc47764e261f8a039e6226739d696cce`, `SC1-b5bb7410bea4b220de8c3e1580bea8754c99d463d143f4c9d8c1c586a3034beb`, `SC1-315b5fa8495d6e414681069796e440aabccbe56497b42cfd65d5f372e9165859`, `SC1-190388ace5f6ebe8412d5eae0cf1e54fcd7d84a29e9cd62abe4ea6d74ddb647e`
- `SC1-4be922003ba74922eecf93a71a425e99978c1d7891bc4276030f3a51065ec5d2`, `SC1-9649ae1b894344d34a690836093c746eb3b2c67423a941499154b9e41705e4ce`, `SC1-8961f1d0bcd230797d1ae9303cede531f18a290945652a502a73d1c0fd05b8d4`, `SC1-8030051c359cf3e79b7d9034e667390bbf69883717f725ef2dd45609032ecf25`, `SC1-1194e48c4dd892d71a77f9718cb73185eaaa651345bdc486034d734e61af3b45`, `SC1-d1eccea781ae6903949566a973fd355942ca5a448f7c6b9141c9a4a18d8f8665`, `SC1-682dff7ef6591f8003744f06ef0fe629879a3a5db5e1efc253e7920cf7b42f9d`, `SC1-788084c77920a910b2116c0e4b214162f5bf567648f87a89da6dbda20b0db962`

### P1 — String macros were changed from C string literals usable in fixed C arrays to Rust fat string references

Linux symbols: `DEV_ENERGYMODEL_FAMILY_NAME` and
`DEV_ENERGYMODEL_MCGRP_EVENT`.

Local evidence: the pinned header defines both as string-literal macros at
lines 10 and 80.  `kernel/power/em_netlink_autogen.c:51--53` initializes the
family name from `DEV_ENERGYMODEL_FAMILY_NAME`.  The local generic-netlink
contract is `char name[GENL_NAMSIZ]` for both `struct genl_family` at
`include/net/genetlink.h:78--81` and `struct genl_multicast_group` at lines
29--31.  A C string literal supplies the terminating NUL and is valid for
those fixed-array initializers.  Candidate lines 12 and 56 expose `&str`, a
two-word Rust reference that neither supplies a C NUL-terminated array nor can
be used as the fixed `[c_char; GENL_NAMSIZ]` initializer.  This changes the
UAPI macro/ABI contract despite preserving visible text.

Required correction: represent both macros with C-compatible static
NUL-terminated byte storage (and the applicable fixed-array initialization
semantics), while retaining their exact names and bytes.

Semantic record keys:

- `SC1-765c152908291fa365030afa9b47a9081f5795d0e5b1122f3c5ab34e2be019ed`, `SC1-00ed72f8ab5a649e198506c1d66d70cf720d7d751a18eb06f10e2498d3c3014d`
- `SC1-4370bf84a0062b1c58ad6cd55686213db529f6bea741fdb78b1662bbe9b71ae4`, `SC1-85643590bb9911bbdbf827f1ae025264c3163e9c01f2133325a18a7f2fec6fb8`

## Checked without a finding

The candidate preserves all three literal macro values, the four anonymous
enum sequences, their `__..._MAX` sentinels, their public `..._MAX` aliases,
and the five command values (`1` through `5`) as the pinned header declares.
The header has no configuration branch beyond its include guard, no storage,
allocation, locking, refcount, RCU, cleanup, or executable error path.  No
unauthorized Lupos branding was found; the frozen branding allowlist is empty
and the candidate contains no Lupos rename.

Result: FINDINGS (2).  The candidate must not be accepted unchanged.
