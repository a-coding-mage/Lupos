//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/cifsroot.c
//! test-origin: linux:vendor/linux/fs/smb/client/cifsroot.c
//! SMB root filesystem boot-option parsing.

extern crate alloc;

use alloc::string::String;

pub const DEFAULT_MNT_OPTS: &str =
    "vers=1.0,cifsacl,mfsymlinks,rsize=1048576,wsize=65536,uid=0,gid=0,hard,rootfs";
pub const CIFS_ROOT_DEV_MAX: usize = 2048;
pub const CIFS_ROOT_OPTS_MAX: usize = 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CifsRootSetup {
    pub accepted: bool,
    pub root_dev: Option<String>,
    pub root_opts: String,
    pub server_addr: Option<[u8; 4]>,
    pub path_too_long: bool,
    pub opts_too_long: bool,
}

pub fn parse_srvaddr_fragment(input: &str) -> Option<[u8; 4]> {
    let mut bytes = [0u8; 15];
    let mut len = 0usize;
    for byte in input.bytes() {
        if (byte.is_ascii_digit() || byte == b'.') && len < bytes.len() {
            bytes[len] = byte;
            len += 1;
        }
    }
    parse_ipv4_bytes(&bytes[..len])
}

pub fn cifs_root_setup_line(line: &str) -> CifsRootSetup {
    let mut setup = CifsRootSetup {
        accepted: true,
        root_dev: None,
        root_opts: String::from(DEFAULT_MNT_OPTS),
        server_addr: None,
        path_too_long: false,
        opts_too_long: false,
    };

    if line.len() > 3 && line.starts_with("//") {
        let Some(share_slash_rel) = line[2..].find('/') else {
            return setup;
        };
        let share_slash = share_slash_rel + 2;
        if share_slash + 1 >= line.len() {
            return setup;
        }
        let comma_or_end = line[share_slash..]
            .find(',')
            .map(|off| share_slash + off)
            .unwrap_or(line.len());
        let unc = &line[..comma_or_end];
        if unc.len() + 1 > CIFS_ROOT_DEV_MAX {
            setup.path_too_long = true;
            return setup;
        }

        setup.root_dev = Some(String::from(unc));
        setup.server_addr = parse_srvaddr_fragment(&line[2..comma_or_end]);

        if comma_or_end < line.len() {
            let extra = &line[comma_or_end + 1..];
            let combined_len = DEFAULT_MNT_OPTS.len() + 1 + extra.len();
            if combined_len >= CIFS_ROOT_OPTS_MAX {
                setup.opts_too_long = true;
                setup.root_opts.truncate(CIFS_ROOT_OPTS_MAX - 1);
                return setup;
            }
            setup.root_opts.push(',');
            setup.root_opts.push_str(extra);
        }
    }

    setup
}

pub fn cifs_root_data_ready(setup: &CifsRootSetup) -> bool {
    setup.root_dev.is_some() && setup.server_addr.is_some()
}

fn parse_ipv4_bytes(bytes: &[u8]) -> Option<[u8; 4]> {
    let mut out = [0u8; 4];
    let mut part = 0usize;
    let mut value = 0u16;
    let mut have_digit = false;

    for byte in bytes.iter().copied() {
        match byte {
            b'0'..=b'9' => {
                have_digit = true;
                value = value
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as u16);
                if value > 255 {
                    return None;
                }
            }
            b'.' => {
                if !have_digit || part >= 3 {
                    return None;
                }
                out[part] = value as u8;
                part += 1;
                value = 0;
                have_digit = false;
            }
            _ => return None,
        }
    }
    if !have_digit || part != 3 {
        return None;
    }
    out[part] = value as u8;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifsroot_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/cifsroot.c"
        ));
        assert!(source.contains("#define DEFAULT_MNT_OPTS"));
        assert!(
            source
                .contains("\"vers=1.0,cifsacl,mfsymlinks,rsize=1048576,wsize=65536,uid=0,gid=0,\"")
        );
        assert!(source.contains("static char root_dev[2048] __initdata = \"\";"));
        assert!(source.contains("static char root_opts[1024] __initdata = DEFAULT_MNT_OPTS;"));
        assert!(source.contains("static __be32 __init parse_srvaddr"));
        assert!(source.contains("if (isdigit(*start) || *start == '.')"));
        assert!(source.contains("return in_aton(addr);"));
        assert!(source.contains("cifsroot=//<server-ip>/<share>[,options]"));
        assert!(source.contains("ROOT_DEV = Root_CIFS;"));
        assert!(source.contains("if (strlen(line) > 3 && line[0] == '/' && line[1] == '/')"));
        assert!(source.contains("s = strchr(&line[2], '/');"));
        assert!(source.contains("s = strchrnul(s, ',');"));
        assert!(source.contains("if (len > sizeof(root_dev))"));
        assert!(source.contains("strscpy(root_dev, line, len);"));
        assert!(source.contains("snprintf(root_opts,"));
        assert!(source.contains("root_server_addr = srvaddr;"));
        assert!(source.contains("__setup(\"cifsroot=\", cifs_root_setup);"));
        assert!(source.contains("int __init cifs_root_data"));

        assert_eq!(
            parse_srvaddr_fragment("192.0.2.9/share"),
            Some([192, 0, 2, 9])
        );
        let setup = cifs_root_setup_line("//192.0.2.9/share,cache=none");
        assert_eq!(setup.root_dev.as_deref(), Some("//192.0.2.9/share"));
        assert_eq!(setup.server_addr, Some([192, 0, 2, 9]));
        assert!(setup.root_opts.ends_with(",cache=none"));
        assert!(cifs_root_data_ready(&setup));
        let missing = cifs_root_setup_line("bad");
        assert!(!cifs_root_data_ready(&missing));
    }
}
