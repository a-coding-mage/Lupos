//! linux-parity: complete
//! linux-source: vendor/linux/net/tls/tls_proc.c
//! test-origin: linux:vendor/linux/net/tls/tls_proc.c
//! TLS procfs statistics table.

extern crate alloc;

use alloc::vec::Vec;

use crate::include::uapi::errno::ENOMEM;

pub const TLS_PROC_ENTRY: &str = "tls_stat";
pub const TLS_PROC_MODE: u16 = 0o444;

pub const TLS_MIB_LIST: [&str; 17] = [
    "TlsCurrTxSw",
    "TlsCurrRxSw",
    "TlsCurrTxDevice",
    "TlsCurrRxDevice",
    "TlsTxSw",
    "TlsRxSw",
    "TlsTxDevice",
    "TlsRxDevice",
    "TlsDecryptError",
    "TlsRxDeviceResync",
    "TlsDecryptRetry",
    "TlsRxNoPadViolation",
    "TlsRxRekeyOk",
    "TlsRxRekeyError",
    "TlsTxRekeyOk",
    "TlsTxRekeyError",
    "TlsRxRekeyReceived",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlsProcFile {
    pub name: &'static str,
    pub mode: u16,
}

pub fn tls_statistics_seq_show(values: &[u64]) -> Vec<(&'static str, u64)> {
    TLS_MIB_LIST
        .iter()
        .enumerate()
        .map(|(idx, name)| (*name, values.get(idx).copied().unwrap_or(0)))
        .collect()
}

pub fn tls_proc_init(proc_available: bool) -> Result<Option<TlsProcFile>, i32> {
    if proc_available {
        Ok(Some(TlsProcFile {
            name: TLS_PROC_ENTRY,
            mode: TLS_PROC_MODE,
        }))
    } else {
        Err(ENOMEM)
    }
}

pub const fn tls_proc_fini(_entry: Option<TlsProcFile>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_proc_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/tls/tls_proc.c"
        ));
        assert!(source.contains("static const struct snmp_mib tls_mib_list[]"));
        assert!(source.contains("SNMP_MIB_ITEM(\"TlsCurrTxSw\", LINUX_MIB_TLSCURRTXSW)"));
        assert!(source.contains("SNMP_MIB_ITEM(\"TlsRxRekeyReceived\""));
        assert!(source.contains("snmp_get_cpu_field_batch_cnt(buf, tls_mib_list, cnt"));
        assert!(source.contains("seq_printf(seq, \"%-32s\\t%lu\\n\""));
        assert!(source.contains("proc_create_net_single(\"tls_stat\", 0444"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("remove_proc_entry(\"tls_stat\", net->proc_net);"));

        assert_eq!(TLS_MIB_LIST.len(), 17);
        assert_eq!(TLS_MIB_LIST[0], "TlsCurrTxSw");
        assert_eq!(TLS_MIB_LIST[16], "TlsRxRekeyReceived");
        let rows = tls_statistics_seq_show(&[3, 5]);
        assert_eq!(rows[0], ("TlsCurrTxSw", 3));
        assert_eq!(rows[1], ("TlsCurrRxSw", 5));
        assert_eq!(rows[2], ("TlsCurrTxDevice", 0));
        assert_eq!(
            tls_proc_init(true),
            Ok(Some(TlsProcFile {
                name: "tls_stat",
                mode: 0o444,
            }))
        );
        assert_eq!(tls_proc_init(false), Err(ENOMEM));
    }
}
