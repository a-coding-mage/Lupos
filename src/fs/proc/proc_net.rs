//! linux-parity: partial
//! linux-source: vendor/linux/fs/proc/proc_net.c
//! `/proc/net`.
//!
//! Ref: `vendor/linux/fs/proc/proc_net.c`

use alloc::sync::Arc;

use crate::fs::kernfs::{KernfsNode, add_child};

fn dev_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    super::util::copy_into(
        buf,
        "Inter-|   Receive                                                |  Transmit\n",
    )
}

pub fn new_net_dir() -> Arc<KernfsNode> {
    let dir = KernfsNode::new_dir("net", 0o555);
    add_child(
        &dir,
        KernfsNode::new_file("dev", 0o444, Some(dev_show), None),
    );
    dir
}
