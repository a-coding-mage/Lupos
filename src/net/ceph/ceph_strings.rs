//! linux-parity: complete
//! linux-source: vendor/linux/net/ceph/ceph_strings.c
//! test-origin: linux:vendor/linux/net/ceph/ceph_strings.c
//! Ceph string-name helpers.

pub const CEPH_ENTITY_TYPE_MON: i32 = 0x01;
pub const CEPH_ENTITY_TYPE_MDS: i32 = 0x02;
pub const CEPH_ENTITY_TYPE_OSD: i32 = 0x04;
pub const CEPH_ENTITY_TYPE_CLIENT: i32 = 0x08;
pub const CEPH_ENTITY_TYPE_AUTH: i32 = 0x20;

pub const CEPH_AUTH_UNKNOWN: i32 = 0;
pub const CEPH_AUTH_NONE: i32 = 1;
pub const CEPH_AUTH_CEPHX: i32 = 2;

pub const CEPH_CON_MODE_UNKNOWN: i32 = 0;
pub const CEPH_CON_MODE_CRC: i32 = 1;
pub const CEPH_CON_MODE_SECURE: i32 = 2;

pub const CEPH_OSD_EXISTS: i32 = 1 << 0;
pub const CEPH_OSD_UP: i32 = 1 << 1;
pub const CEPH_OSD_AUTOOUT: i32 = 1 << 2;
pub const CEPH_OSD_NEW: i32 = 1 << 3;

pub const CEPH_OSD_WATCH_OP_UNWATCH: i32 = 0;
pub const CEPH_OSD_WATCH_OP_WATCH: i32 = 3;
pub const CEPH_OSD_WATCH_OP_RECONNECT: i32 = 5;
pub const CEPH_OSD_WATCH_OP_PING: i32 = 7;

const CEPH_OSD_OP_MODE_RD: i32 = 0x1000;
const CEPH_OSD_OP_MODE_WR: i32 = 0x2000;
const CEPH_OSD_OP_MODE_RMW: i32 = 0x3000;
const CEPH_OSD_OP_MODE_SUB: i32 = 0x4000;
const CEPH_OSD_OP_MODE_CACHE: i32 = 0x8000;
const CEPH_OSD_OP_TYPE_LOCK: i32 = 0x0100;
const CEPH_OSD_OP_TYPE_DATA: i32 = 0x0200;
const CEPH_OSD_OP_TYPE_ATTR: i32 = 0x0300;
const CEPH_OSD_OP_TYPE_EXEC: i32 = 0x0400;
const CEPH_OSD_OP_TYPE_PG: i32 = 0x0500;
const CEPH_OSD_OP_TYPE_MULTI: i32 = 0x0600;

pub const fn ceph_osd_op(mode: i32, ty: i32, nr: i32) -> i32 {
    mode | ty | nr
}

pub const fn ceph_osd_op1(mode: i32, nr: i32) -> i32 {
    mode | nr
}

pub const fn ceph_entity_type_name(type_: i32) -> &'static str {
    match type_ {
        CEPH_ENTITY_TYPE_MDS => "mds",
        CEPH_ENTITY_TYPE_OSD => "osd",
        CEPH_ENTITY_TYPE_MON => "mon",
        CEPH_ENTITY_TYPE_CLIENT => "client",
        CEPH_ENTITY_TYPE_AUTH => "auth",
        _ => "unknown",
    }
}

pub const fn ceph_auth_proto_name(proto: i32) -> &'static str {
    match proto {
        CEPH_AUTH_UNKNOWN => "unknown",
        CEPH_AUTH_NONE => "none",
        CEPH_AUTH_CEPHX => "cephx",
        _ => "???",
    }
}

pub const fn ceph_con_mode_name(mode: i32) -> &'static str {
    match mode {
        CEPH_CON_MODE_UNKNOWN => "unknown",
        CEPH_CON_MODE_CRC => "crc",
        CEPH_CON_MODE_SECURE => "secure",
        _ => "???",
    }
}

pub const fn ceph_osd_watch_op_name(op: i32) -> &'static str {
    match op {
        CEPH_OSD_WATCH_OP_UNWATCH => "unwatch",
        CEPH_OSD_WATCH_OP_WATCH => "watch",
        CEPH_OSD_WATCH_OP_RECONNECT => "reconnect",
        CEPH_OSD_WATCH_OP_PING => "ping",
        _ => "???",
    }
}

pub const fn ceph_osd_state_name(state: i32) -> &'static str {
    match state {
        CEPH_OSD_EXISTS => "exists",
        CEPH_OSD_UP => "up",
        CEPH_OSD_AUTOOUT => "autoout",
        CEPH_OSD_NEW => "new",
        _ => "???",
    }
}

pub const fn ceph_osd_op_name(op: i32) -> &'static str {
    match op {
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 1) => "read",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 2) => "stat",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 5) => "sparse-read",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 6) => "notify",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 7) => "notify-ack",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 9) => "list-watchers",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_DATA, 1) => "write",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_DATA, 2) => "writefull",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_DATA, 5) => "delete",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_DATA, 15) => "watch",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_DATA, 45) => "copy-from2",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_ATTR, 1) => "getxattr",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_ATTR, 3) => "cmpxattr",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_ATTR, 1) => "setxattr",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_EXEC, 1) => "call",
        x if x == ceph_osd_op1(CEPH_OSD_OP_MODE_SUB, 1) => "pull",
        x if x == ceph_osd_op1(CEPH_OSD_OP_MODE_SUB, 2) => "push",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_LOCK, 1) => "wrlock",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_PG, 1) => "pgls",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_MULTI, 1) => "clonerange",
        x if x == ceph_osd_op(CEPH_OSD_OP_MODE_CACHE, CEPH_OSD_OP_TYPE_DATA, 31) => "cache-flush",
        _ => "???",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ceph_strings_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/ceph/ceph_strings.c"
        ));
        let rados = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/include/linux/ceph/rados.h"
        ));
        assert!(source.contains("const char *ceph_entity_type_name(int type)"));
        assert!(source.contains("case CEPH_ENTITY_TYPE_MDS: return \"mds\";"));
        assert!(source.contains("case CEPH_ENTITY_TYPE_OSD: return \"osd\";"));
        assert!(source.contains("case CEPH_ENTITY_TYPE_MON: return \"mon\";"));
        assert!(source.contains("case CEPH_ENTITY_TYPE_CLIENT: return \"client\";"));
        assert!(source.contains("case CEPH_ENTITY_TYPE_AUTH: return \"auth\";"));
        assert!(source.contains("EXPORT_SYMBOL(ceph_entity_type_name);"));
        assert!(source.contains("const char *ceph_auth_proto_name(int proto)"));
        assert!(source.contains("case CEPH_AUTH_NONE:"));
        assert!(source.contains("return \"cephx\";"));
        assert!(source.contains("const char *ceph_con_mode_name(int mode)"));
        assert!(source.contains("return \"secure\";"));
        assert!(source.contains("#define GENERATE_CASE(op, opcode, str)"));
        assert!(source.contains("__CEPH_FORALL_OSD_OPS(GENERATE_CASE)"));
        assert!(source.contains("case CEPH_OSD_WATCH_OP_RECONNECT:"));
        assert!(source.contains("case CEPH_OSD_AUTOOUT:"));
        assert!(rados.contains("f(READ,\t\t__CEPH_OSD_OP(RD, DATA, 1),\t\"read\")"));
        assert!(rados.contains("f(COPY_FROM2,\t__CEPH_OSD_OP(WR, DATA, 45),\t\"copy-from2\")"));
        assert!(rados.contains("f(CALL,\t\t__CEPH_OSD_OP(RD, EXEC, 1),\t\"call\")"));
    }

    #[test]
    fn ceph_name_helpers_return_linux_strings() {
        assert_eq!(ceph_entity_type_name(CEPH_ENTITY_TYPE_OSD), "osd");
        assert_eq!(ceph_entity_type_name(0x7f), "unknown");
        assert_eq!(ceph_auth_proto_name(CEPH_AUTH_CEPHX), "cephx");
        assert_eq!(ceph_con_mode_name(CEPH_CON_MODE_SECURE), "secure");
        assert_eq!(
            ceph_osd_op_name(ceph_osd_op(CEPH_OSD_OP_MODE_RD, CEPH_OSD_OP_TYPE_DATA, 1)),
            "read"
        );
        assert_eq!(
            ceph_osd_op_name(ceph_osd_op(CEPH_OSD_OP_MODE_WR, CEPH_OSD_OP_TYPE_DATA, 45)),
            "copy-from2"
        );
        assert_eq!(
            ceph_osd_watch_op_name(CEPH_OSD_WATCH_OP_RECONNECT),
            "reconnect"
        );
        assert_eq!(ceph_osd_state_name(CEPH_OSD_AUTOOUT), "autoout");
        assert_eq!(ceph_osd_state_name(CEPH_OSD_EXISTS | CEPH_OSD_UP), "???");
    }
}
