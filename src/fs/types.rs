//! linux-parity: complete
//! linux-source: vendor/linux/fs
//! test-origin: linux:vendor/linux/fs
//! Core VFS types — `SuperBlock`, `Inode`, `Dentry`, `File`.
//!
//! Refcounting uses `Arc`; explicit `iget` / `iput` / `dget` / `dput`
//! helpers preserve Linux call-site spelling.  Field layout is *not*
//! pahole-pinned — these are private kernel structs, and ABI parity
//! lives at the syscall boundary (`stat`, `dirent`, etc.).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

use spin::{Mutex, RwLock};

use super::ops::{FileOps, InodeOps, SuperOps};
use crate::include::uapi::stat::{S_IFMT, S_ISGID};
use crate::kernel::cred::current_cred;
use crate::mm::address_space::AddressSpace;

pub type Ino = u64;

/// File-mode `S_IFMT` selector — what kind of object an inode represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InodeKind {
    Regular,
    Directory,
    Symlink,
    Chardev,
    Blockdev,
    Fifo,
    Socket,
}

impl InodeKind {
    pub const fn s_ifmt(self) -> u32 {
        use crate::include::uapi::stat::*;
        match self {
            Self::Regular => S_IFREG,
            Self::Directory => S_IFDIR,
            Self::Symlink => S_IFLNK,
            Self::Chardev => S_IFCHR,
            Self::Blockdev => S_IFBLK,
            Self::Fifo => S_IFIFO,
            Self::Socket => S_IFSOCK,
        }
    }
}

pub type SuperBlockRef = Arc<SuperBlock>;
pub type InodeRef = Arc<Inode>;
pub type DentryRef = Arc<Dentry>;
pub type FileRef = Arc<File>;

/// Filesystem-private inode data — opaque to the VFS.  ramfs stores file
/// bytes here; tmpfs reuses it for shmem page handles; procfs and sysfs
/// stash a callback table.
pub enum InodePrivate {
    None,
    /// In-memory file payload (ramfs, tmpfs early path).
    RamBytes(Mutex<Vec<u8>>),
    /// Read-only payload borrowed from the installed initramfs image.
    StaticBytes(&'static [u8]),
    /// Borrowed initramfs payload that promotes to RAM on first write.
    StaticCowBytes {
        base: &'static [u8],
        overlay: Mutex<Option<Vec<u8>>>,
    },
    /// In-memory directory entries — name → child inode.
    RamDir(Mutex<BTreeMap<String, InodeRef>>),
    /// Filesystem-specific opaque pointer, owned by the FS module.
    Opaque(usize),
}

/// `struct inode` — VFS in-core file representation.
pub struct Inode {
    pub ino: Ino,
    pub kind: InodeKind,
    pub mode: AtomicU32,
    pub uid: AtomicU32,
    pub gid: AtomicU32,
    pub size: AtomicU64,
    pub nlink: AtomicU32,
    pub atime: AtomicU64,
    pub mtime: AtomicU64,
    pub ctime: AtomicU64,
    /// Device number for special (char/block) inodes, in Linux
    /// `new_encode_dev()` form (`major<<8 | minor` for small numbers). Zero for
    /// non-device inodes. `stat(2)` reports this as `st_rdev`; userspace (e.g.
    /// Xorg's `xf86HasTTYs()`) keys VT/console behaviour off `major(st_rdev)`.
    pub rdev: AtomicU64,
    /// VFS reference count (Arc strong count is the source of truth; this is
    /// kept for diagnostics + Linux-style `i_count` callers).
    pub i_count: AtomicUsize,
    pub ops: &'static InodeOps,
    pub fops: &'static FileOps,
    /// Linux `inode::i_data`, with `i_mapping` pointing at this same object.
    /// Keeping the mapping in the inode makes independently opened mappings of
    /// one file share page-cache pages, as they do in vendor Linux.
    pub i_data: AddressSpace,
    pub sb: Mutex<Option<SuperBlockRef>>,
    /// In-memory extended attributes for VFS-backed synthetic filesystems.
    /// Linux keeps this behind each filesystem's xattr handlers; Lupos stores
    /// the generic ramfs/tmpfs/proc-facing state here until per-FS backends
    /// grow their own persistence.
    pub xattrs: Mutex<BTreeMap<String, Vec<u8>>>,
    pub private: InodePrivate,
}

impl Inode {
    pub fn new(
        ino: Ino,
        kind: InodeKind,
        mode: u32,
        ops: &'static InodeOps,
        fops: &'static FileOps,
        private: InodePrivate,
    ) -> InodeRef {
        let mut inode = Arc::new(Self {
            ino,
            kind,
            mode: AtomicU32::new(mode | kind.s_ifmt()),
            uid: AtomicU32::new(0),
            gid: AtomicU32::new(0),
            size: AtomicU64::new(0),
            nlink: AtomicU32::new(1),
            atime: AtomicU64::new(0),
            mtime: AtomicU64::new(0),
            ctime: AtomicU64::new(0),
            rdev: AtomicU64::new(0),
            i_count: AtomicUsize::new(1),
            ops,
            fops,
            i_data: AddressSpace::new(),
            sb: Mutex::new(None),
            xattrs: Mutex::new(BTreeMap::new()),
            private,
        });
        let host = Arc::as_ptr(&inode) as *mut u8;
        // No reference has escaped yet, so the embedded mapping can be linked
        // to its stable Arc allocation exactly once before returning it.
        Arc::get_mut(&mut inode)
            .expect("new inode must be uniquely owned")
            .i_data
            .host = host;
        inode
    }
    pub fn is_dir(&self) -> bool {
        self.kind == InodeKind::Directory
    }
    pub fn is_reg(&self) -> bool {
        self.kind == InodeKind::Regular
    }

    #[inline]
    pub fn mapping(&self) -> *mut AddressSpace {
        (&raw const self.i_data).cast_mut()
    }
}

impl Drop for Inode {
    fn drop(&mut self) {
        // The final inode reference can only disappear after every File/VMA
        // reference has gone away.  Linux's evict path then truncates i_data so
        // cached folios cannot retain a dangling mapping pointer.
        unsafe {
            crate::mm::filemap::truncate_inode_pages_final(&raw mut self.i_data);
        }
    }
}

/// Fallback wall-clock timestamp for synthetic inodes when the platform RTC is
/// unavailable. Linux initialises ramfs inodes with current_time(); this keeps
/// Lupos from exposing epoch-zero metadata to userland when CMOS time is absent.
pub const DEFAULT_INODE_TIMESTAMP_SECS: u64 = 1_779_194_096;

pub fn current_inode_timestamp_secs() -> u64 {
    let seconds = crate::kernel::time::ktime_get_real() / 1_000_000_000;
    if seconds == 0 {
        DEFAULT_INODE_TIMESTAMP_SECS
    } else {
        seconds
    }
}

pub fn init_inode_metadata(inode: &InodeRef, uid: u32, gid: u32, nlink: u32, timestamp: u64) {
    let ts = if timestamp == 0 {
        current_inode_timestamp_secs()
    } else {
        timestamp
    };
    inode.uid.store(uid, Ordering::Release);
    inode.gid.store(gid, Ordering::Release);
    inode.nlink.store(nlink.max(1), Ordering::Release);
    inode.atime.store(ts, Ordering::Release);
    inode.mtime.store(ts, Ordering::Release);
    inode.ctime.store(ts, Ordering::Release);
}

/// Linux `inode_init_owner()` parity for synthetic filesystems.
///
/// Ref: `vendor/linux/fs/inode.c::inode_init_owner`
pub fn init_inode_owner(inode: &InodeRef, dir: Option<&InodeRef>, mode: u32) {
    let cred = current_cred();
    let (uid, current_gid) = if cred.is_null() {
        (0, 0)
    } else {
        unsafe { ((*cred).fsuid.0, (*cred).fsgid.0) }
    };

    let mut owned_mode = mode;
    let gid = if let Some(parent) =
        dir.filter(|parent| parent.mode.load(Ordering::Acquire) & S_ISGID != 0)
    {
        if inode.kind == InodeKind::Directory {
            owned_mode |= S_ISGID;
        }
        parent.gid.load(Ordering::Acquire)
    } else {
        current_gid
    };

    let file_type = if owned_mode & S_IFMT != 0 {
        owned_mode & S_IFMT
    } else {
        inode.kind.s_ifmt()
    };
    let perms = owned_mode & !S_IFMT;

    inode.uid.store(uid, Ordering::Release);
    inode.gid.store(gid, Ordering::Release);
    inode.mode.store(file_type | perms, Ordering::Release);
}

pub fn touch_inode_now(inode: &InodeRef) {
    let ts = current_inode_timestamp_secs();
    inode.mtime.store(ts, Ordering::Release);
    inode.ctime.store(ts, Ordering::Release);
}

/// `struct dentry` — name → inode binding in the dcache.
pub struct Dentry {
    pub name: String,
    pub parent: Mutex<Option<DentryRef>>,
    pub inode: Mutex<Option<InodeRef>>,
    pub children: RwLock<BTreeMap<String, DentryRef>>,
    pub d_count: AtomicUsize,
    pub flags: AtomicU32,
}

pub const DCACHE_NEGATIVE: u32 = 1 << 0;
pub const DCACHE_MOUNTED: u32 = 1 << 1;

impl Dentry {
    pub fn new_negative(name: &str) -> DentryRef {
        Arc::new(Self {
            name: String::from(name),
            parent: Mutex::new(None),
            inode: Mutex::new(None),
            children: RwLock::new(BTreeMap::new()),
            d_count: AtomicUsize::new(1),
            flags: AtomicU32::new(DCACHE_NEGATIVE),
        })
    }

    pub fn instantiate(self: &DentryRef, inode: InodeRef) {
        *self.inode.lock() = Some(inode);
        let cur = self.flags.load(Ordering::Acquire);
        self.flags.store(cur & !DCACHE_NEGATIVE, Ordering::Release);
    }

    pub fn is_negative(&self) -> bool {
        self.flags.load(Ordering::Acquire) & DCACHE_NEGATIVE != 0
    }
    pub fn inode(&self) -> Option<InodeRef> {
        self.inode.lock().clone()
    }
}

/// `struct file` — opened file, holds the dentry pointing at the inode.
pub struct File {
    pub dentry: DentryRef,
    pub path_hint: Mutex<Option<String>>,
    pub pos: Mutex<u64>,
    pub flags: AtomicU32,
    pub mode: u32,
    pub fops: &'static FileOps,
    pub f_count: AtomicUsize,
    /// FS-private file state (e.g., readdir cursor).
    pub private: Mutex<usize>,
}

impl File {
    pub fn new(dentry: DentryRef, flags: u32, mode: u32, fops: &'static FileOps) -> FileRef {
        Arc::new(Self {
            dentry,
            path_hint: Mutex::new(None),
            pos: Mutex::new(0),
            flags: AtomicU32::new(flags),
            mode,
            fops,
            f_count: AtomicUsize::new(1),
            private: Mutex::new(0),
        })
    }
    pub fn inode(&self) -> Option<InodeRef> {
        self.dentry.inode()
    }

    pub fn mapping(&self) -> Option<*mut AddressSpace> {
        self.inode().map(|inode| inode.mapping())
    }
}

/// `struct super_block` — mounted-filesystem instance.
pub struct SuperBlock {
    pub magic: u64,
    pub blocksize: u32,
    pub fs_name: &'static str,
    /// Linux `struct super_block::s_uuid`.  EVM includes this UUID in HMACs
    /// when `CONFIG_EVM_ATTR_FSUUID` is enabled.
    pub uuid: Mutex<[u8; 16]>,
    pub root: Mutex<Option<DentryRef>>,
    pub ops: &'static SuperOps,
    /// FS-private next-ino allocator (shared by ramfs/tmpfs/procfs).
    pub next_ino: AtomicU64,
}

impl SuperBlock {
    pub fn alloc(fs_name: &'static str, magic: u64, ops: &'static SuperOps) -> SuperBlockRef {
        Arc::new(Self {
            magic,
            blocksize: 4096,
            fs_name,
            uuid: Mutex::new([0; 16]),
            root: Mutex::new(None),
            ops,
            next_ino: AtomicU64::new(1),
        })
    }
    pub fn set_uuid(&self, uuid: [u8; 16]) {
        *self.uuid.lock() = uuid;
    }
    pub fn uuid(&self) -> [u8; 16] {
        *self.uuid.lock()
    }
    pub fn alloc_ino(&self) -> Ino {
        self.next_ino.fetch_add(1, Ordering::AcqRel)
    }
    pub fn root(&self) -> Option<DentryRef> {
        self.root.lock().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inode_kind_modes_match_linux() {
        assert_eq!(InodeKind::Regular.s_ifmt(), 0o100000);
        assert_eq!(InodeKind::Directory.s_ifmt(), 0o040000);
        assert_eq!(InodeKind::Symlink.s_ifmt(), 0o120000);
    }

    #[test]
    fn negative_dentry_starts_unbound() {
        let d = Dentry::new_negative("foo");
        assert!(d.is_negative());
        assert!(d.inode().is_none());
    }
}
