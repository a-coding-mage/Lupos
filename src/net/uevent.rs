//! linux-parity: complete
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! `NETLINK_KOBJECT_UEVENT` message builder + broadcast queue.
//!
//! libudev (and Wayland's libinput, by extension) discovers devices by
//! listening on a `NETLINK_KOBJECT_UEVENT` socket and parsing `add@/devices/…`
//! style messages.  This module produces those messages in the Linux wire
//! format and buffers them for any future netlink socket consumer.
//!
//! References:
//!   - `vendor/linux/lib/kobject_uevent.c`
//!   - `vendor/linux/include/linux/kobject.h`

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

/// Action portion of the message header — `enum kobject_action`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UeventAction {
    Add,
    Remove,
    Change,
    Online,
    Offline,
    Move,
    Bind,
    Unbind,
}

impl UeventAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Change => "change",
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Move => "move",
            Self::Bind => "bind",
            Self::Unbind => "unbind",
        }
    }
}

/// One uevent broadcast — a sequence of NUL-separated key=value records, with
/// the first record being the legacy `ACTION@DEVPATH` header that udev still
/// parses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UeventMessage {
    pub payload: Vec<u8>,
}

impl UeventMessage {
    /// Build a Linux-format uevent broadcast.  `devpath` is the `/sys`-relative
    /// kobject path (e.g. `/devices/platform/i8042/serio0/input/input0/event0`).
    /// `properties` is appended as `KEY=value` records.
    pub fn build(action: UeventAction, devpath: &str, properties: &[(&str, &str)]) -> Self {
        let mut payload = Vec::with_capacity(128);
        // Legacy header — `ACTION@DEVPATH\0`.
        payload.extend_from_slice(action.as_str().as_bytes());
        payload.push(b'@');
        payload.extend_from_slice(devpath.as_bytes());
        payload.push(0);

        push_record(&mut payload, "ACTION", action.as_str());
        push_record(&mut payload, "DEVPATH", devpath);
        for (k, v) in properties {
            push_record(&mut payload, k, v);
        }
        Self { payload }
    }
}

fn push_record(buf: &mut Vec<u8>, key: &str, value: &str) {
    buf.extend_from_slice(key.as_bytes());
    buf.push(b'=');
    buf.extend_from_slice(value.as_bytes());
    buf.push(0);
}

lazy_static! {
    /// Pending broadcasts waiting for a netlink listener to drain them.
    /// Capacity is bounded — overflow drops the oldest message to mirror
    /// Linux's `uevent_net_rcv_skb()` overflow path.
    static ref BROADCAST_QUEUE: Mutex<VecDeque<UeventMessage>> = Mutex::new(VecDeque::new());
}

#[cfg(test)]
lazy_static! {
    static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

/// Soft cap so a chatty bring-up doesn't unboundedly grow heap usage before
/// a listener attaches.
const QUEUE_HIGH_WATERMARK: usize = 1024;

/// Global uevent sequence counter, mirrored at `/sys/kernel/uevent_seqnum`.
/// Ref: `vendor/linux/lib/kobject_uevent.c::uevent_seqnum` — an atomic
/// 64-bit counter incremented on every `kobject_uevent_env()` broadcast.
/// libudev's `udev_monitor` and systemd's
/// `vendor/systemd/systemd-260.1/src/libsystemd/sd-device/device-monitor.c`
/// trust the file to be a monotonically non-decreasing integer + newline.
static UEVENT_SEQNUM: AtomicU64 = AtomicU64::new(0);

/// Read the current uevent sequence number without advancing it.  Used by
/// the `/sys/kernel/uevent_seqnum` show callback.
pub fn current_seqnum() -> u64 {
    UEVENT_SEQNUM.load(Ordering::Acquire)
}

/// Broadcast a uevent to every listener.  Until the AF_NETLINK socket is
/// wired into the syscall layer for this family, this stores the message in
/// the broadcast queue so a future listener can replay history.  Bumps
/// `UEVENT_SEQNUM` to match Linux's `kobject_uevent_env` semantics.
pub fn broadcast_uevent(msg: UeventMessage) {
    let mut msg = msg;
    let seqnum = UEVENT_SEQNUM.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
    push_record(&mut msg.payload, "SEQNUM", &alloc::format!("{seqnum}"));
    {
        let mut q = BROADCAST_QUEUE.lock();
        if q.len() >= QUEUE_HIGH_WATERMARK {
            q.pop_front();
        }
        q.push_back(msg.clone());
    }
    crate::net::socket::broadcast_kobject_uevent(&msg.payload);
}

/// Drain all pending uevent messages.  Used by tests and by any in-kernel
/// consumer (e.g. a future uevent socket file_operations).
pub fn drain_pending() -> Vec<UeventMessage> {
    let mut q = BROADCAST_QUEUE.lock();
    q.drain(..).collect()
}

/// Snapshot pending broadcasts without consuming them. New netlink listeners
/// use this to replay device events emitted before userspace opened its udev
/// monitor socket.
pub fn pending_snapshot() -> Vec<UeventMessage> {
    BROADCAST_QUEUE.lock().iter().cloned().collect()
}

#[cfg(test)]
pub fn test_lock() -> spin::MutexGuard<'static, ()> {
    TEST_LOCK.lock()
}

/// Convenience wrapper for the common "device added to /sys/class/<class>/"
/// case used during rootfs bootstrap.
pub fn announce_class_device(class: &str, name: &str, subsystem: &str, devname: &str) {
    let devpath = devpath_for(class, name);
    let major_minor = ("0", "0");
    let msg = UeventMessage::build(
        UeventAction::Add,
        &devpath,
        &[
            ("SUBSYSTEM", subsystem),
            ("DEVNAME", devname),
            ("MAJOR", major_minor.0),
            ("MINOR", major_minor.1),
        ],
    );
    broadcast_uevent(msg);
}

pub fn announce_netdevice(action: UeventAction, ifname: &str, ifindex: u32) {
    let devpath = alloc::format!("/devices/virtual/net/{ifname}");
    let ifindex = alloc::format!("{ifindex}");
    let msg = UeventMessage::build(
        action,
        &devpath,
        &[
            ("SUBSYSTEM", "net"),
            ("INTERFACE", ifname),
            ("IFINDEX", &ifindex),
        ],
    );
    broadcast_uevent(msg);
}

fn devpath_for(class: &str, name: &str) -> String {
    let mut s = String::from("/class/");
    s.push_str(class);
    s.push('/');
    s.push_str(name);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests that mutate the global broadcast queue must serialize against each
    // other — cargo's default test runner is parallel, and a partial drain in
    // one test would otherwise be visible to another.
    #[test]
    fn message_header_uses_action_at_devpath_legacy_form() {
        let msg = UeventMessage::build(
            UeventAction::Add,
            "/devices/platform/i8042/serio0/input/input0/event0",
            &[("SUBSYSTEM", "input"), ("DEVNAME", "input/event0")],
        );
        assert!(msg.payload.starts_with(b"add@/devices/platform/i8042"));
        assert!(msg.payload.windows(8).any(|w| w == b"ACTION=a"));
        assert!(msg.payload.windows(15).any(|w| w == b"SUBSYSTEM=input"));
    }

    #[test]
    fn broadcast_queue_round_trips_messages_in_order() {
        let _guard = test_lock();
        let _ = drain_pending();
        announce_class_device("input", "event0", "input", "input/event0");
        announce_class_device("graphics", "fb0", "graphics", "fb0");

        let drained = drain_pending();
        assert_eq!(drained.len(), 2);
        assert!(drained[0].payload.starts_with(b"add@/class/input/event0"));
        assert!(drained[1].payload.starts_with(b"add@/class/graphics/fb0"));
    }

    #[test]
    fn netdevice_uevent_uses_linux_virtual_net_devpath() {
        let _guard = test_lock();
        let _ = drain_pending();
        announce_netdevice(UeventAction::Add, "eth0", 2);
        let drained = drain_pending();
        assert_eq!(drained.len(), 1);
        let payload = &drained[0].payload;
        assert!(payload.starts_with(b"add@/devices/virtual/net/eth0\0"));
        assert!(
            payload
                .windows(b"SUBSYSTEM=net\0".len())
                .any(|w| w == b"SUBSYSTEM=net\0")
        );
        assert!(
            payload
                .windows(b"INTERFACE=eth0\0".len())
                .any(|w| w == b"INTERFACE=eth0\0")
        );
        assert!(
            payload
                .windows(b"IFINDEX=2\0".len())
                .any(|w| w == b"IFINDEX=2\0")
        );
        assert!(payload.windows(b"SEQNUM=".len()).any(|w| w == b"SEQNUM="));
    }

    #[test]
    fn broadcast_queue_high_watermark_drops_oldest() {
        let _guard = test_lock();
        let _ = drain_pending();
        for i in 0..(QUEUE_HIGH_WATERMARK + 5) {
            let msg = UeventMessage::build(
                UeventAction::Add,
                "/devices/test",
                &[("SEQ", &alloc::format!("{i}"))],
            );
            broadcast_uevent(msg);
        }
        let drained = drain_pending();
        assert_eq!(drained.len(), QUEUE_HIGH_WATERMARK);
        // Oldest five should be gone — first remaining message has SEQ=5.
        assert!(
            drained[0].payload.windows(5).any(|w| w == b"SEQ=5"),
            "first surviving message should be SEQ=5"
        );
    }
}
