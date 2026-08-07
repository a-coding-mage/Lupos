# Rust source review — S016277, attempt 3, slot 2

Reviewed only the pinned source `vendor/linux/include/uapi/linux/netfilter/nf_tables.h` at `425f94c2954b1fe80ebdbf9b29854e89750355df`, the current leased destination `src/include/uapi/linux/netfilter/nf_tables.rs`, and the sealed task-local semantic proposal/seal.  No compiler, formatter, test, Rust-analyzer diagnostic, or historical translation source was used.

Result: FINDINGS.

## R001 — C line-continuation tokens were emitted as Rust tokens

`nf_tables.h` uses preprocessing line continuations in the macro definitions at lines 194–196, 225–227, 840–841, and 1979–1981.  The candidate retains literal backslashes in ordinary Rust expressions at lines 213, 248, 951, and 2281 respectively.  A backslash is not an expression-continuation token in Rust, so these declarations are not valid Rust syntax.  This prevents the C masks from being represented at all.

Closure-key mapping:

- `SC1-4bed182a5de53614167ca775e556f53d24cddc533735b0584c5392e81ee65188`, `SC1-bc6599774eb83ca323ed1aaa56a4adfaa5167bb04c285401840654ed127d7522` — `NFT_TABLE_F_MASK`, upstream line 194.
- `SC1-14b713f7831f956b05d12f8c4039530043eaa579b78d3025d061e0d5a89b25b5`, `SC1-93f24579bfedca2e06f77593b78c28a947c8975a6f7fa4c7396f4434325d79ae` — `NFT_CHAIN_FLAGS`, upstream line 225.
- `SC1-f95ddd337c14f58161a09eef7db4bc2cf44944bd2da9de44c837df4f847cfb23`, `SC1-133d1cdb8481fd15ee7e1cc7c0d26d365c34df55a9e99e5b8289ea1d933190b6` — `NFT_INNER_MASK`, upstream line 840.
- `SC1-5a94a2ac85e931b834460bbb3438254af6bd16fbb651c6fb794dc619d5d43ecf`, `SC1-55129bc90b8a1ed44b810a2952f07aced54bb10ee207d0b93daaf99ca015d779` — `NFT_TUNNEL_F_MASK`, upstream line 1979.

## R002 — nominal Rust enums do not preserve the header's integer-constant semantics

The candidate turns each C enumeration into a distinct `#[repr(i32)]` Rust enum, then re-exports its variants.  In the UAPI, enumerators are integer constants and the accompanying macros perform integer arithmetic and bitwise operations.  The candidate's own examples show the incompatible substitution: it assigns `__NFT_REG_MAX - 1`, `NFT_REG32_15 - NFT_REG32_00 + 1`, and `NFT_BITWISE_MASK_XOR` to `i32` constants at lines 54, 57, 61, and 664, although those operands are values of distinct Rust enum types.  The same defect occurs for the many `*_MAX` constants and masks throughout the file.  It narrows values to closed Rust enum domains rather than retaining the UAPI's raw integer values accepted in `NLA_U32` fields.

The candidate also puts `NFT_FLOWTABLE_MASK` inside `nft_flowtable_flags` at lines 1952–1957 using `NFT_FLOWTABLE_HW_OFFLOAD | NFT_FLOWTABLE_COUNTER`; that is a bitwise expression over enum variants, not an integer discriminant.  Upstream deliberately defines that expression as a C enum integer constant at lines 1707–1712.  `#[repr(i32)]` fixes a Rust enum discriminant width, but it does not make enum variants interchangeable with `i32`, supply the required integer operators, or reproduce the C UAPI macro interface.  The implementation must represent these public numeric names and macro results as appropriate explicit integer constants while preserving all upstream progression and values.

Closure-key mapping:

- `SC1-336dba9ecd4d92fa450270d6a98e6fc0a65d4ef5398df5c1dba9a690b249192d`, `SC1-ae9d315dce37df51e81c659278051d661707f6616443a6d126b80cb589318992` — `NFT_REG_MAX`, upstream line 47 and candidate line 54.
- `SC1-2ca21f97c3be0ef45aa7130f54e5fbda5c40af7835e231de2b1cbcd8829e570e`, `SC1-c23028fee5b1efa81e3944dafd615bc47965d79890477ccfcf5415843688d017` — `NFT_FLOWTABLE_MASK`, upstream line 1710 and candidate lines 1952–1957.

## R003 — `__KERNEL__` condition lacks frozen source evidence for its Rust feature mapping

Upstream exposes `NFT_REG32_MAX` only while the C preprocessor macro `__KERNEL__` is defined (lines 49–51).  The candidate substitutes `#[cfg(feature = "__KERNEL__")]` at lines 56–57.  The frozen conditional records identify the original `ifdef@49`, but the sealed proposal supplies no source evidence that the Rust feature namespace is configured to exactly the same condition for both approved architectures.  Consequently the candidate can omit or expose this UAPI name independently of the pinned C preprocessing condition.  This is a source-review uncertainty that must be resolved with frozen configuration/build-metadata evidence before closure may claim COMPLETE.

Closure-key mapping:

- `SC1-d39378de21edce766b4d3fa9f0b36598225068f6c254404b7d69f2aacfbfd7c8` — aarch64 `ifdef@49`.
- `SC1-6d77279184de85dfcc9d0a65d678cc9f2b0d423313ab7263d7bcb65576ec98ab` — x86_64 `ifdef@49`.

No functions, allocations, unsafe blocks, raw pointers, FFI calls, layout-bearing structs/unions, callbacks, or test configuration occur in this header translation; the findings above are therefore the material ownership/ABI and Rust-semantics blockers.
