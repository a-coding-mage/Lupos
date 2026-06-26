//! linux-parity: partial
//! linux-source: vendor/linux/security/apparmor/apparmorfs.c
//! test-origin: linux:vendor/linux/security/apparmor/apparmorfs.c
//! AppArmor securityfs / apparmorfs control surface.
//!
//! Linux exposes AppArmor's public ABI under
//! `<securityfs>/apparmor`.  This module publishes the root namespace files,
//! policy-control entries, and feature tree.  The policy-control files accept a
//! minimal text-profile subset and bounded aa_ext-like binary profiles backed by
//! `apparmor`, and `.access` supports the Linux multi-transaction query commands
//! for bounded label/file-permission plus policy data-block lookups using
//! per-open transaction buffers. Binary policy loads flow through
//! `apparmor`'s Linux-shaped DFA and namespace-label replacement path.

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::fs::kernfs::KernfsNode;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EFBIG, EINVAL, ENOENT, ESPIPE};
use crate::security::inode::{
    securityfs_create_dir, securityfs_create_file, securityfs_create_file_with_open_ops,
    securityfs_create_symlink,
};

pub const AAFS_NAME: &str = "apparmorfs";
pub const POLICY_PERMSTABLE32: &str =
    "allow deny subtree cond kill complain prompt audit quiet hide xindex tag label";
pub const SIGNAL_MASK: &str = "hup int quit ill trap abrt bus fpe kill usr1 segv usr2 pipe alrm term stkflt chld cont stop stp ttin ttou urg xcpu xfsz vtalrm prof winch io pwr sys emt lost";
const QUERY_CMD_LABEL: &[u8] = b"label\0";
const QUERY_CMD_PROFILE: &[u8] = b"profile\0";
const QUERY_CMD_LABELALL: &[u8] = b"labelall\0";
const QUERY_CMD_DATA: &[u8] = b"data\0";
const MULTI_TRANSACTION_LIMIT: usize = 4096 - 16;

static APPARMORFS_INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init_securityfs() {
    if APPARMORFS_INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let apparmor = securityfs_create_dir("apparmor", None);
    create_root_files(&apparmor);
    create_features_tree(&apparmor);
    create_policy_tree(&apparmor);

    crate::kernel::printk::log_info!("AppArmor", "AppArmor Filesystem Enabled");
}

fn create_root_files(apparmor: &Arc<KernfsNode>) {
    securityfs_create_file_with_open_ops(
        ".access",
        0o666,
        Some(apparmor),
        Some(access_read),
        Some(access_write),
        Some(access_release),
    );
    securityfs_create_file(".stacked", 0o444, Some(apparmor), Some(no_show), None);
    securityfs_create_file(".ns_stacked", 0o444, Some(apparmor), Some(no_show), None);
    securityfs_create_file(".ns_level", 0o444, Some(apparmor), Some(zero_show), None);
    securityfs_create_file(".ns_name", 0o444, Some(apparmor), Some(ns_name_show), None);
    securityfs_create_file("profiles", 0o444, Some(apparmor), Some(profiles_show), None);
    securityfs_create_file(
        "raw_data_compression_level_min",
        0o444,
        Some(apparmor),
        Some(zero_show),
        None,
    );
    securityfs_create_file(
        "raw_data_compression_level_max",
        0o444,
        Some(apparmor),
        Some(zero_show),
        None,
    );
    securityfs_create_file(".load", 0o640, Some(apparmor), None, Some(policy_store));
    securityfs_create_file(".replace", 0o640, Some(apparmor), None, Some(policy_store));
    securityfs_create_file(".remove", 0o640, Some(apparmor), None, Some(policy_store));
    securityfs_create_file("revision", 0o444, Some(apparmor), Some(revision_show), None);
}

fn create_policy_tree(apparmor: &Arc<KernfsNode>) {
    let policy = securityfs_create_dir(".policy", Some(apparmor));
    securityfs_create_dir("profiles", Some(&policy));
    securityfs_create_dir("raw_data", Some(&policy));
    securityfs_create_dir("namespaces", Some(&policy));
    securityfs_create_file("revision", 0o444, Some(&policy), Some(revision_show), None);
    securityfs_create_file(".load", 0o640, Some(&policy), None, Some(policy_store));
    securityfs_create_file(".replace", 0o640, Some(&policy), None, Some(policy_store));
    securityfs_create_file(".remove", 0o640, Some(&policy), None, Some(policy_store));
    securityfs_create_symlink("policy", Some(apparmor), ".policy");
}

fn create_features_tree(apparmor: &Arc<KernfsNode>) {
    let features = securityfs_create_dir("features", Some(apparmor));

    let policy = securityfs_create_dir("policy", Some(&features));
    let versions = securityfs_create_dir("versions", Some(&policy));
    for version in ["v5", "v6", "v7", "v8", "v9"] {
        securityfs_create_file(version, 0o444, Some(&versions), Some(yes_show), None);
    }
    securityfs_create_file("set_load", 0o444, Some(&policy), Some(yes_show), None);
    securityfs_create_file(
        "outofband",
        0o444,
        Some(&policy),
        Some(outofband_show),
        None,
    );
    securityfs_create_file(
        "permstable32_version",
        0o444,
        Some(&policy),
        Some(permstable32_version_show),
        None,
    );
    securityfs_create_file(
        "permstable32",
        0o444,
        Some(&policy),
        Some(permstable32_show),
        None,
    );
    securityfs_create_file("state32", 0o444, Some(&policy), Some(one_hex_show), None);
    let unconfined = securityfs_create_dir("unconfined_restrictions", Some(&policy));
    securityfs_create_file(
        "change_profile",
        0o444,
        Some(&unconfined),
        Some(yes_show),
        None,
    );

    let domain = securityfs_create_dir("domain", Some(&features));
    for name in [
        "change_hat",
        "change_hatv",
        "unconfined_allowed_children",
        "change_onexec",
        "change_profile",
        "stack",
        "fix_binfmt_elf_mmap",
        "post_nnp_subset",
        "computed_longest_left",
        "disconnected.path",
        "kill.signal",
    ] {
        securityfs_create_file(name, 0o444, Some(&domain), Some(yes_show), None);
    }
    securityfs_create_file(
        "version",
        0o444,
        Some(&domain),
        Some(domain_version_show),
        None,
    );
    let attach = securityfs_create_dir("attach_conditions", Some(&domain));
    securityfs_create_file("xattr", 0o444, Some(&attach), Some(yes_show), None);

    let file = securityfs_create_dir("file", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&file), Some(file_mask_show), None);

    let network_v8 = securityfs_create_dir("network_v8", Some(&features));
    securityfs_create_file(
        "af_mask",
        0o444,
        Some(&network_v8),
        Some(network_af_mask_show),
        None,
    );
    let network_v9 = securityfs_create_dir("network_v9", Some(&features));
    securityfs_create_file(
        "af_mask",
        0o444,
        Some(&network_v9),
        Some(network_af_mask_show),
        None,
    );
    securityfs_create_file("af_unix", 0o444, Some(&network_v9), Some(yes_show), None);

    let mount = securityfs_create_dir("mount", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&mount), Some(mount_mask_show), None);
    securityfs_create_file(
        "move_mount",
        0o444,
        Some(&mount),
        Some(mount_move_show),
        None,
    );

    let namespaces = securityfs_create_dir("namespaces", Some(&features));
    securityfs_create_file("profile", 0o444, Some(&namespaces), Some(yes_show), None);
    securityfs_create_file("pivot_root", 0o444, Some(&namespaces), Some(no_show), None);
    securityfs_create_file(
        "mask",
        0o444,
        Some(&namespaces),
        Some(namespace_mask_show),
        None,
    );

    securityfs_create_file(
        "capability",
        0o444,
        Some(&features),
        Some(capability_flags_show),
        None,
    );

    let rlimit = securityfs_create_dir("rlimit", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&rlimit), Some(rlimit_mask_show), None);
    let caps = securityfs_create_dir("caps", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&caps), Some(caps_mask_show), None);
    securityfs_create_file("extended", 0o444, Some(&caps), Some(yes_show), None);

    let ptrace = securityfs_create_dir("ptrace", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&ptrace), Some(ptrace_mask_show), None);
    let signal = securityfs_create_dir("signal", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&signal), Some(signal_mask_show), None);
    let dbus = securityfs_create_dir("dbus", Some(&features));
    securityfs_create_file("mask", 0o444, Some(&dbus), Some(dbus_mask_show), None);
    let query = securityfs_create_dir("query", Some(&features));
    let label = securityfs_create_dir("label", Some(&query));
    securityfs_create_file(
        "perms",
        0o444,
        Some(&label),
        Some(query_label_perms_show),
        None,
    );
    securityfs_create_file("data", 0o444, Some(&label), Some(yes_show), None);
    securityfs_create_file(
        "multi_transaction",
        0o444,
        Some(&label),
        Some(yes_show),
        None,
    );
    let io_uring = securityfs_create_dir("io_uring", Some(&features));
    securityfs_create_file(
        "mask",
        0o444,
        Some(&io_uring),
        Some(io_uring_mask_show),
        None,
    );
}

fn copy_text(buf: &mut [u8], text: &str) -> Result<usize, i32> {
    let n = text.len().min(buf.len());
    buf[..n].copy_from_slice(&text.as_bytes()[..n]);
    Ok(n)
}

fn yes_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "yes\n")
}

fn no_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "no\n")
}

fn zero_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0\n")
}

fn one_hex_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0x000001\n")
}

fn outofband_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0x000001\n")
}

fn permstable32_version_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0x000003\n")
}

fn permstable32_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, POLICY_PERMSTABLE32).and_then(|n| {
        if n < buf.len() {
            copy_text(&mut buf[n..], "\n").map(|m| n + m)
        } else {
            Ok(n)
        }
    })
}

fn revision_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    let text = alloc::format!("{}\n", crate::security::apparmor::policy_revision());
    copy_text(buf, &text)
}

fn ns_name_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "root\n")
}

fn profiles_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, &crate::security::apparmor::profiles_text())
}

fn domain_version_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "1.2\n")
}

fn file_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "create read write exec append mmap_exec link lock\n")
}

fn network_af_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "local inet inet6 netlink packet\n")
}

fn mount_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "mount umount pivot_root\n")
}

fn mount_move_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "detached\n")
}

fn namespace_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "userns_create\n")
}

fn capability_flags_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "0x00ffffff\n")
}

fn rlimit_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(
        buf,
        "cpu fsize data stack core rss nproc nofile memlock as locks sigpending msgqueue nice rtprio rttime\n",
    )
}

fn caps_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(
        buf,
        "chown dac_override dac_read_search fowner fsetid kill setgid setuid setpcap linux_immutable net_bind_service net_broadcast net_admin net_raw ipc_lock ipc_owner sys_module sys_rawio sys_chroot sys_ptrace sys_pacct sys_admin sys_boot sys_nice sys_resource sys_time sys_tty_config mknod lease audit_write audit_control setfcap mac_override mac_admin syslog wake_alarm block_suspend audit_read perfmon bpf checkpoint_restore\n",
    )
}

fn ptrace_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "read trace\n")
}

fn signal_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, SIGNAL_MASK).and_then(|n| {
        if n < buf.len() {
            copy_text(&mut buf[n..], "\n").map(|m| n + m)
        } else {
            Ok(n)
        }
    })
}

fn dbus_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "acquire send receive\n")
}

fn query_label_perms_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "allow deny audit quiet\n")
}

fn io_uring_mask_show(_node: &Arc<KernfsNode>, buf: &mut [u8]) -> Result<usize, i32> {
    copy_text(buf, "sqpoll override_creds\n")
}

fn access_read(
    file: &FileRef,
    _node: &Arc<KernfsNode>,
    buf: &mut [u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    let private = file.private.lock();
    let ptr = *private as *const Vec<u8>;
    if ptr.is_null() {
        return Ok(0);
    }
    let response = unsafe { &*ptr };
    let start = (*pos as usize).min(response.len());
    let copy = (response.len() - start).min(buf.len());
    buf[..copy].copy_from_slice(&response[start..start + copy]);
    *pos += copy as u64;
    Ok(copy)
}

fn access_write(
    file: &FileRef,
    _node: &Arc<KernfsNode>,
    buf: &[u8],
    pos: &mut u64,
) -> Result<usize, i32> {
    if *pos != 0 {
        return Err(-ESPIPE);
    }
    if buf.len() > MULTI_TRANSACTION_LIMIT - 1 {
        return Err(-EFBIG);
    }
    let response = access_query(buf)?;
    access_set_response(file, response);
    Ok(buf.len())
}

fn access_release(file: FileRef, _node: &Arc<KernfsNode>) {
    drop(access_take_response(&file));
}

fn access_take_response(file: &FileRef) -> Option<Box<Vec<u8>>> {
    let mut private = file.private.lock();
    let ptr = *private as *mut Vec<u8>;
    if ptr.is_null() {
        return None;
    }
    *private = 0;
    Some(unsafe { Box::from_raw(ptr) })
}

fn access_set_response(file: &FileRef, response: Vec<u8>) {
    drop(access_take_response(file));
    *file.private.lock() = Box::into_raw(Box::new(response)) as usize;
}

fn access_query(buf: &[u8]) -> Result<Vec<u8>, i32> {
    if let Some(query) = buf.strip_prefix(QUERY_CMD_PROFILE) {
        return access_query_label(query);
    }
    if let Some(query) = buf.strip_prefix(QUERY_CMD_LABEL) {
        return access_query_label(query);
    }
    if let Some(query) = buf.strip_prefix(QUERY_CMD_LABELALL) {
        return access_query_label(query);
    }
    if let Some(query) = buf.strip_prefix(QUERY_CMD_DATA) {
        return access_query_data(query);
    }
    Err(-EINVAL)
}

fn split_nul_field(buf: &[u8]) -> Result<(&[u8], &[u8]), i32> {
    let Some(pos) = buf.iter().position(|byte| *byte == 0) else {
        return Err(-EINVAL);
    };
    let (field, rest) = buf.split_at(pos);
    if field.is_empty() {
        return Err(-EINVAL);
    }
    Ok((field, &rest[1..]))
}

fn access_query_label(query: &[u8]) -> Result<Vec<u8>, i32> {
    let (label, match_bytes) = split_nul_field(query)?;
    if match_bytes.is_empty() {
        return Err(-EINVAL);
    }
    let label = core::str::from_utf8(label).map_err(|_| -EINVAL)?;
    let perms = crate::security::apparmor::query_label_permissions(label, match_bytes)?;
    Ok(alloc::format!(
        "allow 0x{allow:08x}\ndeny 0x{deny:08x}\naudit 0x{audit:08x}\nquiet 0x{quiet:08x}\n",
        allow = perms.allow,
        deny = perms.deny,
        audit = perms.audit,
        quiet = perms.quiet,
    )
    .into_bytes())
}

fn access_query_data(query: &[u8]) -> Result<Vec<u8>, i32> {
    let (label, key_with_nul) = split_nul_field(query)?;
    let (key, trailing) = split_nul_field(key_with_nul)?;
    if !trailing.is_empty() || key.is_empty() {
        return Err(-EINVAL);
    }
    let label = core::str::from_utf8(label).map_err(|_| -EINVAL)?;
    let key = core::str::from_utf8(key).map_err(|_| -EINVAL)?;
    let blocks = crate::security::apparmor::query_label_data(label, key)?;
    let block_count = u32::try_from(blocks.len()).map_err(|_| -EINVAL)?;
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&block_count.to_le_bytes());
    for block in blocks.iter() {
        let len = u32::try_from(block.len()).map_err(|_| -EINVAL)?;
        let projected = out
            .len()
            .checked_add(core::mem::size_of::<u32>())
            .and_then(|len| len.checked_add(block.len()))
            .ok_or(-EINVAL)?;
        if projected > MULTI_TRANSACTION_LIMIT {
            return Err(-EINVAL);
        }
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(block);
    }
    let total = u32::try_from(out.len().saturating_sub(core::mem::size_of::<u32>()))
        .map_err(|_| -EINVAL)?;
    out[..4].copy_from_slice(&total.to_le_bytes());
    Ok(out)
}

fn policy_store(node: &Arc<KernfsNode>, buf: &[u8]) -> Result<usize, i32> {
    match node.name.as_str() {
        ".remove" => crate::security::apparmor::remove_policy_blob(buf),
        ".replace" => crate::security::apparmor::replace_policy_blob(buf),
        _ => crate::security::apparmor::load_policy_blob(buf),
    }
}

#[cfg(test)]
pub fn reset_for_test() {
    APPARMORFS_INITIALIZED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::file::{alloc_file, fput};
    use crate::fs::kernfs::{KERNFS_FILE_FILE_OPS, KernfsKind, lookup};
    use crate::fs::read_write::{vfs_read, vfs_write};
    use crate::fs::types::{Dentry, SuperBlock};
    use crate::include::uapi::fcntl::O_RDWR;

    fn read(node: &Arc<KernfsNode>) -> Vec<u8> {
        let KernfsKind::File { show, .. } = &node.kind else {
            panic!("not a file");
        };
        let mut buf = [0u8; 512];
        let n = (show.expect("show fn"))(node, &mut buf).expect("show ok");
        buf[..n].to_vec()
    }

    fn show(node: &Arc<KernfsNode>) -> alloc::string::String {
        core::str::from_utf8(&read(node)).unwrap().into()
    }

    fn store(node: &Arc<KernfsNode>, bytes: &[u8]) -> Result<usize, i32> {
        let KernfsKind::File { store, .. } = &node.kind else {
            panic!("not a file");
        };
        (store.expect("store fn"))(node, bytes)
    }

    fn open_kernfs_file(node: &Arc<KernfsNode>) -> FileRef {
        let sb = SuperBlock::alloc(
            "securityfs-test",
            0x73636673,
            &crate::fs::ops::NOOP_SUPER_OPS,
        );
        let inode = crate::fs::kernfs::inode_for_node(&sb, node.clone());
        let dentry = Dentry::new_negative(node.name.as_str());
        dentry.instantiate(inode);
        alloc_file(dentry, O_RDWR, 0, &KERNFS_FILE_FILE_OPS)
    }

    fn read_file(file: &FileRef) -> Vec<u8> {
        let mut buf = [0u8; 512];
        let n = vfs_read(file, &mut buf).expect("read ok");
        buf[..n].to_vec()
    }

    #[test]
    fn apparmor_securityfs_tree_exposes_policy_controls_and_features() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::inode::reset_for_test();
        crate::security::apparmor::reset_for_test();
        reset_for_test();

        init_securityfs();

        let root = crate::security::inode::securityfs_root();
        let apparmor = lookup(&root, "apparmor").expect("apparmor dir");
        let load = lookup(&apparmor, ".load").expect(".load");
        let replace = lookup(&apparmor, ".replace").expect(".replace");
        let remove = lookup(&apparmor, ".remove").expect(".remove");
        assert_eq!(load.mode & 0o777, 0o640);
        assert_eq!(
            store(&load, b"profile demo flags=(complain) { file, }\n"),
            Ok(40)
        );
        assert_eq!(
            store(
                &replace,
                b"profile demo flags=(attach_disconnected) { file, }\n"
            ),
            Ok(51)
        );
        assert_eq!(
            show(&lookup(&apparmor, "revision").expect("revision")),
            "2\n"
        );
        assert_eq!(
            show(&lookup(&apparmor, ".ns_name").expect(".ns_name")),
            "root\n"
        );
        assert_eq!(
            show(&lookup(&apparmor, "profiles").expect("profiles")),
            "demo (enforce)\n"
        );
        assert_eq!(store(&remove, b"demo\n"), Ok(5));
        assert_eq!(show(&lookup(&apparmor, "profiles").expect("profiles")), "");

        let policy_link = lookup(&apparmor, "policy").expect("policy symlink");
        match &policy_link.kind {
            KernfsKind::Symlink { target } => assert_eq!(target, ".policy"),
            _ => panic!("policy must be a symlink"),
        }
        let policy_tree = lookup(&apparmor, ".policy").expect(".policy");
        assert!(lookup(&policy_tree, "profiles").is_some());
        assert_eq!(
            lookup(&policy_tree, ".load").expect("policy .load").mode & 0o777,
            0o640
        );

        let features = lookup(&apparmor, "features").expect("features dir");
        let policy = lookup(&features, "policy").expect("features/policy");
        let versions = lookup(&policy, "versions").expect("policy/versions");
        assert_eq!(show(&lookup(&versions, "v9").expect("v9")), "yes\n");
        assert_eq!(
            show(&lookup(&policy, "permstable32").expect("permstable32")),
            alloc::format!("{POLICY_PERMSTABLE32}\n")
        );
        let file = lookup(&features, "file").expect("features/file");
        assert!(show(&lookup(&file, "mask").expect("file/mask")).contains("mmap_exec"));
        let network_v9 = lookup(&features, "network_v9").expect("network_v9");
        assert_eq!(
            show(&lookup(&network_v9, "af_unix").expect("af_unix")),
            "yes\n"
        );
        let query = lookup(&features, "query").expect("query");
        let label = lookup(&query, "label").expect("label");
        assert_eq!(
            show(&lookup(&label, "perms").expect("perms")),
            "allow deny audit quiet\n"
        );
    }

    #[test]
    fn apparmor_policy_store_requires_mac_admin_capability() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::inode::reset_for_test();
        crate::security::apparmor::reset_for_test();
        reset_for_test();

        init_securityfs();

        let root = crate::security::inode::securityfs_root();
        let apparmor = lookup(&root, "apparmor").expect("apparmor dir");
        let load = lookup(&apparmor, ".load").expect(".load");

        let previous = unsafe { crate::kernel::sched::get_current() };
        let mut current =
            Box::new(unsafe { core::mem::zeroed::<crate::kernel::task::TaskStruct>() });
        let cred = crate::kernel::cred::prepare_creds().expect("cred");
        unsafe {
            (*cred).cap_effective = crate::kernel::capability::KernelCapT::empty();
            current.pid = 1000;
            current.tgid = 1000;
            current.cred = cred;
            current.m27.real_cred = cred;
            crate::kernel::sched::set_current(
                &mut *current as *mut crate::kernel::task::TaskStruct,
            );
        }

        let store_result = store(&load, b"profile denied { file, }\n");
        let profiles = show(&lookup(&apparmor, "profiles").expect("profiles"));

        unsafe {
            crate::kernel::sched::set_current(previous);
            crate::kernel::cred::Cred::put(cred);
        }

        assert_eq!(store_result, Err(-crate::include::uapi::errno::EPERM));
        assert_eq!(profiles, "");
    }

    #[test]
    fn apparmor_access_file_answers_linux_label_and_data_queries() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::inode::reset_for_test();
        crate::security::apparmor::reset_for_test();
        reset_for_test();

        init_securityfs();

        let root = crate::security::inode::securityfs_root();
        let apparmor = lookup(&root, "apparmor").expect("apparmor dir");
        let load = lookup(&apparmor, ".load").expect(".load");
        let access = lookup(&apparmor, ".access").expect(".access");
        let access_file = open_kernfs_file(&access);
        let policy = b"profile access.demo {\n  /etc/** r,\n  deny /etc/shadow r,\n}\n";
        assert_eq!(store(&load, policy), Ok(policy.len()));

        let mut query = b"label\0access.demo\0".to_vec();
        query.push(crate::security::apparmor::AA_CLASS_FILE);
        query.extend_from_slice(b"/etc/passwd");
        assert_eq!(vfs_write(&access_file, &query), Ok(query.len()));
        assert_eq!(
            core::str::from_utf8(&read_file(&access_file)).unwrap(),
            "allow 0x00000044\ndeny 0x00000000\naudit 0x00000000\nquiet 0x00000000\n"
        );

        let access_file = open_kernfs_file(&access);
        let mut denied = b"labelall\0access.demo\0".to_vec();
        denied.push(crate::security::apparmor::AA_CLASS_FILE);
        denied.extend_from_slice(b"/etc/shadow");
        assert_eq!(vfs_write(&access_file, &denied), Ok(denied.len()));
        assert_eq!(
            core::str::from_utf8(&read_file(&access_file)).unwrap(),
            "allow 0x00000044\ndeny 0x00000004\naudit 0x00000000\nquiet 0x00000000\n"
        );

        let access_file = open_kernfs_file(&access);
        let data = b"data\0access.demo\0missing-key\0";
        assert_eq!(vfs_write(&access_file, data), Ok(data.len()));
        assert_eq!(read_file(&access_file), [4, 0, 0, 0, 0, 0, 0, 0]);

        let data_policy = binary_profile_with_data("data.demo", &[("os-release", b"ID=arch\n")]);
        assert_eq!(store(&load, &data_policy), Ok(data_policy.len()));
        let access_file = open_kernfs_file(&access);
        let data = b"data\0data.demo\0os-release\0";
        assert_eq!(vfs_write(&access_file, data), Ok(data.len()));
        let mut expected = Vec::new();
        expected.extend_from_slice(&18u32.to_le_bytes());
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&8u32.to_le_bytes());
        expected.extend_from_slice(b"ID=arch\n");
        assert_eq!(read_file(&access_file), expected);

        let access_file = open_kernfs_file(&access);
        assert_eq!(vfs_write(&access_file, b"bogus\0"), Err(-EINVAL));
    }

    #[test]
    fn apparmor_access_transactions_are_file_local_and_pos_sensitive() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::inode::reset_for_test();
        crate::security::apparmor::reset_for_test();
        reset_for_test();

        init_securityfs();

        let root = crate::security::inode::securityfs_root();
        let apparmor = lookup(&root, "apparmor").expect("apparmor dir");
        let load = lookup(&apparmor, ".load").expect(".load");
        let access = lookup(&apparmor, ".access").expect(".access");
        let policy = b"profile peropen.demo {\n  /etc/** r,\n  deny /etc/shadow r,\n}\n";
        assert_eq!(store(&load, policy), Ok(policy.len()));

        let first = open_kernfs_file(&access);
        let second = open_kernfs_file(&access);

        let mut allowed = b"label\0peropen.demo\0".to_vec();
        allowed.push(crate::security::apparmor::AA_CLASS_FILE);
        allowed.extend_from_slice(b"/etc/passwd");
        assert_eq!(vfs_write(&first, &allowed), Ok(allowed.len()));

        let mut denied = b"labelall\0peropen.demo\0".to_vec();
        denied.push(crate::security::apparmor::AA_CLASS_FILE);
        denied.extend_from_slice(b"/etc/shadow");
        assert_eq!(vfs_write(&second, &denied), Ok(denied.len()));

        assert_eq!(
            core::str::from_utf8(&read_file(&first)).unwrap(),
            "allow 0x00000044\ndeny 0x00000000\naudit 0x00000000\nquiet 0x00000000\n"
        );
        assert_eq!(
            core::str::from_utf8(&read_file(&second)).unwrap(),
            "allow 0x00000044\ndeny 0x00000004\naudit 0x00000000\nquiet 0x00000000\n"
        );

        assert_eq!(vfs_write(&first, &allowed), Err(-ESPIPE));
        fput(first);
        fput(second);
    }

    #[test]
    fn apparmor_init_publishes_apparmorfs_once() {
        let _guard = crate::security::lsm_list::TEST_LSM_LOCK.lock();
        crate::security::lsm_list::reset_for_test();
        crate::security::inode::reset_for_test();
        crate::security::apparmor::reset_for_test();
        reset_for_test();

        crate::security::apparmor::init();
        crate::security::apparmor::init();

        let root = crate::security::inode::securityfs_root();
        let apparmor = lookup(&root, "apparmor").expect("apparmor dir");
        assert!(lookup(&apparmor, "features").is_some());
        assert!(lookup(&apparmor, ".load").is_some());
    }

    fn binary_profile_with_data(name: &str, data: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        named_u32(
            &mut out,
            "version",
            crate::security::apparmor::AA_POLICY_ABI_MAX,
        );
        named_struct_start(&mut out, "profile");
        named_string(&mut out, "name", name);
        named_u32(&mut out, "mode", 0);
        named_struct_start(&mut out, "data");
        for (key, value) in data {
            raw_string(&mut out, key);
            raw_blob(&mut out, value);
        }
        out.push(crate::security::apparmor::AA_EXT_STRUCTEND);
        out.push(crate::security::apparmor::AA_EXT_STRUCTEND);
        out
    }

    fn named_u32(out: &mut Vec<u8>, name: &str, value: u32) {
        named(out, name);
        out.push(crate::security::apparmor::AA_EXT_U32);
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn named_string(out: &mut Vec<u8>, name: &str, value: &str) {
        named(out, name);
        raw_string(out, value);
    }

    fn named_struct_start(out: &mut Vec<u8>, name: &str) {
        named(out, name);
        out.push(crate::security::apparmor::AA_EXT_STRUCT);
    }

    fn named(out: &mut Vec<u8>, name: &str) {
        out.push(crate::security::apparmor::AA_EXT_NAME);
        let len = name.len() + 1;
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.push(0);
    }

    fn raw_string(out: &mut Vec<u8>, value: &str) {
        out.push(crate::security::apparmor::AA_EXT_STRING);
        let len = value.len() + 1;
        out.extend_from_slice(&(len as u16).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out.push(0);
    }

    fn raw_blob(out: &mut Vec<u8>, value: &[u8]) {
        out.push(crate::security::apparmor::AA_EXT_BLOB);
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
    }
}
