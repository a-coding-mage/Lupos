# Parity review — S014598 / slot 1

Reviewed source only on the frozen `feat/bun-like-rewrite-test` branch.

- Linux source: `vendor/linux/include/linux/pci_ids.h`
- Rust candidate: `src/include/linux/pci_ids.rs`
- Linux revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`
- Frozen queue fingerprint: `d6c01f29edd048c73608fff2ed65f2485755023f85ffc0fec01e9f511fd6a72c`
- Queue state observed: `S014598`, `REVIEWING`, `P01`, attempt `2`

## Finding P1 — 57 selected PCI-ID definitions are omitted

`pci_ids.h` contains 2,902 object-like PCI-ID definitions after excluding its
include-guard macro. The candidate exposes 2,845 corresponding public constants,
so 57 definitions are absent. Every omitted definition has a trailing upstream
block comment; comments do not make the macro non-operative. Their absence
removes the named vendor/device IDs from the Rust translation.

Upstream source locations and omitted definitions:

| `pci_ids.h` line | Definition | Value |
| ---: | --- | --- |
| 529 | `PCI_VENDOR_ID_COMPEX2` | `0x101a` |
| 702 | `PCI_DEVICE_ID_NEC_CBUS_1` | `0x0001` |
| 703 | `PCI_DEVICE_ID_NEC_LOCAL` | `0x0002` |
| 704 | `PCI_DEVICE_ID_NEC_ATM` | `0x0003` |
| 705 | `PCI_DEVICE_ID_NEC_R4000` | `0x0004` |
| 706 | `PCI_DEVICE_ID_NEC_486` | `0x0005` |
| 707 | `PCI_DEVICE_ID_NEC_ACCEL_1` | `0x0006` |
| 708 | `PCI_DEVICE_ID_NEC_UXBUS` | `0x0007` |
| 709 | `PCI_DEVICE_ID_NEC_ACCEL_2` | `0x0008` |
| 710 | `PCI_DEVICE_ID_NEC_GRAPH` | `0x0009` |
| 711 | `PCI_DEVICE_ID_NEC_VL` | `0x0016` |
| 712 | `PCI_DEVICE_ID_NEC_STARALPHA2` | `0x002c` |
| 713 | `PCI_DEVICE_ID_NEC_CBUS_2` | `0x002d` |
| 714 | `PCI_DEVICE_ID_NEC_USB` | `0x0035` |
| 717 | `PCI_DEVICE_ID_NEC_PCX2` | `0x0046` |
| 721 | `PCI_DEVICE_ID_NEC_PC9821CS01` | `0x800c` |
| 722 | `PCI_DEVICE_ID_NEC_PC9821NRB06` | `0x800d` |
| 1413 | `PCI_VENDOR_ID_CREATIVE` | `0x1102` |
| 1424 | `PCI_VENDOR_ID_ECTIVA` | `0x1102` |
| 1434 | `PCI_DEVICE_ID_TTI_HPT372N` | `0x0009` |
| 1873 | `PCI_VENDOR_ID_CB` | `0x1307` |
| 2083 | `PCI_DEVICE_ID_LAVA_DSERIAL` | `0x0100` |
| 2084 | `PCI_DEVICE_ID_LAVA_QUATRO_A` | `0x0101` |
| 2085 | `PCI_DEVICE_ID_LAVA_QUATRO_B` | `0x0102` |
| 2086 | `PCI_DEVICE_ID_LAVA_QUATTRO_A` | `0x0120` |
| 2087 | `PCI_DEVICE_ID_LAVA_QUATTRO_B` | `0x0121` |
| 2088 | `PCI_DEVICE_ID_LAVA_OCTO_A` | `0x0180` |
| 2089 | `PCI_DEVICE_ID_LAVA_OCTO_B` | `0x0181` |
| 2090 | `PCI_DEVICE_ID_LAVA_PORT_PLUS` | `0x0200` |
| 2091 | `PCI_DEVICE_ID_LAVA_QUAD_A` | `0x0201` |
| 2092 | `PCI_DEVICE_ID_LAVA_QUAD_B` | `0x0202` |
| 2093 | `PCI_DEVICE_ID_LAVA_SSERIAL` | `0x0500` |
| 2094 | `PCI_DEVICE_ID_LAVA_PORT_650` | `0x0600` |
| 2096 | `PCI_DEVICE_ID_LAVA_DUAL_PAR_A` | `0x8002` |
| 2097 | `PCI_DEVICE_ID_LAVA_DUAL_PAR_B` | `0x8003` |
| 2478 | `PCI_VENDOR_ID_FREESCALE` | `0x1957` |
| 2479 | `PCI_VENDOR_ID_NXP` | `0x1957` |
| 2746 | `PCI_DEVICE_ID_INTEL_LIGHT_RIDGE` | `0x1513` |
| 2749 | `PCI_DEVICE_ID_INTEL_CACTUS_RIDGE_4C` | `0x1547` |
| 2752 | `PCI_DEVICE_ID_INTEL_REDWOOD_RIDGE_2C_NHI` | `0x1566` |
| 2756 | `PCI_DEVICE_ID_INTEL_FALCON_RIDGE_2C_NHI` | `0x156a` |
| 2760 | `PCI_DEVICE_ID_INTEL_ALPINE_RIDGE_2C_NHI` | `0x1575` |
| 3031 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_RAS` | `0x3c71` |
| 3032 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_ERR0` | `0x3c72` |
| 3033 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_ERR1` | `0x3c73` |
| 3034 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_ERR2` | `0x3c76` |
| 3035 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_ERR3` | `0x3c77` |
| 3036 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_HA0` | `0x3ca0` |
| 3037 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_TA` | `0x3ca8` |
| 3038 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_TAD0` | `0x3caa` |
| 3039 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_TAD1` | `0x3cab` |
| 3040 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_TAD2` | `0x3cac` |
| 3041 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_TAD3` | `0x3cad` |
| 3046 | `PCI_DEVICE_ID_INTEL_SBRIDGE_IMC_DDRIO` | `0x3cb8` |
| 3048 | `PCI_DEVICE_ID_INTEL_SBRIDGE_SAD0` | `0x3cf4` |
| 3049 | `PCI_DEVICE_ID_INTEL_SBRIDGE_BR` | `0x3cf5` |
| 3050 | `PCI_DEVICE_ID_INTEL_SBRIDGE_SAD1` | `0x3cf6` |

## Coverage and non-findings

- The remaining 2,845 macro names are present exactly once, with no extra Rust
  constants and no value mismatch after removing non-semantic trailing C
  comments. The header has no function-like definitions or non-literal value
  expressions.
- All 2,902 operative values are unsuffixed hexadecimal literals no greater
  than `0xFFFF`; under the frozen x86_64/aarch64 C ABI each has C type `int`.
  The present Rust constants use `i32`, so their integer width, signedness, and
  literal value semantics agree. The 57 omitted values require the same `i32`
  representation.
- The include guard `_LINUX_PCI_IDS_H` is intentionally not a Rust constant and
  is excluded from the 2,902-definition comparison.
- No macro has a function-like form, an arithmetic/cast/bitwise expression,
  a suffix-driven type variation, or a value outside `i32` range.
- The candidate carries the required immutable provenance fields, with the
  exact Linux source path, frozen Linux revision, `common` architecture, and
  task ID. It retains the upstream file notice, and its required SPDX
  provenance line is `GPL-2.0-only`.

Result: reject pending restoration of all 57 omitted operative definitions.
