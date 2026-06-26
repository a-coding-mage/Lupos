//! linux-parity: complete
//! linux-source: vendor/linux/net/dsa/trace.c
//! test-origin: linux:vendor/linux/net/dsa/trace.c
//! DSA trace helper formatting.

extern crate alloc;

use alloc::format;
use alloc::string::String;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsaDb<'a> {
    Port { name: &'a str },
    Lag { dev_name: &'a str, id: i32 },
    Bridge { dev_name: &'a str, num: i32 },
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DsaPortType {
    User,
    Cpu,
    Dsa,
    Unused,
}

pub fn dsa_db_print(db: &DsaDb<'_>) -> String {
    match *db {
        DsaDb::Port { name } => format!("port {name}"),
        DsaDb::Lag { dev_name, id } => format!("lag {dev_name} id {id}"),
        DsaDb::Bridge { dev_name, num } => format!("bridge {dev_name} num {num}"),
        DsaDb::Unknown => String::from("unknown"),
    }
}

pub const fn dsa_port_kind(port_type: DsaPortType) -> &'static str {
    match port_type {
        DsaPortType::User => "user",
        DsaPortType::Cpu => "cpu",
        DsaPortType::Dsa => "dsa",
        DsaPortType::Unused => "unused",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsa_trace_helpers_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/dsa/trace.c"
        ));
        assert!(source.contains("#define CREATE_TRACE_POINTS"));
        assert!(source.contains("void dsa_db_print(const struct dsa_db *db"));
        assert!(source.contains("case DSA_DB_PORT:"));
        assert!(source.contains("sprintf(buf, \"port %s\", db->dp->name);"));
        assert!(source.contains("case DSA_DB_LAG:"));
        assert!(source.contains("sprintf(buf, \"lag %s id %d\""));
        assert!(source.contains("case DSA_DB_BRIDGE:"));
        assert!(source.contains("sprintf(buf, \"bridge %s num %d\""));
        assert!(source.contains("const char *dsa_port_kind"));
        assert!(source.contains("case DSA_PORT_TYPE_USER:"));
        assert!(source.contains("return \"unused\";"));

        assert_eq!(dsa_db_print(&DsaDb::Port { name: "swp0" }), "port swp0");
        assert_eq!(
            dsa_db_print(&DsaDb::Lag {
                dev_name: "bond0",
                id: 7,
            }),
            "lag bond0 id 7"
        );
        assert_eq!(
            dsa_db_print(&DsaDb::Bridge {
                dev_name: "br0",
                num: 2,
            }),
            "bridge br0 num 2"
        );
        assert_eq!(dsa_db_print(&DsaDb::Unknown), "unknown");
        assert_eq!(dsa_port_kind(DsaPortType::User), "user");
        assert_eq!(dsa_port_kind(DsaPortType::Cpu), "cpu");
        assert_eq!(dsa_port_kind(DsaPortType::Dsa), "dsa");
        assert_eq!(dsa_port_kind(DsaPortType::Unused), "unused");
    }
}
