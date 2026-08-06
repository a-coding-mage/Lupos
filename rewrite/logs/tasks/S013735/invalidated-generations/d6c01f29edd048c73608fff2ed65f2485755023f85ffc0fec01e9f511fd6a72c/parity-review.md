# Parity review — S013735

Role: parity reviewer (slot 1)  
Scope: `include/linux/device-id/spi.h` → `src/include/linux/device-id/spi.rs`  
Pinned revision: `425f94c2954b1fe80ebdbf9b29854e89750355df`  
Architectures: x86_64, aarch64

## Verdict

PASS — no source-parity finding.

## Source comparison

The candidate has the required immutable provenance and maps every selected
declaration from the 19-line upstream header:

| Upstream item | Candidate mapping | Review result |
| --- | --- | --- |
| `#ifdef __KERNEL__ typedef unsigned long kernel_ulong_t` | `pub type kernel_ulong_t = u64` | Both frozen targets are 64-bit LP64 kernel builds; this preserves the selected typedef width and unsignedness. The Phase 0 compile commands also define `__KERNEL__`. |
| `#define SPI_NAME_SIZE 32` | exported macro expanding to `32i32` | Preserves the C unsuffixed integer literal value and type; the struct array bound makes the required Rust-only `usize` conversion. |
| `#define SPI_MODULE_PREFIX "spi:"` | exported macro expanding to `b"spi:\0"` | Preserves the C string-literal bytes, including its terminating NUL, without introducing a header object. |
| `struct spi_device_id { char name[32]; kernel_ulong_t driver_data; }` | `#[repr(C)] struct spi_device_id { name: [u8; 32], driver_data: u64 }` | Exact field order and widths. The x86_64 and aarch64 frozen compile-command records both contain `-funsigned-char`, so `u8` is the selected C `char` representation. The resulting natural alignment/layout is 32 bytes followed by an 8-byte unsigned long. |

`Copy, Clone` add no C-visible field, symbol, initialization, or runtime
behavior and do not alter the `#[repr(C)]` layout.

## Conditional, ABI, and direct-user checks

- The two selected architecture command records use `--target=x86_64-linux-gnu`
  and `--target=aarch64-linux-gnu`, define `__KERNEL__`, and include
  `-funsigned-char`; no unselected user-space variant must be represented by
  this kernel translation task.
- The direct SPI-core users retain the required fixed 32-byte character-array
  contract: `include/linux/spi/spi.h` uses `SPI_NAME_SIZE` for both `modalias`
  arrays and exposes `const struct spi_device_id *id_table`.
- `drivers/spi/spi.c` tests `id->name[0]`, passes `id->name` to `strcmp`, and
  emits the `SPI_MODULE_PREFIX` modalias. The candidate preserves the bytes and
  structure layout needed by those operations.
- `scripts/mod/devicetable-offsets.c` requests the size and `name` offset of
  `spi_device_id`; `scripts/mod/file2alias.c` reads `name` and concatenates the
  prefix into the `spi:` module alias. The candidate keeps `name` at offset 0,
  preserves its 32-byte extent, and preserves the prefix string bytes.
- No branding delta, static storage, function, error path, synchronization
  behavior, or allocation behavior exists in the upstream header to carry over.

Review used only pinned source, frozen metadata/configuration records, and
manual source inspection. No compiler, formatter, linker, test, or diagnostic
tool was run.
