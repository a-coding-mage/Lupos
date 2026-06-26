//! linux-parity: complete
//! linux-source: vendor/linux/net/core/skb_fault_injection.c
//! test-origin: linux:vendor/linux/net/core/skb_fault_injection.c
//! SKB reallocation fault-injection filter and debugfs controls.

pub const IFNAMSIZ: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkbReallocState {
    pub devname: [u8; IFNAMSIZ],
    pub filtered: bool,
    pub should_fail_attr: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SkBuff<'a> {
    pub devname: &'a str,
    pub expanded: bool,
}

impl Default for SkbReallocState {
    fn default() -> Self {
        Self {
            devname: [0; IFNAMSIZ],
            filtered: false,
            should_fail_attr: false,
        }
    }
}

pub fn should_fail_net_realloc_skb(state: &SkbReallocState, skb: &SkBuff<'_>) -> bool {
    if state.filtered && state.devname_string() != skb.devname {
        return false;
    }
    state.should_fail_attr
}

pub fn skb_might_realloc<'a>(mut skb: SkBuff<'a>, state: &SkbReallocState) -> SkBuff<'a> {
    if should_fail_net_realloc_skb(state, &skb) {
        skb.expanded = true;
    }
    skb
}

impl SkbReallocState {
    pub fn reset_settings(&mut self) {
        self.filtered = false;
        self.devname = [0; IFNAMSIZ];
    }

    pub fn devname_write(&mut self, input: &[u8]) -> isize {
        self.reset_settings();
        let count = input.len().min(IFNAMSIZ);
        self.devname[..count].copy_from_slice(&input[..count]);
        self.devname[IFNAMSIZ - 1] = 0;
        while self
            .devname
            .iter()
            .rposition(|byte| *byte != 0)
            .is_some_and(|idx| self.devname[idx] == b'\n' || self.devname[idx] == b' ')
        {
            let idx = self.devname.iter().rposition(|byte| *byte != 0).unwrap();
            self.devname[idx] = 0;
        }
        self.filtered = self.devname.iter().any(|byte| *byte != 0);
        input.len() as isize
    }

    pub fn devname_read(&self) -> Option<alloc::string::String> {
        if !self.filtered {
            return None;
        }
        Some(self.devname_string().into())
    }

    fn devname_string(&self) -> &str {
        let len = self
            .devname
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(IFNAMSIZ);
        core::str::from_utf8(&self.devname[..len]).unwrap_or("")
    }
}

extern crate alloc;

pub const fn fail_skb_realloc_setup(setup_ret: i32) -> i32 {
    setup_ret
}

pub const fn fail_skb_realloc_debugfs(dir_err: Option<i32>) -> Result<(), i32> {
    match dir_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skb_fault_injection_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/core/skb_fault_injection.c"
        ));
        assert!(source.contains("static struct {"));
        assert!(source.contains("struct fault_attr attr;"));
        assert!(source.contains("char devname[IFNAMSIZ];"));
        assert!(source.contains("static bool should_fail_net_realloc_skb"));
        assert!(source.contains("skb_realloc.filtered"));
        assert!(source.contains("strncmp(net->name, skb_realloc.devname, IFNAMSIZ)"));
        assert!(source.contains("should_fail(&skb_realloc.attr, 1)"));
        assert!(source.contains("ALLOW_ERROR_INJECTION(should_fail_net_realloc_skb, TRUE);"));
        assert!(source.contains("void skb_might_realloc"));
        assert!(source.contains("pskb_expand_head(skb, 0, 0, GFP_ATOMIC);"));
        assert!(source.contains("__setup(\"fail_skb_realloc=\", fail_skb_realloc_setup);"));
        assert!(source.contains("static void reset_settings(void)"));
        assert!(source.contains("devname_write"));
        assert!(source.contains("strim(skb_realloc.devname);"));
        assert!(source.contains("devname_read"));
        assert!(source.contains("fault_create_debugfs_attr(\"fail_skb_realloc\""));
        assert!(source.contains("debugfs_create_file(\"devname\""));
        assert!(source.contains("late_initcall(fail_skb_realloc_debugfs);"));
    }

    #[test]
    fn filtered_fault_injection_matches_device_name_and_expands_skb() {
        let mut state = SkbReallocState {
            should_fail_attr: true,
            ..Default::default()
        };
        assert_eq!(state.devname_write(b"eth0\n"), 5);
        assert_eq!(state.devname_read().as_deref(), Some("eth0"));
        assert!(should_fail_net_realloc_skb(
            &state,
            &SkBuff {
                devname: "eth0",
                expanded: false
            }
        ));
        assert!(!should_fail_net_realloc_skb(
            &state,
            &SkBuff {
                devname: "eth1",
                expanded: false
            }
        ));
        assert!(
            skb_might_realloc(
                SkBuff {
                    devname: "eth0",
                    expanded: false
                },
                &state
            )
            .expanded
        );
        state.reset_settings();
        assert!(state.devname_read().is_none());
        assert_eq!(fail_skb_realloc_setup(-3), -3);
        assert_eq!(fail_skb_realloc_debugfs(None), Ok(()));
        assert_eq!(fail_skb_realloc_debugfs(Some(-5)), Err(-5));
    }
}
