schema_version	implementation-v1
task_id	S016315
attempt	4
pipeline_id	P01
linux_path	include/uapi/linux/nfsacl.h
destination_path	src/include/uapi/linux/nfsacl.rs
linux_revision	425f94c2954b1fe80ebdbf9b29854e89750355df
architectures	common
source_sha256	8dcebe07f0253944052f0538926bb3a614a8ca521e7354687bb5dbb01a46445b
selected_symbols	_UAPI__LINUX_NFSACL_H;NFS_ACL_PROGRAM;ACLPROC2_NULL;ACLPROC2_GETACL;ACLPROC2_SETACL;ACLPROC2_GETATTR;ACLPROC2_ACCESS;ACLPROC3_NULL;ACLPROC3_GETACL;ACLPROC3_SETACL;NFS_ACL;NFS_ACLCNT;NFS_DFACL;NFS_DFACLCNT;NFS_ACL_MASK;NFS_ACL_DEFAULT
implementation	The complete pinned header is represented by a fresh Rust module. The include guard is retained as a hidden unit marker; all 15 value macros retain their names, numeric values, and declaration order. No configuration branches exist beyond the source guard.
unsafe	None required.
verification	Manual source comparison only; no compiler, formatter, linker, test, or runtime command was run.
