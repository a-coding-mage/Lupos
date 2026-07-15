//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/proc_net.c
//! `/proc/net`.
//!
//! Ref: `vendor/linux/fs/proc/proc_net.c`

use alloc::sync::Arc;
use core::fmt::Write;

use crate::fs::kernfs::{KernfsNode, add_child};

fn dev_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let mut text = alloc::string::String::from(
        "Inter-|   Receive                                                |  Transmit\n",
    );
    text.push_str(
        " face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed\n",
    );
    for dev in crate::net::device::list_netdevices() {
        let stats = dev.stats();
        let _ = writeln!(
            text,
            "{:>6}: {:>8} {:>7} {:>4} {:>4} {:>4} {:>5} {:>10} {:>9} {:>8} {:>7} {:>4} {:>4} {:>4} {:>5} {:>7} {:>10}",
            dev.name, 0, stats.rx_packets, 0, 0, 0, 0, 0, 0, 0, stats.tx_packets, 0, 0, 0, 0, 0, 0
        );
    }
    super::util::copy_into(buf, &text)
}

pub fn new_net_dir() -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir("net", 0o555);
    add_child(
        &dir,
        KernfsNode::new_file("dev", 0o444, Some(dev_show), None),
    );
    dir
}
