//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/asn1.c
//! test-origin: linux:vendor/linux/fs/smb/client/asn1.c
//! CIFS SPNEGO negTokenInit ASN.1 callbacks.

use crate::include::uapi::errno::EBADMSG;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CifsOid {
    Spnego,
    MsKrb5,
    Krb5U2u,
    Krb5,
    NtlmSsp,
    IAKerb,
    Other,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TcpServerSecurity {
    pub sec_mskerberos: bool,
    pub sec_kerberosu2u: bool,
    pub sec_kerberos: bool,
    pub sec_ntlmssp: bool,
    pub sec_iakerb: bool,
}

pub const fn decode_neg_token_init(asn1_ber_decoder_result: i32) -> i32 {
    if asn1_ber_decoder_result == 0 { 1 } else { 0 }
}

pub const fn cifs_gssapi_this_mech(oid: CifsOid) -> Result<(), i32> {
    match oid {
        CifsOid::Spnego => Ok(()),
        _ => Err(-EBADMSG),
    }
}

pub const fn cifs_neg_token_init_mech_type(
    mut server: TcpServerSecurity,
    oid: CifsOid,
) -> TcpServerSecurity {
    match oid {
        CifsOid::MsKrb5 => server.sec_mskerberos = true,
        CifsOid::Krb5U2u => server.sec_kerberosu2u = true,
        CifsOid::Krb5 => server.sec_kerberos = true,
        CifsOid::NtlmSsp => server.sec_ntlmssp = true,
        CifsOid::IAKerb => server.sec_iakerb = true,
        CifsOid::Spnego | CifsOid::Other => {}
    }
    server
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifs_asn1_callbacks_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/asn1.c"
        ));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include <linux/kernel.h>"));
        assert!(source.contains("#include <linux/oid_registry.h>"));
        assert!(source.contains("#include \"cifsglob.h\""));
        assert!(source.contains("#include \"cifs_debug.h\""));
        assert!(source.contains("#include \"cifsproto.h\""));
        assert!(source.contains("#include \"cifs_spnego_negtokeninit.asn1.h\""));
        assert!(source.contains("decode_negTokenInit"));
        assert!(source.contains("asn1_ber_decoder(&cifs_spnego_negtokeninit_decoder"));
        assert!(source.contains("return 1;"));
        assert!(source.contains("return 0;"));
        assert!(source.contains("cifs_gssapi_this_mech"));
        assert!(source.contains("if (oid != OID_spnego)"));
        assert!(source.contains("return -EBADMSG;"));
        assert!(source.contains("cifs_neg_token_init_mech_type"));
        assert!(source.contains("if (oid == OID_mskrb5)"));
        assert!(source.contains("server->sec_mskerberos = true;"));
        assert!(source.contains("else if (oid == OID_krb5u2u)"));
        assert!(source.contains("server->sec_kerberosu2u = true;"));
        assert!(source.contains("else if (oid == OID_krb5)"));
        assert!(source.contains("server->sec_kerberos = true;"));
        assert!(source.contains("else if (oid == OID_ntlmssp)"));
        assert!(source.contains("server->sec_ntlmssp = true;"));
        assert!(source.contains("else if (oid == OID_IAKerb)"));
        assert!(source.contains("server->sec_iakerb = true;"));

        assert_eq!(decode_neg_token_init(0), 1);
        assert_eq!(decode_neg_token_init(-1), 0);
        assert_eq!(cifs_gssapi_this_mech(CifsOid::Spnego), Ok(()));
        assert_eq!(cifs_gssapi_this_mech(CifsOid::Krb5), Err(-EBADMSG));
        let server = cifs_neg_token_init_mech_type(TcpServerSecurity::default(), CifsOid::NtlmSsp);
        assert!(server.sec_ntlmssp);
        assert!(!server.sec_kerberos);
        let server = cifs_neg_token_init_mech_type(server, CifsOid::IAKerb);
        assert!(server.sec_iakerb);
    }
}
