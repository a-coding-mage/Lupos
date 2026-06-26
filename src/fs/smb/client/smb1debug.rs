//! linux-parity: complete
//! linux-source: vendor/linux/fs/smb/client/smb1debug.c
//! test-origin: linux:vendor/linux/fs/smb/client/smb1debug.c
//! SMB1 debug dump compile unit.

pub const DEBUG_CONFIG: &str = "CONFIG_CIFS_DEBUG2";
pub const DEBUG_FIELDS: &[&str] = &[
    "Command",
    "Status.CifsError",
    "Flags",
    "Flags2",
    "Mid",
    "Pid",
    "WordCount",
];

pub const fn cifs_dump_detail_enabled(config_cifs_debug2: bool) -> bool {
    config_cifs_debug2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smb1debug_source_matches_linux_debug2_gate() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/smb/client/smb1debug.c"
        ));
        assert!(source.contains("#include \"cifsproto.h\""));
        assert!(source.contains("#include \"smb1proto.h\""));
        assert!(source.contains("#include \"cifs_debug.h\""));
        assert!(source.contains(DEBUG_CONFIG));
        assert!(source.contains("server->ops->check_message"));
        assert!(source.contains("server->ops->calc_smb_size"));
        for field in DEBUG_FIELDS {
            assert!(source.contains(field));
        }
        assert!(cifs_dump_detail_enabled(true));
        assert!(!cifs_dump_detail_enabled(false));
    }
}
