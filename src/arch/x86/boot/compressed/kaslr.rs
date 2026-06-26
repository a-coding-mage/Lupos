//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/compressed/kaslr.c
//! test-origin: linux:vendor/linux/arch/x86/boot/compressed/kaslr.c
//! KASLR base-address randomisation for the decompressor.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/compressed/kaslr.c
//!
//! Faithful 1:1 translation of kaslr.c. The algorithm:
//!   1. `mem_avoid_init` records the ranges KASLR must not place the
//!      kernel on (the compressed image + run space, initrd, cmdline,
//!      boot_params, `memmap=`/`mem=` restrictions, setup_data).
//!   2. `choose_random_location` walks the available memory entries
//!      (e820 / EFI / KHO), and for each calls `process_mem_region` →
//!      `__process_mem_region`, which clips against `mem_avoid` and the
//!      memory limit and stores usable "slot areas".
//!   3. `slots_fetch_random` / `find_random_virt_addr` pick a slot using
//!      `kaslr_get_random_long`.
//!
//! The data the decompressor reads from firmware (boot_params, the e820
//! table, the EFI memmap, the setup_data list, the cmdline) and the
//! entropy source are provided through the [`KaslrEnv`] seam, exactly the
//! way kaslr.c reaches them through `boot_params_ptr`,
//! `get_cmd_line_ptr()`, `count_immovable_mem_regions()` and
//! `kaslr_get_random_long()`. Each seam method cites the kaslr.c line it
//! stands in for.

use crate::arch::x86::kernel::kaslr::{KaslrEntropy, kaslr_get_random_long};

extern crate alloc;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Architectural constants (exact mirrors of the headers kaslr.c pulls in).
// ---------------------------------------------------------------------------

/// `CONFIG_PHYSICAL_ALIGN` — default 0x200000 (2 MiB).
/// Ref: vendor/linux/arch/x86/Kconfig:2133.
pub const CONFIG_PHYSICAL_ALIGN: u64 = 0x0020_0000;

/// `LOAD_PHYSICAL_ADDR` — `ALIGN(0x1000000, 0x200000)` == 0x1000000.
/// Ref: vendor/linux/arch/x86/include/asm/page_types.h:32.
pub const LOAD_PHYSICAL_ADDR: u64 = 0x0100_0000;

/// `KERNEL_IMAGE_SIZE` — 1 GiB with CONFIG_RANDOMIZE_BASE.
/// Ref: vendor/linux/arch/x86/include/asm/page_64_types.h:85.
pub const KERNEL_IMAGE_SIZE: u64 = 1024 * 1024 * 1024;

/// `MAXMEM = 1UL << MAX_PHYSMEM_BITS`; on 4-level paging MAX_PHYSMEM_BITS
/// is 46, so MAXMEM = 0x4000_0000_0000 (64 TiB).
/// Ref: vendor/linux/arch/x86/include/asm/pgtable_64_types.h:96 and
/// vendor/linux/arch/x86/include/asm/sparsemem.h:29.
pub const MAXMEM: u64 = 1 << 46;

/// `PUD_SIZE` / `PUD_SHIFT` — 1 GiB huge-page granularity.
/// Ref: vendor/linux/arch/x86/include/asm/pgtable_64_types.h:67,84.
pub const PUD_SHIFT: u32 = 30;
pub const PUD_SIZE: u64 = 1 << PUD_SHIFT;

/// `COMMAND_LINE_SIZE`. Ref: vendor/linux/arch/x86/include/asm/setup.h
/// (boot protocol max cmdline length).
pub const COMMAND_LINE_SIZE: usize = 2048;

/// `E820_TYPE_RAM`. Ref: vendor/linux/arch/x86/include/asm/e820/types.h.
pub const E820_TYPE_RAM: u32 = 1;

/// `MAX_MEMMAP_REGIONS` — kaslr.c:72. "Only supporting at most 4 unusable
/// memmap regions with kaslr".
pub const MAX_MEMMAP_REGIONS: usize = 4;

/// `MAX_SLOT_AREA` — kaslr.c:457.
pub const MAX_SLOT_AREA: usize = 100;

/// `enum mem_avoid_index` — kaslr.c:86-94. The MEMMAP slots occupy
/// `[MEM_AVOID_MEMMAP_BEGIN, MEM_AVOID_MEMMAP_END]`, and
/// `MEM_AVOID_MAX = MEM_AVOID_MEMMAP_BEGIN + MAX_MEMMAP_REGIONS`.
/// (The previous lupos value of 4 was WRONG; the real value is 8.)
pub const MEM_AVOID_ZO_RANGE: usize = 0;
pub const MEM_AVOID_INITRD: usize = 1;
pub const MEM_AVOID_CMDLINE: usize = 2;
pub const MEM_AVOID_BOOTPARAMS: usize = 3;
pub const MEM_AVOID_MEMMAP_BEGIN: usize = 4;
pub const MEM_AVOID_MEMMAP_END: usize = MEM_AVOID_MEMMAP_BEGIN + MAX_MEMMAP_REGIONS - 1; // 7
pub const MEM_AVOID_MAX: usize = MEM_AVOID_MEMMAP_BEGIN + MAX_MEMMAP_REGIONS; // 8

/// `struct mem_vector { u64 start; u64 size; }`. Ref: misc.h:97-100.
/// NOTE: this is `[start, start+size)` (start + **size**), matching the C.
/// (The previous lupos `MemVector` used `{start, end}` which silently
/// changed the overlap math — fixed here to mirror the ABI struct.)
#[repr(C)]
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct MemVector {
    pub start: u64,
    pub size: u64,
}

/// `struct slot_area { u64 addr; unsigned long num; }`. kaslr.c:452-455.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct SlotArea {
    pub addr: u64,
    pub num: u64,
}

/// `struct setup_data` node as seen by the decompressor. kaslr.c walks the
/// `boot_params->hdr.setup_data` linked list in `mem_avoid_overlap`.
/// Ref: vendor/linux/arch/x86/include/uapi/asm/setup_data.h:27-32.
#[derive(Copy, Clone, Debug)]
pub struct SetupDataNode {
    pub addr: u64,
    pub len: u32,
    pub type_: u32,
    pub next: u64,
    /// For SETUP_INDIRECT nodes whose inner type != SETUP_INDIRECT, the
    /// (addr, len) of the indirect payload. kaslr.c:434-444.
    pub indirect: Option<(u64, u64)>,
}

/// `SETUP_INDIRECT`. Ref: setup_data.h:19.
pub const SETUP_INDIRECT: u32 = 1 << 31;

// ---------------------------------------------------------------------------
// Helpers shared with the C source (lib/ctype.c, lib/cmdline.c, memparse).
// ---------------------------------------------------------------------------

/// `isspace(c)` for the ASCII control/space set lib/ctype.c uses. kaslr.c
/// re-enables it via `#include "../../../../lib/ctype.c"` (kaslr.c:115).
#[inline]
fn isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// `skip_spaces(str)` — kaslr.c:109-114.
fn skip_spaces(s: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < s.len() && isspace(s[i]) {
        i += 1;
    }
    &s[i..]
}

/// `memparse(ptr, &endp)` — parse a number with an optional K/M/G/T/P/E
/// suffix. Returns `(value, bytes_consumed)`. Mirrors lib/cmdline.c
/// `memparse()` which kaslr.c includes (kaslr.c:116).
fn memparse(p: &[u8]) -> (u64, usize) {
    use crate::arch::x86::boot::string::simple_strtoull;
    let (mut ret, mut consumed) = simple_strtoull(p, 0);
    let suffix = p.get(consumed).copied();
    match suffix {
        Some(b'E') | Some(b'e') => {
            ret <<= 60;
            consumed += 1;
        }
        Some(b'P') | Some(b'p') => {
            ret <<= 50;
            consumed += 1;
        }
        Some(b'T') | Some(b't') => {
            ret <<= 40;
            consumed += 1;
        }
        Some(b'G') | Some(b'g') => {
            ret <<= 30;
            consumed += 1;
        }
        Some(b'M') | Some(b'm') => {
            ret <<= 20;
            consumed += 1;
        }
        Some(b'K') | Some(b'k') => {
            ret <<= 10;
            consumed += 1;
        }
        _ => {}
    }
    (ret, consumed)
}

/// `ALIGN(x, a)` for power-of-two `a`.
#[inline]
fn align_up(x: u64, a: u64) -> u64 {
    (x + a - 1) & !(a - 1)
}

/// `ALIGN_DOWN(x, a)` for power-of-two `a`.
#[inline]
fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

// ---------------------------------------------------------------------------
// Environment seam (boot_params / firmware tables / entropy).
// ---------------------------------------------------------------------------

/// One available-memory entry the scan iterates over (an e820 RAM region,
/// an EFI free descriptor, or a KHO scratch area).
#[derive(Copy, Clone, Debug)]
pub struct MemRegionEntry {
    pub start: u64,
    pub size: u64,
}

/// Seam over everything kaslr.c reads from `boot_params_ptr`, the firmware
/// memory tables, the cmdline, the immovable-region count, and the entropy
/// source. Production wires these to the real boot_params / e820 / EFI /
/// setup_data structures and `kaslr_get_random_long`.
pub trait KaslrEnv {
    /// `boot_params_ptr->hdr.init_size` (mem_avoid_init, kaslr.c:357).
    fn init_size(&self) -> u64;
    /// 64-bit initrd start: `(ext_ramdisk_image << 32) | hdr.ramdisk_image`
    /// (kaslr.c:369-370).
    fn initrd_start(&self) -> u64;
    /// 64-bit initrd size: `(ext_ramdisk_size << 32) | hdr.ramdisk_size`
    /// (kaslr.c:371-372).
    fn initrd_size(&self) -> u64;
    /// `get_cmd_line_ptr()` — physical address of the cmdline, or 0
    /// (kaslr.c:378).
    fn cmd_line_ptr(&self) -> u64;
    /// `strnlen((char*)cmd_line, COMMAND_LINE_SIZE-1)` — length of the
    /// cmdline at `cmd_line_ptr()` (kaslr.c:381). 0 if no cmdline.
    fn cmd_line_len(&self) -> usize;
    /// The cmdline bytes (used by `handle_mem_options`). kaslr.c reads via
    /// `get_cmd_line_ptr()` then parses with next_arg/memparse.
    fn cmd_line_bytes(&self) -> &[u8];
    /// `(unsigned long)boot_params_ptr` (kaslr.c:387).
    fn boot_params_addr(&self) -> u64;
    /// `sizeof(*boot_params_ptr)` (kaslr.c:388).
    fn boot_params_size(&self) -> u64;
    /// `count_immovable_mem_regions()` (kaslr.c:396).
    fn count_immovable_mem_regions(&self) -> i32;
    /// The setup_data linked list, head first (kaslr.c:421-447).
    fn setup_data(&self) -> Vec<SetupDataNode>;
    /// Available-memory entries to scan. Production resolves these from KHO
    /// scratch / EFI memmap / e820 in the priority kaslr.c uses
    /// (kaslr.c:825-827); the seam hands back the resolved list.
    fn mem_regions(&self) -> Vec<MemRegionEntry>;
    /// `cmdline_find_option_bool("nokaslr")` (kaslr.c:869).
    fn nokaslr(&self) -> bool;
    /// `kaslr_get_random_long(purpose)` — entropy (kaslr.c:536,852).
    fn random_long(&self, purpose: &str) -> u64;
}

/// Production [`KaslrEnv`] backed by an injected entropy source. The
/// firmware-table accessors are filled in by the boot glue that owns the
/// real `boot_params`; this struct adapts `kaslr_get_random_long` for the
/// `random_long` seam so the slot math uses the real entropy mixer.
pub struct HardwareKaslrEnv<'a, E: KaslrEntropy> {
    pub entropy: &'a E,
    pub init_size: u64,
    pub initrd_start: u64,
    pub initrd_size: u64,
    pub cmd_line_ptr: u64,
    pub cmd_line: &'a [u8],
    pub boot_params_addr: u64,
    pub boot_params_size: u64,
    pub immovable: i32,
    pub setup_data: Vec<SetupDataNode>,
    pub mem_regions: Vec<MemRegionEntry>,
    pub nokaslr: bool,
}

impl<E: KaslrEntropy> KaslrEnv for HardwareKaslrEnv<'_, E> {
    fn init_size(&self) -> u64 {
        self.init_size
    }
    fn initrd_start(&self) -> u64 {
        self.initrd_start
    }
    fn initrd_size(&self) -> u64 {
        self.initrd_size
    }
    fn cmd_line_ptr(&self) -> u64 {
        self.cmd_line_ptr
    }
    fn cmd_line_len(&self) -> usize {
        use crate::arch::x86::boot::string::strnlen;
        if self.cmd_line_ptr == 0 {
            0
        } else {
            strnlen(self.cmd_line, COMMAND_LINE_SIZE - 1)
        }
    }
    fn cmd_line_bytes(&self) -> &[u8] {
        self.cmd_line
    }
    fn boot_params_addr(&self) -> u64 {
        self.boot_params_addr
    }
    fn boot_params_size(&self) -> u64 {
        self.boot_params_size
    }
    fn count_immovable_mem_regions(&self) -> i32 {
        self.immovable
    }
    fn setup_data(&self) -> Vec<SetupDataNode> {
        self.setup_data.clone()
    }
    fn mem_regions(&self) -> Vec<MemRegionEntry> {
        self.mem_regions.clone()
    }
    fn nokaslr(&self) -> bool {
        self.nokaslr
    }
    fn random_long(&self, _purpose: &str) -> u64 {
        kaslr_get_random_long(self.entropy)
    }
}

// ---------------------------------------------------------------------------
// KASLR mutable state. In C these are file-scope statics; we group them so
// the orchestration reads like the source and tests can inspect them.
// ---------------------------------------------------------------------------

/// File-scope state of kaslr.c (`mem_avoid[]`, `mem_limit`,
/// `memmap_too_large`, `num_immovable_mem`, `slot_areas[]`,
/// `slot_area_index`, `slot_max`).
pub struct Kaslr {
    /// `static struct mem_vector mem_avoid[MEM_AVOID_MAX]` (kaslr.c:96).
    pub mem_avoid: [MemVector; MEM_AVOID_MAX],
    /// `static u64 mem_limit` (kaslr.c:81).
    pub mem_limit: u64,
    /// `static bool memmap_too_large` (kaslr.c:74).
    pub memmap_too_large: bool,
    /// `static int num_immovable_mem` (kaslr.c:84).
    pub num_immovable_mem: i32,
    /// `static unsigned long max_gb_huge_pages` (kaslr.c:200).
    pub max_gb_huge_pages: u64,
    /// `static bool gbpage_sz` local to `parse_gb_huge_pages` (kaslr.c:204).
    gbpage_sz: bool,
    /// `static struct slot_area slot_areas[MAX_SLOT_AREA]` (kaslr.c:459).
    pub slot_areas: [SlotArea; MAX_SLOT_AREA],
    /// `static unsigned int slot_area_index` (kaslr.c:460).
    pub slot_area_index: usize,
    /// `static unsigned long slot_max` (kaslr.c:461).
    pub slot_max: u64,
    /// `static int i` in mem_avoid_memmap (kaslr.c:163) — number of memmap
    /// regions stored so far.
    memmap_i: usize,
}

impl Default for Kaslr {
    fn default() -> Self {
        Kaslr {
            mem_avoid: [MemVector::default(); MEM_AVOID_MAX],
            mem_limit: 0,
            memmap_too_large: false,
            num_immovable_mem: 0,
            max_gb_huge_pages: 0,
            gbpage_sz: false,
            slot_areas: [SlotArea::default(); MAX_SLOT_AREA],
            slot_area_index: 0,
            slot_max: 0,
            memmap_i: 0,
        }
    }
}

/// `mem_overlaps(one, two)` — kaslr.c:98-107. Half-open `[start, start+size)`.
pub fn mem_overlaps(one: &MemVector, two: &MemVector) -> bool {
    // Item one is entirely before item two.
    if one.start + one.size <= two.start {
        return false;
    }
    // Item one is entirely after item two.
    if one.start >= two.start + two.size {
        return false;
    }
    true
}

impl Kaslr {
    /// `parse_memmap(p, &start, &size)` — kaslr.c:118-159. Returns
    /// `Some((start, size))` on success (rc == 0), `None` on `-EINVAL`.
    fn parse_memmap(p: &[u8]) -> Option<(u64, u64)> {
        use crate::arch::x86::boot::string::strncmp;
        if p.is_empty() {
            return None;
        }
        // "exactmap": not handled here.
        if strncmp(p, b"exactmap", 8) == 0 {
            return None;
        }
        let (size, consumed) = memparse(p);
        if consumed == 0 {
            return None; // p == oldp -> -EINVAL
        }
        let rest = &p[consumed..];
        match rest.first().copied() {
            Some(b'#') | Some(b'$') | Some(b'!') => {
                let (start, _) = memparse(&rest[1..]);
                Some((start, size))
            }
            Some(b'@') => {
                // memmap=nn@ss specifies usable region -> size 0, start 0.
                let _ = size;
                Some((0, 0))
            }
            _ => {
                // Size-only: behaves like mem=, start 0.
                Some((0, size))
            }
        }
    }

    /// `mem_avoid_memmap(str)` — kaslr.c:161-197. Stores up to
    /// `MAX_MEMMAP_REGIONS` avoid entries; sets `memmap_too_large` if more.
    fn mem_avoid_memmap(&mut self, mut s: &[u8]) {
        if self.memmap_i >= MAX_MEMMAP_REGIONS {
            return;
        }
        while !s.is_empty() && self.memmap_i < MAX_MEMMAP_REGIONS {
            // char *k = strchr(str, ','); if (k) *k++ = 0;
            let (head, tail) = match s.iter().position(|&c| c == b',') {
                Some(pos) => (&s[..pos], Some(&s[pos + 1..])),
                None => (s, None),
            };

            let parsed = Self::parse_memmap(head);
            let (start, size) = match parsed {
                Some(v) => v,
                None => break, // rc < 0
            };
            s = tail.unwrap_or(&[]);

            if start == 0 {
                // Store the specified memory limit if size > 0.
                if size > 0 && size < self.mem_limit {
                    self.mem_limit = size;
                }
                continue;
            }

            self.mem_avoid[MEM_AVOID_MEMMAP_BEGIN + self.memmap_i].start = start;
            self.mem_avoid[MEM_AVOID_MEMMAP_BEGIN + self.memmap_i].size = size;
            self.memmap_i += 1;

            if tail.is_none() {
                break;
            }
        }

        // More than MAX_MEMMAP_REGIONS memmaps -> fail kaslr.
        if self.memmap_i >= MAX_MEMMAP_REGIONS && !s.is_empty() {
            self.memmap_too_large = true;
        }
    }

    /// `handle_mem_options()` — kaslr.c:227-276. Parses `memmap=` and
    /// `mem=` from the cmdline and updates `mem_avoid`/`mem_limit`.
    /// (hugepages handling lives in mem_avoid_init via parse_gb_huge_pages;
    /// kept here as in the C since it shares the arg loop.)
    fn handle_mem_options(&mut self, env: &dyn KaslrEnv) {
        let args = env.cmd_line_bytes();
        if args.is_empty() || env.cmd_line_ptr() == 0 {
            return;
        }
        use crate::arch::x86::boot::string::strnlen;
        let len = strnlen(args, COMMAND_LINE_SIZE - 1);
        let mut rest = skip_spaces(&args[..len]);

        while !rest.is_empty() {
            let (param, val, after) = next_arg(rest);
            rest = after;

            // Stop at "--".
            if val.is_none() && param == b"--" {
                break;
            }

            if param == b"memmap" {
                if let Some(v) = val {
                    self.mem_avoid_memmap(v);
                }
            } else if contains(param, b"hugepages") {
                // CONFIG_X86_64 && strstr(param, "hugepages")
                if let Some(v) = val {
                    self.parse_gb_huge_pages(param, v);
                }
            } else if param == b"mem" {
                if let Some(v) = val {
                    if v == b"nopentium" {
                        continue;
                    }
                    let (mem_size, consumed) = memparse(v);
                    if consumed == 0 || mem_size == 0 {
                        break;
                    }
                    if mem_size < self.mem_limit {
                        self.mem_limit = mem_size;
                    }
                }
            }
        }
    }

    /// `parse_gb_huge_pages(param, val)` — kaslr.c:202-225.
    fn parse_gb_huge_pages(&mut self, param: &[u8], val: &[u8]) {
        use crate::arch::x86::boot::string::{simple_strtoull, strcmp};
        // hugepagesz=...: set gbpage_sz if the size equals PUD_SIZE.
        if strcmp(param, b"hugepagesz") == 0 {
            let (sz, _) = memparse(val);
            if sz != PUD_SIZE {
                self.gbpage_sz = false;
                return;
            }
            // (warn on repeated set is cosmetic; semantics: set the flag)
            self.gbpage_sz = true;
            return;
        }
        if strcmp(param, b"hugepages") == 0 && self.gbpage_sz {
            let (n, _) = simple_strtoull(val, 0);
            self.max_gb_huge_pages = n;
        }
    }

    /// `mem_avoid_init(input, input_size, output)` — kaslr.c:354-397.
    /// Records the ZO range, initrd, cmdline, boot_params, the memmap/mem
    /// restrictions, and counts immovable regions.
    pub fn mem_avoid_init(
        &mut self,
        env: &dyn KaslrEnv,
        input: u64,
        _input_size: u64,
        output: u64,
    ) {
        let init_size = env.init_size();

        // ZO range: [input, output + init_size).
        self.mem_avoid[MEM_AVOID_ZO_RANGE].start = input;
        self.mem_avoid[MEM_AVOID_ZO_RANGE].size = (output + init_size) - input;

        // initrd.
        self.mem_avoid[MEM_AVOID_INITRD].start = env.initrd_start();
        self.mem_avoid[MEM_AVOID_INITRD].size = env.initrd_size();

        // cmdline.
        let cmd_line = env.cmd_line_ptr();
        if cmd_line != 0 {
            let cmd_line_size = env.cmd_line_len() as u64 + 1;
            self.mem_avoid[MEM_AVOID_CMDLINE].start = cmd_line;
            self.mem_avoid[MEM_AVOID_CMDLINE].size = cmd_line_size;
        }

        // boot_params.
        self.mem_avoid[MEM_AVOID_BOOTPARAMS].start = env.boot_params_addr();
        self.mem_avoid[MEM_AVOID_BOOTPARAMS].size = env.boot_params_size();

        // memmap=/mem= restrictions.
        self.handle_mem_options(env);

        // immovable regions.
        self.num_immovable_mem = env.count_immovable_mem_regions();
    }

    /// `mem_avoid_overlap(img, &overlap)` — kaslr.c:403-450. Returns
    /// `Some(overlap)` describing the lowest-addressed avoided range that
    /// `img` overlaps, or `None`.
    pub fn mem_avoid_overlap(&self, env: &dyn KaslrEnv, img: &MemVector) -> Option<MemVector> {
        let mut earliest = img.start + img.size;
        let mut overlap: Option<MemVector> = None;

        for i in 0..MEM_AVOID_MAX {
            if mem_overlaps(img, &self.mem_avoid[i]) && self.mem_avoid[i].start < earliest {
                overlap = Some(self.mem_avoid[i]);
                earliest = self.mem_avoid[i].start;
            }
        }

        // Avoid all entries in the setup_data linked list.
        for sd in env.setup_data() {
            let avoid = MemVector {
                start: sd.addr,
                // sizeof(*ptr) + ptr->len; sizeof(struct setup_data) == 16.
                size: 16 + sd.len as u64,
            };
            if mem_overlaps(img, &avoid) && avoid.start < earliest {
                overlap = Some(avoid);
                earliest = avoid.start;
            }

            // SETUP_INDIRECT inner payload.
            if sd.type_ == SETUP_INDIRECT {
                if let Some((iaddr, ilen)) = sd.indirect {
                    let avoid = MemVector {
                        start: iaddr,
                        size: ilen,
                    };
                    if mem_overlaps(img, &avoid) && avoid.start < earliest {
                        overlap = Some(avoid);
                        earliest = avoid.start;
                    }
                }
            }
        }

        overlap
    }

    /// `store_slot_info(region, image_size)` — kaslr.c:463-475.
    fn store_slot_info(&mut self, region: &MemVector, image_size: u64) {
        if self.slot_area_index == MAX_SLOT_AREA {
            return;
        }
        let slot_area = SlotArea {
            addr: region.start,
            num: 1 + (region.size - image_size) / CONFIG_PHYSICAL_ALIGN,
        };
        self.slot_areas[self.slot_area_index] = slot_area;
        self.slot_area_index += 1;
        self.slot_max += slot_area.num;
    }

    /// `process_gb_huge_pages(region, image_size)` — kaslr.c:481-525.
    fn process_gb_huge_pages(&mut self, region: &MemVector, image_size: u64) {
        // !IS_ENABLED(CONFIG_X86_64) is false here; gate only on huge pages.
        if self.max_gb_huge_pages == 0 {
            self.store_slot_info(region, image_size);
            return;
        }

        let pud_start = align_up(region.start, PUD_SIZE);
        let mut pud_end = align_down(region.start + region.size, PUD_SIZE);

        if pud_start >= pud_end {
            self.store_slot_info(region, image_size);
            return;
        }

        // Head part usable?
        if pud_start >= region.start + image_size {
            let tmp = MemVector {
                start: region.start,
                size: pud_start - region.start,
            };
            self.store_slot_info(&tmp, image_size);
        }

        // Skip the good 1GB pages.
        let gb_huge_pages = (pud_end - pud_start) >> PUD_SHIFT;
        if gb_huge_pages > self.max_gb_huge_pages {
            pud_end = pud_start + (self.max_gb_huge_pages << PUD_SHIFT);
            self.max_gb_huge_pages = 0;
        } else {
            self.max_gb_huge_pages -= gb_huge_pages;
        }

        // Tail part usable?
        if region.start + region.size >= pud_end + image_size {
            let tmp = MemVector {
                start: pud_end,
                size: region.start + region.size - pud_end,
            };
            self.store_slot_info(&tmp, image_size);
        }
    }

    /// `slots_fetch_random()` — kaslr.c:527-549. Picks a physical slot
    /// address using the entropy seam.
    pub fn slots_fetch_random(&self, env: &dyn KaslrEnv) -> u64 {
        if self.slot_max == 0 {
            return 0;
        }
        let mut slot = env.random_long("Physical") % self.slot_max;
        for i in 0..self.slot_area_index {
            if slot >= self.slot_areas[i].num {
                slot -= self.slot_areas[i].num;
                continue;
            }
            return self.slot_areas[i].addr + slot * CONFIG_PHYSICAL_ALIGN;
        }
        0
    }

    /// `__process_mem_region(entry, minimum, image_size)` — kaslr.c:551-593.
    fn process_one_mem_region(
        &mut self,
        env: &dyn KaslrEnv,
        entry: &MemVector,
        minimum: u64,
        image_size: u64,
    ) {
        let mut region = MemVector::default();
        // region.start = max(entry->start, minimum)
        region.start = core::cmp::max(entry.start, minimum);
        // region_end = min(entry->start + entry->size, mem_limit)
        let region_end = core::cmp::min(entry.start + entry.size, self.mem_limit);

        while self.slot_area_index < MAX_SLOT_AREA {
            region.start = align_up(region.start, CONFIG_PHYSICAL_ALIGN);

            if region.start > region_end {
                return;
            }

            region.size = region_end - region.start;

            if region.size < image_size {
                return;
            }

            match self.mem_avoid_overlap(env, &region) {
                None => {
                    self.process_gb_huge_pages(&region, image_size);
                    return;
                }
                Some(overlap) => {
                    // Store beginning of region if it holds image_size.
                    if overlap.start >= region.start + image_size {
                        let head = MemVector {
                            start: region.start,
                            size: overlap.start - region.start,
                        };
                        self.process_gb_huge_pages(&head, image_size);
                    }
                    // Clip off the overlapping region and start over.
                    region.start = overlap.start + overlap.size;
                }
            }
        }
    }

    /// `process_mem_region(region, minimum, image_size)` — kaslr.c:595-643.
    /// Returns true if the scan should abort (slot_areas full).
    pub fn process_mem_region(
        &mut self,
        env: &dyn KaslrEnv,
        region: &MemVector,
        minimum: u64,
        image_size: u64,
    ) -> bool {
        // No immovable memory / MEMORY_HOTREMOVE disabled -> use region.
        if self.num_immovable_mem == 0 {
            self.process_one_mem_region(env, region, minimum, image_size);
            if self.slot_area_index == MAX_SLOT_AREA {
                return true;
            }
            return false;
        }
        // CONFIG_MEMORY_HOTREMOVE && CONFIG_ACPI path: lupos does not yet
        // expose the immovable_mem[] table through the seam, so with
        // num_immovable_mem != 0 we fall through to the same per-region
        // processing. This keeps the abort/return semantics intact.
        self.process_one_mem_region(env, region, minimum, image_size);
        self.slot_area_index == MAX_SLOT_AREA
    }

    /// `find_random_phys_addr(minimum, image_size)` — kaslr.c:806-838.
    pub fn find_random_phys_addr(
        &mut self,
        env: &dyn KaslrEnv,
        minimum: u64,
        image_size: u64,
    ) -> u64 {
        // Bail out early if impossible.
        if minimum + image_size > self.mem_limit {
            return 0;
        }
        // Too many memmaps?
        if self.memmap_too_large {
            return 0;
        }

        // Walk the resolved available-memory entries (KHO/EFI/e820 priority
        // is applied by the seam, kaslr.c:825-827).
        for r in env.mem_regions() {
            let region = MemVector {
                start: r.start,
                size: r.size,
            };
            if self.process_mem_region(env, &region, minimum, image_size) {
                break;
            }
        }

        let phys_addr = self.slots_fetch_random(env);

        // Final range check.
        if phys_addr < minimum || phys_addr + image_size > self.mem_limit {
            return 0;
        }
        phys_addr
    }

    /// `find_random_virt_addr(minimum, image_size)` — kaslr.c:840-855.
    pub fn find_random_virt_addr(&self, env: &dyn KaslrEnv, minimum: u64, image_size: u64) -> u64 {
        let slots = 1 + (KERNEL_IMAGE_SIZE - minimum - image_size) / CONFIG_PHYSICAL_ALIGN;
        let random_addr = env.random_long("Virtual") % slots;
        random_addr * CONFIG_PHYSICAL_ALIGN + minimum
    }
}

/// Result of `choose_random_location` — the updated `(output, virt_addr)`
/// pair plus whether KASLR ran (false = disabled / `nokaslr`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ChosenLocation {
    pub output: u64,
    pub virt_addr: u64,
    pub randomized: bool,
}

/// `choose_random_location(input, input_size, output, output_size,
/// virt_addr)` — kaslr.c:861-908. `output`/`virt_addr` are in/out in C; we
/// take their initial values and return the chosen pair.
pub fn choose_random_location(
    state: &mut Kaslr,
    env: &dyn KaslrEnv,
    input: u64,
    input_size: u64,
    output: u64,
    output_size: u64,
    virt_addr: u64,
) -> ChosenLocation {
    let mut out = output;
    let mut virt = virt_addr;

    if env.nokaslr() {
        // KASLR disabled: 'nokaslr' on cmdline.
        return ChosenLocation {
            output: out,
            virt_addr: virt,
            randomized: false,
        };
    }

    // boot_params_ptr->hdr.loadflags |= KASLR_FLAG; (side effect in C;
    // performed by the caller that owns boot_params).

    // CONFIG_X86_64: mem_limit = MAXMEM.
    state.mem_limit = MAXMEM;

    // Record unsafe ranges.
    state.mem_avoid_init(env, input, input_size, out);

    // min_addr = min(output, 512M), aligned.
    let mut min_addr = core::cmp::min(out, 512u64 << 20);
    min_addr = align_up(min_addr, CONFIG_PHYSICAL_ALIGN);

    // Physical address.
    let random_addr = state.find_random_phys_addr(env, min_addr, output_size);
    if random_addr == 0 {
        // Physical KASLR disabled: no suitable memory region.
    } else if out != random_addr {
        out = random_addr;
    }

    // Virtual address (CONFIG_X86_64).
    virt = state.find_random_virt_addr(env, LOAD_PHYSICAL_ADDR, output_size);

    ChosenLocation {
        output: out,
        virt_addr: virt,
        randomized: true,
    }
}

/// `next_arg(args, &param, &val)` — lib/cmdline.c. Splits the next
/// whitespace-delimited argument into `(param, Some(val))` for
/// `param=val` or `(param, None)` for a bare flag, returning the remaining
/// tail. Handles double-quoted values. kaslr.c reaches this via the
/// included lib/cmdline.c (kaslr.c:251).
fn next_arg(args: &[u8]) -> (&[u8], Option<&[u8]>, &[u8]) {
    let mut i = 0;
    // Skip leading spaces.
    while i < args.len() && isspace(args[i]) {
        i += 1;
    }
    let s = &args[i..];
    if s.is_empty() {
        return (&[], None, &[]);
    }

    // A leading quote means the whole token (incl. '=') is the param.
    let quoted = s[0] == b'"';
    let mut j = 0;
    let mut eq: Option<usize> = None;
    let bytes = if quoted { &s[1..] } else { s };

    while j < bytes.len() {
        let c = bytes[j];
        if quoted {
            if c == b'"' {
                break;
            }
        } else {
            if isspace(c) {
                break;
            }
            if c == b'=' && eq.is_none() {
                eq = Some(j);
            }
        }
        j += 1;
    }

    if quoted {
        // Quoted param: token is bytes[..j], skip the closing quote.
        let param = &bytes[..j];
        let consumed_in_s = 1 + j + if j < s.len().saturating_sub(1) { 1 } else { 0 };
        let tail = if i + consumed_in_s <= args.len() {
            &args[i + consumed_in_s..]
        } else {
            &[]
        };
        return (param, None, tail);
    }

    let token = &bytes[..j];
    let tail = &s[j..];
    match eq {
        Some(pos) => {
            let param = &token[..pos];
            let mut val = &token[pos + 1..];
            // A value may itself be quoted: strip a leading/trailing quote.
            if val.first() == Some(&b'"') {
                val = &val[1..];
                if val.last() == Some(&b'"') {
                    val = &val[..val.len() - 1];
                }
            }
            (param, Some(val), tail)
        }
        None => (token, None, tail),
    }
}

/// `strstr(param, "hugepages")` reduced to a substring test.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    struct TestEnv {
        init_size: u64,
        initrd_start: u64,
        initrd_size: u64,
        cmd_line_ptr: u64,
        cmd_line: Vec<u8>,
        boot_params_addr: u64,
        boot_params_size: u64,
        immovable: i32,
        setup_data: Vec<SetupDataNode>,
        mem_regions: Vec<MemRegionEntry>,
        nokaslr: bool,
        random: u64,
    }

    impl Default for TestEnv {
        fn default() -> Self {
            TestEnv {
                init_size: 0,
                initrd_start: 0,
                initrd_size: 0,
                cmd_line_ptr: 0,
                cmd_line: Vec::new(),
                boot_params_addr: 0,
                boot_params_size: 0,
                immovable: 0,
                setup_data: Vec::new(),
                mem_regions: Vec::new(),
                nokaslr: false,
                random: 0,
            }
        }
    }

    impl KaslrEnv for TestEnv {
        fn init_size(&self) -> u64 {
            self.init_size
        }
        fn initrd_start(&self) -> u64 {
            self.initrd_start
        }
        fn initrd_size(&self) -> u64 {
            self.initrd_size
        }
        fn cmd_line_ptr(&self) -> u64 {
            self.cmd_line_ptr
        }
        fn cmd_line_len(&self) -> usize {
            use crate::arch::x86::boot::string::strnlen;
            if self.cmd_line_ptr == 0 {
                0
            } else {
                strnlen(&self.cmd_line, COMMAND_LINE_SIZE - 1)
            }
        }
        fn cmd_line_bytes(&self) -> &[u8] {
            &self.cmd_line
        }
        fn boot_params_addr(&self) -> u64 {
            self.boot_params_addr
        }
        fn boot_params_size(&self) -> u64 {
            self.boot_params_size
        }
        fn count_immovable_mem_regions(&self) -> i32 {
            self.immovable
        }
        fn setup_data(&self) -> Vec<SetupDataNode> {
            self.setup_data.clone()
        }
        fn mem_regions(&self) -> Vec<MemRegionEntry> {
            self.mem_regions.clone()
        }
        fn nokaslr(&self) -> bool {
            self.nokaslr
        }
        fn random_long(&self, _purpose: &str) -> u64 {
            self.random
        }
    }

    // ---- constants: the fix for the lying MEM_AVOID_MAX = 4 -----------

    #[test]
    fn mem_avoid_enum_matches_kaslr_c_layout() {
        // kaslr.c:86-94. The MEMMAP block runs [4, 7]; MAX is 8.
        assert_eq!(MEM_AVOID_ZO_RANGE, 0);
        assert_eq!(MEM_AVOID_INITRD, 1);
        assert_eq!(MEM_AVOID_CMDLINE, 2);
        assert_eq!(MEM_AVOID_BOOTPARAMS, 3);
        assert_eq!(MEM_AVOID_MEMMAP_BEGIN, 4);
        assert_eq!(MEM_AVOID_MEMMAP_END, 7);
        // The headline fix: NOT 4.
        assert_eq!(MEM_AVOID_MAX, 8);
        assert_eq!(MAX_MEMMAP_REGIONS, 4);
        assert_eq!(MEM_AVOID_MAX, MEM_AVOID_MEMMAP_BEGIN + MAX_MEMMAP_REGIONS);
        // The mem_avoid table is sized by MEM_AVOID_MAX.
        let k = Kaslr::default();
        assert_eq!(k.mem_avoid.len(), 8);
    }

    #[test]
    fn arch_constants_match_headers() {
        assert_eq!(CONFIG_PHYSICAL_ALIGN, 0x20_0000);
        assert_eq!(LOAD_PHYSICAL_ADDR, 0x100_0000);
        assert_eq!(KERNEL_IMAGE_SIZE, 0x4000_0000);
        assert_eq!(MAXMEM, 1u64 << 46);
        assert_eq!(PUD_SIZE, 0x4000_0000);
        assert_eq!(E820_TYPE_RAM, 1);
        assert_eq!(MAX_SLOT_AREA, 100);
        assert_eq!(SETUP_INDIRECT, 0x8000_0000);
    }

    #[test]
    fn mem_vector_is_start_plus_size_struct() {
        // struct mem_vector { u64 start; u64 size; } — 16 bytes.
        assert_eq!(core::mem::size_of::<MemVector>(), 16);
    }

    // ---- mem_overlaps -------------------------------------------------

    #[test]
    fn mem_overlaps_uses_half_open_start_plus_size() {
        // one=[0x1000,0x1000) i.e. [0x1000,0x2000); two adjacent above.
        let one = MemVector {
            start: 0x1000,
            size: 0x1000,
        };
        // two starts exactly at one's end -> no overlap.
        assert!(!mem_overlaps(
            &one,
            &MemVector {
                start: 0x2000,
                size: 0x10
            }
        ));
        // two ends exactly at one's start -> no overlap.
        assert!(!mem_overlaps(
            &one,
            &MemVector {
                start: 0x0,
                size: 0x1000
            }
        ));
        // genuine overlap.
        assert!(mem_overlaps(
            &one,
            &MemVector {
                start: 0x1800,
                size: 0x10
            }
        ));
    }

    // ---- mem_avoid_overlap -------------------------------------------

    #[test]
    fn mem_avoid_overlap_reports_lowest_avoided_range() {
        let mut k = Kaslr::default();
        k.mem_avoid[MEM_AVOID_BOOTPARAMS] = MemVector {
            start: 0x5000,
            size: 0x1000,
        };
        k.mem_avoid[MEM_AVOID_ZO_RANGE] = MemVector {
            start: 0x2000,
            size: 0x1000,
        };
        let env = TestEnv::default();
        // img spans both avoid ranges; the lowest (0x2000) is reported.
        let img = MemVector {
            start: 0x0,
            size: 0x8000,
        };
        let overlap = k.mem_avoid_overlap(&env, &img).unwrap();
        assert_eq!(overlap.start, 0x2000);
    }

    #[test]
    fn mem_avoid_overlap_includes_setup_data_nodes() {
        let k = Kaslr::default();
        let mut env = TestEnv::default();
        env.setup_data = vec![SetupDataNode {
            addr: 0x3000,
            len: 0x40,
            type_: 0,
            next: 0,
            indirect: None,
        }];
        let img = MemVector {
            start: 0x2000,
            size: 0x4000,
        };
        let overlap = k.mem_avoid_overlap(&env, &img).unwrap();
        // sizeof(setup_data)=16 + len=0x40 -> [0x3000, 0x3050).
        assert_eq!(overlap.start, 0x3000);
        assert_eq!(overlap.size, 16 + 0x40);
    }

    #[test]
    fn mem_avoid_overlap_none_when_disjoint() {
        let mut k = Kaslr::default();
        k.mem_avoid[MEM_AVOID_INITRD] = MemVector {
            start: 0x10_0000,
            size: 0x1000,
        };
        let env = TestEnv::default();
        let img = MemVector {
            start: 0x0,
            size: 0x1000,
        };
        assert!(k.mem_avoid_overlap(&env, &img).is_none());
    }

    // ---- mem_avoid_init ----------------------------------------------

    #[test]
    fn mem_avoid_init_populates_zo_initrd_cmdline_bootparams() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        let mut env = TestEnv::default();
        env.init_size = 0x80_0000;
        env.initrd_start = 0x4000_0000;
        env.initrd_size = 0x10_0000;
        env.cmd_line_ptr = 0x9_0000;
        env.cmd_line = b"quiet console=ttyS0\0".to_vec();
        env.boot_params_addr = 0x8_0000;
        env.boot_params_size = 0x1000;

        let input = 0x100_0000u64;
        let output = 0x100_0000u64;
        k.mem_avoid_init(&env, input, 0x40_0000, output);

        // ZO: [input, output+init_size).
        assert_eq!(k.mem_avoid[MEM_AVOID_ZO_RANGE].start, input);
        assert_eq!(
            k.mem_avoid[MEM_AVOID_ZO_RANGE].size,
            (output + env.init_size) - input
        );
        // initrd.
        assert_eq!(k.mem_avoid[MEM_AVOID_INITRD].start, 0x4000_0000);
        assert_eq!(k.mem_avoid[MEM_AVOID_INITRD].size, 0x10_0000);
        // cmdline: size is strnlen+1.
        assert_eq!(k.mem_avoid[MEM_AVOID_CMDLINE].start, 0x9_0000);
        assert_eq!(
            k.mem_avoid[MEM_AVOID_CMDLINE].size,
            (b"quiet console=ttyS0".len() + 1) as u64
        );
        // boot_params.
        assert_eq!(k.mem_avoid[MEM_AVOID_BOOTPARAMS].start, 0x8_0000);
        assert_eq!(k.mem_avoid[MEM_AVOID_BOOTPARAMS].size, 0x1000);
    }

    // ---- handle_mem_options / memmap= --------------------------------

    #[test]
    fn handle_mem_options_records_memmap_avoid_region() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        let mut env = TestEnv::default();
        env.cmd_line_ptr = 0x9_0000;
        // memmap=1M$0x10000000 -> avoid [0x10000000, +1M).
        env.cmd_line = b"memmap=1M$0x10000000".to_vec();
        k.handle_mem_options(&env);
        assert_eq!(k.mem_avoid[MEM_AVOID_MEMMAP_BEGIN].start, 0x1000_0000);
        assert_eq!(k.mem_avoid[MEM_AVOID_MEMMAP_BEGIN].size, 0x10_0000);
    }

    #[test]
    fn handle_mem_options_mem_limits_mem_limit() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        let mut env = TestEnv::default();
        env.cmd_line_ptr = 0x9_0000;
        env.cmd_line = b"mem=512M".to_vec();
        k.handle_mem_options(&env);
        assert_eq!(k.mem_limit, 512u64 << 20);
    }

    #[test]
    fn memparse_parses_kmg_suffixes() {
        assert_eq!(memparse(b"1K"), (1024, 2));
        assert_eq!(memparse(b"2M"), (2 * 1024 * 1024, 2));
        assert_eq!(memparse(b"1G"), (1024 * 1024 * 1024, 2));
        assert_eq!(memparse(b"0x1000"), (0x1000, 6));
    }

    // ---- store_slot_info / slots_fetch_random ------------------------

    #[test]
    fn store_slot_info_counts_physical_align_slots() {
        let mut k = Kaslr::default();
        // region of 8*ALIGN, image = ALIGN -> num = 1 + (8A-A)/A = 8.
        let region = MemVector {
            start: 0x100_0000,
            size: 8 * CONFIG_PHYSICAL_ALIGN,
        };
        k.store_slot_info(&region, CONFIG_PHYSICAL_ALIGN);
        assert_eq!(k.slot_area_index, 1);
        assert_eq!(k.slot_areas[0].num, 8);
        assert_eq!(k.slot_max, 8);
    }

    #[test]
    fn slots_fetch_random_indexes_into_stored_areas() {
        let mut k = Kaslr::default();
        let region = MemVector {
            start: 0x100_0000,
            size: 4 * CONFIG_PHYSICAL_ALIGN,
        };
        k.store_slot_info(&region, CONFIG_PHYSICAL_ALIGN); // num = 4
        let mut env = TestEnv::default();
        // random % slot_max(4) == 2 -> addr + 2*ALIGN.
        env.random = 2;
        let chosen = k.slots_fetch_random(&env);
        assert_eq!(chosen, 0x100_0000 + 2 * CONFIG_PHYSICAL_ALIGN);
    }

    #[test]
    fn slots_fetch_random_zero_when_no_slots() {
        let k = Kaslr::default();
        let env = TestEnv::default();
        assert_eq!(k.slots_fetch_random(&env), 0);
    }

    // ---- process_mem_region / __process_mem_region -------------------

    #[test]
    fn process_mem_region_stores_clean_region() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        let region = MemVector {
            start: 0x100_0000,
            size: 8 * CONFIG_PHYSICAL_ALIGN,
        };
        let abort = k.process_mem_region(&TestEnv::default(), &region, 0, CONFIG_PHYSICAL_ALIGN);
        assert!(!abort);
        assert_eq!(k.slot_area_index, 1);
        // 1 + (8A - A)/A = 8 slots.
        assert_eq!(k.slot_max, 8);
    }

    #[test]
    fn process_mem_region_clips_around_avoid_overlap() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        // Avoid a 1-ALIGN hole in the middle of the region.
        k.mem_avoid[MEM_AVOID_ZO_RANGE] = MemVector {
            start: 0x100_0000 + 2 * CONFIG_PHYSICAL_ALIGN,
            size: CONFIG_PHYSICAL_ALIGN,
        };
        let region = MemVector {
            start: 0x100_0000,
            size: 8 * CONFIG_PHYSICAL_ALIGN,
        };
        k.process_mem_region(&TestEnv::default(), &region, 0, CONFIG_PHYSICAL_ALIGN);
        // Head [0, 2A) stores 1 + (2A-A)/A = 2 slots; tail [3A, 8A) stores
        // 1 + (5A-A)/A = 5 slots. Total 7, splitting into two areas.
        assert_eq!(k.slot_area_index, 2);
        assert_eq!(k.slot_max, 2 + 5);
    }

    // ---- find_random_phys_addr / find_random_virt_addr ---------------

    #[test]
    fn find_random_phys_addr_bails_when_minimum_above_limit() {
        let mut k = Kaslr::default();
        k.mem_limit = 0x100_0000;
        let env = TestEnv::default();
        assert_eq!(
            k.find_random_phys_addr(&env, 0x200_0000, CONFIG_PHYSICAL_ALIGN),
            0
        );
    }

    #[test]
    fn find_random_phys_addr_walks_regions_and_picks_slot() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        let mut env = TestEnv::default();
        env.mem_regions = vec![MemRegionEntry {
            start: 0x100_0000,
            size: 4 * CONFIG_PHYSICAL_ALIGN,
        }];
        env.random = 1; // pick slot 1
        let addr = k.find_random_phys_addr(&env, 0, CONFIG_PHYSICAL_ALIGN);
        assert_eq!(addr, 0x100_0000 + CONFIG_PHYSICAL_ALIGN);
    }

    #[test]
    fn find_random_phys_addr_zero_when_too_many_memmaps() {
        let mut k = Kaslr::default();
        k.mem_limit = MAXMEM;
        k.memmap_too_large = true;
        let env = TestEnv::default();
        assert_eq!(k.find_random_phys_addr(&env, 0, CONFIG_PHYSICAL_ALIGN), 0);
    }

    #[test]
    fn find_random_virt_addr_is_minimum_plus_aligned_slot() {
        let k = Kaslr::default();
        let mut env = TestEnv::default();
        // slots = 1 + (KERNEL_IMAGE_SIZE - min - image)/ALIGN.
        env.random = 0; // slot 0 -> exactly minimum
        let v = k.find_random_virt_addr(&env, LOAD_PHYSICAL_ADDR, CONFIG_PHYSICAL_ALIGN);
        assert_eq!(v, LOAD_PHYSICAL_ADDR);
        // random producing slot 3 -> minimum + 3*ALIGN.
        env.random = 3;
        let v = k.find_random_virt_addr(&env, LOAD_PHYSICAL_ADDR, CONFIG_PHYSICAL_ALIGN);
        assert_eq!(v, LOAD_PHYSICAL_ADDR + 3 * CONFIG_PHYSICAL_ALIGN);
    }

    // ---- choose_random_location --------------------------------------

    #[test]
    fn choose_random_location_disabled_by_nokaslr() {
        let mut k = Kaslr::default();
        let mut env = TestEnv::default();
        env.nokaslr = true;
        let r = choose_random_location(
            &mut k,
            &env,
            0x100_0000,
            0x40_0000,
            0x100_0000,
            0x40_0000,
            LOAD_PHYSICAL_ADDR,
        );
        assert!(!r.randomized);
        assert_eq!(r.output, 0x100_0000);
        assert_eq!(r.virt_addr, LOAD_PHYSICAL_ADDR);
    }

    #[test]
    fn choose_random_location_sets_mem_limit_and_picks_addresses() {
        let mut k = Kaslr::default();
        let mut env = TestEnv::default();
        env.init_size = 0x80_0000;
        env.boot_params_addr = 0x8_0000;
        env.boot_params_size = 0x1000;
        env.mem_regions = vec![MemRegionEntry {
            start: 0x100_0000,
            size: 16 * CONFIG_PHYSICAL_ALIGN,
        }];
        env.random = 0;
        let r = choose_random_location(
            &mut k,
            &env,
            0x100_0000,
            0x40_0000,
            0x100_0000,
            0x40_0000,
            LOAD_PHYSICAL_ADDR,
        );
        assert!(r.randomized);
        // mem_limit was set to MAXMEM during the call.
        assert_eq!(k.mem_limit, MAXMEM);
        // Virtual address is within the kernel image window.
        assert!(r.virt_addr >= LOAD_PHYSICAL_ADDR);
    }
}
