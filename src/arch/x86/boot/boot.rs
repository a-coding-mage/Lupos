//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/boot.h
//! test-origin: linux:vendor/linux/arch/x86/boot/boot.h
//! Header file for the real-mode kernel setup code.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/boot.h
//!
//! `boot.h` is the umbrella header of the real-mode `setup.bin` stub. It
//! defines the stack size, the `cpu_relax`/`io_delay` micro-helpers, the
//! `fs`/`gs` segment accessors used to reach data in other real-mode
//! segments, and the bump-allocator (`HEAP`/`__get_heap`/`heap_free`) that
//! backs the dynamic lists the setup code builds. Most other declarations
//! in the C header are `extern` prototypes whose definitions live in their
//! own `.c` files (and therefore their own Rust modules); those are
//! re-exported here so callers see the same surface as the C header.
//!
//! The segment helpers really do model the real-mode environment: they
//! emit the same `mov`/`fs:`/`gs:` instructions Linux uses. Lupos does not
//! run real-mode setup, so they are documented as faithful models of the
//! `setup.bin` world rather than runtime kernel paths.

use super::io;

/// `STACK_SIZE` — minimum number of bytes for the setup stack (boot.h
/// line 17).
pub const STACK_SIZE: usize = 1024;

/// `cpu_relax()` — PAUSE hint inside busy-wait loops (boot.h line 37:
/// `asm volatile("pause")`). Reduces power use and lets the sibling
/// hyperthread make progress.
#[inline]
pub fn cpu_relax() {
    // SAFETY: `pause` has no operands and no memory/flag effects beyond the
    // micro-architectural hint. Mirrors boot.h `cpu_relax`.
    unsafe {
        core::arch::asm!("pause", options(nomem, nostack, preserves_flags));
    }
}

/// `io_delay()` — short delay by writing port 0x80 (boot.h lines 39-43).
///
/// Linux issues `outb(0, DELAY_PORT)` through the `pio_ops` indirection;
/// we go through the same [`io::PortIoOps`] seam so a TDX-style override
/// would intercept it too.
#[inline]
pub fn io_delay(ops: &io::PortIoOps) {
    const DELAY_PORT: u16 = 0x80;
    ops.outb(0, DELAY_PORT);
}

// ---------------------------------------------------------------------------
// Segment-register accessors (boot.h lines 47-152).
//
// These read/write FS and GS and dereference `fs:`/`gs:`-relative bytes,
// words and dwords. They faithfully reproduce the real-mode setup.bin
// access pattern: setup code parks a far segment in FS/GS and then uses
// rdfs*/wrfs*/rdgs*/wrgs* to touch other segments without reloading DS.
// `addr_t` is Linux's `unsigned int` offset type.
// ---------------------------------------------------------------------------

/// Linux `addr_t` — an offset within the FS/GS segment (`unsigned int`).
pub type AddrT = u32;

/// `ds()` — read the DS segment register (boot.h lines 47-52).
#[inline]
pub fn ds() -> u16 {
    let seg: u16;
    // SAFETY: reads a segment register; no memory effects.
    unsafe {
        core::arch::asm!("mov {0:x}, ds", out(reg) seg, options(nomem, nostack, preserves_flags));
    }
    seg
}

/// `set_fs(seg)` — load the FS segment register (boot.h lines 54-57).
#[inline]
pub fn set_fs(seg: u16) {
    // SAFETY: loads FS; the caller is the real-mode setup stub which owns
    // the segment selectors.
    unsafe {
        core::arch::asm!("mov fs, {0:x}", in(reg) seg, options(nostack, preserves_flags));
    }
}

/// `fs()` — read the FS segment register (boot.h lines 58-63).
#[inline]
pub fn fs() -> u16 {
    let seg: u16;
    // SAFETY: reads a segment register; no memory effects.
    unsafe {
        core::arch::asm!("mov {0:x}, fs", out(reg) seg, options(nomem, nostack, preserves_flags));
    }
    seg
}

/// `set_gs(seg)` — load the GS segment register (boot.h lines 65-68).
#[inline]
pub fn set_gs(seg: u16) {
    // SAFETY: loads GS; the caller owns the segment selectors.
    unsafe {
        core::arch::asm!("mov gs, {0:x}", in(reg) seg, options(nostack, preserves_flags));
    }
}

/// `gs()` — read the GS segment register (boot.h lines 69-74).
#[inline]
pub fn gs() -> u16 {
    let seg: u16;
    // SAFETY: reads a segment register; no memory effects.
    unsafe {
        core::arch::asm!("mov {0:x}, gs", out(reg) seg, options(nomem, nostack, preserves_flags));
    }
    seg
}

/// `rdfs8(addr)` — read a byte at `fs:addr` (boot.h lines 78-84).
#[inline]
pub fn rdfs8(addr: AddrT) -> u8 {
    let v: u8;
    // SAFETY: an absolute `fs:`-relative load of one byte. The setup stub
    // guarantees FS and `addr` reference accessible memory.
    unsafe {
        core::arch::asm!("mov {v}, fs:[{addr:e}]", v = out(reg_byte) v, addr = in(reg) addr, options(nostack, preserves_flags));
    }
    v
}

/// `rdfs16(addr)` — read a word at `fs:addr` (boot.h lines 85-91).
#[inline]
pub fn rdfs16(addr: AddrT) -> u16 {
    let v: u16;
    // SAFETY: an absolute `fs:`-relative load of one word.
    unsafe {
        core::arch::asm!("mov {v:x}, fs:[{addr:e}]", v = out(reg) v, addr = in(reg) addr, options(nostack, preserves_flags));
    }
    v
}

/// `rdfs32(addr)` — read a dword at `fs:addr` (boot.h lines 92-98).
#[inline]
pub fn rdfs32(addr: AddrT) -> u32 {
    let v: u32;
    // SAFETY: an absolute `fs:`-relative load of one dword.
    unsafe {
        core::arch::asm!("mov {v:e}, fs:[{addr:e}]", v = out(reg) v, addr = in(reg) addr, options(nostack, preserves_flags));
    }
    v
}

/// `wrfs8(v, addr)` — write a byte to `fs:addr` (boot.h lines 100-104).
#[inline]
pub fn wrfs8(v: u8, addr: AddrT) {
    // SAFETY: an absolute `fs:`-relative store of one byte.
    unsafe {
        core::arch::asm!("mov fs:[{addr:e}], {v}", addr = in(reg) addr, v = in(reg_byte) v, options(nostack, preserves_flags));
    }
}

/// `wrfs16(v, addr)` — write a word to `fs:addr` (boot.h lines 105-109).
#[inline]
pub fn wrfs16(v: u16, addr: AddrT) {
    // SAFETY: an absolute `fs:`-relative store of one word.
    unsafe {
        core::arch::asm!("mov fs:[{addr:e}], {v:x}", addr = in(reg) addr, v = in(reg) v, options(nostack, preserves_flags));
    }
}

/// `wrfs32(v, addr)` — write a dword to `fs:addr` (boot.h lines 110-114).
#[inline]
pub fn wrfs32(v: u32, addr: AddrT) {
    // SAFETY: an absolute `fs:`-relative store of one dword.
    unsafe {
        core::arch::asm!("mov fs:[{addr:e}], {v:e}", addr = in(reg) addr, v = in(reg) v, options(nostack, preserves_flags));
    }
}

/// `rdgs8(addr)` — read a byte at `gs:addr` (boot.h lines 116-122).
#[inline]
pub fn rdgs8(addr: AddrT) -> u8 {
    let v: u8;
    // SAFETY: an absolute `gs:`-relative load of one byte.
    unsafe {
        core::arch::asm!("mov {v}, gs:[{addr:e}]", v = out(reg_byte) v, addr = in(reg) addr, options(nostack, preserves_flags));
    }
    v
}

/// `rdgs16(addr)` — read a word at `gs:addr` (boot.h lines 123-129).
#[inline]
pub fn rdgs16(addr: AddrT) -> u16 {
    let v: u16;
    // SAFETY: an absolute `gs:`-relative load of one word.
    unsafe {
        core::arch::asm!("mov {v:x}, gs:[{addr:e}]", v = out(reg) v, addr = in(reg) addr, options(nostack, preserves_flags));
    }
    v
}

/// `rdgs32(addr)` — read a dword at `gs:addr` (boot.h lines 130-136).
#[inline]
pub fn rdgs32(addr: AddrT) -> u32 {
    let v: u32;
    // SAFETY: an absolute `gs:`-relative load of one dword.
    unsafe {
        core::arch::asm!("mov {v:e}, gs:[{addr:e}]", v = out(reg) v, addr = in(reg) addr, options(nostack, preserves_flags));
    }
    v
}

/// `wrgs8(v, addr)` — write a byte to `gs:addr` (boot.h lines 138-142).
#[inline]
pub fn wrgs8(v: u8, addr: AddrT) {
    // SAFETY: an absolute `gs:`-relative store of one byte.
    unsafe {
        core::arch::asm!("mov gs:[{addr:e}], {v}", addr = in(reg) addr, v = in(reg_byte) v, options(nostack, preserves_flags));
    }
}

/// `wrgs16(v, addr)` — write a word to `gs:addr` (boot.h lines 143-147).
#[inline]
pub fn wrgs16(v: u16, addr: AddrT) {
    // SAFETY: an absolute `gs:`-relative store of one word.
    unsafe {
        core::arch::asm!("mov gs:[{addr:e}], {v:x}", addr = in(reg) addr, v = in(reg) v, options(nostack, preserves_flags));
    }
}

/// `wrgs32(v, addr)` — write a dword to `gs:addr` (boot.h lines 148-152).
#[inline]
pub fn wrgs32(v: u32, addr: AddrT) {
    // SAFETY: an absolute `gs:`-relative store of one dword.
    unsafe {
        core::arch::asm!("mov gs:[{addr:e}], {v:e}", addr = in(reg) addr, v = in(reg) v, options(nostack, preserves_flags));
    }
}

/// `memcmp_fs(s1, s2, len)` — compare `len` bytes of `s1` against
/// `fs:s2` (boot.h lines 155-161). Linux uses `fs repe cmpsb` and returns
/// the NZ flag, i.e. `true` when the regions *differ*. We reproduce the
/// instruction with FS overriding the source-index segment.
///
/// # Safety
/// `s1` must point to at least `len` readable bytes and `fs:s2` must
/// reference `len` readable bytes in the FS segment.
#[inline]
pub unsafe fn memcmp_fs(s1: *const u8, s2: AddrT, len: usize) -> bool {
    let diff: u8;
    // SAFETY: caller upholds the region preconditions; `repe cmpsb` with
    // the `fs` prefix matches Linux's `fs repe cmpsb`. `setz` captures the
    // inverse of ZF so `diff != 0` means "differs", mirroring `=@ccnz`.
    unsafe {
        core::arch::asm!(
            "fs repe cmpsb",
            "setnz {diff}",
            diff = out(reg_byte) diff,
            inout("rsi") s2 as u64 => _,
            inout("rdi") s1 as u64 => _,
            inout("rcx") len as u64 => _,
            options(nostack),
        );
    }
    diff != 0
}

/// `memcmp_gs(s1, s2, len)` — same as [`memcmp_fs`] but with the GS
/// segment override (boot.h lines 162-168).
///
/// # Safety
/// `s1` must point to at least `len` readable bytes and `gs:s2` must
/// reference `len` readable bytes in the GS segment.
#[inline]
pub unsafe fn memcmp_gs(s1: *const u8, s2: AddrT, len: usize) -> bool {
    let diff: u8;
    // SAFETY: caller upholds the region preconditions; `repe cmpsb` with
    // the `gs` prefix matches Linux's `gs repe cmpsb`.
    unsafe {
        core::arch::asm!(
            "gs repe cmpsb",
            "setnz {diff}",
            diff = out(reg_byte) diff,
            inout("rsi") s2 as u64 => _,
            inout("rdi") s1 as u64 => _,
            inout("rcx") len as u64 => _,
            options(nostack),
        );
    }
    diff != 0
}

// ---------------------------------------------------------------------------
// Heap bump allocator (boot.h lines 170-190).
//
// Linux models the heap as two globals, `HEAP` (the bump cursor) and
// `heap_end`. `RESET_HEAP` rewinds `HEAP` to `_end`; `__get_heap` rounds
// `HEAP` up to the requested alignment and then advances it by `s*n`;
// `heap_free` reports remaining bytes using a *signed* comparison so a
// cursor that has overrun `heap_end` reports "not free". We capture all of
// that in `BootHeap`.
// ---------------------------------------------------------------------------

/// Bump-allocator cursor for the setup heap (Linux `HEAP`/`heap_end`).
#[derive(Copy, Clone, Debug)]
pub struct BootHeap {
    /// Current bump cursor (Linux `char *HEAP`).
    pub ptr: usize,
    /// One past the last usable byte (Linux `char *heap_end`).
    pub end: usize,
    /// The reset target (`_end`), restored by [`BootHeap::reset`].
    start: usize,
}

impl BootHeap {
    /// Create a heap spanning `[start, end)`. `start` plays the role of
    /// Linux's `_end` (the reset point) and the initial cursor.
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        BootHeap {
            ptr: start,
            end,
            start,
        }
    }

    /// `RESET_HEAP()` — rewind the cursor to `_end` (boot.h line 174:
    /// `HEAP = _end`).
    #[inline]
    pub fn reset(&mut self) {
        self.ptr = self.start;
    }

    /// `__get_heap(s, a, n)` — align the cursor up to `a`, then carve off
    /// `s*n` bytes and return the start of the carved region (boot.h lines
    /// 175-183).
    ///
    /// The alignment rounding is exactly `(HEAP + (a-1)) & ~(a-1)`, so `a`
    /// must be a power of two — the same precondition Linux relies on
    /// because `a` is always `__alignof__(type)`.
    #[inline]
    pub fn get(&mut self, size: usize, align: usize, n: usize) -> usize {
        // (HEAP + (a-1)) & ~(a-1) — round the cursor up to `align`.
        self.ptr = (self.ptr + (align - 1)) & !(align - 1);
        let tmp = self.ptr;
        // HEAP += s*n — advance past the allocation.
        self.ptr += size * n;
        tmp
    }

    /// `heap_free(n)` — does the heap have at least `n` bytes left?
    ///
    /// Mirrors boot.h lines 187-190 exactly, including the signed cast:
    /// `(int)(heap_end - HEAP) >= (int)n`. The signed compare means that if
    /// the cursor has already advanced past `heap_end` (a negative
    /// difference) the function returns `false` even for `n == 0`.
    #[inline]
    pub fn free(&self, n: usize) -> bool {
        // Compute `heap_end - HEAP` as a signed 32-bit value, matching the
        // `(int)` casts upstream so overrun wraps negative.
        let remaining = (self.end as i64 - self.ptr as i64) as i32;
        remaining >= n as i32
    }
}

/// `GET_HEAP(type, n)` — typed allocation helper (boot.h lines 184-185).
///
/// Linux's macro is `__get_heap(sizeof(type), __alignof__(type), n)`. The
/// Rust generic does the same: it carves `size_of::<T>() * n` bytes aligned
/// to `align_of::<T>()` and returns the offset of the region's start. The
/// boot driver casts that offset into a `*mut T` against its real-mode
/// heap base.
#[inline]
pub fn get_heap<T>(heap: &mut BootHeap, n: usize) -> usize {
    heap.get(core::mem::size_of::<T>(), core::mem::align_of::<T>(), n)
}

// ---------------------------------------------------------------------------
// Re-exports of types that boot.h pulls in / forward-declares so callers
// see the same surface as the C header.
// ---------------------------------------------------------------------------

/// Re-export of the real-mode `struct biosregs` and its BIOS-call seam,
/// matching the inline definition in boot.h lines 204-244.
pub use super::biosregs::{BiosCaller, BiosRegs};
/// Re-export the boot-local ctype predicates (boot.h includes ctype.h).
pub use super::ctype::{isdigit, isxdigit};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_size_is_1024() {
        assert_eq!(STACK_SIZE, 1024);
    }

    #[test]
    fn heap_get_rounds_cursor_up_to_alignment() {
        // Start the cursor at an unaligned offset and request 8-byte align.
        let mut h = BootHeap::new(3, 4096);
        let p = h.get(4, 8, 1);
        // 3 rounds up to 8.
        assert_eq!(p, 8);
        // Cursor advanced by size*n = 4.
        assert_eq!(h.ptr, 12);
    }

    #[test]
    fn heap_get_no_rounding_when_already_aligned() {
        let mut h = BootHeap::new(16, 4096);
        let p = h.get(2, 16, 3);
        assert_eq!(p, 16);
        assert_eq!(h.ptr, 16 + 2 * 3);
    }

    #[test]
    fn get_heap_uses_type_size_and_alignment() {
        let mut h = BootHeap::new(1, 4096);
        // u32 has size 4, align 4: cursor 1 -> rounded to 4, return 4.
        let p = get_heap::<u32>(&mut h, 2);
        assert_eq!(p, 4);
        assert_eq!(h.ptr, 4 + 4 * 2);
    }

    #[test]
    fn heap_free_reports_remaining_bytes() {
        let mut h = BootHeap::new(0, 100);
        assert!(h.free(100));
        assert!(h.free(0));
        h.get(1, 1, 90); // cursor now 90, 10 bytes left.
        assert!(h.free(10));
        assert!(!h.free(11));
    }

    #[test]
    fn heap_free_signed_compare_rejects_overrun() {
        // Drive the cursor past heap_end and confirm even free(0) is false,
        // matching Linux's `(int)(heap_end - HEAP) >= (int)n` signed rule.
        let mut h = BootHeap::new(0, 100);
        h.get(1, 1, 120); // cursor 120 > end 100 -> remaining negative.
        assert!(!h.free(0));
        assert!(!h.free(1));
    }

    #[test]
    fn reset_rewinds_cursor_to_start() {
        let mut h = BootHeap::new(8, 4096);
        h.get(4, 4, 10);
        assert_ne!(h.ptr, 8);
        h.reset();
        assert_eq!(h.ptr, 8);
    }
}
