//! linux-parity: complete
//! linux-source: vendor/linux/arch/x86/mm/ident_map.c
//! test-origin: linux:vendor/linux/arch/x86/mm/ident_map.c
//! Identity-mapping page-table builder.
//!
//! Ports / mirrors:
//! - vendor/linux/arch/x86/mm/ident_map.c
//!
//! `kernel_ident_mapping_init()` builds (or extends) identity-mapped page
//! tables for `[pstart, pend)`, choosing 1 GiB / 2 MiB leaves where allowed
//! and never overwriting existing mappings. Linux compiles this file into
//! both the regular kernel (kexec, machine_kexec_64.c) and the decompressor
//! (`arch/x86/boot/compressed/ident_map_64.c` #includes it).
//!
//! Lupos adaptations (documented, not stubs):
//! - `x86_mapping_info`'s `alloc_pgt_page(context)` callback plus the
//!   `__pa`/`__va` macros become the [`IdentMapEnv`] trait, so the same
//!   walker serves the decompressor (identity `__pa`) and the kernel
//!   (direct-map `__pa`), and host tests run on an arena.
//! - `pgtable_l5_enabled()` is an env probe; the folded-p4d (4-level) walk
//!   reproduces Linux's `pgtable-nop4d.h` behavior, including pointing the
//!   pgd entry directly at the pud table.

use crate::include::uapi::errno::ENOMEM;

/// One 4 KiB page table: 512 八-byte entries.
pub type PgtPage = [u64; 512];

pub const PTRS_PER_PGD: usize = 512;
pub const PTRS_PER_P4D: usize = 512;
pub const PTRS_PER_PUD: usize = 512;
pub const PTRS_PER_PMD: usize = 512;

pub const PGDIR_SHIFT: u32 = 39;
pub const PGDIR_SHIFT_L5: u32 = 48;
pub const P4D_SHIFT: u32 = 39;
pub const PUD_SHIFT: u32 = 30;
pub const PMD_SHIFT: u32 = 21;

pub const PMD_SIZE: u64 = 1 << PMD_SHIFT;
pub const PMD_MASK: u64 = !(PMD_SIZE - 1);
pub const PUD_SIZE: u64 = 1 << PUD_SHIFT;
pub const PUD_MASK: u64 = !(PUD_SIZE - 1);

/// `_PAGE_PRESENT` / `_PAGE_PSE` (leaf) bits, `asm/pgtable_types.h`.
pub const PAGE_PRESENT: u64 = 1 << 0;
pub const PAGE_PSE: u64 = 1 << 7;
/// `_PAGE_NOPTISHADOW` — "this root entry has no PTI shadow". Bit 58
/// (`_PAGE_BIT_SOFTW5`) with CONFIG_MITIGATION_PAGE_TABLE_ISOLATION=y,
/// which the QEMU/Arch defconfig sets.
pub const PAGE_NOPTISHADOW: u64 = 1 << 58;
/// `_KERNPG_TABLE` sans the SME `_ENC` modifier (callers add it via
/// `kernpg_flag`, as the decompressor does).
pub const KERNPG_TABLE: u64 = 0x63;
/// `PTE_PFN_MASK` — bits 51:12 carry the physical table/page address.
pub const PTE_PFN_MASK: u64 = 0x000f_ffff_ffff_f000;

/// Allocation + address-translation seam: Linux's `alloc_pgt_page`
/// callback, `free_pgt_page`, `__pa`/`__va`, and `pgtable_l5_enabled()`.
pub trait IdentMapEnv {
    /// Returns a zeroed page-table page, or `None` when the buffer is
    /// exhausted (Linux returns NULL → -ENOMEM).
    fn alloc_pgt_page(&mut self) -> Option<*mut PgtPage>;
    /// `free_pgt_page` callback used by `kernel_ident_mapping_free`.
    fn free_pgt_page(&mut self, _page: *mut PgtPage) {}
    /// `__pa(table)` — physical address stored into higher-level entries.
    fn pa(&self, page: *const PgtPage) -> u64;
    /// `__va(pa)` — walk back down from an entry to its table.
    fn va(&self, pa: u64) -> *mut PgtPage;
    /// `pgtable_l5_enabled()`.
    fn l5_enabled(&self) -> bool {
        false
    }
}

/// `struct x86_mapping_info` (minus the callback, which lives on the env).
#[derive(Copy, Clone, Debug)]
pub struct X86MappingInfo {
    /// Flags for large-page leaf entries (`__PAGE_KERNEL_LARGE_EXEC`…).
    pub page_flag: u64,
    /// Flags for table entries; 0 → `_KERNPG_TABLE` default.
    pub kernpg_flag: u64,
    /// Filter applied to `kernpg_flag` (`__default_kernel_pte_mask`).
    pub default_kernel_pte_mask: u64,
    /// PA→VA offset added to the mapped range (0 for identity maps).
    pub offset: u64,
    /// Whether 1 GiB leaves may be used (`direct_gbpages`).
    pub direct_gbpages: bool,
}

impl Default for X86MappingInfo {
    fn default() -> Self {
        Self {
            page_flag: 0,
            kernpg_flag: 0,
            default_kernel_pte_mask: !0,
            offset: 0,
            direct_gbpages: false,
        }
    }
}

#[inline]
const fn index_for(addr: u64, shift: u32, ptrs: usize) -> usize {
    ((addr >> shift) & (ptrs as u64 - 1)) as usize
}

/// `pXd_addr_end(addr, end)` — end of the current pXd region, capped at
/// `end`. Mirrors `include/linux/pgtable.h`.
#[inline]
const fn addr_end(addr: u64, size_shift: u32, end: u64) -> u64 {
    let boundary = (addr.wrapping_add(1 << size_shift)) & !((1u64 << size_shift) - 1);
    if boundary < end && boundary != 0 {
        boundary
    } else {
        end
    }
}

#[inline]
fn entry_present(e: u64) -> bool {
    e & PAGE_PRESENT != 0
}

#[inline]
fn entry_leaf(e: u64) -> bool {
    e & PAGE_PSE != 0
}

/// `ident_pmd_init()` — fill 2 MiB leaves, skipping present entries.
/// Mirrors ident_map.c lines 80-92.
fn ident_pmd_init<E: IdentMapEnv>(
    env: &mut E,
    info: &X86MappingInfo,
    pmd_page: *mut PgtPage,
    addr: u64,
    end: u64,
) {
    let pmd = unsafe { &mut *pmd_page };
    let mut addr = addr & PMD_MASK;
    while addr < end {
        let i = index_for(addr, PMD_SHIFT, PTRS_PER_PMD);
        if !entry_present(pmd[i]) {
            pmd[i] = addr.wrapping_sub(info.offset) | info.page_flag;
        }
        addr += PMD_SIZE;
    }
    let _ = env;
}

/// `ident_pud_init()` — gbpage fast path or pmd-table descent. Mirrors
/// ident_map.c lines 94-143.
fn ident_pud_init<E: IdentMapEnv>(
    env: &mut E,
    info: &X86MappingInfo,
    pud_page: *mut PgtPage,
    mut addr: u64,
    end: u64,
) -> Result<(), i32> {
    while addr < end {
        let i = index_for(addr, PUD_SHIFT, PTRS_PER_PUD);
        let next = addr_end(addr, PUD_SHIFT, end);
        let pud = unsafe { &mut (*pud_page)[i] };

        // If this is already a gbpage, this portion is already mapped.
        if entry_leaf(*pud) {
            addr = next;
            continue;
        }

        // Is using a gbpage allowed? Only when it maps exactly the
        // requested region (aligned at both ends) and nothing exists.
        let use_gbpage = info.direct_gbpages
            && (addr & !PUD_MASK) == 0
            && (next & !PUD_MASK) == 0
            && !entry_present(*pud);

        if use_gbpage {
            *pud = addr.wrapping_sub(info.offset) | info.page_flag;
            addr = next;
            continue;
        }

        if entry_present(*pud) {
            let pmd = env.va(*pud & PTE_PFN_MASK);
            ident_pmd_init(env, info, pmd, addr, next);
            addr = next;
            continue;
        }
        let pmd = env.alloc_pgt_page().ok_or(-ENOMEM)?;
        ident_pmd_init(env, info, pmd, addr, next);
        let pud = unsafe { &mut (*pud_page)[i] };
        *pud = env.pa(pmd) | info.kernpg_flag;
        addr = next;
    }

    Ok(())
}

/// `ident_p4d_init()`. With 4-level paging the p4d is folded: the "page"
/// holds a single effective entry (index 0), exactly as
/// `pgtable-nop4d.h` folds `p4d_index()` to 0. Mirrors ident_map.c
/// lines 145-176.
fn ident_p4d_init<E: IdentMapEnv>(
    env: &mut E,
    info: &X86MappingInfo,
    p4d_page: *mut PgtPage,
    mut addr: u64,
    end: u64,
) -> Result<(), i32> {
    while addr < end {
        let i = if env.l5_enabled() {
            index_for(addr, P4D_SHIFT, PTRS_PER_P4D)
        } else {
            0
        };
        let next = if env.l5_enabled() {
            addr_end(addr, P4D_SHIFT, end)
        } else {
            // Folded p4d covers the whole pgd range.
            end
        };
        let p4d = unsafe { &mut (*p4d_page)[i] };

        if entry_present(*p4d) {
            let pud = env.va(*p4d & PTE_PFN_MASK);
            ident_pud_init(env, info, pud, addr, next)?;
            addr = next;
            continue;
        }
        let pud = env.alloc_pgt_page().ok_or(-ENOMEM)?;
        ident_pud_init(env, info, pud, addr, next)?;
        let p4d = unsafe { &mut (*p4d_page)[i] };
        *p4d = env.pa(pud) | info.kernpg_flag | PAGE_NOPTISHADOW;
        addr = next;
    }

    Ok(())
}

/// `kernel_ident_mapping_init()` — top-level walk over `[pstart, pend)`
/// (+ `info.offset`). Mirrors ident_map.c lines 178-225.
pub fn kernel_ident_mapping_init<E: IdentMapEnv>(
    env: &mut E,
    info: &mut X86MappingInfo,
    pgd_page: *mut PgtPage,
    pstart: u64,
    pend: u64,
) -> Result<(), i32> {
    let mut addr = pstart.wrapping_add(info.offset);
    let end = pend.wrapping_add(info.offset);

    // Set the default pagetable flags if not supplied.
    if info.kernpg_flag == 0 {
        info.kernpg_flag = KERNPG_TABLE;
    }
    // Filter out unsupported __PAGE_KERNEL_* bits.
    info.kernpg_flag &= info.default_kernel_pte_mask;

    let pgdir_shift = if env.l5_enabled() {
        PGDIR_SHIFT_L5
    } else {
        PGDIR_SHIFT
    };

    while addr < end {
        let i = index_for(addr, pgdir_shift, PTRS_PER_PGD);
        let next = addr_end(addr, pgdir_shift, end);
        let pgd = unsafe { &mut (*pgd_page)[i] };

        if entry_present(*pgd) {
            let p4d = if env.l5_enabled() {
                env.va(*pgd & PTE_PFN_MASK)
            } else {
                // Folded p4d: the pgd entry *is* the p4d entry, but Linux
                // descends via p4d_offset(pgd, 0) == (p4d_t *)pgd, i.e.
                // treats this entry slot as the one-entry p4d table.
                core::ptr::addr_of_mut!(*pgd) as *mut PgtPage
            };
            ident_p4d_init(env, info, p4d, addr, next)?;
            addr = next;
            continue;
        }

        let p4d = env.alloc_pgt_page().ok_or(-ENOMEM)?;
        ident_p4d_init(env, info, p4d, addr, next)?;
        let pgd = unsafe { &mut (*pgd_page)[i] };
        if env.l5_enabled() {
            *pgd = env.pa(p4d) | info.kernpg_flag | PAGE_NOPTISHADOW;
        } else {
            // With p4d folded, point the pgd entry at the pud table the
            // scratch p4d page's slot 0 references.
            let pud_pa = unsafe { (*p4d)[0] } & PTE_PFN_MASK;
            *pgd = pud_pa | info.kernpg_flag | PAGE_NOPTISHADOW;
        }
        addr = next;
    }

    Ok(())
}

/// `kernel_ident_mapping_free()` — release every table the walk above
/// allocated. Mirrors ident_map.c lines 7-78.
pub fn kernel_ident_mapping_free<E: IdentMapEnv>(env: &mut E, pgd_page: *mut PgtPage) {
    for i in 0..PTRS_PER_PGD {
        let pgd = unsafe { (*pgd_page)[i] };
        if !entry_present(pgd) {
            continue;
        }
        // free_p4d: with l5 disabled the pgd entry references the pud
        // table directly and there is no separate p4d page to free.
        if env.l5_enabled() {
            let p4d_page = env.va(pgd & PTE_PFN_MASK);
            for j in 0..PTRS_PER_P4D {
                let p4d = unsafe { (*p4d_page)[j] };
                if !entry_present(p4d) {
                    continue;
                }
                free_pud(env, p4d & PTE_PFN_MASK);
            }
            env.free_pgt_page(p4d_page);
        } else {
            free_pud(env, pgd & PTE_PFN_MASK);
        }
    }
    env.free_pgt_page(pgd_page);
}

fn free_pud<E: IdentMapEnv>(env: &mut E, pud_pa: u64) {
    let pud_page = env.va(pud_pa);
    for i in 0..PTRS_PER_PUD {
        let pud = unsafe { (*pud_page)[i] };
        if !entry_present(pud) || entry_leaf(pud) {
            continue;
        }
        let pmd_page = env.va(pud & PTE_PFN_MASK);
        // free_pmd: PTE level is never allocated by the identity mapper
        // (leaves stop at 2 MiB), so present non-leaf pmds cannot occur
        // here; Linux still walks them for kexec'd tables with PTEs.
        for j in 0..PTRS_PER_PMD {
            let pmd = unsafe { (*pmd_page)[j] };
            if !entry_present(pmd) || entry_leaf(pmd) {
                continue;
            }
            env.free_pgt_page(env.va(pmd & PTE_PFN_MASK));
        }
        env.free_pgt_page(pmd_page);
    }
    env.free_pgt_page(pud_page);
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate alloc;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    /// A page-table page with real page-table alignment: entries store
    /// `pa | flags`, so table addresses must leave bits 11:0 free.
    #[repr(C, align(4096))]
    struct AlignedPage(PgtPage);

    /// Arena env: identity __pa/__va (host addresses), like the
    /// decompressor's mapping context.
    struct Arena {
        pages: Vec<Box<AlignedPage>>,
        freed: Vec<*mut PgtPage>,
        limit: usize,
        l5: bool,
    }

    impl Arena {
        fn new(limit: usize) -> Self {
            Self {
                pages: Vec::new(),
                freed: Vec::new(),
                limit,
                l5: false,
            }
        }
    }

    impl IdentMapEnv for Arena {
        fn alloc_pgt_page(&mut self) -> Option<*mut PgtPage> {
            if self.pages.len() >= self.limit {
                return None;
            }
            self.pages.push(Box::new(AlignedPage([0u64; 512])));
            Some(&mut self.pages.last_mut().unwrap().0 as *mut PgtPage)
        }
        fn free_pgt_page(&mut self, page: *mut PgtPage) {
            self.freed.push(page);
        }
        fn pa(&self, page: *const PgtPage) -> u64 {
            page as u64
        }
        fn va(&self, pa: u64) -> *mut PgtPage {
            pa as *mut PgtPage
        }
        fn l5_enabled(&self) -> bool {
            self.l5
        }
    }

    const LARGE_EXEC: u64 = 0x1e3; // __PAGE_KERNEL_LARGE_EXEC

    fn info() -> X86MappingInfo {
        X86MappingInfo {
            page_flag: LARGE_EXEC,
            ..Default::default()
        }
    }

    fn walk_to_pmd(env: &Arena, pgd: &PgtPage, addr: u64) -> u64 {
        let pgd_e = pgd[index_for(addr, PGDIR_SHIFT, 512)];
        assert!(entry_present(pgd_e), "pgd entry present");
        let pud_page = unsafe { &*env.va(pgd_e & PTE_PFN_MASK) };
        let pud_e = pud_page[index_for(addr, PUD_SHIFT, 512)];
        assert!(entry_present(pud_e), "pud entry present");
        if entry_leaf(pud_e) {
            return pud_e;
        }
        let pmd_page = unsafe { &*env.va(pud_e & PTE_PFN_MASK) };
        pmd_page[index_for(addr, PMD_SHIFT, 512)]
    }

    #[test]
    fn maps_2mib_leaves_for_small_identity_range() {
        let mut env = Arena::new(16);
        let mut inf = info();
        let mut pgd: Box<PgtPage> = Box::new([0; 512]);
        // 16 MiB..24 MiB → four 2 MiB pmd leaves.
        let (s, e) = (16 * 1024 * 1024, 24 * 1024 * 1024);
        kernel_ident_mapping_init(&mut env, &mut inf, pgd.as_mut(), s, e).unwrap();

        for off in (0..(e - s)).step_by(PMD_SIZE as usize) {
            let pmd = walk_to_pmd(&env, &pgd, s + off);
            assert_eq!(pmd, (s + off) | LARGE_EXEC);
            assert!(entry_leaf(pmd));
        }
        // Nothing below/above the range.
        assert_eq!(walk_to_pmd(&env, &pgd, s - PMD_SIZE), 0);
        assert_eq!(walk_to_pmd(&env, &pgd, e), 0);
        // kernpg default applied to table entries; pgd carries NOPTISHADOW.
        assert_eq!(inf.kernpg_flag, KERNPG_TABLE);
        let pgd_e = pgd[index_for(s, PGDIR_SHIFT, 512)];
        assert_ne!(pgd_e & PAGE_NOPTISHADOW, 0);
    }

    #[test]
    fn uses_gbpage_leaf_only_when_aligned_and_enabled() {
        let mut env = Arena::new(16);
        let mut inf = info();
        inf.direct_gbpages = true;
        let mut pgd: Box<PgtPage> = Box::new([0; 512]);
        // Exactly one aligned GiB → single pud leaf, no pmd table alloc.
        kernel_ident_mapping_init(&mut env, &mut inf, pgd.as_mut(), PUD_SIZE, 2 * PUD_SIZE)
            .unwrap();
        let leaf = walk_to_pmd(&env, &pgd, PUD_SIZE);
        assert!(entry_leaf(leaf));
        assert_eq!(leaf, PUD_SIZE | LARGE_EXEC);
        // Allocations: 1 scratch p4d + 1 pud. No pmd.
        assert_eq!(env.pages.len(), 2);

        // Unaligned tail falls back to 2 MiB leaves even with gbpages on.
        let mut env2 = Arena::new(16);
        let mut inf2 = info();
        inf2.direct_gbpages = true;
        let mut pgd2: Box<PgtPage> = Box::new([0; 512]);
        kernel_ident_mapping_init(
            &mut env2,
            &mut inf2,
            pgd2.as_mut(),
            PUD_SIZE,
            2 * PUD_SIZE + PMD_SIZE,
        )
        .unwrap();
        let head = walk_to_pmd(&env2, &pgd2, PUD_SIZE);
        assert!(entry_leaf(head)); // first GiB is a gbpage
        let tail = walk_to_pmd(&env2, &pgd2, 2 * PUD_SIZE);
        assert_eq!(tail, (2 * PUD_SIZE) | LARGE_EXEC); // tail is a 2M leaf
    }

    #[test]
    fn never_overwrites_existing_mappings() {
        let mut env = Arena::new(16);
        let mut inf = info();
        let mut pgd: Box<PgtPage> = Box::new([0; 512]);
        let s = 16 * 1024 * 1024;
        kernel_ident_mapping_init(&mut env, &mut inf, pgd.as_mut(), s, s + PMD_SIZE).unwrap();
        let before = walk_to_pmd(&env, &pgd, s);

        // Re-map an overlapping, larger range: the existing pmd survives.
        kernel_ident_mapping_init(&mut env, &mut inf, pgd.as_mut(), s, s + 2 * PMD_SIZE).unwrap();
        assert_eq!(walk_to_pmd(&env, &pgd, s), before);
        assert_eq!(
            walk_to_pmd(&env, &pgd, s + PMD_SIZE),
            (s + PMD_SIZE) | LARGE_EXEC
        );
    }

    #[test]
    fn offset_produces_virtual_to_physical_delta() {
        // Linux uses info->offset for kexec's __START_KERNEL_map windows:
        // table indices come from VA = PA + offset, entries store PA.
        let mut env = Arena::new(16);
        let mut inf = info();
        inf.offset = 0x40_0000_0000; // 256 GiB shift
        let mut pgd: Box<PgtPage> = Box::new([0; 512]);
        let pa = 16 * 1024 * 1024u64;
        kernel_ident_mapping_init(&mut env, &mut inf, pgd.as_mut(), pa, pa + PMD_SIZE).unwrap();
        let va = pa + inf.offset;
        let pmd = walk_to_pmd(&env, &pgd, va);
        assert_eq!(pmd & PTE_PFN_MASK, pa);
    }

    #[test]
    fn alloc_exhaustion_returns_enomem() {
        let mut env = Arena::new(1); // only the scratch p4d fits
        let mut inf = info();
        let mut pgd: Box<PgtPage> = Box::new([0; 512]);
        let r = kernel_ident_mapping_init(
            &mut env,
            &mut inf,
            pgd.as_mut(),
            0x100_0000,
            0x100_0000 + PMD_SIZE,
        );
        assert_eq!(r, Err(-ENOMEM));
    }

    #[test]
    fn mapping_free_releases_allocated_tables() {
        let mut env = Arena::new(16);
        let mut inf = info();
        let pgd = env.alloc_pgt_page().unwrap();
        let s = 16 * 1024 * 1024;
        kernel_ident_mapping_init(&mut env, &mut inf, pgd, s, s + PMD_SIZE).unwrap();
        let allocated = env.pages.len(); // pgd + p4d scratch + pud + pmd = 4

        kernel_ident_mapping_free(&mut env, pgd);
        // Freed: pud + pmd + pgd. The folded-p4d scratch page is not
        // reachable from the pgd (Linux leaks it the same way: with
        // l5 disabled free_p4d() only frees when pgtable_l5_enabled()).
        assert_eq!(env.freed.len(), allocated - 1);
    }

    #[test]
    fn five_level_walk_inserts_real_p4d_level() {
        let mut env = Arena::new(16);
        env.l5 = true;
        let mut inf = info();
        let mut pgd: Box<PgtPage> = Box::new([0; 512]);
        let s = 16 * 1024 * 1024;
        kernel_ident_mapping_init(&mut env, &mut inf, pgd.as_mut(), s, s + PMD_SIZE).unwrap();

        // pgd (shift 48) → p4d (shift 39) → pud → pmd
        let pgd_e = pgd[index_for(s, PGDIR_SHIFT_L5, 512)];
        assert!(entry_present(pgd_e));
        assert_ne!(pgd_e & PAGE_NOPTISHADOW, 0);
        let p4d_page = unsafe { &*env.va(pgd_e & PTE_PFN_MASK) };
        let p4d_e = p4d_page[index_for(s, P4D_SHIFT, 512)];
        assert!(entry_present(p4d_e));
        let pud_page = unsafe { &*env.va(p4d_e & PTE_PFN_MASK) };
        let pud_e = pud_page[index_for(s, PUD_SHIFT, 512)];
        let pmd_page = unsafe { &*env.va(pud_e & PTE_PFN_MASK) };
        let pmd_e = pmd_page[index_for(s, PMD_SHIFT, 512)];
        assert_eq!(pmd_e, s | LARGE_EXEC);
    }
}
