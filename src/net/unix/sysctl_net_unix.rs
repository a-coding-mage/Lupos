//! linux-parity: complete
//! linux-source: vendor/linux/net/unix/sysctl_net_unix.c
//! test-origin: linux:vendor/linux/net/unix/sysctl_net_unix.c
//! AF_UNIX per-net sysctl registration.

use crate::include::uapi::errno::ENOMEM;

pub const UNIX_SYSCTL_PATH: &str = "net/unix";
pub const MAX_DGRAM_QLEN: &str = "max_dgram_qlen";
pub const UNIX_SYSCTL_MODE: u16 = 0o644;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnixCtlTable {
    pub procname: &'static str,
    pub maxlen: usize,
    pub mode: u16,
    pub data: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnixNet {
    pub is_init_net: bool,
    pub sysctl_max_dgram_qlen: i32,
    pub ctl: Option<UnixCtlTable>,
    pub duplicated_table: bool,
}

impl UnixNet {
    pub const fn new(is_init_net: bool, sysctl_max_dgram_qlen: i32) -> Self {
        Self {
            is_init_net,
            sysctl_max_dgram_qlen,
            ctl: None,
            duplicated_table: false,
        }
    }
}

pub fn unix_sysctl_register(net: &mut UnixNet) -> Result<(), i32> {
    unix_sysctl_register_with_alloc(net, true)
}

pub fn unix_sysctl_register_with_alloc(net: &mut UnixNet, alloc_ok: bool) -> Result<(), i32> {
    if !net.is_init_net && !alloc_ok {
        return Err(ENOMEM);
    }
    net.duplicated_table = !net.is_init_net;
    net.ctl = Some(UnixCtlTable {
        procname: MAX_DGRAM_QLEN,
        maxlen: core::mem::size_of::<i32>(),
        mode: UNIX_SYSCTL_MODE,
        data: net.sysctl_max_dgram_qlen,
    });
    Ok(())
}

pub fn unix_sysctl_unregister(net: &mut UnixNet) {
    net.ctl = None;
    net.duplicated_table = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_sysctl_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/unix/sysctl_net_unix.c"
        ));
        assert!(source.contains(".procname\t= \"max_dgram_qlen\""));
        assert!(source.contains(".data\t\t= &init_net.unx.sysctl_max_dgram_qlen"));
        assert!(source.contains(".maxlen\t\t= sizeof(int)"));
        assert!(source.contains(".mode\t\t= 0644"));
        assert!(source.contains(".proc_handler\t= proc_dointvec"));
        assert!(source.contains("if (net_eq(net, &init_net))"));
        assert!(source.contains("table = kmemdup(unix_table, sizeof(unix_table), GFP_KERNEL);"));
        assert!(source.contains("table[0].data = &net->unx.sysctl_max_dgram_qlen;"));
        assert!(source.contains("register_net_sysctl_sz(net, \"net/unix\", table"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("unregister_net_sysctl_table(net->unx.ctl);"));

        let mut init_net = UnixNet::new(true, 10);
        assert_eq!(unix_sysctl_register(&mut init_net), Ok(()));
        assert_eq!(
            init_net.ctl,
            Some(UnixCtlTable {
                procname: "max_dgram_qlen",
                maxlen: core::mem::size_of::<i32>(),
                mode: 0o644,
                data: 10,
            })
        );
        assert!(!init_net.duplicated_table);
        unix_sysctl_unregister(&mut init_net);
        assert_eq!(init_net.ctl, None);

        let mut netns = UnixNet::new(false, 20);
        assert_eq!(
            unix_sysctl_register_with_alloc(&mut netns, false),
            Err(ENOMEM)
        );
        assert_eq!(unix_sysctl_register(&mut netns), Ok(()));
        assert!(netns.duplicated_table);
    }
}
