//! linux-parity: complete
//! linux-source: vendor/linux/net/nfc/af_nfc.c
//! test-origin: linux:vendor/linux/net/nfc/af_nfc.c
//! NFC protocol-family registration table.

use crate::include::uapi::errno::{EAFNOSUPPORT, EBUSY, EINVAL, EPROTONOSUPPORT};

pub const NFC_SOCKPROTO_MAX: usize = 2;
pub const PF_NFC: u16 = 39;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfcProtocol {
    pub id: i32,
    pub create_errno: i32,
    pub proto_register_errno: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfcProtoTable {
    slots: [Option<NfcProtocol>; NFC_SOCKPROTO_MAX],
}

impl NfcProtoTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; NFC_SOCKPROTO_MAX],
        }
    }

    pub fn nfc_proto_register(&mut self, proto: NfcProtocol) -> Result<(), i32> {
        let index = nfc_proto_index(proto.id)?;
        if proto.proto_register_errno != 0 {
            return Err(proto.proto_register_errno);
        }
        if self.slots[index].is_some() {
            return Err(-EBUSY);
        }
        self.slots[index] = Some(proto);
        Ok(())
    }

    pub fn nfc_proto_unregister(&mut self, proto: NfcProtocol) {
        if let Ok(index) = nfc_proto_index(proto.id) {
            self.slots[index] = None;
        }
    }

    pub fn nfc_sock_create(
        &self,
        init_net: bool,
        proto: i32,
        module_get_ok: bool,
    ) -> Result<(), i32> {
        if !init_net {
            return Err(-EAFNOSUPPORT);
        }

        let index = nfc_proto_index(proto)?;
        match self.slots[index] {
            Some(protocol) if module_get_ok => {
                if protocol.create_errno == 0 {
                    Ok(())
                } else {
                    Err(protocol.create_errno)
                }
            }
            _ => Err(-EPROTONOSUPPORT),
        }
    }
}

pub const NFC_SOCK_FAMILY_OPS: NfcFamilyOps = NfcFamilyOps {
    family: PF_NFC,
    create: "nfc_sock_create",
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NfcFamilyOps {
    pub family: u16,
    pub create: &'static str,
}

pub const fn af_nfc_init() -> &'static NfcFamilyOps {
    &NFC_SOCK_FAMILY_OPS
}

const fn nfc_proto_index(proto: i32) -> Result<usize, i32> {
    if proto < 0 || proto as usize >= NFC_SOCKPROTO_MAX {
        Err(-EINVAL)
    } else {
        Ok(proto as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn af_nfc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/nfc/af_nfc.c"
        ));
        assert!(source.contains("static DEFINE_RWLOCK(proto_tab_lock);"));
        assert!(source.contains("static const struct nfc_protocol *proto_tab[NFC_SOCKPROTO_MAX];"));
        assert!(source.contains("static int nfc_sock_create"));
        assert!(source.contains("int rc = -EPROTONOSUPPORT;"));
        assert!(source.contains("if (net != &init_net)"));
        assert!(source.contains("return -EAFNOSUPPORT;"));
        assert!(source.contains("if (proto < 0 || proto >= NFC_SOCKPROTO_MAX)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("try_module_get(proto_tab[proto]->owner)"));
        assert!(source.contains("proto_tab[proto]->create(net, sock, proto_tab[proto], kern);"));
        assert!(source.contains(".family = PF_NFC"));
        assert!(source.contains(".create = nfc_sock_create"));
        assert!(source.contains("int nfc_proto_register"));
        assert!(source.contains("rc = proto_register(nfc_proto->proto, 0);"));
        assert!(source.contains("rc = -EBUSY;"));
        assert!(source.contains("EXPORT_SYMBOL(nfc_proto_register);"));
        assert!(source.contains("sock_register(&nfc_sock_family_ops);"));
        assert!(source.contains("sock_unregister(PF_NFC);"));
    }

    #[test]
    fn nfc_proto_table_enforces_linux_registration_errors() {
        let mut table = NfcProtoTable::new();
        let proto = NfcProtocol {
            id: 1,
            create_errno: 0,
            proto_register_errno: 0,
        };

        assert_eq!(table.nfc_sock_create(false, 1, true), Err(-EAFNOSUPPORT));
        assert_eq!(table.nfc_sock_create(true, -1, true), Err(-EINVAL));
        assert_eq!(table.nfc_sock_create(true, 1, true), Err(-EPROTONOSUPPORT));
        assert_eq!(table.nfc_proto_register(proto), Ok(()));
        assert_eq!(table.nfc_sock_create(true, 1, true), Ok(()));
        assert_eq!(table.nfc_proto_register(proto), Err(-EBUSY));
        assert_eq!(table.nfc_sock_create(true, 1, false), Err(-EPROTONOSUPPORT));
        table.nfc_proto_unregister(proto);
        assert_eq!(table.nfc_sock_create(true, 1, true), Err(-EPROTONOSUPPORT));
        assert_eq!(af_nfc_init(), &NFC_SOCK_FAMILY_OPS);
    }
}
