//! linux-parity: complete
//! linux-source: vendor/linux/fs/fs_pin.c
//! test-origin: linux:vendor/linux/fs/fs_pin.c
//! VFS pin list removal and kill-state transitions.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FsPinKillAction {
    NullPinUnlockRcu,
    CallKill,
    AlreadyDone,
    WaitForRemoval,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FsPinState {
    pub on_mount_list: bool,
    pub on_superblock_list: bool,
    pub done: i8,
    pub wake_waiters: bool,
}

pub const fn pin_insert_state() -> FsPinState {
    FsPinState {
        on_mount_list: true,
        on_superblock_list: true,
        done: 0,
        wake_waiters: false,
    }
}

pub const fn pin_remove_state(mut state: FsPinState) -> FsPinState {
    state.on_mount_list = false;
    state.on_superblock_list = false;
    state.done = 1;
    state.wake_waiters = true;
    state
}

pub const fn pin_kill_action(done: Option<i8>) -> FsPinKillAction {
    match done {
        None => FsPinKillAction::NullPinUnlockRcu,
        Some(0) => FsPinKillAction::CallKill,
        Some(value) if value > 0 => FsPinKillAction::AlreadyDone,
        Some(_) => FsPinKillAction::WaitForRemoval,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fs_pin_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/fs_pin.c"
        ));
        assert!(source.contains("#include <linux/fs.h>"));
        assert!(source.contains("#include \"internal.h\""));
        assert!(source.contains("#include \"mount.h\""));
        assert!(source.contains("static DEFINE_SPINLOCK(pin_lock);"));
        assert!(source.contains("void pin_remove"));
        assert!(source.contains("hlist_del_init(&pin->m_list);"));
        assert!(source.contains("hlist_del_init(&pin->s_list);"));
        assert!(source.contains("pin->done = 1;"));
        assert!(source.contains("wake_up_locked(&pin->wait);"));
        assert!(source.contains("void pin_insert"));
        assert!(source.contains("hlist_add_head(&pin->s_list, &m->mnt_sb->s_pins);"));
        assert!(source.contains("hlist_add_head(&pin->m_list, &real_mount(m)->mnt_pins);"));
        assert!(source.contains("void pin_kill"));
        assert!(source.contains("if (!p)"));
        assert!(source.contains("p->done = -1;"));
        assert!(source.contains("p->kill(p);"));
        assert!(source.contains("if (p->done > 0)"));
        assert!(source.contains("__add_wait_queue(&p->wait, &wait);"));
        assert!(source.contains("void mnt_pin_kill"));
        assert!(source.contains("void group_pin_kill"));

        let inserted = pin_insert_state();
        assert!(inserted.on_mount_list);
        assert!(inserted.on_superblock_list);
        let removed = pin_remove_state(inserted);
        assert!(!removed.on_mount_list);
        assert!(!removed.on_superblock_list);
        assert_eq!(removed.done, 1);
        assert!(removed.wake_waiters);
        assert_eq!(pin_kill_action(None), FsPinKillAction::NullPinUnlockRcu);
        assert_eq!(pin_kill_action(Some(0)), FsPinKillAction::CallKill);
        assert_eq!(pin_kill_action(Some(1)), FsPinKillAction::AlreadyDone);
        assert_eq!(pin_kill_action(Some(-1)), FsPinKillAction::WaitForRemoval);
    }
}
