//! linux-parity: complete
//! linux-source: vendor/linux/mm/debug.c
//! test-origin: linux:vendor/linux/mm/debug.c
//! MM debug helper tables and `vm_debug` page poisoning control.

pub const TRACE_PRINT_TABLE_SENTINEL: (u64, Option<&str>) = (0, None);

pub const PAGE_POISON_PATTERN: u8 = 0xff;

pub const PGTY_BUDDY: u32 = 0xf0;
pub const PGTY_OFFLINE: u32 = 0xf1;
pub const PGTY_TABLE: u32 = 0xf2;
pub const PGTY_GUARD: u32 = 0xf3;
pub const PGTY_HUGETLB: u32 = 0xf4;
pub const PGTY_SLAB: u32 = 0xf5;
pub const PGTY_ZSMALLOC: u32 = 0xf6;
pub const PGTY_UNACCEPTED: u32 = 0xf7;
pub const PGTY_LARGE_KMALLOC: u32 = 0xf8;
pub const PGTY_MAPCOUNT_UNDERFLOW: u32 = 0xff;

pub const MIGRATE_REASON_NAMES: &[&str] = &[
    "compaction",
    "memory_failure",
    "memory_hotplug",
    "syscall_or_cpuset",
    "mempolicy_mbind",
    "numa_misplaced",
    "contig_range",
    "longterm_pin",
    "demotion",
    "damon",
];

pub const PAGE_TYPE_NAMES: &[Option<&str>] = &[
    Some("buddy"),
    Some("offline"),
    Some("table"),
    Some("guard"),
    Some("hugetlb"),
    Some("slab"),
    None,
    Some("unaccepted"),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmDebugState {
    pub page_init_poisoning: bool,
}

impl Default for VmDebugState {
    fn default() -> Self {
        Self {
            page_init_poisoning: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumpPageDecision {
    pub poisoned: bool,
    pub dump_snapshot: bool,
    pub dump_owner: bool,
    pub reason_reported: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VmDebugSetupPlan {
    pub accepted: bool,
    pub page_init_poisoning: bool,
    pub warn_poisoning_disabled: bool,
    pub unknown_options: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceFlagTablesPlan {
    pub pageflag_names_terminated: bool,
    pub gfpflag_names_terminated: bool,
    pub vmaflag_names_terminated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolioTypePrefix {
    None,
    Ksm,
    Anon,
    Mapping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumpFolioPlan {
    pub mapcount_type_cleared: bool,
    pub large_head_line: bool,
    pub pincount_read: bool,
    pub memcg_line: bool,
    pub type_prefix: FolioTypePrefix,
    pub dump_mapping: bool,
    pub flags_line: bool,
    pub cma_suffix: bool,
    pub page_type_line: Option<Option<&'static str>>,
    pub raw_page_dump: bool,
    pub raw_head_dump: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumpPagePlan {
    pub snapshot_page: bool,
    pub mismatch_warning: bool,
    pub dump_folio: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumpVmaPlan {
    pub header_line: bool,
    pub per_vma_lock_refcnt_line: bool,
    pub flags_line: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumpMmPlan {
    pub identity_line: bool,
    pub layout_lines: bool,
    pub flags_line: bool,
    pub aio_line: bool,
    pub memcg_owner_field: bool,
    pub mmu_notifier_line: bool,
    pub numa_balancing_line: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DumpVmgPlan {
    pub reason_line: bool,
    pub null_state_line: bool,
    pub state_block: bool,
    pub dump_mm: bool,
    pub dump_prev: bool,
    pub dump_middle: bool,
    pub dump_next: bool,
    pub dump_vmi_tree: bool,
}

pub const fn trace_flag_tables_plan() -> TraceFlagTablesPlan {
    TraceFlagTablesPlan {
        pageflag_names_terminated: TRACE_PRINT_TABLE_SENTINEL.0 == 0,
        gfpflag_names_terminated: TRACE_PRINT_TABLE_SENTINEL.0 == 0,
        vmaflag_names_terminated: TRACE_PRINT_TABLE_SENTINEL.0 == 0,
    }
}

pub fn page_type_name(page_type: u32) -> Option<&'static str> {
    let encoded = page_type >> 24;
    let Some(index) = encoded.checked_sub(PGTY_BUDDY) else {
        return Some("unknown");
    };
    match PAGE_TYPE_NAMES.get(index as usize) {
        Some(name) => *name,
        None => Some("unknown"),
    }
}

pub fn setup_vm_debug_plan(previous: VmDebugState, arg: &str) -> VmDebugSetupPlan {
    let Some(rest) = arg.strip_prefix('=') else {
        return VmDebugSetupPlan {
            accepted: true,
            page_init_poisoning: true,
            warn_poisoning_disabled: false,
            unknown_options: 0,
        };
    };
    if rest.is_empty() {
        return VmDebugSetupPlan {
            accepted: true,
            page_init_poisoning: true,
            warn_poisoning_disabled: false,
            unknown_options: 0,
        };
    }

    let mut page_init_poisoning = false;
    let mut unknown_options = 0;
    if !rest.starts_with('-') {
        for byte in rest.bytes() {
            match byte.to_ascii_lowercase() {
                b'p' => page_init_poisoning = true,
                _ => unknown_options += 1,
            }
        }
    }

    VmDebugSetupPlan {
        accepted: true,
        page_init_poisoning,
        warn_poisoning_disabled: previous.page_init_poisoning && !page_init_poisoning,
        unknown_options,
    }
}

pub fn setup_vm_debug(state: &mut VmDebugState, arg: &str) -> bool {
    let plan = setup_vm_debug_plan(*state, arg);
    state.page_init_poisoning = plan.page_init_poisoning;
    plan.accepted
}

pub fn page_init_poison(state: VmDebugState, page: &mut [u8]) {
    if state.page_init_poisoning {
        page.fill(PAGE_POISON_PATTERN);
    }
}

pub const fn dump_page_decision(poisoned: bool, reason: Option<&str>) -> DumpPageDecision {
    DumpPageDecision {
        poisoned,
        dump_snapshot: !poisoned,
        dump_owner: true,
        reason_reported: reason.is_some(),
    }
}

pub fn dump_folio_plan(
    mapcount_is_type: bool,
    large: bool,
    has_pincount: bool,
    memcg_data: bool,
    ksm: bool,
    anon: bool,
    has_mapping: bool,
    migrate_cma: bool,
    page_has_type: bool,
    page_type: u32,
) -> DumpFolioPlan {
    let type_prefix = if ksm {
        FolioTypePrefix::Ksm
    } else if anon {
        FolioTypePrefix::Anon
    } else if has_mapping {
        FolioTypePrefix::Mapping
    } else {
        FolioTypePrefix::None
    };

    DumpFolioPlan {
        mapcount_type_cleared: mapcount_is_type,
        large_head_line: large,
        pincount_read: large && has_pincount,
        memcg_line: memcg_data,
        type_prefix,
        dump_mapping: type_prefix == FolioTypePrefix::Mapping,
        flags_line: true,
        cma_suffix: migrate_cma,
        page_type_line: if page_has_type {
            Some(page_type_name(page_type))
        } else {
            None
        },
        raw_page_dump: true,
        raw_head_dump: large,
    }
}

pub const fn dump_page_plan(snapshot_faithful: bool) -> DumpPagePlan {
    DumpPagePlan {
        snapshot_page: true,
        mismatch_warning: !snapshot_faithful,
        dump_folio: true,
    }
}

pub const fn dump_vma_plan(config_per_vma_lock: bool) -> DumpVmaPlan {
    DumpVmaPlan {
        header_line: true,
        per_vma_lock_refcnt_line: config_per_vma_lock,
        flags_line: true,
    }
}

pub const fn dump_mm_plan(
    config_aio: bool,
    config_memcg: bool,
    config_mmu_notifier: bool,
    config_numa_balancing: bool,
) -> DumpMmPlan {
    DumpMmPlan {
        identity_line: true,
        layout_lines: true,
        flags_line: true,
        aio_line: config_aio,
        memcg_owner_field: config_memcg,
        mmu_notifier_line: config_mmu_notifier,
        numa_balancing_line: config_numa_balancing,
    }
}

pub const fn dump_vmg_plan(
    has_vmg: bool,
    has_reason: bool,
    has_mm: bool,
    has_prev: bool,
    has_middle: bool,
    has_next: bool,
    has_vmi: bool,
    config_debug_vm_maple_tree: bool,
) -> DumpVmgPlan {
    DumpVmgPlan {
        reason_line: has_reason,
        null_state_line: !has_vmg,
        state_block: has_vmg,
        dump_mm: has_vmg && has_mm,
        dump_prev: has_vmg && has_prev,
        dump_middle: has_vmg && has_middle,
        dump_next: has_vmg && has_next,
        dump_vmi_tree: has_vmg && has_vmi && config_debug_vm_maple_tree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_tables_and_vm_debug_setup_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/debug.c"
        ));
        let migrate_source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/trace/events/migrate.h"
        ));
        let page_flags = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/page-flags.h"
        ));

        assert!(source.contains("const char *migrate_reason_names[MR_TYPES]"));
        assert!(source.contains("MIGRATE_REASON"));
        assert!(source.contains("const struct trace_print_flags pageflag_names[]"));
        assert!(source.contains("const struct trace_print_flags gfpflag_names[]"));
        assert!(source.contains("const struct trace_print_flags vmaflag_names[]"));
        assert!(source.contains("{0, NULL}"));
        assert!(source.contains("static const char *page_type_names[]"));
        assert!(source.contains("DEF_PAGETYPE_NAME(slab)"));
        assert!(source.contains("static void __dump_folio"));
        assert!(source.contains("if (page_mapcount_is_type(mapcount))"));
        assert!(source.contains("if (folio_test_large(folio))"));
        assert!(source.contains("if (folio_has_pincount(folio))"));
        assert!(source.contains("if (folio_test_ksm(folio))"));
        assert!(source.contains("else if (folio_test_anon(folio))"));
        assert!(source.contains("else if (mapping)"));
        assert!(source.contains("dump_mapping(mapping);"));
        assert!(source.contains("BUILD_BUG_ON(ARRAY_SIZE(pageflag_names)"));
        assert!(source.contains("is_migrate_cma_folio(folio, pfn) ? \" CMA\" : \"\""));
        assert!(source.contains("if (page_has_type(&folio->page))"));
        assert!(source.contains("page_type_name(folio->page.page_type)"));
        assert!(source.contains("print_hex_dump(KERN_WARNING, \"raw: \","));
        assert!(source.contains("print_hex_dump(KERN_WARNING, \"head: \","));
        assert!(source.contains("static void __dump_page(const struct page *page)"));
        assert!(source.contains("snapshot_page(&ps, page);"));
        assert!(source.contains("if (!snapshot_page_is_faithful(&ps))"));
        assert!(source.contains("void dump_page(const struct page *page, const char *reason)"));
        assert!(source.contains("if (PagePoisoned(page))"));
        assert!(source.contains("dump_page_owner(page);"));
        assert!(source.contains("void dump_vma(const struct vm_area_struct *vma)"));
        assert!(source.contains("void dump_mm(const struct mm_struct *mm)"));
        assert!(source.contains("void dump_vmg(const struct vma_merge_struct *vmg"));
        assert!(source.contains("if (!vmg)"));
        assert!(source.contains("if (vmg->mm)"));
        assert!(source.contains("if (vmg->prev)"));
        assert!(source.contains("if (vmg->middle)"));
        assert!(source.contains("if (vmg->next)"));
        assert!(source.contains("vma_iter_dump_tree(vmg->vmi);"));
        assert!(source.contains("static bool page_init_poisoning __read_mostly = true;"));
        assert!(source.contains("if (*str++ != '=' || !*str)"));
        assert!(source.contains("if (*str == '-')"));
        assert!(source.contains("case 'p':"));
        assert!(source.contains("vm_debug option '%c' unknown. skipped"));
        assert!(source.contains("Page struct poisoning disabled by kernel command line option"));
        assert!(source.contains("memset(page, PAGE_POISON_PATTERN, size);"));
        assert!(source.contains("void vma_iter_dump_tree(const struct vma_iterator *vmi)"));
        assert!(source.contains("mas_dump(&vmi->mas);"));
        assert!(source.contains("mt_dump(vmi->mas.tree, mt_dump_hex);"));
        assert!(migrate_source.contains("EM( MR_COMPACTION,\t\"compaction\")"));
        assert!(migrate_source.contains("EMe(MR_DAMON,\t\t\"damon\")"));
        assert!(page_flags.contains("PGTY_buddy\t\t= 0xf0"));
        assert!(page_flags.contains("PGTY_zsmalloc\t\t= 0xf6"));
        assert!(page_flags.contains("PGTY_unaccepted\t\t= 0xf7"));
        assert!(page_flags.contains("PGTY_large_kmalloc\t= 0xf8"));

        assert_eq!(MIGRATE_REASON_NAMES.len(), 10);
        assert_eq!(trace_flag_tables_plan().pageflag_names_terminated, true);
        assert_eq!(trace_flag_tables_plan().gfpflag_names_terminated, true);
        assert_eq!(trace_flag_tables_plan().vmaflag_names_terminated, true);
        assert_eq!(PAGE_TYPE_NAMES.len(), 8);
        assert_eq!(page_type_name(PGTY_BUDDY << 24), Some("buddy"));
        assert_eq!(page_type_name(PGTY_SLAB << 24), Some("slab"));
        assert_eq!(page_type_name(PGTY_ZSMALLOC << 24), None);
        assert_eq!(page_type_name(PGTY_UNACCEPTED << 24), Some("unaccepted"));
        assert_eq!(page_type_name(PGTY_LARGE_KMALLOC << 24), Some("unknown"));
        assert_eq!(
            page_type_name(PGTY_MAPCOUNT_UNDERFLOW << 24),
            Some("unknown")
        );

        let mut state = VmDebugState::default();
        let bare = setup_vm_debug_plan(state, "");
        assert!(bare.accepted);
        assert!(bare.page_init_poisoning);
        let unknown = setup_vm_debug_plan(state, "=xP");
        assert_eq!(unknown.unknown_options, 1);
        assert!(unknown.page_init_poisoning);
        assert!(setup_vm_debug(&mut state, "=-"));
        assert!(!state.page_init_poisoning);
        assert!(setup_vm_debug_plan(VmDebugState::default(), "=-").warn_poisoning_disabled);
        assert!(setup_vm_debug(&mut state, "=p"));
        assert!(state.page_init_poisoning);

        let mut bytes = [0_u8; 4];
        page_init_poison(state, &mut bytes);
        assert_eq!(bytes, [PAGE_POISON_PATTERN; 4]);
        assert_eq!(
            dump_page_decision(false, Some("reason")),
            DumpPageDecision {
                poisoned: false,
                dump_snapshot: true,
                dump_owner: true,
                reason_reported: true,
            }
        );

        let folio = dump_folio_plan(
            true,
            true,
            true,
            true,
            false,
            false,
            true,
            true,
            true,
            PGTY_ZSMALLOC << 24,
        );
        assert!(folio.mapcount_type_cleared);
        assert!(folio.large_head_line);
        assert!(folio.pincount_read);
        assert!(folio.memcg_line);
        assert_eq!(folio.type_prefix, FolioTypePrefix::Mapping);
        assert!(folio.dump_mapping);
        assert!(folio.cma_suffix);
        assert_eq!(folio.page_type_line, Some(None));
        assert!(folio.raw_page_dump);
        assert!(folio.raw_head_dump);

        let ksm_folio = dump_folio_plan(
            false,
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            true,
            PGTY_SLAB << 24,
        );
        assert_eq!(ksm_folio.type_prefix, FolioTypePrefix::Ksm);
        assert!(!ksm_folio.dump_mapping);
        assert_eq!(ksm_folio.page_type_line, Some(Some("slab")));

        assert_eq!(
            dump_page_plan(false),
            DumpPagePlan {
                snapshot_page: true,
                mismatch_warning: true,
                dump_folio: true,
            }
        );
        assert!(dump_vma_plan(true).per_vma_lock_refcnt_line);
        let mm = dump_mm_plan(true, true, true, true);
        assert!(mm.aio_line);
        assert!(mm.memcg_owner_field);
        assert!(mm.mmu_notifier_line);
        assert!(mm.numa_balancing_line);
        let vmg_null = dump_vmg_plan(false, true, true, true, true, true, true, true);
        assert!(vmg_null.reason_line);
        assert!(vmg_null.null_state_line);
        assert!(!vmg_null.dump_mm);
        let vmg_full = dump_vmg_plan(true, true, true, true, false, true, true, true);
        assert!(vmg_full.state_block);
        assert!(vmg_full.dump_mm);
        assert!(vmg_full.dump_prev);
        assert!(!vmg_full.dump_middle);
        assert!(vmg_full.dump_next);
        assert!(vmg_full.dump_vmi_tree);
    }
}
