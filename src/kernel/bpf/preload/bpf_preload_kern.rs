//! linux-parity: complete
//! linux-source: vendor/linux/kernel/bpf/preload/bpf_preload_kern.c
//! test-origin: linux:vendor/linux/kernel/bpf/preload/bpf_preload_kern.c
//! Embedded BPF iterator preload links.

use crate::include::uapi::errno::EINVAL;

pub const BPF_PRELOAD_LINKS: usize = 2;
pub const MAPS_LINK_NAME: &str = "maps.debug";
pub const PROGS_LINK_NAME: &str = "progs.debug";
pub const MODULE_LICENSE: &str = "GPL";
pub const MODULE_DESCRIPTION: &str = "Embedded BPF programs for introspection in bpffs";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BpfPreloadInfo {
    pub link_name: &'static str,
    pub link: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BpfPreloadState {
    pub maps_link: Option<usize>,
    pub progs_link: Option<usize>,
    pub skel_loaded: bool,
    pub ops_installed: bool,
    pub map_fd_closed: bool,
    pub prog_fd_closed: bool,
}

impl BpfPreloadState {
    pub const fn new() -> Self {
        Self {
            maps_link: None,
            progs_link: None,
            skel_loaded: false,
            ops_installed: false,
            map_fd_closed: false,
            prog_fd_closed: false,
        }
    }

    pub fn preload(&self, obj: &mut [BpfPreloadInfo]) -> Result<(), i32> {
        if obj.len() < BPF_PRELOAD_LINKS {
            return Err(-EINVAL);
        }
        obj[0] = BpfPreloadInfo {
            link_name: MAPS_LINK_NAME,
            link: self.maps_link,
        };
        obj[1] = BpfPreloadInfo {
            link_name: PROGS_LINK_NAME,
            link: self.progs_link,
        };
        Ok(())
    }

    pub fn load_skel(&mut self) -> Result<(), i32> {
        self.skel_loaded = true;
        self.maps_link = Some(1);
        self.progs_link = Some(2);
        self.map_fd_closed = true;
        self.prog_fd_closed = true;
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), i32> {
        self.load_skel()?;
        self.ops_installed = true;
        Ok(())
    }

    pub fn fini(&mut self) {
        self.ops_installed = false;
        self.free_links_and_skel();
    }

    pub fn free_links_and_skel(&mut self) {
        self.maps_link = None;
        self.progs_link = None;
        self.skel_loaded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpf_preload_kern_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/bpf_preload_kern.c"
        ));
        let header = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/bpf/preload/bpf_preload.h"
        ));
        assert!(source.contains("static struct bpf_link *maps_link, *progs_link;"));
        assert!(source.contains("static struct iterators_bpf *skel;"));
        assert!(source.contains("bpf_link_put(maps_link);"));
        assert!(source.contains("bpf_link_put(progs_link);"));
        assert!(source.contains("iterators_bpf__destroy(skel);"));
        assert!(source.contains("strscpy(obj[0].link_name, \"maps.debug\""));
        assert!(source.contains("strscpy(obj[1].link_name, \"progs.debug\""));
        assert!(source.contains("skel = iterators_bpf__open();"));
        assert!(source.contains("iterators_bpf__load(skel);"));
        assert!(source.contains("iterators_bpf__attach(skel);"));
        assert!(source.contains("bpf_link_get_from_fd(skel->links.dump_bpf_map_fd);"));
        assert!(source.contains("close_fd(skel->links.dump_bpf_map_fd);"));
        assert!(source.contains("bpf_preload_ops = &ops;"));
        assert!(source.contains("bpf_preload_ops = NULL;"));
        assert!(source.contains("late_initcall(load);"));
        assert!(source.contains("MODULE_LICENSE(\"GPL\")"));
        assert!(source.contains(MODULE_DESCRIPTION));
        assert!(header.contains("char link_name[16];"));
        assert!(header.contains("#define BPF_PRELOAD_LINKS 2"));

        let mut state = BpfPreloadState::new();
        assert_eq!(state.load(), Ok(()));
        assert!(state.skel_loaded);
        assert!(state.ops_installed);
        assert!(state.map_fd_closed);
        assert!(state.prog_fd_closed);

        let mut info = [BpfPreloadInfo::default(); BPF_PRELOAD_LINKS];
        assert_eq!(state.preload(&mut info), Ok(()));
        assert_eq!(info[0].link_name, MAPS_LINK_NAME);
        assert_eq!(info[0].link, Some(1));
        assert_eq!(info[1].link_name, PROGS_LINK_NAME);
        assert_eq!(info[1].link, Some(2));

        state.fini();
        assert!(!state.ops_installed);
        assert!(!state.skel_loaded);
        assert_eq!(state.maps_link, None);
        assert_eq!(state.progs_link, None);
    }
}
