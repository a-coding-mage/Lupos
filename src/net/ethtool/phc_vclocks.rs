//! linux-parity: complete
//! linux-source: vendor/linux/net/ethtool/phc_vclocks.c
//! test-origin: linux:vendor/linux/net/ethtool/phc_vclocks.c
//! ethtool PHC virtual clocks netlink reply.

extern crate alloc;

use alloc::vec::Vec;

pub const EMSGSIZE: i32 = 90;
pub const ETHTOOL_MSG_PHC_VCLOCKS_GET: u8 = 1;
pub const ETHTOOL_MSG_PHC_VCLOCKS_GET_REPLY: u8 = 2;
pub const ETHTOOL_A_PHC_VCLOCKS_HEADER: u16 = 1;
pub const ETHTOOL_A_PHC_VCLOCKS_NUM: u16 = 2;
pub const ETHTOOL_A_PHC_VCLOCKS_INDEX: u16 = 3;
pub const NLA_HDRLEN: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhcVclocksReplyData {
    pub num: i32,
    pub index: Vec<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhcVclocksAttrs {
    pub num: Option<u32>,
    pub index: Vec<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EthnlRequestOps {
    pub request_cmd: u8,
    pub reply_cmd: u8,
    pub hdr_attr: u16,
    pub req_info_size: usize,
    pub reply_data_size: usize,
}

pub const ETHNL_PHC_VCLOCKS_REQUEST_OPS: EthnlRequestOps = EthnlRequestOps {
    request_cmd: ETHTOOL_MSG_PHC_VCLOCKS_GET,
    reply_cmd: ETHTOOL_MSG_PHC_VCLOCKS_GET_REPLY,
    hdr_attr: ETHTOOL_A_PHC_VCLOCKS_HEADER,
    req_info_size: 1,
    reply_data_size: core::mem::size_of::<PhcVclocksReplyData>(),
};

pub fn phc_vclocks_prepare_data(
    ethnl_ops_begin_ret: i32,
    device_indices: &[i32],
) -> Result<PhcVclocksReplyData, i32> {
    if ethnl_ops_begin_ret < 0 {
        return Err(ethnl_ops_begin_ret);
    }
    Ok(PhcVclocksReplyData {
        num: device_indices.len() as i32,
        index: device_indices.to_vec(),
    })
}

pub fn phc_vclocks_reply_size(data: &PhcVclocksReplyData) -> usize {
    if data.num <= 0 {
        return 0;
    }
    nla_total_size(core::mem::size_of::<u32>())
        + nla_total_size(core::mem::size_of::<i32>() * data.num as usize)
}

pub fn phc_vclocks_fill_reply(
    data: &PhcVclocksReplyData,
    can_put_attrs: bool,
) -> Result<PhcVclocksAttrs, i32> {
    if data.num <= 0 {
        return Ok(PhcVclocksAttrs {
            num: None,
            index: Vec::new(),
        });
    }
    if !can_put_attrs {
        return Err(-EMSGSIZE);
    }
    Ok(PhcVclocksAttrs {
        num: Some(data.num as u32),
        index: data.index.clone(),
    })
}

pub fn phc_vclocks_cleanup_data(data: &mut PhcVclocksReplyData) {
    data.index.clear();
    data.num = 0;
}

pub const fn nla_total_size(payload_len: usize) -> usize {
    NLA_HDRLEN + ((payload_len + 3) & !3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn phc_vclocks_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ethtool/phc_vclocks.c"
        ));
        assert!(source.contains("struct phc_vclocks_req_info"));
        assert!(source.contains("struct phc_vclocks_reply_data"));
        assert!(source.contains("int\t\t\t\tnum;"));
        assert!(source.contains("int\t\t\t\t*index;"));
        assert!(source.contains("const struct nla_policy ethnl_phc_vclocks_get_policy[]"));
        assert!(source.contains("[ETHTOOL_A_PHC_VCLOCKS_HEADER] = NLA_POLICY_NESTED"));
        assert!(source.contains("static int phc_vclocks_prepare_data"));
        assert!(source.contains("ret = ethnl_ops_begin(dev);"));
        assert!(source.contains("if (ret < 0)"));
        assert!(source.contains("data->num = ethtool_get_phc_vclocks(dev, &data->index);"));
        assert!(source.contains("ethnl_ops_complete(dev);"));
        assert!(source.contains("static int phc_vclocks_reply_size"));
        assert!(source.contains("len += nla_total_size(sizeof(u32));"));
        assert!(source.contains("len += nla_total_size(sizeof(s32) * data->num);"));
        assert!(source.contains("static int phc_vclocks_fill_reply"));
        assert!(source.contains("if (data->num <= 0)"));
        assert!(source.contains("nla_put_u32(skb, ETHTOOL_A_PHC_VCLOCKS_NUM, data->num)"));
        assert!(source.contains("return -EMSGSIZE;"));
        assert!(source.contains("kfree(data->index);"));
        assert!(source.contains("const struct ethnl_request_ops ethnl_phc_vclocks_request_ops"));
        assert!(source.contains(".request_cmd\t\t= ETHTOOL_MSG_PHC_VCLOCKS_GET"));
    }

    #[test]
    fn phc_vclocks_reply_size_and_fill_follow_num_guard() {
        assert_eq!(phc_vclocks_prepare_data(-5, &[1, 2]), Err(-5));
        let mut data = phc_vclocks_prepare_data(0, &[4, 7, 9]).unwrap();
        assert_eq!(data.num, 3);
        assert_eq!(phc_vclocks_reply_size(&data), 24);
        assert_eq!(phc_vclocks_fill_reply(&data, false), Err(-EMSGSIZE));
        assert_eq!(
            phc_vclocks_fill_reply(&data, true).unwrap(),
            PhcVclocksAttrs {
                num: Some(3),
                index: vec![4, 7, 9],
            }
        );
        phc_vclocks_cleanup_data(&mut data);
        assert_eq!(phc_vclocks_reply_size(&data), 0);
        assert_eq!(data.index, Vec::<i32>::new());
    }
}
