# Parity review — S016384, attempt 2, slot 1

Reviewer role: parity reviewer (slot 1)  
Actual reviewer model/effort: `gpt-5.6-terra` / `high`  
Pipeline: `P02`  
Decision: **REJECT**

## Reviewed inputs and binding

| Item | Expected by sealed proposal | Actual reviewed value |
| --- | --- | --- |
| Linux revision | `425f94c2954b1fe80ebdbf9b29854e89750355df` | `425f94c2954b1fe80ebdbf9b29854e89750355df` |
| Candidate snapshot (`candidate.diff`) SHA-256 | `b28480343761524695d038fe26af475c81671e60238a2003fac9b47df5cf91d1` | `b28480343761524695d038fe26af475c81671e60238a2003fac9b47df5cf91d1` |
| Implementation evidence SHA-256 | `1805f1270306be72f5047f68a43d99b0da9cf34618989a914f3971f388555de7` | `1805f1270306be72f5047f68a43d99b0da9cf34618989a914f3971f388555de7` |
| Sealed proposal SHA-256 | `74a241bbd4d1bc78dcf9e818412f22a692a4a6895b828996ed6480aed715542d` | `74a241bbd4d1bc78dcf9e818412f22a692a4a6895b828996ed6480aed715542d` |
| Phase-0 identity SHA-256 | `0123af9e96b58af4e98b91a2de5608d36b63c67864b9041d3340587f3dbe40d2` | proposal-bound value reviewed |
| Queue fingerprint | `cfa8dcf0d81c408de38d4202fe1ce9d9e5e301d1e67439c9530da94b1e64ee3f` | proposal-bound value reviewed |

The candidate binding is current: its hash is the hash of `candidate.diff`, not a hash substituted from the destination source file.

## Exhaustive source comparison

`vendor/linux/include/uapi/linux/snmp.h` contains eight anonymous enum declarations (lines 19, 69, 110, 129, 155, 171, 313, and 352), **296** enumerators, and two value macros.  The 296 enumerators include eight terminating `__*MAX` names.  The remaining 288 non-terminating enumerators, together with the two macros, account for the candidate's 290 `pub const NAME: i32 = value;` definitions.

Every emitted candidate constant has the source name, `i32` integer representation, and the explicit source or implicit-next value required by its containing C enum.  The two macros retain value 512.  The source C enumerator expressions are `int` values in their declared sequence; `i32` preserves the values here (all are non-negative and no greater than 512).  The C include guard is `_LINUX_SNMP_H` (`#ifndef`/`#define` at lines 8–9 and `#endif` at line 375); Rust module inclusion replaces that preprocessing mechanism and creates no missing Rust value name.

Accordingly, the proposed count “8 anonymous enum declarations, 288 enumerators, 2 macros; 298 semantic records including declarations” cannot be confirmed as a complete source inventory.  It omits the eight source enumerators below: source totals are 8 enum declarations, 296 enumerators, and 2 value macros (306 source declarations/items when each is counted once).  This is also not merely a count discrepancy: the sealed proposal itself contains all 296 enum constants, including the omitted eight.

## Finding P1 — eight selected UAPI enum names omitted

Severity: **blocking**

`src/include/uapi/linux/snmp.rs` lacks all eight terminating enumerator names.  They are selected `enum_constant` records in the sealed proposal for both frozen architectures, so omitting them changes the public translated UAPI name/value set.

| Missing source UAPI enumerator | Source line/value | aarch64 sealed status key | x86_64 sealed status key |
| --- | --- | --- | --- |
| `__IPSTATS_MIB_MAX` | 61 / 38 | `SC1-db8cf60c725b56a9c5b006189b3d6a734ad5d7e10a9b32e614e140fb2a598161` | `SC1-796850e9adc1fe0c51fe0efd6c00a7a8d55f13123cdb276ce7a266b86e3d6ac2` |
| `__ICMP_MIB_MAX` | 101 / 30 | `SC1-a0beac152dc7cbe43587623468fb03b0352a4a59a3700e6e51877593a2752e8b` | `SC1-636184e2bd14e5a4399f37f4c4a01d36fb3eebcffa01b9b68ac256499806e780` |
| `__ICMP6_MIB_MAX` | 119 / 7 | `SC1-a7c51507a84ad9af38b98f6330a9357825f570c5fbfcda050024297962aabb5a` | `SC1-7b9a73c0b13f189f1dba754b640d759bb2e655e8e0554a49a1a44114c07b618a` |
| `__TCP_MIB_MAX` | 147 / 16 | `SC1-48822390dbe3c3756aa5bae4c8be37383253c326ac4272bcb19375aeff580234` | `SC1-f0f01b4a7f15c74bc93f671c8a2846ea247ba3a2b8a39a0b6666fa820cd2f8d5` |
| `__UDP_MIB_MAX` | 167 / 10 | `SC1-922ab189e7c2509616a1344d6a91085b968468ee558aa0d9bd937440eca0cf76` | `SC1-c8ddc11a6cb93192e089a5b6660ee29b2c0989f4ba19b6311bff2ae8f9772c90` |
| `__LINUX_MIB_MAX` | 309 / 136 | `SC1-72335f2512edc8230caa9abb68be33313d2bca6d8569d888d8dd7a26bc37f80b` | `SC1-36462f6ec529f3d73992a5276ec5960093600d52181ab06f0b19a09c63ce66d6` |
| `__LINUX_MIB_XFRMMAX` | 348 / 33 | `SC1-633940c5ed8df4e5ffd7864d2e2f89b8c42cf8756cc89e1ea57aa7dc2d6a2154` | `SC1-8e3ed87ae55600eac42cb12e0ef2f4a47294bd9e7b83d338ff683540851b4233` |
| `__LINUX_MIB_TLSMAX` | 372 / 18 | `SC1-b6ca7460fd69edd418ce6d2c67ce0e6d80e9627927b875673a07b4d943a58e6f` | `SC1-9d7ba189acae74192e152ea01c819cfca10298701cd6a71719c002224ec218fe` |

Required resolution: add these eight `pub const` definitions with the stated `i32` values, then update any count assertion to distinguish 288 non-terminating enumerators from the source's complete 296-enumerator set.  Re-review is required after source changes.

No compiler, formatter, test, diagnostic, or historical evidence was used.
