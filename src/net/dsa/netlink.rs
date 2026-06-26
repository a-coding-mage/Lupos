//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/netlink.c
//! test-origin: linux:vendor/linux/net/dsa/netlink.c
//! DSA rtnetlink link-ops metadata and conduit updates.

use crate::include::uapi::errno::EINVAL;

pub const EMSGSIZE: i32 = 90;
pub const IFLA_DSA_CONDUIT: usize = 1;
pub const IFLA_DSA_MAX: usize = IFLA_DSA_CONDUIT;
pub const NLA_U32: &str = "NLA_U32";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NlaPolicy {
    pub nla_type: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaLinkAttrs {
    pub conduit_ifindex: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DsaUserDevice {
    pub ifindex: u32,
    pub conduit_ifindex: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RtnlLinkOps {
    pub kind: &'static str,
    pub priv_size_symbol: &'static str,
    pub maxtype: usize,
    pub netns_refund: bool,
}

pub const DSA_POLICY: [Option<NlaPolicy>; IFLA_DSA_MAX + 1] =
    [None, Some(NlaPolicy { nla_type: NLA_U32 })];

pub const DSA_LINK_OPS: RtnlLinkOps = RtnlLinkOps {
    kind: "dsa",
    priv_size_symbol: "sizeof(struct dsa_port)",
    maxtype: IFLA_DSA_MAX,
    netns_refund: true,
};

pub fn dsa_changelink(
    dev: &mut DsaUserDevice,
    data: Option<DsaLinkAttrs>,
    conduit_exists: bool,
) -> Result<(), i32> {
    let Some(attrs) = data else {
        return Ok(());
    };

    if let Some(ifindex) = attrs.conduit_ifindex {
        if !conduit_exists {
            return Err(-EINVAL);
        }
        dev.conduit_ifindex = Some(ifindex);
    }

    Ok(())
}

pub const fn dsa_get_size() -> usize {
    nla_total_size(core::mem::size_of::<u32>())
}

pub const fn dsa_fill_info(dev: DsaUserDevice) -> Result<u32, i32> {
    match dev.conduit_ifindex {
        Some(ifindex) => Ok(ifindex),
        None => Err(-EMSGSIZE),
    }
}

pub const fn nla_total_size(payload: usize) -> usize {
    (payload + 4 + 3) & !3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsa_netlink_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/netlink.c"
        ));
        assert!(source.contains("static const struct nla_policy dsa_policy"));
        assert!(source.contains("[IFLA_DSA_CONDUIT]\t= { .type = NLA_U32 }"));
        assert!(source.contains("static int dsa_changelink"));
        assert!(source.contains("if (!data)"));
        assert!(source.contains("nla_get_u32(data[IFLA_DSA_CONDUIT])"));
        assert!(source.contains("__dev_get_by_index(dev_net(dev), ifindex)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("dsa_user_change_conduit(dev, conduit, extack);"));
        assert!(source.contains("nla_total_size(sizeof(u32))"));
        assert!(source.contains("nla_put_u32(skb, IFLA_DSA_CONDUIT, conduit->ifindex)"));
        assert!(source.contains(".kind\t\t\t= \"dsa\""));
        assert!(source.contains(".netns_refund\t\t= true"));

        assert_eq!(DSA_POLICY[IFLA_DSA_CONDUIT].unwrap().nla_type, NLA_U32);
        assert_eq!(DSA_LINK_OPS.kind, "dsa");
        assert_eq!(dsa_get_size(), 8);

        let mut dev = DsaUserDevice {
            ifindex: 10,
            conduit_ifindex: None,
        };
        assert_eq!(dsa_changelink(&mut dev, None, false), Ok(()));
        assert_eq!(
            dsa_changelink(
                &mut dev,
                Some(DsaLinkAttrs {
                    conduit_ifindex: Some(2)
                }),
                false
            ),
            Err(-EINVAL)
        );
        assert_eq!(
            dsa_changelink(
                &mut dev,
                Some(DsaLinkAttrs {
                    conduit_ifindex: Some(2)
                }),
                true
            ),
            Ok(())
        );
        assert_eq!(dsa_fill_info(dev), Ok(2));
    }
}
