//! linux-parity: partial
//! linux-source: vendor/linux/kernel/rcu
//! RCU — Read-Copy-Update (M34).
//!
//! Will be filled in by the M34 step.  Module declared early so M33's
//! locking primitives can reference `RcuHead` once the wiring lands.

pub mod rcuscale;
pub mod rcutorture;
pub mod refscale;
pub mod segcblist;
pub mod srcu;
pub mod srcutree;
pub mod sync;
pub mod tasks;
pub mod tiny;
pub mod tree;
pub mod types;
pub mod update;

pub use srcu::{SrcuStruct, srcu_read_lock, srcu_read_unlock, synchronize_srcu};
pub use tasks::{call_rcu_tasks, synchronize_rcu_tasks, tasks_rcu_qs};
pub use tree::{
    call_rcu, rcu_barrier, rcu_check_callbacks, rcu_init, rcu_qs, rcu_read_lock, rcu_read_unlock,
    synchronize_rcu,
};
pub use types::{RcuHead, rcu_head_init};

use crate::kernel::module::{export_symbol, find_symbol};

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

/// Register the out-of-line PREEMPT_RCU entry points referenced by modules
/// built with the vendor configuration.
pub fn register_module_exports() {
    export_symbol_once("__rcu_read_lock", linux___rcu_read_lock as usize, true);
    export_symbol_once("__rcu_read_unlock", linux___rcu_read_unlock as usize, true);
    export_symbol_once(
        "init_srcu_struct",
        srcu::linux_init_srcu_struct as usize,
        true,
    );
    export_symbol_once(
        "cleanup_srcu_struct",
        srcu::linux_cleanup_srcu_struct as usize,
        true,
    );
    export_symbol_once(
        "__srcu_read_lock",
        srcu::linux___srcu_read_lock as usize,
        true,
    );
    export_symbol_once(
        "__srcu_read_unlock",
        srcu::linux___srcu_read_unlock as usize,
        true,
    );
    export_symbol_once(
        "synchronize_srcu",
        srcu::linux_synchronize_srcu as usize,
        true,
    );
    export_symbol_once(
        "synchronize_srcu_expedited",
        srcu::linux_synchronize_srcu_expedited as usize,
        true,
    );
    export_symbol_once(
        "synchronize_rcu_expedited",
        linux_synchronize_rcu_expedited as usize,
        true,
    );
    export_symbol_once(
        "get_state_synchronize_rcu",
        linux_get_state_synchronize_rcu as usize,
        true,
    );
    export_symbol_once(
        "start_poll_synchronize_rcu",
        linux_start_poll_synchronize_rcu as usize,
        true,
    );
    export_symbol_once(
        "poll_state_synchronize_rcu",
        linux_poll_state_synchronize_rcu as usize,
        true,
    );
    export_symbol_once(
        "cond_synchronize_rcu",
        linux_cond_synchronize_rcu as usize,
        true,
    );
}

/// `__rcu_read_lock()` — `vendor/linux/kernel/rcu/tree_plugin.h:412`.
#[unsafe(export_name = "__rcu_read_lock")]
pub extern "C" fn linux___rcu_read_lock() {
    tree::rcu_read_lock();
}

/// `__rcu_read_unlock()` — `vendor/linux/kernel/rcu/tree_plugin.h:430`.
#[unsafe(export_name = "__rcu_read_unlock")]
pub extern "C" fn linux___rcu_read_unlock() {
    tree::rcu_read_unlock();
}

/// `synchronize_rcu_expedited()` — `vendor/linux/kernel/rcu/tree_exp.h:924`.
#[unsafe(export_name = "synchronize_rcu_expedited")]
pub extern "C" fn linux_synchronize_rcu_expedited() {
    let _ = update::synchronize_rcu_expedited();
}

/// `get_state_synchronize_rcu()` — `vendor/linux/kernel/rcu/tree.c:3439`.
#[unsafe(export_name = "get_state_synchronize_rcu")]
pub extern "C" fn linux_get_state_synchronize_rcu() -> usize {
    tree::get_state_synchronize_rcu() as usize
}

/// `start_poll_synchronize_rcu()` — `vendor/linux/kernel/rcu/tree.c:3519`.
#[unsafe(export_name = "start_poll_synchronize_rcu")]
pub extern "C" fn linux_start_poll_synchronize_rcu() -> usize {
    tree::start_poll_synchronize_rcu() as usize
}

/// `poll_state_synchronize_rcu()` — `vendor/linux/kernel/rcu/tree.c:3580`.
#[unsafe(export_name = "poll_state_synchronize_rcu")]
pub extern "C" fn linux_poll_state_synchronize_rcu(oldstate: usize) -> bool {
    tree::poll_state_synchronize_rcu(oldstate as u64)
}

/// `cond_synchronize_rcu()` — `vendor/linux/kernel/rcu/tree.c:3658`.
#[unsafe(export_name = "cond_synchronize_rcu")]
pub extern "C" fn linux_cond_synchronize_rcu(oldstate: usize) {
    tree::cond_synchronize_rcu(oldstate as u64);
}
