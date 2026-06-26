//! linux-parity: complete
//! linux-source: vendor/linux/mm/bpf_memcontrol.c
//! test-origin: linux:vendor/linux/mm/bpf_memcontrol.c
//! Memory-controller BPF kfunc semantics.

extern crate alloc;

use alloc::vec::Vec;

pub const PAGE_SIZE: usize = 4096;
pub const INVALID_COUNTER_VALUE: usize = usize::MAX;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemCgroup {
    pub css_id: usize,
    pub controller_id: usize,
    pub refcount: usize,
    pub usage_pages: usize,
    pub vm_events: Vec<usize>,
    pub memory_events: Vec<usize>,
    pub page_state_bytes: Vec<usize>,
    pub stats_flushes: usize,
}

impl MemCgroup {
    pub fn new(css_id: usize, controller_id: usize) -> Self {
        Self {
            css_id,
            controller_id,
            refcount: 1,
            usage_pages: 0,
            vm_events: Vec::new(),
            memory_events: Vec::new(),
            page_state_bytes: Vec::new(),
            stats_flushes: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CgroupSubsysState {
    pub css_id: usize,
    pub controller_id: usize,
    pub cgroup_memcg_css_id: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KfuncFlag {
    Acquire,
    Release,
    RetNull,
    Rcu,
    Sleepable,
}

pub const BPF_MEMCONTROL_KFUNCS: &[(&str, &[KfuncFlag])] = &[
    (
        "bpf_get_root_mem_cgroup",
        &[KfuncFlag::Acquire, KfuncFlag::RetNull],
    ),
    (
        "bpf_get_mem_cgroup",
        &[KfuncFlag::Acquire, KfuncFlag::RetNull, KfuncFlag::Rcu],
    ),
    ("bpf_put_mem_cgroup", &[KfuncFlag::Release]),
    ("bpf_mem_cgroup_vm_events", &[]),
    ("bpf_mem_cgroup_memory_events", &[]),
    ("bpf_mem_cgroup_usage", &[]),
    ("bpf_mem_cgroup_page_state", &[]),
    ("bpf_mem_cgroup_flush_stats", &[KfuncFlag::Sleepable]),
];

pub fn bpf_get_root_mem_cgroup(
    disabled: bool,
    root: Option<&mut MemCgroup>,
) -> Option<&mut MemCgroup> {
    if disabled { None } else { root }
}

pub fn bpf_get_mem_cgroup<'a>(
    disabled: bool,
    root: Option<&MemCgroup>,
    css: CgroupSubsysState,
    candidates: &'a mut [MemCgroup],
) -> Option<&'a mut MemCgroup> {
    let root = root?;
    if disabled {
        return None;
    }

    let target_controller = root.controller_id;
    let target_css_id = if css.controller_id == target_controller {
        Some(css.css_id)
    } else {
        css.cgroup_memcg_css_id
    }?;
    candidates
        .iter_mut()
        .find(|memcg| memcg.controller_id == target_controller && memcg.css_id == target_css_id)
        .map(|memcg| {
            memcg.refcount += 1;
            memcg
        })
}

pub fn bpf_put_mem_cgroup(memcg: &mut MemCgroup) {
    memcg.refcount = memcg.refcount.saturating_sub(1);
}

pub fn bpf_mem_cgroup_vm_events(memcg: &MemCgroup, event: usize) -> usize {
    memcg
        .vm_events
        .get(event)
        .copied()
        .unwrap_or(INVALID_COUNTER_VALUE)
}

pub fn bpf_mem_cgroup_usage(memcg: &MemCgroup) -> usize {
    memcg.usage_pages.saturating_mul(PAGE_SIZE)
}

pub fn bpf_mem_cgroup_memory_events(memcg: &MemCgroup, event: usize) -> usize {
    memcg
        .memory_events
        .get(event)
        .copied()
        .unwrap_or(INVALID_COUNTER_VALUE)
}

pub fn bpf_mem_cgroup_page_state(memcg: &MemCgroup, idx: usize) -> usize {
    memcg
        .page_state_bytes
        .get(idx)
        .copied()
        .unwrap_or(INVALID_COUNTER_VALUE)
}

pub fn bpf_mem_cgroup_flush_stats(memcg: &mut MemCgroup) {
    memcg.stats_flushes += 1;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BpfMemcontrolInitReport {
    pub register_ret: i32,
    pub prog_type_unspec: bool,
    pub owner_this_module: bool,
    pub warning_emitted: bool,
}

pub const fn bpf_memcontrol_init(register_ret: i32) -> BpfMemcontrolInitReport {
    BpfMemcontrolInitReport {
        register_ret,
        prog_type_unspec: true,
        owner_this_module: true,
        warning_emitted: register_ret != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpf_memcontrol_kfuncs_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/bpf_memcontrol.c"
        ));
        let selftest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/bpf/progs/cgroup_iter_memcg.c"
        ));
        assert!(source.contains("__bpf_kfunc struct mem_cgroup *bpf_get_root_mem_cgroup(void)"));
        assert!(source.contains("if (mem_cgroup_disabled())"));
        assert!(source.contains("return root_mem_cgroup;"));
        assert!(source.contains("css = rcu_dereference_raw(cgroup->subsys[ssid]);"));
        assert!(source.contains("if (css && css_tryget(css))"));
        assert!(source.contains("css_put(&memcg->css);"));
        assert!(source.contains("return page_counter_read(&memcg->memory) * PAGE_SIZE;"));
        assert!(source.contains("return (unsigned long)-1;"));
        assert!(source.contains("mem_cgroup_flush_stats(memcg);"));
        assert!(
            source
                .contains("BTF_ID_FLAGS(func, bpf_get_root_mem_cgroup, KF_ACQUIRE | KF_RET_NULL)")
        );
        assert!(source.contains("BTF_ID_FLAGS(func, bpf_mem_cgroup_flush_stats, KF_SLEEPABLE)"));
        assert!(source.contains(".owner          = THIS_MODULE"));
        assert!(source.contains("register_btf_kfunc_id_set(BPF_PROG_TYPE_UNSPEC"));
        assert!(source.contains("pr_warn(\"error while registering bpf memcontrol kfuncs: %d\""));
        assert!(source.contains("late_initcall(bpf_memcontrol_init);"));
        assert!(selftest.contains("memcg = bpf_get_mem_cgroup(css);"));
        assert!(selftest.contains("bpf_mem_cgroup_flush_stats(memcg);"));
        assert!(selftest.contains("bpf_mem_cgroup_page_state("));
        assert!(selftest.contains("bpf_mem_cgroup_vm_events("));
        assert!(selftest.contains("bpf_put_mem_cgroup(memcg);"));

        let mut root = MemCgroup::new(1, 5);
        assert!(bpf_get_root_mem_cgroup(true, Some(&mut root)).is_none());
        assert!(bpf_get_root_mem_cgroup(false, Some(&mut root)).is_some());

        let root_snapshot = root.clone();
        let mut children = [MemCgroup::new(9, 5), MemCgroup::new(10, 7)];
        let got = bpf_get_mem_cgroup(
            false,
            Some(&root_snapshot),
            CgroupSubsysState {
                css_id: 9,
                controller_id: 5,
                cgroup_memcg_css_id: None,
            },
            &mut children,
        )
        .unwrap();
        assert_eq!(got.css_id, 9);
        assert_eq!(got.refcount, 2);
        bpf_put_mem_cgroup(got);
        assert_eq!(got.refcount, 1);

        let got_from_unified = bpf_get_mem_cgroup(
            false,
            Some(&root_snapshot),
            CgroupSubsysState {
                css_id: 99,
                controller_id: 1,
                cgroup_memcg_css_id: Some(9),
            },
            &mut children,
        )
        .unwrap();
        assert_eq!(got_from_unified.css_id, 9);
        bpf_put_mem_cgroup(got_from_unified);
        assert!(
            bpf_get_mem_cgroup(
                false,
                Some(&root_snapshot),
                CgroupSubsysState {
                    css_id: 99,
                    controller_id: 1,
                    cgroup_memcg_css_id: None,
                },
                &mut children,
            )
            .is_none()
        );

        children[0].usage_pages = 3;
        children[0].vm_events = alloc::vec![11];
        children[0].memory_events = alloc::vec![13];
        children[0].page_state_bytes = alloc::vec![17];
        assert_eq!(bpf_mem_cgroup_usage(&children[0]), 3 * PAGE_SIZE);
        assert_eq!(bpf_mem_cgroup_vm_events(&children[0], 0), 11);
        assert_eq!(bpf_mem_cgroup_memory_events(&children[0], 9), usize::MAX);
        bpf_mem_cgroup_flush_stats(&mut children[0]);
        assert_eq!(children[0].stats_flushes, 1);
        assert_eq!(BPF_MEMCONTROL_KFUNCS.len(), 8);

        assert_eq!(
            bpf_memcontrol_init(0),
            BpfMemcontrolInitReport {
                register_ret: 0,
                prog_type_unspec: true,
                owner_this_module: true,
                warning_emitted: false,
            }
        );
        assert!(bpf_memcontrol_init(-22).warning_emitted);
    }
}
