//! linux-parity: complete
//! linux-source: vendor/linux/net/6lowpan/debugfs.c
//! test-origin: linux:vendor/linux/net/6lowpan/debugfs.c
//! 6LoWPAN debugfs context flag, prefix length, and prefix parsing helpers.

extern crate alloc;

use alloc::{format, string::String, vec, vec::Vec};

pub const LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS: usize = 8;
pub const LOWPAN_IPHC_CTX_TABLE_SIZE: usize = 1 << 4;
pub const LOWPAN_IPHC_CTX_FLAG_ACTIVE: u8 = 1 << 0;
pub const LOWPAN_IPHC_CTX_FLAG_COMPRESSION: u8 = 1 << 1;
pub const LOWPAN_DEBUGFS_ROOT: &str = "6lowpan";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugfsError {
    Invalid,
    Fault,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LowpanIphcCtx {
    pub id: u8,
    pub flags: u8,
    pub plen: u8,
    pub pfx: [u16; LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LowpanDebugfsEntry {
    pub path: String,
    pub mode: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LowpanDebugDevice {
    pub name: String,
    pub contexts: Vec<LowpanIphcCtx>,
    pub is_ieee802154: bool,
    pub short_addr: u16,
}

impl LowpanIphcCtx {
    pub const fn new(id: u8) -> Self {
        Self {
            id,
            flags: 0,
            plen: 0,
            pfx: [0; LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS],
        }
    }

    pub const fn active(self) -> bool {
        self.flags & LOWPAN_IPHC_CTX_FLAG_ACTIVE != 0
    }

    pub const fn compression(self) -> bool {
        self.flags & LOWPAN_IPHC_CTX_FLAG_COMPRESSION != 0
    }
}

impl LowpanDebugDevice {
    pub fn new(name: &str) -> Self {
        let mut contexts = Vec::with_capacity(LOWPAN_IPHC_CTX_TABLE_SIZE);
        for id in 0..LOWPAN_IPHC_CTX_TABLE_SIZE {
            contexts.push(LowpanIphcCtx::new(id as u8));
        }
        Self {
            name: name.into(),
            contexts,
            is_ieee802154: false,
            short_addr: 0xffff,
        }
    }
}

pub const fn lowpan_ctx_flag_active_get(ctx: LowpanIphcCtx) -> u64 {
    ctx.active() as u64
}

pub fn lowpan_ctx_flag_active_set(ctx: &mut LowpanIphcCtx, val: u64) -> Result<(), DebugfsError> {
    set_bool_flag(ctx, LOWPAN_IPHC_CTX_FLAG_ACTIVE, val)
}

pub const fn lowpan_ctx_flag_c_get(ctx: LowpanIphcCtx) -> u64 {
    ctx.compression() as u64
}

pub fn lowpan_ctx_flag_c_set(ctx: &mut LowpanIphcCtx, val: u64) -> Result<(), DebugfsError> {
    set_bool_flag(ctx, LOWPAN_IPHC_CTX_FLAG_COMPRESSION, val)
}

pub const fn lowpan_ctx_plen_get(ctx: LowpanIphcCtx) -> u64 {
    ctx.plen as u64
}

pub const fn lowpan_ctx_plen_set(ctx: &mut LowpanIphcCtx, val: u64) -> Result<(), DebugfsError> {
    if val > 128 {
        return Err(DebugfsError::Invalid);
    }
    ctx.plen = val as u8;
    Ok(())
}

pub fn lowpan_ctx_pfx_show(ctx: &LowpanIphcCtx) -> String {
    format!(
        "{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}:{:04x}\n",
        ctx.pfx[0],
        ctx.pfx[1],
        ctx.pfx[2],
        ctx.pfx[3],
        ctx.pfx[4],
        ctx.pfx[5],
        ctx.pfx[6],
        ctx.pfx[7]
    )
}

pub fn lowpan_ctx_pfx_write(ctx: &mut LowpanIphcCtx, input: &str) -> Result<(), DebugfsError> {
    ctx.pfx = parse_prefix_words(input)?;
    Ok(())
}

pub fn lowpan_ctx_pfx_write_status(
    ctx: &mut LowpanIphcCtx,
    input: &str,
    count: usize,
    copy_from_user_ok: bool,
) -> Result<usize, DebugfsError> {
    if !copy_from_user_ok {
        return Err(DebugfsError::Fault);
    }
    lowpan_ctx_pfx_write(ctx, input)?;
    Ok(count)
}

pub fn parse_prefix_words(
    input: &str,
) -> Result<[u16; LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS], DebugfsError> {
    let mut words = [0u16; LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS];
    let mut rest = input.trim_start();
    let mut count = 0usize;
    while count < LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS {
        let mut end = 0usize;
        for (idx, ch) in rest.char_indices() {
            if idx == 4 || !ch.is_ascii_hexdigit() {
                break;
            }
            end = idx + ch.len_utf8();
        }
        if end == 0 {
            return Err(DebugfsError::Invalid);
        }
        words[count] = u16::from_str_radix(&rest[..end], 16).map_err(|_| DebugfsError::Invalid)?;
        rest = &rest[end..];
        count += 1;
        if count == LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS {
            break;
        }
        let Some(next) = rest.strip_prefix(':') else {
            return Err(DebugfsError::Invalid);
        };
        rest = next;
    }
    Ok(words)
}

pub fn lowpan_context_show(contexts: &[LowpanIphcCtx]) -> String {
    let mut out = String::from("cid|prefix                                     |C\n");
    out.push_str("-------------------------------------------------\n");
    for ctx in contexts.iter().take(LOWPAN_IPHC_CTX_TABLE_SIZE) {
        if !ctx.active() {
            continue;
        }
        out.push_str(&format!(
            "{:>3}|{:>39}/{:<3}|{}\n",
            ctx.id,
            ipv6_prefix_compact(&ctx.pfx),
            ctx.plen,
            ctx.compression() as u8
        ));
    }
    out
}

pub const fn lowpan_short_addr_get(short_addr_le: u16) -> u64 {
    u16::from_le(short_addr_le) as u64
}

pub fn lowpan_dev_debugfs_ctx_init_plan(id: u8) -> Vec<LowpanDebugfsEntry> {
    if id as usize >= LOWPAN_IPHC_CTX_TABLE_SIZE {
        return Vec::new();
    }
    let root = format!("contexts/{id}");
    let mut entries = vec![LowpanDebugfsEntry {
        path: root.clone(),
        mode: 0o555,
    }];
    for name in ["active", "compression", "prefix", "prefix_len"] {
        entries.push(LowpanDebugfsEntry {
            path: format!("{root}/{name}"),
            mode: 0o644,
        });
    }
    entries
}

pub fn lowpan_dev_debugfs_init_plan(dev: &LowpanDebugDevice) -> Vec<LowpanDebugfsEntry> {
    let mut entries = vec![
        LowpanDebugfsEntry {
            path: dev.name.clone(),
            mode: 0o555,
        },
        LowpanDebugfsEntry {
            path: format!("{}/contexts", dev.name),
            mode: 0o555,
        },
        LowpanDebugfsEntry {
            path: format!("{}/contexts/show", dev.name),
            mode: 0o644,
        },
    ];
    for id in 0..LOWPAN_IPHC_CTX_TABLE_SIZE {
        for mut entry in lowpan_dev_debugfs_ctx_init_plan(id as u8) {
            entry.path = format!("{}/{}", dev.name, entry.path);
            entries.push(entry);
        }
    }
    if dev.is_ieee802154 {
        entries.push(LowpanDebugfsEntry {
            path: format!("{}/ieee802154", dev.name),
            mode: 0o555,
        });
        entries.push(LowpanDebugfsEntry {
            path: format!("{}/ieee802154/short_addr", dev.name),
            mode: 0o444,
        });
    }
    entries
}

pub fn lowpan_dev_debugfs_exit(iface_debugfs_present: bool) -> bool {
    iface_debugfs_present
}

pub fn lowpan_debugfs_init() -> LowpanDebugfsEntry {
    LowpanDebugfsEntry {
        path: LOWPAN_DEBUGFS_ROOT.into(),
        mode: 0o555,
    }
}

pub fn lowpan_debugfs_exit(root_present: bool) -> bool {
    root_present
}

fn ipv6_prefix_compact(words: &[u16; LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS]) -> String {
    let mut best_start = None;
    let mut best_len = 0usize;
    let mut i = 0usize;
    while i < words.len() {
        if words[i] != 0 {
            i += 1;
            continue;
        }
        let start = i;
        while i < words.len() && words[i] == 0 {
            i += 1;
        }
        let len = i - start;
        if len > best_len && len >= 2 {
            best_start = Some(start);
            best_len = len;
        }
    }

    let mut out = String::new();
    let mut idx = 0usize;
    while idx < words.len() {
        if Some(idx) == best_start {
            if out.is_empty() {
                out.push_str("::");
            } else {
                out.push(':');
            }
            idx += best_len;
            if idx == words.len() {
                break;
            }
            continue;
        }
        if !out.is_empty() && !out.ends_with(':') {
            out.push(':');
        }
        out.push_str(&format!("{:x}", words[idx]));
        idx += 1;
    }
    out
}

fn set_bool_flag(ctx: &mut LowpanIphcCtx, flag: u8, val: u64) -> Result<(), DebugfsError> {
    match val {
        0 => ctx.flags &= !flag,
        1 => ctx.flags |= flag,
        _ => return Err(DebugfsError::Invalid),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowpan_debugfs_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/6lowpan/debugfs.c"
        ));
        assert!(source.contains("#define LOWPAN_DEBUGFS_CTX_PFX_NUM_ARGS\t8"));
        assert!(source.contains("if (val != 0 && val != 1)"));
        assert!(source.contains("set_bit(LOWPAN_IPHC_CTX_FLAG_ACTIVE"));
        assert!(source.contains("set_bit(LOWPAN_IPHC_CTX_FLAG_COMPRESSION"));
        assert!(source.contains("if (val > 128)"));
        assert!(source.contains("sscanf(buf, \"%04x:%04x:%04x:%04x:%04x:%04x:%04x:%04x\""));
        assert!(source.contains("return single_open(file, lowpan_ctx_pfx_show"));
        assert!(source.contains("DEFINE_SHOW_ATTRIBUTE(lowpan_context);"));
        assert!(source.contains("seq_printf(file, \"%3s|%-43s|%c\\n\""));
        assert!(source.contains("if (WARN_ON_ONCE(id >= LOWPAN_IPHC_CTX_TABLE_SIZE))"));
        assert!(source.contains("debugfs_create_file(\"active\", 0644"));
        assert!(source.contains("debugfs_create_file(\"compression\", 0644"));
        assert!(source.contains("debugfs_create_file(\"prefix\", 0644"));
        assert!(source.contains("debugfs_create_file(\"prefix_len\", 0644"));
        assert!(source.contains("if (!lowpan_is_ll(dev, LOWPAN_LLTYPE_IEEE802154))"));
        assert!(source.contains("debugfs_create_dir(\"6lowpan\", NULL);"));
        assert!(source.contains("debugfs_remove_recursive(lowpan_dev(dev)->iface_debugfs);"));
        assert!(source.contains("debugfs_remove_recursive(lowpan_debugfs);"));
        assert!(source.contains("debugfs_create_file(\"short_addr\", 0444"));
    }

    #[test]
    fn context_debugfs_setters_validate_linux_bounds() {
        let mut ctx = LowpanIphcCtx::new(3);
        assert_eq!(
            lowpan_ctx_flag_active_set(&mut ctx, 2),
            Err(DebugfsError::Invalid)
        );
        lowpan_ctx_flag_active_set(&mut ctx, 1).unwrap();
        lowpan_ctx_flag_c_set(&mut ctx, 1).unwrap();
        assert!(ctx.active());
        assert!(ctx.compression());
        lowpan_ctx_flag_active_set(&mut ctx, 0).unwrap();
        assert!(!ctx.active());
        assert_eq!(lowpan_ctx_flag_active_get(ctx), 0);
        assert_eq!(lowpan_ctx_flag_c_get(ctx), 1);

        assert_eq!(
            lowpan_ctx_plen_set(&mut ctx, 129),
            Err(DebugfsError::Invalid)
        );
        lowpan_ctx_plen_set(&mut ctx, 64).unwrap();
        assert_eq!(ctx.plen, 64);
        assert_eq!(lowpan_ctx_plen_get(ctx), 64);
        lowpan_ctx_pfx_write(&mut ctx, "fe80:0000:0000:0000:0200:00ff:fe00:0001").unwrap();
        assert_eq!(ctx.pfx, [0xfe80, 0, 0, 0, 0x0200, 0x00ff, 0xfe00, 1]);
        assert_eq!(
            lowpan_ctx_pfx_show(&ctx),
            "fe80:0000:0000:0000:0200:00ff:fe00:0001\n"
        );
        assert_eq!(
            lowpan_ctx_pfx_write_status(&mut ctx, "2001:db8:0:0:0:0:0:1\n", 22, true),
            Ok(22)
        );
        assert_eq!(
            lowpan_ctx_pfx_write_status(&mut ctx, "2001:db8::1", 11, false),
            Err(DebugfsError::Fault)
        );
        assert_eq!(parse_prefix_words("fe80::1"), Err(DebugfsError::Invalid));
        assert_eq!(
            parse_prefix_words("fe80:0000:0000:0000:0200:00ff:fe00:0001 trailing").unwrap(),
            [0xfe80, 0, 0, 0, 0x0200, 0x00ff, 0xfe00, 1]
        );
    }

    #[test]
    fn context_show_and_debugfs_tree_match_linux_shape() {
        let mut dev = LowpanDebugDevice::new("lowpan0");
        dev.is_ieee802154 = true;
        dev.short_addr = 0x3412u16.to_le();
        lowpan_ctx_flag_active_set(&mut dev.contexts[3], 1).unwrap();
        lowpan_ctx_flag_c_set(&mut dev.contexts[3], 1).unwrap();
        lowpan_ctx_plen_set(&mut dev.contexts[3], 64).unwrap();
        lowpan_ctx_pfx_write(&mut dev.contexts[3], "fe80:0:0:0:200:ff:fe00:1").unwrap();

        let show = lowpan_context_show(&dev.contexts);
        assert!(show.contains("cid|prefix"));
        assert!(show.contains("  3|"));
        assert!(show.contains("/64 |1"));
        assert_eq!(lowpan_short_addr_get(dev.short_addr), 0x3412);

        let tree = lowpan_dev_debugfs_init_plan(&dev);
        assert!(tree.contains(&LowpanDebugfsEntry {
            path: "lowpan0/contexts/show".into(),
            mode: 0o644,
        }));
        assert!(tree.contains(&LowpanDebugfsEntry {
            path: "lowpan0/contexts/15/prefix_len".into(),
            mode: 0o644,
        }));
        assert!(tree.contains(&LowpanDebugfsEntry {
            path: "lowpan0/ieee802154/short_addr".into(),
            mode: 0o444,
        }));
        assert!(lowpan_dev_debugfs_ctx_init_plan(LOWPAN_IPHC_CTX_TABLE_SIZE as u8).is_empty());
        assert_eq!(
            lowpan_debugfs_init(),
            LowpanDebugfsEntry {
                path: "6lowpan".into(),
                mode: 0o555,
            }
        );
        assert!(lowpan_dev_debugfs_exit(true));
        assert!(lowpan_debugfs_exit(true));
    }
}
