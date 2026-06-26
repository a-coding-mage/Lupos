//! linux-parity: complete
//! linux-source: vendor/linux/net/ipv4/protocol.c
//! test-origin: linux:vendor/linux/net/ipv4/protocol.c
//! IPv4 protocol and offload dispatch table registration.

use spin::Mutex;

pub const MAX_INET_PROTOS: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetProtocol {
    pub name: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NetOffload {
    pub name: &'static str,
}

static INET_PROTOS: Mutex<[Option<&'static NetProtocol>; MAX_INET_PROTOS]> =
    Mutex::new([None; MAX_INET_PROTOS]);
static INET_OFFLOADS: Mutex<[Option<&'static NetOffload>; MAX_INET_PROTOS]> =
    Mutex::new([None; MAX_INET_PROTOS]);

pub fn inet_add_protocol(prot: &'static NetProtocol, protocol: u8) -> i32 {
    let mut protos = INET_PROTOS.lock();
    let slot = &mut protos[protocol as usize];
    if slot.is_none() {
        *slot = Some(prot);
        0
    } else {
        -1
    }
}

pub fn inet_add_offload(prot: &'static NetOffload, protocol: u8) -> i32 {
    let mut offloads = INET_OFFLOADS.lock();
    let slot = &mut offloads[protocol as usize];
    if slot.is_none() {
        *slot = Some(prot);
        0
    } else {
        -1
    }
}

pub fn inet_del_protocol(prot: &'static NetProtocol, protocol: u8) -> i32 {
    let mut protos = INET_PROTOS.lock();
    let slot = &mut protos[protocol as usize];
    if slot.is_some_and(|registered| core::ptr::eq(registered, prot)) {
        *slot = None;
        0
    } else {
        -1
    }
}

pub fn inet_del_offload(prot: &'static NetOffload, protocol: u8) -> i32 {
    let mut offloads = INET_OFFLOADS.lock();
    let slot = &mut offloads[protocol as usize];
    if slot.is_some_and(|registered| core::ptr::eq(registered, prot)) {
        *slot = None;
        0
    } else {
        -1
    }
}

pub fn inet_protocol(protocol: u8) -> Option<&'static NetProtocol> {
    INET_PROTOS.lock()[protocol as usize]
}

pub fn inet_offload(protocol: u8) -> Option<&'static NetOffload> {
    INET_OFFLOADS.lock()[protocol as usize]
}

pub fn inet_clear_tables() {
    INET_PROTOS.lock().fill(None);
    INET_OFFLOADS.lock().fill(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    static UDP: NetProtocol = NetProtocol { name: "udp" };
    static TCP: NetProtocol = NetProtocol { name: "tcp" };
    static UDP_OFFLOAD: NetOffload = NetOffload {
        name: "udp-offload",
    };

    #[test]
    fn ipv4_protocol_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ipv4/protocol.c"
        ));
        assert!(source.contains("struct net_protocol __rcu *inet_protos[MAX_INET_PROTOS]"));
        assert!(source.contains("const struct net_offload __rcu *inet_offloads[MAX_INET_PROTOS]"));
        assert!(source.contains("int inet_add_protocol"));
        assert!(source.contains("!cmpxchg((const struct net_protocol **)&inet_protos[protocol]"));
        assert!(source.contains("int inet_add_offload"));
        assert!(source.contains("!cmpxchg((const struct net_offload **)&inet_offloads[protocol]"));
        assert!(source.contains("int inet_del_protocol"));
        assert!(source.contains("cmpxchg((const struct net_protocol **)&inet_protos[protocol]"));
        assert!(source.contains("synchronize_net();"));
        assert!(source.contains("int inet_del_offload"));

        assert_eq!(MAX_INET_PROTOS, 256);
    }

    #[test]
    fn add_and_delete_protocols_use_compare_exchange_semantics() {
        inet_clear_tables();
        assert_eq!(inet_add_protocol(&UDP, 17), 0);
        assert_eq!(inet_protocol(17), Some(&UDP));
        assert_eq!(inet_add_protocol(&TCP, 17), -1);
        assert_eq!(inet_del_protocol(&TCP, 17), -1);
        assert_eq!(inet_del_protocol(&UDP, 17), 0);
        assert_eq!(inet_protocol(17), None);

        assert_eq!(inet_add_offload(&UDP_OFFLOAD, 17), 0);
        assert_eq!(inet_offload(17), Some(&UDP_OFFLOAD));
        assert_eq!(inet_del_offload(&UDP_OFFLOAD, 17), 0);
        assert_eq!(inet_offload(17), None);
    }
}
