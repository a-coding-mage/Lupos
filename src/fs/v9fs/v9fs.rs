//! linux-parity: partial
//! linux-source: vendor/linux/fs/9p/v9fs.c
//! test-origin: linux:vendor/linux/fs/9p/v9fs.c
//! 9P mount option parsing, session flag normalization, and show-options logic.

extern crate alloc;

use alloc::{format, string::String, vec::Vec};

use crate::include::uapi::errno::EINVAL;

use super::types::*;
use super::vfs_inode::INVALID_UID;

pub const DEFAULT_MSIZE: u32 = (128 * 1024) + P9_IOHDRSZ;
pub const P9_LOCK_TIMEOUT_HZ: u64 = 30;
pub const P9_FD_PORT: u32 = 564;
pub const P9_RDMA_PORT: u32 = 5640;
pub const P9_RDMA_SQ_DEPTH: u32 = 32;
pub const P9_RDMA_RQ_DEPTH: u32 = 32;
pub const P9_RDMA_TIMEOUT: u32 = 30_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtoVersion {
    Legacy,
    P9P2000U,
    P9P2000L,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionOptions {
    pub debug: u32,
    pub dfltuid: u32,
    pub dfltgid: u32,
    pub afid: u32,
    pub uname: String,
    pub aname: String,
    pub nodev: bool,
    pub flags: u32,
    pub cache: u32,
    pub cachetag: Option<String>,
    pub uid: u32,
    pub session_lock_timeout: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientOptions {
    pub proto_version: ProtoVersion,
    pub msize: u32,
    pub trans: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V9fsContext {
    pub session: SessionOptions,
    pub client: ClientOptions,
    pub fd_port: u32,
    pub rdma_port: u32,
    pub rdma_sq_depth: u32,
    pub rdma_rq_depth: u32,
    pub rdma_timeout: u32,
    pub fd_privport: bool,
    pub rdma_privport: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedSession {
    pub flags: u32,
    pub uid: u32,
    pub maxdata: u32,
    pub acl_enabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum V9fsParam<'a> {
    Debug(u32),
    DfltUid(u32),
    DfltGid(u32),
    Afid(u32),
    Uname(&'a str),
    Aname(&'a str),
    Nodevmap,
    Noxattr,
    Directio,
    IgnoreQv,
    Cache(&'a str),
    CacheTag(&'a str),
    Access(&'a str),
    PosixAcl,
    LockTimeout(u32),
    Msize(u32),
    Trans(&'a str),
    Legacy,
    Version(ProtoVersion),
    Port(u32),
    PrivPort,
    Unknown,
}

impl Default for V9fsContext {
    fn default() -> Self {
        Self {
            session: SessionOptions {
                debug: 0,
                dfltuid: u32::MAX - 1,
                dfltgid: u32::MAX - 1,
                afid: !0,
                uname: String::from(V9FS_DEFUSER),
                aname: String::from(V9FS_DEFANAME),
                nodev: false,
                flags: 0,
                cache: CACHE_NONE,
                cachetag: None,
                uid: INVALID_UID,
                session_lock_timeout: P9_LOCK_TIMEOUT_HZ,
            },
            client: ClientOptions {
                proto_version: ProtoVersion::P9P2000L,
                msize: DEFAULT_MSIZE,
                trans: None,
            },
            fd_port: P9_FD_PORT,
            rdma_port: P9_RDMA_PORT,
            rdma_sq_depth: P9_RDMA_SQ_DEPTH,
            rdma_rq_depth: P9_RDMA_RQ_DEPTH,
            rdma_timeout: P9_RDMA_TIMEOUT,
            fd_privport: false,
            rdma_privport: false,
        }
    }
}

pub fn get_cache_mode(s: &str) -> Result<u32, i32> {
    match s {
        "loose" => Ok(CACHE_SC_LOOSE),
        "fscache" => Ok(CACHE_SC_FSCACHE),
        "mmap" => Ok(CACHE_SC_MMAP),
        "readahead" => Ok(CACHE_SC_READAHEAD),
        "none" => Ok(CACHE_SC_NONE),
        _ => parse_u32_auto(s).ok_or(-EINVAL),
    }
}

fn parse_u32_auto(s: &str) -> Option<u32> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else if s.len() > 1 && s.starts_with('0') {
        u32::from_str_radix(&s[1..], 8).ok()
    } else {
        s.parse().ok()
    }
}

pub fn parse_param(
    ctx: &mut V9fsContext,
    param: V9fsParam<'_>,
    posix_acl_enabled: bool,
) -> Result<(), i32> {
    match param {
        V9fsParam::Unknown => return Ok(()),
        V9fsParam::Debug(debug) => ctx.session.debug = debug,
        V9fsParam::DfltUid(uid) => ctx.session.dfltuid = uid,
        V9fsParam::DfltGid(gid) => ctx.session.dfltgid = gid,
        V9fsParam::Afid(afid) => ctx.session.afid = afid,
        V9fsParam::Uname(uname) => ctx.session.uname = String::from(uname),
        V9fsParam::Aname(aname) => ctx.session.aname = String::from(aname),
        V9fsParam::Nodevmap => ctx.session.nodev = true,
        V9fsParam::Noxattr => ctx.session.flags |= V9FS_NO_XATTR,
        V9fsParam::Directio => ctx.session.flags |= V9FS_DIRECT_IO,
        V9fsParam::IgnoreQv => ctx.session.flags |= V9FS_IGNORE_QV,
        V9fsParam::Cache(cache) => ctx.session.cache = get_cache_mode(cache)?,
        V9fsParam::CacheTag(tag) => ctx.session.cachetag = Some(String::from(tag)),
        V9fsParam::Access(access) => parse_access(&mut ctx.session, access)?,
        V9fsParam::PosixAcl => {
            if posix_acl_enabled {
                ctx.session.flags |= V9FS_POSIX_ACL;
            }
        }
        V9fsParam::LockTimeout(timeout) => {
            if timeout < 1 {
                return Err(-EINVAL);
            }
            ctx.session.session_lock_timeout = timeout as u64;
        }
        V9fsParam::Msize(msize) => {
            if !(4096..=i32::MAX as u32).contains(&msize) {
                return Err(-EINVAL);
            }
            ctx.client.msize = msize;
        }
        V9fsParam::Trans(trans) => ctx.client.trans = Some(String::from(trans)),
        V9fsParam::Legacy => ctx.client.proto_version = ProtoVersion::Legacy,
        V9fsParam::Version(version) => ctx.client.proto_version = version,
        V9fsParam::Port(port) => {
            ctx.fd_port = port;
            ctx.rdma_port = port;
        }
        V9fsParam::PrivPort => {
            ctx.fd_privport = true;
            ctx.rdma_privport = true;
        }
    }
    Ok(())
}

fn parse_access(session: &mut SessionOptions, s: &str) -> Result<(), i32> {
    session.flags &= !V9FS_ACCESS_MASK;
    match s {
        "user" => session.flags |= V9FS_ACCESS_USER,
        "any" => session.flags |= V9FS_ACCESS_ANY,
        "client" => session.flags |= V9FS_ACCESS_CLIENT,
        _ => {
            let uid = s.parse().map_err(|_| -EINVAL)?;
            session.flags |= V9FS_ACCESS_SINGLE;
            session.uid = uid;
        }
    }
    Ok(())
}

pub fn normalize_session(ctx: &V9fsContext) -> NormalizedSession {
    let mut flags = V9FS_ACCESS_USER;
    match ctx.client.proto_version {
        ProtoVersion::P9P2000L => {
            flags = V9FS_ACCESS_CLIENT | V9FS_PROTO_2000L;
        }
        ProtoVersion::P9P2000U => {
            flags |= V9FS_PROTO_2000U;
        }
        ProtoVersion::Legacy => {}
    }

    if ctx.session.flags & V9FS_ACCESS_MASK != 0 {
        flags &= !V9FS_ACCESS_MASK;
    }
    flags |= ctx.session.flags;
    let mut uid = ctx.session.uid;

    if flags & V9FS_PROTO_2000L == 0 && (flags & V9FS_ACCESS_MASK) == V9FS_ACCESS_CLIENT {
        flags &= !V9FS_ACCESS_MASK;
        flags |= V9FS_ACCESS_USER;
    }
    if flags & (V9FS_PROTO_2000U | V9FS_PROTO_2000L) == 0
        && (flags & V9FS_ACCESS_MASK) == V9FS_ACCESS_USER
    {
        flags &= !V9FS_ACCESS_MASK;
        flags |= V9FS_ACCESS_ANY;
        uid = INVALID_UID;
    }
    if flags & V9FS_PROTO_2000L == 0 || (flags & V9FS_ACCESS_MASK) != V9FS_ACCESS_CLIENT {
        flags &= !V9FS_ACL_MASK;
    }
    NormalizedSession {
        flags,
        uid,
        maxdata: ctx.client.msize.saturating_sub(P9_IOHDRSZ),
        acl_enabled: flags & V9FS_POSIX_ACL != 0,
    }
}

pub fn show_options(session: &SessionOptions, normalized_flags: u32) -> Vec<String> {
    let mut out = Vec::new();
    if session.debug != 0 {
        out.push(format!("debug={:#x}", session.debug));
    }
    if session.afid != !0 {
        out.push(format!("afid={}", session.afid));
    }
    if session.uname != V9FS_DEFUSER {
        out.push(format!("uname={}", session.uname));
    }
    if session.aname != V9FS_DEFANAME {
        out.push(format!("aname={}", session.aname));
    }
    if session.nodev {
        out.push(String::from("nodevmap"));
    }
    if session.cache != 0 {
        out.push(format!("cache={:#x}", session.cache));
    }
    if let Some(cachetag) = &session.cachetag {
        if session.cache & CACHE_FSCACHE != 0 {
            out.push(format!("cachetag={cachetag}"));
        }
    }
    match normalized_flags & V9FS_ACCESS_MASK {
        V9FS_ACCESS_USER => out.push(String::from("access=user")),
        V9FS_ACCESS_ANY => out.push(String::from("access=any")),
        V9FS_ACCESS_CLIENT => out.push(String::from("access=client")),
        V9FS_ACCESS_SINGLE => out.push(format!("access={}", session.uid)),
        _ => {}
    }
    if normalized_flags & V9FS_IGNORE_QV != 0 {
        out.push(String::from("ignoreqv"));
    }
    if normalized_flags & V9FS_DIRECT_IO != 0 {
        out.push(String::from("directio"));
    }
    if normalized_flags & V9FS_POSIX_ACL != 0 {
        out.push(String::from("posixacl"));
    }
    if normalized_flags & V9FS_NO_XATTR != 0 {
        out.push(String::from("noxattr"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v9fs_options_match_linux_source_and_statmount_selftest() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/9p/v9fs.c"
        ));
        let statmount = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/tools/testing/selftests/filesystems/statmount/statmount_test.c"
        ));
        assert!(source.contains("static const struct constant_table p9_versions[]"));
        assert!(source.contains("{ \"9p2000.L\",\tp9_proto_2000L }"));
        assert!(source.contains("fsparam_string\t(\"cache\",\tOpt_cache)"));
        assert!(source.contains("static int get_cache_mode(char *s)"));
        assert!(source.contains("!strcmp(s, \"fscache\")"));
        assert!(source.contains("int v9fs_show_options(struct seq_file *m, struct dentry *root)"));
        assert!(source.contains("seq_puts(m, \",access=client\")"));
        assert!(
            source.contains(
                "int v9fs_parse_param(struct fs_context *fc, struct fs_parameter *param)"
            )
        );
        assert!(source.contains("traditionally 9p has ignored unknown mount options"));
        assert!(source.contains("session_opts->flags &= ~V9FS_ACCESS_MASK;"));
        assert!(source.contains("locktimeout must be a greater than zero integer"));
        assert!(source.contains("clnt->proto_version = result.uint_32;"));
        assert!(source.contains("v9ses->flags = V9FS_ACCESS_USER;"));
        assert!(source.contains("v9ses->flags = V9FS_ACCESS_CLIENT;"));
        assert!(source.contains("Fall back to ACCESS_USER"));
        assert!(source.contains("fall back to V9FS_ACCESS_ANY"));
        assert!(statmount.contains("\"9p\""));

        assert_eq!(get_cache_mode("loose"), Ok(CACHE_SC_LOOSE));
        assert_eq!(get_cache_mode("0x5"), Ok(5));
        assert_eq!(get_cache_mode("bad"), Err(-EINVAL));

        let mut ctx = V9fsContext::default();
        parse_param(&mut ctx, V9fsParam::Access("user"), true).unwrap();
        parse_param(&mut ctx, V9fsParam::PosixAcl, true).unwrap();
        let normalized = normalize_session(&ctx);
        assert_eq!(
            normalized.flags & (V9FS_ACCESS_MASK | V9FS_PROTO_2000L),
            V9FS_ACCESS_USER | V9FS_PROTO_2000L
        );
        assert!(!normalized.acl_enabled);

        parse_param(&mut ctx, V9fsParam::Access("client"), true).unwrap();
        let normalized = normalize_session(&ctx);
        assert_eq!(normalized.flags & V9FS_ACCESS_MASK, V9FS_ACCESS_CLIENT);
        assert!(normalized.acl_enabled);

        parse_param(&mut ctx, V9fsParam::Cache("fscache"), true).unwrap();
        parse_param(&mut ctx, V9fsParam::CacheTag("tag"), true).unwrap();
        let opts = show_options(&ctx.session, normalized.flags | V9FS_NO_XATTR);
        assert!(opts.iter().any(|opt| opt == "access=client"));
        assert!(opts.iter().any(|opt| opt == "cache=0x8f"));
        assert!(opts.iter().any(|opt| opt == "cachetag=tag"));
        assert!(opts.iter().any(|opt| opt == "noxattr"));

        assert_eq!(
            parse_param(&mut ctx, V9fsParam::LockTimeout(0), true),
            Err(-EINVAL)
        );
        assert_eq!(
            parse_param(&mut ctx, V9fsParam::Msize(1024), true),
            Err(-EINVAL)
        );
        assert_eq!(parse_param(&mut ctx, V9fsParam::Unknown, true), Ok(()));
    }
}
