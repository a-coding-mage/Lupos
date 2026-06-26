//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/boot/startup/map_kernel.c
//! test-origin: linux:vendor/linux/arch/x86/boot/startup/map_kernel.c
//! Kernel page-table installer (runs before the main kernel entry).
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/boot/startup/map_kernel.c
//!
//! Linux's `__startup_64()` runs from the early 1:1 mapping right after
//! decompression: it relocates the static high-mapping page tables by
//! `load_delta`, builds the temporary identity mapping used during the
//! switchover to kernel virtual addresses, and invalidates `level2_kernel_pgt`
//! entries outside the kernel image so speculation can't reach reserved areas.
//!
//! Lupos adaptations (documented, not stubs):
//! - The C code addresses the static tables through `rip_rel_ptr()`; the port
//!   receives them as `StartupPageTables` slices and uses the slice base
//!   addresses where Linux stores "physical" table addresses (the same
//!   identity-map assumption the C code makes).
//! - Hardware/SME hooks (`native_read_cr4`, `sme_*`,
//!   `early_snp_set_memory_shared`) sit behind [`Startup64Env`]; the QEMU
//!   boot target runs without SME, where they are no-ops.
//! - Linux hangs (`for (;;)`) on an oversized or misaligned load address;
//!   the port returns `Err(Startup64Error)` and kernel callers loop, keeping
//!   the logic host-testable.

/// Kernel virtual-base address (Linux `__START_KERNEL_map`). Matches
/// `arch/x86/include/asm/page_64_types.h`.
pub const START_KERNEL_MAP: u64 = 0xffff_ffff_8000_0000;

/// `PTRS_PER_{PGD,P4D,PUD,PMD}` — 512 entries per table on x86_64.
pub const PTRS_PER_PGD: usize = 512;
pub const PTRS_PER_P4D: usize = 512;
pub const PTRS_PER_PUD: usize = 512;
pub const PTRS_PER_PMD: usize = 512;
/// `MAX_PTRS_PER_P4D`.
pub const MAX_PTRS_PER_P4D: usize = 512;

/// Paging shifts (4-level): `PGDIR_SHIFT`/`P4D_SHIFT` 39, `PUD_SHIFT` 30,
/// `PMD_SHIFT` 21. With LA57 the pgdir shift becomes 48.
pub const PGDIR_SHIFT: u32 = 39;
pub const PGDIR_SHIFT_L5: u32 = 48;
pub const P4D_SHIFT: u32 = 39;
pub const PUD_SHIFT: u32 = 30;
pub const PMD_SHIFT: u32 = 21;
/// `PMD_SIZE` — 2 MiB; `PMD_MASK` clears offsets inside one PMD.
pub const PMD_SIZE: u64 = 1 << PMD_SHIFT;
pub const PMD_MASK: u64 = !(PMD_SIZE - 1);

/// `MAX_PHYSMEM_BITS` (`asm/sparsemem.h`): 46 with 4-level paging,
/// 52 with LA57.
pub const MAX_PHYSMEM_BITS: u32 = 46;
pub const MAX_PHYSMEM_BITS_L5: u32 = 52;

/// `X86_CR4_LA57` — CR4 bit 12.
pub const X86_CR4_LA57: u64 = 1 << 12;

/// Page-table entry flag sets from `asm/pgtable_types.h`:
/// `_PAGE_TABLE` = P|RW|USER|A|D (encrypted variant adds the SME mask).
pub const PAGE_TABLE: u64 = 0x67;
/// `_KERNPG_TABLE_NOENC` = P|RW|A|D.
pub const KERNPG_TABLE_NOENC: u64 = 0x63;
/// `__PAGE_KERNEL_LARGE_EXEC` = P|RW|A|D|PSE|G.
pub const PAGE_KERNEL_LARGE_EXEC: u64 = 0x1e3;
/// `_PAGE_GLOBAL` / `_PAGE_PRESENT` bits.
pub const PAGE_GLOBAL: u64 = 0x100;
pub const PAGE_PRESENT: u64 = 0x1;

/// `FIXMAP_PMD_TOP` / `FIXMAP_PMD_NUM` (`asm/fixmap.h`): the fixmap PMDs
/// occupy the top of `level2_fixmap_pgt`.
pub const FIXMAP_PMD_TOP: usize = 507;
pub const FIXMAP_PMD_NUM: usize = 2;

/// `EARLY_DYNAMIC_PAGE_TABLES` (`asm/pgtable_64_types.h`).
pub const EARLY_DYNAMIC_PAGE_TABLES: usize = 64;

/// `pgd_index(va)` for 4-level paging.
#[inline]
pub const fn pgd_index(va: u64) -> usize {
    ((va >> PGDIR_SHIFT) & (PTRS_PER_PGD as u64 - 1)) as usize
}

/// `pmd_index(va)`.
#[inline]
pub const fn pmd_index(va: u64) -> usize {
    ((va >> PMD_SHIFT) & (PTRS_PER_PMD as u64 - 1)) as usize
}

/// The static early page tables `__startup_64()` fixes up. Linux reaches
/// them via `rip_rel_ptr()`; lupos passes them in explicitly.
pub struct StartupPageTables<'a> {
    /// `early_top_pgt` (pgd).
    pub early_top_pgt: &'a mut [u64; PTRS_PER_PGD],
    /// `level4_kernel_pgt` (p4d, used only under LA57).
    pub level4_kernel_pgt: &'a mut [u64; MAX_PTRS_PER_P4D],
    /// `level3_kernel_pgt` (pud).
    pub level3_kernel_pgt: &'a mut [u64; PTRS_PER_PUD],
    /// `level2_kernel_pgt` (pmd covering the kernel image).
    pub level2_kernel_pgt: &'a mut [u64; PTRS_PER_PMD],
    /// `level2_fixmap_pgt`.
    pub level2_fixmap_pgt: &'a mut [u64; PTRS_PER_PMD],
    /// `early_dynamic_pgts` scratch tables for the identity map.
    pub early_dynamic_pgts: &'a mut [[u64; PTRS_PER_PMD]; EARLY_DYNAMIC_PAGE_TABLES],
    /// `next_early_pgt` allocation cursor.
    pub next_early_pgt: u32,
}

/// Kernel image placement: `rip_rel_ptr(_text)` / `rip_rel_ptr(_end)`.
#[derive(Copy, Clone, Debug)]
pub struct KernelImage {
    pub text_pa: u64,
    pub end_pa: u64,
}

/// Hardware/SME seam for `__startup_64()`. The QEMU/Arch boot target has
/// no SME/SNP, so the kernel implementation is a thin pass-through.
pub trait Startup64Env {
    /// `native_read_cr4()`.
    fn read_cr4(&self) -> u64;
    /// `check_la57_support()` side effects: `__pgtable_l5_enabled = 1`,
    /// `pgdir_shift = 48`, `ptrs_per_p4d = 512`.
    fn set_pgtable_l5_enabled(&mut self);
    /// `phys_base = load_delta` store.
    fn set_phys_base(&mut self, phys_base: u64);
    /// `sme_get_me_mask()` — 0 when SME is inactive.
    fn sme_get_me_mask(&self) -> u64;
    /// `sme_encrypt_kernel(bp)`.
    fn sme_encrypt_kernel(&mut self);
    /// `early_snp_set_memory_shared(paddr, vaddr, npages)`.
    fn early_snp_set_memory_shared(&mut self, paddr: u64, vaddr: u64, npages: u64);
    /// `rip_rel_ptr(__start_bss_decrypted)..rip_rel_ptr(__end_bss_decrypted)`.
    fn bss_decrypted_range(&self) -> (u64, u64);
}

/// Linux hangs in `for (;;)` for these; lupos surfaces them as errors and
/// lets the kernel boot path loop.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Startup64Error {
    /// `physaddr >> MAX_PHYSMEM_BITS` non-zero ("Is the address too large?").
    PhysAddrTooLarge,
    /// `load_delta & ~PMD_MASK` non-zero ("Is the address not 2M aligned?").
    LoadDeltaUnaligned,
}

/// `check_la57_support()` — LA57 is decided at decompression; only probe
/// CR4 here. Mirrors map_kernel.c lines 17-31.
fn check_la57_support<E: Startup64Env>(env: &mut E) -> bool {
    if env.read_cr4() & X86_CR4_LA57 == 0 {
        return false;
    }
    env.set_pgtable_l5_enabled();
    true
}

/// `sme_postprocess_startup()` — encrypt the kernel under SME and strip the
/// encryption mask from `.bss..decrypted` PMDs. Returns the SME mask used
/// as the CR3 modifier. Mirrors map_kernel.c lines 33-76.
fn sme_postprocess_startup<E: Startup64Env>(
    env: &mut E,
    pmd: &mut [u64; PTRS_PER_PMD],
    p2v_offset: u64,
) -> u64 {
    env.sme_encrypt_kernel();

    if env.sme_get_me_mask() != 0 {
        let (mut paddr, paddr_end) = env.bss_decrypted_range();
        while paddr < paddr_end {
            // On SNP, transition the page to shared in the RMP table to
            // stay consistent with the attribute change below.
            env.early_snp_set_memory_shared(paddr, paddr, PTRS_PER_PMD as u64);

            let i = pmd_index(paddr.wrapping_sub(p2v_offset));
            pmd[i] = pmd[i].wrapping_sub(env.sme_get_me_mask());
            paddr += PMD_SIZE;
        }
    }

    // The SME encryption mask (if active) modifies the initial CR3 pgdir.
    env.sme_get_me_mask()
}

/// `__startup_64(p2v_offset, bp)` — fix up the static high-mapping tables by
/// `load_delta`, build the switchover identity map in `early_dynamic_pgts`,
/// and invalidate `level2_kernel_pgt` outside the image. Mirrors
/// map_kernel.c lines 87-217.
///
/// Table "addresses" written into higher-level entries are the slice base
/// addresses, exactly as the C code stores `rip_rel_ptr` (1:1) addresses.
pub fn startup_64<E: Startup64Env>(
    env: &mut E,
    tables: &mut StartupPageTables<'_>,
    image: KernelImage,
    p2v_offset: u64,
) -> Result<u64, Startup64Error> {
    let physaddr = image.text_pa;

    let la57 = check_la57_support(env);
    let max_physmem_bits = if la57 {
        MAX_PHYSMEM_BITS_L5
    } else {
        MAX_PHYSMEM_BITS
    };

    // Is the address too large?
    if physaddr >> max_physmem_bits != 0 {
        return Err(Startup64Error::PhysAddrTooLarge);
    }

    // Delta between the compiled-at and the running-at address.
    let mut load_delta = START_KERNEL_MAP.wrapping_add(p2v_offset);
    env.set_phys_base(load_delta);

    // Is the address not 2M aligned?
    if load_delta & !PMD_MASK != 0 {
        return Err(Startup64Error::LoadDeltaUnaligned);
    }

    let va_text = physaddr.wrapping_sub(p2v_offset);
    let va_end = image.end_pa.wrapping_sub(p2v_offset);

    // Include the SME encryption mask in the fixup value.
    load_delta = load_delta.wrapping_add(env.sme_get_me_mask());

    // --- Fixup the physical addresses in the page table. ---

    let kernel_pgd_idx = pgd_index(START_KERNEL_MAP);
    tables.early_top_pgt[kernel_pgd_idx] =
        tables.early_top_pgt[kernel_pgd_idx].wrapping_add(load_delta);

    if la57 {
        tables.level4_kernel_pgt[MAX_PTRS_PER_P4D - 1] =
            tables.level4_kernel_pgt[MAX_PTRS_PER_P4D - 1].wrapping_add(load_delta);
        tables.early_top_pgt[kernel_pgd_idx] =
            (tables.level4_kernel_pgt.as_ptr() as u64) | PAGE_TABLE;
    }

    tables.level3_kernel_pgt[PTRS_PER_PUD - 2] =
        tables.level3_kernel_pgt[PTRS_PER_PUD - 2].wrapping_add(load_delta);
    tables.level3_kernel_pgt[PTRS_PER_PUD - 1] =
        tables.level3_kernel_pgt[PTRS_PER_PUD - 1].wrapping_add(load_delta);

    let mut i = FIXMAP_PMD_TOP;
    while i > FIXMAP_PMD_TOP - FIXMAP_PMD_NUM {
        tables.level2_fixmap_pgt[i] = tables.level2_fixmap_pgt[i].wrapping_add(load_delta);
        i -= 1;
    }

    // --- Identity mapping for the switchover. These entries must NOT have
    // the global bit set; bogus neighbours are fine (avoids wraparound
    // problems). ---

    let pud_addr = tables.early_dynamic_pgts[0].as_ptr() as u64;
    let pmd_addr = tables.early_dynamic_pgts[1].as_ptr() as u64;
    tables.next_early_pgt = 2;

    let pgtable_flags = KERNPG_TABLE_NOENC.wrapping_add(env.sme_get_me_mask());

    if la57 {
        let p4d_table_idx = tables.next_early_pgt as usize;
        tables.next_early_pgt += 1;
        let p4d_addr = tables.early_dynamic_pgts[p4d_table_idx].as_ptr() as u64;

        let i = ((physaddr >> PGDIR_SHIFT_L5) % PTRS_PER_PGD as u64) as usize;
        tables.early_top_pgt[i] = p4d_addr.wrapping_add(pgtable_flags);
        tables.early_top_pgt[i + 1] = p4d_addr.wrapping_add(pgtable_flags);

        let i = physaddr >> P4D_SHIFT;
        let (lo, hi) = (
            (i % PTRS_PER_P4D as u64) as usize,
            ((i + 1) % PTRS_PER_P4D as u64) as usize,
        );
        tables.early_dynamic_pgts[p4d_table_idx][lo] = pud_addr.wrapping_add(pgtable_flags);
        tables.early_dynamic_pgts[p4d_table_idx][hi] = pud_addr.wrapping_add(pgtable_flags);
    } else {
        let i = ((physaddr >> PGDIR_SHIFT) % PTRS_PER_PGD as u64) as usize;
        tables.early_top_pgt[i] = pud_addr.wrapping_add(pgtable_flags);
        tables.early_top_pgt[i + 1] = pud_addr.wrapping_add(pgtable_flags);
    }

    let i = physaddr >> PUD_SHIFT;
    let (lo, hi) = (
        (i % PTRS_PER_PUD as u64) as usize,
        ((i + 1) % PTRS_PER_PUD as u64) as usize,
    );
    tables.early_dynamic_pgts[0][lo] = pmd_addr.wrapping_add(pgtable_flags);
    tables.early_dynamic_pgts[0][hi] = pmd_addr.wrapping_add(pgtable_flags);

    let mut pmd_entry = PAGE_KERNEL_LARGE_EXEC & !PAGE_GLOBAL;
    pmd_entry = pmd_entry.wrapping_add(env.sme_get_me_mask());
    pmd_entry = pmd_entry.wrapping_add(physaddr);

    // DIV_ROUND_UP(va_end - va_text, PMD_SIZE) large pages cover the image.
    let n_pmds = va_end.wrapping_sub(va_text).div_ceil(PMD_SIZE);
    for i in 0..n_pmds {
        let idx = (i + (physaddr >> PMD_SHIFT)) % PTRS_PER_PMD as u64;
        tables.early_dynamic_pgts[1][idx as usize] = pmd_entry.wrapping_add(i * PMD_SIZE);
    }

    // --- Fixup kernel text+data virtual addresses in level2_kernel_pgt.
    // Entries outside the image are invalidated: speculative access into
    // reserved areas can halt some platforms (e.g. UV). ---

    // Invalidate pages before the kernel image.
    let mut i = 0;
    while i < pmd_index(va_text) {
        tables.level2_kernel_pgt[i] &= !PAGE_PRESENT;
        i += 1;
    }

    // Fixup pages that are part of the kernel image.
    while i <= pmd_index(va_end) {
        if tables.level2_kernel_pgt[i] & PAGE_PRESENT != 0 {
            tables.level2_kernel_pgt[i] = tables.level2_kernel_pgt[i].wrapping_add(load_delta);
        }
        i += 1;
    }

    // Invalidate pages after the kernel image.
    while i < PTRS_PER_PMD {
        tables.level2_kernel_pgt[i] &= !PAGE_PRESENT;
        i += 1;
    }

    Ok(sme_postprocess_startup(
        env,
        tables.level2_kernel_pgt,
        p2v_offset,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate alloc;
    use alloc::boxed::Box;

    struct TestEnv {
        cr4: u64,
        me_mask: u64,
        l5_enabled: bool,
        phys_base: Option<u64>,
        encrypt_calls: u32,
        snp_shared_calls: u32,
        bss_decrypted: (u64, u64),
    }

    impl Default for TestEnv {
        fn default() -> Self {
            Self {
                cr4: 0,
                me_mask: 0,
                l5_enabled: false,
                phys_base: None,
                encrypt_calls: 0,
                snp_shared_calls: 0,
                bss_decrypted: (0, 0),
            }
        }
    }

    impl Startup64Env for TestEnv {
        fn read_cr4(&self) -> u64 {
            self.cr4
        }
        fn set_pgtable_l5_enabled(&mut self) {
            self.l5_enabled = true;
        }
        fn set_phys_base(&mut self, phys_base: u64) {
            self.phys_base = Some(phys_base);
        }
        fn sme_get_me_mask(&self) -> u64 {
            self.me_mask
        }
        fn sme_encrypt_kernel(&mut self) {
            self.encrypt_calls += 1;
        }
        fn early_snp_set_memory_shared(&mut self, _paddr: u64, _vaddr: u64, _npages: u64) {
            self.snp_shared_calls += 1;
        }
        fn bss_decrypted_range(&self) -> (u64, u64) {
            self.bss_decrypted
        }
    }

    struct TestTables {
        pgd: Box<[u64; PTRS_PER_PGD]>,
        p4d: Box<[u64; MAX_PTRS_PER_P4D]>,
        pud: Box<[u64; PTRS_PER_PUD]>,
        pmd: Box<[u64; PTRS_PER_PMD]>,
        fixmap: Box<[u64; PTRS_PER_PMD]>,
        dynamic: Box<[[u64; PTRS_PER_PMD]; EARLY_DYNAMIC_PAGE_TABLES]>,
    }

    impl TestTables {
        fn new() -> Self {
            Self {
                pgd: Box::new([0; PTRS_PER_PGD]),
                p4d: Box::new([0; MAX_PTRS_PER_P4D]),
                pud: Box::new([0; PTRS_PER_PUD]),
                pmd: Box::new([0; PTRS_PER_PMD]),
                fixmap: Box::new([0; PTRS_PER_PMD]),
                dynamic: Box::new([[0; PTRS_PER_PMD]; EARLY_DYNAMIC_PAGE_TABLES]),
            }
        }

        fn view(&mut self) -> StartupPageTables<'_> {
            StartupPageTables {
                early_top_pgt: &mut self.pgd,
                level4_kernel_pgt: &mut self.p4d,
                level3_kernel_pgt: &mut self.pud,
                level2_kernel_pgt: &mut self.pmd,
                level2_fixmap_pgt: &mut self.fixmap,
                early_dynamic_pgts: &mut self.dynamic,
                next_early_pgt: 0,
            }
        }
    }

    /// Default-link layout: text at VA __START_KERNEL_map, loaded at
    /// `text_pa`, so p2v_offset = text_pa - VA and load_delta = text_pa.
    fn default_image(text_pa: u64, size: u64) -> (KernelImage, u64) {
        let image = KernelImage {
            text_pa,
            end_pa: text_pa + size,
        };
        let p2v_offset = text_pa.wrapping_sub(START_KERNEL_MAP);
        (image, p2v_offset)
    }

    #[test]
    fn pte_flag_constants_match_linux_pgtable_types() {
        // _KERNPG_TABLE_NOENC = __PP|__RW|___A|___D.
        assert_eq!(KERNPG_TABLE_NOENC, 0x63);
        // _PAGE_TABLE (sans SME mask) = __PP|__RW|_USR|___A|___D.
        assert_eq!(PAGE_TABLE, 0x67);
        // __PAGE_KERNEL_LARGE_EXEC = __PP|__RW|___A|___D|_PSE|___G.
        assert_eq!(PAGE_KERNEL_LARGE_EXEC, 0x1e3);
        assert_eq!(FIXMAP_PMD_TOP, 507);
        assert_eq!(FIXMAP_PMD_NUM, 2);
        assert_eq!(EARLY_DYNAMIC_PAGE_TABLES, 64);
        assert_eq!(START_KERNEL_MAP, 0xffff_ffff_8000_0000);
        // __START_KERNEL_map lives in the last pgd slot.
        assert_eq!(pgd_index(START_KERNEL_MAP), 511);
    }

    #[test]
    fn startup_64_relocates_high_mapping_by_load_delta() {
        let mut env = TestEnv::default();
        let mut t = TestTables::new();
        // Seed the entries Linux relocates.
        t.pgd[511] = 0x1000;
        t.pud[PTRS_PER_PUD - 2] = 0x2000;
        t.pud[PTRS_PER_PUD - 1] = 0x3000;
        t.fixmap[FIXMAP_PMD_TOP] = 0x4000;
        t.fixmap[FIXMAP_PMD_TOP - 1] = 0x5000;
        t.fixmap[FIXMAP_PMD_TOP - 2] = 0x6000; // outside FIXMAP_PMD_NUM

        let load = 16 * 1024 * 1024; // 16 MiB, 2M-aligned
        let (image, p2v) = default_image(load, 4 * PMD_SIZE);
        let mut view = t.view();
        let r = startup_64(&mut env, &mut view, image, p2v);
        assert_eq!(r, Ok(0));

        assert_eq!(env.phys_base, Some(load));
        assert_eq!(t.pgd[511], 0x1000 + load);
        assert_eq!(t.pud[PTRS_PER_PUD - 2], 0x2000 + load);
        assert_eq!(t.pud[PTRS_PER_PUD - 1], 0x3000 + load);
        assert_eq!(t.fixmap[FIXMAP_PMD_TOP], 0x4000 + load);
        assert_eq!(t.fixmap[FIXMAP_PMD_TOP - 1], 0x5000 + load);
        // Only FIXMAP_PMD_NUM (2) entries are touched.
        assert_eq!(t.fixmap[FIXMAP_PMD_TOP - 2], 0x6000);
    }

    #[test]
    fn startup_64_builds_identity_map_for_switchover() {
        let mut env = TestEnv::default();
        let mut t = TestTables::new();
        let load = 16 * 1024 * 1024;
        let size = 4 * PMD_SIZE; // 8 MiB image
        let (image, p2v) = default_image(load, size);

        let pud_addr = t.dynamic[0].as_ptr() as u64;
        let pmd_addr = t.dynamic[1].as_ptr() as u64;
        let mut view = t.view();
        assert_eq!(startup_64(&mut env, &mut view, image, p2v), Ok(0));
        let next = view.next_early_pgt;

        // pgd[i], pgd[i+1] point at early pud with _KERNPG_TABLE_NOENC.
        let i = ((load >> PGDIR_SHIFT) % PTRS_PER_PGD as u64) as usize;
        assert_eq!(t.pgd[i], pud_addr + KERNPG_TABLE_NOENC);
        assert_eq!(t.pgd[i + 1], pud_addr + KERNPG_TABLE_NOENC);

        // pud entries point at the early pmd table.
        let i = ((load >> PUD_SHIFT) % PTRS_PER_PUD as u64) as usize;
        assert_eq!(t.dynamic[0][i], pmd_addr + KERNPG_TABLE_NOENC);
        assert_eq!(t.dynamic[0][i + 1], pmd_addr + KERNPG_TABLE_NOENC);

        // 4 large pages cover [text, end), global bit stripped.
        let base = (PAGE_KERNEL_LARGE_EXEC & !PAGE_GLOBAL) + load;
        let first = ((load >> PMD_SHIFT) % PTRS_PER_PMD as u64) as usize;
        for k in 0..4u64 {
            assert_eq!(t.dynamic[1][first + k as usize], base + k * PMD_SIZE);
            assert_eq!(t.dynamic[1][first + k as usize] & PAGE_GLOBAL, 0);
        }
        assert_eq!(t.dynamic[1][first + 4], 0); // nothing past the image
        assert_eq!(next, 2); // two early tables consumed (no la57)
    }

    #[test]
    fn startup_64_invalidates_level2_outside_kernel_image() {
        let mut env = TestEnv::default();
        let mut t = TestTables::new();
        // All level2 entries present with distinct payloads.
        for (k, e) in t.pmd.iter_mut().enumerate() {
            *e = ((k as u64) << 21) | PAGE_PRESENT;
        }
        // One hole inside the image: stays non-present, NOT relocated.
        t.pmd[2] = 0;

        let load = 16 * 1024 * 1024;
        let size = 4 * PMD_SIZE;
        let (image, p2v) = default_image(load, size);
        let mut view = t.view();
        assert_eq!(startup_64(&mut env, &mut view, image, p2v), Ok(0));

        // va_text = __START_KERNEL_map → pmd_index 0; va_end → index 4.
        for k in 0..=4usize {
            if k == 2 {
                assert_eq!(t.pmd[2], 0); // the hole is untouched
            } else {
                assert_eq!(t.pmd[k], (((k as u64) << 21) | PAGE_PRESENT) + load);
            }
        }
        // Entries after the image lose _PAGE_PRESENT but keep the payload.
        for k in 5..PTRS_PER_PMD {
            assert_eq!(t.pmd[k] & PAGE_PRESENT, 0);
            assert_eq!(t.pmd[k] & !PAGE_PRESENT, (k as u64) << 21);
        }
    }

    #[test]
    fn startup_64_rejects_oversized_or_unaligned_load_address() {
        let mut env = TestEnv::default();
        let mut t = TestTables::new();

        // physaddr >= 2^46 → "address too large" hang in Linux.
        let (image, p2v) = default_image(1 << MAX_PHYSMEM_BITS, PMD_SIZE);
        let mut view = t.view();
        assert_eq!(
            startup_64(&mut env, &mut view, image, p2v),
            Err(Startup64Error::PhysAddrTooLarge)
        );

        // Load delta not 2M-aligned → "not 2M aligned" hang in Linux.
        let (image, p2v) = default_image(16 * 1024 * 1024 + 0x1000, PMD_SIZE);
        let mut view = t.view();
        assert_eq!(
            startup_64(&mut env, &mut view, image, p2v),
            Err(Startup64Error::LoadDeltaUnaligned)
        );
    }

    #[test]
    fn startup_64_la57_routes_through_level4_and_extra_early_table() {
        let mut env = TestEnv {
            cr4: X86_CR4_LA57,
            ..TestEnv::default()
        };
        let mut t = TestTables::new();
        t.p4d[MAX_PTRS_PER_P4D - 1] = 0x7000;

        let load = 16 * 1024 * 1024;
        let (image, p2v) = default_image(load, 2 * PMD_SIZE);
        let p4d_static_addr = t.p4d.as_ptr() as u64;
        let pud_addr = t.dynamic[0].as_ptr() as u64;
        let p4d_dyn_addr = t.dynamic[2].as_ptr() as u64;

        let mut view = t.view();
        assert_eq!(startup_64(&mut env, &mut view, image, p2v), Ok(0));
        let next = view.next_early_pgt;

        assert!(env.l5_enabled);
        // level4_kernel_pgt top slot relocated, then pgd repointed at it.
        assert_eq!(t.p4d[MAX_PTRS_PER_P4D - 1], 0x7000 + load);
        assert_eq!(t.pgd[511], p4d_static_addr | PAGE_TABLE);
        // Identity map uses a third early table as the p4d level.
        assert_eq!(next, 3);
        let i = ((load >> PGDIR_SHIFT_L5) % PTRS_PER_PGD as u64) as usize;
        assert_eq!(t.pgd[i], p4d_dyn_addr + KERNPG_TABLE_NOENC);
        let i = ((load >> P4D_SHIFT) % PTRS_PER_P4D as u64) as usize;
        assert_eq!(t.dynamic[2][i], pud_addr + KERNPG_TABLE_NOENC);
    }

    #[test]
    fn sme_postprocess_strips_mask_from_bss_decrypted_and_returns_mask() {
        let me_mask = 1u64 << 47; // C-bit style mask
        let load = 16 * 1024 * 1024;
        let mut env = TestEnv {
            me_mask,
            // One PMD-sized .bss..decrypted window at text + 2 MiB.
            bss_decrypted: (load + PMD_SIZE, load + 2 * PMD_SIZE),
            ..TestEnv::default()
        };
        let mut t = TestTables::new();
        for (k, e) in t.pmd.iter_mut().enumerate() {
            *e = (((k as u64) << 21) | PAGE_PRESENT).wrapping_add(me_mask);
        }

        let (image, p2v) = default_image(load, 4 * PMD_SIZE);
        let mut view = t.view();
        // Return value is the CR3 modifier mask.
        assert_eq!(startup_64(&mut env, &mut view, image, p2v), Ok(me_mask));
        assert_eq!(env.encrypt_calls, 1);
        assert_eq!(env.snp_shared_calls, 1);

        // bss..decrypted PMD (index 1) lost the mask after the load_delta
        // fixup (load_delta itself includes the mask, hence net +load).
        let expected_relocated = ((1u64 << 21) | PAGE_PRESENT)
            .wrapping_add(me_mask)
            .wrapping_add(load)
            .wrapping_add(me_mask);
        assert_eq!(t.pmd[1], expected_relocated.wrapping_sub(me_mask));
        // A neighbouring in-image PMD keeps its mask.
        assert_eq!(
            t.pmd[3],
            (((3u64) << 21) | PAGE_PRESENT)
                .wrapping_add(me_mask)
                .wrapping_add(load)
                .wrapping_add(me_mask)
        );
    }
}
