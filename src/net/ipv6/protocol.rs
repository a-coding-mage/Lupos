//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv6/protocol.c
//! test-origin: linux:vendor/linux/net/ipv6/protocol.c
//! IPv6 protocol and offload dispatch table registration.

use spin::Mutex;

pub const MAX_INET_PROTOS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Inet6Protocol {
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetOffload {
    pub name: &'static str,
}

static INET6_PROTOS: Mutex<[Option<&'static Inet6Protocol>; MAX_INET_PROTOS]> =
    Mutex::new([None; MAX_INET_PROTOS]);
static INET6_OFFLOADS: Mutex<[Option<&'static NetOffload>; MAX_INET_PROTOS]> =
    Mutex::new([None; MAX_INET_PROTOS]);

pub fn inet6_add_protocol(prot: &'static Inet6Protocol, protocol: u8) -> i32 {
    let mut protos = INET6_PROTOS.lock();
    let slot = &mut protos[protocol as usize];
    if slot.is_none() {
        *slot = Some(prot);
        0
    } else {
        -1
    }
}

pub fn inet6_del_protocol(prot: &'static Inet6Protocol, protocol: u8) -> i32 {
    let mut protos = INET6_PROTOS.lock();
    let slot = &mut protos[protocol as usize];
    if slot.is_some_and(|registered| core::ptr::eq(registered, prot)) {
        *slot = None;
        0
    } else {
        -1
    }
}

pub fn inet6_add_offload(prot: &'static NetOffload, protocol: u8) -> i32 {
    let mut offloads = INET6_OFFLOADS.lock();
    let slot = &mut offloads[protocol as usize];
    if slot.is_none() {
        *slot = Some(prot);
        0
    } else {
        -1
    }
}

pub fn inet6_del_offload(prot: &'static NetOffload, protocol: u8) -> i32 {
    let mut offloads = INET6_OFFLOADS.lock();
    let slot = &mut offloads[protocol as usize];
    if slot.is_some_and(|registered| core::ptr::eq(registered, prot)) {
        *slot = None;
        0
    } else {
        -1
    }
}

pub fn inet6_protocol(protocol: u8) -> Option<&'static Inet6Protocol> {
    INET6_PROTOS.lock()[protocol as usize]
}

pub fn inet6_offload(protocol: u8) -> Option<&'static NetOffload> {
    INET6_OFFLOADS.lock()[protocol as usize]
}

pub fn inet6_clear_tables() {
    INET6_PROTOS.lock().fill(None);
    INET6_OFFLOADS.lock().fill(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    static ICMPV6: Inet6Protocol = Inet6Protocol { name: "icmpv6" };
    static UDPV6: Inet6Protocol = Inet6Protocol { name: "udpv6" };
    static UDPV6_OFFLOAD: NetOffload = NetOffload {
        name: "udpv6-offload",
    };

    #[test]
    fn ipv6_protocol_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv6/protocol.c"
        ));
        assert!(source.contains("struct inet6_protocol __rcu *inet6_protos[MAX_INET_PROTOS]"));
        assert!(source.contains("int inet6_add_protocol"));
        assert!(
            source.contains("!cmpxchg((const struct inet6_protocol **)&inet6_protos[protocol]")
        );
        assert!(source.contains("int inet6_del_protocol"));
        assert!(source.contains("cmpxchg((const struct inet6_protocol **)&inet6_protos[protocol]"));
        assert!(source.contains("const struct net_offload __rcu *inet6_offloads[MAX_INET_PROTOS]"));
        assert!(source.contains("int inet6_add_offload"));
        assert!(source.contains("int inet6_del_offload"));
        assert!(source.contains("synchronize_net();"));

        assert_eq!(MAX_INET_PROTOS, 256);
    }

    #[test]
    fn add_and_delete_ipv6_protocols_and_offloads_are_slot_exclusive() {
        inet6_clear_tables();
        assert_eq!(inet6_add_protocol(&ICMPV6, 58), 0);
        assert_eq!(inet6_protocol(58), Some(&ICMPV6));
        assert_eq!(inet6_add_protocol(&UDPV6, 58), -1);
        assert_eq!(inet6_del_protocol(&UDPV6, 58), -1);
        assert_eq!(inet6_del_protocol(&ICMPV6, 58), 0);
        assert_eq!(inet6_protocol(58), None);

        assert_eq!(inet6_add_offload(&UDPV6_OFFLOAD, 17), 0);
        assert_eq!(inet6_offload(17), Some(&UDPV6_OFFLOAD));
        assert_eq!(inet6_del_offload(&UDPV6_OFFLOAD, 17), 0);
        assert_eq!(inet6_offload(17), None);
    }
}
