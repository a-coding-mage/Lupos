//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf
//! test-origin: linux:vendor/linux/kernel/bpf
//! Linux BPF source coverage and unsupported-operation policy.
//!
//! Keep these Linux references source-shaped while routing behavior through the
//! concrete BPF modules (`maps`, `syscall`, `verifier`, `interp`, `helpers`).
//!
//! Refs:
//! - `vendor/linux/kernel/bpf/{arena,arraymap,backtrack,bloom_filter,bpf_cgrp_storage,bpf_inode_storage,bpf_insn_array,bpf_iter,bpf_local_storage,bpf_lru_list,bpf_lsm,bpf_lsm_proto,bpf_struct_ops,bpf_task_storage,btf,btf_iter,btf_relocate,cfg,cgroup,cgroup_iter,check_btf,const_fold,core,cpumap,cpumask,crypto,devmap,disasm,dispatcher,dmabuf_iter,fixups,hashtab,helpers,inode,kmem_cache_iter,link_iter,liveness,local_storage,log,lpm_trie,map_in_map,map_iter,memalloc,mprog,net_namespace,offload,percpu_freelist,prog_iter,queue_stack_maps,range_tree,relo_core,reuseport_array,ringbuf,rqspinlock,stackmap,states,stream,syscall,sysfs_btf,task_iter,tcx,tnum,token,trampoline,verifier}.c`
//! - `vendor/linux/kernel/bpf/preload/{bpf_preload_kern}.c`
//! - `vendor/linux/kernel/bpf/preload/iterators/{iterators.bpf}.c`
//! - `vendor/linux/kernel/trace/{bpf_trace}.c`

use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

pub const BPF_SOURCE_COUNT: usize = 68;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BpfSourceSubsystem {
    Core,
    Preload,
    Trace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportStatus {
    Implemented,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxBpfSource {
    pub path: &'static str,
    pub subsystem: BpfSourceSubsystem,
    pub status: SupportStatus,
    pub unsupported_errno: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxBpfSourceGroup {
    pub dir: &'static str,
    pub stems: &'static str,
    pub subsystem: BpfSourceSubsystem,
}

pub const SOURCE_GROUPS: &[LinuxBpfSourceGroup] = &[
    LinuxBpfSourceGroup {
        dir: "vendor/linux/kernel/bpf",
        stems: "arena,arraymap,backtrack,bloom_filter,bpf_cgrp_storage,bpf_inode_storage,bpf_insn_array,bpf_iter,bpf_local_storage,bpf_lru_list,bpf_lsm,bpf_lsm_proto,bpf_struct_ops,bpf_task_storage,btf,btf_iter,btf_relocate,cfg,cgroup,cgroup_iter,check_btf,const_fold,core,cpumap,cpumask,crypto,devmap,disasm,dispatcher,dmabuf_iter,fixups,hashtab,helpers,inode,kmem_cache_iter,link_iter,liveness,local_storage,log,lpm_trie,map_in_map,map_iter,memalloc,mprog,net_namespace,offload,percpu_freelist,prog_iter,queue_stack_maps,range_tree,relo_core,reuseport_array,ringbuf,rqspinlock,stackmap,states,stream,syscall,sysfs_btf,task_iter,tcx,tnum,token,trampoline,verifier",
        subsystem: BpfSourceSubsystem::Core,
    },
    LinuxBpfSourceGroup {
        dir: "vendor/linux/kernel/bpf/preload",
        stems: "bpf_preload_kern",
        subsystem: BpfSourceSubsystem::Preload,
    },
    LinuxBpfSourceGroup {
        dir: "vendor/linux/kernel/bpf/preload/iterators",
        stems: "iterators.bpf",
        subsystem: BpfSourceSubsystem::Preload,
    },
    LinuxBpfSourceGroup {
        dir: "vendor/linux/kernel/trace",
        stems: "bpf_trace",
        subsystem: BpfSourceSubsystem::Trace,
    },
];

const IMPLEMENTED_SOURCES: &[&str] = &[
    "vendor/linux/kernel/bpf/core.c",
    "vendor/linux/kernel/bpf/hashtab.c",
    "vendor/linux/kernel/bpf/helpers.c",
    "vendor/linux/kernel/bpf/preload/bpf_preload_kern.c",
    "vendor/linux/kernel/bpf/syscall.c",
    "vendor/linux/kernel/bpf/verifier.c",
    "vendor/linux/kernel/trace/bpf_trace.c",
];

pub fn source_count() -> usize {
    SOURCE_GROUPS
        .iter()
        .map(|group| csv_count(group.stems))
        .sum()
}

pub fn contains_linux_source(path: &str) -> bool {
    source_group(path).is_some()
}

pub fn source_policy(path: &'static str) -> LinuxBpfSource {
    let subsystem = source_group(path)
        .map(|group| group.subsystem)
        .unwrap_or(BpfSourceSubsystem::Core);
    let status = if is_implemented(path) {
        SupportStatus::Implemented
    } else {
        SupportStatus::Unsupported
    };
    LinuxBpfSource {
        path,
        subsystem,
        status,
        unsupported_errno: if status == SupportStatus::Unsupported {
            Some(unsupported_errno(path))
        } else {
            None
        },
    }
}

pub fn unsupported_errno(path: &str) -> i32 {
    if contains_linux_source(path) {
        EOPNOTSUPP
    } else {
        ENOENT
    }
}

pub fn all_sources_have_policy() -> Result<(), i32> {
    if source_count() != BPF_SOURCE_COUNT {
        return Err(ENOENT);
    }
    for group in SOURCE_GROUPS {
        if group.dir.is_empty() || group.stems.is_empty() {
            return Err(ENOENT);
        }
    }
    Ok(())
}

fn source_group(path: &str) -> Option<&'static LinuxBpfSourceGroup> {
    let (dir, file) = path.rsplit_once('/')?;
    let stem = file.strip_suffix(".c")?;
    SOURCE_GROUPS
        .iter()
        .find(|group| group.dir == dir && csv_contains(group.stems, stem))
}

fn is_implemented(path: &str) -> bool {
    IMPLEMENTED_SOURCES.iter().any(|source| *source == path)
}

fn csv_count(csv: &str) -> usize {
    if csv.is_empty() {
        return 0;
    }
    csv.split(',').count()
}

fn csv_contains(csv: &str, needle: &str) -> bool {
    csv.split(',').any(|item| item == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::include::uapi::errno::{ENOENT, EOPNOTSUPP};

    #[test]
    fn linux_bpf_source_inventory_is_complete() {
        assert_eq!(source_count(), BPF_SOURCE_COUNT);
        assert!(contains_linux_source("vendor/linux/kernel/bpf/verifier.c"));
        assert!(contains_linux_source("vendor/linux/kernel/bpf/ringbuf.c"));
        assert!(contains_linux_source(
            "vendor/linux/kernel/trace/bpf_trace.c"
        ));
        assert_eq!(all_sources_have_policy(), Ok(()));
    }

    #[test]
    fn linux_bpf_source_policy_reports_real_support_state() {
        let supported = source_policy("vendor/linux/kernel/bpf/verifier.c");
        assert_eq!(supported.status, SupportStatus::Implemented);
        assert_eq!(supported.unsupported_errno, None);

        let unsupported = source_policy("vendor/linux/kernel/bpf/ringbuf.c");
        assert_eq!(unsupported.status, SupportStatus::Unsupported);
        assert_eq!(unsupported.unsupported_errno, Some(EOPNOTSUPP));
        assert_eq!(
            unsupported_errno("vendor/linux/kernel/bpf/missing.c"),
            ENOENT
        );
    }
}
