//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/dns_resolve.c
//! test-origin: linux:vendor/linux/fs/smb/client/dns_resolve.c
//! CIFS DNS upcall resolution flow.

extern crate alloc;

use alloc::string::String;

use crate::include::uapi::errno::{EHOSTUNREACH, EINVAL, ENOMEM};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CifsDnsResolvePlan {
    AddressLiteral,
    ResolveFqdnFirst(String),
    ResolveNameOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CifsResolveNameReport {
    pub dns_query_called: bool,
    pub query_len: usize,
    pub unable_to_resolve_logged: bool,
    pub resolved_logged: bool,
    pub convert_address_called: bool,
    pub ip_freed: bool,
    pub unable_to_determine_ip_logged: bool,
    pub returned: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CifsDnsResolveReport {
    pub initial_debug_logged: bool,
    pub hostname_debug_logged: bool,
    pub address_literal: bool,
    pub fqdn_alloc_len: Option<usize>,
    pub fqdn_name: Option<String>,
    pub fqdn_freed: bool,
    pub fallback_to_name: bool,
    pub returned: i32,
}

pub fn dns_resolve_name_plan(
    dom: Option<&str>,
    name: &str,
    cifs_convert_address_positive: bool,
    cifs_netbios_name: bool,
) -> Result<CifsDnsResolvePlan, i32> {
    if name.is_empty() {
        return Err(-EINVAL);
    }
    if cifs_convert_address_positive {
        return Ok(CifsDnsResolvePlan::AddressLiteral);
    }
    if let Some(domain) = dom {
        if !domain.is_empty() && cifs_netbios_name {
            let mut fqdn = String::from(name);
            fqdn.push('.');
            fqdn.push_str(domain);
            return Ok(CifsDnsResolvePlan::ResolveFqdnFirst(fqdn));
        }
    }
    Ok(CifsDnsResolvePlan::ResolveNameOnly)
}

pub const fn resolve_name_report(
    name_len: usize,
    dns_query_ret: i32,
    cifs_convert_address_ret: i32,
) -> CifsResolveNameReport {
    if dns_query_ret < 0 {
        return CifsResolveNameReport {
            dns_query_called: true,
            query_len: name_len,
            unable_to_resolve_logged: true,
            resolved_logged: false,
            convert_address_called: false,
            ip_freed: false,
            unable_to_determine_ip_logged: false,
            returned: dns_query_ret,
        };
    }

    CifsResolveNameReport {
        dns_query_called: true,
        query_len: name_len,
        unable_to_resolve_logged: false,
        resolved_logged: true,
        convert_address_called: true,
        ip_freed: true,
        unable_to_determine_ip_logged: cifs_convert_address_ret == 0,
        returned: if cifs_convert_address_ret == 0 {
            -EHOSTUNREACH
        } else {
            0
        },
    }
}

pub const fn resolve_name_result(dns_query_ret: i32, cifs_convert_address_ret: i32) -> i32 {
    if dns_query_ret < 0 {
        return dns_query_ret;
    }
    if cifs_convert_address_ret == 0 {
        return -EHOSTUNREACH;
    }
    0
}

pub fn dns_resolve_name_report(
    dom: Option<&str>,
    name: Option<&str>,
    ip_addr_present: bool,
    cifs_convert_address_ret: i32,
    cifs_netbios_name: bool,
    fqdn_alloc_failed: bool,
    fqdn_resolve_ret: i32,
    name_resolve_ret: i32,
) -> CifsDnsResolveReport {
    let Some(name) = name else {
        return CifsDnsResolveReport {
            initial_debug_logged: true,
            hostname_debug_logged: false,
            address_literal: false,
            fqdn_alloc_len: None,
            fqdn_name: None,
            fqdn_freed: false,
            fallback_to_name: false,
            returned: -EINVAL,
        };
    };

    if !ip_addr_present || name.is_empty() {
        return CifsDnsResolveReport {
            initial_debug_logged: true,
            hostname_debug_logged: false,
            address_literal: false,
            fqdn_alloc_len: None,
            fqdn_name: None,
            fqdn_freed: false,
            fallback_to_name: false,
            returned: -EINVAL,
        };
    }

    if cifs_convert_address_ret > 0 {
        return CifsDnsResolveReport {
            initial_debug_logged: true,
            hostname_debug_logged: true,
            address_literal: true,
            fqdn_alloc_len: None,
            fqdn_name: None,
            fqdn_freed: false,
            fallback_to_name: false,
            returned: 0,
        };
    }

    if let Some(domain) = dom {
        if !domain.is_empty() && cifs_netbios_name {
            let alloc_len = domain.len() + name.len() + 2;
            if fqdn_alloc_failed {
                return CifsDnsResolveReport {
                    initial_debug_logged: true,
                    hostname_debug_logged: true,
                    address_literal: false,
                    fqdn_alloc_len: Some(alloc_len),
                    fqdn_name: None,
                    fqdn_freed: false,
                    fallback_to_name: false,
                    returned: -ENOMEM,
                };
            }

            let mut fqdn = String::from(name);
            fqdn.push('.');
            fqdn.push_str(domain);
            if fqdn_resolve_ret == 0 {
                return CifsDnsResolveReport {
                    initial_debug_logged: true,
                    hostname_debug_logged: true,
                    address_literal: false,
                    fqdn_alloc_len: Some(alloc_len),
                    fqdn_name: Some(fqdn),
                    fqdn_freed: true,
                    fallback_to_name: false,
                    returned: 0,
                };
            }

            return CifsDnsResolveReport {
                initial_debug_logged: true,
                hostname_debug_logged: true,
                address_literal: false,
                fqdn_alloc_len: Some(alloc_len),
                fqdn_name: Some(fqdn),
                fqdn_freed: true,
                fallback_to_name: true,
                returned: name_resolve_ret,
            };
        }
    }

    CifsDnsResolveReport {
        initial_debug_logged: true,
        hostname_debug_logged: true,
        address_literal: false,
        fqdn_alloc_len: None,
        fqdn_name: None,
        fqdn_freed: false,
        fallback_to_name: true,
        returned: name_resolve_ret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifs_dns_resolve_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/dns_resolve.c"
        ));
        assert!(source.contains("#include <linux/dns_resolver.h>"));
        assert!(source.contains("#include \"dns_resolve.h\""));
        assert!(source.contains("static int resolve_name"));
        assert!(source.contains("dns_query(current->nsproxy->net_ns, NULL, name,"));
        assert!(source.contains("cifs_dbg(FYI, \"%s: unable to resolve: %*.*s\\n\""));
        assert!(source.contains("cifs_dbg(FYI, \"%s: resolved: %*.*s to %s\\n\""));
        assert!(source.contains("rc = cifs_convert_address(addr, ip, strlen(ip));"));
        assert!(source.contains("kfree(ip);"));
        assert!(source.contains("if (!rc)"));
        assert!(source.contains("cifs_dbg(FYI, \"%s: unable to determine ip address\\n\""));
        assert!(source.contains("rc = -EHOSTUNREACH;"));
        assert!(source.contains("int dns_resolve_name"));
        assert!(source.contains("cifs_dbg(FYI, \"%s: dom=%s name=%.*s\\n\""));
        assert!(source.contains("if (!ip_addr || !name || !*name || !namelen)"));
        assert!(source.contains("return -EINVAL;"));
        assert!(source.contains("cifs_dbg(FYI, \"%s: hostname=%.*s\\n\""));
        assert!(source.contains("rc = cifs_convert_address(ip_addr, name, namelen);"));
        assert!(source.contains("if (rc > 0)"));
        assert!(source.contains("cifs_dbg(FYI, \"%s: unc is IP, skipping dns upcall: %*.*s\\n\""));
        assert!(source.contains("return 0;"));
        assert!(source.contains("if (dom && *dom && cifs_netbios_name(name, namelen))"));
        assert!(source.contains("strnlen(dom, CIFS_MAX_DOMAINNAME_LEN) + namelen + 2;"));
        assert!(source.contains("s = kmalloc(len, GFP_KERNEL);"));
        assert!(source.contains("if (!s)"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("scnprintf(s, len, \"%.*s.%s\""));
        assert!(source.contains("rc = resolve_name(s, len - 1, ip_addr);"));
        assert!(source.contains("kfree(s);"));
        assert!(source.contains("if (!rc)"));
        assert!(source.contains("return resolve_name(name, namelen, ip_addr);"));

        assert_eq!(dns_resolve_name_plan(None, "", false, false), Err(-EINVAL));
        assert_eq!(
            dns_resolve_name_plan(None, "192.0.2.1", true, false),
            Ok(CifsDnsResolvePlan::AddressLiteral)
        );
        assert_eq!(
            dns_resolve_name_plan(Some("example.com"), "SERVER", false, true),
            Ok(CifsDnsResolvePlan::ResolveFqdnFirst(String::from(
                "SERVER.example.com"
            )))
        );
        assert_eq!(
            dns_resolve_name_plan(Some("example.com"), "server.example.com", false, false),
            Ok(CifsDnsResolvePlan::ResolveNameOnly)
        );
        assert_eq!(resolve_name_result(-2, 0), -2);
        assert_eq!(resolve_name_result(4, 0), -113);
        assert_eq!(resolve_name_result(4, 1), 0);
    }

    #[test]
    fn resolve_name_report_matches_dns_query_and_conversion_paths() {
        assert_eq!(
            resolve_name_report(6, -2, 0),
            CifsResolveNameReport {
                dns_query_called: true,
                query_len: 6,
                unable_to_resolve_logged: true,
                resolved_logged: false,
                convert_address_called: false,
                ip_freed: false,
                unable_to_determine_ip_logged: false,
                returned: -2,
            }
        );
        assert_eq!(
            resolve_name_report(6, 4, 0),
            CifsResolveNameReport {
                dns_query_called: true,
                query_len: 6,
                unable_to_resolve_logged: false,
                resolved_logged: true,
                convert_address_called: true,
                ip_freed: true,
                unable_to_determine_ip_logged: true,
                returned: -EHOSTUNREACH,
            }
        );
        assert_eq!(resolve_name_report(6, 4, 1).returned, 0);
    }

    #[test]
    fn dns_report_matches_invalid_and_literal_paths() {
        assert_eq!(
            dns_resolve_name_report(None, None, true, 0, false, false, 0, 0).returned,
            -EINVAL
        );
        assert_eq!(
            dns_resolve_name_report(None, Some("host"), false, 0, false, false, 0, 0).returned,
            -EINVAL
        );
        assert_eq!(
            dns_resolve_name_report(None, Some("192.0.2.1"), true, 1, false, false, 0, -2),
            CifsDnsResolveReport {
                initial_debug_logged: true,
                hostname_debug_logged: true,
                address_literal: true,
                fqdn_alloc_len: None,
                fqdn_name: None,
                fqdn_freed: false,
                fallback_to_name: false,
                returned: 0,
            }
        );
    }

    #[test]
    fn dns_report_matches_fqdn_allocation_success_and_fallback() {
        assert_eq!(
            dns_resolve_name_report(
                Some("example.com"),
                Some("SERVER"),
                true,
                0,
                true,
                true,
                0,
                -2
            )
            .returned,
            -ENOMEM
        );

        assert_eq!(
            dns_resolve_name_report(
                Some("example.com"),
                Some("SERVER"),
                true,
                0,
                true,
                false,
                0,
                -2
            ),
            CifsDnsResolveReport {
                initial_debug_logged: true,
                hostname_debug_logged: true,
                address_literal: false,
                fqdn_alloc_len: Some(19),
                fqdn_name: Some(String::from("SERVER.example.com")),
                fqdn_freed: true,
                fallback_to_name: false,
                returned: 0,
            }
        );

        assert_eq!(
            dns_resolve_name_report(
                Some("example.com"),
                Some("SERVER"),
                true,
                0,
                true,
                false,
                -2,
                -5
            ),
            CifsDnsResolveReport {
                initial_debug_logged: true,
                hostname_debug_logged: true,
                address_literal: false,
                fqdn_alloc_len: Some(19),
                fqdn_name: Some(String::from("SERVER.example.com")),
                fqdn_freed: true,
                fallback_to_name: true,
                returned: -5,
            }
        );
    }
}
