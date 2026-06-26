//! linux-parity: partial
//! linux-source: vendor/linux/drivers/net/virtio_net.c
//! virtio-net Linux module handoff.
//!
//! The network core may expose the Linux-visible registration hooks that a
//! Linux-built virtio-net module uses, but it must not synthesize a local Rust
//! virtio-net device. Loading `virtio_net` therefore requires a real `.ko`
//! produced from `vendor/linux/drivers/net/virtio_net.c`.

pub const VIRTIO_NET_VENDOR_SOURCE: &str = "vendor/linux/drivers/net/virtio_net.c";
