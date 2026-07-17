//! linux-parity: partial
//! linux-source: vendor/linux/drivers/base vendor/linux/block vendor/linux/kernel
//! Core Linux ABI exports needed by generic disk-controller modules.
//!
//! AHCI/SCSI/libata pull in a broader part of Linux's driver core than the
//! virtio smoke path did.  This module exports the generic kernel services
//! those vendor-built modules expect while keeping the actual hardware drivers
//! in the `.ko` payloads built from `vendor/linux`.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::{c_char, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::include::uapi::errno::{EINVAL, ENODEV, ENOENT, ENOMEM, ERANGE};
use crate::kernel::module::{export_symbol, find_symbol};
use crate::kernel::workqueue::{
    SYSTEM_HIGHPRI_WQ, SYSTEM_LONG_WQ, SYSTEM_UNBOUND_WQ, SYSTEM_WQ, WorkStruct, Workqueue,
    linux_flush_work, linux_queue_work_on,
};
use crate::lib::scatterlist::LinuxScatterList;
use crate::linux_driver_abi::base::{
    LinuxBusType, LinuxDevice, register_linux_bus_type, unregister_linux_bus_type,
};
use crate::linux_driver_abi::block::{
    BLK_STS_OK, LINUX_REQUEST_PDU_OFFSET, LINUX_STRUCT_DEVICE_SIZE, LinuxBlkMqTagSet,
    LinuxBlkStatus, LinuxGendisk, LinuxQueueLimits, LinuxRequest, LinuxRequestQueue,
    blk_execute_rq, blk_mq_end_request,
    linux_blk_mq_alloc_disk_for_queue as block_blk_mq_alloc_disk_for_queue,
    linux_blk_mq_alloc_queue as block_blk_mq_alloc_queue,
};
use crate::linux_driver_abi::pci::device::{linux_pci_config_read, linux_pci_config_write};

const PAGE_SIZE: usize = 4096;
const CLOCKS_PER_SEC: u64 = 100;
const SG_END: usize = crate::lib::scatterlist::SG_END;
const SG_PAGE_LINK_MASK: usize = crate::lib::scatterlist::SG_PAGE_LINK_MASK;
const LINUX_TIMER_LIST_FLAGS_OFFSET: usize = 0x20;
const LINUX_TIMER_LIST_SIZE: usize = 0x28;
const LINUX_WAIT_QUEUE_ENTRY_SIZE: usize = 0x28;
const LINUX_WAIT_QUEUE_ENTRY_LIST_OFFSET: usize = 0x18;
const LINUX_LIST_HEAD_PREV_OFFSET: usize = 0x8;
const LINUX_SCATTERLIST_SIZE: usize = 0x20;
const LINUX_SCATTERLIST_OFFSET_OFFSET: usize = 0x8;
const LINUX_SCATTERLIST_LENGTH_OFFSET: usize = 0xc;
const LINUX_SCATTERLIST_DMA_ADDRESS_OFFSET: usize = 0x10;
const LINUX_SCATTERLIST_DMA_LENGTH_OFFSET: usize = 0x18;
const LINUX_SG_TABLE_SIZE: usize = 0x10;
const LINUX_SG_TABLE_SGL_OFFSET: usize = 0x0;
const LINUX_SG_TABLE_NENTS_OFFSET: usize = 0x8;
const LINUX_SG_TABLE_ORIG_NENTS_OFFSET: usize = 0xc;
const LINUX_SG_PAGE_ITER_SIZE: usize = 0x18;
const LINUX_SCSI_DATA_BUFFER_SIZE: usize = 0x18;
const LINUX_SCSI_DATA_BUFFER_TABLE_OFFSET: usize = 0x0;
const LINUX_SCSI_DATA_BUFFER_LENGTH_OFFSET: usize = 0x10;
const LINUX_SBITMAP_SIZE: usize = 0x20;
const LINUX_SBITMAP_WORD_SIZE: usize = 0x10;
const LINUX_SCSI_INLINE_SG_CNT: usize = 2;
const LINUX_SCSI_INLINE_PROT_SG_CNT: usize = 1;
const LINUX_SCSI_CMND_SIZE: usize = 0x128;
const LINUX_SCSI_CMND_INLINE_SGL_OFFSET: usize = LINUX_SCSI_CMND_SIZE;
const LINUX_SCSI_INLINE_SGL_SIZE: usize = LINUX_SCATTERLIST_SIZE * LINUX_SCSI_INLINE_SG_CNT;
pub(crate) const LINUX_SCSI_AHCI_CMD_SIZE: usize =
    LINUX_SCSI_CMND_SIZE + LINUX_SCSI_INLINE_SGL_SIZE;
const LINUX_SCSI_CMND_DEVICE_OFFSET: usize = 0x0;
const LINUX_SCSI_CMND_EH_ENTRY_OFFSET: usize = 0x8;
const LINUX_SCSI_CMND_ABORT_WORK_OFFSET: usize = 0x18;
const LINUX_SCSI_CMND_RCU_OFFSET: usize = 0x70;
const LINUX_SCSI_CMND_EH_EFLAGS_OFFSET: usize = 0x80;
const LINUX_SCSI_CMND_BUDGET_TOKEN_OFFSET: usize = 0x84;
const LINUX_SCSI_CMND_JIFFIES_AT_ALLOC_OFFSET: usize = 0x88;
const LINUX_SCSI_CMND_RETRIES_OFFSET: usize = 0x90;
const LINUX_SCSI_CMND_ALLOWED_OFFSET: usize = 0x94;
const LINUX_SCSI_CMND_PROT_OP_OFFSET: usize = 0x98;
const LINUX_SCSI_CMND_PROT_TYPE_OFFSET: usize = 0x99;
const LINUX_SCSI_CMND_PROT_FLAGS_OFFSET: usize = 0x9a;
const LINUX_SCSI_CMND_SUBMITTER_OFFSET: usize = 0x9b;
const LINUX_SCSI_CMND_CMD_LEN_OFFSET: usize = 0x9c;
const LINUX_SCSI_CMND_SC_DATA_DIRECTION_OFFSET: usize = 0xa0;
const LINUX_SCSI_CMND_CMND_OFFSET: usize = 0xa4;
const LINUX_SCSI_CMND_SDB_OFFSET: usize = 0xc8;
const LINUX_SCSI_CMND_PROT_SDB_OFFSET: usize = 0xe0;
const LINUX_SCSI_CMND_UNDERFLOW_OFFSET: usize = 0xe8;
const LINUX_SCSI_CMND_TRANSFERSIZE_OFFSET: usize = 0xec;
const LINUX_SCSI_CMND_RESID_LEN_OFFSET: usize = 0xf0;
const LINUX_SCSI_CMND_SENSE_LEN_OFFSET: usize = 0xf4;
const LINUX_SCSI_CMND_SENSE_BUFFER_OFFSET: usize = 0xf8;
const LINUX_SCSI_CMND_FLAGS_OFFSET: usize = 0x100;
const LINUX_SCSI_CMND_STATE_OFFSET: usize = 0x108;
const LINUX_SCSI_CMND_EXTRA_LEN_OFFSET: usize = 0x110;
const LINUX_SCSI_CMND_HOST_SCRIBBLE_OFFSET: usize = 0x118;
const LINUX_SCSI_CMND_RESULT_OFFSET: usize = 0x120;
const LINUX_SCSI_DEVICE_SIZE: usize = 0x588;
const LINUX_SCSI_DEVICE_HOST_OFFSET: usize = 0x0;
const LINUX_SCSI_DEVICE_REQUEST_QUEUE_OFFSET: usize = 0x8;
const LINUX_SCSI_DEVICE_SIBLINGS_OFFSET: usize = 0x10;
const LINUX_SCSI_DEVICE_SAME_TARGET_SIBLINGS_OFFSET: usize = 0x20;
const LINUX_SCSI_DEVICE_BUDGET_MAP_OFFSET: usize = 0x30;
const LINUX_SCSI_DEVICE_DEVICE_BLOCKED_OFFSET: usize = 0x50;
const LINUX_SCSI_DEVICE_RESTARTS_OFFSET: usize = 0x54;
const LINUX_SCSI_DEVICE_STARVED_ENTRY_OFFSET: usize = 0x58;
const LINUX_SCSI_DEVICE_QUEUE_DEPTH_OFFSET: usize = 0x68;
const LINUX_SCSI_DEVICE_ID_OFFSET: usize = 0x88;
const LINUX_SCSI_DEVICE_CHANNEL_OFFSET: usize = 0x8c;
const LINUX_SCSI_DEVICE_LUN_OFFSET: usize = 0x90;
const LINUX_SCSI_DEVICE_SECTOR_SIZE_OFFSET: usize = 0x9c;
const LINUX_SCSI_DEVICE_HOSTDATA_OFFSET: usize = 0xa0;
const LINUX_SCSI_DEVICE_TYPE_OFFSET: usize = 0xa8;
const LINUX_SCSI_DEVICE_SDEV_TARGET_OFFSET: usize = 0x128;
const LINUX_SCSI_DEVICE_QUEUE_STOPPED_OFFSET: usize = 0x144;
const LINUX_SCSI_DEVICE_SDEV_GENDEV_OFFSET: usize = 0x1b0;
const LINUX_SCSI_DEVICE_SDEV_DEV_OFFSET: usize = 0x360;
const LINUX_SCSI_DEVICE_SDEV_STATE_OFFSET: usize = 0x578;
const LINUX_SCSI_DEVICE_SDEV_DATA_OFFSET: usize = 0x588;
const LINUX_SCSI_TARGET_SIZE: usize = 0x350;
const LINUX_SCSI_TARGET_STARGET_SDEV_USER_OFFSET: usize = 0x0;
const LINUX_SCSI_TARGET_SIBLINGS_OFFSET: usize = 0x8;
const LINUX_SCSI_TARGET_DEVICES_OFFSET: usize = 0x18;
const LINUX_SCSI_TARGET_DEV_OFFSET: usize = 0x28;
const LINUX_SCSI_TARGET_REAP_REF_OFFSET: usize = 0x320;
const LINUX_SCSI_TARGET_CHANNEL_OFFSET: usize = 0x324;
const LINUX_SCSI_TARGET_ID_OFFSET: usize = 0x328;
const LINUX_SCSI_TARGET_BITFLAGS_OFFSET: usize = 0x32c;
const LINUX_SCSI_TARGET_TARGET_BUSY_OFFSET: usize = 0x330;
const LINUX_SCSI_TARGET_TARGET_BLOCKED_OFFSET: usize = 0x334;
const LINUX_SCSI_TARGET_CAN_QUEUE_OFFSET: usize = 0x338;
const LINUX_SCSI_TARGET_MAX_TARGET_BLOCKED_OFFSET: usize = 0x33c;
const LINUX_SCSI_TARGET_SCSI_LEVEL_OFFSET: usize = 0x340;
const LINUX_SCSI_TARGET_STATE_OFFSET: usize = 0x344;
const LINUX_SCSI_TARGET_HOSTDATA_OFFSET: usize = 0x348;
const LINUX_SCSI_TARGET_STARGET_DATA_OFFSET: usize = 0x350;
const LINUX_SCSI_HOST_TEMPLATE_SIZE: usize = 0x160;
const LINUX_SCSI_HOST_TEMPLATE_CMD_SIZE_OFFSET: usize = 0x0;
const LINUX_SCSI_HOST_TEMPLATE_QUEUECOMMAND_OFFSET: usize = 0x8;
const LINUX_SCSI_HOST_TEMPLATE_QUEUE_RESERVED_COMMAND_OFFSET: usize = 0x10;
const LINUX_SCSI_HOST_TEMPLATE_COMMIT_RQS_OFFSET: usize = 0x18;
const LINUX_SCSI_HOST_TEMPLATE_MODULE_OFFSET: usize = 0x20;
const LINUX_SCSI_HOST_TEMPLATE_NAME_OFFSET: usize = 0x28;
const LINUX_SCSI_HOST_TEMPLATE_INFO_OFFSET: usize = 0x30;
const LINUX_SCSI_HOST_TEMPLATE_IOCTL_OFFSET: usize = 0x38;
const LINUX_SCSI_HOST_TEMPLATE_INIT_CMD_PRIV_OFFSET: usize = 0x40;
const LINUX_SCSI_HOST_TEMPLATE_EXIT_CMD_PRIV_OFFSET: usize = 0x48;
const LINUX_SCSI_HOST_TEMPLATE_EH_ABORT_HANDLER_OFFSET: usize = 0x50;
const LINUX_SCSI_HOST_TEMPLATE_EH_DEVICE_RESET_HANDLER_OFFSET: usize = 0x58;
const LINUX_SCSI_HOST_TEMPLATE_EH_TARGET_RESET_HANDLER_OFFSET: usize = 0x60;
const LINUX_SCSI_HOST_TEMPLATE_EH_BUS_RESET_HANDLER_OFFSET: usize = 0x68;
const LINUX_SCSI_HOST_TEMPLATE_EH_HOST_RESET_HANDLER_OFFSET: usize = 0x70;
const LINUX_SCSI_HOST_TEMPLATE_SDEV_INIT_OFFSET: usize = 0x78;
const LINUX_SCSI_HOST_TEMPLATE_SDEV_CONFIGURE_OFFSET: usize = 0x80;
const LINUX_SCSI_HOST_TEMPLATE_SDEV_DESTROY_OFFSET: usize = 0x88;
const LINUX_SCSI_HOST_TEMPLATE_TARGET_ALLOC_OFFSET: usize = 0x90;
const LINUX_SCSI_HOST_TEMPLATE_TARGET_DESTROY_OFFSET: usize = 0x98;
const LINUX_SCSI_HOST_TEMPLATE_SCAN_FINISHED_OFFSET: usize = 0xa0;
const LINUX_SCSI_HOST_TEMPLATE_SCAN_START_OFFSET: usize = 0xa8;
const LINUX_SCSI_HOST_TEMPLATE_CHANGE_QUEUE_DEPTH_OFFSET: usize = 0xb0;
const LINUX_SCSI_HOST_TEMPLATE_MAP_QUEUES_OFFSET: usize = 0xb8;
const LINUX_SCSI_HOST_TEMPLATE_MQ_POLL_OFFSET: usize = 0xc0;
const LINUX_SCSI_HOST_TEMPLATE_DMA_NEED_DRAIN_OFFSET: usize = 0xc8;
const LINUX_SCSI_HOST_TEMPLATE_EH_TIMED_OUT_OFFSET: usize = 0xf0;
const LINUX_SCSI_HOST_TEMPLATE_EH_SHOULD_RETRY_CMD_OFFSET: usize = 0xf8;
const LINUX_SCSI_HOST_TEMPLATE_HOST_RESET_OFFSET: usize = 0x100;
const LINUX_SCSI_HOST_TEMPLATE_PROC_NAME_OFFSET: usize = 0x108;
const LINUX_SCSI_HOST_TEMPLATE_CAN_QUEUE_OFFSET: usize = 0x110;
const LINUX_SCSI_HOST_TEMPLATE_NR_RESERVED_CMDS_OFFSET: usize = 0x114;
const LINUX_SCSI_HOST_TEMPLATE_THIS_ID_OFFSET: usize = 0x118;
const LINUX_SCSI_HOST_TEMPLATE_SG_TABLESIZE_OFFSET: usize = 0x11c;
const LINUX_SCSI_HOST_TEMPLATE_SG_PROT_TABLESIZE_OFFSET: usize = 0x11e;
const LINUX_SCSI_HOST_TEMPLATE_MAX_SECTORS_OFFSET: usize = 0x120;
const LINUX_SCSI_HOST_TEMPLATE_MAX_SEGMENT_SIZE_OFFSET: usize = 0x124;
const LINUX_SCSI_HOST_TEMPLATE_DMA_ALIGNMENT_OFFSET: usize = 0x128;
const LINUX_SCSI_HOST_TEMPLATE_DMA_BOUNDARY_OFFSET: usize = 0x130;
const LINUX_SCSI_HOST_TEMPLATE_VIRT_BOUNDARY_MASK_OFFSET: usize = 0x138;
const LINUX_SCSI_HOST_TEMPLATE_CMD_PER_LUN_OFFSET: usize = 0x140;
const LINUX_SCSI_HOST_TEMPLATE_BITFLAGS_OFFSET: usize = 0x142;
const LINUX_SCSI_HOST_TEMPLATE_MAX_HOST_BLOCKED_OFFSET: usize = 0x144;
const LINUX_SCSI_HOST_TEMPLATE_SHOST_GROUPS_OFFSET: usize = 0x148;
const LINUX_SCSI_HOST_TEMPLATE_SDEV_GROUPS_OFFSET: usize = 0x150;
const LINUX_SCSI_HOST_TEMPLATE_VENDOR_ID_OFFSET: usize = 0x158;
const LINUX_SCSI_HOST_SIZE: usize = 0x868;
const LINUX_SCSI_HOST_DEVICES_OFFSET: usize = 0x0;
const LINUX_SCSI_HOST_HOST_LOCK_OFFSET: usize = 0x30;
const LINUX_SCSI_HOST_EH_ABORT_LIST_OFFSET: usize = 0x48;
const LINUX_SCSI_HOST_EHANDLER_OFFSET: usize = 0x68;
const LINUX_SCSI_HOST_EH_ACTION_OFFSET: usize = 0x70;
const LINUX_SCSI_HOST_HOST_WAIT_OFFSET: usize = 0x78;
const LINUX_SCSI_HOST_HOSTT_OFFSET: usize = 0x88;
const LINUX_SCSI_HOST_TAG_SET_OFFSET: usize = 0xb8;
const LINUX_SCSI_HOST_HOST_BLOCKED_OFFSET: usize = 0x198;
const LINUX_SCSI_HOST_HOST_FAILED_OFFSET: usize = 0x19c;
const LINUX_SCSI_HOST_HOST_EH_SCHEDULED_OFFSET: usize = 0x1a0;
const LINUX_SCSI_HOST_HOST_NO_OFFSET: usize = 0x1a4;
const LINUX_SCSI_HOST_MAX_CMD_LEN_OFFSET: usize = 0x1cc;
const LINUX_SCSI_HOST_CAN_QUEUE_OFFSET: usize = 0x1d4;
const LINUX_SCSI_HOST_NR_RESERVED_CMDS_OFFSET: usize = 0x1d8;
const LINUX_SCSI_HOST_CMD_PER_LUN_OFFSET: usize = 0x1dc;
const LINUX_SCSI_HOST_SG_TABLESIZE_OFFSET: usize = 0x1de;
const LINUX_SCSI_HOST_NR_HW_QUEUES_OFFSET: usize = 0x208;
const LINUX_SCSI_HOST_NR_MAPS_OFFSET: usize = 0x20c;
const LINUX_SCSI_HOST_WORK_Q_OFFSET: usize = 0x218;
const LINUX_SCSI_HOST_SHOST_STATE_OFFSET: usize = 0x250;
const LINUX_SCSI_HOST_SHOST_GENDEV_OFFSET: usize = 0x258;
const LINUX_SCSI_HOST_SHOST_DEV_OFFSET: usize = 0x550;
const LINUX_SCSI_HOST_PSEUDO_SDEV_OFFSET: usize = 0x848;
const LINUX_SCSI_HOST_SHOST_DATA_OFFSET: usize = 0x850;
const LINUX_SCSI_HOST_DMA_DEV_OFFSET: usize = 0x858;
const LINUX_SCSI_HOST_HOSTDATA_OFFSET: usize = 0x868;
const WORKQUEUE_DRAIN_SPINS: usize = 1024;

fn export_symbol_once(name: &'static str, addr: usize, gpl_only: bool) {
    if find_symbol(name).is_none() {
        export_symbol(name, addr, gpl_only);
    }
}

macro_rules! export_fn {
    ($name:literal, $func:path, $gpl:expr) => {
        export_symbol_once($name, $func as usize, $gpl)
    };
}

macro_rules! export_data {
    ($name:literal, $data:expr, $gpl:expr) => {
        export_symbol_once($name, $data as usize, $gpl)
    };
}

pub fn register_module_exports() {
    init_system_workqueue_exports();
    crate::linux_driver_abi::register_driver_abi_poller("ahci", poll_ahci_interrupts);

    export_data!("param_ops_bool", ptr::addr_of!(LINUX_PARAM_OPS_BOOL), false);
    export_data!("param_ops_int", ptr::addr_of!(LINUX_PARAM_OPS_INT), false);
    export_data!(
        "param_array_ops",
        ptr::addr_of!(LINUX_PARAM_ARRAY_OPS),
        false
    );
    export_data!(
        "param_ops_string",
        ptr::addr_of!(LINUX_PARAM_OPS_STRING),
        false
    );
    export_data!(
        "param_ops_ullong",
        ptr::addr_of!(LINUX_PARAM_OPS_ULLONG),
        false
    );
    export_data!(
        "param_ops_charp",
        ptr::addr_of!(LINUX_PARAM_OPS_CHARP),
        false
    );
    export_data!("system_state", ptr::addr_of!(LINUX_SYSTEM_STATE), false);
    export_data!("platform_bus", ptr::addr_of!(LINUX_PLATFORM_BUS), false);
    export_data!(
        "pci_bus_type",
        crate::linux_driver_abi::pci::driver::linux_pci_bus_type_ptr(),
        false
    );
    export_data!("system_wq", ptr::addr_of!(LINUX_SYSTEM_WQ), false);
    export_data!("system_long_wq", ptr::addr_of!(LINUX_SYSTEM_LONG_WQ), false);
    // `system_dfl_long_wq` is the renamed long-running-work queue introduced in
    // newer kernels (vendor/linux/kernel/workqueue.c — "similar to system_dfl_wq
    // but it may host long running works"); libata references it. Back it with
    // the same workqueue as `system_long_wq`. Without this export, libata.ko
    // fails to load (unresolved `system_dfl_long_wq`), which cascades into
    // ahci.ko failing on libata's `ata_pci_shutdown_one` → no root disk → panic.
    export_data!(
        "system_dfl_long_wq",
        ptr::addr_of!(LINUX_SYSTEM_LONG_WQ),
        false
    );
    export_data!(
        "system_unbound_wq",
        ptr::addr_of!(LINUX_SYSTEM_UNBOUND_WQ),
        false
    );
    export_data!(
        "system_highpri_wq",
        ptr::addr_of!(LINUX_SYSTEM_HIGHPRI_WQ),
        false
    );
    export_data!("system_percpu_wq", ptr::addr_of!(LINUX_SYSTEM_WQ), false);

    export_fn!("bus_register", linux_bus_register, false);
    export_fn!("bus_unregister", linux_bus_unregister, false);
    export_fn!("class_register", linux_class_register, false);
    export_fn!("class_unregister", linux_class_unregister, false);
    export_fn!("class_find_device", linux_class_find_device, false);
    export_fn!(
        "class_interface_register",
        linux_class_interface_register,
        false
    );
    export_fn!(
        "class_interface_unregister",
        linux_class_interface_unregister,
        false
    );
    export_fn!("dev_set_name", linux_dev_set_name, false);
    export_fn!("dev_driver_string", linux_dev_driver_string, false);
    export_fn!("device_create", linux_device_create, true);
    export_fn!("device_destroy", linux_device_destroy, true);
    export_fn!("device_del", linux_device_del, false);
    export_fn!("device_create_file", linux_device_create_file, true);
    export_fn!("device_remove_file", linux_device_remove_file, false);
    export_fn!(
        "device_create_bin_file",
        linux_device_create_bin_file,
        false
    );
    export_fn!(
        "device_remove_bin_file",
        linux_device_remove_bin_file,
        false
    );
    export_fn!("device_for_each_child", linux_device_for_each_child, false);
    export_fn!("device_link_add", linux_device_link_add, false);
    export_fn!("device_link_del", linux_device_link_del, false);
    export_fn!("device_link_remove", linux_device_link_remove, false);
    export_fn!("kobject_uevent_env", linux_kobject_uevent_env, false);
    export_fn!("add_uevent_var", linux_add_uevent_var, false);

    export_fn!("__kmalloc_large_noprof", linux___kmalloc_large_noprof, true);
    export_fn!(
        "__kmem_cache_create_args",
        linux___kmem_cache_create_args,
        true
    );
    export_fn!(
        "kmem_cache_alloc_node_noprof",
        linux_kmem_cache_alloc_node_noprof,
        true
    );
    export_fn!("kmem_cache_destroy", linux_kmem_cache_destroy, true);
    export_fn!("kmem_cache_free", linux_kmem_cache_free, true);
    export_fn!("kmemdup_noprof", linux_kmemdup_noprof, true);
    export_fn!("kvfree", linux_kvfree, true);
    export_fn!("kvfree_call_rcu", linux_kvfree_call_rcu, true);
    export_fn!("__folio_put", linux___folio_put, true);
    export_fn!("free_percpu", linux_free_percpu, true);
    export_fn!(
        "__clear_pages_unrolled",
        linux___clear_pages_unrolled,
        false
    );
    export_fn!("__get_user_1", linux___get_user_1, false);
    export_fn!("__get_user_4", linux___get_user_4, false);
    export_fn!("__put_user_4", linux___put_user_4, false);
    export_fn!("__put_user_8", linux___put_user_8, false);

    export_fn!(
        "mempool_create_node_noprof",
        crate::mm::mempool::mempool_create_node_noprof,
        true
    );
    export_fn!(
        "mempool_alloc_noprof",
        crate::mm::mempool::mempool_alloc_noprof,
        true
    );
    export_fn!(
        "mempool_alloc_pages",
        crate::mm::mempool::mempool_alloc_pages,
        true
    );
    export_fn!(
        "mempool_alloc_slab",
        crate::mm::mempool::mempool_alloc_slab,
        true
    );
    export_fn!("mempool_free", crate::mm::mempool::mempool_free, true);
    export_fn!(
        "mempool_free_slab",
        crate::mm::mempool::mempool_free_slab,
        true
    );
    export_fn!(
        "mempool_free_pages",
        crate::mm::mempool::mempool_free_pages,
        true
    );
    export_fn!("mempool_destroy", crate::mm::mempool::mempool_destroy, true);

    export_fn!("__msecs_to_jiffies", linux___msecs_to_jiffies, false);
    export_fn!("clock_t_to_jiffies", linux_clock_t_to_jiffies, false);
    export_fn!("jiffies_to_clock_t", linux_jiffies_to_clock_t, false);
    export_fn!(
        "round_jiffies_relative",
        linux_round_jiffies_relative,
        false
    );
    export_fn!("__const_udelay", linux___const_udelay, false);
    export_fn!("__udelay", linux___udelay, false);
    export_fn!("timer_init_key", linux_timer_init_key, false);
    export_fn!("add_timer", linux_add_timer, false);
    export_fn!("mod_timer", linux_mod_timer, false);
    export_fn!("timer_delete_sync", linux_timer_delete_sync, false);
    export_fn!(
        "schedule_timeout_uninterruptible",
        linux_schedule_timeout_uninterruptible,
        false
    );
    export_fn!("usleep_range_state", linux_usleep_range_state, false);

    export_fn!("__init_waitqueue_head", linux___init_waitqueue_head, false);
    export_fn!("init_wait_entry", linux_init_wait_entry, false);
    export_fn!("add_wait_queue", linux_add_wait_queue, false);
    export_fn!("remove_wait_queue", linux_remove_wait_queue, false);
    export_fn!("prepare_to_wait", linux_prepare_to_wait, false);
    export_fn!("prepare_to_wait_event", linux_prepare_to_wait_event, false);
    export_fn!("finish_wait", linux_finish_wait, false);
    export_fn!("__wake_up", linux___wake_up, false);
    export_fn!(
        "autoremove_wake_function",
        linux_autoremove_wake_function,
        false
    );
    export_fn!("default_wake_function", linux_default_wake_function, false);
    export_fn!(
        "wait_for_completion_timeout",
        linux_wait_for_completion_timeout,
        false
    );
    export_fn!("schedule", linux_schedule, false);
    export_fn!("wake_up_process", linux_wake_up_process, false);
    export_fn!("lupos_ata_diag", linux_lupos_ata_diag, false);

    export_fn!("queue_delayed_work_on", linux_queue_delayed_work_on, true);
    export_fn!("__flush_workqueue", linux___flush_workqueue, true);
    export_fn!("cancel_work", linux_cancel_work, true);
    export_fn!("cancel_work_sync", linux_cancel_work_sync, true);
    export_fn!(
        "cancel_delayed_work_sync",
        linux_cancel_delayed_work_sync,
        true
    );
    export_fn!("delayed_work_timer_fn", linux_delayed_work_timer_fn, true);

    export_fn!("synchronize_rcu", linux_synchronize_rcu, false);
    export_fn!("rcu_barrier", linux_rcu_barrier, false);
    export_fn!("call_rcu", linux_call_rcu, false);

    export_fn!(
        "kthread_create_on_node",
        linux_kthread_create_on_node,
        false
    );
    export_fn!("kthread_should_stop", linux_kthread_should_stop, false);
    export_fn!("kthread_stop", linux_kthread_stop, false);

    export_fn!("dma_map_sg_attrs", linux_dma_map_sg_attrs, true);
    export_fn!("dma_unmap_sg_attrs", linux_dma_unmap_sg_attrs, true);
    export_fn!("dma_max_mapping_size", linux_dma_max_mapping_size, true);
    export_fn!("dmam_alloc_attrs", linux_dmam_alloc_attrs, true);

    export_fn!("sg_copy_from_buffer", linux_sg_copy_from_buffer, true);
    export_fn!("sg_copy_to_buffer", linux_sg_copy_to_buffer, true);
    export_fn!("sg_miter_start", linux_sg_miter_start, true);
    export_fn!("sg_miter_skip", linux_sg_miter_skip, false);
    export_fn!("sg_miter_next", linux_sg_miter_next, true);
    export_fn!("sg_miter_stop", linux_sg_miter_stop, true);

    export_fn!("__blk_mq_end_request", linux___blk_mq_end_request, true);
    export_fn!("blk_mq_alloc_queue", linux_blk_mq_alloc_queue, true);
    export_fn!(
        "blk_mq_alloc_disk_for_queue",
        linux_blk_mq_alloc_disk_for_queue,
        true
    );
    export_fn!("blk_mq_destroy_queue", linux_blk_mq_destroy_queue, true);
    export_fn!("blk_mq_run_hw_queues", linux_blk_mq_run_hw_queues, true);
    export_fn!(
        "blk_mq_delay_run_hw_queues",
        linux_blk_mq_delay_run_hw_queues,
        true
    );
    export_fn!(
        "blk_mq_kick_requeue_list",
        linux_blk_mq_kick_requeue_list,
        true
    );
    export_fn!(
        "blk_mq_delay_kick_requeue_list",
        linux_blk_mq_delay_kick_requeue_list,
        true
    );
    export_fn!(
        "blk_mq_tagset_busy_iter",
        linux_blk_mq_tagset_busy_iter,
        true
    );
    export_fn!(
        "blk_mq_wait_quiesce_done",
        linux_blk_mq_wait_quiesce_done,
        true
    );
    export_fn!("blk_get_queue", linux_blk_get_queue, true);
    export_fn!("blk_put_queue", linux_blk_put_queue, true);
    export_fn!("blk_queue_rq_timeout", linux_blk_queue_rq_timeout, true);
    export_fn!("blk_set_queue_depth", linux_blk_set_queue_depth, true);
    export_fn!("blk_clear_pm_only", linux_blk_clear_pm_only, true);
    export_fn!("blk_set_pm_only", linux_blk_set_pm_only, true);
    export_fn!("blk_update_request", linux_blk_update_request, true);
    export_fn!("blk_abort_request", linux_blk_abort_request, true);
    export_fn!("blk_rq_init", linux_blk_rq_init, true);
    export_fn!("blk_rq_map_user", linux_blk_rq_map_user, true);
    export_fn!("blk_rq_map_user_iov", linux_blk_rq_map_user_iov, true);
    export_fn!("blk_rq_map_user_io", linux_blk_rq_map_user_io, true);
    export_fn!("blk_rq_unmap_user", linux_blk_rq_unmap_user, true);
    export_fn!("blk_execute_rq_nowait", linux_blk_execute_rq_nowait, true);
    export_fn!(
        "queue_limits_commit_update",
        linux_queue_limits_commit_update,
        true
    );
    export_fn!("add_disk_randomness", linux_add_disk_randomness, true);
    export_fn!(
        "disk_alloc_independent_access_ranges",
        linux_disk_alloc_independent_access_ranges,
        true
    );
    export_fn!(
        "disk_set_independent_access_ranges",
        linux_disk_set_independent_access_ranges,
        true
    );
    export_fn!(
        "disk_check_media_change",
        linux_disk_check_media_change,
        true
    );
    // `bsg_register_queue()` returns `struct bsg_device *` and owns the
    // cdev/device/sysfs lifecycle in vendor Linux. The previous local no-op
    // used an incompatible integer return ABI. These symbols are supplied by
    // the staged vendor `block/bsg.ko`; do not mask a missing dependency with
    // a fabricated built-in provider.
    export_fn!("kblockd_schedule_work", linux_kblockd_schedule_work, true);

    export_fn!("pcim_enable_device", linux_pcim_enable_device, false);
    export_fn!("pcim_pin_device", linux_pcim_pin_device, false);
    export_fn!("pcim_intx", linux_pcim_intx, true);
    export_fn!("pcim_iomap", linux_pcim_iomap, false);
    export_fn!("pcim_iomap_region", linux_pcim_iomap_region, false);
    export_fn!("pcim_iomap_regions", linux_pcim_iomap_regions, false);
    export_fn!("pcim_iomap_table", linux_pcim_iomap_table, false);
    export_fn!(
        "pcim_request_all_regions",
        linux_pcim_request_all_regions,
        false
    );
    export_fn!("pci_write_config_byte", linux_pci_write_config_byte, false);
    export_fn!("pci_write_config_word", linux_pci_write_config_word, false);
    export_fn!(
        "pci_write_config_dword",
        linux_pci_write_config_dword,
        false
    );

    export_fn!("transport_setup_device", linux_transport_setup_device, true);
    export_fn!("transport_add_device", linux_transport_add_device, true);
    export_fn!(
        "transport_configure_device",
        linux_transport_configure_device,
        true
    );
    export_fn!(
        "transport_remove_device",
        linux_transport_remove_device,
        true
    );
    export_fn!(
        "transport_destroy_device",
        linux_transport_destroy_device,
        true
    );
    export_fn!(
        "transport_class_register",
        linux_transport_class_register,
        true
    );
    export_fn!(
        "transport_class_unregister",
        linux_transport_class_unregister,
        true
    );
    export_fn!(
        "attribute_container_register",
        linux_attribute_container_register,
        true
    );
    export_fn!(
        "attribute_container_unregister",
        linux_attribute_container_unregister,
        true
    );
    export_fn!(
        "anon_transport_class_register",
        linux_anon_transport_class_register,
        true
    );
    export_fn!(
        "anon_transport_class_unregister",
        linux_anon_transport_class_unregister,
        true
    );

    export_fn!("async_schedule_node", linux_async_schedule_node, false);
    export_fn!(
        "async_synchronize_cookie",
        linux_async_synchronize_cookie,
        false
    );

    export_fn!("kstrtoull", linux_kstrtoull, false);
    export_fn!("kstrtouint", linux_kstrtouint, false);
    export_fn!("kstrtou16", linux_kstrtou16, false);
    export_fn!("kstrtou8", linux_kstrtou8, false);
    export_fn!("kstrtoint", linux_kstrtoint, false);
    export_fn!("kstrtobool", linux_kstrtobool, false);
    export_fn!("kstrdup", linux_kstrdup, false);
    export_fn!("simple_strtol", linux_simple_strtol, false);
    export_fn!("simple_strtoul", linux_simple_strtoul, false);
    export_fn!("simple_strtoull", linux_simple_strtoull, false);
    export_fn!("sysfs_streq", linux_sysfs_streq, false);
    export_fn!("strim", linux_strim, false);
    export_fn!(
        "memory_read_from_buffer",
        linux_memory_read_from_buffer,
        false
    );
    export_fn!("hex_dump_to_buffer", linux_hex_dump_to_buffer, false);
    export_fn!("memcpy_and_pad", linux_memcpy_and_pad, false);
    export_fn!("scnprintf", linux_scnprintf, false);
    export_fn!("vscnprintf", linux_vscnprintf, false);
    export_fn!("sscanf", linux_sscanf, false);
    export_fn!("read_cache_folio", linux_read_cache_folio, false);
    export_fn!(
        "refcount_warn_saturate",
        linux_refcount_warn_saturate,
        false
    );

    export_fn!("ida_destroy", linux_ida_destroy, true);
    export_fn!("sbitmap_init_node", linux_sbitmap_init_node, true);
    export_fn!("sbitmap_resize", linux_sbitmap_resize, true);
    export_fn!("sbitmap_get", linux_sbitmap_get, true);
    export_fn!("sbitmap_weight", linux_sbitmap_weight, true);

    export_fn!("__devres_alloc_node", linux___devres_alloc_node, false);
    export_fn!("devres_add", linux_devres_add, false);
    export_fn!("devres_find", linux_devres_find, true);
    export_fn!("devres_remove", linux_devres_remove, true);
    export_fn!("devres_destroy", linux_devres_destroy, true);
    export_fn!("devres_release", linux_devres_release, true);
    export_fn!("devres_free", linux_devres_free, false);
    export_fn!("devres_open_group", linux_devres_open_group, false);
    export_fn!("devres_release_group", linux_devres_release_group, false);
    export_fn!("devres_remove_group", linux_devres_remove_group, false);
    export_fn!("__devm_add_action", linux___devm_add_action, true);
    export_fn!("devm_release_action", linux_devm_release_action, true);
    export_fn!(
        "devm_remove_action_nowarn",
        linux_devm_remove_action_nowarn,
        true
    );
    export_fn!("devm_kmalloc", linux_devm_kmalloc, false);
    export_fn!("devm_kfree", linux_devm_kfree, false);
    export_fn!("devm_kasprintf", linux_devm_kasprintf, false);
    export_fn!(
        "devm_request_threaded_irq",
        linux_devm_request_threaded_irq,
        false
    );
    export_fn!("devm_free_irq", linux_devm_free_irq, false);
}

static LINUX_PARAM_OPS_BOOL: usize = 0;
static LINUX_PARAM_OPS_INT: usize = 0;
static LINUX_PARAM_ARRAY_OPS: usize = 0;
static LINUX_PARAM_OPS_STRING: usize = 0;
static LINUX_PARAM_OPS_ULLONG: usize = 0;
static LINUX_PARAM_OPS_CHARP: usize = 0;
static LINUX_SYSTEM_STATE: i32 = 0;
static LINUX_PLATFORM_BUS: usize = 0;
static LINUX_SYSTEM_WQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_SYSTEM_LONG_WQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_SYSTEM_UNBOUND_WQ: AtomicUsize = AtomicUsize::new(0);
static LINUX_SYSTEM_HIGHPRI_WQ: AtomicUsize = AtomicUsize::new(0);

fn init_system_wq_slot(slot: &AtomicUsize, queue: Arc<Workqueue>) {
    if slot.load(Ordering::Acquire) == 0 {
        let ptr = Arc::into_raw(queue) as usize;
        match slot.compare_exchange(0, ptr, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {}
            Err(_) => unsafe {
                let _ = Arc::from_raw(ptr as *const Workqueue);
            },
        }
    }
}

fn init_system_workqueue_exports() {
    init_system_wq_slot(&LINUX_SYSTEM_WQ, SYSTEM_WQ.get());
    init_system_wq_slot(&LINUX_SYSTEM_LONG_WQ, SYSTEM_LONG_WQ.get());
    init_system_wq_slot(&LINUX_SYSTEM_UNBOUND_WQ, SYSTEM_UNBOUND_WQ.get());
    init_system_wq_slot(&LINUX_SYSTEM_HIGHPRI_WQ, SYSTEM_HIGHPRI_WQ.get());
}

unsafe fn c_str_len(ptr: *const c_char, max: usize) -> Option<usize> {
    if ptr.is_null() {
        return None;
    }
    let mut len = 0usize;
    while len < max {
        if unsafe { *ptr.add(len) } == 0 {
            return Some(len);
        }
        len += 1;
    }
    None
}

unsafe fn c_str_bytes<'a>(ptr: *const c_char, max: usize) -> Option<&'a [u8]> {
    let len = unsafe { c_str_len(ptr, max)? };
    Some(unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) })
}

unsafe fn c_str_eq(ptr: *const c_char, bytes: &[u8]) -> bool {
    (unsafe { c_str_bytes(ptr, 512) }) == Some(bytes)
}

fn linux_error_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as usize as *mut c_void
}

unsafe extern "C" fn linux_bus_register(bus: *const LinuxBusType) -> i32 {
    if bus.is_null() {
        return -EINVAL;
    }
    register_linux_bus_type(bus);
    0
}

unsafe extern "C" fn linux_bus_unregister(bus: *const LinuxBusType) {
    if !bus.is_null() {
        unregister_linux_bus_type(bus);
    }
}

#[repr(C)]
struct LinuxClass {
    name: *const c_char,
}

#[derive(Clone, Copy)]
struct LinuxCreatedDevice {
    class: usize,
    devt: u32,
    dev: usize,
}

fn err_ptr(errno: i32) -> *mut c_void {
    (-(errno as isize)) as *mut c_void
}

unsafe extern "C" fn linux_class_register(class: *const LinuxClass) -> i32 {
    if class.is_null() {
        return -EINVAL;
    }
    0
}

unsafe extern "C" fn linux_class_unregister(_class: *const LinuxClass) {}

unsafe extern "C" fn linux_class_find_device(
    _class: *const LinuxClass,
    _start: *mut LinuxDevice,
    _data: *const c_void,
    _match: *const c_void,
) -> *mut LinuxDevice {
    ptr::null_mut()
}

unsafe extern "C" fn linux_class_interface_register(_class_intf: *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_class_interface_unregister(_class_intf: *mut c_void) {}

unsafe extern "C" fn linux_device_create(
    class: *const LinuxClass,
    parent: *mut LinuxDevice,
    devt: u32,
    drvdata: *mut c_void,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> *mut LinuxDevice {
    if class.is_null() || fmt.is_null() {
        return err_ptr(ENODEV).cast();
    }

    let mut name = [0u8; 64];
    let written = unsafe {
        format_args(
            name.as_mut_ptr(),
            name.len(),
            fmt,
            &[arg0, arg1, arg2, arg3],
        )
    };
    if written <= 0 {
        return err_ptr(EINVAL).cast();
    }

    let dev = Box::into_raw(Box::new(unsafe { core::mem::zeroed::<LinuxDevice>() }));
    unsafe {
        (*dev).parent = parent;
        (*dev).driver_data = drvdata;
    }
    if unsafe {
        crate::linux_driver_abi::base::linux_device_set_name_bytes(dev, &name[..written as usize])
    }
    .is_err()
    {
        unsafe {
            let _ = Box::from_raw(dev);
        }
        return err_ptr(EINVAL).cast();
    }

    let ret = unsafe { crate::linux_driver_abi::base::linux_device_register(dev) };
    if ret != 0 {
        unsafe {
            let _ = Box::from_raw(dev);
        }
        return err_ptr(-ret).cast();
    }

    LINUX_CREATED_DEVICES.lock().push(LinuxCreatedDevice {
        class: class as usize,
        devt,
        dev: dev as usize,
    });
    dev
}

unsafe extern "C" fn linux_device_destroy(class: *const LinuxClass, devt: u32) {
    let mut devices = LINUX_CREATED_DEVICES.lock();
    let Some(pos) = devices
        .iter()
        .position(|entry| entry.class == class as usize && entry.devt == devt)
    else {
        return;
    };
    let dev = devices.swap_remove(pos).dev as *mut LinuxDevice;
    drop(devices);

    unsafe {
        crate::linux_driver_abi::base::linux_device_unregister(dev);
        let _ = Box::from_raw(dev);
    }
}

unsafe extern "C" fn linux_dev_set_name(
    dev: *mut LinuxDevice,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> i32 {
    if dev.is_null() || fmt.is_null() {
        return -EINVAL;
    }
    let mut bytes = [0u8; 64];
    let args = [arg0, arg1, arg2, arg3];
    let len = unsafe { format_args(bytes.as_mut_ptr(), bytes.len(), fmt, &args) };
    if len <= 0 {
        return -EINVAL;
    }
    match unsafe {
        crate::linux_driver_abi::base::linux_device_set_name_bytes(dev, &bytes[..len as usize])
    } {
        Ok(()) => 0,
        Err(errno) => -errno,
    }
}

unsafe extern "C" fn linux_dev_driver_string(dev: *const LinuxDevice) -> *const c_char {
    static EMPTY: &[u8; 1] = b"\0";
    if dev.is_null() {
        return EMPTY.as_ptr().cast::<c_char>();
    }
    let driver = unsafe { (*dev).driver };
    if driver.is_null() || unsafe { (*driver).name.is_null() } {
        EMPTY.as_ptr().cast::<c_char>()
    } else {
        unsafe { (*driver).name }
    }
}

unsafe extern "C" fn linux_device_del(dev: *mut LinuxDevice) {
    if !dev.is_null() {
        unsafe { crate::linux_driver_abi::base::linux_device_unregister(dev) };
    }
}

unsafe extern "C" fn linux_device_create_file(_dev: *mut LinuxDevice, _attr: *const c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_device_remove_file(_dev: *mut LinuxDevice, _attr: *const c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_device_create_bin_file(
    _dev: *mut LinuxDevice,
    _attr: *const c_void,
) -> i32 {
    0
}

unsafe extern "C" fn linux_device_remove_bin_file(_dev: *mut LinuxDevice, _attr: *const c_void) {}

type DeviceChildCallback = unsafe extern "C" fn(*mut LinuxDevice, *mut c_void) -> i32;

unsafe extern "C" fn linux_device_for_each_child(
    _parent: *mut LinuxDevice,
    _data: *mut c_void,
    _fn: Option<DeviceChildCallback>,
) -> i32 {
    0
}

unsafe extern "C" fn linux_device_link_add(
    _consumer: *mut LinuxDevice,
    _supplier: *mut LinuxDevice,
    _flags: u32,
) -> *mut c_void {
    core::ptr::dangling_mut::<c_void>()
}

unsafe extern "C" fn linux_device_link_del(_link: *mut c_void) {}

unsafe extern "C" fn linux_device_link_remove(
    _consumer: *mut LinuxDevice,
    _supplier: *mut LinuxDevice,
) {
}

unsafe extern "C" fn linux_kobject_uevent_env(
    _kobj: *mut c_void,
    _action: i32,
    _envp: *mut *mut c_char,
) -> i32 {
    0
}

unsafe extern "C" fn linux_add_uevent_var(
    _env: *mut c_void,
    _fmt: *const c_char,
    _arg0: usize,
) -> i32 {
    0
}

unsafe extern "C" fn linux___kmalloc_large_noprof(size: usize, flags: u32) -> *mut u8 {
    let ptr = unsafe { crate::mm::slab::__kmalloc_large_noprof(size, flags) };
    if ptr.is_null() || size >= 64 * 1024 {
        crate::log_warn!(
            "kmalloc",
            "__kmalloc_large_noprof: size={} flags=0x{:x} ptr={:p}",
            size,
            flags,
            ptr
        );
    }
    ptr
}

#[repr(C)]
struct LinuxKmemCacheArgs {
    align: u32,
}

unsafe fn linux_kmem_cache_name(name: *const c_char) -> &'static str {
    let Some(bytes) = (unsafe { c_str_bytes(name, 96) }) else {
        return "linux-kmem-cache";
    };
    let Ok(name) = core::str::from_utf8(bytes) else {
        return "linux-kmem-cache";
    };
    Box::leak(String::from(name).into_boxed_str())
}

unsafe extern "C" fn linux___kmem_cache_create_args(
    name: *const c_char,
    object_size: u32,
    args: *const LinuxKmemCacheArgs,
    _flags: u32,
) -> *mut crate::mm::slab::KmemCache {
    if object_size == 0 {
        return ptr::null_mut();
    }
    let cache_name = unsafe { linux_kmem_cache_name(name) };
    let align = if args.is_null() {
        core::mem::size_of::<usize>()
    } else {
        unsafe { (*args).align as usize }.max(core::mem::size_of::<usize>())
    };
    let mut cache = Box::new(crate::mm::slab::KmemCache::const_uninit());
    unsafe {
        cache.init(cache_name, object_size as usize, align);
    }
    let ptr = Box::into_raw(cache);
    crate::log_info!(
        "slab",
        "__kmem_cache_create_args: name={} size={} align={} ptr={:p}",
        cache_name,
        object_size,
        align,
        ptr
    );
    ptr
}

unsafe extern "C" fn linux_kmem_cache_alloc_node_noprof(
    cache: *mut crate::mm::slab::KmemCache,
    flags: u32,
    node: i32,
) -> *mut u8 {
    unsafe { crate::mm::slab::kmem_cache_alloc_node_noprof(cache, flags, node) }
}

unsafe extern "C" fn linux_kmem_cache_destroy(cache: *mut crate::mm::slab::KmemCache) {
    crate::mm::slab::kmem_cache_destroy(cache);
}

unsafe extern "C" fn linux_kmem_cache_free(cache: *mut crate::mm::slab::KmemCache, ptr: *mut u8) {
    unsafe { crate::mm::slab::kmem_cache_free(cache, ptr) };
}

unsafe extern "C" fn linux_kmemdup_noprof(
    src: *const c_void,
    len: usize,
    flags: u32,
) -> *mut c_void {
    if src.is_null() {
        return ptr::null_mut();
    }
    let dst = unsafe { crate::mm::slab::kmalloc(len, flags) };
    if !dst.is_null() && len != 0 {
        unsafe { ptr::copy_nonoverlapping(src.cast::<u8>(), dst, len) };
    }
    dst.cast::<c_void>()
}

unsafe extern "C" fn linux_kvfree(ptr: *mut u8) {
    unsafe { crate::mm::slab::kvfree(ptr) };
}

unsafe extern "C" fn linux_kvfree_call_rcu(head: *mut u8, ptr: *mut u8) {
    crate::mm::mm_public::kvfree_call_rcu(head, ptr);
}

unsafe extern "C" fn linux___folio_put(_folio: *mut c_void) {}

unsafe extern "C" fn linux_free_percpu(ptr: *mut c_void) {
    unsafe { crate::mm::slab::kfree(ptr.cast::<u8>()) };
}

unsafe extern "C" fn linux___clear_pages_unrolled(page: *mut u8) {
    if !page.is_null() {
        unsafe { ptr::write_bytes(page, 0, PAGE_SIZE) };
    }
}

unsafe extern "C" fn linux___get_user_1(ptr: *const u8) -> usize {
    if ptr.is_null() {
        0
    } else {
        unsafe { *ptr as usize }
    }
}

unsafe extern "C" fn linux___get_user_4(ptr: *const u32) -> usize {
    if ptr.is_null() {
        0
    } else {
        unsafe { *ptr as usize }
    }
}

unsafe extern "C" fn linux___put_user_4(value: u32, ptr: *mut u32) -> i32 {
    if ptr.is_null() {
        -EINVAL
    } else {
        unsafe { *ptr = value };
        0
    }
}

unsafe extern "C" fn linux___put_user_8(value: u64, ptr: *mut u64) -> i32 {
    if ptr.is_null() {
        -EINVAL
    } else {
        unsafe { *ptr = value };
        0
    }
}

unsafe extern "C" fn linux___msecs_to_jiffies(ms: u64) -> u64 {
    crate::kernel::time::jiffies::msecs_to_jiffies(ms)
}

unsafe extern "C" fn linux_clock_t_to_jiffies(clock: u64) -> u64 {
    clock.saturating_mul(crate::kernel::time::jiffies::HZ) / CLOCKS_PER_SEC
}

unsafe extern "C" fn linux_jiffies_to_clock_t(jiffies: u64) -> u64 {
    jiffies.saturating_mul(CLOCKS_PER_SEC) / crate::kernel::time::jiffies::HZ
}

unsafe extern "C" fn linux_round_jiffies_relative(jiffies: u64) -> u64 {
    jiffies
}

fn linux_busy_udelay(usecs: u64) {
    if usecs == 0 {
        return;
    }

    let khz = crate::arch::x86::kernel::tsc::tsc_khz();
    if khz != 0 {
        let cycles = usecs.saturating_mul(khz).saturating_add(999) / 1000;
        let start = crate::arch::x86::kernel::tsc::read_ordered();
        while crate::arch::x86::kernel::tsc::read_ordered().wrapping_sub(start) < cycles {
            core::hint::spin_loop();
        }
        return;
    }

    for _ in 0..usecs.saturating_mul(100) {
        core::hint::spin_loop();
    }
}

unsafe extern "C" fn linux___const_udelay(xloops: u64) {
    const UDELAY_XLOOPS_PER_USEC: u64 = 0x0000_10c7;
    let usecs = xloops.saturating_add(UDELAY_XLOOPS_PER_USEC - 1) / UDELAY_XLOOPS_PER_USEC;
    linux_busy_udelay(usecs.max(1));
}

unsafe extern "C" fn linux___udelay(usecs: u64) {
    linux_busy_udelay(usecs);
}

#[repr(C)]
struct LinuxTimerList {
    entry_next: *mut c_void,
    entry_prev: *mut c_void,
    expires: u64,
    function: Option<unsafe extern "C" fn(*mut LinuxTimerList)>,
    flags: u32,
    _pad_after_flags: u32,
}

unsafe extern "C" fn linux_timer_init_key(
    timer: *mut LinuxTimerList,
    func: Option<unsafe extern "C" fn(*mut LinuxTimerList)>,
    flags: u32,
    _name: *const c_char,
    _key: *mut c_void,
) {
    if !timer.is_null() {
        unsafe {
            (*timer).entry_next = ptr::null_mut();
            (*timer).entry_prev = ptr::null_mut();
            (*timer).expires = 0;
            (*timer).function = func;
            (*timer).flags = flags;
            (*timer)._pad_after_flags = 0;
        }
    }
}

unsafe extern "C" fn linux_add_timer(_timer: *mut LinuxTimerList) {}

unsafe extern "C" fn linux_mod_timer(timer: *mut LinuxTimerList, expires: u64) -> i32 {
    if !timer.is_null() {
        unsafe { (*timer).expires = expires };
    }
    0
}

unsafe extern "C" fn linux_timer_delete_sync(_timer: *mut LinuxTimerList) -> i32 {
    0
}

unsafe extern "C" fn linux_schedule_timeout_uninterruptible(timeout: u64) -> u64 {
    crate::kernel::time::sleep_timeout::schedule_timeout_uninterruptible(timeout)
}

unsafe extern "C" fn linux_usleep_range_state(min: u64, max: u64, state: u32) {
    let usec = max.max(min);
    if usec == 0 {
        return;
    }
    let msec = usec.saturating_add(999) / 1000;
    let timeout = crate::kernel::time::sleep_timeout::msecs_to_schedule_timeout(msec).max(1);
    let _ = crate::kernel::time::sleep_timeout::schedule_timeout_with_state(timeout, state);
}

#[repr(C)]
struct LinuxWaitQueueHead {
    head_next: *mut c_void,
    head_prev: *mut c_void,
}

#[repr(C)]
struct LinuxWaitQueueEntry {
    flags: u32,
    private: *mut c_void,
    func: Option<unsafe extern "C" fn(*mut c_void, u32, i32, *mut c_void) -> i32>,
    entry_next: *mut c_void,
    entry_prev: *mut c_void,
}

fn scsi_host_diag(data: *mut c_void) -> Option<(u32, u32, u32)> {
    if data.is_null() || (data as u64) < crate::arch::x86::mm::paging::PAGE_OFFSET {
        return None;
    }
    Some(unsafe {
        (
            core::ptr::read_unaligned(
                (data as *const u8).add(LINUX_SCSI_HOST_HOST_FAILED_OFFSET) as *const u32
            ),
            core::ptr::read_unaligned(
                (data as *const u8).add(LINUX_SCSI_HOST_HOST_EH_SCHEDULED_OFFSET) as *const u32,
            ),
            core::ptr::read_unaligned(
                (data as *const u8).add(LINUX_SCSI_HOST_SHOST_STATE_OFFSET) as *const u32
            ),
        )
    })
}

unsafe extern "C" fn linux___init_waitqueue_head(
    queue: *mut LinuxWaitQueueHead,
    _name: *const c_char,
    _key: *mut c_void,
) {
    if queue.is_null() {
        return;
    }
    unsafe {
        let head = queue.cast::<c_void>();
        (*queue).head_next = head;
        (*queue).head_prev = head;
    }
}

unsafe extern "C" fn linux_init_wait_entry(wait: *mut c_void, flags: i32) {
    if wait.is_null() {
        return;
    }
    unsafe {
        ptr::write_bytes(wait, 0, LINUX_WAIT_QUEUE_ENTRY_SIZE);
        *(wait.cast::<i32>()) = flags;
    }
}

unsafe fn wait_entry_list(wait: *mut c_void) -> *mut c_void {
    unsafe { (wait as *mut u8).add(LINUX_WAIT_QUEUE_ENTRY_LIST_OFFSET) as *mut c_void }
}

unsafe fn list_next(list: *mut c_void) -> *mut *mut c_void {
    list as *mut *mut c_void
}

unsafe fn list_prev(list: *mut c_void) -> *mut *mut c_void {
    unsafe { (list as *mut u8).add(LINUX_LIST_HEAD_PREV_OFFSET) as *mut *mut c_void }
}

unsafe extern "C" fn linux_add_wait_queue(queue: *mut LinuxWaitQueueHead, wait: *mut c_void) {
    if queue.is_null() || wait.is_null() {
        return;
    }
    let head = queue.cast::<c_void>();
    let entry = unsafe { wait_entry_list(wait) };
    unsafe {
        if (*queue).head_next.is_null() || (*queue).head_prev.is_null() {
            (*queue).head_next = head;
            (*queue).head_prev = head;
        }
        let next = (*queue).head_next;
        *list_next(entry) = next;
        *list_prev(entry) = head;
        *list_prev(next) = entry;
        (*queue).head_next = entry;
    }
}

unsafe extern "C" fn linux_remove_wait_queue(_queue: *mut LinuxWaitQueueHead, wait: *mut c_void) {
    if wait.is_null() {
        return;
    }
    let entry = unsafe { wait_entry_list(wait) };
    unsafe {
        let next = *list_next(entry);
        let prev = *list_prev(entry);
        if !next.is_null() && !prev.is_null() {
            *list_next(prev) = next;
            *list_prev(next) = prev;
        }
        *list_next(entry) = entry;
        *list_prev(entry) = entry;
    }
}

unsafe extern "C" fn linux_prepare_to_wait(
    _queue: *mut LinuxWaitQueueHead,
    _wait: *mut c_void,
    _state: i32,
) {
}

unsafe extern "C" fn linux_prepare_to_wait_event(
    _queue: *mut LinuxWaitQueueHead,
    _wait: *mut c_void,
    _state: i32,
) -> i64 {
    0
}

unsafe extern "C" fn linux_finish_wait(_queue: *mut LinuxWaitQueueHead, _wait: *mut c_void) {}

unsafe extern "C" fn linux___wake_up(
    _queue: *mut LinuxWaitQueueHead,
    _mode: u32,
    _nr_exclusive: i32,
    _key: *mut c_void,
) {
}

unsafe extern "C" fn linux_autoremove_wake_function(
    _wait: *mut c_void,
    _mode: u32,
    _sync: i32,
    _key: *mut c_void,
) -> i32 {
    1
}

unsafe extern "C" fn linux_default_wake_function(
    _wait: *mut c_void,
    _mode: u32,
    _sync: i32,
    _key: *mut c_void,
) -> i32 {
    1
}

unsafe extern "C" fn linux_wait_for_completion_timeout(
    completion: *mut c_void,
    timeout: u64,
) -> u64 {
    if completion.is_null() {
        return 0;
    }
    if unsafe { crate::kernel::sched::completion::linux_try_wait_for_completion_raw(completion) } {
        return timeout.max(1);
    }
    if timeout == 0 {
        return 0;
    }

    let expires = crate::kernel::time::jiffies::jiffies().saturating_add(timeout);
    loop {
        let _ = poll_ahci_interrupts();

        #[cfg(not(test))]
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
        #[cfg(test)]
        return 0;

        if unsafe {
            crate::kernel::sched::completion::linux_try_wait_for_completion_raw(completion)
        } {
            return expires
                .saturating_sub(crate::kernel::time::jiffies::jiffies())
                .max(1);
        }
        if crate::kernel::time::jiffies::jiffies() >= expires {
            return 0;
        }
    }
}

unsafe extern "C" fn linux_schedule() {
    #[cfg(not(test))]
    {
        // Reap driver-ABI completions before yielding. A Linux-built driver that
        // waits by busy-calling schedule() (e.g. libata's reset/wait loops) keeps
        // a task runnable, so the scheduler never goes idle and the idle-path
        // driver pump never runs — on a multi-CPU boot the AHCI completion it is
        // waiting for would then never be delivered. The reentrancy guard inside
        // poll_driver_abi_events makes this safe on any path.
        let _ = crate::linux_driver_abi::poll_driver_abi_events();
        unsafe {
            crate::kernel::sched::schedule_with_irqs_enabled();
        }
    }
}

unsafe extern "C" fn linux_wake_up_process(task: *mut c_void) -> i32 {
    let woke = unsafe { crate::kernel::sched::wake_task(task.cast()) };
    if woke { 1 } else { 0 }
}

unsafe extern "C" fn linux_lupos_ata_diag(
    stage: *const c_char,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
    a6: usize,
    a7: usize,
    a8: usize,
    a9: usize,
    a10: usize,
    a11: usize,
    a12: usize,
) {
    // Diagnostic-only scalar hook; this module does not mirror or dereference
    // vendor `struct ata_*` objects.
    let stage = unsafe { c_str_bytes(stage, 96) }.unwrap_or(b"(null)");
    crate::log_warn!(
        "ata",
        "diag: stage={:?} a=[{:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}]",
        stage,
        a0,
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        a8,
        a9,
        a10,
        a11,
        a12
    );
}

unsafe extern "C" fn linux_queue_delayed_work_on(
    cpu: i32,
    wq: *mut Workqueue,
    dwork: *mut c_void,
    _delay: u64,
) -> bool {
    unsafe { linux_queue_work_on(cpu, wq, dwork.cast::<WorkStruct>()) }
}

unsafe extern "C" fn linux___flush_workqueue(wq: *mut Workqueue) {
    if wq.is_null() {
        return;
    }
    for _ in 0..WORKQUEUE_DRAIN_SPINS {
        let queue = unsafe { Arc::from_raw(wq) };
        let pending = queue.nr_pending();
        crate::kernel::workqueue::flush_workqueue(&queue);
        let _ = Arc::into_raw(queue);
        if pending == 0 {
            break;
        }
    }
}

unsafe extern "C" fn linux_cancel_work(work: *mut WorkStruct) -> bool {
    if work.is_null() {
        false
    } else {
        unsafe {
            (*work)
                .data
                .fetch_and(!crate::kernel::workqueue::WORK_PENDING, Ordering::AcqRel)
                & crate::kernel::workqueue::WORK_PENDING
                != 0
        }
    }
}

unsafe extern "C" fn linux_cancel_work_sync(work: *mut WorkStruct) -> bool {
    unsafe { linux_cancel_work(work) }
}

unsafe extern "C" fn linux_cancel_delayed_work_sync(dwork: *mut c_void) -> bool {
    unsafe { linux_cancel_work_sync(dwork.cast::<WorkStruct>()) }
}

unsafe extern "C" fn linux_delayed_work_timer_fn(_timer: *mut c_void) {}

unsafe extern "C" fn linux_synchronize_rcu() {
    crate::kernel::rcu::synchronize_rcu();
}

unsafe extern "C" fn linux_rcu_barrier() {
    crate::kernel::rcu::rcu_barrier();
}

unsafe extern "C" fn linux_call_rcu(
    head: *mut crate::kernel::rcu::RcuHead,
    func: unsafe extern "C" fn(*mut crate::kernel::rcu::RcuHead),
) {
    crate::kernel::rcu::call_rcu(head, func);
}

type LinuxKthreadFn = unsafe extern "C" fn(*mut c_void) -> i32;

unsafe extern "C" fn linux_kthread_create_on_node(
    threadfn: LinuxKthreadFn,
    data: *mut c_void,
    _node: i32,
    name: *const c_char,
) -> *mut c_void {
    let mut task_name = [0u8; 16];
    if let Some(bytes) = unsafe { c_str_bytes(name, 15) } {
        let copy = core::cmp::min(bytes.len(), task_name.len() - 1);
        task_name[..copy].copy_from_slice(&bytes[..copy]);
    }
    let task: *mut c_void = unsafe {
        crate::kernel::kthread::kthread_create_on_node(threadfn, data, &task_name).cast()
    };
    let (scsi_failed, scsi_eh_scheduled, scsi_state) = scsi_host_diag(data).unwrap_or((0, 0, 0));
    if task.is_null() {
        crate::log_warn!("kthread", "kthread_create_on_node: returned null");
    } else {
        crate::log_info!(
            "kthread",
            "kthread_create_on_node: task={:p} data={:p} host_failed={} host_eh_scheduled={} data_state={} name_ptr={:p}",
            task,
            data,
            scsi_failed,
            scsi_eh_scheduled,
            scsi_state,
            name
        );
    }
    task
}

unsafe extern "C" fn linux_kthread_should_stop() -> bool {
    let current = unsafe { crate::kernel::sched::get_current() };
    if !current.is_null() {
        let data = crate::kernel::kthread::kthread_data(current);
        let scsi = scsi_host_diag(data);
        crate::log_info!(
            "kthread",
            "kthread_should_stop entry: current={:p} pid={} state={:#x} data={:p} scsi={:?}",
            current,
            unsafe { (*current).pid },
            unsafe { (*current).__state.load(Ordering::Acquire) },
            data,
            scsi
        );
    }
    let ret = unsafe { crate::kernel::kthread::kthread_should_stop() };
    if !current.is_null() {
        let data = crate::kernel::kthread::kthread_data(current);
        let scsi = scsi_host_diag(data);
        crate::log_info!(
            "kthread",
            "kthread_should_stop: current={:p} pid={} state={:#x} data={:p} scsi={:?} ret={}",
            current,
            unsafe { (*current).pid },
            unsafe { (*current).__state.load(Ordering::Acquire) },
            data,
            scsi,
            ret
        );
    }
    ret
}

unsafe extern "C" fn linux_kthread_stop(task: *mut c_void) -> i32 {
    unsafe { crate::kernel::kthread::kthread_stop(task.cast()) }
}

unsafe extern "C" fn linux_dma_map_sg_attrs(
    _dev: *mut c_void,
    sg: *mut LinuxScatterList,
    nents: i32,
    dir: crate::kernel::dma::DmaDirection,
    _attrs: u64,
) -> i32 {
    if sg.is_null() || nents <= 0 {
        return 0;
    }
    let mut mapped = 0i32;
    let mut current = sg;
    while mapped < nents && !current.is_null() {
        let cpu = unsafe { linux_sg_cpu_addr(current) };
        let len = unsafe { (*current).length as usize };
        let dma = crate::kernel::dma::dma_map_single(cpu.cast(), len, dir);
        unsafe {
            (*current).dma_address = dma as usize;
            (*current).dma_length = (*current).length;
        }
        mapped += 1;
        if unsafe { (*current).page_link & SG_END != 0 } {
            break;
        }
        current = unsafe { current.add(1) };
    }
    mapped
}

unsafe extern "C" fn linux_dma_unmap_sg_attrs(
    _dev: *mut c_void,
    _sg: *mut LinuxScatterList,
    _nents: i32,
    _dir: crate::kernel::dma::DmaDirection,
    _attrs: u64,
) {
}

unsafe extern "C" fn linux_dma_max_mapping_size(_dev: *mut c_void) -> usize {
    usize::MAX
}

unsafe extern "C" fn linux_dmam_alloc_attrs(
    dev: *mut c_void,
    size: usize,
    dma_handle: *mut u64,
    _gfp: u32,
    _attrs: u64,
) -> *mut c_void {
    let Some((ptr, dma)) = crate::kernel::dma::dma_alloc_coherent(size) else {
        crate::log_warn!(
            "dma",
            "dmam_alloc_attrs: dev={:p} size={} returned null",
            dev,
            size
        );
        return ptr::null_mut();
    };
    if !dma_handle.is_null() {
        unsafe { *dma_handle = dma };
    }
    crate::log_info!(
        "dma",
        "dmam_alloc_attrs: dev={:p} size={} ptr={:p} dma={:#x}",
        dev,
        size,
        ptr,
        dma
    );
    ptr.cast::<c_void>()
}

unsafe fn linux_sg_cpu_addr(sg: *const LinuxScatterList) -> *mut u8 {
    if sg.is_null() {
        return ptr::null_mut();
    }
    let base = unsafe { (*sg).page_link & !SG_PAGE_LINK_MASK };
    let offset = unsafe { (*sg).offset as usize };
    (base + offset) as *mut u8
}

unsafe extern "C" fn linux_sg_copy_to_buffer(
    sg: *mut LinuxScatterList,
    nents: u32,
    buf: *mut c_void,
    buflen: usize,
) -> usize {
    unsafe { linux_sg_copy_buffer(sg, nents, buf.cast::<u8>(), buflen, true) }
}

unsafe extern "C" fn linux_sg_copy_from_buffer(
    sg: *mut LinuxScatterList,
    nents: u32,
    buf: *const c_void,
    buflen: usize,
) -> usize {
    unsafe { linux_sg_copy_buffer(sg, nents, buf.cast_mut().cast::<u8>(), buflen, false) }
}

unsafe fn linux_sg_copy_buffer(
    sg: *mut LinuxScatterList,
    nents: u32,
    buf: *mut u8,
    buflen: usize,
    to_buffer: bool,
) -> usize {
    if sg.is_null() || buf.is_null() {
        return 0;
    }
    let mut copied = 0usize;
    let mut current = sg;
    let mut idx = 0u32;
    while idx < nents && copied < buflen && !current.is_null() {
        let sg_len = unsafe { (*current).length as usize };
        let len = core::cmp::min(sg_len, buflen - copied);
        let sg_addr = unsafe { linux_sg_cpu_addr(current) };
        if !sg_addr.is_null() && len != 0 {
            unsafe {
                if to_buffer {
                    ptr::copy_nonoverlapping(sg_addr, buf.add(copied), len);
                } else {
                    ptr::copy_nonoverlapping(buf.add(copied), sg_addr, len);
                }
            }
            copied += len;
        }
        if unsafe { (*current).page_link & SG_END != 0 } {
            break;
        }
        idx += 1;
        current = unsafe { current.add(1) };
    }
    copied
}

#[repr(C)]
struct LinuxSgPageIter {
    sg: *mut LinuxScatterList,
    sg_pgoffset: u32,
    nents: u32,
    pg_advance: i32,
}

#[repr(C)]
struct LinuxSgMappingIter {
    page: *mut c_void,
    addr: *mut c_void,
    length: usize,
    consumed: usize,
    piter: LinuxSgPageIter,
    offset: u32,
    remaining: u32,
    flags: u32,
}

unsafe extern "C" fn linux_sg_miter_start(
    miter: *mut LinuxSgMappingIter,
    sg: *mut LinuxScatterList,
    nents: u32,
    _flags: u32,
) {
    if !miter.is_null() {
        unsafe {
            ptr::write_bytes(miter, 0, 1);
            (*miter).piter.sg = sg;
            (*miter).piter.nents = nents;
            (*miter).page = ptr::null_mut();
            (*miter).addr = ptr::null_mut();
            (*miter).length = 0;
            (*miter).consumed = 0;
        }
    }
}

unsafe extern "C" fn linux_sg_miter_next(miter: *mut LinuxSgMappingIter) -> bool {
    if miter.is_null() {
        return false;
    }
    unsafe {
        if (*miter).piter.nents == 0 || (*miter).piter.sg.is_null() {
            return false;
        }
        let sg = (*miter).piter.sg;
        (*miter).addr = linux_sg_cpu_addr(sg).cast::<c_void>();
        (*miter).page =
            ((*sg).page_link & !crate::lib::scatterlist::SG_PAGE_LINK_MASK) as *mut c_void;
        (*miter).length = (*sg).length as usize;
        (*miter).consumed = (*miter).length;
        (*miter).piter.sg = sg.add(1);
        (*miter).piter.nents -= 1;
        true
    }
}

unsafe extern "C" fn linux_sg_miter_skip(miter: *mut LinuxSgMappingIter, mut offset: i64) -> bool {
    if miter.is_null() || offset < 0 {
        return false;
    }
    unsafe { linux_sg_miter_stop(miter) };

    unsafe {
        while offset != 0 {
            if (*miter).piter.nents == 0 || (*miter).piter.sg.is_null() {
                return false;
            }
            let sg = (*miter).piter.sg;
            let len = (*sg).length as i64;
            if offset < len {
                let skipped = offset as usize;
                (*miter).addr = linux_sg_cpu_addr(sg).add(skipped).cast::<c_void>();
                (*miter).page =
                    ((*sg).page_link & !crate::lib::scatterlist::SG_PAGE_LINK_MASK) as *mut c_void;
                (*miter).length = (*sg).length as usize - skipped;
                (*miter).consumed = skipped;
                (*miter).offset = (*sg).offset.saturating_add(skipped as u32);
                (*miter).remaining = ((*sg).length as usize - skipped) as u32;
                return true;
            }
            offset -= len;
            (*miter).piter.sg = sg.add(1);
            (*miter).piter.nents -= 1;
        }
    }

    true
}

unsafe extern "C" fn linux_sg_miter_stop(_miter: *mut LinuxSgMappingIter) {}

unsafe extern "C" fn linux___blk_mq_end_request(rq: *mut LinuxRequest, status: LinuxBlkStatus) {
    unsafe { blk_mq_end_request(rq, status) };
}

unsafe extern "C" fn linux_blk_mq_alloc_queue(
    set: *mut LinuxBlkMqTagSet,
    lim: *const LinuxQueueLimits,
    queuedata: *mut c_void,
) -> *mut LinuxRequestQueue {
    let q = unsafe { block_blk_mq_alloc_queue(set, lim, queuedata) };
    crate::log_warn!(
        "block",
        "blk_mq_alloc_queue export: set={:p} queuedata={:p} q={:p}",
        set,
        queuedata,
        q
    );
    q
}

unsafe extern "C" fn linux_blk_mq_alloc_disk_for_queue(
    q: *mut LinuxRequestQueue,
    _lkclass: *mut c_void,
) -> *mut LinuxGendisk {
    if q.is_null() {
        return ptr::null_mut();
    }
    let set = unsafe { (*q).tag_set };
    if set.is_null() {
        return ptr::null_mut();
    }
    let disk = unsafe { block_blk_mq_alloc_disk_for_queue(q, ptr::null_mut(), true) };
    crate::log_warn!(
        "block",
        "blk_mq_alloc_disk_for_queue export: q={:p} disk={:p}",
        q,
        disk
    );
    disk
}

unsafe extern "C" fn linux_blk_mq_destroy_queue(_q: *mut LinuxRequestQueue) {}
unsafe extern "C" fn linux_blk_mq_run_hw_queues(_q: *mut LinuxRequestQueue, _async_: bool) {}
unsafe extern "C" fn linux_blk_mq_delay_run_hw_queues(_q: *mut LinuxRequestQueue, _msecs: u64) {}
unsafe extern "C" fn linux_blk_mq_kick_requeue_list(_q: *mut LinuxRequestQueue) {}
unsafe extern "C" fn linux_blk_mq_delay_kick_requeue_list(_q: *mut LinuxRequestQueue, _msecs: u64) {
}
unsafe extern "C" fn linux_blk_mq_tagset_busy_iter(
    _set: *mut LinuxBlkMqTagSet,
    _fn: *mut c_void,
    _priv_: *mut c_void,
) {
}
unsafe extern "C" fn linux_blk_mq_wait_quiesce_done(_set: *mut LinuxBlkMqTagSet) {}

unsafe extern "C" fn linux_blk_get_queue(q: *mut LinuxRequestQueue) -> bool {
    if !q.is_null() {
        unsafe { (*q).refs = (*q).refs.saturating_add(1) };
        true
    } else {
        false
    }
}

unsafe extern "C" fn linux_blk_put_queue(q: *mut LinuxRequestQueue) {
    if !q.is_null() {
        unsafe { (*q).refs = (*q).refs.saturating_sub(1) };
    }
}

unsafe extern "C" fn linux_blk_queue_rq_timeout(q: *mut LinuxRequestQueue, timeout: u32) {
    if !q.is_null() {
        unsafe { (*q).rq_timeout = timeout };
    }
}

unsafe extern "C" fn linux_blk_set_queue_depth(q: *mut LinuxRequestQueue, depth: u32) {
    if !q.is_null() {
        unsafe { (*q).queue_depth = depth };
    }
}

unsafe extern "C" fn linux_blk_clear_pm_only(q: *mut LinuxRequestQueue) {
    if !q.is_null() {
        unsafe { (*q).pm_only = (*q).pm_only.saturating_sub(1) };
    }
}

unsafe extern "C" fn linux_blk_set_pm_only(q: *mut LinuxRequestQueue) {
    if !q.is_null() {
        unsafe { (*q).pm_only = (*q).pm_only.saturating_add(1) };
    }
}

unsafe extern "C" fn linux_blk_update_request(
    rq: *mut LinuxRequest,
    _error: LinuxBlkStatus,
    nr_bytes: u32,
) -> bool {
    if rq.is_null() {
        return false;
    }
    unsafe {
        if nr_bytes >= (*rq).data_len {
            (*rq).data_len = 0;
            false
        } else {
            (*rq).data_len -= nr_bytes;
            true
        }
    }
}

unsafe extern "C" fn linux_blk_abort_request(rq: *mut LinuxRequest) {
    if !rq.is_null() {
        unsafe { blk_mq_end_request(rq, crate::linux_driver_abi::block::BLK_STS_IOERR) };
    }
}

unsafe extern "C" fn linux_blk_rq_init(q: *mut LinuxRequestQueue, rq: *mut LinuxRequest) {
    if !rq.is_null() {
        unsafe {
            ptr::write_bytes(rq.cast::<u8>(), 0, core::mem::size_of::<LinuxRequest>());
            (*rq).q = q;
        }
    }
}

unsafe extern "C" fn linux_blk_rq_map_user(
    _q: *mut LinuxRequestQueue,
    _rq: *mut LinuxRequest,
    _map_data: *mut c_void,
    _ubuf: *mut c_void,
    _len: u32,
    _gfp: u32,
) -> i32 {
    -ENODEV
}

unsafe extern "C" fn linux_blk_rq_map_user_iov(
    _q: *mut LinuxRequestQueue,
    _rq: *mut LinuxRequest,
    _map_data: *mut c_void,
    _iter: *mut c_void,
    _gfp: u32,
) -> i32 {
    -ENODEV
}

unsafe extern "C" fn linux_blk_rq_map_user_io(
    _q: *mut LinuxRequestQueue,
    _rq: *mut LinuxRequest,
    _map_data: *mut c_void,
    _ubuf: *mut c_void,
    _len: u32,
    _gfp: u32,
    _reading: bool,
    _copy: bool,
) -> i32 {
    -ENODEV
}

unsafe extern "C" fn linux_blk_rq_unmap_user(_bio: *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_blk_execute_rq_nowait(rq: *mut LinuxRequest, at_head: bool) {
    let _ = unsafe { blk_execute_rq(rq, at_head) };
}

unsafe extern "C" fn linux_queue_limits_commit_update(
    q: *mut LinuxRequestQueue,
    lim: *const LinuxQueueLimits,
) -> i32 {
    unsafe { crate::linux_driver_abi::block::queue_limits_commit_update_frozen(q, lim) }
}

unsafe extern "C" fn linux_add_disk_randomness(_disk: *mut LinuxGendisk) {}
unsafe extern "C" fn linux_disk_alloc_independent_access_ranges(
    _disk: *mut LinuxGendisk,
    _nr: i32,
) -> *mut c_void {
    ptr::null_mut()
}
unsafe extern "C" fn linux_disk_set_independent_access_ranges(
    _disk: *mut LinuxGendisk,
    _iars: *mut c_void,
) {
}
unsafe extern "C" fn linux_disk_check_media_change(_disk: *mut LinuxGendisk) -> bool {
    false
}
unsafe extern "C" fn linux_kblockd_schedule_work(work: *mut WorkStruct) -> bool {
    unsafe {
        linux_queue_work_on(
            0,
            LINUX_SYSTEM_WQ.load(Ordering::Acquire) as *mut Workqueue,
            work,
        )
    }
}

unsafe extern "C" fn linux_pcim_enable_device(dev: *mut c_void) -> i32 {
    let ret = unsafe { crate::linux_driver_abi::pci::pci::pci_enable_device(dev) };
    if ret != 0 {
        crate::log_warn!("pci", "pcim_enable_device: dev={:p} errno {}", dev, ret);
    } else {
        crate::log_warn!("pci", "pcim_enable_device: dev={:p} ok", dev);
    }
    ret
}

unsafe extern "C" fn linux_pcim_pin_device(_dev: *mut c_void) {}

unsafe extern "C" fn linux_pcim_intx(dev: *mut c_void, enable: i32) -> i32 {
    const PCI_COMMAND: usize = 0x04;
    const PCI_COMMAND_INTX_DISABLE: u16 = 0x0400;

    let Some(command) = linux_pci_config_read(dev.cast_const(), PCI_COMMAND, 2) else {
        return -EINVAL;
    };
    let command = if enable != 0 {
        (command as u16) & !PCI_COMMAND_INTX_DISABLE
    } else {
        (command as u16) | PCI_COMMAND_INTX_DISABLE
    };
    if linux_pci_config_write(dev.cast_const(), PCI_COMMAND, 2, command as u32) {
        0
    } else {
        -EINVAL
    }
}

#[derive(Clone)]
struct PcimIomapEntry {
    dev: usize,
    table: Box<[usize; 6]>,
}

#[derive(Clone, Copy)]
struct DevmActionEntry {
    dev: usize,
    action: usize,
    data: usize,
}

lazy_static! {
    static ref PCIM_IOMAPS: Mutex<Vec<PcimIomapEntry>> = Mutex::new(Vec::new());
    static ref DEVM_ACTIONS: Mutex<Vec<DevmActionEntry>> = Mutex::new(Vec::new());
    static ref LINUX_CREATED_DEVICES: Mutex<Vec<LinuxCreatedDevice>> = Mutex::new(Vec::new());
}

static AHCI_BAR5_MMIO: AtomicUsize = AtomicUsize::new(0);
static AHCI_LEGACY_IRQ: AtomicUsize = AtomicUsize::new(0);
static AHCI_IRQ_DEV_ID: AtomicUsize = AtomicUsize::new(0);
/// Cached `ahci_handle_port_intr` address so the hot completion-poll path
/// avoids the O(n) `find_symbol` linear scan on every poll.
static AHCI_PORT_INTR_FN: AtomicUsize = AtomicUsize::new(0);
/// Per-port last-seen outstanding-command mask (`PxCI | PxSACT`). Used to detect
/// the falling edge — commands that have just drained — so the completion poll
/// can reap them even on emulated HBAs that finish commands without latching any
/// interrupt status (`PxIS`/`HOST_IRQ_STAT` stay 0, `PxIE` may be 0).
static AHCI_PORT_PREV_OUTSTANDING: [core::sync::atomic::AtomicU32; 32] =
    [const { core::sync::atomic::AtomicU32::new(0) }; 32];

fn pcim_iomap_entry(dev: *mut c_void) -> *mut [usize; 6] {
    let mut entries = PCIM_IOMAPS.lock();
    if let Some(entry) = entries.iter_mut().find(|entry| entry.dev == dev as usize) {
        return &mut *entry.table;
    }
    entries.push(PcimIomapEntry {
        dev: dev as usize,
        table: Box::new([0; 6]),
    });
    &mut *entries.last_mut().unwrap().table
}

unsafe fn ahci_readl(mmio: *mut c_void, offset: usize) -> u32 {
    unsafe { core::ptr::read_volatile((mmio as *const u8).add(offset).cast::<u32>()) }
}

unsafe fn ahci_writel(mmio: *mut c_void, offset: usize, value: u32) {
    unsafe { core::ptr::write_volatile((mmio as *mut u8).add(offset).cast::<u32>(), value) };
}

/// Enable AHCI interrupt *latching*: GHC.IE (bit1 @0x04) + PxIE (base+0x14) for
/// every implemented port. This is the key to reading PxSACT consistently.
///
/// `ahci_qc_complete` reads PxSACT and hands it to `ata_qc_complete_multiple`,
/// which FREEZES the port (`-EINVAL` → `ata_port_freeze`) if PxSACT has any bit
/// not in `ap->qc_active`. On VirtualBox PxSACT clears late, so reading it at an
/// arbitrary poll catches a stale bit → freeze → BLK_STS_RESOURCE forever. But
/// the HBA sets PxIS (and HOST_IRQ_STAT) only *after* the SDB FIS has cleared
/// PxSACT, so reading PxSACT in response to a latched PxIS is always consistent.
/// Enabling PxIE makes PxIS latch on completion so the poll can gate on it.
/// GHC=0x04, PORT_IRQ_MASK=0x14 — vendor/linux/drivers/ata/ahci.h.
unsafe fn ahci_enable_interrupts(mmio: *mut c_void) {
    if mmio.is_null() {
        return;
    }
    let ports_impl = unsafe { ahci_readl(mmio, 0x0c) };
    let mut port = 0usize;
    while port < 32 {
        if ports_impl & (1u32 << port) != 0 {
            let base = 0x100 + port * 0x80;
            unsafe { ahci_writel(mmio, base + 0x14, 0x7840_00ff) };
        }
        port += 1;
    }
    let ghc = unsafe { ahci_readl(mmio, 0x04) };
    unsafe { ahci_writel(mmio, 0x04, ghc | 0x2) };
}

/// Upper bound for a physical address the AHCI debug dumper is willing to
/// dereference.  Matches the direct-map coverage tracked by the frame
/// allocator (`MAX_PHYS_MEMORY` in src/mm/frame.rs).  Command lists for
/// inactive AHCI ports can contain uninitialised DMA garbage; without this
/// guard the dumper would translate a bogus `table` pointer through
/// `phys_to_virt` and fault on an unaligned/out-of-range `read_volatile`.
const AHCI_DEBUG_MAX_PHYS: u64 = 64 * 1024 * 1024 * 1024;

unsafe fn ahci_read_phys_u32(phys: u64, offset: usize) -> u32 {
    let addr = phys.saturating_add(offset as u64);
    // u32 MMIO/DMA reads must be naturally aligned, and we must stay inside the
    // RAM the direct map actually covers.  Diagnostics that hit a garbage
    // pointer return 0 rather than crashing the kernel.
    if addr % 4 != 0 || addr >= AHCI_DEBUG_MAX_PHYS {
        return 0;
    }
    let ptr = crate::arch::x86::mm::paging::phys_to_virt(addr);
    unsafe { core::ptr::read_volatile(ptr.cast::<u32>()) }
}

fn poll_ahci_interrupts() -> usize {
    let mmio = AHCI_BAR5_MMIO.load(Ordering::Acquire) as *mut c_void;
    let irq = AHCI_LEGACY_IRQ.load(Ordering::Acquire) as u32;
    if mmio.is_null() || irq == 0 {
        return 0;
    }

    let host_is = unsafe { ahci_readl(mmio, 0x08) };
    let mut handled = 0usize;
    if host_is != 0 {
        let irq_handled = crate::kernel::irq::generic_handle_irq(irq);
        if irq_handled > 0 {
            handled += irq_handled as usize;
        }
    }
    // Always run the software completion reaper too; it completes synchronously and
    // is idempotent for an already-reaped qc. Empirically this is what gets real
    // VirtualBox furthest (skipping it makes queue_rq() hit BLK_STS_RESOURCE sooner).
    handled += poll_ahci_completed_ports(mmio);
    handled
}

fn poll_ahci_completed_ports(mmio: *mut c_void) -> usize {
    let host = AHCI_IRQ_DEV_ID.load(Ordering::Acquire) as *mut c_void;
    if host.is_null() {
        return 0;
    }
    let ports_impl = unsafe { ahci_readl(mmio, 0x0c) };
    // Service ONLY ports with a *latched* port interrupt (PxIS & PxIE) — never
    // poll-and-service unconditionally.
    //
    // ahci_qc_complete() reads PxSACT/PxCI and hands it to
    // ata_qc_complete_multiple(), which FREEZES the port (-EINVAL ->
    // ata_port_freeze) if the hardware register holds any bit not in
    // ap->qc_active. On VirtualBox PxSACT clears late, so reading it at an
    // arbitrary instant catches a stale bit and permanently freezes the port
    // (queue_rq -> BLK_STS_RESOURCE forever). The HBA latches PxIS only AFTER the
    // SDB FIS has cleared PxSACT, so reading PxSACT in response to a latched PxIS
    // is always consistent. We enable PxIE at registration (ahci_enable_interrupts)
    // precisely so PxIS latches on completion and this gate fires.
    // PORT_IRQ_STAT=0x10 PORT_IRQ_MASK=0x14 — vendor/linux drivers/ata/ahci.h.
    let mut reap_mask = 0u32;
    let mut port = 0usize;
    while port < 32 {
        if ports_impl & (1u32 << port) != 0 {
            let base = 0x100 + port * 0x80;
            let is = unsafe { ahci_readl(mmio, base + 0x10) };
            let ie = unsafe { ahci_readl(mmio, base + 0x14) };
            if (is & ie) != 0 {
                reap_mask |= 1u32 << port;
            }
        }
        port += 1;
    }
    let activity = reap_mask.count_ones() as usize;
    if reap_mask == 0 {
        return 0;
    }
    // Use only the address resolved once at IRQ registration. Never walk the
    // exported-symbol table from this hot, idle-/wait-reachable path.
    let cached = AHCI_PORT_INTR_FN.load(Ordering::Acquire);
    if cached == 0 {
        return 0;
    }
    let handler: unsafe extern "C" fn(*mut c_void, u32) -> u32 =
        unsafe { core::mem::transmute(cached) };
    let _ = unsafe { handler(host, reap_mask) };
    activity
}

unsafe fn log_ahci_bar5_registers(dev: *mut c_void, mmio: *mut c_void) {
    if mmio.is_null() {
        return;
    }

    let cap = unsafe { ahci_readl(mmio, 0x00) };
    let ghc = unsafe { ahci_readl(mmio, 0x04) };
    let irq_stat = unsafe { ahci_readl(mmio, 0x08) };
    let ports_impl = unsafe { ahci_readl(mmio, 0x0c) };
    let version = unsafe { ahci_readl(mmio, 0x10) };
    let cap2 = unsafe { ahci_readl(mmio, 0x24) };
    let bohc = unsafe { ahci_readl(mmio, 0x28) };

    crate::log_warn!(
        "ahci",
        "bar5 regs: dev={:p} mmio={:p} cap=0x{:08x} ghc=0x{:08x} is=0x{:08x} pi=0x{:08x} vs=0x{:08x} cap2=0x{:08x} bohc=0x{:08x}",
        dev,
        mmio,
        cap,
        ghc,
        irq_stat,
        ports_impl,
        version,
        cap2,
        bohc
    );

    let mut port = 0usize;
    while port < 32 {
        if ports_impl & (1u32 << port) != 0 {
            let base = 0x100 + port * 0x80;
            let clb = unsafe { ahci_readl(mmio, base) };
            let clbu = unsafe { ahci_readl(mmio, base + 0x04) };
            let fb = unsafe { ahci_readl(mmio, base + 0x08) };
            let fbu = unsafe { ahci_readl(mmio, base + 0x0c) };
            let is = unsafe { ahci_readl(mmio, base + 0x10) };
            let ie = unsafe { ahci_readl(mmio, base + 0x14) };
            let cmd = unsafe { ahci_readl(mmio, base + 0x18) };
            let tfd = unsafe { ahci_readl(mmio, base + 0x20) };
            let sig = unsafe { ahci_readl(mmio, base + 0x24) };
            let ssts = unsafe { ahci_readl(mmio, base + 0x28) };
            let sctl = unsafe { ahci_readl(mmio, base + 0x2c) };
            let serr = unsafe { ahci_readl(mmio, base + 0x30) };
            let sact = unsafe { ahci_readl(mmio, base + 0x34) };
            let ci = unsafe { ahci_readl(mmio, base + 0x38) };
            crate::log_warn!(
                "ahci",
                "bar5 port{}: clb=0x{:08x} clbu=0x{:08x} fb=0x{:08x} fbu=0x{:08x} is=0x{:08x} ie=0x{:08x} cmd=0x{:08x} tfd=0x{:08x} sig=0x{:08x} ssts=0x{:08x} sctl=0x{:08x} serr=0x{:08x} sact=0x{:08x} ci=0x{:08x}",
                port,
                clb,
                clbu,
                fb,
                fbu,
                is,
                ie,
                cmd,
                tfd,
                sig,
                ssts,
                sctl,
                serr,
                sact,
                ci
            );
            let cmd_list = ((clbu as u64) << 32) | clb as u64;
            if cmd_list != 0 {
                unsafe { log_ahci_cmd_slots(port, cmd_list) };
            }
        }
        port += 1;
    }
}

unsafe fn log_ahci_cmd_slots(port: usize, cmd_list: u64) {
    const AHCI_CMD_HDR_SZ: usize = 0x20;
    let mut slot = 0usize;
    while slot < 4 {
        let hdr = cmd_list + (slot * AHCI_CMD_HDR_SZ) as u64;
        let opts = unsafe { ahci_read_phys_u32(hdr, 0x00) };
        let status = unsafe { ahci_read_phys_u32(hdr, 0x04) };
        let tbl_addr = unsafe { ahci_read_phys_u32(hdr, 0x08) };
        let tbl_addr_hi = unsafe { ahci_read_phys_u32(hdr, 0x0c) };
        if opts != 0 || status != 0 || tbl_addr != 0 || tbl_addr_hi != 0 {
            let table = ((tbl_addr_hi as u64) << 32) | tbl_addr as u64;
            let fis0 = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x00) }
            };
            let fis1 = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x04) }
            };
            let fis2 = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x08) }
            };
            let fis3 = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x0c) }
            };
            let fis4 = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x10) }
            };
            let prdt_addr = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x80) }
            };
            let prdt_addr_hi = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x84) }
            };
            let prdt_flags = if table == 0 {
                0
            } else {
                unsafe { ahci_read_phys_u32(table, 0x8c) }
            };
            crate::log_warn!(
                "ahci",
                "bar5 port{} slot{}: opts=0x{:08x} status=0x{:08x} table=0x{:016x} fis=[{:08x} {:08x} {:08x} {:08x} {:08x}] prdt=0x{:08x}{:08x}/0x{:08x}",
                port,
                slot,
                opts,
                status,
                table,
                fis0,
                fis1,
                fis2,
                fis3,
                fis4,
                prdt_addr_hi,
                prdt_addr,
                prdt_flags
            );
        }
        slot += 1;
    }
}

pub fn debug_dump_ahci_bar5(reason: &str) {
    let mmio = AHCI_BAR5_MMIO.load(Ordering::Acquire) as *mut c_void;
    if mmio.is_null() {
        return;
    }
    crate::log_warn!("ahci", "bar5 debug dump: {}", reason);
    unsafe { log_ahci_bar5_registers(core::ptr::null_mut(), mmio) };
}

unsafe extern "C" fn linux_pcim_iomap(dev: *mut c_void, bar: i32, maxlen: usize) -> *mut c_void {
    let mapped = unsafe { crate::linux_driver_abi::pci::iomap::pci_iomap(dev, bar, maxlen) };
    if mapped.is_null() {
        crate::log_warn!(
            "pci",
            "pcim_iomap: dev={:p} bar={} maxlen={} returned null",
            dev,
            bar,
            maxlen
        );
    } else if let Ok(index) = usize::try_from(bar)
        && let Some(resource) =
            crate::linux_driver_abi::pci::device::linux_pci_bar_resource(dev.cast_const(), index)
    {
        crate::log_warn!(
            "pci",
            "pcim_iomap: dev={:p} bar={} start=0x{:x} len=0x{:x} mapped={:p}",
            dev,
            bar,
            resource.start,
            resource.len,
            mapped
        );
        if index == 5 {
            AHCI_BAR5_MMIO.store(mapped as usize, Ordering::Release);
            unsafe { log_ahci_bar5_registers(dev, mapped) };
        }
    }
    if !mapped.is_null()
        && let Ok(index) = usize::try_from(bar)
        && index < 6
    {
        let table = pcim_iomap_entry(dev);
        unsafe { (*table)[index] = mapped as usize };
    }
    mapped
}

unsafe extern "C" fn linux_pcim_iomap_region(
    dev: *mut c_void,
    bar: i32,
    name: *const c_char,
) -> *mut c_void {
    let ret = unsafe { crate::linux_driver_abi::pci::pci::pci_request_region(dev, bar, name) };
    if ret != 0 {
        return linux_error_ptr(-ret);
    }

    let mapped = unsafe { linux_pcim_iomap(dev, bar, 0) };
    if mapped.is_null() {
        unsafe { crate::linux_driver_abi::pci::pci::pci_release_region(dev, bar) };
        return linux_error_ptr(EINVAL);
    }
    mapped
}

unsafe extern "C" fn linux_pcim_iomap_regions(
    dev: *mut c_void,
    mask: i32,
    _name: *const c_char,
) -> i32 {
    for bar in 0..6 {
        if mask & (1 << bar) != 0 {
            let _ = unsafe { linux_pcim_iomap(dev, bar, 0) };
        }
    }
    0
}

unsafe extern "C" fn linux_pcim_iomap_table(dev: *mut c_void) -> *mut *mut c_void {
    pcim_iomap_entry(dev).cast::<*mut c_void>()
}

unsafe extern "C" fn linux_pcim_request_all_regions(dev: *mut c_void, name: *const c_char) -> i32 {
    crate::log_warn!(
        "pci",
        "pcim_request_all_regions: dev={:p} name={:p} ok",
        dev,
        name
    );
    0
}

fn pci_write_config(dev: *const c_void, offset: i32, width: usize, value: u32) -> i32 {
    if dev.is_null() {
        return crate::linux_driver_abi::pci::access::PCIBIOS_DEVICE_NOT_FOUND;
    }
    let Ok(offset) = usize::try_from(offset) else {
        return crate::linux_driver_abi::pci::access::PCIBIOS_BAD_REGISTER_NUMBER;
    };
    if crate::linux_driver_abi::pci::device::linux_pci_config_write(dev, offset, width, value) {
        crate::linux_driver_abi::pci::access::PCIBIOS_SUCCESSFUL
    } else {
        crate::linux_driver_abi::pci::access::PCIBIOS_BAD_REGISTER_NUMBER
    }
}

unsafe extern "C" fn linux_pci_write_config_byte(
    dev: *const c_void,
    offset: i32,
    value: u8,
) -> i32 {
    pci_write_config(dev, offset, 1, value as u32)
}

unsafe extern "C" fn linux_pci_write_config_word(
    dev: *const c_void,
    offset: i32,
    value: u16,
) -> i32 {
    pci_write_config(dev, offset, 2, value as u32)
}

unsafe extern "C" fn linux_pci_write_config_dword(
    dev: *const c_void,
    offset: i32,
    value: u32,
) -> i32 {
    pci_write_config(dev, offset, 4, value)
}

unsafe extern "C" fn linux_transport_setup_device(_dev: *mut LinuxDevice) {}
unsafe extern "C" fn linux_transport_add_device(_dev: *mut LinuxDevice) -> i32 {
    crate::log_info!("transport", "transport_add_device: dev={:p}", _dev);
    0
}
unsafe extern "C" fn linux_transport_configure_device(_dev: *mut LinuxDevice) {}
unsafe extern "C" fn linux_transport_remove_device(_dev: *mut LinuxDevice) {}
unsafe extern "C" fn linux_transport_destroy_device(_dev: *mut LinuxDevice) {}
unsafe extern "C" fn linux_transport_class_register(_class: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn linux_transport_class_unregister(_class: *mut c_void) {}
unsafe extern "C" fn linux_attribute_container_register(_container: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn linux_attribute_container_unregister(_container: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn linux_anon_transport_class_register(_atc: *mut c_void) {}
unsafe extern "C" fn linux_anon_transport_class_unregister(_atc: *mut c_void) {}

type AsyncFunc = unsafe extern "C" fn(*mut c_void, u64);

unsafe extern "C" fn linux_async_schedule_node(
    func: Option<AsyncFunc>,
    data: *mut c_void,
    _node: i32,
) -> u64 {
    static NEXT_COOKIE: AtomicUsize = AtomicUsize::new(1);
    let cookie = NEXT_COOKIE.fetch_add(1, Ordering::AcqRel) as u64;
    if let Some(func) = func {
        unsafe { func(data, cookie) };
    }
    cookie
}

unsafe extern "C" fn linux_async_synchronize_cookie(_cookie: u64) {}

fn parse_u64_bytes(mut bytes: &[u8], base: u32) -> Result<u64, i32> {
    while matches!(bytes.first(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        bytes = &bytes[1..];
    }
    let mut end = bytes.len();
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t' | b'\n' | b'\r') {
        end -= 1;
    }
    bytes = &bytes[..end];
    if bytes.is_empty() {
        return Err(EINVAL);
    }
    let mut radix = if base == 0 { 10 } else { base };
    if bytes.len() > 2 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X') {
        radix = 16;
        bytes = &bytes[2..];
    } else if base == 0 && bytes.len() > 1 && bytes[0] == b'0' {
        radix = 8;
        bytes = &bytes[1..];
    }
    if !(2..=36).contains(&radix) {
        return Err(EINVAL);
    }
    let mut value = 0u64;
    let mut parsed = false;
    for &byte in bytes {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'a'..=b'z' => (byte - b'a' + 10) as u32,
            b'A'..=b'Z' => (byte - b'A' + 10) as u32,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        parsed = true;
        value = value
            .checked_mul(radix as u64)
            .and_then(|v| v.checked_add(digit as u64))
            .ok_or(EINVAL)?;
    }
    parsed.then_some(value).ok_or(EINVAL)
}

unsafe extern "C" fn linux_kstrtoull(s: *const c_char, base: u32, res: *mut u64) -> i32 {
    let Some(bytes) = (unsafe { c_str_bytes(s, 128) }) else {
        return -EINVAL;
    };
    match parse_u64_bytes(bytes, base) {
        Ok(value) => {
            if !res.is_null() {
                unsafe { *res = value };
            }
            0
        }
        Err(err) => -err,
    }
}

unsafe extern "C" fn linux_kstrtouint(s: *const c_char, base: u32, res: *mut u32) -> i32 {
    let mut value = 0u64;
    let ret = unsafe { linux_kstrtoull(s, base, &mut value) };
    if ret == 0 && value <= u32::MAX as u64 {
        if !res.is_null() {
            unsafe { *res = value as u32 };
        }
        0
    } else {
        -EINVAL
    }
}

unsafe extern "C" fn linux_kstrtou16(s: *const c_char, base: u32, res: *mut u16) -> i32 {
    let mut value = 0u64;
    let ret = unsafe { linux_kstrtoull(s, base, &mut value) };
    if ret < 0 {
        return ret;
    }
    if value > u16::MAX as u64 {
        return -ERANGE;
    }
    if !res.is_null() {
        unsafe { *res = value as u16 };
    }
    0
}

unsafe extern "C" fn linux_kstrtou8(s: *const c_char, base: u32, res: *mut u8) -> i32 {
    let mut value = 0u64;
    let ret = unsafe { linux_kstrtoull(s, base, &mut value) };
    if ret < 0 {
        return ret;
    }
    if value > u8::MAX as u64 {
        return -ERANGE;
    }
    if !res.is_null() {
        unsafe { *res = value as u8 };
    }
    0
}

unsafe extern "C" fn linux_kstrtoint(s: *const c_char, base: u32, res: *mut i32) -> i32 {
    if s.is_null() {
        return -EINVAL;
    }
    let negative = unsafe { *s == b'-' as c_char };
    let parse_ptr = if negative { unsafe { s.add(1) } } else { s };
    let mut value = 0u64;
    let ret = unsafe { linux_kstrtoull(parse_ptr, base, &mut value) };
    if ret != 0 || value > i32::MAX as u64 + negative as u64 {
        return -EINVAL;
    }
    if !res.is_null() {
        unsafe {
            *res = if negative {
                -(value as i64) as i32
            } else {
                value as i32
            };
        }
    }
    0
}

unsafe extern "C" fn linux_kstrtobool(s: *const c_char, res: *mut bool) -> i32 {
    let Some(bytes) = (unsafe { c_str_bytes(s, 16) }) else {
        return -EINVAL;
    };
    let value = match bytes.first().copied() {
        Some(b'y' | b'Y' | b'1') => true,
        Some(b'n' | b'N' | b'0') => false,
        Some(b'o' | b'O') if bytes.len() >= 2 && matches!(bytes[1], b'n' | b'N') => true,
        Some(b'o' | b'O') if bytes.len() >= 3 && matches!(bytes[1], b'f' | b'F') => false,
        _ => return -EINVAL,
    };
    if !res.is_null() {
        unsafe { *res = value };
    }
    0
}

unsafe extern "C" fn linux_kstrdup(src: *const c_char, gfp: u32) -> *mut c_char {
    unsafe { crate::mm::util::kstrdup(src.cast::<u8>(), gfp).cast::<c_char>() }
}

unsafe extern "C" fn linux_simple_strtoull(
    cp: *const c_char,
    endp: *mut *mut c_char,
    base: u32,
) -> u64 {
    let Some(bytes) = (unsafe { c_str_bytes(cp, 128) }) else {
        return 0;
    };
    let value = parse_u64_bytes(bytes, base).unwrap_or(0);
    if !endp.is_null() {
        unsafe { *endp = cp.add(bytes.len()).cast_mut() };
    }
    value
}

unsafe extern "C" fn linux_simple_strtoul(
    cp: *const c_char,
    endp: *mut *mut c_char,
    base: u32,
) -> u64 {
    unsafe { linux_simple_strtoull(cp, endp, base) }
}

unsafe extern "C" fn linux_simple_strtol(
    cp: *const c_char,
    endp: *mut *mut c_char,
    base: u32,
) -> i64 {
    if cp.is_null() {
        return 0;
    }
    if unsafe { *cp.cast::<u8>() } == b'-' {
        let value = unsafe { linux_simple_strtoul(cp.add(1), endp, base) };
        -(value as i64)
    } else {
        unsafe { linux_simple_strtoul(cp, endp, base) as i64 }
    }
}

unsafe extern "C" fn linux_sysfs_streq(s1: *const c_char, s2: *const c_char) -> bool {
    let Some(mut a) = (unsafe { c_str_bytes(s1, 512) }) else {
        return false;
    };
    let Some(mut b) = (unsafe { c_str_bytes(s2, 512) }) else {
        return false;
    };
    if a.ends_with(b"\n") {
        a = &a[..a.len() - 1];
    }
    if b.ends_with(b"\n") {
        b = &b[..b.len() - 1];
    }
    a == b
}

unsafe extern "C" fn linux_strim(s: *mut c_char) -> *mut c_char {
    if s.is_null() {
        return s;
    }
    let Some(len) = (unsafe { c_str_len(s, 4096) }) else {
        return s;
    };
    let bytes = unsafe { core::slice::from_raw_parts_mut(s.cast::<u8>(), len) };
    let mut start = 0usize;
    while start < bytes.len() && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    let mut end = bytes.len();
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    if end < bytes.len() {
        bytes[end] = 0;
    }
    unsafe { s.add(start) }
}

unsafe extern "C" fn linux_memory_read_from_buffer(
    to: *mut c_void,
    count: usize,
    ppos: *mut i64,
    from: *const c_void,
    available: usize,
) -> isize {
    if to.is_null() || from.is_null() {
        return -EINVAL as isize;
    }
    let pos = if ppos.is_null() {
        0usize
    } else {
        let raw = unsafe { *ppos };
        if raw < 0 {
            return -EINVAL as isize;
        }
        raw as usize
    };
    if pos >= available {
        return 0;
    }
    let n = core::cmp::min(count, available - pos);
    unsafe {
        ptr::copy_nonoverlapping(from.cast::<u8>().add(pos), to.cast::<u8>(), n);
        if !ppos.is_null() {
            *ppos = ppos.read().saturating_add(n as i64);
        }
    }
    n as isize
}

unsafe extern "C" fn linux_hex_dump_to_buffer(
    _buf: *const c_void,
    _len: usize,
    _rowsize: i32,
    _groupsize: i32,
    linebuf: *mut c_char,
    linebuflen: usize,
    _ascii: bool,
) -> i32 {
    if !linebuf.is_null() && linebuflen != 0 {
        unsafe { *linebuf = 0 };
    }
    0
}

unsafe extern "C" fn linux_memcpy_and_pad(
    dest: *mut c_void,
    dest_len: usize,
    src: *const c_void,
    count: usize,
    pad: i32,
) {
    if dest.is_null() {
        return;
    }
    let copy = core::cmp::min(dest_len, count);
    if !src.is_null() && copy != 0 {
        unsafe { ptr::copy_nonoverlapping(src.cast::<u8>(), dest.cast::<u8>(), copy) };
    }
    if dest_len > copy {
        unsafe { ptr::write_bytes(dest.cast::<u8>().add(copy), pad as u8, dest_len - copy) };
    }
}

unsafe fn format_args(buf: *mut u8, size: usize, fmt: *const c_char, args: &[usize]) -> i32 {
    if buf.is_null() || size == 0 || fmt.is_null() {
        return 0;
    }
    let fmt_bytes = unsafe { c_str_bytes(fmt, 512) }.unwrap_or(b"");
    let mut written = 0usize;
    let mut i = 0usize;
    let mut arg_index = 0usize;
    while i < fmt_bytes.len() && written + 1 < size {
        if fmt_bytes[i] == b'%' && i + 1 < fmt_bytes.len() {
            i += 1;
            if fmt_bytes[i] == b'%' {
                write_byte(buf, size, &mut written, b'%');
                i += 1;
                continue;
            }

            let mut alternate = false;
            let mut zero_pad = false;
            if fmt_bytes[i] == b'#' {
                alternate = true;
                i += 1;
            }
            if i < fmt_bytes.len() && fmt_bytes[i] == b'0' {
                zero_pad = true;
                i += 1;
            }

            let mut width = 0usize;
            while i < fmt_bytes.len() && fmt_bytes[i].is_ascii_digit() {
                width = width
                    .saturating_mul(10)
                    .saturating_add((fmt_bytes[i] - b'0') as usize);
                i += 1;
            }

            while i < fmt_bytes.len() && matches!(fmt_bytes[i], b'h' | b'l' | b'L' | b'z' | b't') {
                i += 1;
            }
            if i >= fmt_bytes.len() {
                break;
            }

            let arg = if matches!(
                fmt_bytes[i],
                b's' | b'd' | b'i' | b'u' | b'x' | b'X' | b'p' | b'c'
            ) {
                let value = args.get(arg_index).copied().unwrap_or(0);
                arg_index += 1;
                value
            } else {
                0
            };

            match fmt_bytes[i] {
                b's' => {
                    let value =
                        unsafe { c_str_bytes(arg as *const c_char, 512) }.unwrap_or(b"(null)");
                    write_padded_bytes(buf, size, &mut written, value, width, false);
                }
                b'd' | b'i' => {
                    let mut tmp = [0u8; 32];
                    let value = arg as isize;
                    let negative = value < 0;
                    let magnitude = if negative {
                        value.wrapping_neg() as usize
                    } else {
                        value as usize
                    };
                    let len = format_usize(magnitude, 10, false, &mut tmp);
                    write_number(
                        buf,
                        size,
                        &mut written,
                        &tmp[..len],
                        negative,
                        b"",
                        width,
                        zero_pad,
                    );
                }
                b'u' => {
                    let mut tmp = [0u8; 32];
                    let len = format_usize(arg, 10, false, &mut tmp);
                    write_number(
                        buf,
                        size,
                        &mut written,
                        &tmp[..len],
                        false,
                        b"",
                        width,
                        zero_pad,
                    );
                }
                b'x' | b'X' => {
                    let mut tmp = [0u8; 32];
                    let upper = fmt_bytes[i] == b'X';
                    let len = format_usize(arg, 16, upper, &mut tmp);
                    let prefix = if alternate { b"0x".as_slice() } else { b"" };
                    write_number(
                        buf,
                        size,
                        &mut written,
                        &tmp[..len],
                        false,
                        prefix,
                        width,
                        zero_pad,
                    );
                }
                b'p' => {
                    let mut tmp = [0u8; 32];
                    let len = format_usize(arg, 16, false, &mut tmp);
                    write_number(
                        buf,
                        size,
                        &mut written,
                        &tmp[..len],
                        false,
                        b"0x",
                        width,
                        zero_pad,
                    );
                }
                b'c' => {
                    write_byte(buf, size, &mut written, arg as u8);
                }
                other => {
                    write_byte(buf, size, &mut written, b'%');
                    write_byte(buf, size, &mut written, other);
                }
            }
        } else {
            write_byte(buf, size, &mut written, fmt_bytes[i]);
        }
        i += 1;
    }
    unsafe { *buf.add(written) = 0 };
    written as i32
}

fn write_byte(buf: *mut u8, size: usize, written: &mut usize, byte: u8) {
    if *written + 1 < size {
        unsafe { *buf.add(*written) = byte };
        *written += 1;
    }
}

fn write_padded_bytes(
    buf: *mut u8,
    size: usize,
    written: &mut usize,
    bytes: &[u8],
    width: usize,
    zero_pad: bool,
) {
    let pad = width.saturating_sub(bytes.len());
    let pad_byte = if zero_pad { b'0' } else { b' ' };
    for _ in 0..pad {
        write_byte(buf, size, written, pad_byte);
    }
    for byte in bytes.iter().copied() {
        write_byte(buf, size, written, byte);
    }
}

fn write_number(
    buf: *mut u8,
    size: usize,
    written: &mut usize,
    digits: &[u8],
    negative: bool,
    prefix: &[u8],
    width: usize,
    zero_pad: bool,
) {
    let sign_len = usize::from(negative);
    let pad = width.saturating_sub(sign_len + prefix.len() + digits.len());
    if !zero_pad {
        for _ in 0..pad {
            write_byte(buf, size, written, b' ');
        }
    }
    if negative {
        write_byte(buf, size, written, b'-');
    }
    for byte in prefix.iter().copied() {
        write_byte(buf, size, written, byte);
    }
    if zero_pad {
        for _ in 0..pad {
            write_byte(buf, size, written, b'0');
        }
    }
    for byte in digits.iter().copied() {
        write_byte(buf, size, written, byte);
    }
}

fn format_usize(mut value: usize, radix: usize, upper: bool, out: &mut [u8; 32]) -> usize {
    let mut tmp = [0u8; 32];
    let mut len = 0usize;
    loop {
        let digit = value % radix;
        tmp[len] = if digit < 10 {
            b'0' + digit as u8
        } else if upper {
            b'A' + (digit as u8 - 10)
        } else {
            b'a' + (digit as u8 - 10)
        };
        len += 1;
        value /= radix;
        if value == 0 {
            break;
        }
    }
    for idx in 0..len {
        out[idx] = tmp[len - 1 - idx];
    }
    len
}

unsafe extern "C" fn linux_vscnprintf(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    arg0: usize,
) -> i32 {
    unsafe { format_args(buf.cast::<u8>(), size, fmt, &[arg0]) }
}

unsafe extern "C" fn linux_scnprintf(
    buf: *mut c_char,
    size: usize,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> i32 {
    unsafe { format_args(buf.cast::<u8>(), size, fmt, &[arg0, arg1, arg2, arg3]) }
}

unsafe extern "C" fn linux_sscanf(_buf: *const c_char, _fmt: *const c_char, _arg0: usize) -> i32 {
    0
}

unsafe extern "C" fn linux_read_cache_folio(_mapping: *mut c_void, _index: u64) -> *mut c_void {
    linux_error_ptr(ENOMEM)
}

unsafe extern "C" fn linux_refcount_warn_saturate(_refcount: *mut c_void, _type_: i32) {}

unsafe extern "C" fn linux_ida_destroy(_ida: *mut c_void) {}

#[repr(C)]
struct LinuxSbitmapWord {
    word: usize,
    cleared: usize,
}

#[repr(C)]
struct LinuxSbitmap {
    depth: u32,
    shift: u32,
    map_nr: u32,
    round_robin: bool,
    _pad_after_round_robin: [u8; 3],
    map: *mut LinuxSbitmapWord,
    alloc_hint: *mut c_void,
}

unsafe extern "C" fn linux_sbitmap_init_node(
    sb: *mut c_void,
    _depth: u32,
    _shift: i32,
    _flags: u32,
    _node: i32,
    _round_robin: bool,
    _alloc_hint: bool,
) -> i32 {
    if !sb.is_null() {
        unsafe { ptr::write_bytes(sb.cast::<LinuxSbitmap>(), 0, 1) };
    }
    0
}

unsafe extern "C" fn linux_sbitmap_resize(_sb: *mut c_void, _depth: u32) -> i32 {
    0
}
unsafe extern "C" fn linux_sbitmap_get(_sb: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn linux_sbitmap_weight(_sb: *const c_void) -> u32 {
    0
}

unsafe extern "C" fn linux___devres_alloc_node(
    release: *mut c_void,
    size: usize,
    gfp: u32,
    _node: i32,
    _name: *const c_char,
) -> *mut c_void {
    let total = size.saturating_add(core::mem::size_of::<usize>());
    let ptr = unsafe { crate::mm::slab::kmalloc(total, gfp) };
    if ptr.is_null() {
        crate::log_warn!(
            "devres",
            "__devres_alloc_node: release={:p} size={} total={} returned null",
            release,
            size,
            total
        );
        return ptr::null_mut();
    }
    crate::log_warn!(
        "devres",
        "__devres_alloc_node: release={:p} size={} total={} ptr={:p}",
        release,
        size,
        total,
        ptr
    );
    unsafe {
        *(ptr as *mut usize) = release as usize;
        ptr.add(core::mem::size_of::<usize>()).cast::<c_void>()
    }
}

unsafe extern "C" fn linux_devres_add(_dev: *mut LinuxDevice, _res: *mut c_void) {}

unsafe extern "C" fn linux_devres_find(
    _dev: *mut LinuxDevice,
    _release: *mut c_void,
    _match_: *mut c_void,
    _match_data: *mut c_void,
) -> *mut c_void {
    ptr::null_mut()
}

unsafe extern "C" fn linux_devres_remove(
    dev: *mut LinuxDevice,
    release: *mut c_void,
    match_: *mut c_void,
    match_data: *mut c_void,
) -> *mut c_void {
    unsafe { linux_devres_find(dev, release, match_, match_data) }
}

unsafe extern "C" fn linux_devres_destroy(
    dev: *mut LinuxDevice,
    release: *mut c_void,
    match_: *mut c_void,
    match_data: *mut c_void,
) -> i32 {
    let res = unsafe { linux_devres_remove(dev, release, match_, match_data) };
    if res.is_null() {
        return -ENOENT;
    }
    unsafe { linux_devres_free(res) };
    0
}

unsafe extern "C" fn linux_devres_release(
    dev: *mut LinuxDevice,
    release: Option<unsafe extern "C" fn(*mut LinuxDevice, *mut c_void)>,
    match_: *mut c_void,
    match_data: *mut c_void,
) -> i32 {
    let res = unsafe {
        linux_devres_remove(
            dev,
            release.map(|f| f as usize).unwrap_or(0) as *mut c_void,
            match_,
            match_data,
        )
    };
    if res.is_null() {
        return -ENOENT;
    }
    if let Some(release) = release {
        unsafe { release(dev, res) };
    }
    unsafe { linux_devres_free(res) };
    0
}

unsafe extern "C" fn linux_devres_free(res: *mut c_void) {
    if res.is_null() {
        return;
    }
    let base = unsafe { res.cast::<u8>().sub(core::mem::size_of::<usize>()) };
    unsafe { crate::mm::slab::kfree(base) };
}

unsafe extern "C" fn linux_devres_open_group(
    _dev: *mut LinuxDevice,
    _id: *mut c_void,
    _gfp: u32,
) -> *mut c_void {
    core::ptr::dangling_mut::<c_void>()
}

unsafe extern "C" fn linux_devres_release_group(_dev: *mut LinuxDevice, _id: *mut c_void) -> i32 {
    0
}

unsafe extern "C" fn linux_devres_remove_group(_dev: *mut LinuxDevice, _id: *mut c_void) {}

unsafe extern "C" fn linux___devm_add_action(
    dev: *mut LinuxDevice,
    action: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
    _name: *const c_char,
) -> i32 {
    let Some(action) = action else {
        return -EINVAL;
    };
    let mut actions = DEVM_ACTIONS.lock();
    if actions.try_reserve_exact(1).is_err() {
        return -ENOMEM;
    }
    actions.push(DevmActionEntry {
        dev: dev as usize,
        action: action as usize,
        data: data as usize,
    });
    0
}

fn remove_devm_action(
    dev: *mut LinuxDevice,
    action: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) -> Option<DevmActionEntry> {
    let action = action? as usize;
    let mut actions = DEVM_ACTIONS.lock();
    let index = actions.iter().rposition(|entry| {
        entry.dev == dev as usize && entry.action == action && entry.data == data as usize
    })?;
    Some(actions.swap_remove(index))
}

unsafe extern "C" fn linux_devm_remove_action_nowarn(
    dev: *mut LinuxDevice,
    action: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) -> i32 {
    if remove_devm_action(dev, action, data).is_some() {
        0
    } else {
        -ENOENT
    }
}

unsafe extern "C" fn linux_devm_release_action(
    dev: *mut LinuxDevice,
    action: Option<unsafe extern "C" fn(*mut c_void)>,
    data: *mut c_void,
) {
    let Some(entry) = remove_devm_action(dev, action, data) else {
        return;
    };
    if let Some(action) = action {
        unsafe { action(entry.data as *mut c_void) };
    }
}

unsafe extern "C" fn linux_devm_kmalloc(
    dev: *mut LinuxDevice,
    size: usize,
    gfp: u32,
) -> *mut c_void {
    let ptr = unsafe { crate::mm::slab::kmalloc(size, gfp).cast::<c_void>() };
    if ptr.is_null() {
        crate::log_warn!(
            "devres",
            "devm_kmalloc: dev={:p} size={} returned null",
            dev,
            size
        );
    } else {
        crate::log_warn!(
            "devres",
            "devm_kmalloc: dev={:p} size={} ptr={:p}",
            dev,
            size,
            ptr
        );
    }
    ptr
}

unsafe extern "C" fn linux_devm_kfree(_dev: *mut LinuxDevice, ptr: *mut c_void) {
    unsafe { crate::mm::slab::kfree(ptr.cast::<u8>()) };
}

unsafe extern "C" fn linux_devm_kasprintf(
    dev: *mut LinuxDevice,
    gfp: u32,
    fmt: *const c_char,
    arg0: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
) -> *mut c_char {
    let ptr = unsafe { linux_devm_kmalloc(dev, 128, gfp).cast::<c_char>() };
    if ptr.is_null() {
        crate::log_warn!("devres", "devm_kasprintf: allocation returned null");
        return ptr;
    }
    unsafe {
        let _ = format_args(ptr.cast::<u8>(), 128, fmt, &[arg0, arg1, arg2, arg3]);
    }
    crate::log_info!("devres", "devm_kasprintf: ptr={:p}", ptr);
    ptr
}

unsafe extern "C" fn linux_devm_request_threaded_irq(
    dev: *mut LinuxDevice,
    irq: u32,
    handler: Option<unsafe extern "C" fn(u32, *mut c_void) -> i32>,
    thread_fn: Option<unsafe extern "C" fn(u32, *mut c_void) -> i32>,
    flags: u64,
    name: *const c_char,
    dev_id: *mut c_void,
) -> i32 {
    let _ = dev;
    if AHCI_BAR5_MMIO.load(Ordering::Acquire) != 0 {
        AHCI_LEGACY_IRQ.store(irq as usize, Ordering::Release);
        AHCI_IRQ_DEV_ID.store(dev_id as usize, Ordering::Release);
        // Resolve ahci_handle_port_intr ONCE here, at AHCI IRQ registration —
        // i.e. during module probe, before any block I/O runs. The hot
        // completion-poll path must never call find_symbol(): it can be reached
        // from the idle pump / block wait loop while other modules are still
        // registering exported symbols, and iterating the symbol table there
        // raced into a corrupted entry (a String with a bogus name pointer) →
        // memcmp on a bad pointer → #UD. Caching the address now keeps the poll
        // free of any symbol-table walk.
        if AHCI_PORT_INTR_FN.load(Ordering::Acquire) == 0 {
            if let Some(addr) = find_symbol("ahci_handle_port_intr") {
                AHCI_PORT_INTR_FN.store(addr, Ordering::Release);
            }
        }
        let mmio = AHCI_BAR5_MMIO.load(Ordering::Acquire) as *mut c_void;
        unsafe { ahci_enable_interrupts(mmio) };
    }
    let ret = unsafe {
        crate::kernel::irq::manage::linux_request_threaded_irq(
            irq,
            handler,
            thread_fn,
            flags as usize,
            name,
            dev_id,
        )
    };
    if ret != 0 {
        crate::log_warn!(
            "irq",
            "devm_request_threaded_irq: dev={:p} irq={} errno {}",
            dev,
            irq,
            ret
        );
    } else {
        crate::log_info!(
            "irq",
            "devm_request_threaded_irq: dev={:p} irq={} flags=0x{:x} dev_id={:p} ok",
            dev,
            irq,
            flags,
            dev_id
        );
    }
    ret
}

unsafe extern "C" fn linux_devm_free_irq(_dev: *mut LinuxDevice, irq: u32, dev_id: *mut c_void) {
    unsafe {
        let _ = crate::kernel::irq::manage::linux_free_irq(irq, dev_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{offset_of, size_of};

    #[test]
    fn storage_core_exports_representative_symbols() {
        register_module_exports();
        assert_eq!(
            crate::kernel::module::find_symbol("blk_mq_alloc_queue"),
            Some(linux_blk_mq_alloc_queue as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcim_iomap_table"),
            Some(linux_pcim_iomap_table as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("pcim_iomap_region"),
            Some(linux_pcim_iomap_region as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("devres_destroy"),
            Some(linux_devres_destroy as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("devm_release_action"),
            Some(linux_devm_release_action as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("device_create_file"),
            Some(linux_device_create_file as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("sg_miter_skip"),
            Some(linux_sg_miter_skip as usize)
        );
        assert_eq!(
            crate::kernel::module::find_symbol("root-only-never-present"),
            None
        );
    }

    static DEVM_ACTION_TEST_VALUE: AtomicUsize = AtomicUsize::new(0);

    unsafe extern "C" fn record_devm_action(data: *mut c_void) {
        DEVM_ACTION_TEST_VALUE.store(data as usize, Ordering::SeqCst);
    }

    #[test]
    fn devm_release_action_runs_latest_matching_action_once() {
        DEVM_ACTION_TEST_VALUE.store(0, Ordering::SeqCst);
        let mut dev = unsafe { core::mem::zeroed::<LinuxDevice>() };
        let dev = core::ptr::addr_of_mut!(dev);
        unsafe {
            assert_eq!(
                linux___devm_add_action(
                    dev,
                    Some(record_devm_action),
                    0xabcusize as *mut c_void,
                    core::ptr::null()
                ),
                0
            );
            linux_devm_release_action(dev, Some(record_devm_action), 0xabcusize as *mut c_void);
            assert_eq!(DEVM_ACTION_TEST_VALUE.load(Ordering::SeqCst), 0xabc);
            assert_eq!(
                linux_devm_remove_action_nowarn(
                    dev,
                    Some(record_devm_action),
                    0xabcusize as *mut c_void
                ),
                -ENOENT
            );
        }
    }

    #[test]
    fn kstrtouint_parses_decimal() {
        let input = b"42\n\0";
        let mut out = 0u32;
        assert_eq!(
            unsafe { linux_kstrtouint(input.as_ptr().cast(), 10, &mut out) },
            0
        );
        assert_eq!(out, 42);
    }

    #[test]
    fn narrow_kstrtou_helpers_report_range() {
        let input = b"65535\n\0";
        let mut out16 = 0u16;
        assert_eq!(
            unsafe { linux_kstrtou16(input.as_ptr().cast(), 10, &mut out16) },
            0
        );
        assert_eq!(out16, u16::MAX);

        let too_wide = b"256\n\0";
        let mut out8 = 0u8;
        assert_eq!(
            unsafe { linux_kstrtou8(too_wide.as_ptr().cast(), 10, &mut out8) },
            -ERANGE
        );
    }

    #[test]
    fn storage_core_layout_prefixes_match_vendor_headers() {
        assert_eq!(offset_of!(LinuxWaitQueueHead, head_next), 0);
        assert_eq!(offset_of!(LinuxWaitQueueHead, head_prev), 8);
        assert_eq!(size_of::<LinuxWaitQueueHead>(), 0x10);

        assert_eq!(offset_of!(LinuxWaitQueueEntry, flags), 0);
        assert_eq!(offset_of!(LinuxWaitQueueEntry, private), 0x8);
        assert_eq!(offset_of!(LinuxWaitQueueEntry, func), 0x10);
        assert_eq!(offset_of!(LinuxWaitQueueEntry, entry_next), 0x18);
        assert_eq!(offset_of!(LinuxWaitQueueEntry, entry_prev), 0x20);
        assert_eq!(
            size_of::<LinuxWaitQueueEntry>(),
            LINUX_WAIT_QUEUE_ENTRY_SIZE
        );

        assert_eq!(offset_of!(LinuxTimerList, entry_next), 0);
        assert_eq!(offset_of!(LinuxTimerList, entry_prev), 8);
        assert_eq!(offset_of!(LinuxTimerList, expires), 0x10);
        assert_eq!(offset_of!(LinuxTimerList, function), 0x18);
        assert_eq!(
            offset_of!(LinuxTimerList, flags),
            LINUX_TIMER_LIST_FLAGS_OFFSET
        );
        assert_eq!(size_of::<LinuxTimerList>(), LINUX_TIMER_LIST_SIZE);

        assert_eq!(offset_of!(LinuxScatterList, page_link), 0);
        assert_eq!(
            offset_of!(LinuxScatterList, offset),
            LINUX_SCATTERLIST_OFFSET_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxScatterList, length),
            LINUX_SCATTERLIST_LENGTH_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxScatterList, dma_address),
            LINUX_SCATTERLIST_DMA_ADDRESS_OFFSET
        );
        assert_eq!(
            offset_of!(LinuxScatterList, dma_length),
            LINUX_SCATTERLIST_DMA_LENGTH_OFFSET
        );
        assert_eq!(size_of::<LinuxScatterList>(), LINUX_SCATTERLIST_SIZE);

        assert_eq!(
            offset_of!(crate::lib::scatterlist::LinuxSgTable, sgl),
            LINUX_SG_TABLE_SGL_OFFSET
        );
        assert_eq!(
            offset_of!(crate::lib::scatterlist::LinuxSgTable, nents),
            LINUX_SG_TABLE_NENTS_OFFSET
        );
        assert_eq!(
            offset_of!(crate::lib::scatterlist::LinuxSgTable, orig_nents),
            LINUX_SG_TABLE_ORIG_NENTS_OFFSET
        );
        assert_eq!(
            size_of::<crate::lib::scatterlist::LinuxSgTable>(),
            LINUX_SG_TABLE_SIZE
        );

        assert_eq!(offset_of!(LinuxSgPageIter, sg), 0);
        assert_eq!(offset_of!(LinuxSgPageIter, sg_pgoffset), 0x8);
        assert_eq!(offset_of!(LinuxSgPageIter, nents), 0xc);
        assert_eq!(offset_of!(LinuxSgPageIter, pg_advance), 0x10);
        assert_eq!(size_of::<LinuxSgPageIter>(), LINUX_SG_PAGE_ITER_SIZE);

        assert_eq!(offset_of!(LinuxSgMappingIter, page), 0);
        assert_eq!(offset_of!(LinuxSgMappingIter, addr), 0x8);
        assert_eq!(offset_of!(LinuxSgMappingIter, length), 0x10);
        assert_eq!(offset_of!(LinuxSgMappingIter, consumed), 0x18);
        assert_eq!(offset_of!(LinuxSgMappingIter, piter), 0x20);
        assert_eq!(offset_of!(LinuxSgMappingIter, offset), 0x38);
        assert_eq!(offset_of!(LinuxSgMappingIter, remaining), 0x3c);
        assert_eq!(offset_of!(LinuxSgMappingIter, flags), 0x40);
        assert_eq!(size_of::<LinuxSgMappingIter>(), 0x48);

        assert_eq!(offset_of!(LinuxSbitmapWord, word), 0);
        assert_eq!(offset_of!(LinuxSbitmapWord, cleared), 0x8);
        assert_eq!(size_of::<LinuxSbitmapWord>(), LINUX_SBITMAP_WORD_SIZE);

        assert_eq!(offset_of!(LinuxSbitmap, depth), 0);
        assert_eq!(offset_of!(LinuxSbitmap, shift), 0x4);
        assert_eq!(offset_of!(LinuxSbitmap, map_nr), 0x8);
        assert_eq!(offset_of!(LinuxSbitmap, round_robin), 0xc);
        assert_eq!(offset_of!(LinuxSbitmap, map), 0x10);
        assert_eq!(offset_of!(LinuxSbitmap, alloc_hint), 0x18);
        assert_eq!(size_of::<LinuxSbitmap>(), LINUX_SBITMAP_SIZE);

        // These SCSI surfaces are vendor-probed ABI anchors adjacent to the
        // block path. Lupos does not currently mirror them as Rust structs.
        assert_eq!(LINUX_SCSI_DATA_BUFFER_SIZE, 0x18);
        assert_eq!(LINUX_SCSI_DATA_BUFFER_TABLE_OFFSET, 0x0);
        assert_eq!(LINUX_SCSI_DATA_BUFFER_LENGTH_OFFSET, 0x10);
        assert_eq!(
            LINUX_SCSI_DATA_BUFFER_TABLE_OFFSET + LINUX_SG_TABLE_SIZE,
            LINUX_SCSI_DATA_BUFFER_LENGTH_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_DATA_BUFFER_LENGTH_OFFSET + size_of::<u32>(),
            LINUX_SCSI_DATA_BUFFER_SIZE - size_of::<u32>()
        );
        assert_eq!(LINUX_SCSI_CMND_SIZE, 0x128);
        assert_eq!(LINUX_SCSI_INLINE_SG_CNT, 2);
        assert_eq!(LINUX_SCSI_INLINE_PROT_SG_CNT, 1);
        assert_eq!(LINUX_SCSI_CMND_INLINE_SGL_OFFSET, LINUX_SCSI_CMND_SIZE);
        assert_eq!(LINUX_SCSI_INLINE_SGL_SIZE, 0x40);
        assert_eq!(LINUX_SCSI_AHCI_CMD_SIZE, 0x168);
        assert_eq!(LINUX_SCSI_CMND_DEVICE_OFFSET, 0x0);
        assert_eq!(LINUX_SCSI_CMND_EH_ENTRY_OFFSET, 0x8);
        assert_eq!(LINUX_SCSI_CMND_ABORT_WORK_OFFSET, 0x18);
        assert_eq!(LINUX_SCSI_CMND_RCU_OFFSET, 0x70);
        assert_eq!(LINUX_SCSI_CMND_EH_EFLAGS_OFFSET, 0x80);
        assert_eq!(LINUX_SCSI_CMND_BUDGET_TOKEN_OFFSET, 0x84);
        assert_eq!(LINUX_SCSI_CMND_JIFFIES_AT_ALLOC_OFFSET, 0x88);
        assert_eq!(LINUX_SCSI_CMND_RETRIES_OFFSET, 0x90);
        assert_eq!(LINUX_SCSI_CMND_ALLOWED_OFFSET, 0x94);
        assert_eq!(LINUX_SCSI_CMND_PROT_OP_OFFSET, 0x98);
        assert_eq!(LINUX_SCSI_CMND_PROT_TYPE_OFFSET, 0x99);
        assert_eq!(LINUX_SCSI_CMND_PROT_FLAGS_OFFSET, 0x9a);
        assert_eq!(LINUX_SCSI_CMND_SUBMITTER_OFFSET, 0x9b);
        assert_eq!(LINUX_SCSI_CMND_CMD_LEN_OFFSET, 0x9c);
        assert_eq!(LINUX_SCSI_CMND_SC_DATA_DIRECTION_OFFSET, 0xa0);
        assert_eq!(LINUX_SCSI_CMND_CMND_OFFSET, 0xa4);
        assert_eq!(LINUX_SCSI_CMND_SDB_OFFSET, 0xc8);
        assert_eq!(LINUX_SCSI_CMND_PROT_SDB_OFFSET, 0xe0);
        assert_eq!(LINUX_SCSI_CMND_UNDERFLOW_OFFSET, 0xe8);
        assert_eq!(LINUX_SCSI_CMND_TRANSFERSIZE_OFFSET, 0xec);
        assert_eq!(LINUX_SCSI_CMND_RESID_LEN_OFFSET, 0xf0);
        assert_eq!(LINUX_SCSI_CMND_SENSE_LEN_OFFSET, 0xf4);
        assert_eq!(LINUX_SCSI_CMND_SENSE_BUFFER_OFFSET, 0xf8);
        assert_eq!(LINUX_SCSI_CMND_FLAGS_OFFSET, 0x100);
        assert_eq!(LINUX_SCSI_CMND_STATE_OFFSET, 0x108);
        assert_eq!(LINUX_SCSI_CMND_EXTRA_LEN_OFFSET, 0x110);
        assert_eq!(LINUX_SCSI_CMND_HOST_SCRIBBLE_OFFSET, 0x118);
        assert_eq!(LINUX_SCSI_CMND_RESULT_OFFSET, 0x120);
        assert_eq!(
            LINUX_SCSI_CMND_PROT_FLAGS_OFFSET + size_of::<u8>(),
            LINUX_SCSI_CMND_SUBMITTER_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_CMND_SUBMITTER_OFFSET + size_of::<u8>(),
            LINUX_SCSI_CMND_CMD_LEN_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_CMND_SDB_OFFSET + LINUX_SCSI_DATA_BUFFER_SIZE,
            LINUX_SCSI_CMND_PROT_SDB_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_CMND_PROT_SDB_OFFSET + size_of::<usize>(),
            LINUX_SCSI_CMND_UNDERFLOW_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_CMND_SENSE_BUFFER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_CMND_FLAGS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_CMND_STATE_OFFSET + size_of::<usize>(),
            LINUX_SCSI_CMND_EXTRA_LEN_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_CMND_HOST_SCRIBBLE_OFFSET + size_of::<usize>(),
            LINUX_SCSI_CMND_RESULT_OFFSET
        );
        assert!(LINUX_SCSI_CMND_RESULT_OFFSET + size_of::<i32>() <= LINUX_SCSI_CMND_SIZE);
        // scsi_mq_setup_tags() starts tag_set->cmd_size with scsi_cmnd, and
        // scsi_cmd_to_rq() subtracts the block request prefix from that PDU.
        assert_eq!(LINUX_REQUEST_PDU_OFFSET, size_of::<LinuxRequest>());
        assert_eq!(LINUX_REQUEST_PDU_OFFSET + LINUX_SCSI_CMND_SIZE, 0x220);
        assert_eq!(
            LINUX_REQUEST_PDU_OFFSET + LINUX_SCSI_CMND_INLINE_SGL_OFFSET,
            0x220
        );
        assert_eq!(LINUX_REQUEST_PDU_OFFSET + LINUX_SCSI_AHCI_CMD_SIZE, 0x260);

        assert_eq!(LINUX_SCSI_DEVICE_SIZE, 0x588);
        assert_eq!(LINUX_SCSI_DEVICE_HOST_OFFSET, 0x0);
        assert_eq!(LINUX_SCSI_DEVICE_REQUEST_QUEUE_OFFSET, 0x8);
        assert_eq!(LINUX_SCSI_DEVICE_SIBLINGS_OFFSET, 0x10);
        assert_eq!(LINUX_SCSI_DEVICE_SAME_TARGET_SIBLINGS_OFFSET, 0x20);
        assert_eq!(LINUX_SCSI_DEVICE_BUDGET_MAP_OFFSET, 0x30);
        assert_eq!(LINUX_SCSI_DEVICE_DEVICE_BLOCKED_OFFSET, 0x50);
        assert_eq!(LINUX_SCSI_DEVICE_RESTARTS_OFFSET, 0x54);
        assert_eq!(LINUX_SCSI_DEVICE_STARVED_ENTRY_OFFSET, 0x58);
        assert_eq!(
            LINUX_SCSI_DEVICE_BUDGET_MAP_OFFSET + LINUX_SBITMAP_SIZE,
            LINUX_SCSI_DEVICE_DEVICE_BLOCKED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_DEVICE_RESTARTS_OFFSET + size_of::<u32>(),
            LINUX_SCSI_DEVICE_STARVED_ENTRY_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_DEVICE_STARVED_ENTRY_OFFSET + size_of::<usize>() * 2,
            LINUX_SCSI_DEVICE_QUEUE_DEPTH_OFFSET
        );
        assert_eq!(LINUX_SCSI_DEVICE_QUEUE_DEPTH_OFFSET, 0x68);
        assert_eq!(LINUX_SCSI_DEVICE_ID_OFFSET, 0x88);
        assert_eq!(LINUX_SCSI_DEVICE_CHANNEL_OFFSET, 0x8c);
        assert_eq!(LINUX_SCSI_DEVICE_LUN_OFFSET, 0x90);
        assert_eq!(LINUX_SCSI_DEVICE_SECTOR_SIZE_OFFSET, 0x9c);
        assert_eq!(LINUX_SCSI_DEVICE_HOSTDATA_OFFSET, 0xa0);
        assert_eq!(LINUX_SCSI_DEVICE_TYPE_OFFSET, 0xa8);
        assert_eq!(LINUX_SCSI_DEVICE_SDEV_TARGET_OFFSET, 0x128);
        assert_eq!(LINUX_SCSI_DEVICE_QUEUE_STOPPED_OFFSET, 0x144);
        assert_eq!(LINUX_SCSI_DEVICE_SDEV_GENDEV_OFFSET, 0x1b0);
        assert_eq!(LINUX_SCSI_DEVICE_SDEV_DEV_OFFSET, 0x360);
        assert_eq!(LINUX_SCSI_DEVICE_SDEV_STATE_OFFSET, 0x578);
        assert_eq!(LINUX_SCSI_DEVICE_SDEV_DATA_OFFSET, LINUX_SCSI_DEVICE_SIZE);
        assert_eq!(
            LINUX_SCSI_DEVICE_HOST_OFFSET + size_of::<usize>(),
            LINUX_SCSI_DEVICE_REQUEST_QUEUE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_DEVICE_ID_OFFSET + size_of::<u32>(),
            LINUX_SCSI_DEVICE_CHANNEL_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_DEVICE_CHANNEL_OFFSET + size_of::<u32>(),
            LINUX_SCSI_DEVICE_LUN_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_DEVICE_SECTOR_SIZE_OFFSET + size_of::<u32>(),
            LINUX_SCSI_DEVICE_HOSTDATA_OFFSET
        );
        assert!(LINUX_SCSI_DEVICE_SDEV_STATE_OFFSET + size_of::<i32>() <= LINUX_SCSI_DEVICE_SIZE);

        assert_eq!(LINUX_SCSI_TARGET_SIZE, 0x350);
        assert_eq!(LINUX_SCSI_TARGET_STARGET_SDEV_USER_OFFSET, 0x0);
        assert_eq!(LINUX_SCSI_TARGET_SIBLINGS_OFFSET, 0x8);
        assert_eq!(LINUX_SCSI_TARGET_DEVICES_OFFSET, 0x18);
        assert_eq!(LINUX_SCSI_TARGET_DEV_OFFSET, 0x28);
        assert_eq!(LINUX_SCSI_TARGET_REAP_REF_OFFSET, 0x320);
        assert_eq!(LINUX_SCSI_TARGET_CHANNEL_OFFSET, 0x324);
        assert_eq!(LINUX_SCSI_TARGET_ID_OFFSET, 0x328);
        assert_eq!(LINUX_SCSI_TARGET_BITFLAGS_OFFSET, 0x32c);
        assert_eq!(LINUX_SCSI_TARGET_TARGET_BUSY_OFFSET, 0x330);
        assert_eq!(LINUX_SCSI_TARGET_TARGET_BLOCKED_OFFSET, 0x334);
        assert_eq!(LINUX_SCSI_TARGET_CAN_QUEUE_OFFSET, 0x338);
        assert_eq!(LINUX_SCSI_TARGET_MAX_TARGET_BLOCKED_OFFSET, 0x33c);
        assert_eq!(LINUX_SCSI_TARGET_SCSI_LEVEL_OFFSET, 0x340);
        assert_eq!(LINUX_SCSI_TARGET_STATE_OFFSET, 0x344);
        assert_eq!(LINUX_SCSI_TARGET_HOSTDATA_OFFSET, 0x348);
        assert_eq!(
            LINUX_SCSI_TARGET_STARGET_DATA_OFFSET,
            LINUX_SCSI_TARGET_SIZE
        );
        assert_eq!(
            LINUX_SCSI_TARGET_STARGET_SDEV_USER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_TARGET_SIBLINGS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_SIBLINGS_OFFSET + size_of::<usize>() * 2,
            LINUX_SCSI_TARGET_DEVICES_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_DEVICES_OFFSET + size_of::<usize>() * 2,
            LINUX_SCSI_TARGET_DEV_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_DEV_OFFSET + LINUX_STRUCT_DEVICE_SIZE,
            LINUX_SCSI_TARGET_REAP_REF_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_REAP_REF_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_CHANNEL_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_CHANNEL_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_ID_OFFSET
        );
        // create/single_lun and related target flags share this u32 bitfield.
        assert_eq!(
            LINUX_SCSI_TARGET_ID_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_BITFLAGS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_BITFLAGS_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_TARGET_BUSY_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_TARGET_BUSY_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_TARGET_BLOCKED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_TARGET_BLOCKED_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_CAN_QUEUE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_CAN_QUEUE_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_MAX_TARGET_BLOCKED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_MAX_TARGET_BLOCKED_OFFSET + size_of::<u32>(),
            LINUX_SCSI_TARGET_SCSI_LEVEL_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_TARGET_HOSTDATA_OFFSET + size_of::<usize>(),
            LINUX_SCSI_TARGET_STARGET_DATA_OFFSET
        );

        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SIZE, 0x160);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_CMD_SIZE_OFFSET, 0);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_QUEUECOMMAND_OFFSET, 0x8);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_QUEUE_RESERVED_COMMAND_OFFSET, 0x10);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_COMMIT_RQS_OFFSET, 0x18);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_MODULE_OFFSET, 0x20);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_NAME_OFFSET, 0x28);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_INFO_OFFSET, 0x30);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_IOCTL_OFFSET, 0x38);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_INIT_CMD_PRIV_OFFSET, 0x40);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_EXIT_CMD_PRIV_OFFSET, 0x48);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_EH_ABORT_HANDLER_OFFSET, 0x50);
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_DEVICE_RESET_HANDLER_OFFSET,
            0x58
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_TARGET_RESET_HANDLER_OFFSET,
            0x60
        );
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_EH_BUS_RESET_HANDLER_OFFSET, 0x68);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_EH_HOST_RESET_HANDLER_OFFSET, 0x70);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SDEV_INIT_OFFSET, 0x78);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SDEV_CONFIGURE_OFFSET, 0x80);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SDEV_DESTROY_OFFSET, 0x88);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_TARGET_ALLOC_OFFSET, 0x90);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_TARGET_DESTROY_OFFSET, 0x98);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SCAN_FINISHED_OFFSET, 0xa0);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SCAN_START_OFFSET, 0xa8);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_CHANGE_QUEUE_DEPTH_OFFSET, 0xb0);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_MAP_QUEUES_OFFSET, 0xb8);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_MQ_POLL_OFFSET, 0xc0);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_DMA_NEED_DRAIN_OFFSET, 0xc8);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_EH_TIMED_OUT_OFFSET, 0xf0);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_EH_SHOULD_RETRY_CMD_OFFSET, 0xf8);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_HOST_RESET_OFFSET, 0x100);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_PROC_NAME_OFFSET, 0x108);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_CAN_QUEUE_OFFSET, 0x110);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_NR_RESERVED_CMDS_OFFSET, 0x114);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_THIS_ID_OFFSET, 0x118);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SG_TABLESIZE_OFFSET, 0x11c);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SG_PROT_TABLESIZE_OFFSET, 0x11e);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_MAX_SECTORS_OFFSET, 0x120);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_MAX_SEGMENT_SIZE_OFFSET, 0x124);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_DMA_ALIGNMENT_OFFSET, 0x128);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_DMA_BOUNDARY_OFFSET, 0x130);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_VIRT_BOUNDARY_MASK_OFFSET, 0x138);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_CMD_PER_LUN_OFFSET, 0x140);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_BITFLAGS_OFFSET, 0x142);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_MAX_HOST_BLOCKED_OFFSET, 0x144);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SHOST_GROUPS_OFFSET, 0x148);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_SDEV_GROUPS_OFFSET, 0x150);
        assert_eq!(LINUX_SCSI_HOST_TEMPLATE_VENDOR_ID_OFFSET, 0x158);
        assert!(
            LINUX_SCSI_HOST_TEMPLATE_CMD_SIZE_OFFSET + size_of::<u32>()
                <= LINUX_SCSI_HOST_TEMPLATE_QUEUECOMMAND_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_QUEUECOMMAND_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_QUEUE_RESERVED_COMMAND_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_QUEUE_RESERVED_COMMAND_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_COMMIT_RQS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_COMMIT_RQS_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_MODULE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_MODULE_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_NAME_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_NAME_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_INFO_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_INFO_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_IOCTL_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_IOCTL_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_INIT_CMD_PRIV_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_INIT_CMD_PRIV_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EXIT_CMD_PRIV_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EXIT_CMD_PRIV_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EH_ABORT_HANDLER_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_ABORT_HANDLER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EH_DEVICE_RESET_HANDLER_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_DEVICE_RESET_HANDLER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EH_TARGET_RESET_HANDLER_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_TARGET_RESET_HANDLER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EH_BUS_RESET_HANDLER_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_BUS_RESET_HANDLER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EH_HOST_RESET_HANDLER_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_HOST_RESET_HANDLER_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_SDEV_INIT_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SDEV_INIT_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_SDEV_CONFIGURE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SDEV_CONFIGURE_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_SDEV_DESTROY_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SDEV_DESTROY_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_TARGET_ALLOC_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_TARGET_ALLOC_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_TARGET_DESTROY_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_TARGET_DESTROY_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_SCAN_FINISHED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SCAN_FINISHED_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_SCAN_START_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SCAN_START_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_CHANGE_QUEUE_DEPTH_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_CHANGE_QUEUE_DEPTH_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_MAP_QUEUES_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_MAP_QUEUES_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_MQ_POLL_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_MQ_POLL_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_DMA_NEED_DRAIN_OFFSET
        );
        assert!(
            LINUX_SCSI_HOST_TEMPLATE_DMA_NEED_DRAIN_OFFSET + size_of::<usize>()
                <= LINUX_SCSI_HOST_TEMPLATE_EH_TIMED_OUT_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_TIMED_OUT_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_EH_SHOULD_RETRY_CMD_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_EH_SHOULD_RETRY_CMD_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_HOST_RESET_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_HOST_RESET_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_PROC_NAME_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_PROC_NAME_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_CAN_QUEUE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_CAN_QUEUE_OFFSET + size_of::<i32>(),
            LINUX_SCSI_HOST_TEMPLATE_NR_RESERVED_CMDS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_NR_RESERVED_CMDS_OFFSET + size_of::<i32>(),
            LINUX_SCSI_HOST_TEMPLATE_THIS_ID_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_THIS_ID_OFFSET + size_of::<i32>(),
            LINUX_SCSI_HOST_TEMPLATE_SG_TABLESIZE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SG_TABLESIZE_OFFSET + size_of::<u16>(),
            LINUX_SCSI_HOST_TEMPLATE_SG_PROT_TABLESIZE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SG_PROT_TABLESIZE_OFFSET + size_of::<u16>(),
            LINUX_SCSI_HOST_TEMPLATE_MAX_SECTORS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_MAX_SECTORS_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_TEMPLATE_MAX_SEGMENT_SIZE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_MAX_SEGMENT_SIZE_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_TEMPLATE_DMA_ALIGNMENT_OFFSET
        );
        assert!(
            LINUX_SCSI_HOST_TEMPLATE_DMA_ALIGNMENT_OFFSET + size_of::<u32>()
                <= LINUX_SCSI_HOST_TEMPLATE_DMA_BOUNDARY_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_DMA_BOUNDARY_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_VIRT_BOUNDARY_MASK_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_VIRT_BOUNDARY_MASK_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_CMD_PER_LUN_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_CMD_PER_LUN_OFFSET + size_of::<i16>(),
            LINUX_SCSI_HOST_TEMPLATE_BITFLAGS_OFFSET
        );
        // tag_alloc_policy_rr, no_write_same, host_tagset, and
        // queuecommand_may_block are packed into this probed bitfield span.
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_BITFLAGS_OFFSET + size_of::<u16>(),
            LINUX_SCSI_HOST_TEMPLATE_MAX_HOST_BLOCKED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_MAX_HOST_BLOCKED_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_TEMPLATE_SHOST_GROUPS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SHOST_GROUPS_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_SDEV_GROUPS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_SDEV_GROUPS_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_TEMPLATE_VENDOR_ID_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_TEMPLATE_VENDOR_ID_OFFSET + size_of::<u64>(),
            LINUX_SCSI_HOST_TEMPLATE_SIZE
        );

        assert_eq!(LINUX_SCSI_HOST_SIZE, 0x868);
        assert_eq!(LINUX_SCSI_HOST_DEVICES_OFFSET, 0x0);
        assert_eq!(LINUX_SCSI_HOST_HOST_LOCK_OFFSET, 0x30);
        assert_eq!(LINUX_SCSI_HOST_EH_ABORT_LIST_OFFSET, 0x48);
        assert_eq!(LINUX_SCSI_HOST_EHANDLER_OFFSET, 0x68);
        assert_eq!(LINUX_SCSI_HOST_EH_ACTION_OFFSET, 0x70);
        assert_eq!(LINUX_SCSI_HOST_HOST_WAIT_OFFSET, 0x78);
        assert_eq!(LINUX_SCSI_HOST_HOSTT_OFFSET, 0x88);
        assert_eq!(LINUX_SCSI_HOST_TAG_SET_OFFSET, 0xb8);
        assert_eq!(LINUX_SCSI_HOST_HOST_BLOCKED_OFFSET, 0x198);
        assert_eq!(LINUX_SCSI_HOST_HOST_FAILED_OFFSET, 0x19c);
        assert_eq!(LINUX_SCSI_HOST_HOST_EH_SCHEDULED_OFFSET, 0x1a0);
        assert_eq!(LINUX_SCSI_HOST_HOST_NO_OFFSET, 0x1a4);
        assert_eq!(LINUX_SCSI_HOST_MAX_CMD_LEN_OFFSET, 0x1cc);
        assert_eq!(LINUX_SCSI_HOST_CAN_QUEUE_OFFSET, 0x1d4);
        assert_eq!(LINUX_SCSI_HOST_NR_RESERVED_CMDS_OFFSET, 0x1d8);
        assert_eq!(LINUX_SCSI_HOST_CMD_PER_LUN_OFFSET, 0x1dc);
        assert_eq!(LINUX_SCSI_HOST_SG_TABLESIZE_OFFSET, 0x1de);
        assert_eq!(LINUX_SCSI_HOST_NR_HW_QUEUES_OFFSET, 0x208);
        assert_eq!(LINUX_SCSI_HOST_NR_MAPS_OFFSET, 0x20c);
        assert_eq!(LINUX_SCSI_HOST_WORK_Q_OFFSET, 0x218);
        assert_eq!(LINUX_SCSI_HOST_SHOST_STATE_OFFSET, 0x250);
        assert_eq!(LINUX_SCSI_HOST_SHOST_GENDEV_OFFSET, 0x258);
        assert_eq!(LINUX_SCSI_HOST_SHOST_DEV_OFFSET, 0x550);
        assert_eq!(LINUX_SCSI_HOST_PSEUDO_SDEV_OFFSET, 0x848);
        assert_eq!(LINUX_SCSI_HOST_SHOST_DATA_OFFSET, 0x850);
        assert_eq!(LINUX_SCSI_HOST_DMA_DEV_OFFSET, 0x858);
        assert_eq!(LINUX_SCSI_HOST_HOSTDATA_OFFSET, LINUX_SCSI_HOST_SIZE);
        assert_eq!(
            LINUX_SCSI_HOST_TAG_SET_OFFSET + size_of::<LinuxBlkMqTagSet>(),
            LINUX_SCSI_HOST_HOST_BLOCKED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_HOST_BLOCKED_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_HOST_FAILED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_HOST_FAILED_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_HOST_EH_SCHEDULED_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_HOST_EH_SCHEDULED_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_HOST_NO_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_NR_RESERVED_CMDS_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_CMD_PER_LUN_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_CMD_PER_LUN_OFFSET + size_of::<u16>(),
            LINUX_SCSI_HOST_SG_TABLESIZE_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_NR_HW_QUEUES_OFFSET + size_of::<u32>(),
            LINUX_SCSI_HOST_NR_MAPS_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_SHOST_GENDEV_OFFSET + LINUX_STRUCT_DEVICE_SIZE,
            LINUX_SCSI_HOST_SHOST_DEV_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_PSEUDO_SDEV_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_SHOST_DATA_OFFSET
        );
        assert_eq!(
            LINUX_SCSI_HOST_SHOST_DATA_OFFSET + size_of::<usize>(),
            LINUX_SCSI_HOST_DMA_DEV_OFFSET
        );
        assert!(LINUX_SCSI_HOST_HOST_FAILED_OFFSET + size_of::<u32>() <= LINUX_SCSI_HOST_SIZE);
        assert!(
            LINUX_SCSI_HOST_HOST_EH_SCHEDULED_OFFSET + size_of::<u32>() <= LINUX_SCSI_HOST_SIZE
        );
        assert!(LINUX_SCSI_HOST_SHOST_STATE_OFFSET + size_of::<u32>() <= LINUX_SCSI_HOST_SIZE);
    }

    #[test]
    fn format_args_handles_storage_device_names() {
        unsafe {
            let mut buf = [0u8; 64];
            assert_eq!(
                format_args(
                    buf.as_mut_ptr(),
                    buf.len(),
                    b"host%d\0".as_ptr().cast(),
                    &[7]
                ),
                5
            );
            assert_eq!(&buf[..6], b"host7\0");

            assert_eq!(
                format_args(
                    buf.as_mut_ptr(),
                    buf.len(),
                    b"%d:%d:%d:%llu\0".as_ptr().cast(),
                    &[1, 2, 3, 4]
                ),
                7
            );
            assert_eq!(&buf[..8], b"1:2:3:4\0");

            assert_eq!(
                format_args(
                    buf.as_mut_ptr(),
                    buf.len(),
                    b"%04x:%02x\0".as_ptr().cast(),
                    &[0x1a, 0x2]
                ),
                7
            );
            assert_eq!(&buf[..8], b"001a:02\0");
        }
    }
}
