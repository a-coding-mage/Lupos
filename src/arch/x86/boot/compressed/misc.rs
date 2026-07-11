//! linux-parity: partial
//! linux-source: vendor/linux/arch/x86/boot/compressed/misc.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/misc.c
//! Decompressor entry point (`extract_kernel`) plus ELF parsing,
//! relocation processing, and the screen/serial output helpers.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/misc.c
//! - vendor/linux/arch/x86/boot/compressed/misc.h
//!
//! This is a source-shaped model of misc.c. The live bzImage still uses the
//! assembly extractor documented below, so this cannot claim complete runtime
//! parity. The decompressor model runs
//! with no host hardware available, so the raw operations that cannot
//! execute inside a host unit test are routed through explicit seams
//! (the [`MiscEnv`] trait). Linux itself indirects most of these via
//! function pointers / inline asm (`pio_ops`, `__decompress`, the
//! `void (*error)(char*)` callback passed into `decompress_kernel`,
//! `read_cr2`, etc.), so the seam is faithful to the source structure,
//! not a simplification. Each seam method cites the misc.c line it
//! stands in for.

use super::error::ErrorSink;
use super::kaslr::ChosenLocation;
use crate::arch::x86::include::uapi::asm::bootparam::BootParams;

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;

/// `LOAD_PHYSICAL_ADDR` — `ALIGN(CONFIG_PHYSICAL_START, CONFIG_PHYSICAL_ALIGN)`.
/// With the x86 defaults `CONFIG_PHYSICAL_START = 0x1000000` and
/// `CONFIG_PHYSICAL_ALIGN = 0x200000`, the aligned result is 0x1000000.
/// Ref: vendor/linux/arch/x86/include/asm/page_types.h:32 and
/// vendor/linux/arch/x86/Kconfig:2034 / :2133.
pub const LOAD_PHYSICAL_ADDR: u64 = 0x0100_0000;

/// `MIN_KERNEL_ALIGN = 1 << PMD_SHIFT` on x86_64 (= 2 MiB).
/// Ref: vendor/linux/arch/x86/include/asm/boot.h:11,15 and
/// vendor/linux/arch/x86/include/asm/pgtable_64_types.h:74.
pub const MIN_KERNEL_ALIGN: u64 = 1 << 21;

/// `KERNEL_IMAGE_SIZE` — 1 GiB with CONFIG_RANDOMIZE_BASE.
/// Ref: vendor/linux/arch/x86/include/asm/page_64_types.h:85.
pub const KERNEL_IMAGE_SIZE: u64 = 1024 * 1024 * 1024;

/// `__START_KERNEL_map`.
/// Ref: vendor/linux/arch/x86/include/asm/page_64_types.h:46.
pub const START_KERNEL_MAP: u64 = 0xffff_ffff_8000_0000;

/// `BOOT_HEAP_SIZE` with CONFIG_KERNEL_GZIP.
/// Ref: vendor/linux/arch/x86/include/asm/boot.h:23-34.
pub const BOOT_HEAP_SIZE: usize = 0x0001_0000;

/// `ULONG_MAX` returned by `decompress_kernel` on failure (misc.c:356).
pub const ULONG_MAX: u64 = u64::MAX;

/// `KASLR_FLAG` cleared from `hdr.loadflags` in extract_kernel (misc.c:418).
/// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h:14.
pub const KASLR_FLAG: u8 = 1 << 1;

/// `XLF_MEM_ENCRYPTION` set by parse_mem_encrypt (misc.c:374).
/// Ref: vendor/linux/arch/x86/include/uapi/asm/bootparam.h:27.
pub const XLF_MEM_ENCRYPTION: u16 = 1 << 7;

/// `MSR_AMD64_SEV_ES_ENABLED` bit tested in early_sev_detect (misc.c:386).
/// Ref: vendor/linux/arch/x86/include/asm/msr-index.h.
pub const MSR_AMD64_SEV_ES_ENABLED: u64 = 1 << 1;

/// x86_64 compressed-stage heap ceiling checked by extract_kernel().
/// Ref: misc.c:503.
pub const BOOT_COMPRESSED_MAX_HEAP_ADDR: u64 = 0x3fff_ffff_ffff;

/// Fixed gzip header length skipped by Linux `__gunzip`.
/// Ref: vendor/linux/lib/decompress_inflate.c:100-113.
pub const GZIP_HEADER_SIZE: usize = 10;

/// gzip `FLG.FNAME`: Linux's preboot gunzip is unusual here; it only skips
/// this optional field before invoking raw deflate. Ref: decompress_inflate.c:114-126.
pub const GZIP_FLAG_FNAME: u8 = 0x08;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GzipHeaderError {
    NotGzip,
    HeaderError,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct GzipHeader {
    pub deflate_offset: usize,
    pub avail_in: usize,
}

/// Linux preboot gzip header handling from `__gunzip()`.
///
/// The decompressor verifies only the gzip magic and deflate method, skips the
/// fixed ten-byte header, and if `FLG.FNAME` is set skips one NUL-terminated
/// original filename. Other flag bits are intentionally not decoded here; in
/// Linux they remain in `next_in` and the raw deflate engine reports the
/// stream error. This helper mirrors that contract for host tests and for the
/// live assembly shim.
pub fn gzip_preboot_header(input: &[u8]) -> Result<GzipHeader, GzipHeaderError> {
    if input.len() < GZIP_HEADER_SIZE || input[0] != 0x1f || input[1] != 0x8b || input[2] != 0x08 {
        return Err(GzipHeaderError::NotGzip);
    }

    let mut next_in = GZIP_HEADER_SIZE;
    let mut avail_in = input.len() - GZIP_HEADER_SIZE;
    if (input[3] & GZIP_FLAG_FNAME) != 0 {
        loop {
            if avail_in == 0 {
                return Err(GzipHeaderError::HeaderError);
            }
            avail_in -= 1;
            let byte = input[next_in];
            next_in += 1;
            if byte == 0 {
                break;
            }
        }
    }

    Ok(GzipHeader {
        deflate_offset: next_in,
        avail_in,
    })
}

/// `__gunzip` reports `pos = strm->next_in - zbuf + 8`, adding the gzip
/// trailer length after raw deflate has stopped. Ref: decompress_inflate.c:165-167.
pub const fn gzip_preboot_pos(deflate_next_in_offset: usize) -> usize {
    deflate_next_in_offset + 8
}

// ---------------------------------------------------------------------------
// ELF constants (include/uapi/linux/elf.h) used by parse_elf().
// ---------------------------------------------------------------------------

/// `e_ident[]` magic indices. Ref: include/uapi/linux/elf.h:352-355.
pub const EI_MAG0: usize = 0;
pub const EI_MAG1: usize = 1;
pub const EI_MAG2: usize = 2;
pub const EI_MAG3: usize = 3;
/// `EI_NIDENT` — size of the `e_ident[]` array. Ref: elf.h:215.
pub const EI_NIDENT: usize = 16;

/// ELF magic bytes. Ref: include/uapi/linux/elf.h:362-365.
pub const ELFMAG0: u8 = 0x7f;
pub const ELFMAG1: u8 = b'E';
pub const ELFMAG2: u8 = b'L';
pub const ELFMAG3: u8 = b'F';

/// Program-header types. Ref: include/uapi/linux/elf.h:29-31.
pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;

/// `Elf64_Ehdr` — exact field-for-field mirror of `struct elf64_hdr`.
/// Ref: include/uapi/linux/elf.h:234-249.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

/// `Elf64_Phdr` — exact field-for-field mirror of `struct elf64_phdr`.
/// Ref: include/uapi/linux/elf.h:268-277.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// Environment seam for the operations misc.c performs that cannot run
/// inside a host unit test (raw MMU/firmware/console/decompress). Linux
/// reaches these through `pio_ops`, the `__decompress` macro, the
/// `void (*error)(char*)` callback, `read_cr2`, etc.; this trait keeps
/// the same indirection so the orchestration logic is testable while the
/// production wiring drives real hardware.
pub trait MiscEnv {
    /// `serial_putchar` / video write — `__putstr` console sink (misc.c:108,146).
    fn putstr(&mut self, s: &str);

    /// `__decompress(input, input_len, ..., outbuf, output_len, ..., error)`.
    /// Returns `Ok(())` on success, `Err(())` when `__decompress(...) < 0`,
    /// in which case `decompress_kernel` returns `ULONG_MAX` (misc.c:354-356).
    fn decompress(&mut self, input: &[u8], output: &mut [u8]) -> Result<(), ()>;

    /// `memcpy(dest, output + off, len)` reads from the decompressed image
    /// buffer. Used by parse_elf to pull the ehdr/phdrs (misc.c:293,306).
    /// Returns the requested bytes, or `Err(())` if out of range.
    fn read_output(&self, off: u64, len: usize) -> Result<Vec<u8>, ()>;

    /// `memmove(dest, output + p_offset, p_filesz)` — copy one PT_LOAD
    /// segment to its destination (misc.c:323). The seam records the move
    /// (src offset, dest address, length) so tests can assert placement.
    fn move_segment(&mut self, src_off: u64, dest: u64, len: u64);

    /// `*(u32 *)ptr += delta` for a 32-bit relocation (misc.c:260).
    /// Returns the patched word for test observation.
    fn apply_reloc32(&mut self, ptr: u64, delta: u64) -> u32;

    /// `*(u64 *)ptr += delta` for a 64-bit relocation (misc.c:271).
    fn apply_reloc64(&mut self, ptr: u64, delta: u64) -> u64;

    /// Read one signed 32-bit relocation table entry, sign-extended to 64
    /// bits, located `output + output_len - 4*(index+1)` working backwards
    /// from the end of the image (misc.c:252-253). Index 0 is the last
    /// 32-bit word.
    fn read_reloc(&self, index: usize) -> i32;
}

/// Inputs that Linux's `extract_kernel(rmode, output)` obtains from linker
/// symbols and register arguments in `boot/compressed`.
#[derive(Copy, Clone, Debug)]
pub struct ExtractKernelConfig<'a> {
    pub rmode_addr: u64,
    pub input_data_addr: u64,
    pub input: &'a [u8],
    pub input_len: u64,
    pub output: u64,
    pub output_len: u64,
    pub kernel_total_size: u64,
    pub bss_minus_text: u64,
    pub heap: u64,
    pub trampoline_32bit: u64,
    pub relocatable: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ExtractKernelResult {
    pub output: u64,
    pub virt_addr: u64,
    pub needed_size: u64,
    pub entry_offset: u64,
    pub entry_addr: u64,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ExtractKernelError {
    DestinationPhysicalAlignment,
    DestinationVirtualAlignment,
    DestinationAddressTooLarge,
    DestinationVirtualAddressBeyondKernelMapping,
    DestinationVirtualAddressChangedWhenNotRelocatable,
    DecompressFailed,
}

/// Environment seam for Linux `extract_kernel()` orchestration. The Rust
/// decompressor pieces are host-testable through [`MiscEnv`]; this trait
/// covers the surrounding compressed-stage calls Linux wires to firmware,
/// early console, TDX/SEV, ACPI, and exception-cleanup code.
pub trait ExtractKernelEnv: MiscEnv {
    /// `boot_params_ptr = rmode` (misc.c:415).
    fn retain_boot_params(&mut self, rmode_addr: u64);
    /// `parse_mem_encrypt(&boot_params_ptr->hdr)` (misc.c:420).
    fn parse_mem_encrypt(&mut self, boot_params: &mut BootParams);
    /// `sanitize_boot_params(boot_params_ptr)` (misc.c:422).
    fn sanitize_boot_params(&mut self, boot_params: &mut BootParams);
    /// `vidmem` / `vidport` plus `lines` / `cols` setup (misc.c:424-433).
    fn set_video_console(&mut self, vidmem: u64, vidport: u16, lines: u8, cols: u8);
    /// `init_default_io_ops()` (misc.c:435).
    fn init_default_io_ops(&mut self);
    /// `early_tdx_detect()` before console initialization (misc.c:443).
    fn early_tdx_detect(&mut self);
    /// `early_sev_detect()` (misc.c:445).
    fn early_sev_detect(&mut self);
    /// `console_init()` (misc.c:447).
    fn console_init(&mut self);
    /// `get_rsdp_addr()` (misc.c:454).
    fn get_rsdp_addr(&mut self) -> u64;
    fn debug_putstr(&mut self, s: &str);
    fn debug_putaddr(&mut self, value: u64);
    fn debug_puthex(&mut self, value: u64);
    /// `free_mem_ptr = heap; free_mem_end_ptr = heap + BOOT_HEAP_SIZE`.
    fn set_free_mem_bounds(&mut self, start: u64, end: u64);
    /// `choose_random_location(...)` (misc.c:490-493).
    fn choose_random_location(
        &mut self,
        input: u64,
        input_size: u64,
        output: u64,
        output_size: u64,
        virt_addr: u64,
    ) -> ChosenLocation;
    /// `init_unaccepted_memory()` (misc.c:514).
    fn init_unaccepted_memory(&mut self) -> bool;
    /// `__pa(output)` for the accept-memory call.
    fn physical_address(&self, addr: u64) -> u64 {
        addr
    }
    /// `accept_memory(__pa(output), needed_size)` (misc.c:516).
    fn accept_memory(&mut self, phys: u64, size: u64);
    /// `cleanup_exception_handling()` (misc.c:528).
    fn cleanup_exception_handling(&mut self);
    /// `spurious_nmi_count` (misc.c:530).
    fn spurious_nmi_count(&self) -> u64;
    fn error_putstr(&mut self, s: &str);
    fn error_putdec(&mut self, value: u64);
}

/// `error(msg)` — misc.c routes the fatal path through the callback
/// passed into `decompress_kernel` (and `error.c::error`). Here we surface
/// the message through the [`ErrorSink`] and return the never-type via the
/// caller's halt. Kept as a thin wrapper so parse_elf/handle_relocations
/// read like the C. Ref: misc.c uses `error()` from error.h.
fn error_msg<S: ErrorSink>(sink: &mut S, msg: &str) {
    super::error::warn(sink, msg);
    sink.putstr(" -- System halted");
}

/// `ALIGN(needed_size, MIN_KERNEL_ALIGN)` in extract_kernel().
#[inline]
fn align_up_extract(x: u64, a: u64) -> u64 {
    (x + a - 1) & !(a - 1)
}

fn extract_kernel_error<S: ErrorSink>(
    sink: &mut S,
    msg: &str,
    err: ExtractKernelError,
) -> Result<ExtractKernelResult, ExtractKernelError> {
    error_msg(sink, msg);
    Err(err)
}

/// `extract_kernel(rmode, output)` orchestration from misc.c:407-535.
///
/// The live bzImage still links a temporary assembly `extract_kernel` shim,
/// but this Rust mirror is the source-shaped target for replacing that shim:
/// it retains the boot_params pointer, clears in-kernel-only load flags,
/// performs early console/TDX/SEV/ACPI setup, runs KASLR placement, validates
/// chosen addresses, calls `decompress_kernel`, cleans up compressed-stage
/// exception handling, and returns `output + entry_offset`.
pub fn extract_kernel<E: ExtractKernelEnv, S: ErrorSink>(
    env: &mut E,
    sink: &mut S,
    boot_params: &mut BootParams,
    config: ExtractKernelConfig<'_>,
    output_buf: &mut [u8],
) -> Result<ExtractKernelResult, ExtractKernelError> {
    let mut virt_addr = LOAD_PHYSICAL_ADDR;
    let heap = config.heap;
    let mut output = config.output;

    env.retain_boot_params(config.rmode_addr);
    boot_params.clear_loadflags(KASLR_FLAG);
    env.parse_mem_encrypt(boot_params);
    env.sanitize_boot_params(boot_params);

    let (vidmem, vidport) = if boot_params.screen_orig_video_mode() == 7 {
        (0xb0000, 0x3b4)
    } else {
        (0xb8000, 0x3d4)
    };
    env.set_video_console(
        vidmem,
        vidport,
        boot_params.screen_orig_video_lines(),
        boot_params.screen_orig_video_cols(),
    );

    env.init_default_io_ops();
    env.early_tdx_detect();
    env.early_sev_detect();
    env.console_init();
    let rsdp = env.get_rsdp_addr();
    boot_params.set_acpi_rsdp_addr(rsdp);
    env.debug_putstr("early console in extract_kernel\n");

    env.set_free_mem_bounds(heap, heap + BOOT_HEAP_SIZE as u64);

    let mut needed_size = core::cmp::max(config.output_len, config.kernel_total_size);
    needed_size = align_up_extract(needed_size, MIN_KERNEL_ALIGN);

    env.debug_putaddr(config.input_data_addr);
    env.debug_putaddr(config.input_len);
    env.debug_putaddr(output);
    env.debug_putaddr(config.output_len);
    env.debug_putaddr(config.kernel_total_size);
    env.debug_putaddr(needed_size);
    env.debug_putaddr(config.trampoline_32bit);

    let chosen = env.choose_random_location(
        config.input_data_addr,
        config.input_len,
        output,
        needed_size,
        virt_addr,
    );
    output = chosen.output;
    virt_addr = chosen.virt_addr;
    if chosen.randomized {
        boot_params.set_loadflags(boot_params.loadflags() | KASLR_FLAG);
    }

    if (output & (MIN_KERNEL_ALIGN - 1)) != 0 {
        return extract_kernel_error(
            sink,
            "Destination physical address inappropriately aligned",
            ExtractKernelError::DestinationPhysicalAlignment,
        );
    }
    if (virt_addr & (MIN_KERNEL_ALIGN - 1)) != 0 {
        return extract_kernel_error(
            sink,
            "Destination virtual address inappropriately aligned",
            ExtractKernelError::DestinationVirtualAlignment,
        );
    }
    if heap > BOOT_COMPRESSED_MAX_HEAP_ADDR {
        return extract_kernel_error(
            sink,
            "Destination address too large",
            ExtractKernelError::DestinationAddressTooLarge,
        );
    }
    if virt_addr
        .checked_add(needed_size)
        .is_none_or(|end| end > KERNEL_IMAGE_SIZE)
    {
        return extract_kernel_error(
            sink,
            "Destination virtual address is beyond the kernel mapping area",
            ExtractKernelError::DestinationVirtualAddressBeyondKernelMapping,
        );
    }
    if !config.relocatable && virt_addr != LOAD_PHYSICAL_ADDR {
        return extract_kernel_error(
            sink,
            "Destination virtual address changed when not relocatable",
            ExtractKernelError::DestinationVirtualAddressChangedWhenNotRelocatable,
        );
    }

    env.debug_putstr("\nDecompressing Linux... ");
    if env.init_unaccepted_memory() {
        env.debug_putstr("Accepting memory... ");
        env.accept_memory(env.physical_address(output), needed_size);
    }

    let entry_offset = decompress_kernel(
        env,
        sink,
        config.input,
        output,
        output_buf,
        config.output_len,
        virt_addr,
        config.bss_minus_text,
    );
    if entry_offset == ULONG_MAX {
        return Err(ExtractKernelError::DecompressFailed);
    }

    env.debug_putstr("done.\nBooting the kernel (entry_offset: 0x");
    env.debug_puthex(entry_offset);
    env.debug_putstr(").\n");
    env.cleanup_exception_handling();

    let nmi_count = env.spurious_nmi_count();
    if nmi_count != 0 {
        env.error_putstr("Spurious early NMIs ignored: ");
        env.error_putdec(nmi_count);
        env.error_putstr("\n");
    }

    Ok(ExtractKernelResult {
        output,
        virt_addr,
        needed_size,
        entry_offset,
        entry_addr: output + entry_offset,
    })
}

/// `__putnum(value, base, mindig)` — render an unsigned value in the given
/// base with at least `mindig` digits, then push it through `__putstr`.
/// Faithful mirror of misc.c:167-185.
fn putnum<E: MiscEnv>(env: &mut E, mut value: u64, base: u64, mut mindig: i32) {
    // buf[8*sizeof(value)+1] in C; for u64 that is 65 bytes.
    let mut buf = [0u8; 8 * core::mem::size_of::<u64>() + 1];
    let mut p = buf.len();
    p -= 1;
    buf[p] = b'\0';

    // do { ... } while(mindig-- > 0 || value)
    loop {
        let do_iter = {
            let cond = mindig > 0 || value != 0;
            mindig -= 1;
            cond
        };
        if !do_iter {
            break;
        }
        let mut digit = (value % base) as u8;
        digit += if digit >= 10 { b'a' - 10 } else { b'0' };
        p -= 1;
        buf[p] = digit;
        value /= base;
    }

    // __putstr(p): bytes from p up to (not including) the NUL.
    let end = buf.len() - 1;
    if let Ok(s) = core::str::from_utf8(&buf[p..end]) {
        env.putstr(s);
    }
}

/// `__puthex(value)` — hex with `sizeof(value)*2` minimum digits (misc.c:187-190).
pub fn puthex<E: MiscEnv>(env: &mut E, value: u64) {
    putnum(env, value, 16, (core::mem::size_of::<u64>() * 2) as i32);
}

/// `__putdec(value)` — decimal, minimum 1 digit (misc.c:192-195).
pub fn putdec<E: MiscEnv>(env: &mut E, value: u64) {
    putnum(env, value, 10, 1);
}

/// `handle_relocations(output, output_len, virt_addr)` for CONFIG_X86_64.
/// Faithful mirror of misc.c:198-274. `output` is the load address of the
/// decompressed image, `min_addr`/`max_addr` bound the kernel text/bss.
///
/// Returns the number of (32-bit, 64-bit) relocations applied so tests can
/// observe the walk; the C function returns void.
///
/// On an out-of-range pointer Linux calls `error(...)` (never returns); we
/// surface that through `sink` and stop, matching the halt semantics.
pub fn handle_relocations<E: MiscEnv, S: ErrorSink>(
    env: &mut E,
    sink: &mut S,
    output: u64,
    output_len: u64,
    virt_addr: u64,
    bss_minus_text: u64,
) -> (usize, usize) {
    let min_addr = output;
    let max_addr = min_addr + bss_minus_text;

    // delta = min_addr - LOAD_PHYSICAL_ADDR
    let mut delta = min_addr.wrapping_sub(LOAD_PHYSICAL_ADDR);
    // map = delta - __START_KERNEL_map
    let map = delta.wrapping_sub(START_KERNEL_MAP);

    // CONFIG_X86_64: delta = virt_addr - LOAD_PHYSICAL_ADDR
    delta = virt_addr.wrapping_sub(LOAD_PHYSICAL_ADDR);

    if delta == 0 {
        env.putstr("No relocation needed... ");
        return (0, 0);
    }
    env.putstr("Performing relocations... ");

    // 32-bit relocations, working backwards from the end of the image.
    // for (reloc = output + output_len - 4; *reloc; reloc--)
    let mut idx: usize = 0;
    let mut count32 = 0usize;
    loop {
        let raw = env.read_reloc(idx); // *reloc, 32-bit
        if raw == 0 {
            break; // zero terminator for 32-bit relocations
        }
        // long extended = *reloc; extended += map;
        let extended = (raw as i64 as u64).wrapping_add(map);
        let ptr = extended;
        if ptr < min_addr || ptr > max_addr {
            error_msg(sink, "32-bit relocation outside of kernel!\n");
            return (count32, 0);
        }
        env.apply_reloc32(ptr, delta);
        count32 += 1;
        idx += 1;
    }

    // 64-bit relocations: for (reloc--; *reloc; reloc--)
    // The C does `reloc--` once to step past the 32-bit zero terminator;
    // our index already points at that terminator, so advance past it.
    idx += 1;
    let mut count64 = 0usize;
    loop {
        let raw = env.read_reloc(idx);
        if raw == 0 {
            break; // zero terminator for 64-bit relocations
        }
        let extended = (raw as i64 as u64).wrapping_add(map);
        let ptr = extended;
        if ptr < min_addr || ptr > max_addr {
            error_msg(sink, "64-bit relocation outside of kernel!\n");
            return (count32, count64);
        }
        env.apply_reloc64(ptr, delta);
        count64 += 1;
        idx += 1;
    }

    (count32, count64)
}

/// `parse_elf(output)` for CONFIG_X86_64 + CONFIG_RELOCATABLE.
/// Faithful mirror of misc.c:281-332.
///
/// Validates the ELF magic, walks every program header, and for each
/// PT_LOAD: checks 2 MiB `p_align`, computes `dest = output + (p_paddr -
/// LOAD_PHYSICAL_ADDR)`, and moves `p_filesz` bytes there. Returns
/// `e_entry - LOAD_PHYSICAL_ADDR` (the entry offset), or `Err(())` if the
/// image is not a valid ELF / a LOAD alignment check fails (Linux calls
/// `error()` which halts; we surface it through `sink`).
pub fn parse_elf<E: MiscEnv, S: ErrorSink>(
    env: &mut E,
    sink: &mut S,
    output: u64,
) -> Result<u64, ()> {
    // memcpy(&ehdr, output, sizeof(ehdr));
    let ehdr_bytes = env
        .read_output(0, core::mem::size_of::<Elf64Ehdr>())
        .map_err(|_| ())?;
    let ehdr = ehdr_from_bytes(&ehdr_bytes);

    if ehdr.e_ident[EI_MAG0] != ELFMAG0
        || ehdr.e_ident[EI_MAG1] != ELFMAG1
        || ehdr.e_ident[EI_MAG2] != ELFMAG2
        || ehdr.e_ident[EI_MAG3] != ELFMAG3
    {
        error_msg(sink, "Kernel is not a valid ELF file");
        return Err(());
    }

    env.putstr("Parsing ELF... ");

    // phdrs = malloc(sizeof(*phdrs) * e_phnum);
    // memcpy(phdrs, output + e_phoff, sizeof(*phdrs) * e_phnum);
    let phsz = core::mem::size_of::<Elf64Phdr>();
    let nph = ehdr.e_phnum as usize;
    let phdr_bytes = env.read_output(ehdr.e_phoff, phsz * nph).map_err(|_| ())?;

    for i in 0..nph {
        let phdr = phdr_from_bytes(&phdr_bytes[i * phsz..(i + 1) * phsz]);

        match phdr.p_type {
            PT_LOAD => {
                // CONFIG_X86_64: alignment must be a multiple of 2 MiB.
                if (phdr.p_align % 0x0020_0000) != 0 {
                    error_msg(sink, "Alignment of LOAD segment isn't multiple of 2MB");
                    return Err(());
                }
                // CONFIG_RELOCATABLE: dest = output + (p_paddr - LOAD_PHYSICAL_ADDR)
                let dest = output.wrapping_add(phdr.p_paddr.wrapping_sub(LOAD_PHYSICAL_ADDR));
                env.move_segment(phdr.p_offset, dest, phdr.p_filesz);
            }
            _ => { /* Ignore other PT_* */ }
        }
    }

    // free(phdrs);  -- nothing to do for the Vec on the host.
    Ok(ehdr.e_entry.wrapping_sub(LOAD_PHYSICAL_ADDR))
}

/// `decompress_kernel(outbuf, virt_addr, error)` — misc.c:344-362.
/// Faithful mirror: run `__decompress`; on failure return `ULONG_MAX`;
/// otherwise `parse_elf` then `handle_relocations`, returning the entry
/// offset.
#[allow(clippy::too_many_arguments)]
pub fn decompress_kernel<E: MiscEnv, S: ErrorSink>(
    env: &mut E,
    sink: &mut S,
    input: &[u8],
    output: u64,
    output_buf: &mut [u8],
    output_len: u64,
    virt_addr: u64,
    bss_minus_text: u64,
) -> u64 {
    if env.decompress(input, output_buf).is_err() {
        return ULONG_MAX;
    }
    let entry = match parse_elf(env, sink, output) {
        Ok(e) => e,
        Err(()) => return ULONG_MAX,
    };
    handle_relocations(env, sink, output, output_len, virt_addr, bss_minus_text);
    entry
}

fn ehdr_from_bytes(b: &[u8]) -> Elf64Ehdr {
    let mut e_ident = [0u8; EI_NIDENT];
    e_ident.copy_from_slice(&b[0..EI_NIDENT]);
    let g16 = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]);
    let g32 = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let g64 = |o: usize| {
        u64::from_le_bytes([
            b[o],
            b[o + 1],
            b[o + 2],
            b[o + 3],
            b[o + 4],
            b[o + 5],
            b[o + 6],
            b[o + 7],
        ])
    };
    Elf64Ehdr {
        e_ident,
        e_type: g16(16),
        e_machine: g16(18),
        e_version: g32(20),
        e_entry: g64(24),
        e_phoff: g64(32),
        e_shoff: g64(40),
        e_flags: g32(48),
        e_ehsize: g16(52),
        e_phentsize: g16(54),
        e_phnum: g16(56),
        e_shentsize: g16(58),
        e_shnum: g16(60),
        e_shstrndx: g16(62),
    }
}

fn phdr_from_bytes(b: &[u8]) -> Elf64Phdr {
    let g32 = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
    let g64 = |o: usize| {
        u64::from_le_bytes([
            b[o],
            b[o + 1],
            b[o + 2],
            b[o + 3],
            b[o + 4],
            b[o + 5],
            b[o + 6],
            b[o + 7],
        ])
    };
    Elf64Phdr {
        p_type: g32(0),
        p_flags: g32(4),
        p_offset: g64(8),
        p_vaddr: g64(16),
        p_paddr: g64(24),
        p_filesz: g64(32),
        p_memsz: g64(40),
        p_align: g64(48),
    }
}

/// Encode an `Elf64Ehdr` to its 64-byte on-disk form (little-endian).
/// Test helper that mirrors the exact `struct elf64_hdr` layout so the
/// parse path can be exercised round-trip.
pub fn ehdr_to_bytes(e: &Elf64Ehdr) -> [u8; 64] {
    let mut b = [0u8; 64];
    b[0..EI_NIDENT].copy_from_slice(&e.e_ident);
    b[16..18].copy_from_slice(&e.e_type.to_le_bytes());
    b[18..20].copy_from_slice(&e.e_machine.to_le_bytes());
    b[20..24].copy_from_slice(&e.e_version.to_le_bytes());
    b[24..32].copy_from_slice(&e.e_entry.to_le_bytes());
    b[32..40].copy_from_slice(&e.e_phoff.to_le_bytes());
    b[40..48].copy_from_slice(&e.e_shoff.to_le_bytes());
    b[48..52].copy_from_slice(&e.e_flags.to_le_bytes());
    b[52..54].copy_from_slice(&e.e_ehsize.to_le_bytes());
    b[54..56].copy_from_slice(&e.e_phentsize.to_le_bytes());
    b[56..58].copy_from_slice(&e.e_phnum.to_le_bytes());
    b[58..60].copy_from_slice(&e.e_shentsize.to_le_bytes());
    b[60..62].copy_from_slice(&e.e_shnum.to_le_bytes());
    b[62..64].copy_from_slice(&e.e_shstrndx.to_le_bytes());
    b
}

/// Encode an `Elf64Phdr` to its 56-byte on-disk form (little-endian).
pub fn phdr_to_bytes(p: &Elf64Phdr) -> [u8; 56] {
    let mut b = [0u8; 56];
    b[0..4].copy_from_slice(&p.p_type.to_le_bytes());
    b[4..8].copy_from_slice(&p.p_flags.to_le_bytes());
    b[8..16].copy_from_slice(&p.p_offset.to_le_bytes());
    b[16..24].copy_from_slice(&p.p_vaddr.to_le_bytes());
    b[24..32].copy_from_slice(&p.p_paddr.to_le_bytes());
    b[32..40].copy_from_slice(&p.p_filesz.to_le_bytes());
    b[40..48].copy_from_slice(&p.p_memsz.to_le_bytes());
    b[48..56].copy_from_slice(&p.p_align.to_le_bytes());
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::string::{String, ToString};

    /// Backing image + recorder used to exercise parse_elf /
    /// handle_relocations / decompress_kernel without real hardware.
    struct TestEnv {
        /// Decompressed image bytes (what `output` points at).
        image: Vec<u8>,
        /// 32-bit relocation words, last-word-first (read_reloc index 0 =
        /// the final 32-bit word of the image).
        relocs: Vec<i32>,
        /// Recorded segment moves: (src_off, dest, len).
        moves: Vec<(u64, u64, u64)>,
        /// Recorded relocation patches: (ptr, width_bits, delta).
        patches: Vec<(u64, u32, u64)>,
        /// Console output.
        out: String,
        /// Force `decompress` to fail.
        fail_decompress: bool,
        /// Optional bytes the fake `__decompress` writes to outbuf.
        decompressed_image: Option<Vec<u8>>,
        /// Recorded `__decompress` inputs and output buffer lengths.
        decompress_calls: Vec<(Vec<u8>, usize)>,
        /// Recorded extract_kernel orchestration calls.
        calls: Vec<&'static str>,
        /// Result returned by the KASLR placement seam.
        chosen: ChosenLocation,
        /// RSDP address returned by the ACPI seam.
        rsdp: u64,
        /// Accepted-memory calls: (physical start, size).
        accepted: Vec<(u64, u64)>,
        /// Free-memory bounds set by extract_kernel.
        free_mem: Option<(u64, u64)>,
        /// Video console globals set by extract_kernel.
        video_console: Option<(u64, u16, u8, u8)>,
        /// Whether init_unaccepted_memory should report work.
        unaccepted_memory: bool,
        /// Spurious NMI count reported after cleanup.
        spurious_nmi_count: u64,
    }

    impl MiscEnv for TestEnv {
        fn putstr(&mut self, s: &str) {
            self.out.push_str(s);
        }
        fn decompress(&mut self, input: &[u8], output: &mut [u8]) -> Result<(), ()> {
            self.calls.push("decompress");
            if self.fail_decompress {
                return Err(());
            }
            self.decompress_calls.push((input.to_vec(), output.len()));
            let payload = self.decompressed_image.as_deref().unwrap_or(input);
            let n = payload.len().min(output.len());
            output[..n].copy_from_slice(&payload[..n]);
            Ok(())
        }
        fn read_output(&self, off: u64, len: usize) -> Result<Vec<u8>, ()> {
            let start = off as usize;
            let end = start.checked_add(len).ok_or(())?;
            if end > self.image.len() {
                return Err(());
            }
            Ok(self.image[start..end].to_vec())
        }
        fn move_segment(&mut self, src_off: u64, dest: u64, len: u64) {
            self.calls.push("move_segment");
            self.moves.push((src_off, dest, len));
        }
        fn apply_reloc32(&mut self, ptr: u64, delta: u64) -> u32 {
            self.patches.push((ptr, 32, delta));
            delta as u32
        }
        fn apply_reloc64(&mut self, ptr: u64, delta: u64) -> u64 {
            self.patches.push((ptr, 64, delta));
            delta
        }
        fn read_reloc(&self, index: usize) -> i32 {
            *self.relocs.get(index).unwrap_or(&0)
        }
    }

    impl ExtractKernelEnv for TestEnv {
        fn retain_boot_params(&mut self, _rmode_addr: u64) {
            self.calls.push("retain_boot_params");
        }
        fn parse_mem_encrypt(&mut self, _boot_params: &mut BootParams) {
            self.calls.push("parse_mem_encrypt");
        }
        fn sanitize_boot_params(&mut self, _boot_params: &mut BootParams) {
            self.calls.push("sanitize_boot_params");
        }
        fn set_video_console(&mut self, vidmem: u64, vidport: u16, lines: u8, cols: u8) {
            self.calls.push("set_video_console");
            self.video_console = Some((vidmem, vidport, lines, cols));
        }
        fn init_default_io_ops(&mut self) {
            self.calls.push("init_default_io_ops");
        }
        fn early_tdx_detect(&mut self) {
            self.calls.push("early_tdx_detect");
        }
        fn early_sev_detect(&mut self) {
            self.calls.push("early_sev_detect");
        }
        fn console_init(&mut self) {
            self.calls.push("console_init");
        }
        fn get_rsdp_addr(&mut self) -> u64 {
            self.calls.push("get_rsdp_addr");
            self.rsdp
        }
        fn debug_putstr(&mut self, s: &str) {
            self.calls.push("debug_putstr");
            self.out.push_str(s);
        }
        fn debug_putaddr(&mut self, _value: u64) {
            self.calls.push("debug_putaddr");
        }
        fn debug_puthex(&mut self, value: u64) {
            self.calls.push("debug_puthex");
            self.out.push_str(&format!("{value:x}"));
        }
        fn set_free_mem_bounds(&mut self, start: u64, end: u64) {
            self.calls.push("set_free_mem_bounds");
            self.free_mem = Some((start, end));
        }
        fn choose_random_location(
            &mut self,
            _input: u64,
            _input_size: u64,
            _output: u64,
            _output_size: u64,
            _virt_addr: u64,
        ) -> ChosenLocation {
            self.calls.push("choose_random_location");
            self.chosen
        }
        fn init_unaccepted_memory(&mut self) -> bool {
            self.calls.push("init_unaccepted_memory");
            self.unaccepted_memory
        }
        fn accept_memory(&mut self, phys: u64, size: u64) {
            self.calls.push("accept_memory");
            self.accepted.push((phys, size));
        }
        fn cleanup_exception_handling(&mut self) {
            self.calls.push("cleanup_exception_handling");
        }
        fn spurious_nmi_count(&self) -> u64 {
            self.spurious_nmi_count
        }
        fn error_putstr(&mut self, s: &str) {
            self.calls.push("error_putstr");
            self.out.push_str(s);
        }
        fn error_putdec(&mut self, value: u64) {
            self.calls.push("error_putdec");
            self.out.push_str(&value.to_string());
        }
    }

    struct CapSink(String);
    impl ErrorSink for CapSink {
        fn putstr(&mut self, msg: &str) {
            self.0.push_str(msg);
        }
    }

    fn fresh_env() -> TestEnv {
        TestEnv {
            image: Vec::new(),
            relocs: Vec::new(),
            moves: Vec::new(),
            patches: Vec::new(),
            out: String::new(),
            fail_decompress: false,
            decompressed_image: None,
            decompress_calls: Vec::new(),
            calls: Vec::new(),
            chosen: ChosenLocation {
                output: 0,
                virt_addr: LOAD_PHYSICAL_ADDR,
                randomized: false,
            },
            rsdp: 0,
            accepted: Vec::new(),
            free_mem: None,
            video_console: None,
            unaccepted_memory: false,
            spurious_nmi_count: 0,
        }
    }

    // ---- constants ---------------------------------------------------

    #[test]
    fn load_physical_addr_is_aligned_physical_start() {
        // ALIGN(0x1000000, 0x200000) == 0x1000000. misc.c relocations and
        // parse_elf subtract this from p_paddr/e_entry.
        assert_eq!(LOAD_PHYSICAL_ADDR, 0x0100_0000);
    }

    #[test]
    fn min_kernel_align_is_2mib_on_x86_64() {
        // boot.h: MIN_KERNEL_ALIGN = 1 << PMD_SHIFT, PMD_SHIFT = 21.
        assert_eq!(MIN_KERNEL_ALIGN, 0x0020_0000);
    }

    #[test]
    fn boot_heap_size_matches_linux_gzip_configuration() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/arch/x86/include/asm/boot.h"
        ));
        assert!(source.contains("# define BOOT_HEAP_SIZE\t\t 0x10000"));
        assert_eq!(BOOT_HEAP_SIZE, 0x0001_0000);
    }

    #[test]
    fn start_kernel_map_matches_page_64_types() {
        assert_eq!(START_KERNEL_MAP, 0xffff_ffff_8000_0000);
    }

    #[test]
    fn elf_magic_constants_match_uapi_elf_h() {
        assert_eq!(ELFMAG0, 0x7f);
        assert_eq!(ELFMAG1, b'E');
        assert_eq!(ELFMAG2, b'L');
        assert_eq!(ELFMAG3, b'F');
        assert_eq!((EI_MAG0, EI_MAG1, EI_MAG2, EI_MAG3), (0, 1, 2, 3));
        assert_eq!(EI_NIDENT, 16);
    }

    #[test]
    fn pt_constants_match_elf_h() {
        assert_eq!(PT_LOAD, 1);
        assert_eq!(PT_DYNAMIC, 2);
        assert_eq!(PT_INTERP, 3);
    }

    #[test]
    fn gzip_preboot_header_matches_linux_inflate_contract() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/decompress_inflate.c"
        ));
        assert!(source.contains("zbuf[0] != 0x1f || zbuf[1] != 0x8b || zbuf[2] != 0x08"));
        assert!(source.contains("strm->next_in = zbuf + 10;"));
        assert!(source.contains("strm->avail_in = len - 10;"));
        assert!(source.contains("if (zbuf[3] & 0x8)"));
        assert!(source.contains("error(\"header error\")"));
        assert!(source.contains("*pos = strm->next_in - zbuf+8;"));

        let plain = [0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 3, 1, 0, 0xff];
        assert_eq!(
            gzip_preboot_header(&plain),
            Ok(GzipHeader {
                deflate_offset: GZIP_HEADER_SIZE,
                avail_in: plain.len() - GZIP_HEADER_SIZE,
            })
        );

        let with_name = [
            0x1f,
            0x8b,
            0x08,
            GZIP_FLAG_FNAME,
            0,
            0,
            0,
            0,
            0,
            3,
            b'v',
            b'm',
            0,
            1,
            0,
            0xff,
        ];
        assert_eq!(
            gzip_preboot_header(&with_name),
            Ok(GzipHeader {
                deflate_offset: 13,
                avail_in: with_name.len() - 13,
            })
        );
        assert_eq!(gzip_preboot_pos(13 + 7), 28);

        assert_eq!(
            gzip_preboot_header(&[0x1f, 0x8b, 0x00]),
            Err(GzipHeaderError::NotGzip)
        );
        assert_eq!(
            gzip_preboot_header(&[0x1f, 0x8b, 0x08, GZIP_FLAG_FNAME, 0, 0, 0, 0, 0, 3, b'v',]),
            Err(GzipHeaderError::HeaderError)
        );
    }

    #[test]
    fn elf64_struct_sizes_match_abi() {
        // struct elf64_hdr is 64 bytes; struct elf64_phdr is 56 bytes.
        assert_eq!(core::mem::size_of::<Elf64Ehdr>(), 64);
        assert_eq!(core::mem::size_of::<Elf64Phdr>(), 56);
    }

    // ---- extract_kernel ---------------------------------------------

    fn extract_test_config<'a>(input: &'a [u8]) -> ExtractKernelConfig<'a> {
        ExtractKernelConfig {
            rmode_addr: 0x7000,
            input_data_addr: 0x20_0000,
            input,
            input_len: input.len() as u64,
            output: 0x0200_0000,
            output_len: input.len() as u64,
            kernel_total_size: MIN_KERNEL_ALIGN + 0x1000,
            bss_minus_text: 0x10_0000,
            heap: 0x90000,
            trampoline_32bit: 0x70000,
            relocatable: true,
        }
    }

    fn call_pos(calls: &[&'static str], name: &'static str) -> usize {
        calls
            .iter()
            .position(|&call| call == name)
            .unwrap_or_else(|| panic!("missing call {name}; calls={calls:?}"))
    }

    #[test]
    fn extract_kernel_orchestration_matches_linux_call_order() {
        let (img, entry) = build_image_with_one_load_segment();
        let chosen_output = 0x4000_0000;
        let chosen_virt = LOAD_PHYSICAL_ADDR + MIN_KERNEL_ALIGN;
        let mut env = fresh_env();
        env.image = img.clone();
        env.chosen = ChosenLocation {
            output: chosen_output,
            virt_addr: chosen_virt,
            randomized: true,
        };
        env.rsdp = 0x000f_1234;
        env.unaccepted_memory = true;
        env.spurious_nmi_count = 2;

        let mut bp = BootParams::new();
        bp.set_loadflags(0x80 | KASLR_FLAG);
        bp.set_screen_orig_video_mode(7);
        bp.set_screen_orig_video_cols(80);
        bp.set_screen_orig_video_lines(25);
        let mut sink = CapSink(String::new());
        let mut outbuf = vec![0u8; img.len()];

        let result = extract_kernel(
            &mut env,
            &mut sink,
            &mut bp,
            extract_test_config(&img),
            &mut outbuf,
        )
        .expect("extract_kernel should complete");

        assert_eq!(result.output, chosen_output);
        assert_eq!(result.virt_addr, chosen_virt);
        assert_eq!(result.needed_size, MIN_KERNEL_ALIGN * 2);
        assert_eq!(result.entry_offset, entry - LOAD_PHYSICAL_ADDR);
        assert_eq!(result.entry_addr, chosen_output + result.entry_offset);
        assert_eq!(bp.loadflags() & KASLR_FLAG, KASLR_FLAG);
        assert_eq!(bp.loadflags() & 0x80, 0x80);
        assert_eq!(bp.acpi_rsdp_addr(), env.rsdp);
        assert_eq!(env.video_console, Some((0xb0000, 0x3b4, 25, 80)));
        assert_eq!(
            env.free_mem,
            Some((0x90000, 0x90000 + BOOT_HEAP_SIZE as u64))
        );
        assert_eq!(env.accepted, vec![(chosen_output, result.needed_size)]);

        let calls = &env.calls;
        assert!(
            call_pos(calls, "retain_boot_params") < call_pos(calls, "parse_mem_encrypt")
                && call_pos(calls, "parse_mem_encrypt") < call_pos(calls, "sanitize_boot_params")
                && call_pos(calls, "sanitize_boot_params") < call_pos(calls, "set_video_console")
                && call_pos(calls, "set_video_console") < call_pos(calls, "init_default_io_ops")
                && call_pos(calls, "init_default_io_ops") < call_pos(calls, "early_tdx_detect")
                && call_pos(calls, "early_tdx_detect") < call_pos(calls, "early_sev_detect")
                && call_pos(calls, "early_sev_detect") < call_pos(calls, "console_init")
                && call_pos(calls, "console_init") < call_pos(calls, "get_rsdp_addr")
                && call_pos(calls, "get_rsdp_addr") < call_pos(calls, "set_free_mem_bounds")
                && call_pos(calls, "set_free_mem_bounds")
                    < call_pos(calls, "choose_random_location")
                && call_pos(calls, "choose_random_location")
                    < call_pos(calls, "init_unaccepted_memory")
                && call_pos(calls, "init_unaccepted_memory") < call_pos(calls, "accept_memory")
                && call_pos(calls, "accept_memory") < call_pos(calls, "decompress")
                && call_pos(calls, "decompress") < call_pos(calls, "move_segment")
                && call_pos(calls, "move_segment") < call_pos(calls, "cleanup_exception_handling")
                && call_pos(calls, "cleanup_exception_handling") < call_pos(calls, "error_putdec"),
            "extract_kernel call order drifted: {calls:?}"
        );
    }

    #[test]
    fn extract_kernel_clears_kaslr_flag_when_randomization_is_disabled() {
        let (img, _) = build_image_with_one_load_segment();
        let mut env = fresh_env();
        env.image = img.clone();
        env.chosen = ChosenLocation {
            output: 0x0200_0000,
            virt_addr: LOAD_PHYSICAL_ADDR,
            randomized: false,
        };
        let mut bp = BootParams::new();
        bp.set_loadflags(0x80 | KASLR_FLAG);
        bp.set_screen_orig_video_cols(80);
        bp.set_screen_orig_video_lines(25);
        let mut sink = CapSink(String::new());
        let mut outbuf = vec![0u8; img.len()];

        extract_kernel(
            &mut env,
            &mut sink,
            &mut bp,
            extract_test_config(&img),
            &mut outbuf,
        )
        .expect("extract_kernel should complete");

        assert_eq!(bp.loadflags() & KASLR_FLAG, 0);
        assert_eq!(bp.loadflags() & 0x80, 0x80);
    }

    #[test]
    fn extract_kernel_rejects_unaligned_chosen_physical_address_before_decompress() {
        let mut env = fresh_env();
        env.chosen = ChosenLocation {
            output: 0x0200_1000,
            virt_addr: LOAD_PHYSICAL_ADDR,
            randomized: false,
        };
        let mut bp = BootParams::new();
        let mut sink = CapSink(String::new());
        let mut outbuf = [];
        let err = extract_kernel(
            &mut env,
            &mut sink,
            &mut bp,
            extract_test_config(&[]),
            &mut outbuf,
        )
        .expect_err("unaligned output should be rejected");

        assert_eq!(err, ExtractKernelError::DestinationPhysicalAlignment);
        assert!(sink.0.contains("Destination physical address"));
        assert!(env.calls.contains(&"choose_random_location"));
        assert!(!env.calls.contains(&"decompress"));
    }

    // ---- __putnum / __puthex / __putdec ------------------------------

    #[test]
    fn puthex_pads_to_sizeof_value_times_two_digits() {
        // misc.c __puthex uses mindig = sizeof(unsigned long)*2 = 16.
        let mut env = fresh_env();
        puthex(&mut env, 0x1f);
        assert_eq!(env.out, "000000000000001f");
    }

    #[test]
    fn putdec_uses_minimum_one_digit_and_no_padding() {
        let mut env = fresh_env();
        putdec(&mut env, 0);
        assert_eq!(env.out, "0");
        env.out.clear();
        putdec(&mut env, 12345);
        assert_eq!(env.out, "12345");
    }

    // ---- parse_elf ---------------------------------------------------

    fn build_image_with_one_load_segment() -> (Vec<u8>, u64) {
        // ehdr at offset 0; phdrs right after; entry chosen so the offset
        // is observable.
        let phoff = 64u64;
        let entry = LOAD_PHYSICAL_ADDR + 0x40; // entry offset 0x40
        let mut ehdr = Elf64Ehdr {
            e_ident: [0; EI_NIDENT],
            e_type: 2,
            e_machine: 0x3e,
            e_version: 1,
            e_entry: entry,
            e_phoff: phoff,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: 64,
            e_phentsize: 56,
            e_phnum: 1,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        };
        ehdr.e_ident[EI_MAG0] = ELFMAG0;
        ehdr.e_ident[EI_MAG1] = ELFMAG1;
        ehdr.e_ident[EI_MAG2] = ELFMAG2;
        ehdr.e_ident[EI_MAG3] = ELFMAG3;

        let phdr = Elf64Phdr {
            p_type: PT_LOAD,
            p_flags: 5,
            p_offset: 0x1000,
            p_vaddr: 0,
            p_paddr: LOAD_PHYSICAL_ADDR + 0x20_0000,
            p_filesz: 0x800,
            p_memsz: 0x800,
            p_align: 0x20_0000,
        };

        let mut img = Vec::new();
        img.extend_from_slice(&ehdr_to_bytes(&ehdr));
        img.extend_from_slice(&phdr_to_bytes(&phdr));
        (img, entry)
    }

    #[test]
    fn parse_elf_rejects_image_without_elf_magic() {
        let mut env = fresh_env();
        env.image = vec![0u8; 64]; // all zero -> no magic
        let mut sink = CapSink(String::new());
        let r = parse_elf(&mut env, &mut sink, 0);
        assert!(r.is_err());
        assert!(sink.0.contains("not a valid ELF"));
    }

    #[test]
    fn parse_elf_moves_load_segment_to_output_relative_dest() {
        let (img, entry) = build_image_with_one_load_segment();
        let mut env = fresh_env();
        env.image = img;
        let mut sink = CapSink(String::new());
        let output = 0x4000_0000u64;
        let off = parse_elf(&mut env, &mut sink, output).unwrap();

        // Return value is e_entry - LOAD_PHYSICAL_ADDR.
        assert_eq!(off, entry - LOAD_PHYSICAL_ADDR);
        // One PT_LOAD recorded: dest = output + (p_paddr - LOAD_PHYSICAL_ADDR).
        assert_eq!(env.moves.len(), 1);
        let (src_off, dest, len) = env.moves[0];
        assert_eq!(src_off, 0x1000);
        assert_eq!(dest, output + 0x20_0000);
        assert_eq!(len, 0x800);
    }

    #[test]
    fn parse_elf_rejects_unaligned_load_segment() {
        let (mut img, _) = build_image_with_one_load_segment();
        // Corrupt p_align of the single phdr (located at offset 64, p_align
        // field at +48) to a non-2MB-multiple value.
        let palign_off = 64 + 48;
        img[palign_off..palign_off + 8].copy_from_slice(&0x1234u64.to_le_bytes());
        let mut env = fresh_env();
        env.image = img;
        let mut sink = CapSink(String::new());
        let r = parse_elf(&mut env, &mut sink, 0);
        assert!(r.is_err());
        assert!(sink.0.contains("multiple of 2MB"));
    }

    #[test]
    fn parse_elf_ignores_non_load_segments() {
        // Build an image whose only phdr is PT_DYNAMIC -> no moves.
        let phoff = 64u64;
        let mut ehdr = Elf64Ehdr {
            e_ident: [0; EI_NIDENT],
            e_type: 2,
            e_machine: 0x3e,
            e_version: 1,
            e_entry: LOAD_PHYSICAL_ADDR,
            e_phoff: phoff,
            e_shoff: 0,
            e_flags: 0,
            e_ehsize: 64,
            e_phentsize: 56,
            e_phnum: 1,
            e_shentsize: 0,
            e_shnum: 0,
            e_shstrndx: 0,
        };
        ehdr.e_ident[EI_MAG0] = ELFMAG0;
        ehdr.e_ident[EI_MAG1] = ELFMAG1;
        ehdr.e_ident[EI_MAG2] = ELFMAG2;
        ehdr.e_ident[EI_MAG3] = ELFMAG3;
        let phdr = Elf64Phdr {
            p_type: PT_DYNAMIC,
            p_flags: 6,
            p_offset: 0x2000,
            p_vaddr: 0,
            p_paddr: 0,
            p_filesz: 0x10,
            p_memsz: 0x10,
            p_align: 8,
        };
        let mut img = Vec::new();
        img.extend_from_slice(&ehdr_to_bytes(&ehdr));
        img.extend_from_slice(&phdr_to_bytes(&phdr));

        let mut env = fresh_env();
        env.image = img;
        let mut sink = CapSink(String::new());
        let off = parse_elf(&mut env, &mut sink, 0x1000).unwrap();
        assert_eq!(off, 0); // e_entry == LOAD_PHYSICAL_ADDR
        assert!(env.moves.is_empty());
    }

    // ---- handle_relocations ------------------------------------------

    #[test]
    fn handle_relocations_noop_when_delta_zero() {
        // virt_addr == LOAD_PHYSICAL_ADDR -> delta == 0 -> "No relocation".
        let mut env = fresh_env();
        let mut sink = CapSink(String::new());
        let (c32, c64) = handle_relocations(
            &mut env,
            &mut sink,
            LOAD_PHYSICAL_ADDR, // output
            0x1000,             // output_len
            LOAD_PHYSICAL_ADDR, // virt_addr -> delta 0
            0x10_0000,          // bss-text span
        );
        assert_eq!((c32, c64), (0, 0));
        assert!(env.out.contains("No relocation needed"));
        assert!(env.patches.is_empty());
    }

    #[test]
    fn handle_relocations_applies_in_range_32_and_64_bit_entries() {
        // Lay out: output at LOAD_PHYSICAL_ADDR so map = -__START_KERNEL_map.
        // A reloc word R yields ptr = (R + map). To land inside
        // [min_addr, max_addr] = [output, output+span] we pick
        // R = __START_KERNEL_map + (output + k), giving ptr = output + k.
        let output = LOAD_PHYSICAL_ADDR;
        let span = 0x10_0000u64;
        // map = (output - LOAD_PHYSICAL_ADDR) - __START_KERNEL_map = -SKM.
        // ptr = R(sign-extended) + map. We need a 32-bit reloc; the kernel
        // table stores 32-bit values sign-extended. Choose R so that
        // (R as i64) + map == output (== ptr in range). map is huge, so the
        // arithmetic wraps; we just assert the in-range entry is patched and
        // the terminator stops the walk.
        //
        // Construct directly: pick ptr_target = output, solve R:
        let map = (output.wrapping_sub(LOAD_PHYSICAL_ADDR)).wrapping_sub(START_KERNEL_MAP);
        let ptr_target = output; // in range [output, output+span]
        let r_full = ptr_target.wrapping_sub(map); // R sign-extended == this
        // Low 32 bits; sign-extension reproduces r_full only if r_full fits.
        let r32 = r_full as i32;
        // Verify our reconstruction.
        assert_eq!((r32 as i64 as u64).wrapping_add(map), ptr_target);

        let mut env = fresh_env();
        // 32-bit table: [r32, 0 terminator]; then 64-bit table: [r32, 0].
        env.relocs = vec![r32, 0, r32, 0];
        let mut sink = CapSink(String::new());

        // virt_addr != LOAD_PHYSICAL_ADDR so delta != 0.
        let virt = LOAD_PHYSICAL_ADDR + 0x20_0000;
        let (c32, c64) = handle_relocations(&mut env, &mut sink, output, 0x1000, virt, span);
        assert_eq!(c32, 1);
        assert_eq!(c64, 1);
        // Two patches recorded: one 32-bit, one 64-bit, both at ptr_target.
        assert_eq!(env.patches.len(), 2);
        assert_eq!(env.patches[0], (ptr_target, 32, virt - LOAD_PHYSICAL_ADDR));
        assert_eq!(env.patches[1], (ptr_target, 64, virt - LOAD_PHYSICAL_ADDR));
    }

    #[test]
    fn handle_relocations_halts_on_out_of_range_pointer() {
        let output = LOAD_PHYSICAL_ADDR;
        let span = 0x10_0000u64;
        let map = (output.wrapping_sub(LOAD_PHYSICAL_ADDR)).wrapping_sub(START_KERNEL_MAP);
        // Target a pointer well above max_addr (= output + span).
        let ptr_target = output + span + 0x1000;
        let r_full = ptr_target.wrapping_sub(map);
        let r32 = r_full as i32;
        // Only patch if reconstruction is faithful; otherwise the test of
        // out-of-range still holds because ptr will differ but remain out of
        // range. Assert it's genuinely out of range:
        assert!((r32 as i64 as u64).wrapping_add(map) > output + span);

        let mut env = fresh_env();
        env.relocs = vec![r32, 0];
        let mut sink = CapSink(String::new());
        let virt = LOAD_PHYSICAL_ADDR + 0x20_0000;
        let (c32, _c64) = handle_relocations(&mut env, &mut sink, output, 0x1000, virt, span);
        assert_eq!(c32, 0);
        assert!(sink.0.contains("32-bit relocation outside of kernel"));
    }

    // ---- decompress_kernel -------------------------------------------

    #[test]
    fn decompress_kernel_returns_ulong_max_on_decompress_failure() {
        let mut env = fresh_env();
        env.fail_decompress = true;
        let mut sink = CapSink(String::new());
        let mut outbuf = [0u8; 16];
        let r = decompress_kernel(
            &mut env,
            &mut sink,
            &[1, 2, 3],
            0x4000_0000,
            &mut outbuf,
            0x10,
            LOAD_PHYSICAL_ADDR,
            0x10_0000,
        );
        assert_eq!(r, ULONG_MAX);
    }

    #[test]
    fn decompress_kernel_returns_entry_offset_on_success() {
        let (img, entry) = build_image_with_one_load_segment();
        // decompress copies `input` into the output buffer; we then parse
        // `env.image` (which the test seeds to the same bytes).
        let mut env = fresh_env();
        env.image = img.clone();
        let mut sink = CapSink(String::new());
        let mut outbuf = vec![0u8; img.len()];
        let r = decompress_kernel(
            &mut env,
            &mut sink,
            &img,
            0x4000_0000,
            &mut outbuf,
            img.len() as u64,
            LOAD_PHYSICAL_ADDR, // virt == LOAD -> delta 0, relocations skipped
            0x10_0000,
        );
        assert_eq!(r, entry - LOAD_PHYSICAL_ADDR);
        // Output buffer received the decompressed bytes (identity copy).
        assert_eq!(&outbuf[..], &img[..]);
    }

    #[test]
    fn decompress_kernel_passes_full_gzip_input_to_decompressor() {
        let (img, entry) = build_image_with_one_load_segment();
        let mut compressed = vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0];
        compressed.extend_from_slice(b"deflated-payload-placeholder");
        compressed.extend_from_slice(&0u32.to_le_bytes());
        compressed.extend_from_slice(&(img.len() as u32).to_le_bytes());

        assert_eq!(
            crate::lib::decompress::decompress_method_name(&compressed),
            Some("gzip")
        );

        let mut env = fresh_env();
        env.image = img.clone();
        env.decompressed_image = Some(img.clone());
        let mut sink = CapSink(String::new());
        let mut outbuf = vec![0u8; img.len()];
        let r = decompress_kernel(
            &mut env,
            &mut sink,
            &compressed,
            0x4000_0000,
            &mut outbuf,
            img.len() as u64,
            LOAD_PHYSICAL_ADDR,
            0x10_0000,
        );

        assert_eq!(r, entry - LOAD_PHYSICAL_ADDR);
        assert_eq!(env.decompress_calls.len(), 1);
        assert_eq!(env.decompress_calls[0].0, compressed);
        assert_eq!(env.decompress_calls[0].1, img.len());
        assert_eq!(&outbuf[..], &img[..]);
    }
}
