//! linux-parity: partial
//! linux-source: vendor/linux/net
//! test-origin: linux:vendor/linux/net
//! Niche IPv6/MPLS registration surfaces used during Linux networking init.
//!
//! This mirrors the init-time shape of:
//! - `vendor/linux/net/mpls/mpls_gso.c`
//! - `vendor/linux/net/ipv6/ioam6.c`
//! - `vendor/linux/net/ipv6/ioam6_iptunnel.c`
//! - `vendor/linux/net/ipv6/mip6.c`
//!
//! This module records the registration contracts and boot banners, then runs
//! the packet paths through Lupos `SkBuff`s: MPLS GSO segments MAC/MPLS frames,
//! IOAM6 inserts a Hop-by-Hop option and emits trace events, and MIP6 xfrm plus
//! raw mobility-header filtering emit IPv6/ICMPv6 packets.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{E2BIG, EEXIST, EINVAL, ENOENT, ENOMEM};
use crate::net::ip::{build_ipv6_packet, checksum, parse_ipv6_packet};
use crate::net::skbuff::{SkBuff, alloc_skb, skb_put};
use crate::net::socket::AF_INET6;

pub const ETH_P_MPLS_UC: u16 = 0x8847;
pub const ETH_P_MPLS_MC: u16 = 0x8848;
pub const ETH_P_IP: u16 = 0x0800;
pub const ETH_P_IPV6: u16 = 0x86dd;
pub const MPLS_HLEN: usize = 4;
pub const MPLS_GSO_MAX_SEGMENTS: usize = 128;
pub const IPPROTO_DSTOPTS: u8 = 60;
pub const IPPROTO_ROUTING: u8 = 43;
pub const IPPROTO_MH: u8 = 135;
pub const IPPROTO_NONE: u8 = 59;
pub const IPPROTO_HOPOPTS: u8 = 0;
pub const IPPROTO_ICMPV6: u8 = 58;
pub const IPV6_HEADER_LEN: usize = 40;
pub const ICMPV6_PARAMPROB: u8 = 4;
pub const ICMPV6_HDR_FIELD: u8 = 0;
pub const LWTUNNEL_ENCAP_IOAM6: u16 = 9;
pub const IOAM6_GENL_NAME: &str = "IOAM6";
pub const IOAM6_GENL_VERSION: u8 = 0x1;
pub const IOAM6_GENL_EV_GRP_NAME: &str = "ioam6_events";
pub const IOAM6_MAX_SCHEMA_DATA_LEN: usize = 255 * 4;
pub const IOAM6_TRACE_DATA_SIZE_MAX: usize = 244;
pub const IOAM6_U32_UNAVAILABLE: u32 = u32::MAX;
pub const IOAM6_U64_UNAVAILABLE: u64 = u64::MAX;
pub const IOAM6_TYPE_PREALLOC: u8 = 0;
pub const IPV6_TLV_IOAM: u8 = 49;

pub const IOAM6_ATTR_NS_ID: u16 = 1;
pub const IOAM6_ATTR_NS_DATA: u16 = 2;
pub const IOAM6_ATTR_NS_DATA_WIDE: u16 = 3;
pub const IOAM6_ATTR_SC_ID: u16 = 4;
pub const IOAM6_ATTR_SC_DATA: u16 = 5;
pub const IOAM6_ATTR_SC_NONE: u16 = 6;

pub const IOAM6_CMD_ADD_NAMESPACE: u8 = 1;
pub const IOAM6_CMD_DEL_NAMESPACE: u8 = 2;
pub const IOAM6_CMD_DUMP_NAMESPACES: u8 = 3;
pub const IOAM6_CMD_ADD_SCHEMA: u8 = 4;
pub const IOAM6_CMD_DEL_SCHEMA: u8 = 5;
pub const IOAM6_CMD_DUMP_SCHEMAS: u8 = 6;
pub const IOAM6_CMD_NS_SET_SCHEMA: u8 = 7;
pub const IOAM6_EVENT_TRACE_FILLED: &str = "trace_filled";
pub const IOAM6_MASK_SHORT_FIELDS: u32 = 0xff1f_fc00;
pub const IOAM6_MASK_WIDE_FIELDS: u32 = 0x00e0_0000;
pub const IOAM6_TRACE_FORBIDDEN_LWT_BITS: u32 = 0x000f_fd00;
pub const IOAM6_IPTUNNEL_FREQ_MIN: u32 = 1;
pub const IOAM6_IPTUNNEL_FREQ_MAX: u32 = 1_000_000;
pub const IOAM6_IPTUNNEL_MODE_INLINE: u8 = 1;
pub const IOAM6_IPTUNNEL_MODE_ENCAP: u8 = 2;
pub const IOAM6_IPTUNNEL_MODE_AUTO: u8 = 3;
pub const IOAM6_HDR_LEN: usize = 4;
pub const IOAM6_TRACE_HDR_LEN: usize = 8;
pub const IOAM6_LWT_ENCAP_BASE_LEN: usize = 16;
pub const LWTUNNEL_STATE_OUTPUT_REDIRECT: u32 = 1;
pub const IPV6_TLV_PAD1: u8 = 0;
pub const IPV6_TLV_PADN: u8 = 1;
pub const IPV6_TLV_HAO: u8 = 201;
pub const IPV6_SRCRT_TYPE_2: u8 = 2;
pub const MIP6_DESTOPT_HEADER_LEN: usize = 24;
pub const MIP6_RTHDR_HEADER_LEN: usize = 24;
pub const MIP6_MH_BASE_LEN: usize = 6;
pub const MIP6_MH_PROTO_OFFSET: usize = 0;
pub const MIP6_MH_HDRLEN_OFFSET: usize = 1;
pub const IP6_MH_TYPE_BRR: u8 = 0;
pub const IP6_MH_TYPE_HOTI: u8 = 1;
pub const IP6_MH_TYPE_COTI: u8 = 2;
pub const IP6_MH_TYPE_HOT: u8 = 3;
pub const IP6_MH_TYPE_COT: u8 = 4;
pub const IP6_MH_TYPE_BU: u8 = 5;
pub const IP6_MH_TYPE_BACK: u8 = 6;
pub const IP6_MH_TYPE_BERROR: u8 = 7;

pub const NICHE_NET_BOOT_LOGS: [&str; 3] = [
    "MPLS GSO support",
    "In-situ OAM (IOAM) with IPv6",
    "Mobile IPv6",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketOffloadRegistration {
    pub ethertype: u16,
    pub priority: u8,
    pub gso_segment: &'static str,
}

pub const MPLS_GSO_OFFLOADS: [PacketOffloadRegistration; 2] = [
    PacketOffloadRegistration {
        ethertype: ETH_P_MPLS_UC,
        priority: 15,
        gso_segment: "mpls_gso_segment",
    },
    PacketOffloadRegistration {
        ethertype: ETH_P_MPLS_MC,
        priority: 15,
        gso_segment: "mpls_gso_segment",
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MplsGsoPacket {
    pub ethertype: u16,
    pub inner_protocol: Option<u16>,
    pub mac_header: Vec<u8>,
    pub mpls_header: Vec<u8>,
    pub inner_payload: Vec<u8>,
    pub gso_size: usize,
    pub requested_features: u64,
    pub device_mpls_features: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MplsGsoSegment {
    pub ethertype: u16,
    pub inner_protocol: u16,
    pub mac_len: usize,
    pub mpls_hlen: usize,
    pub mac_header_offset: usize,
    pub network_header_offset: usize,
    pub inner_network_header_offset: usize,
    pub payload_offset: usize,
    pub payload_len: usize,
    pub gso_features: u64,
    pub frame: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MplsGsoSkbContext {
    pub ethertype: u16,
    pub inner_protocol: Option<u16>,
    pub mac_len: usize,
    pub mpls_hlen: usize,
    pub gso_size: usize,
    pub requested_features: u64,
    pub device_mpls_features: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ioam6Registration {
    pub genl_name: &'static str,
    pub genl_version: u8,
    pub event_group: &'static str,
    pub pernet_subsys: bool,
    pub lwtunnel_encap: u16,
}

pub const IOAM6_REGISTRATION: Ioam6Registration = Ioam6Registration {
    genl_name: IOAM6_GENL_NAME,
    genl_version: IOAM6_GENL_VERSION,
    event_group: IOAM6_GENL_EV_GRP_NAME,
    pernet_subsys: true,
    lwtunnel_encap: LWTUNNEL_ENCAP_IOAM6,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ioam6GenlOp {
    pub cmd: u8,
    pub name: &'static str,
    pub admin_perm: bool,
    pub dumps: bool,
}

pub const IOAM6_GENL_OPS: [Ioam6GenlOp; 7] = [
    Ioam6GenlOp {
        cmd: IOAM6_CMD_ADD_NAMESPACE,
        name: "addns",
        admin_perm: true,
        dumps: false,
    },
    Ioam6GenlOp {
        cmd: IOAM6_CMD_DEL_NAMESPACE,
        name: "delns",
        admin_perm: true,
        dumps: false,
    },
    Ioam6GenlOp {
        cmd: IOAM6_CMD_DUMP_NAMESPACES,
        name: "dumpns",
        admin_perm: true,
        dumps: true,
    },
    Ioam6GenlOp {
        cmd: IOAM6_CMD_ADD_SCHEMA,
        name: "addsc",
        admin_perm: true,
        dumps: false,
    },
    Ioam6GenlOp {
        cmd: IOAM6_CMD_DEL_SCHEMA,
        name: "delsc",
        admin_perm: true,
        dumps: false,
    },
    Ioam6GenlOp {
        cmd: IOAM6_CMD_DUMP_SCHEMAS,
        name: "dumpsc",
        admin_perm: true,
        dumps: true,
    },
    Ioam6GenlOp {
        cmd: IOAM6_CMD_NS_SET_SCHEMA,
        name: "ns_set_schema",
        admin_perm: true,
        dumps: false,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mip6XfrmRegistration {
    pub name: &'static str,
    pub family: u16,
    pub proto: u8,
    pub route_optimization_only: bool,
}

pub const MIP6_XFRM_TYPES: [Mip6XfrmRegistration; 2] = [
    Mip6XfrmRegistration {
        name: "mip6_destopt_type",
        family: AF_INET6,
        proto: IPPROTO_DSTOPTS,
        route_optimization_only: true,
    },
    Mip6XfrmRegistration {
        name: "mip6_rthdr_type",
        family: AF_INET6,
        proto: IPPROTO_ROUTING,
        route_optimization_only: true,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mip6Packet {
    pub next_header: u8,
    pub src: [u8; 16],
    pub dst: [u8; 16],
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mip6XfrmOutput {
    pub proto: u8,
    pub next_header: u8,
    pub src: [u8; 16],
    pub dst: [u8; 16],
    pub home_address: [u8; 16],
    pub extension_header: Vec<u8>,
    pub payload_offset: usize,
    pub frame: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mip6MobilityHeader {
    pub payload_proto: u8,
    pub hdrlen: u8,
    pub mh_type: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mip6Icmpv6ParameterProblem {
    pub icmp_type: u8,
    pub code: u8,
    pub pointer: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mip6MhRejectReason {
    HeaderUnavailable,
    HeaderLengthExceedsPacket,
    MessageTooShort,
    InvalidPayloadProtocol,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mip6MhFilterDrop {
    pub reason: Mip6MhRejectReason,
    pub parameter_problem: Option<Mip6Icmpv6ParameterProblem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mip6MhFilterOutcome {
    Accept,
    Drop(Mip6MhFilterDrop),
}

#[derive(Clone)]
pub struct Mip6XfrmSkbOutput {
    pub skb: SkBuff,
    pub xfrm: Mip6XfrmOutput,
}

#[derive(Clone)]
pub struct Mip6MhSkbDrop {
    pub drop: Mip6MhFilterDrop,
    pub icmpv6: Option<SkBuff>,
}

#[derive(Clone)]
pub enum Mip6MhSkbFilterOutcome {
    Accept,
    Drop(Mip6MhSkbDrop),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ioam6Namespace {
    pub id: u16,
    pub data: u32,
    pub data_wide: u64,
    pub schema_id: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ioam6Schema {
    pub id: u32,
    pub data: Vec<u8>,
    pub namespace_id: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ioam6TraceEvent {
    pub group: &'static str,
    pub name: &'static str,
    pub namespace_id: u16,
    pub schema_id: Option<u32>,
    pub trace_len: usize,
    pub payload_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ioam6LwtunnelOutput {
    pub namespace: Ioam6Namespace,
    pub schema: Option<Ioam6Schema>,
    pub trace_data: Vec<u8>,
    pub payload_offset: usize,
    pub frame: Vec<u8>,
    pub event: Ioam6TraceEvent,
}

#[derive(Clone)]
pub struct Ioam6SkbOutput {
    pub skb: SkBuff,
    pub old_next_header: u8,
    pub hopopt_len: usize,
    pub event: Ioam6TraceEvent,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ioam6TraceHeader {
    pub namespace_id: u16,
    pub nodelen: u8,
    pub remlen: u8,
    pub type_be32: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ioam6LwtunnelConfig {
    pub freq_k: Option<u32>,
    pub freq_n: Option<u32>,
    pub mode: Option<u8>,
    pub tunsrc: Option<[u8; 16]>,
    pub tundst: Option<[u8; 16]>,
    pub trace: Option<Ioam6TraceHeader>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ioam6LwtunnelState {
    pub freq_k: u32,
    pub freq_n: u32,
    pub mode: u8,
    pub has_tunsrc: bool,
    pub tunsrc: Option<[u8; 16]>,
    pub tundst: Option<[u8; 16]>,
    pub trace: Ioam6TraceHeader,
    pub len_aligned: usize,
    pub hopopt_hdrlen: u8,
    pub ioam_opt_type: u8,
    pub ioam_type: u8,
    pub ioam_opt_len: u8,
    pub trace_padding: Vec<u8>,
    pub lwtunnel_type: u16,
    pub flags: u32,
}

#[derive(Default)]
struct Ioam6State {
    namespaces: BTreeMap<u16, Ioam6Namespace>,
    schemas: BTreeMap<u32, Ioam6Schema>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NicheNetRegistrationState {
    pub mpls_gso: bool,
    pub mpls_packet_offloads: usize,
    pub ioam6_pernet_subsys: bool,
    pub ioam6_genl_family: bool,
    pub ioam6_lwtunnel: bool,
    pub mip6_xfrm_types: usize,
    pub mip6_rawv6_mh_filter: bool,
}

static NICHE_NET_REGISTERED: AtomicBool = AtomicBool::new(false);

lazy_static! {
    static ref IOAM6_STATE: Mutex<Ioam6State> = Mutex::new(Ioam6State::default());
    static ref IOAM6_EVENTS: Mutex<Vec<Ioam6TraceEvent>> = Mutex::new(Vec::new());
}

static IOAM6_EVENT_LISTENERS: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if !NICHE_NET_REGISTERED.swap(true, Ordering::AcqRel) {
        for line in NICHE_NET_BOOT_LOGS {
            crate::log_info!("", "{}", line);
        }
    }
}

pub fn registration_snapshot() -> NicheNetRegistrationState {
    let registered = NICHE_NET_REGISTERED.load(Ordering::Acquire);
    NicheNetRegistrationState {
        mpls_gso: registered,
        mpls_packet_offloads: if registered {
            MPLS_GSO_OFFLOADS.len()
        } else {
            0
        },
        ioam6_pernet_subsys: registered && IOAM6_REGISTRATION.pernet_subsys,
        ioam6_genl_family: registered,
        ioam6_lwtunnel: registered && IOAM6_REGISTRATION.lwtunnel_encap == LWTUNNEL_ENCAP_IOAM6,
        mip6_xfrm_types: if registered { MIP6_XFRM_TYPES.len() } else { 0 },
        mip6_rawv6_mh_filter: registered,
    }
}

pub fn ioam6_clear_state() {
    *IOAM6_STATE.lock() = Ioam6State::default();
    IOAM6_EVENTS.lock().clear();
    ioam6_set_event_listener_present(false);
}

pub fn ioam6_set_event_listener_present(present: bool) {
    IOAM6_EVENT_LISTENERS.store(present, Ordering::Release);
}

pub fn ioam6_event_listener_present() -> bool {
    IOAM6_EVENT_LISTENERS.load(Ordering::Acquire)
}

pub fn ioam6_add_namespace(id: u16, data: Option<u32>, data_wide: Option<u64>) -> Result<(), i32> {
    let mut state = IOAM6_STATE.lock();
    if state.namespaces.contains_key(&id) {
        return Err(EEXIST);
    }
    state.namespaces.insert(
        id,
        Ioam6Namespace {
            id,
            data: data.unwrap_or(IOAM6_U32_UNAVAILABLE),
            data_wide: data_wide.unwrap_or(IOAM6_U64_UNAVAILABLE),
            schema_id: None,
        },
    );
    Ok(())
}

pub fn ioam6_del_namespace(id: u16) -> Result<(), i32> {
    let mut state = IOAM6_STATE.lock();
    let ns = state.namespaces.remove(&id).ok_or(ENOENT)?;
    if let Some(schema_id) = ns.schema_id {
        if let Some(schema) = state.schemas.get_mut(&schema_id) {
            schema.namespace_id = None;
        }
    }
    Ok(())
}

pub fn ioam6_dump_namespaces() -> Vec<Ioam6Namespace> {
    IOAM6_STATE.lock().namespaces.values().cloned().collect()
}

pub fn ioam6_add_schema(id: u32, data: &[u8]) -> Result<(), i32> {
    if data.is_empty() {
        return Err(EINVAL);
    }
    if data.len() > IOAM6_MAX_SCHEMA_DATA_LEN {
        return Err(E2BIG);
    }

    let mut state = IOAM6_STATE.lock();
    if state.schemas.contains_key(&id) {
        return Err(EEXIST);
    }
    state.schemas.insert(
        id,
        Ioam6Schema {
            id,
            data: data.to_vec(),
            namespace_id: None,
        },
    );
    Ok(())
}

pub fn ioam6_del_schema(id: u32) -> Result<(), i32> {
    let mut state = IOAM6_STATE.lock();
    let schema = state.schemas.remove(&id).ok_or(ENOENT)?;
    if let Some(ns_id) = schema.namespace_id {
        if let Some(ns) = state.namespaces.get_mut(&ns_id) {
            ns.schema_id = None;
        }
    }
    Ok(())
}

pub fn ioam6_dump_schemas() -> Vec<Ioam6Schema> {
    IOAM6_STATE.lock().schemas.values().cloned().collect()
}

pub fn ioam6_events() -> Vec<Ioam6TraceEvent> {
    IOAM6_EVENTS.lock().clone()
}

pub fn ioam6_namespace_set_schema(ns_id: u16, schema_id: Option<u32>) -> Result<(), i32> {
    let mut state = IOAM6_STATE.lock();
    if !state.namespaces.contains_key(&ns_id) {
        return Err(ENOENT);
    }
    if let Some(schema_id) = schema_id {
        if !state.schemas.contains_key(&schema_id) {
            return Err(ENOENT);
        }
    }

    if let Some(old_schema_id) = state.namespaces.get(&ns_id).and_then(|ns| ns.schema_id) {
        if let Some(old_schema) = state.schemas.get_mut(&old_schema_id) {
            old_schema.namespace_id = None;
        }
    }

    if let Some(schema_id) = schema_id {
        if let Some(old_ns_id) = state.schemas.get(&schema_id).and_then(|sc| sc.namespace_id) {
            if let Some(old_ns) = state.namespaces.get_mut(&old_ns_id) {
                old_ns.schema_id = None;
            }
        }
        state.namespaces.get_mut(&ns_id).unwrap().schema_id = Some(schema_id);
        state.schemas.get_mut(&schema_id).unwrap().namespace_id = Some(ns_id);
    } else {
        state.namespaces.get_mut(&ns_id).unwrap().schema_id = None;
    }

    Ok(())
}

pub fn mpls_gso_segment(packet: &MplsGsoPacket) -> Result<Vec<MplsGsoSegment>, i32> {
    match packet.ethertype {
        ETH_P_MPLS_UC | ETH_P_MPLS_MC => {}
        _ => return Err(EINVAL),
    }

    let inner_protocol = packet.inner_protocol.ok_or(EINVAL)?;
    let mpls_hlen = packet.mpls_header.len();
    if mpls_hlen == 0 || mpls_hlen % MPLS_HLEN != 0 {
        return Err(EINVAL);
    }
    if packet.inner_payload.is_empty() || packet.gso_size == 0 {
        return Err(EINVAL);
    }

    let segment_count = packet.inner_payload.len().div_ceil(packet.gso_size);
    if segment_count == 0 || segment_count > MPLS_GSO_MAX_SEGMENTS {
        return Err(EINVAL);
    }

    let mac_len = packet.mac_header.len();
    let payload_offset = mac_len.checked_add(mpls_hlen).ok_or(EINVAL)?;
    let gso_features = packet.requested_features & packet.device_mpls_features;
    let mut segments = Vec::new();
    segments
        .try_reserve_exact(segment_count)
        .map_err(|_| ENOMEM)?;

    for chunk in packet.inner_payload.chunks(packet.gso_size) {
        let frame_len = payload_offset.checked_add(chunk.len()).ok_or(EINVAL)?;
        let mut frame = Vec::new();
        frame.try_reserve_exact(frame_len).map_err(|_| ENOMEM)?;
        frame.extend_from_slice(&packet.mac_header);
        frame.extend_from_slice(&packet.mpls_header);
        frame.extend_from_slice(chunk);

        segments.push(MplsGsoSegment {
            ethertype: packet.ethertype,
            inner_protocol,
            mac_len,
            mpls_hlen,
            mac_header_offset: 0,
            network_header_offset: mac_len,
            inner_network_header_offset: payload_offset,
            payload_offset,
            payload_len: chunk.len(),
            gso_features,
            frame,
        });
    }

    Ok(segments)
}

/// skb-backed MPLS GSO path.
///
/// Source shape: `vendor/linux/net/mpls/mpls_gso.c::mpls_gso_segment`.
/// The caller supplies the metadata Linux stores in the skb: protocol,
/// inner protocol, MAC length, MPLS header span, GSO size, and device MPLS
/// features. The output skbs preserve MAC/MPLS headers around each inner
/// segment, matching Linux's unwind/re-push sequence after inner GSO.
pub fn mpls_gso_segment_skb(skb: &SkBuff, ctx: MplsGsoSkbContext) -> Result<Vec<SkBuff>, i32> {
    match ctx.ethertype {
        ETH_P_MPLS_UC | ETH_P_MPLS_MC => {}
        _ => return Err(EINVAL),
    }
    let inner_protocol = ctx.inner_protocol.ok_or(EINVAL)?;
    if inner_protocol != ETH_P_IP && inner_protocol != ETH_P_IPV6 {
        return Err(EINVAL);
    }
    if ctx.mpls_hlen == 0
        || ctx.mpls_hlen % MPLS_HLEN != 0
        || ctx.gso_size == 0
        || ctx.gso_size > u16::MAX as usize
    {
        return Err(EINVAL);
    }

    let data = skb.data();
    let payload_offset = ctx.mac_len.checked_add(ctx.mpls_hlen).ok_or(EINVAL)?;
    if data.len() <= payload_offset {
        return Err(EINVAL);
    }
    let payload = &data[payload_offset..];
    let segment_count = payload.len().div_ceil(ctx.gso_size);
    if segment_count == 0 || segment_count > MPLS_GSO_MAX_SEGMENTS {
        return Err(EINVAL);
    }

    let mut segments = Vec::new();
    segments
        .try_reserve_exact(segment_count)
        .map_err(|_| ENOMEM)?;
    for chunk in payload.chunks(ctx.gso_size) {
        let frame_len = payload_offset.checked_add(chunk.len()).ok_or(EINVAL)?;
        let mut out = alloc_skb(frame_len)?;
        let frame = skb_put(&mut out, frame_len)?;
        frame[..payload_offset].copy_from_slice(&data[..payload_offset]);
        frame[payload_offset..].copy_from_slice(chunk);
        out.shared_info.gso_size = ctx.gso_size as u16;
        out.shared_info.gso_type = ctx.ethertype;
        segments.push(out);
    }
    Ok(segments)
}

pub fn validate_ioam6_lwtunnel(family: u16, encap: u16) -> Result<(), i32> {
    if family != AF_INET6 || encap != LWTUNNEL_ENCAP_IOAM6 {
        return Err(EINVAL);
    }
    Ok(())
}

fn align_to_8(value: usize) -> usize {
    (value + 7) & !7
}

fn ipv6_addr_any(addr: &[u8; 16]) -> bool {
    addr.iter().all(|byte| *byte == 0)
}

pub fn ioam6_trace_compute_nodelen(trace_type: u32) -> u8 {
    (trace_type & IOAM6_MASK_SHORT_FIELDS).count_ones() as u8
        + ((trace_type & IOAM6_MASK_WIDE_FIELDS).count_ones() as u8 * 2)
}

pub fn ioam6_validate_trace_header(mut trace: Ioam6TraceHeader) -> Result<Ioam6TraceHeader, i32> {
    if trace.type_be32 == 0
        || trace.remlen == 0
        || usize::from(trace.remlen) > IOAM6_TRACE_DATA_SIZE_MAX / 4
        || trace.type_be32 & IOAM6_TRACE_FORBIDDEN_LWT_BITS != 0
    {
        return Err(EINVAL);
    }

    trace.nodelen = ioam6_trace_compute_nodelen(trace.type_be32);
    Ok(trace)
}

pub fn ioam6_build_lwtunnel_state(
    family: u16,
    config: Ioam6LwtunnelConfig,
) -> Result<Ioam6LwtunnelState, i32> {
    if family != AF_INET6 {
        return Err(EINVAL);
    }

    let (freq_k, freq_n) = match (config.freq_k, config.freq_n) {
        (None, None) => (IOAM6_IPTUNNEL_FREQ_MIN, IOAM6_IPTUNNEL_FREQ_MIN),
        (Some(_), None) | (None, Some(_)) => return Err(EINVAL),
        (Some(freq_k), Some(freq_n)) => {
            if !(IOAM6_IPTUNNEL_FREQ_MIN..=IOAM6_IPTUNNEL_FREQ_MAX).contains(&freq_k)
                || !(IOAM6_IPTUNNEL_FREQ_MIN..=IOAM6_IPTUNNEL_FREQ_MAX).contains(&freq_n)
                || freq_k > freq_n
            {
                return Err(EINVAL);
            }
            (freq_k, freq_n)
        }
    };

    let mode = config.mode.unwrap_or(IOAM6_IPTUNNEL_MODE_INLINE);
    if !(IOAM6_IPTUNNEL_MODE_INLINE..=IOAM6_IPTUNNEL_MODE_AUTO).contains(&mode) {
        return Err(EINVAL);
    }

    if config.tunsrc.is_some() && mode == IOAM6_IPTUNNEL_MODE_INLINE {
        return Err(EINVAL);
    }

    if config.tundst.is_none() && mode != IOAM6_IPTUNNEL_MODE_INLINE {
        return Err(EINVAL);
    }

    if config.tunsrc.as_ref().is_some_and(ipv6_addr_any)
        || config.tundst.as_ref().is_some_and(ipv6_addr_any)
    {
        return Err(EINVAL);
    }

    let trace = ioam6_validate_trace_header(config.trace.ok_or(EINVAL)?)?;
    let trace_data_len = usize::from(trace.remlen).checked_mul(4).ok_or(EINVAL)?;
    let len_aligned = align_to_8(trace_data_len);
    let hopopt_hdrlen = ((IOAM6_LWT_ENCAP_BASE_LEN + len_aligned) >> 3)
        .checked_sub(1)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or(EINVAL)?;
    let ioam_opt_len = (IOAM6_HDR_LEN - 2)
        .checked_add(IOAM6_TRACE_HDR_LEN)
        .and_then(|len| len.checked_add(trace_data_len))
        .and_then(|len| u8::try_from(len).ok())
        .ok_or(EINVAL)?;

    let mut trace_padding = Vec::new();
    let padding_len = len_aligned - trace_data_len;
    if padding_len != 0 {
        trace_padding
            .try_reserve_exact(padding_len)
            .map_err(|_| ENOMEM)?;
        trace_padding.push(IPV6_TLV_PADN);
        trace_padding.push((padding_len - 2) as u8);
        trace_padding.resize(padding_len, 0);
    }

    Ok(Ioam6LwtunnelState {
        freq_k,
        freq_n,
        mode,
        has_tunsrc: config.tunsrc.is_some(),
        tunsrc: config.tunsrc,
        tundst: config.tundst,
        trace,
        len_aligned,
        hopopt_hdrlen,
        ioam_opt_type: IPV6_TLV_IOAM,
        ioam_type: IOAM6_TYPE_PREALLOC,
        ioam_opt_len,
        trace_padding,
        lwtunnel_type: LWTUNNEL_ENCAP_IOAM6,
        flags: LWTUNNEL_STATE_OUTPUT_REDIRECT,
    })
}

pub fn ioam6_lwtunnel_output(
    family: u16,
    encap: u16,
    namespace_id: u16,
    payload: &[u8],
) -> Result<Ioam6LwtunnelOutput, i32> {
    validate_ioam6_lwtunnel(family, encap)?;
    if payload.is_empty() {
        return Err(EINVAL);
    }

    let (namespace, schema) = {
        let state = IOAM6_STATE.lock();
        let namespace = state.namespaces.get(&namespace_id).cloned().ok_or(ENOENT)?;
        let schema = namespace
            .schema_id
            .and_then(|schema_id| state.schemas.get(&schema_id).cloned());
        (namespace, schema)
    };

    let schema_len = schema.as_ref().map(|schema| schema.data.len()).unwrap_or(0);
    let trace_capacity = 2usize
        .checked_add(if namespace.data == IOAM6_U32_UNAVAILABLE {
            0
        } else {
            4
        })
        .and_then(|len| {
            len.checked_add(if namespace.data_wide == IOAM6_U64_UNAVAILABLE {
                0
            } else {
                8
            })
        })
        .and_then(|len| len.checked_add(schema.as_ref().map(|_| 4).unwrap_or(0)))
        .and_then(|len| len.checked_add(schema_len))
        .ok_or(EINVAL)?;
    let mut trace_data = Vec::new();
    trace_data
        .try_reserve_exact(trace_capacity)
        .map_err(|_| ENOMEM)?;
    trace_data.extend_from_slice(&namespace.id.to_be_bytes());
    if namespace.data != IOAM6_U32_UNAVAILABLE {
        trace_data.extend_from_slice(&namespace.data.to_be_bytes());
    }
    if namespace.data_wide != IOAM6_U64_UNAVAILABLE {
        trace_data.extend_from_slice(&namespace.data_wide.to_be_bytes());
    }
    if let Some(schema) = schema.as_ref() {
        trace_data.extend_from_slice(&schema.id.to_be_bytes());
        trace_data.extend_from_slice(&schema.data);
    }

    let frame_len = trace_data.len().checked_add(payload.len()).ok_or(EINVAL)?;
    let payload_offset = trace_data.len();
    let mut frame = Vec::new();
    frame.try_reserve_exact(frame_len).map_err(|_| ENOMEM)?;
    frame.extend_from_slice(&trace_data);
    frame.extend_from_slice(payload);

    let event = Ioam6TraceEvent {
        group: IOAM6_GENL_EV_GRP_NAME,
        name: IOAM6_EVENT_TRACE_FILLED,
        namespace_id,
        schema_id: schema.as_ref().map(|schema| schema.id),
        trace_len: trace_data.len(),
        payload_len: payload.len(),
    };
    if ioam6_event_listener_present() {
        IOAM6_EVENTS.lock().push(event.clone());
    }

    Ok(Ioam6LwtunnelOutput {
        namespace,
        schema,
        trace_data,
        payload_offset,
        frame,
        event,
    })
}

/// Insert a Linux-shaped IOAM Hop-by-Hop option into an IPv6 skb.
///
/// Source shape:
/// - `vendor/linux/net/ipv6/ioam6_iptunnel.c::ioam6_output`
/// - `vendor/linux/net/ipv6/exthdrs.c` IOAM option parsing/fill path
/// - `vendor/linux/net/ipv6/ioam6.c::ioam6_event`
pub fn ioam6_lwtunnel_output_skb(
    family: u16,
    encap: u16,
    state: &Ioam6LwtunnelState,
    namespace_id: u16,
    skb: &SkBuff,
) -> Result<Ioam6SkbOutput, i32> {
    validate_ioam6_lwtunnel(family, encap)?;
    let ipv6 = parse_ipv6_packet(skb).map_err(|_| EINVAL)?;
    let output = ioam6_lwtunnel_output(family, encap, namespace_id, &ipv6.payload)?;
    let trace_capacity = usize::from(state.trace.remlen)
        .checked_mul(4)
        .ok_or(EINVAL)?;
    if output.trace_data.len() > trace_capacity {
        return Err(EINVAL);
    }

    let mut trace_data = output.trace_data.clone();
    trace_data.resize(trace_capacity, 0);

    let hopopt_base_len = 2usize
        .checked_add(IOAM6_HDR_LEN)
        .and_then(|len| len.checked_add(IOAM6_TRACE_HDR_LEN))
        .and_then(|len| len.checked_add(trace_data.len()))
        .and_then(|len| len.checked_add(state.trace_padding.len()))
        .ok_or(EINVAL)?;
    let mut hopopt = Vec::new();
    hopopt
        .try_reserve_exact(hopopt_base_len + 8)
        .map_err(|_| ENOMEM)?;
    hopopt.push(ipv6.next_header);
    hopopt.push(0);
    hopopt.push(state.ioam_opt_type);
    hopopt.push(state.ioam_opt_len);
    hopopt.push(0);
    hopopt.push(state.ioam_type);
    hopopt.extend_from_slice(&state.trace.namespace_id.to_be_bytes());
    hopopt.push(state.trace.nodelen & 0x1f);
    hopopt.push(state.trace.remlen & 0x7f);
    hopopt.extend_from_slice(&state.trace.type_be32.to_be_bytes());
    hopopt.extend_from_slice(&trace_data);
    hopopt.extend_from_slice(&state.trace_padding);
    let pad_len = (8 - (hopopt.len() % 8)) % 8;
    mip6_append_padn(&mut hopopt, pad_len);
    if hopopt.len() % 8 != 0 {
        return Err(EINVAL);
    }
    let hopopt_len = hopopt.len();
    hopopt[1] = ((hopopt_len / 8).checked_sub(1).ok_or(EINVAL)?) as u8;

    let mut payload = Vec::new();
    payload
        .try_reserve_exact(hopopt.len() + ipv6.payload.len())
        .map_err(|_| ENOMEM)?;
    payload.extend_from_slice(&hopopt);
    payload.extend_from_slice(&ipv6.payload);
    let skb = build_ipv6_packet(
        ipv6.src,
        ipv6.dst,
        IPPROTO_HOPOPTS,
        &payload,
        ipv6.hop_limit,
    )?;

    Ok(Ioam6SkbOutput {
        skb,
        old_next_header: ipv6.next_header,
        hopopt_len,
        event: output.event,
    })
}

pub fn validate_mip6_xfrm_type(family: u16, proto: u8) -> Result<(), i32> {
    if family != AF_INET6 {
        return Err(EINVAL);
    }
    if MIP6_XFRM_TYPES
        .iter()
        .any(|entry| entry.family == family && entry.proto == proto)
    {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

fn mip6_calc_padlen(len: usize, n: usize) -> usize {
    (n.wrapping_sub(len).wrapping_add(16)) & 0x7
}

fn mip6_append_padn(out: &mut Vec<u8>, padlen: usize) {
    if padlen == 0 {
        return;
    }
    if padlen == 1 {
        out.push(IPV6_TLV_PAD1);
        return;
    }
    out.push(IPV6_TLV_PADN);
    out.push((padlen - 2) as u8);
    out.resize(out.len() + padlen - 2, 0);
}

pub fn mip6_xfrm_output(
    family: u16,
    proto: u8,
    packet: &Mip6Packet,
    care_of_addr: [u8; 16],
) -> Result<Mip6XfrmOutput, i32> {
    validate_mip6_xfrm_type(family, proto)?;
    if packet.payload.is_empty() {
        return Err(EINVAL);
    }

    let mut extension_header = Vec::new();
    let (src, dst, home_address, expected_header_len) = match proto {
        IPPROTO_DSTOPTS => {
            extension_header
                .try_reserve_exact(MIP6_DESTOPT_HEADER_LEN)
                .map_err(|_| ENOMEM)?;
            extension_header.push(packet.next_header);
            extension_header.push((MIP6_DESTOPT_HEADER_LEN / 8 - 1) as u8);
            mip6_append_padn(&mut extension_header, mip6_calc_padlen(2, 6));
            extension_header.push(IPV6_TLV_HAO);
            extension_header.push(16);
            extension_header.extend_from_slice(&packet.src);
            (
                care_of_addr,
                packet.dst,
                packet.src,
                MIP6_DESTOPT_HEADER_LEN,
            )
        }
        IPPROTO_ROUTING => {
            extension_header
                .try_reserve_exact(MIP6_RTHDR_HEADER_LEN)
                .map_err(|_| ENOMEM)?;
            extension_header.push(packet.next_header);
            extension_header.push((MIP6_RTHDR_HEADER_LEN / 8 - 1) as u8);
            extension_header.push(IPV6_SRCRT_TYPE_2);
            extension_header.push(1);
            extension_header.extend_from_slice(&[0, 0, 0, 0]);
            extension_header.extend_from_slice(&packet.dst);
            (packet.src, care_of_addr, packet.dst, MIP6_RTHDR_HEADER_LEN)
        }
        _ => return Err(EINVAL),
    };

    if extension_header.len() != expected_header_len {
        return Err(EINVAL);
    }
    let payload_offset = extension_header.len();
    let frame_len = payload_offset
        .checked_add(packet.payload.len())
        .ok_or(EINVAL)?;
    let mut frame = Vec::new();
    frame.try_reserve_exact(frame_len).map_err(|_| ENOMEM)?;
    frame.extend_from_slice(&extension_header);
    frame.extend_from_slice(&packet.payload);

    Ok(Mip6XfrmOutput {
        proto,
        next_header: packet.next_header,
        src,
        dst,
        home_address,
        extension_header,
        payload_offset,
        frame,
    })
}

/// skb-backed MIP6 xfrm output.
///
/// Source shape: `vendor/linux/net/ipv6/mip6.c` destination-option and
/// type-2-routing-header xfrm output handlers.
pub fn mip6_xfrm_output_skb(
    family: u16,
    proto: u8,
    skb: &SkBuff,
    care_of_addr: [u8; 16],
) -> Result<Mip6XfrmSkbOutput, i32> {
    let ipv6 = parse_ipv6_packet(skb).map_err(|_| EINVAL)?;
    let packet = Mip6Packet {
        next_header: ipv6.next_header,
        src: ipv6.src,
        dst: ipv6.dst,
        payload: ipv6.payload,
    };
    let xfrm = mip6_xfrm_output(family, proto, &packet, care_of_addr)?;
    let skb = build_ipv6_packet(xfrm.src, xfrm.dst, proto, &xfrm.frame, 64)?;
    Ok(Mip6XfrmSkbOutput { skb, xfrm })
}

pub fn mip6_xfrm_input(
    family: u16,
    output: &Mip6XfrmOutput,
    care_of_addr: [u8; 16],
) -> Result<u8, i32> {
    validate_mip6_xfrm_type(family, output.proto)?;
    match output.proto {
        IPPROTO_DSTOPTS => {
            if care_of_addr != [0; 16] && output.src != care_of_addr {
                return Err(ENOENT);
            }
        }
        IPPROTO_ROUTING => {
            if care_of_addr != [0; 16] && output.dst != care_of_addr {
                return Err(ENOENT);
            }
        }
        _ => return Err(EINVAL),
    }
    Ok(output.next_header)
}

fn mip6_mh_min_hdrlen(mh_type: u8) -> u8 {
    match mh_type {
        IP6_MH_TYPE_BRR => 0,
        IP6_MH_TYPE_HOTI | IP6_MH_TYPE_COTI | IP6_MH_TYPE_BU | IP6_MH_TYPE_BACK => 1,
        IP6_MH_TYPE_HOT | IP6_MH_TYPE_COT | IP6_MH_TYPE_BERROR => 2,
        _ => 0,
    }
}

fn mip6_param_problem(
    network_header_len: usize,
    field_offset: usize,
) -> Mip6Icmpv6ParameterProblem {
    Mip6Icmpv6ParameterProblem {
        icmp_type: ICMPV6_PARAMPROB,
        code: ICMPV6_HDR_FIELD,
        pointer: network_header_len.saturating_add(field_offset),
    }
}

pub fn mip6_mh_filter_report(
    header: Mip6MobilityHeader,
    packet_len: usize,
    network_header_len: usize,
) -> Mip6MhFilterOutcome {
    if packet_len < MIP6_MH_BASE_LEN {
        return Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
            reason: Mip6MhRejectReason::HeaderUnavailable,
            parameter_problem: None,
        });
    }

    let header_len = match usize::from(header.hdrlen)
        .checked_add(1)
        .and_then(|len| len.checked_mul(8))
    {
        Some(header_len) => header_len,
        None => {
            return Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
                reason: Mip6MhRejectReason::HeaderLengthExceedsPacket,
                parameter_problem: None,
            });
        }
    };
    if header_len > packet_len {
        return Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
            reason: Mip6MhRejectReason::HeaderLengthExceedsPacket,
            parameter_problem: None,
        });
    }

    if header.hdrlen < mip6_mh_min_hdrlen(header.mh_type) {
        return Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
            reason: Mip6MhRejectReason::MessageTooShort,
            parameter_problem: Some(mip6_param_problem(
                network_header_len,
                MIP6_MH_HDRLEN_OFFSET,
            )),
        });
    }
    if header.payload_proto != IPPROTO_NONE {
        return Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
            reason: Mip6MhRejectReason::InvalidPayloadProtocol,
            parameter_problem: Some(mip6_param_problem(network_header_len, MIP6_MH_PROTO_OFFSET)),
        });
    }

    Mip6MhFilterOutcome::Accept
}

pub fn mip6_mh_filter(header: Mip6MobilityHeader, packet_len: usize) -> Result<(), i32> {
    match mip6_mh_filter_report(header, packet_len, IPV6_HEADER_LEN) {
        Mip6MhFilterOutcome::Accept => Ok(()),
        Mip6MhFilterOutcome::Drop(_) => Err(EINVAL),
    }
}

pub fn rawv6_mh_filter_protocol() -> u8 {
    IPPROTO_MH
}

fn ipv6_upper_layer_checksum(
    src: [u8; 16],
    dst: [u8; 16],
    next_header: u8,
    payload: &[u8],
) -> Result<u16, i32> {
    if payload.len() > u32::MAX as usize {
        return Err(EINVAL);
    }
    let mut pseudo = Vec::new();
    pseudo
        .try_reserve_exact(40 + payload.len())
        .map_err(|_| ENOMEM)?;
    pseudo.extend_from_slice(&src);
    pseudo.extend_from_slice(&dst);
    pseudo.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    pseudo.extend_from_slice(&[0, 0, 0]);
    pseudo.push(next_header);
    pseudo.extend_from_slice(payload);
    Ok(checksum(&pseudo))
}

pub fn build_icmpv6_parameter_problem_skb(
    src: [u8; 16],
    dst: [u8; 16],
    pointer: usize,
    invoking_packet: &[u8],
) -> Result<SkBuff, i32> {
    if pointer > u32::MAX as usize {
        return Err(EINVAL);
    }
    let payload_len = 8usize.checked_add(invoking_packet.len()).ok_or(EINVAL)?;
    let mut payload = Vec::new();
    payload.try_reserve_exact(payload_len).map_err(|_| ENOMEM)?;
    payload.push(ICMPV6_PARAMPROB);
    payload.push(ICMPV6_HDR_FIELD);
    payload.extend_from_slice(&0u16.to_be_bytes());
    payload.extend_from_slice(&(pointer as u32).to_be_bytes());
    payload.extend_from_slice(invoking_packet);
    let csum = ipv6_upper_layer_checksum(dst, src, IPPROTO_ICMPV6, &payload)?;
    payload[2..4].copy_from_slice(&csum.to_be_bytes());
    build_ipv6_packet(dst, src, IPPROTO_ICMPV6, &payload, 64)
}

/// skb-backed rawv6 mobility-header filter.
///
/// Source shape: `vendor/linux/net/ipv6/mip6.c::mip6_mh_filter`, including
/// ICMPv6 Parameter Problem emission for Linux-reported bad fields.
pub fn mip6_mh_filter_skb(skb: &SkBuff) -> Result<Mip6MhSkbFilterOutcome, i32> {
    let ipv6 = parse_ipv6_packet(skb).map_err(|_| EINVAL)?;
    if ipv6.next_header != IPPROTO_MH || ipv6.payload.len() < MIP6_MH_BASE_LEN {
        let drop = Mip6MhFilterDrop {
            reason: Mip6MhRejectReason::HeaderUnavailable,
            parameter_problem: None,
        };
        return Ok(Mip6MhSkbFilterOutcome::Drop(Mip6MhSkbDrop {
            drop,
            icmpv6: None,
        }));
    }
    let header = Mip6MobilityHeader {
        payload_proto: ipv6.payload[0],
        hdrlen: ipv6.payload[1],
        mh_type: ipv6.payload[2],
    };
    match mip6_mh_filter_report(header, ipv6.payload.len(), IPV6_HEADER_LEN) {
        Mip6MhFilterOutcome::Accept => Ok(Mip6MhSkbFilterOutcome::Accept),
        Mip6MhFilterOutcome::Drop(drop) => {
            let icmpv6 = drop
                .parameter_problem
                .map(|problem| {
                    build_icmpv6_parameter_problem_skb(
                        ipv6.src,
                        ipv6.dst,
                        problem.pointer,
                        skb.data(),
                    )
                })
                .transpose()?;
            Ok(Mip6MhSkbFilterOutcome::Drop(Mip6MhSkbDrop { drop, icmpv6 }))
        }
    }
}

pub fn icmpv6_checksum_valid(skb: &SkBuff) -> bool {
    let Ok(ipv6) = parse_ipv6_packet(skb) else {
        return false;
    };
    if ipv6.next_header != IPPROTO_ICMPV6 {
        return false;
    }
    ipv6_upper_layer_checksum(ipv6.src, ipv6.dst, IPPROTO_ICMPV6, &ipv6.payload) == Ok(0)
}

pub fn run_niche_acceptance() -> Result<(), i32> {
    let mut mpls = alloc_skb(30)?;
    skb_put(&mut mpls, 30)?.copy_from_slice(&[
        b'm', b'a', b'c', b'm', b'a', b'c', 0, 0, 1, 0xff, 0, 0, 2, 0xff, 0, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 11, 12, 13, 14, 15,
    ]);
    let mpls_segments = mpls_gso_segment_skb(
        &mpls,
        MplsGsoSkbContext {
            ethertype: ETH_P_MPLS_UC,
            inner_protocol: Some(ETH_P_IP),
            mac_len: 6,
            mpls_hlen: 8,
            gso_size: 4,
            requested_features: 0b1110,
            device_mpls_features: 0b1010,
        },
    )?;
    assert_eq!(mpls_segments.len(), 4);
    assert!(
        mpls_segments
            .iter()
            .all(|seg| seg.data().starts_with(b"macmac"))
    );

    ioam6_clear_state();
    ioam6_set_event_listener_present(true);
    ioam6_add_namespace(7, Some(0x11223344), None)?;
    let ioam_state = ioam6_build_lwtunnel_state(
        AF_INET6,
        Ioam6LwtunnelConfig {
            trace: Some(Ioam6TraceHeader {
                namespace_id: 7,
                nodelen: 0,
                remlen: 2,
                type_be32: 0x8000_0000,
            }),
            ..Ioam6LwtunnelConfig::default()
        },
    )?;
    let ipv6 = build_ipv6_packet([1; 16], [2; 16], 17, b"payload", 64)?;
    let ioam = ioam6_lwtunnel_output_skb(AF_INET6, LWTUNNEL_ENCAP_IOAM6, &ioam_state, 7, &ipv6)?;
    let ioam_ipv6 = parse_ipv6_packet(&ioam.skb).map_err(|_| EINVAL)?;
    assert_eq!(ioam_ipv6.next_header, IPPROTO_HOPOPTS);
    assert_eq!(ioam.old_next_header, 17);
    assert_eq!(ioam.event.namespace_id, 7);
    assert_eq!(ioam6_events(), alloc::vec![ioam.event]);

    let mip6_packet = build_ipv6_packet([0x11; 16], [0x22; 16], IPPROTO_MH, b"mh-payload", 64)?;
    let xfrm = mip6_xfrm_output_skb(AF_INET6, IPPROTO_DSTOPTS, &mip6_packet, [0x33; 16])?;
    let xfrm_ipv6 = parse_ipv6_packet(&xfrm.skb).map_err(|_| EINVAL)?;
    assert_eq!(xfrm_ipv6.next_header, IPPROTO_DSTOPTS);
    assert_eq!(xfrm.xfrm.home_address, [0x11; 16]);

    let bad_mh = build_ipv6_packet(
        [0x44; 16],
        [0x55; 16],
        IPPROTO_MH,
        &[
            IPPROTO_MH,
            1,
            IP6_MH_TYPE_BU,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ],
        64,
    )?;
    match mip6_mh_filter_skb(&bad_mh)? {
        Mip6MhSkbFilterOutcome::Drop(drop) => {
            let icmp = drop.icmpv6.ok_or(EINVAL)?;
            assert!(icmpv6_checksum_valid(&icmp));
        }
        Mip6MhSkbFilterOutcome::Accept => return Err(EINVAL),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    static IOAM6_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn ioam6_test_guard() -> spin::MutexGuard<'static, ()> {
        let guard = IOAM6_TEST_LOCK.lock();
        ioam6_clear_state();
        guard
    }

    #[test]
    fn niche_boot_lines_match_linux_init_tokens() {
        assert_eq!(
            NICHE_NET_BOOT_LOGS,
            [
                "MPLS GSO support",
                "In-situ OAM (IOAM) with IPv6",
                "Mobile IPv6",
            ]
        );
    }

    #[test]
    fn mpls_gso_registers_uc_and_mc_packet_offloads() {
        assert_eq!(
            MPLS_GSO_OFFLOADS,
            [
                PacketOffloadRegistration {
                    ethertype: ETH_P_MPLS_UC,
                    priority: 15,
                    gso_segment: "mpls_gso_segment",
                },
                PacketOffloadRegistration {
                    ethertype: ETH_P_MPLS_MC,
                    priority: 15,
                    gso_segment: "mpls_gso_segment",
                },
            ]
        );
        for offload in MPLS_GSO_OFFLOADS {
            let packet = mpls_test_packet(offload.ethertype, 4, 8);
            let segments = mpls_gso_segment(&packet).expect("segment MPLS packet");
            assert_eq!(segments.len(), 2);
            assert!(segments.iter().all(|segment| {
                segment.ethertype == offload.ethertype && segment.inner_protocol == ETH_P_IP
            }));
        }
    }

    #[test]
    fn mpls_gso_segments_inner_payload_and_restores_headers() {
        let packet = mpls_test_packet(ETH_P_MPLS_UC, 4, 10);
        let segments = mpls_gso_segment(&packet).expect("segment MPLS packet");

        assert_eq!(segments.len(), 3);
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.payload_len)
                .collect::<Vec<_>>(),
            alloc::vec![4, 4, 2]
        );
        assert_eq!(
            segments
                .iter()
                .map(|segment| segment.gso_features)
                .collect::<Vec<_>>(),
            alloc::vec![0b1010, 0b1010, 0b1010]
        );

        let mut reassembled = Vec::new();
        for segment in &segments {
            assert_eq!(segment.mac_len, 6);
            assert_eq!(segment.mpls_hlen, 8);
            assert_eq!(segment.mac_header_offset, 0);
            assert_eq!(segment.network_header_offset, 6);
            assert_eq!(segment.inner_network_header_offset, 14);
            assert_eq!(segment.payload_offset, 14);
            assert_eq!(&segment.frame[..6], b"macmac");
            assert_eq!(
                &segment.frame[6..14],
                &[0x00, 0x00, 0x01, 0xff, 0x00, 0x00, 0x02, 0xff]
            );
            reassembled.extend_from_slice(&segment.frame[segment.payload_offset..]);
        }
        assert_eq!(reassembled, packet.inner_payload);
    }

    #[test]
    fn mpls_gso_rejects_linux_invalid_header_preconditions() {
        let mut packet = mpls_test_packet(ETH_P_MPLS_UC, 4, 8);

        packet.ethertype = ETH_P_IP;
        assert_eq!(mpls_gso_segment(&packet), Err(EINVAL));

        packet = mpls_test_packet(ETH_P_MPLS_UC, 4, 8);
        packet.inner_protocol = None;
        assert_eq!(mpls_gso_segment(&packet), Err(EINVAL));

        packet = mpls_test_packet(ETH_P_MPLS_UC, 4, 8);
        packet.mpls_header.clear();
        assert_eq!(mpls_gso_segment(&packet), Err(EINVAL));

        packet = mpls_test_packet(ETH_P_MPLS_UC, 4, 8);
        packet.mpls_header.pop();
        assert_eq!(mpls_gso_segment(&packet), Err(EINVAL));

        packet = mpls_test_packet(ETH_P_MPLS_UC, 0, 8);
        assert_eq!(mpls_gso_segment(&packet), Err(EINVAL));

        packet = mpls_test_packet(ETH_P_MPLS_UC, 1, MPLS_GSO_MAX_SEGMENTS + 1);
        assert_eq!(mpls_gso_segment(&packet), Err(EINVAL));
    }

    #[test]
    fn ioam6_registers_genl_pernet_and_lwtunnel_state() {
        assert_eq!(IOAM6_REGISTRATION.genl_name, "IOAM6");
        assert_eq!(IOAM6_REGISTRATION.genl_version, 1);
        assert_eq!(IOAM6_REGISTRATION.event_group, "ioam6_events");
        assert!(IOAM6_REGISTRATION.pernet_subsys);
        assert_eq!(IOAM6_REGISTRATION.lwtunnel_encap, LWTUNNEL_ENCAP_IOAM6);
        assert_eq!(IOAM6_GENL_OPS.len(), 7);
        assert_eq!(IOAM6_GENL_OPS[0].cmd, IOAM6_CMD_ADD_NAMESPACE);
        assert_eq!(IOAM6_GENL_OPS[2].cmd, IOAM6_CMD_DUMP_NAMESPACES);
        assert!(IOAM6_GENL_OPS[2].dumps);
        assert_eq!(IOAM6_GENL_OPS[6].cmd, IOAM6_CMD_NS_SET_SCHEMA);
        assert!(IOAM6_GENL_OPS.iter().all(|op| op.admin_perm));
        assert_eq!(
            validate_ioam6_lwtunnel(AF_INET6, LWTUNNEL_ENCAP_IOAM6),
            Ok(())
        );
        assert_eq!(validate_ioam6_lwtunnel(AF_INET6, 0), Err(EINVAL));
    }

    #[test]
    fn ioam6_lwtunnel_build_state_matches_linux_validation_rules() {
        let trace = Ioam6TraceHeader {
            namespace_id: 7,
            nodelen: 0xff,
            remlen: 3,
            type_be32: 0x8020_0000,
        };
        let state = ioam6_build_lwtunnel_state(
            AF_INET6,
            Ioam6LwtunnelConfig {
                trace: Some(trace),
                ..Ioam6LwtunnelConfig::default()
            },
        )
        .expect("default inline state");

        assert_eq!(state.freq_k, IOAM6_IPTUNNEL_FREQ_MIN);
        assert_eq!(state.freq_n, IOAM6_IPTUNNEL_FREQ_MIN);
        assert_eq!(state.mode, IOAM6_IPTUNNEL_MODE_INLINE);
        assert!(!state.has_tunsrc);
        assert_eq!(state.trace.nodelen, 3);
        assert_eq!(state.len_aligned, 16);
        assert_eq!(state.hopopt_hdrlen, 3);
        assert_eq!(state.ioam_opt_type, IPV6_TLV_IOAM);
        assert_eq!(state.ioam_type, IOAM6_TYPE_PREALLOC);
        assert_eq!(state.ioam_opt_len, 22);
        assert_eq!(state.trace_padding, alloc::vec![IPV6_TLV_PADN, 2, 0, 0]);
        assert_eq!(state.lwtunnel_type, LWTUNNEL_ENCAP_IOAM6);
        assert_eq!(state.flags, LWTUNNEL_STATE_OUTPUT_REDIRECT);

        let encap = ioam6_build_lwtunnel_state(
            AF_INET6,
            Ioam6LwtunnelConfig {
                freq_k: Some(2),
                freq_n: Some(5),
                mode: Some(IOAM6_IPTUNNEL_MODE_ENCAP),
                tunsrc: Some([0x20; 16]),
                tundst: Some([0x30; 16]),
                trace: Some(Ioam6TraceHeader {
                    remlen: 2,
                    type_be32: 0x8040_0000,
                    ..trace
                }),
            },
        )
        .expect("encap state");
        assert_eq!(encap.freq_k, 2);
        assert_eq!(encap.freq_n, 5);
        assert_eq!(encap.mode, IOAM6_IPTUNNEL_MODE_ENCAP);
        assert!(encap.has_tunsrc);
        assert_eq!(encap.tundst, Some([0x30; 16]));
        assert!(encap.trace_padding.is_empty());
    }

    #[test]
    fn ioam6_lwtunnel_build_state_rejects_linux_invalid_netlink_attrs() {
        let trace = Ioam6TraceHeader {
            namespace_id: 1,
            nodelen: 0,
            remlen: 1,
            type_be32: 0x8000_0000,
        };

        assert_eq!(
            ioam6_build_lwtunnel_state(
                0,
                Ioam6LwtunnelConfig {
                    trace: Some(trace),
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_build_lwtunnel_state(
                AF_INET6,
                Ioam6LwtunnelConfig {
                    freq_k: Some(1),
                    trace: Some(trace),
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_build_lwtunnel_state(
                AF_INET6,
                Ioam6LwtunnelConfig {
                    freq_k: Some(6),
                    freq_n: Some(5),
                    trace: Some(trace),
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_build_lwtunnel_state(
                AF_INET6,
                Ioam6LwtunnelConfig {
                    mode: Some(IOAM6_IPTUNNEL_MODE_AUTO),
                    trace: Some(trace),
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_build_lwtunnel_state(
                AF_INET6,
                Ioam6LwtunnelConfig {
                    mode: Some(IOAM6_IPTUNNEL_MODE_INLINE),
                    tunsrc: Some([0x44; 16]),
                    trace: Some(trace),
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_build_lwtunnel_state(
                AF_INET6,
                Ioam6LwtunnelConfig {
                    mode: Some(IOAM6_IPTUNNEL_MODE_ENCAP),
                    tundst: Some([0; 16]),
                    trace: Some(trace),
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_build_lwtunnel_state(
                AF_INET6,
                Ioam6LwtunnelConfig {
                    trace: None,
                    ..Ioam6LwtunnelConfig::default()
                },
            ),
            Err(EINVAL)
        );
    }

    #[test]
    fn ioam6_trace_header_validation_computes_linux_nodelen_and_reserved_bits() {
        let trace_type = 0x8000_0000 | 0x4000_0000 | 0x0080_0000 | 0x0020_0000 | 0x0010_0000;
        assert_eq!(ioam6_trace_compute_nodelen(trace_type), 7);

        let validated = ioam6_validate_trace_header(Ioam6TraceHeader {
            namespace_id: 9,
            nodelen: 0,
            remlen: 61,
            type_be32: trace_type,
        })
        .expect("valid trace");
        assert_eq!(validated.nodelen, 7);

        assert_eq!(
            ioam6_validate_trace_header(Ioam6TraceHeader {
                type_be32: 0,
                ..validated
            }),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_validate_trace_header(Ioam6TraceHeader {
                remlen: 0,
                ..validated
            }),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_validate_trace_header(Ioam6TraceHeader {
                remlen: 62,
                ..validated
            }),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_validate_trace_header(Ioam6TraceHeader {
                type_be32: 0x0008_0000,
                ..validated
            }),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_validate_trace_header(Ioam6TraceHeader {
                type_be32: 0x0000_0100,
                ..validated
            }),
            Err(EINVAL)
        );
    }

    #[test]
    fn ioam6_namespace_schema_control_plane_matches_linux_genl_model() {
        let _guard = ioam6_test_guard();

        ioam6_add_namespace(7, Some(0x1234), Some(0x5678)).expect("add namespace");
        assert_eq!(ioam6_add_namespace(7, None, None), Err(EEXIST));
        ioam6_add_namespace(8, None, None).expect("add default namespace");
        let namespaces = ioam6_dump_namespaces();
        assert_eq!(namespaces.len(), 2);
        assert!(namespaces.iter().any(|ns| {
            ns.id == 8 && ns.data == IOAM6_U32_UNAVAILABLE && ns.data_wide == IOAM6_U64_UNAVAILABLE
        }));

        ioam6_add_schema(11, b"trace-schema").expect("add schema");
        assert_eq!(ioam6_add_schema(11, b"dupe"), Err(EEXIST));
        assert_eq!(ioam6_add_schema(12, &[]), Err(EINVAL));
        let too_big = alloc::vec![0u8; IOAM6_MAX_SCHEMA_DATA_LEN + 1];
        assert_eq!(ioam6_add_schema(12, &too_big), Err(E2BIG));

        ioam6_namespace_set_schema(7, Some(11)).expect("link schema");
        assert_eq!(
            ioam6_dump_namespaces()
                .iter()
                .find(|ns| ns.id == 7)
                .and_then(|ns| ns.schema_id),
            Some(11)
        );
        assert_eq!(
            ioam6_dump_schemas()
                .iter()
                .find(|sc| sc.id == 11)
                .and_then(|sc| sc.namespace_id),
            Some(7)
        );

        ioam6_namespace_set_schema(7, None).expect("unlink schema");
        assert_eq!(
            ioam6_dump_namespaces()
                .iter()
                .find(|ns| ns.id == 7)
                .and_then(|ns| ns.schema_id),
            None
        );

        assert_eq!(ioam6_namespace_set_schema(7, Some(99)), Err(ENOENT));
        assert_eq!(ioam6_del_schema(99), Err(ENOENT));
        ioam6_del_schema(11).expect("delete schema");
        ioam6_del_namespace(7).expect("delete namespace");
        assert_eq!(ioam6_del_namespace(7), Err(ENOENT));
    }

    #[test]
    fn ioam6_lwtunnel_output_fills_trace_data_and_emits_event() {
        let _guard = ioam6_test_guard();
        ioam6_set_event_listener_present(true);
        ioam6_add_namespace(7, Some(0x11223344), Some(0x0102030405060708)).expect("add namespace");
        ioam6_add_schema(11, b"schema").expect("add schema");
        ioam6_namespace_set_schema(7, Some(11)).expect("link schema");

        let output = ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 7, b"inner-payload")
            .expect("ioam output");

        let mut expected_trace = Vec::new();
        expected_trace.extend_from_slice(&7u16.to_be_bytes());
        expected_trace.extend_from_slice(&0x11223344u32.to_be_bytes());
        expected_trace.extend_from_slice(&0x0102030405060708u64.to_be_bytes());
        expected_trace.extend_from_slice(&11u32.to_be_bytes());
        expected_trace.extend_from_slice(b"schema");

        assert_eq!(output.namespace.id, 7);
        assert_eq!(output.schema.as_ref().map(|schema| schema.id), Some(11));
        assert_eq!(output.trace_data, expected_trace);
        assert_eq!(output.payload_offset, expected_trace.len());
        assert_eq!(&output.frame[..output.payload_offset], &expected_trace);
        assert_eq!(&output.frame[output.payload_offset..], b"inner-payload");
        assert_eq!(
            output.event,
            Ioam6TraceEvent {
                group: IOAM6_GENL_EV_GRP_NAME,
                name: IOAM6_EVENT_TRACE_FILLED,
                namespace_id: 7,
                schema_id: Some(11),
                trace_len: expected_trace.len(),
                payload_len: b"inner-payload".len(),
            }
        );
        assert_eq!(ioam6_events(), alloc::vec![output.event]);
    }

    #[test]
    fn ioam6_lwtunnel_output_rejects_missing_or_invalid_inputs() {
        let _guard = ioam6_test_guard();
        ioam6_add_namespace(8, None, None).expect("add namespace");

        assert_eq!(
            ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 99, b"payload"),
            Err(ENOENT)
        );
        assert_eq!(
            ioam6_lwtunnel_output(AF_INET6, 0, 8, b"payload"),
            Err(EINVAL)
        );
        assert_eq!(
            ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 8, b""),
            Err(EINVAL)
        );

        let output = ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 8, b"payload")
            .expect("namespace-only trace");
        assert_eq!(output.trace_data, 8u16.to_be_bytes().to_vec());
        assert_eq!(&output.frame[output.payload_offset..], b"payload");
        assert!(ioam6_events().is_empty());
    }

    #[test]
    fn ioam6_trace_events_follow_linux_multicast_listener_gate() {
        let _guard = ioam6_test_guard();
        ioam6_add_namespace(9, Some(0x01020304), None).expect("add namespace");

        let silent = ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 9, b"payload")
            .expect("trace without listeners");
        assert_eq!(silent.event.namespace_id, 9);
        assert!(ioam6_events().is_empty());

        ioam6_set_event_listener_present(true);
        let delivered = ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 9, b"payload")
            .expect("trace with listener");
        assert_eq!(ioam6_events(), alloc::vec![delivered.event.clone()]);

        ioam6_set_event_listener_present(false);
        let suppressed = ioam6_lwtunnel_output(AF_INET6, LWTUNNEL_ENCAP_IOAM6, 9, b"payload")
            .expect("trace after listener leaves");
        assert_eq!(suppressed.event.namespace_id, 9);
        assert_eq!(ioam6_events(), alloc::vec![delivered.event]);
    }

    #[test]
    fn mip6_registers_af_inet6_xfrm_types_and_mh_filter() {
        assert_eq!(MIP6_XFRM_TYPES.len(), 2);
        assert_eq!(
            MIP6_XFRM_TYPES[0],
            Mip6XfrmRegistration {
                name: "mip6_destopt_type",
                family: AF_INET6,
                proto: IPPROTO_DSTOPTS,
                route_optimization_only: true,
            }
        );
        assert_eq!(
            MIP6_XFRM_TYPES[1],
            Mip6XfrmRegistration {
                name: "mip6_rthdr_type",
                family: AF_INET6,
                proto: IPPROTO_ROUTING,
                route_optimization_only: true,
            }
        );
        assert_eq!(validate_mip6_xfrm_type(AF_INET6, IPPROTO_DSTOPTS), Ok(()));
        assert_eq!(validate_mip6_xfrm_type(AF_INET6, IPPROTO_ROUTING), Ok(()));
        assert_eq!(validate_mip6_xfrm_type(AF_INET6, IPPROTO_MH), Err(EINVAL));
        assert_eq!(rawv6_mh_filter_protocol(), IPPROTO_MH);
    }

    #[test]
    fn mip6_destopt_output_rewrites_source_and_inserts_home_address_option() {
        let src = [0x11; 16];
        let dst = [0x22; 16];
        let care_of = [0x33; 16];
        let packet = Mip6Packet {
            next_header: IPPROTO_MH,
            src,
            dst,
            payload: b"mh-payload".to_vec(),
        };

        let output =
            mip6_xfrm_output(AF_INET6, IPPROTO_DSTOPTS, &packet, care_of).expect("destopt output");

        assert_eq!(output.proto, IPPROTO_DSTOPTS);
        assert_eq!(output.src, care_of);
        assert_eq!(output.dst, dst);
        assert_eq!(output.home_address, src);
        assert_eq!(output.payload_offset, MIP6_DESTOPT_HEADER_LEN);
        assert_eq!(
            &output.extension_header[..8],
            &[IPPROTO_MH, 2, IPV6_TLV_PADN, 2, 0, 0, IPV6_TLV_HAO, 16]
        );
        assert_eq!(&output.extension_header[8..24], &src);
        assert_eq!(&output.frame[output.payload_offset..], b"mh-payload");
        assert_eq!(mip6_xfrm_input(AF_INET6, &output, care_of), Ok(IPPROTO_MH));
        assert_eq!(mip6_xfrm_input(AF_INET6, &output, [0x44; 16]), Err(ENOENT));
    }

    #[test]
    fn mip6_rthdr_output_rewrites_destination_and_inserts_type2_routing_header() {
        let src = [0xaa; 16];
        let dst = [0xbb; 16];
        let care_of = [0xcc; 16];
        let packet = Mip6Packet {
            next_header: 17,
            src,
            dst,
            payload: b"udp-payload".to_vec(),
        };

        let output =
            mip6_xfrm_output(AF_INET6, IPPROTO_ROUTING, &packet, care_of).expect("rthdr output");

        assert_eq!(output.proto, IPPROTO_ROUTING);
        assert_eq!(output.src, src);
        assert_eq!(output.dst, care_of);
        assert_eq!(output.home_address, dst);
        assert_eq!(output.payload_offset, MIP6_RTHDR_HEADER_LEN);
        assert_eq!(
            &output.extension_header[..8],
            &[17, 2, IPV6_SRCRT_TYPE_2, 1, 0, 0, 0, 0]
        );
        assert_eq!(&output.extension_header[8..24], &dst);
        assert_eq!(&output.frame[output.payload_offset..], b"udp-payload");
        assert_eq!(mip6_xfrm_input(AF_INET6, &output, care_of), Ok(17));
        assert_eq!(mip6_xfrm_input(AF_INET6, &output, [0xdd; 16]), Err(ENOENT));
    }

    #[test]
    fn mip6_xfrm_output_and_mh_filter_reject_linux_invalid_preconditions() {
        let packet = Mip6Packet {
            next_header: IPPROTO_MH,
            src: [1; 16],
            dst: [2; 16],
            payload: b"payload".to_vec(),
        };
        assert_eq!(
            mip6_xfrm_output(0, IPPROTO_DSTOPTS, &packet, [3; 16]),
            Err(EINVAL)
        );
        assert_eq!(
            mip6_xfrm_output(AF_INET6, IPPROTO_MH, &packet, [3; 16]),
            Err(EINVAL)
        );
        let empty = Mip6Packet {
            payload: Vec::new(),
            ..packet
        };
        assert_eq!(
            mip6_xfrm_output(AF_INET6, IPPROTO_DSTOPTS, &empty, [3; 16]),
            Err(EINVAL)
        );

        assert_eq!(
            mip6_mh_filter(
                Mip6MobilityHeader {
                    payload_proto: IPPROTO_NONE,
                    hdrlen: 1,
                    mh_type: IP6_MH_TYPE_BU,
                },
                16,
            ),
            Ok(())
        );
        assert_eq!(
            mip6_mh_filter(
                Mip6MobilityHeader {
                    payload_proto: IPPROTO_NONE,
                    hdrlen: 0,
                    mh_type: IP6_MH_TYPE_BU,
                },
                8,
            ),
            Err(EINVAL)
        );
        assert_eq!(
            mip6_mh_filter(
                Mip6MobilityHeader {
                    payload_proto: IPPROTO_MH,
                    hdrlen: 1,
                    mh_type: IP6_MH_TYPE_BU,
                },
                16,
            ),
            Err(EINVAL)
        );
        assert_eq!(
            mip6_mh_filter(
                Mip6MobilityHeader {
                    payload_proto: IPPROTO_NONE,
                    hdrlen: 2,
                    mh_type: IP6_MH_TYPE_HOT,
                },
                16,
            ),
            Err(EINVAL)
        );
    }

    #[test]
    fn mip6_mh_filter_reports_parameter_problem_for_linux_reported_fields() {
        let too_short = Mip6MobilityHeader {
            payload_proto: IPPROTO_NONE,
            hdrlen: 0,
            mh_type: IP6_MH_TYPE_BU,
        };
        assert_eq!(
            mip6_mh_filter_report(too_short, 8, IPV6_HEADER_LEN),
            Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
                reason: Mip6MhRejectReason::MessageTooShort,
                parameter_problem: Some(Mip6Icmpv6ParameterProblem {
                    icmp_type: ICMPV6_PARAMPROB,
                    code: ICMPV6_HDR_FIELD,
                    pointer: IPV6_HEADER_LEN + MIP6_MH_HDRLEN_OFFSET,
                }),
            })
        );

        let invalid_payload_proto = Mip6MobilityHeader {
            payload_proto: IPPROTO_MH,
            hdrlen: 1,
            mh_type: IP6_MH_TYPE_BU,
        };
        assert_eq!(
            mip6_mh_filter_report(invalid_payload_proto, 16, IPV6_HEADER_LEN),
            Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
                reason: Mip6MhRejectReason::InvalidPayloadProtocol,
                parameter_problem: Some(Mip6Icmpv6ParameterProblem {
                    icmp_type: ICMPV6_PARAMPROB,
                    code: ICMPV6_HDR_FIELD,
                    pointer: IPV6_HEADER_LEN + MIP6_MH_PROTO_OFFSET,
                }),
            })
        );
        assert_eq!(mip6_mh_filter(invalid_payload_proto, 16), Err(EINVAL));
    }

    #[test]
    fn mip6_mh_filter_drops_without_report_for_unavailable_or_overlong_headers() {
        let header = Mip6MobilityHeader {
            payload_proto: IPPROTO_NONE,
            hdrlen: 2,
            mh_type: IP6_MH_TYPE_HOT,
        };

        assert_eq!(
            mip6_mh_filter_report(header, MIP6_MH_BASE_LEN - 1, IPV6_HEADER_LEN),
            Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
                reason: Mip6MhRejectReason::HeaderUnavailable,
                parameter_problem: None,
            })
        );
        assert_eq!(
            mip6_mh_filter_report(header, 16, IPV6_HEADER_LEN),
            Mip6MhFilterOutcome::Drop(Mip6MhFilterDrop {
                reason: Mip6MhRejectReason::HeaderLengthExceedsPacket,
                parameter_problem: None,
            })
        );
    }

    #[test]
    fn niche_skb_paths_cover_mpls_ioam_and_mip6_runtime_acceptance() {
        let _guard = ioam6_test_guard();
        run_niche_acceptance().expect("skb-backed niche networking acceptance");
    }

    #[test]
    fn niche_init_records_registered_features() {
        init();
        assert_eq!(
            registration_snapshot(),
            NicheNetRegistrationState {
                mpls_gso: true,
                mpls_packet_offloads: 2,
                ioam6_pernet_subsys: true,
                ioam6_genl_family: true,
                ioam6_lwtunnel: true,
                mip6_xfrm_types: 2,
                mip6_rawv6_mh_filter: true,
            }
        );
    }

    fn mpls_test_packet(ethertype: u16, gso_size: usize, payload_len: usize) -> MplsGsoPacket {
        MplsGsoPacket {
            ethertype,
            inner_protocol: Some(ETH_P_IP),
            mac_header: b"macmac".to_vec(),
            mpls_header: alloc::vec![0x00, 0x00, 0x01, 0xff, 0x00, 0x00, 0x02, 0xff],
            inner_payload: (0..payload_len).map(|byte| byte as u8).collect(),
            gso_size,
            requested_features: 0b1110,
            device_mpls_features: 0b1010,
        }
    }
}
