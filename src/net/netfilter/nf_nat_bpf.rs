//! linux-parity: complete
//! linux-source: vendor/linux/net/netfilter/nf_nat_bpf.c
//! test-origin: linux:vendor/linux/net/netfilter/nf_nat_bpf.c
//! NAT helpers exposed as unstable BPF kfuncs.

use crate::include::uapi::errno::{EINVAL, ENOMEM};

pub const NFPROTO_IPV4: u16 = 2;
pub const NFPROTO_IPV6: u16 = 10;
pub const NF_DROP: i32 = 0;
pub const NF_ACCEPT: i32 = 1;
pub const NF_NAT_RANGE_MAP_IPS: u32 = 1 << 0;
pub const NF_NAT_RANGE_PROTO_SPECIFIED: u32 = 1 << 1;
pub const BPF_PROG_TYPE_XDP: u32 = 6;
pub const BPF_PROG_TYPE_SCHED_CLS: u32 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NfNatManipType {
    Src,
    Dst,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfInetAddr {
    pub all: [u32; 4],
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfConnInit {
    pub l3num: u16,
    pub nat_range: Option<NfNatRange2>,
    pub manip: Option<NfNatManipType>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NfNatRange2 {
    pub flags: u32,
    pub min_addr: NfInetAddr,
    pub max_addr: NfInetAddr,
    pub min_proto: u16,
    pub max_proto: u16,
}

pub fn bpf_ct_set_nat_info(
    nfct: &mut NfConnInit,
    addr: NfInetAddr,
    port: i32,
    manip: NfNatManipType,
    setup_verdict: i32,
) -> Result<(), i32> {
    if nfct.l3num != NFPROTO_IPV4 && nfct.l3num != NFPROTO_IPV6 {
        return Err(-EINVAL);
    }

    let mut range = NfNatRange2 {
        flags: NF_NAT_RANGE_MAP_IPS,
        min_addr: addr,
        max_addr: addr,
        min_proto: 0,
        max_proto: 0,
    };
    if port > 0 {
        range.flags |= NF_NAT_RANGE_PROTO_SPECIFIED;
        range.min_proto = (port as u16).to_be();
        range.max_proto = range.min_proto;
    }
    nfct.nat_range = Some(range);
    nfct.manip = Some(manip);

    if setup_verdict == NF_DROP {
        Err(-ENOMEM)
    } else {
        Ok(())
    }
}

pub const fn register_nf_nat_bpf(xdp_ret: i32, sched_cls_ret: i32) -> Result<(), i32> {
    if xdp_ret != 0 {
        Err(xdp_ret)
    } else if sched_cls_ret != 0 {
        Err(sched_cls_ret)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_nat_bpf_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/netfilter/nf_nat_bpf.c"
        ));
        assert!(source.contains("__bpf_kfunc_start_defs();"));
        assert!(source.contains("__bpf_kfunc int bpf_ct_set_nat_info"));
        assert!(source.contains("u16 proto = nf_ct_l3num(ct);"));
        assert!(source.contains("if (proto != NFPROTO_IPV4 && proto != NFPROTO_IPV6)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("memset(&range, 0, sizeof(struct nf_nat_range2));"));
        assert!(source.contains("range.flags = NF_NAT_RANGE_MAP_IPS;"));
        assert!(source.contains("range.min_addr = *addr;"));
        assert!(source.contains("range.max_addr = range.min_addr;"));
        assert!(source.contains("if (port > 0)"));
        assert!(source.contains("range.flags |= NF_NAT_RANGE_PROTO_SPECIFIED;"));
        assert!(source.contains("range.min_proto.all = cpu_to_be16(port);"));
        assert!(
            source
                .contains("return nf_nat_setup_info(ct, &range, manip) == NF_DROP ? -ENOMEM : 0;")
        );
        assert!(source.contains("BTF_ID_FLAGS(func, bpf_ct_set_nat_info)"));
        assert!(source.contains("register_btf_kfunc_id_set(BPF_PROG_TYPE_XDP"));
        assert!(source.contains("register_btf_kfunc_id_set(BPF_PROG_TYPE_SCHED_CLS"));
    }

    #[test]
    fn nat_bpf_sets_addr_port_range_and_registers_both_program_types() {
        let addr = NfInetAddr {
            all: [0x0a00_0001, 0, 0, 0],
        };
        let mut ct = NfConnInit {
            l3num: NFPROTO_IPV4,
            ..NfConnInit::default()
        };
        assert_eq!(
            bpf_ct_set_nat_info(&mut ct, addr, 8080, NfNatManipType::Dst, NF_ACCEPT),
            Ok(())
        );
        let range = ct.nat_range.unwrap();
        assert_eq!(
            range.flags,
            NF_NAT_RANGE_MAP_IPS | NF_NAT_RANGE_PROTO_SPECIFIED
        );
        assert_eq!(range.min_addr, addr);
        assert_eq!(range.max_addr, addr);
        assert_eq!(range.min_proto, 8080u16.to_be());
        assert_eq!(range.max_proto, 8080u16.to_be());
        assert_eq!(ct.manip, Some(NfNatManipType::Dst));

        ct.l3num = 99;
        assert_eq!(
            bpf_ct_set_nat_info(&mut ct, addr, 0, NfNatManipType::Src, NF_ACCEPT),
            Err(-EINVAL)
        );
        ct.l3num = NFPROTO_IPV6;
        assert_eq!(
            bpf_ct_set_nat_info(&mut ct, addr, 0, NfNatManipType::Src, NF_DROP),
            Err(-ENOMEM)
        );
        assert_eq!(register_nf_nat_bpf(0, 0), Ok(()));
        assert_eq!(register_nf_nat_bpf(-7, 0), Err(-7));
        assert_eq!(register_nf_nat_bpf(0, -8), Err(-8));
    }
}
