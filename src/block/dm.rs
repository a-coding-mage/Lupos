//! linux-parity: partial
//! linux-source: vendor/linux/drivers/md
//! linux-source: vendor/linux/drivers/md/dm.c
//! linux-source: vendor/linux/drivers/md/dm-linear.c
//! linux-source: vendor/linux/drivers/md/dm-mpath.c
//! linux-source: vendor/linux/drivers/md/dm-uevent.c
//! linux-source: vendor/linux/include/uapi/linux/dm-ioctl.h
//! test-origin: linux:vendor/linux/drivers/md
//! Device-mapper core slice: linear target plus bounded control plane.
//!
//! Mirrors Linux `dm-linear`: a mapped block device remaps bios to an
//! underlying block device at a fixed sector offset. The control-plane model
//! mirrors the Linux create/table-load/resume/status state machine enough for
//! LVM-style activation tests and emits Linux-shaped kobject uevents for the
//! mapped disk. A bounded multipath probe/path-event model mirrors the Linux
//! `dm-mpath` table, path-selection, and uevent surfaces used by udev and LVM
//! monitors. A bounded LVM2 PV scanner reads `LABELONE`/`LVM2 001` labels,
//! metadata-area headers, and raw metadata text records before activating the
//! parsed one-segment logical volumes through the dm-linear control plane.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::Any;
use core::mem::{MaybeUninit, size_of};
use core::sync::atomic::{AtomicU32, Ordering};

use lazy_static::lazy_static;
use spin::Mutex;

use crate::fs::ops::FileOps;
use crate::fs::types::FileRef;
use crate::include::uapi::errno::{EBUSY, EFAULT, EINVAL, EIO, ENOENT, ENOTCONN, ENOTTY};

use super::bio::{BIO_OP_READ, BIO_OP_WRITE, BioRef, BioVec, bio_alloc, submit_bio};
use super::block_device::{
    BlockDevice, BlockDeviceOps, BlockDeviceRef, lookup_block_device, register_block_device,
    registered_block_devices, unregister_block_device,
};
use super::gendisk::{register_gendisk, unregister_gendisk};

pub struct DmLinearTarget {
    pub parent: BlockDeviceRef,
    pub start_sector: u64,
    pub nr_sectors: u64,
    pub mapped_name: Option<String>,
    pub segments: Vec<DmLinearTargetSegment>,
}

pub struct DmLinearTargetSegment {
    pub parent: BlockDeviceRef,
    pub sector_start: u64,
    pub length: u64,
    pub target_start: u64,
}

pub struct DmMultipathTarget {
    pub mapped_name: String,
    pub nr_sectors: u64,
}

pub struct RegisteredDmDevice {
    pub name: String,
    pub aliases: Vec<String>,
    pub bdev: BlockDeviceRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LvmLogicalVolumeSpec {
    pub vg_name: String,
    pub lv_name: String,
    pub dm_name: String,
    pub parent_name: String,
    pub sector_start: u64,
    pub length: u64,
    pub target_start: u64,
    pub segments: Vec<DmLinearTableSpec>,
}

pub const DM_NAME_LEN: usize = 128;
pub const DM_UUID_LEN: usize = 129;
pub const DM_MAX_TYPE_NAME: usize = 16;
pub const DM_VERSION_MAJOR: u32 = 4;
pub const DM_VERSION_MINOR: u32 = 50;
pub const DM_VERSION_PATCHLEVEL: u32 = 0;
pub const DM_MAJOR: u32 = 253;
pub const DM_SUSPEND_FLAG: u32 = 1 << 1;
pub const DM_IOCTL_TYPE: u32 = 0xfd;
pub const DM_VERSION_CMD: u32 = 0;
pub const DM_LIST_DEVICES_CMD: u32 = 2;
pub const DM_DEV_CREATE_CMD: u32 = 3;
pub const DM_DEV_REMOVE_CMD: u32 = 4;
pub const DM_DEV_RENAME_CMD: u32 = 5;
pub const DM_DEV_SUSPEND_CMD: u32 = 6;
pub const DM_DEV_STATUS_CMD: u32 = 7;
pub const DM_DEV_WAIT_CMD: u32 = 8;
pub const DM_TABLE_LOAD_CMD: u32 = 9;
pub const DM_TABLE_CLEAR_CMD: u32 = 10;
pub const DM_TABLE_DEPS_CMD: u32 = 11;
pub const DM_TABLE_STATUS_CMD: u32 = 12;
pub const DM_LIST_VERSIONS_CMD: u32 = 13;
pub const DM_TARGET_MSG_CMD: u32 = 14;
pub const DM_DEV_SET_GEOMETRY_CMD: u32 = 15;
pub const DM_DEV_ARM_POLL_CMD: u32 = 16;
pub const DM_GET_TARGET_VERSION_CMD: u32 = 17;
pub const DM_MPATH_PROBE_PATHS_CMD: u32 = 18;
pub const DM_UUID_FLAG: u32 = 1 << 14;
pub const DM_BUFFER_FULL_FLAG: u32 = 1 << 8;
pub const DM_MAX_IOCTL_BUFFER: usize = 4096;
pub const DM_LINEAR_TARGET_VERSION: [u32; 3] = [1, 5, 0];
pub const DM_MULTIPATH_TARGET_VERSION: [u32; 3] = [1, 15, 0];
const DM_MAX_TABLE_TARGETS: usize = 8;

const LVM_SECTOR_SIZE: usize = 512;
const LVM_LABEL_SCAN_SECTORS: u64 = 4;
const LVM_LABEL_ID: &[u8; 8] = b"LABELONE";
const LVM_LABEL_TYPE: &[u8; 8] = b"LVM2 001";
const LVM_LABEL_HEADER_SIZE: usize = 32;
const LVM_PV_HEADER_SIZE: usize = 40;
const LVM_DISK_LOCN_SIZE: usize = 16;
const LVM_MDA_HEADER_SIZE: usize = 40;
const LVM_RAW_LOCN_SIZE: usize = 24;
const LVM_MDA_MAGIC: &[u8; 16] = b" LVM2 x[5A%r0N*>";
const LVM_RAW_LOCN_IGNORED: u32 = 1;
const LVM_METADATA_TEXT_LIMIT: usize = 1024 * 1024;

const IOC_WRITE: u32 = 1;
const IOC_READ: u32 = 2;
const IOC_TYPESHIFT: u32 = 8;
const IOC_SIZESHIFT: u32 = 16;
const IOC_DIRSHIFT: u32 = 30;

const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u32 {
    (dir << IOC_DIRSHIFT) | (ty << IOC_TYPESHIFT) | nr | (size << IOC_SIZESHIFT)
}

const fn iowr(ty: u32, nr: u32, size: u32) -> u32 {
    ioc(IOC_READ | IOC_WRITE, ty, nr, size)
}

pub const DM_VERSION_IOCTL: u32 = iowr(DM_IOCTL_TYPE, DM_VERSION_CMD, size_of::<DmIoctl>() as u32);
pub const DM_LIST_DEVICES_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_LIST_DEVICES_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_CREATE_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_CREATE_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_REMOVE_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_REMOVE_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_RENAME_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_RENAME_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_SUSPEND_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_SUSPEND_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_STATUS_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_STATUS_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_WAIT_IOCTL: u32 =
    iowr(DM_IOCTL_TYPE, DM_DEV_WAIT_CMD, size_of::<DmIoctl>() as u32);
pub const DM_TABLE_LOAD_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_TABLE_LOAD_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_TABLE_CLEAR_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_TABLE_CLEAR_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_TABLE_DEPS_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_TABLE_DEPS_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_TABLE_STATUS_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_TABLE_STATUS_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_LIST_VERSIONS_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_LIST_VERSIONS_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_TARGET_MSG_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_TARGET_MSG_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_SET_GEOMETRY_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_SET_GEOMETRY_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_DEV_ARM_POLL_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_DEV_ARM_POLL_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_GET_TARGET_VERSION_IOCTL: u32 = iowr(
    DM_IOCTL_TYPE,
    DM_GET_TARGET_VERSION_CMD,
    size_of::<DmIoctl>() as u32,
);
pub const DM_MPATH_PROBE_PATHS_IOCTL: u32 = ioc(0, DM_IOCTL_TYPE, DM_MPATH_PROBE_PATHS_CMD, 0);

/// UAPI mirror of `struct dm_ioctl`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DmIoctl {
    pub version: [u32; 3],
    pub data_size: u32,
    pub data_start: u32,
    pub target_count: u32,
    pub open_count: i32,
    pub flags: u32,
    pub event_nr: u32,
    pub padding: u32,
    pub dev: u64,
    pub name: [u8; DM_NAME_LEN],
    pub uuid: [u8; DM_UUID_LEN],
    pub data: [u8; 7],
}

/// UAPI mirror of `struct dm_target_spec`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DmTargetSpec {
    pub sector_start: u64,
    pub length: u64,
    pub status: i32,
    pub next: u32,
    pub target_type: [u8; DM_MAX_TYPE_NAME],
}

/// UAPI mirror of `struct dm_target_deps`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DmTargetDeps {
    pub count: u32,
    pub padding: u32,
}

/// UAPI mirror of `struct dm_target_versions`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DmTargetVersions {
    pub next: u32,
    pub version: [u32; 3],
}

/// UAPI mirror of `struct dm_target_msg`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct DmTargetMsg {
    pub sector: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DmParsedTargetMsg {
    sector: u64,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmLinearTableSpec {
    pub parent_name: String,
    pub sector_start: u64,
    pub length: u64,
    pub target_start: u64,
}

impl DmLinearTableSpec {
    pub fn new(parent_name: &str, sector_start: u64, length: u64, target_start: u64) -> Self {
        Self {
            parent_name: String::from(parent_name),
            sector_start,
            length,
            target_start,
        }
    }

    pub fn params(&self) -> String {
        format!("{} {}", self.parent_name, self.target_start)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmMultipathPathSpec {
    pub parent_name: String,
    pub active: bool,
    pub fail_count: u32,
    pub group: usize,
    pub per_path_args: Vec<String>,
}

impl DmMultipathPathSpec {
    pub fn new(parent_name: &str) -> Self {
        Self {
            parent_name: String::from(parent_name),
            active: true,
            fail_count: 0,
            group: 0,
            per_path_args: Vec::new(),
        }
    }

    fn with_group(parent_name: &str, group: usize, per_path_args: Vec<String>) -> Self {
        Self {
            parent_name: String::from(parent_name),
            active: true,
            fail_count: 0,
            group,
            per_path_args,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmMultipathPriorityGroup {
    pub selector: String,
    pub selector_args: Vec<String>,
    pub per_path_arg_count: usize,
    pub bypassed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmMultipathTableSpec {
    pub sector_start: u64,
    pub length: u64,
    pub features: Vec<String>,
    pub hardware_handler: Vec<String>,
    pub initial_group: usize,
    pub selector: String,
    pub groups: Vec<DmMultipathPriorityGroup>,
    pub paths: Vec<DmMultipathPathSpec>,
}

impl DmMultipathTableSpec {
    pub fn new(sector_start: u64, length: u64, selector: &str, paths: Vec<&str>) -> Self {
        Self {
            sector_start,
            length,
            features: Vec::new(),
            hardware_handler: Vec::new(),
            initial_group: 0,
            selector: String::from(selector),
            groups: alloc::vec![DmMultipathPriorityGroup {
                selector: String::from(selector),
                selector_args: Vec::new(),
                per_path_arg_count: 0,
                bypassed: false,
            }],
            paths: paths.into_iter().map(DmMultipathPathSpec::new).collect(),
        }
    }

    pub fn params(&self) -> String {
        let mut params = format!("{}", self.features.len());
        for feature in &self.features {
            params.push(' ');
            params.push_str(feature);
        }
        params.push(' ');
        params.push_str(&format!("{}", self.hardware_handler.len()));
        for handler_arg in &self.hardware_handler {
            params.push(' ');
            params.push_str(handler_arg);
        }
        params.push(' ');
        params.push_str(&format!("{} {}", self.groups.len(), self.initial_group + 1));
        for (group_idx, group) in self.groups.iter().enumerate() {
            let group_paths = self
                .paths
                .iter()
                .filter(|path| path.group == group_idx)
                .collect::<Vec<_>>();
            params.push(' ');
            params.push_str(&group.selector);
            params.push(' ');
            params.push_str(&format!("{}", group.selector_args.len()));
            for selector_arg in &group.selector_args {
                params.push(' ');
                params.push_str(selector_arg);
            }
            params.push(' ');
            params.push_str(&format!(
                "{} {}",
                group_paths.len(),
                group.per_path_arg_count
            ));
            for path in group_paths {
                params.push(' ');
                params.push_str(&path.parent_name);
                for arg in &path.per_path_args {
                    params.push(' ');
                    params.push_str(arg);
                }
            }
        }
        params
    }

    fn valid_paths(&self) -> u32 {
        self.paths.iter().filter(|path| path.active).count() as u32
    }

    fn active_group_from(&self, preferred: usize) -> Option<usize> {
        if self.groups.is_empty() {
            return None;
        }
        for offset in 0..self.groups.len() {
            let group = (preferred + offset) % self.groups.len();
            if !self.groups[group].bypassed
                && self
                    .paths
                    .iter()
                    .any(|path| path.group == group && path.active)
            {
                return Some(group);
            }
        }
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DmTableSpecKind {
    Linear(DmLinearTableSpec),
    LinearTargets(Vec<DmLinearTableSpec>),
    Multipath(DmMultipathTableSpec),
}

impl DmTableSpecKind {
    fn sector_start(&self) -> u64 {
        match self {
            Self::Linear(spec) => spec.sector_start,
            Self::LinearTargets(specs) => specs.first().map(|spec| spec.sector_start).unwrap_or(0),
            Self::Multipath(spec) => spec.sector_start,
        }
    }

    fn length(&self) -> u64 {
        match self {
            Self::Linear(spec) => spec.length,
            Self::LinearTargets(specs) => linear_specs_length(specs),
            Self::Multipath(spec) => spec.length,
        }
    }

    fn target_type(&self) -> &'static str {
        match self {
            Self::Linear(_) | Self::LinearTargets(_) => "linear",
            Self::Multipath(_) => "multipath",
        }
    }

    fn params(&self) -> String {
        match self {
            Self::Linear(spec) => spec.params(),
            Self::LinearTargets(specs) => specs
                .first()
                .map(DmLinearTableSpec::params)
                .unwrap_or_default(),
            Self::Multipath(spec) => spec.params(),
        }
    }

    fn valid_paths(&self) -> u32 {
        match self {
            Self::Linear(_) | Self::LinearTargets(_) => 1,
            Self::Multipath(spec) => spec.valid_paths(),
        }
    }

    fn target_count(&self) -> u32 {
        match self {
            Self::Linear(_) | Self::Multipath(_) => 1,
            Self::LinearTargets(specs) => specs.len() as u32,
        }
    }

    fn table_statuses(&self) -> Vec<DmTableStatus> {
        match self {
            Self::Linear(spec) => alloc::vec![DmTableStatus::from_linear(spec)],
            Self::LinearTargets(specs) => specs.iter().map(DmTableStatus::from_linear).collect(),
            Self::Multipath(spec) => alloc::vec![DmTableStatus {
                sector_start: spec.sector_start,
                length: spec.length,
                target_type: "multipath",
                params: spec.params(),
            }],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmTableStatus {
    pub sector_start: u64,
    pub length: u64,
    pub target_type: &'static str,
    pub params: String,
}

impl DmTableStatus {
    fn from_linear(spec: &DmLinearTableSpec) -> Self {
        Self {
            sector_start: spec.sector_start,
            length: spec.length,
            target_type: "linear",
            params: spec.params(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmGeometry {
    pub cylinders: u16,
    pub heads: u8,
    pub sectors: u8,
    pub start: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmPathEventKind {
    PathFailed,
    PathReinstated,
}

impl DmPathEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PathFailed => "PATH_FAILED",
            Self::PathReinstated => "PATH_REINSTATED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmPathEvent {
    pub name: String,
    pub path: String,
    pub kind: DmPathEventKind,
    pub event_nr: u32,
    pub valid_paths: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DmDeviceStatus {
    pub version: [u32; 3],
    pub name: String,
    pub uuid: String,
    pub minor: u32,
    pub dev: u64,
    pub flags: u32,
    pub event_nr: u32,
    pub suspended: bool,
    pub active: bool,
    pub inactive: bool,
    pub open_count: i32,
    pub target_count: u32,
    pub valid_paths: u32,
    pub table: Option<DmTableStatus>,
    pub geometry: Option<DmGeometry>,
}

#[derive(Clone, Debug)]
struct DmMappedDevice {
    name: String,
    uuid: String,
    minor: u32,
    event_nr: u32,
    uevent_seq: u32,
    suspended: bool,
    active: Option<DmTableSpecKind>,
    inactive: Option<DmTableSpecKind>,
    valid_paths: u32,
    next_path: usize,
    geometry: Option<DmGeometry>,
}

lazy_static! {
    static ref DM_CONTROL: Mutex<Vec<DmMappedDevice>> = Mutex::new(Vec::new());
    static ref DM_PATH_EVENTS: Mutex<Vec<DmPathEvent>> = Mutex::new(Vec::new());
}

static NEXT_DM_MINOR: AtomicU32 = AtomicU32::new(0);

pub fn dm_version() -> [u32; 3] {
    [DM_VERSION_MAJOR, DM_VERSION_MINOR, DM_VERSION_PATCHLEVEL]
}

pub fn dm_dev_create(name: &str, uuid: &str) -> Result<DmDeviceStatus, i32> {
    validate_dm_name(name)?;
    validate_dm_uuid(uuid)?;

    let mut devices = DM_CONTROL.lock();
    if devices
        .iter()
        .any(|dev| dev.name == name || (!uuid.is_empty() && dev.uuid == uuid))
    {
        return Err(EBUSY);
    }

    let minor = NEXT_DM_MINOR.fetch_add(1, Ordering::AcqRel);
    devices.push(DmMappedDevice {
        name: String::from(name),
        uuid: String::from(uuid),
        minor,
        event_nr: 0,
        uevent_seq: 0,
        suspended: true,
        active: None,
        inactive: None,
        valid_paths: 0,
        next_path: 0,
        geometry: None,
    });

    let dev = devices.last_mut().unwrap();
    emit_dm_uevent(dev, crate::net::uevent::UeventAction::Add, 0, false);
    Ok(status_from_device(dev))
}

pub fn dm_table_load_linear(name: &str, spec: DmLinearTableSpec) -> Result<DmDeviceStatus, i32> {
    validate_linear_spec(&spec)?;

    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    dev.inactive = Some(DmTableSpecKind::Linear(spec));
    Ok(status_from_device(dev))
}

pub fn dm_table_load_linear_targets(
    name: &str,
    specs: Vec<DmLinearTableSpec>,
) -> Result<DmDeviceStatus, i32> {
    validate_linear_table_specs(&specs)?;

    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    dev.inactive = if specs.len() == 1 {
        Some(DmTableSpecKind::Linear(specs[0].clone()))
    } else {
        Some(DmTableSpecKind::LinearTargets(specs))
    };
    Ok(status_from_device(dev))
}

pub fn dm_table_load_multipath(
    name: &str,
    spec: DmMultipathTableSpec,
) -> Result<DmDeviceStatus, i32> {
    validate_multipath_spec(&spec)?;

    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    dev.inactive = Some(DmTableSpecKind::Multipath(spec));
    Ok(status_from_device(dev))
}

pub fn dm_resume(name: &str) -> Result<DmDeviceStatus, i32> {
    dm_resume_with_cookie(name, 0)
}

fn dm_resume_with_cookie(name: &str, cookie: u32) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;

    if let Some(inactive) = dev.inactive.take() {
        if dev.active.is_some() {
            dev.inactive = Some(inactive);
            return Err(EBUSY);
        }

        let dev_name = format!("dm-{}", dev.minor);
        let alias = format!("mapper/{}", dev.name);
        match &inactive {
            DmTableSpecKind::Linear(linear) => {
                let parent = lookup_block_device(&linear.parent_name).ok_or(ENOENT)?;
                register_dm_linear(
                    &dev_name,
                    &[alias.as_str()],
                    parent,
                    linear.target_start,
                    linear.length,
                )?;
            }
            DmTableSpecKind::LinearTargets(linear) => {
                register_dm_linear_targets(&dev_name, &[alias.as_str()], linear)?;
            }
            DmTableSpecKind::Multipath(multipath) => {
                register_dm_multipath(&dev_name, &[alias.as_str()], &dev.name, multipath.length)?;
            }
        }
        dev.valid_paths = inactive.valid_paths();
        dev.next_path = 0;
        dev.active = Some(inactive);
    } else if dev.active.is_none() {
        return Err(EINVAL);
    }

    if dev.valid_paths == 0 && !matches!(dev.active, Some(DmTableSpecKind::Multipath(_))) {
        dev.valid_paths = 1;
    }
    dev.suspended = false;
    dev.event_nr = dev.event_nr.saturating_add(1);
    emit_dm_uevent(dev, crate::net::uevent::UeventAction::Change, cookie, false);
    Ok(status_from_device(dev))
}

pub fn dm_suspend(name: &str) -> Result<DmDeviceStatus, i32> {
    dm_suspend_with_cookie(name, 0)
}

fn dm_suspend_with_cookie(name: &str, cookie: u32) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    dev.suspended = true;
    dev.event_nr = dev.event_nr.saturating_add(1);
    emit_dm_uevent(dev, crate::net::uevent::UeventAction::Change, cookie, false);
    Ok(status_from_device(dev))
}

pub fn dm_dev_status(name: &str) -> Result<DmDeviceStatus, i32> {
    let devices = DM_CONTROL.lock();
    let dev = find_device(&devices, name)?;
    Ok(status_from_device(dev))
}

pub fn dm_list_devices() -> Vec<DmDeviceStatus> {
    DM_CONTROL.lock().iter().map(status_from_device).collect()
}

pub fn dm_table_clear(name: &str) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    dev.inactive = None;
    Ok(status_from_device(dev))
}

pub fn dm_dev_set_geometry(name: &str, geometry: DmGeometry) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    dev.geometry = Some(geometry);
    Ok(status_from_device(dev))
}

pub fn dm_multipath_probe_active_paths(name: &str) -> Result<u32, i32> {
    let devices = DM_CONTROL.lock();
    let dev = find_device(&devices, name)?;
    let _active = dev.active.as_ref().ok_or(EINVAL)?;
    if dev.valid_paths == 0 {
        return Err(ENOTCONN);
    }
    Ok(dev.valid_paths)
}

pub fn dm_path_events() -> Vec<DmPathEvent> {
    DM_PATH_EVENTS.lock().clone()
}

pub fn dm_multipath_path_event(
    name: &str,
    path: &str,
    kind: DmPathEventKind,
    cookie: u32,
) -> Result<DmDeviceStatus, i32> {
    validate_dm_path(path)?;

    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    let _active = dev.active.as_ref().ok_or(EINVAL)?;
    if dev.suspended {
        return Err(EBUSY);
    }

    let mut changed = false;
    if let Some(DmTableSpecKind::Multipath(spec)) = dev.active.as_mut() {
        let path_spec = spec
            .paths
            .iter_mut()
            .find(|candidate| dm_path_name_matches(&candidate.parent_name, path))
            .ok_or(ENOENT)?;
        match kind {
            DmPathEventKind::PathFailed if path_spec.active => {
                path_spec.active = false;
                path_spec.fail_count = path_spec.fail_count.saturating_add(1);
                changed = true;
            }
            DmPathEventKind::PathReinstated if !path_spec.active => {
                path_spec.active = true;
                changed = true;
            }
            _ => {}
        }
        dev.valid_paths = spec.valid_paths();
    } else {
        match kind {
            DmPathEventKind::PathFailed => {
                let old = dev.valid_paths;
                dev.valid_paths = dev.valid_paths.saturating_sub(1);
                changed = dev.valid_paths != old;
            }
            DmPathEventKind::PathReinstated => {
                if dev.valid_paths == 0 {
                    dev.valid_paths = 1;
                    changed = true;
                }
            }
        }
    }
    if changed {
        dev.next_path = 0;
    }

    dev.event_nr = dev.event_nr.saturating_add(1);
    let valid_paths = format!("{}", dev.valid_paths);
    let extra = [
        ("DM_TARGET", "multipath"),
        ("DM_ACTION", kind.as_str()),
        ("DM_PATH", path),
        ("DM_NR_VALID_PATHS", valid_paths.as_str()),
    ];
    emit_dm_uevent_with_extra(
        dev,
        crate::net::uevent::UeventAction::Change,
        cookie,
        false,
        &extra,
    );

    let event = DmPathEvent {
        name: dev.name.clone(),
        path: String::from(path),
        kind,
        event_nr: dev.event_nr,
        valid_paths: dev.valid_paths,
    };
    DM_PATH_EVENTS.lock().push(event);
    Ok(status_from_device(dev))
}

fn dm_multipath_set_queue_if_no_path(name: &str, enabled: bool) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    if dev.suspended {
        return Err(EBUSY);
    }
    {
        let DmTableSpecKind::Multipath(spec) = dev.active.as_mut().ok_or(EINVAL)? else {
            return Err(EINVAL);
        };
        if enabled {
            if !spec
                .features
                .iter()
                .any(|feature| feature == "queue_if_no_path")
            {
                spec.features.push(String::from("queue_if_no_path"));
            }
        } else {
            spec.features
                .retain(|feature| feature != "queue_if_no_path");
        }
    }
    dev.next_path = 0;
    Ok(status_from_device(dev))
}

fn parse_multipath_group_number(spec: &DmMultipathTableSpec, group: &str) -> Result<usize, i32> {
    let group = parse_usize_arg(group)?;
    if group == 0 || group > spec.groups.len() {
        return Err(EINVAL);
    }
    Ok(group - 1)
}

fn dm_multipath_set_group_bypass(
    name: &str,
    group: &str,
    bypassed: bool,
) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    if dev.suspended {
        return Err(EBUSY);
    }
    {
        let DmTableSpecKind::Multipath(spec) = dev.active.as_mut().ok_or(EINVAL)? else {
            return Err(EINVAL);
        };
        let group = parse_multipath_group_number(spec, group)?;
        spec.groups[group].bypassed = bypassed;
    }
    dev.next_path = 0;
    Ok(status_from_device(dev))
}

fn dm_multipath_switch_group(name: &str, group: &str) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let dev = find_device_mut(&mut devices, name)?;
    if dev.suspended {
        return Err(EBUSY);
    }
    {
        let DmTableSpecKind::Multipath(spec) = dev.active.as_mut().ok_or(EINVAL)? else {
            return Err(EINVAL);
        };
        spec.initial_group = parse_multipath_group_number(spec, group)?;
    }
    dev.next_path = 0;
    Ok(status_from_device(dev))
}

pub fn lvm_parse_text_metadata(metadata: &str) -> Result<Vec<LvmLogicalVolumeSpec>, i32> {
    let parsed = parse_lvm_metadata(metadata)?;
    parsed.logical_volumes()
}

pub fn lvm_scan_text_metadata_from_block_device(bdev: &BlockDeviceRef) -> Result<Vec<String>, i32> {
    let labels = super::partitions::read_sectors(bdev, 0, LVM_LABEL_SCAN_SECTORS)?;
    let mut metadata = Vec::new();

    for sector in 0..LVM_LABEL_SCAN_SECTORS as usize {
        let sector_off = sector.checked_mul(LVM_SECTOR_SIZE).ok_or(EIO)?;
        let label = labels
            .get(sector_off..sector_off + LVM_SECTOR_SIZE)
            .ok_or(EINVAL)?;
        if !lvm_label_header_matches(label, sector as u64)? {
            continue;
        }
        let pv_offset = le_u32_at(label, 20)? as usize;
        let metadata_areas = lvm_parse_pv_metadata_areas(label, pv_offset)?;
        for (mda_offset, mda_size) in metadata_areas {
            if let Some(text) = lvm_read_metadata_area_text(bdev, mda_offset, mda_size)? {
                push_unique_metadata(&mut metadata, text);
            }
        }
    }

    if metadata.is_empty() {
        return Err(ENOENT);
    }
    Ok(metadata)
}

pub fn lvm_scan_registered_text_metadata() -> Result<Vec<String>, i32> {
    let mut metadata = Vec::new();
    let mut seen_bdevs = Vec::new();

    for (_name, bdev) in registered_block_devices() {
        if seen_bdevs.iter().any(|id| *id == bdev.id) {
            continue;
        }
        seen_bdevs.push(bdev.id);
        if bdev.capacity_sectors() < LVM_LABEL_SCAN_SECTORS {
            continue;
        }
        match lvm_scan_text_metadata_from_block_device(&bdev) {
            Ok(records) => {
                for text in records {
                    push_unique_metadata(&mut metadata, text);
                }
            }
            Err(ENOENT) => {}
            Err(errno) => return Err(errno),
        }
    }

    if metadata.is_empty() {
        return Err(ENOENT);
    }
    Ok(metadata)
}

pub fn lvm_activate_block_device(name: &str) -> Result<Vec<DmDeviceStatus>, i32> {
    let bdev = lookup_block_device(name).ok_or(ENOENT)?;
    let metadata = lvm_scan_text_metadata_from_block_device(&bdev)?;
    lvm_activate_metadata_records(metadata)
}

pub fn lvm_activate_registered_block_devices() -> Result<Vec<DmDeviceStatus>, i32> {
    lvm_activate_metadata_records(lvm_scan_registered_text_metadata()?)
}

pub fn lvm_activate_text_metadata(metadata: &str) -> Result<Vec<DmDeviceStatus>, i32> {
    let lvs = lvm_parse_text_metadata(metadata)?;
    let mut statuses = Vec::new();
    for lv in lvs {
        let status = match dm_dev_create(&lv.dm_name, "") {
            Ok(status) => status,
            Err(EBUSY) => dm_dev_status(&lv.dm_name)?,
            Err(errno) => return Err(errno),
        };
        if status.active {
            statuses.push(status);
            continue;
        }
        dm_table_load_linear_targets(&lv.dm_name, lv.segments.clone())?;
        statuses.push(dm_resume(&lv.dm_name)?);
    }
    Ok(statuses)
}

fn lvm_activate_metadata_records(metadata: Vec<String>) -> Result<Vec<DmDeviceStatus>, i32> {
    let mut statuses = Vec::new();
    for text in metadata {
        for status in lvm_activate_text_metadata(&text)? {
            push_unique_status(&mut statuses, status);
        }
    }
    Ok(statuses)
}

fn dm_target_message(name: &str, target_msg: &DmParsedTargetMsg) -> Result<DmDeviceStatus, i32> {
    let mut args = target_msg.message.split_whitespace();
    let first = args.next().ok_or(EINVAL)?;
    let rest = args.collect::<Vec<_>>();
    if first == "@cancel_deferred_remove" {
        if !rest.is_empty() {
            return Err(EINVAL);
        }
        let devices = DM_CONTROL.lock();
        let dev = find_device(&devices, name)?;
        return Ok(status_from_device(dev));
    }
    if first.starts_with('@') {
        return Err(EINVAL);
    }

    {
        let devices = DM_CONTROL.lock();
        let dev = find_device(&devices, name)?;
        let active = dev.active.as_ref().ok_or(EINVAL)?;
        let target_end = active
            .sector_start()
            .checked_add(active.length())
            .ok_or(EIO)?;
        if target_msg.sector < active.sector_start() || target_msg.sector >= target_end {
            return Err(EINVAL);
        }
        if !matches!(active, DmTableSpecKind::Multipath(_)) {
            // Linux `dm-linear` has no `target_type.message` callback, so a
            // valid envelope routed to a linear table returns `-EINVAL`.
            return Err(EINVAL);
        }
    }

    match (first, rest.as_slice()) {
        ("queue_if_no_path", []) => dm_multipath_set_queue_if_no_path(name, true),
        ("fail_if_no_path", []) => dm_multipath_set_queue_if_no_path(name, false),
        ("disable_group", [group]) => dm_multipath_set_group_bypass(name, group, true),
        ("enable_group", [group]) => dm_multipath_set_group_bypass(name, group, false),
        ("switch_group", [group]) => dm_multipath_switch_group(name, group),
        ("fail_path", [path]) => {
            dm_multipath_path_event(name, path, DmPathEventKind::PathFailed, 0)
        }
        ("reinstate_path", [path]) => {
            dm_multipath_path_event(name, path, DmPathEventKind::PathReinstated, 0)
        }
        _ => Err(EINVAL),
    }
}

pub fn dm_dev_rename(
    name: &str,
    new_value: &str,
    change_uuid: bool,
) -> Result<DmDeviceStatus, i32> {
    if change_uuid {
        validate_dm_uuid(new_value)?;
    } else {
        validate_dm_name(new_value)?;
    }

    let mut devices = DM_CONTROL.lock();
    if change_uuid {
        if devices
            .iter()
            .any(|dev| !new_value.is_empty() && dev.uuid == new_value && dev.name != name)
        {
            return Err(EBUSY);
        }
    } else if devices.iter().any(|dev| dev.name == new_value) {
        return Err(EBUSY);
    }

    let dev = find_device_mut(&mut devices, name)?;
    if change_uuid {
        dev.uuid = String::from(new_value);
    } else {
        let old_alias = format!("mapper/{}", dev.name);
        let new_alias = format!("mapper/{new_value}");
        if dev.active.is_some() {
            if let Some(bdev) = lookup_block_device(&format!("dm-{}", dev.minor)) {
                let _ = unregister_block_device(&old_alias);
                register_name(&new_alias, bdev)?;
            }
        }
        dev.name = String::from(new_value);
    }
    dev.event_nr = dev.event_nr.saturating_add(1);
    emit_dm_uevent(dev, crate::net::uevent::UeventAction::Change, 0, false);
    Ok(status_from_device(dev))
}

pub fn dm_dev_remove(name: &str) -> Result<DmDeviceStatus, i32> {
    let mut devices = DM_CONTROL.lock();
    let idx = devices
        .iter()
        .position(|dev| dev.name == name || (!name.is_empty() && dev.uuid == name))
        .ok_or(ENOENT)?;
    let mut dev = devices.remove(idx);
    dev.event_nr = dev.event_nr.saturating_add(1);
    let status = status_from_device(&dev);
    emit_dm_uevent(&mut dev, crate::net::uevent::UeventAction::Remove, 0, false);

    let dev_name = format!("dm-{}", dev.minor);
    let alias = format!("mapper/{}", dev.name);
    let _ = unregister_block_device(&dev_name);
    let _ = unregister_block_device(&alias);
    let _ = unregister_gendisk(&dev_name);
    Ok(status)
}

pub fn dm_control_ioctl_buffer(cmd: u32, buf: &mut [u8]) -> Result<i64, i32> {
    if buf.len() < size_of::<DmIoctl>() || buf.len() > DM_MAX_IOCTL_BUFFER {
        return Err(EINVAL);
    }
    let mut ioctl: DmIoctl = read_struct(buf, 0)?;
    if ioctl.data_size == 0 {
        ioctl.data_size = buf.len() as u32;
    }
    if ioctl.data_size as usize > buf.len() || ioctl.data_size as usize > DM_MAX_IOCTL_BUFFER {
        return Err(EINVAL);
    }
    if ioctl.data_start == 0 {
        ioctl.data_start = size_of::<DmIoctl>() as u32;
    }
    ioctl.version = dm_version();

    let result = match cmd {
        DM_VERSION_IOCTL => {
            ioctl.target_count = 0;
            Ok(())
        }
        DM_LIST_VERSIONS_IOCTL => write_target_versions_to_ioctl(buf, &mut ioctl, None),
        DM_GET_TARGET_VERSION_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            write_target_versions_to_ioctl(buf, &mut ioctl, Some(&name))
        }
        DM_DEV_ARM_POLL_IOCTL => {
            ioctl.target_count = 0;
            Ok(())
        }
        DM_TARGET_MSG_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let target_msg = parse_target_msg_from_ioctl(buf, &ioctl)?;
            let status = dm_target_message(&name, &target_msg)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_DEV_CREATE_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let uuid = fixed_cstr_to_string(&ioctl.uuid)?;
            let status = dm_dev_create(&name, &uuid)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_DEV_REMOVE_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = dm_dev_remove(&name)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_DEV_RENAME_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let new_value = cstr_from_ioctl_data(buf, &ioctl)?;
            let status = dm_dev_rename(&name, new_value, ioctl.flags & DM_UUID_FLAG != 0)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_TABLE_LOAD_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let spec = parse_table_from_ioctl(buf, &ioctl)?;
            let status = match spec {
                DmTableSpecKind::Linear(spec) => dm_table_load_linear(&name, spec)?,
                DmTableSpecKind::LinearTargets(specs) => {
                    dm_table_load_linear_targets(&name, specs)?
                }
                DmTableSpecKind::Multipath(spec) => dm_table_load_multipath(&name, spec)?,
            };
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_TABLE_CLEAR_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = dm_table_clear(&name)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_TABLE_DEPS_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = dm_dev_status(&name)?;
            fill_ioctl_status(&mut ioctl, &status);
            write_table_deps_to_ioctl(buf, &mut ioctl, &name)
        }
        DM_DEV_SET_GEOMETRY_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let geometry = parse_geometry_from_ioctl(buf, &ioctl)?;
            let status = dm_dev_set_geometry(&name, geometry)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_DEV_SUSPEND_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = if ioctl.flags & DM_SUSPEND_FLAG != 0 {
                dm_suspend_with_cookie(&name, ioctl.event_nr)?
            } else {
                dm_resume_with_cookie(&name, ioctl.event_nr)?
            };
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_DEV_STATUS_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = dm_dev_status(&name)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_DEV_WAIT_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = dm_dev_status(&name)?;
            fill_ioctl_status(&mut ioctl, &status);
            Ok(())
        }
        DM_LIST_DEVICES_IOCTL => write_device_list_to_ioctl(buf, &mut ioctl),
        DM_TABLE_STATUS_IOCTL => {
            let name = fixed_cstr_to_string(&ioctl.name)?;
            let status = dm_dev_status(&name)?;
            fill_ioctl_status(&mut ioctl, &status);
            write_table_status_to_ioctl(buf, &mut ioctl, &status)
        }
        _ => Err(ENOTTY),
    };

    write_struct(buf, 0, &ioctl)?;
    result.map(|()| 0)
}

fn dm_control_ioctl(_file: &FileRef, cmd: u32, arg: u64) -> Result<i64, i32> {
    if arg == 0 {
        return Err(EFAULT);
    }

    let mut header = [0u8; size_of::<DmIoctl>()];
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(
            header.as_mut_ptr(),
            arg as *const u8,
            header.len(),
        )
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    let ioctl: DmIoctl = read_struct(&header, 0)?;
    let size = if ioctl.data_size == 0 {
        size_of::<DmIoctl>()
    } else {
        ioctl.data_size as usize
    };
    if !(size_of::<DmIoctl>()..=DM_MAX_IOCTL_BUFFER).contains(&size) {
        return Err(EINVAL);
    }

    let mut buf = alloc::vec![0u8; size];
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_from_user(buf.as_mut_ptr(), arg as *const u8, size)
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }

    let ret = dm_control_ioctl_buffer(cmd, &mut buf)?;
    let not_copied = unsafe {
        crate::arch::x86::kernel::uaccess::copy_to_user(arg as *mut u8, buf.as_ptr(), size)
    };
    if not_copied != 0 {
        return Err(EFAULT);
    }
    Ok(ret)
}

pub static DM_CONTROL_FILE_OPS: FileOps = FileOps {
    name: "dm-control",
    read: None,
    write: None,
    llseek: None,
    fsync: None,
    poll: None,
    ioctl: Some(dm_control_ioctl),
    mmap: None,
    release: None,
    readdir: None,
};

pub fn dm_linear_block_device(
    parent: BlockDeviceRef,
    start_sector: u64,
    nr_sectors: u64,
) -> Result<BlockDeviceRef, i32> {
    dm_linear_block_device_with_name(parent, start_sector, nr_sectors, None)
}

fn dm_linear_block_device_with_name(
    parent: BlockDeviceRef,
    start_sector: u64,
    nr_sectors: u64,
    mapped_name: Option<String>,
) -> Result<BlockDeviceRef, i32> {
    let segment = DmLinearTargetSegment {
        parent,
        sector_start: 0,
        length: nr_sectors,
        target_start: start_sector,
    };
    dm_linear_table_block_device(alloc::vec![segment], mapped_name)
}

fn dm_linear_table_block_device(
    segments: Vec<DmLinearTargetSegment>,
    mapped_name: Option<String>,
) -> Result<BlockDeviceRef, i32> {
    validate_linear_target_segments(&segments)?;
    let first = segments.first().ok_or(EINVAL)?;
    let nr_sectors = linear_target_segments_length(&segments);
    let backing = Arc::new(DmLinearTarget {
        parent: first.parent.clone(),
        start_sector: first.target_start,
        nr_sectors,
        mapped_name,
        segments,
    });
    Ok(BlockDevice::wrap(backing, &DM_LINEAR_OPS))
}

pub fn register_dm_linear(
    name: &str,
    aliases: &[&str],
    parent: BlockDeviceRef,
    start_sector: u64,
    nr_sectors: u64,
) -> Result<RegisteredDmDevice, i32> {
    let mapped_name = aliases
        .iter()
        .find_map(|alias| alias.strip_prefix("mapper/"))
        .map(String::from);
    let bdev = dm_linear_block_device_with_name(parent, start_sector, nr_sectors, mapped_name)?;
    register_name(name, bdev.clone())?;
    register_gendisk(name, bdev.clone());

    let mut registered_aliases = Vec::new();
    for alias in aliases {
        register_name(alias, bdev.clone())?;
        registered_aliases.push(String::from(*alias));
    }

    Ok(RegisteredDmDevice {
        name: String::from(name),
        aliases: registered_aliases,
        bdev,
    })
}

pub fn register_dm_linear_targets(
    name: &str,
    aliases: &[&str],
    specs: &[DmLinearTableSpec],
) -> Result<RegisteredDmDevice, i32> {
    validate_linear_table_specs(specs)?;
    let mapped_name = aliases
        .iter()
        .find_map(|alias| alias.strip_prefix("mapper/"))
        .map(String::from);
    let mut segments = Vec::new();
    for spec in specs {
        let parent = lookup_block_device(&spec.parent_name).ok_or(ENOENT)?;
        segments.push(DmLinearTargetSegment {
            parent,
            sector_start: spec.sector_start,
            length: spec.length,
            target_start: spec.target_start,
        });
    }
    let bdev = dm_linear_table_block_device(segments, mapped_name)?;
    register_name(name, bdev.clone())?;
    register_gendisk(name, bdev.clone());

    let mut registered_aliases = Vec::new();
    for alias in aliases {
        register_name(alias, bdev.clone())?;
        registered_aliases.push(String::from(*alias));
    }

    Ok(RegisteredDmDevice {
        name: String::from(name),
        aliases: registered_aliases,
        bdev,
    })
}

fn register_name(name: &str, bdev: BlockDeviceRef) -> Result<(), i32> {
    match register_block_device(name, bdev.clone()) {
        Ok(()) => Ok(()),
        Err(EBUSY) => match lookup_block_device(name) {
            Some(existing) if Arc::ptr_eq(&existing, &bdev) => Ok(()),
            _ => Err(EBUSY),
        },
        Err(err) => Err(err),
    }
}

fn dm_linear_submit_bio(bdev: &BlockDeviceRef, bio: &BioRef) -> Result<(), i32> {
    let target = dm_linear_backing(bdev)?;
    let sectors = (bio.total_size() as u64).div_ceil(512);
    let end = bio.sector.checked_add(sectors).ok_or(EIO)?;
    if end > target.nr_sectors {
        return Err(EIO);
    }
    let segment = target
        .segments
        .iter()
        .find(|segment| {
            let segment_end = segment.sector_start.saturating_add(segment.length);
            bio.sector >= segment.sector_start && end <= segment_end
        })
        .ok_or(EIO)?;
    let translated_sector = segment
        .target_start
        .checked_add(bio.sector.saturating_sub(segment.sector_start))
        .ok_or(EIO)?;

    match bio.op.0 {
        BIO_OP_READ => dm_linear_read(&segment.parent, bio, translated_sector),
        BIO_OP_WRITE => dm_linear_write(&segment.parent, bio, translated_sector),
        _ => {
            let parent_bio = bio_alloc(segment.parent.clone(), bio.op, translated_sector);
            submit_bio(parent_bio)
        }
    }
}

fn dm_linear_read(parent: &BlockDeviceRef, bio: &BioRef, sector: u64) -> Result<(), i32> {
    let parent_bio = bio_alloc(parent.clone(), bio.op, sector);
    {
        let vecs = bio.vecs.lock();
        for vec in vecs.iter() {
            parent_bio.add_vec(BioVec::new(alloc::vec![0u8; vec.len]));
        }
    }
    submit_bio(parent_bio.clone())?;

    let child_vecs = bio.vecs.lock();
    let parent_vecs = parent_bio.vecs.lock();
    for (child, parent) in child_vecs.iter().zip(parent_vecs.iter()) {
        let mut child_data = child.data.lock();
        let parent_data = parent.data.lock();
        child_data[child.off..child.off + child.len]
            .copy_from_slice(&parent_data[parent.off..parent.off + parent.len]);
    }
    Ok(())
}

fn dm_linear_write(parent: &BlockDeviceRef, bio: &BioRef, sector: u64) -> Result<(), i32> {
    let parent_bio = bio_alloc(parent.clone(), bio.op, sector);
    {
        let vecs = bio.vecs.lock();
        for vec in vecs.iter() {
            let data = vec.data.lock();
            parent_bio.add_vec(BioVec::new(data[vec.off..vec.off + vec.len].to_vec()));
        }
    }
    submit_bio(parent_bio)
}

fn dm_linear_get_capacity(bdev: &BlockDeviceRef) -> u64 {
    dm_linear_backing(bdev)
        .map(|target| target.nr_sectors)
        .unwrap_or(0)
}

fn dm_linear_block_size(bdev: &BlockDeviceRef) -> u32 {
    dm_linear_backing(bdev)
        .map(|target| (target.parent.ops.block_size)(&target.parent))
        .unwrap_or(512)
}

fn dm_linear_ioctl(bdev: &BlockDeviceRef, cmd: u32, _arg: u64) -> Result<i64, i32> {
    if cmd != DM_MPATH_PROBE_PATHS_IOCTL {
        return Err(ENOTTY);
    }

    let target = dm_linear_backing(bdev)?;
    let mapped_name = target.mapped_name.as_deref().ok_or(ENOTTY)?;
    dm_multipath_probe_active_paths(mapped_name).map(|_| 0)
}

pub fn dm_multipath_block_device(
    mapped_name: &str,
    nr_sectors: u64,
) -> Result<BlockDeviceRef, i32> {
    if nr_sectors == 0 {
        return Err(EINVAL);
    }
    let backing = Arc::new(DmMultipathTarget {
        mapped_name: String::from(mapped_name),
        nr_sectors,
    });
    Ok(BlockDevice::wrap(backing, &DM_MULTIPATH_OPS))
}

pub fn register_dm_multipath(
    name: &str,
    aliases: &[&str],
    mapped_name: &str,
    nr_sectors: u64,
) -> Result<RegisteredDmDevice, i32> {
    let bdev = dm_multipath_block_device(mapped_name, nr_sectors)?;
    register_name(name, bdev.clone())?;
    register_gendisk(name, bdev.clone());

    let mut registered_aliases = Vec::new();
    for alias in aliases {
        register_name(alias, bdev.clone())?;
        registered_aliases.push(String::from(*alias));
    }

    Ok(RegisteredDmDevice {
        name: String::from(name),
        aliases: registered_aliases,
        bdev,
    })
}

fn dm_multipath_submit_bio(bdev: &BlockDeviceRef, bio: &BioRef) -> Result<(), i32> {
    let target = dm_multipath_backing(bdev)?;
    let sectors = (bio.total_size() as u64).div_ceil(512);
    let end = bio.sector.checked_add(sectors).ok_or(EIO)?;
    if end > target.nr_sectors {
        return Err(EIO);
    }
    let (parent, translated_sector) = dm_multipath_select_path(&target.mapped_name, bio.sector)?;

    match bio.op.0 {
        BIO_OP_READ => dm_linear_read(&parent, bio, translated_sector),
        BIO_OP_WRITE => dm_linear_write(&parent, bio, translated_sector),
        _ => {
            let parent_bio = bio_alloc(parent, bio.op, translated_sector);
            submit_bio(parent_bio)
        }
    }
}

fn dm_multipath_get_capacity(bdev: &BlockDeviceRef) -> u64 {
    dm_multipath_backing(bdev)
        .map(|target| target.nr_sectors)
        .unwrap_or(0)
}

fn dm_multipath_block_size(bdev: &BlockDeviceRef) -> u32 {
    let target = match dm_multipath_backing(bdev) {
        Ok(target) => target,
        Err(_) => return 512,
    };
    let parent = match dm_multipath_select_path(&target.mapped_name, 0) {
        Ok((parent, _)) => parent,
        Err(_) => return 512,
    };
    (parent.ops.block_size)(&parent)
}

fn dm_multipath_ioctl(bdev: &BlockDeviceRef, cmd: u32, _arg: u64) -> Result<i64, i32> {
    if cmd != DM_MPATH_PROBE_PATHS_IOCTL {
        return Err(ENOTTY);
    }

    let target = dm_multipath_backing(bdev)?;
    dm_multipath_probe_active_paths(&target.mapped_name).map(|_| 0)
}

fn dm_multipath_backing(bdev: &BlockDeviceRef) -> Result<Arc<DmMultipathTarget>, i32> {
    let backing = bdev.backing.lock().clone().ok_or(EIO)?;
    backing
        .downcast::<DmMultipathTarget>()
        .map_err(|_: Arc<dyn Any + Send + Sync>| EIO)
}

fn dm_multipath_select_path(name: &str, sector: u64) -> Result<(BlockDeviceRef, u64), i32> {
    let parent_name = {
        let mut devices = DM_CONTROL.lock();
        let dev = find_device_mut(&mut devices, name)?;
        if dev.suspended {
            return Err(EBUSY);
        }
        let DmTableSpecKind::Multipath(spec) = dev.active.as_ref().ok_or(EINVAL)? else {
            return Err(EINVAL);
        };
        if spec.valid_paths() == 0 {
            return Err(ENOTCONN);
        }
        let group = spec.active_group_from(spec.initial_group).ok_or(ENOTCONN)?;
        let active_indices = spec
            .paths
            .iter()
            .enumerate()
            .filter_map(|(idx, path)| {
                if path.group == group && path.active {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let path_slot = dev.next_path % active_indices.len();
        let idx = active_indices[path_slot];
        dev.next_path = (path_slot + 1) % active_indices.len();
        spec.paths[idx].parent_name.clone()
    };

    let parent = lookup_block_device(&parent_name).ok_or(ENOENT)?;
    Ok((parent, sector))
}

fn dm_linear_backing(bdev: &BlockDeviceRef) -> Result<Arc<DmLinearTarget>, i32> {
    let backing = bdev.backing.lock().clone().ok_or(EIO)?;
    backing
        .downcast::<DmLinearTarget>()
        .map_err(|_: Arc<dyn Any + Send + Sync>| EIO)
}

static DM_LINEAR_OPS: BlockDeviceOps = BlockDeviceOps {
    name: "dm-linear",
    submit_bio: dm_linear_submit_bio,
    get_capacity: dm_linear_get_capacity,
    block_size: dm_linear_block_size,
    ioctl: Some(dm_linear_ioctl),
};

static DM_MULTIPATH_OPS: BlockDeviceOps = BlockDeviceOps {
    name: "dm-multipath",
    submit_bio: dm_multipath_submit_bio,
    get_capacity: dm_multipath_get_capacity,
    block_size: dm_multipath_block_size,
    ioctl: Some(dm_multipath_ioctl),
};

fn validate_dm_name(name: &str) -> Result<(), i32> {
    if name.is_empty() || name.len() >= DM_NAME_LEN || name.contains('/') {
        return Err(EINVAL);
    }
    Ok(())
}

fn validate_dm_path(path: &str) -> Result<(), i32> {
    if path.is_empty() || path.len() > 255 || path.bytes().any(|byte| byte == 0) {
        return Err(EINVAL);
    }
    Ok(())
}

fn validate_dm_uuid(uuid: &str) -> Result<(), i32> {
    if uuid.len() >= DM_UUID_LEN {
        return Err(EINVAL);
    }
    Ok(())
}

fn validate_linear_spec(spec: &DmLinearTableSpec) -> Result<(), i32> {
    validate_linear_table_specs(core::slice::from_ref(spec))
}

fn validate_linear_table_specs(specs: &[DmLinearTableSpec]) -> Result<(), i32> {
    if specs.is_empty() || specs.len() > DM_MAX_TABLE_TARGETS {
        return Err(EINVAL);
    }
    let mut expected_start = 0u64;
    for spec in specs {
        if spec.sector_start != expected_start || spec.length == 0 {
            return Err(EINVAL);
        }
        let _ = spec.target_start.checked_add(spec.length).ok_or(EIO)?;
        let parent = lookup_block_device(&spec.parent_name).ok_or(ENOENT)?;
        let capacity = parent.capacity_sectors();
        if capacity != 0 && spec.target_start + spec.length > capacity {
            return Err(EIO);
        }
        expected_start = expected_start.checked_add(spec.length).ok_or(EIO)?;
    }
    Ok(())
}

fn validate_linear_target_segments(segments: &[DmLinearTargetSegment]) -> Result<(), i32> {
    if segments.is_empty() || segments.len() > DM_MAX_TABLE_TARGETS {
        return Err(EINVAL);
    }
    let mut expected_start = 0u64;
    for segment in segments {
        if segment.sector_start != expected_start || segment.length == 0 {
            return Err(EINVAL);
        }
        let end = segment
            .target_start
            .checked_add(segment.length)
            .ok_or(EIO)?;
        let parent_capacity = segment.parent.capacity_sectors();
        if parent_capacity != 0 && end > parent_capacity {
            return Err(EIO);
        }
        expected_start = expected_start.checked_add(segment.length).ok_or(EIO)?;
    }
    Ok(())
}

fn linear_specs_length(specs: &[DmLinearTableSpec]) -> u64 {
    specs
        .last()
        .map(|spec| spec.sector_start.saturating_add(spec.length))
        .unwrap_or(0)
}

fn linear_target_segments_length(segments: &[DmLinearTargetSegment]) -> u64 {
    segments
        .last()
        .map(|segment| segment.sector_start.saturating_add(segment.length))
        .unwrap_or(0)
}

fn validate_multipath_spec(spec: &DmMultipathTableSpec) -> Result<(), i32> {
    if spec.sector_start != 0 || spec.length == 0 || spec.paths.is_empty() {
        return Err(EINVAL);
    }
    if spec.groups.is_empty() || spec.initial_group >= spec.groups.len() {
        return Err(EINVAL);
    }
    for group in &spec.groups {
        if group.selector.is_empty() || group.selector.len() >= DM_MAX_TYPE_NAME {
            return Err(EINVAL);
        }
    }
    for path in &spec.paths {
        if path.group >= spec.groups.len()
            || path.per_path_args.len() != spec.groups[path.group].per_path_arg_count
        {
            return Err(EINVAL);
        }
        validate_dm_path(&path.parent_name)?;
        let parent = lookup_block_device(&path.parent_name).ok_or(ENOENT)?;
        let capacity = parent.capacity_sectors();
        if capacity != 0 && spec.length > capacity {
            return Err(EIO);
        }
    }
    if spec
        .groups
        .iter()
        .enumerate()
        .any(|(idx, _)| !spec.paths.iter().any(|path| path.group == idx))
    {
        return Err(EINVAL);
    }
    Ok(())
}

fn dm_path_name_matches(parent_name: &str, path: &str) -> bool {
    parent_name == path
        || path
            .strip_prefix("/dev/")
            .is_some_and(|short| short == parent_name)
}

fn push_unique_metadata(metadata: &mut Vec<String>, text: String) {
    if !metadata.iter().any(|existing| existing == &text) {
        metadata.push(text);
    }
}

fn push_unique_status(statuses: &mut Vec<DmDeviceStatus>, status: DmDeviceStatus) {
    if !statuses.iter().any(|existing| existing.name == status.name) {
        statuses.push(status);
    }
}

fn lvm_label_header_matches(label: &[u8], expected_sector: u64) -> Result<bool, i32> {
    if label.len() < LVM_LABEL_HEADER_SIZE {
        return Err(EINVAL);
    }
    if label.get(0..8) != Some(LVM_LABEL_ID.as_slice())
        || label.get(24..32) != Some(LVM_LABEL_TYPE.as_slice())
    {
        return Ok(false);
    }
    let sector = le_u64_at(label, 8)?;
    if sector != expected_sector {
        return Ok(false);
    }
    let offset = le_u32_at(label, 20)? as usize;
    Ok(offset >= LVM_LABEL_HEADER_SIZE && offset + LVM_PV_HEADER_SIZE <= LVM_SECTOR_SIZE)
}

fn lvm_parse_pv_metadata_areas(label: &[u8], pv_offset: usize) -> Result<Vec<(u64, u64)>, i32> {
    if pv_offset + LVM_PV_HEADER_SIZE > label.len() {
        return Err(EINVAL);
    }

    let mut out = Vec::new();
    let mut off = pv_offset + LVM_PV_HEADER_SIZE;
    let mut in_metadata_areas = false;
    let mut entries = 0usize;
    while off + LVM_DISK_LOCN_SIZE <= label.len() && entries < 32 {
        entries += 1;
        let loc = le_u64_at(label, off)?;
        let size = le_u64_at(label, off + 8)?;
        off += LVM_DISK_LOCN_SIZE;

        if loc == 0 && size == 0 {
            if in_metadata_areas {
                break;
            }
            in_metadata_areas = true;
            continue;
        }
        if in_metadata_areas {
            if loc % LVM_SECTOR_SIZE as u64 != 0 || size < LVM_SECTOR_SIZE as u64 {
                return Err(EINVAL);
            }
            out.push((loc, size));
        }
    }

    if out.is_empty() {
        return Err(ENOENT);
    }
    Ok(out)
}

fn lvm_read_metadata_area_text(
    bdev: &BlockDeviceRef,
    mda_offset: u64,
    mda_size: u64,
) -> Result<Option<String>, i32> {
    let header = lvm_read_bytes(bdev, mda_offset, LVM_SECTOR_SIZE)?;
    if header.len() < LVM_MDA_HEADER_SIZE
        || header.get(4..20) != Some(LVM_MDA_MAGIC.as_slice())
        || le_u32_at(&header, 20)? == 0
    {
        return Ok(None);
    }

    let header_start = le_u64_at(&header, 24)?;
    let header_size = le_u64_at(&header, 32)?;
    if header_start != mda_offset || header_size > mda_size || header_size < LVM_SECTOR_SIZE as u64
    {
        return Err(EINVAL);
    }

    let mut off = LVM_MDA_HEADER_SIZE;
    let mut entries = 0usize;
    while off + LVM_RAW_LOCN_SIZE <= header.len() && entries < 8 {
        entries += 1;
        let raw_offset = le_u64_at(&header, off)?;
        let raw_size = le_u64_at(&header, off + 8)?;
        let raw_checksum = le_u32_at(&header, off + 16)?;
        let raw_flags = le_u32_at(&header, off + 20)?;
        off += LVM_RAW_LOCN_SIZE;

        if raw_offset == 0 && raw_size == 0 && raw_checksum == 0 && raw_flags == 0 {
            break;
        }
        if raw_flags & LVM_RAW_LOCN_IGNORED != 0 {
            continue;
        }
        let text_size = if raw_size == 0 {
            header_size.checked_sub(raw_offset).ok_or(EINVAL)?
        } else {
            raw_size
        };
        if text_size == 0
            || text_size as usize > LVM_METADATA_TEXT_LIMIT
            || raw_offset < LVM_SECTOR_SIZE as u64
            || raw_offset.checked_add(text_size).ok_or(EIO)? > header_size
        {
            return Err(EINVAL);
        }

        let text = lvm_read_bytes(
            bdev,
            mda_offset.checked_add(raw_offset).ok_or(EIO)?,
            text_size as usize,
        )?;
        if let Some(parsed) = lvm_text_from_raw_location(&text)? {
            return Ok(Some(parsed));
        }
    }
    Ok(None)
}

fn lvm_text_from_raw_location(raw: &[u8]) -> Result<Option<String>, i32> {
    let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    let text = core::str::from_utf8(&raw[..end])
        .map_err(|_| EINVAL)?
        .trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(String::from(text)))
}

fn lvm_read_bytes(
    bdev: &BlockDeviceRef,
    byte_offset: u64,
    byte_len: usize,
) -> Result<Vec<u8>, i32> {
    if byte_len == 0 || byte_offset % LVM_SECTOR_SIZE as u64 != 0 {
        return Err(EINVAL);
    }
    let sectors = (byte_len as u64).div_ceil(LVM_SECTOR_SIZE as u64);
    let mut bytes =
        super::partitions::read_sectors(bdev, byte_offset / LVM_SECTOR_SIZE as u64, sectors)?;
    bytes.truncate(byte_len);
    Ok(bytes)
}

fn le_u32_at(bytes: &[u8], off: usize) -> Result<u32, i32> {
    let raw = bytes.get(off..off + 4).ok_or(EINVAL)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn le_u64_at(bytes: &[u8], off: usize) -> Result<u64, i32> {
    let raw = bytes.get(off..off + 8).ok_or(EINVAL)?;
    Ok(u64::from_le_bytes([
        raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
    ]))
}

#[derive(Clone, Debug, Default)]
struct LvmParsedMetadata {
    vg_name: String,
    extent_size: u64,
    pvs: Vec<LvmPvMetadata>,
    lvs: Vec<LvmLvMetadata>,
}

#[derive(Clone, Debug, Default)]
struct LvmPvMetadata {
    label: String,
    device: String,
    pe_start: u64,
}

#[derive(Clone, Debug, Default)]
struct LvmLvMetadata {
    name: String,
    segment_count: u64,
    segments: Vec<LvmSegmentMetadata>,
    current_segment: Option<LvmSegmentMetadata>,
}

#[derive(Clone, Debug, Default)]
struct LvmSegmentMetadata {
    start_extent: u64,
    extent_count: u64,
    pv_label: String,
    pv_start_extent: u64,
}

impl LvmParsedMetadata {
    fn logical_volumes(&self) -> Result<Vec<LvmLogicalVolumeSpec>, i32> {
        if self.vg_name.is_empty() || self.extent_size == 0 {
            return Err(EINVAL);
        }
        let mut out = Vec::new();
        for lv in &self.lvs {
            if lv.name.is_empty()
                || lv.segment_count == 0
                || lv.segment_count as usize != lv.segments.len()
            {
                return Err(EINVAL);
            }
            let mut segments = Vec::new();
            for segment in &lv.segments {
                if segment.extent_count == 0 || segment.pv_label.is_empty() {
                    return Err(EINVAL);
                }
                let pv = self
                    .pvs
                    .iter()
                    .find(|pv| pv.label == segment.pv_label)
                    .ok_or(ENOENT)?;
                if pv.device.is_empty() {
                    return Err(EINVAL);
                }
                let sector_start = segment
                    .start_extent
                    .checked_mul(self.extent_size)
                    .ok_or(EIO)?;
                let target_start = pv
                    .pe_start
                    .checked_add(
                        segment
                            .pv_start_extent
                            .checked_mul(self.extent_size)
                            .ok_or(EIO)?,
                    )
                    .ok_or(EIO)?;
                let length = segment
                    .extent_count
                    .checked_mul(self.extent_size)
                    .ok_or(EIO)?;
                segments.push(DmLinearTableSpec::new(
                    &normalize_lvm_device(&pv.device),
                    sector_start,
                    length,
                    target_start,
                ));
            }
            validate_lvm_linear_segments(&segments)?;
            let first = segments.first().ok_or(EINVAL)?;
            out.push(LvmLogicalVolumeSpec {
                vg_name: self.vg_name.clone(),
                lv_name: lv.name.clone(),
                dm_name: lvm_dm_name(&self.vg_name, &lv.name),
                parent_name: first.parent_name.clone(),
                sector_start: first.sector_start,
                length: linear_specs_length(&segments),
                target_start: first.target_start,
                segments,
            });
        }
        Ok(out)
    }
}

fn parse_lvm_metadata(metadata: &str) -> Result<LvmParsedMetadata, i32> {
    let mut parsed = LvmParsedMetadata::default();
    let mut stack = Vec::<String>::new();
    let mut current_pv: Option<LvmPvMetadata> = None;
    let mut current_lv: Option<LvmLvMetadata> = None;
    let mut in_stripes = false;
    let mut stripes = String::new();

    for raw in metadata.lines() {
        let line = trim_lvm_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if in_stripes {
            stripes.push(' ');
            stripes.push_str(line);
            if line.contains(']') {
                apply_lvm_stripes(&mut current_lv, &stripes)?;
                stripes.clear();
                in_stripes = false;
            }
            continue;
        }
        if let Some(name) = line.strip_suffix('{') {
            let name = name.trim();
            if stack.is_empty() {
                parsed.vg_name = String::from(name);
            } else if stack
                .last()
                .is_some_and(|section| section == "physical_volumes")
            {
                current_pv = Some(LvmPvMetadata {
                    label: String::from(name),
                    ..LvmPvMetadata::default()
                });
            } else if stack
                .last()
                .is_some_and(|section| section == "logical_volumes")
            {
                current_lv = Some(LvmLvMetadata {
                    name: String::from(name),
                    ..LvmLvMetadata::default()
                });
            } else if stack.len() == 3 && stack[1] == "logical_volumes" {
                let lv = current_lv.as_mut().ok_or(EINVAL)?;
                if !name.starts_with("segment") || lv.current_segment.is_some() {
                    return Err(EINVAL);
                }
                lv.current_segment = Some(LvmSegmentMetadata::default());
            }
            stack.push(String::from(name));
            continue;
        }
        if line == "}" {
            let ending = stack.pop().ok_or(EINVAL)?;
            if stack.len() == 3 && stack[1] == "logical_volumes" && ending.starts_with("segment") {
                let lv = current_lv.as_mut().ok_or(EINVAL)?;
                let segment = lv.current_segment.take().ok_or(EINVAL)?;
                lv.segments.push(segment);
            } else if stack
                .last()
                .is_some_and(|section| section == "physical_volumes")
            {
                let pv = current_pv.take().ok_or(EINVAL)?;
                if pv.label == ending {
                    parsed.pvs.push(pv);
                }
            } else if stack
                .last()
                .is_some_and(|section| section == "logical_volumes")
            {
                let lv = current_lv.take().ok_or(EINVAL)?;
                if lv.current_segment.is_some() {
                    return Err(EINVAL);
                }
                if lv.name == ending {
                    parsed.lvs.push(lv);
                }
            }
            continue;
        }

        let (key, value) = split_lvm_assignment(line)?;
        if stack.len() == 1 && key == "extent_size" {
            parsed.extent_size = parse_lvm_u64(value)?;
        } else if stack.len() == 3 && stack[1] == "physical_volumes" {
            if let Some(pv) = current_pv.as_mut() {
                match key {
                    "device" => pv.device = parse_lvm_string(value)?,
                    "pe_start" => pv.pe_start = parse_lvm_u64(value)?,
                    _ => {}
                }
            }
        } else if stack.len() == 3 && stack[1] == "logical_volumes" {
            if let Some(lv) = current_lv.as_mut() {
                if key == "segment_count" {
                    lv.segment_count = parse_lvm_u64(value)?;
                }
            }
        } else if stack.len() == 4 && stack[1] == "logical_volumes" {
            if let Some(lv) = current_lv.as_mut() {
                let segment = lv.current_segment.as_mut().ok_or(EINVAL)?;
                match key {
                    "start_extent" => segment.start_extent = parse_lvm_u64(value)?,
                    "extent_count" => segment.extent_count = parse_lvm_u64(value)?,
                    "stripes" => {
                        stripes.push_str(value);
                        if value.contains(']') {
                            apply_lvm_stripes(&mut current_lv, &stripes)?;
                            stripes.clear();
                        } else {
                            in_stripes = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    if !stack.is_empty() || in_stripes {
        return Err(EINVAL);
    }
    Ok(parsed)
}

fn trim_lvm_comment(line: &str) -> &str {
    line.split_once('#').map(|(head, _)| head).unwrap_or(line)
}

fn split_lvm_assignment(line: &str) -> Result<(&str, &str), i32> {
    let (key, value) = line.split_once('=').ok_or(EINVAL)?;
    Ok((key.trim(), value.trim()))
}

fn parse_lvm_string(value: &str) -> Result<String, i32> {
    let value = value.trim();
    let value = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or(EINVAL)?;
    Ok(String::from(value))
}

fn parse_lvm_u64(value: &str) -> Result<u64, i32> {
    value.trim().parse::<u64>().map_err(|_| EINVAL)
}

fn apply_lvm_stripes(current_lv: &mut Option<LvmLvMetadata>, stripes: &str) -> Result<(), i32> {
    let lv = current_lv.as_mut().ok_or(EINVAL)?;
    let segment = lv.current_segment.as_mut().ok_or(EINVAL)?;
    let body = stripes
        .trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or(EINVAL)?;
    let mut pieces = body.split(',').map(str::trim);
    segment.pv_label = parse_lvm_string(pieces.next().ok_or(EINVAL)?)?;
    segment.pv_start_extent = parse_lvm_u64(pieces.next().ok_or(EINVAL)?)?;
    if pieces.next().is_some() {
        return Err(EINVAL);
    }
    Ok(())
}

fn validate_lvm_linear_segments(segments: &[DmLinearTableSpec]) -> Result<(), i32> {
    if segments.is_empty() || segments.len() > DM_MAX_TABLE_TARGETS {
        return Err(EINVAL);
    }
    let mut expected_start = 0u64;
    for segment in segments {
        if segment.sector_start != expected_start || segment.length == 0 {
            return Err(EINVAL);
        }
        let _ = segment
            .target_start
            .checked_add(segment.length)
            .ok_or(EIO)?;
        expected_start = expected_start.checked_add(segment.length).ok_or(EIO)?;
    }
    Ok(())
}

fn normalize_lvm_device(device: &str) -> String {
    String::from(device.strip_prefix("/dev/").unwrap_or(device))
}

fn lvm_dm_name(vg: &str, lv: &str) -> String {
    format!("{}-{}", lvm_escape_name(vg), lvm_escape_name(lv))
}

fn lvm_escape_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch == '-' {
            out.push('-');
        }
        out.push(ch);
    }
    out
}

fn find_device<'a>(devices: &'a [DmMappedDevice], name: &str) -> Result<&'a DmMappedDevice, i32> {
    devices
        .iter()
        .find(|dev| dev.name == name || (!name.is_empty() && dev.uuid == name))
        .ok_or(ENOENT)
}

fn find_device_mut<'a>(
    devices: &'a mut [DmMappedDevice],
    name: &str,
) -> Result<&'a mut DmMappedDevice, i32> {
    devices
        .iter_mut()
        .find(|dev| dev.name == name || (!name.is_empty() && dev.uuid == name))
        .ok_or(ENOENT)
}

fn status_from_device(dev: &DmMappedDevice) -> DmDeviceStatus {
    let table_spec = dev.active.as_ref();
    let table = table_spec.and_then(|spec| spec.table_statuses().into_iter().next());
    DmDeviceStatus {
        version: dm_version(),
        name: dev.name.clone(),
        uuid: dev.uuid.clone(),
        minor: dev.minor,
        dev: encoded_dm_dev(dev.minor),
        flags: if dev.suspended { DM_SUSPEND_FLAG } else { 0 },
        event_nr: dev.event_nr,
        suspended: dev.suspended,
        active: dev.active.is_some(),
        inactive: dev.inactive.is_some(),
        open_count: 0,
        target_count: table_spec.map(DmTableSpecKind::target_count).unwrap_or(0),
        valid_paths: dev.valid_paths,
        table,
        geometry: dev.geometry.clone(),
    }
}

fn encoded_dm_dev(minor: u32) -> u64 {
    (u64::from(DM_MAJOR) << 20) | u64::from(minor)
}

fn emit_dm_uevent(
    dev: &mut DmMappedDevice,
    action: crate::net::uevent::UeventAction,
    cookie: u32,
    resize: bool,
) {
    emit_dm_uevent_with_extra(dev, action, cookie, resize, &[]);
}

fn emit_dm_uevent_with_extra(
    dev: &mut DmMappedDevice,
    action: crate::net::uevent::UeventAction,
    cookie: u32,
    resize: bool,
    extra_properties: &[(&str, &str)],
) {
    dev.uevent_seq = dev.uevent_seq.saturating_add(1);

    let devname = format!("dm-{}", dev.minor);
    let devpath = format!("/devices/virtual/block/{devname}");
    let minor = format!("{}", dev.minor);
    let seq = format!("{}", dev.uevent_seq);
    let suspended = if dev.suspended { "1" } else { "0" };
    let cookie_string = format!("{cookie}");
    let resize_string = if resize { "1" } else { "0" };

    let mut properties = alloc::vec![
        ("SUBSYSTEM", "block"),
        ("DEVTYPE", "disk"),
        ("DEVNAME", devname.as_str()),
        ("MAJOR", "253"),
        ("MINOR", minor.as_str()),
        ("DM_NAME", dev.name.as_str()),
        ("DM_UUID", dev.uuid.as_str()),
        ("DM_SEQNUM", seq.as_str()),
        ("DM_SUSPENDED", suspended),
    ];
    if cookie != 0 {
        properties.push(("DM_COOKIE", cookie_string.as_str()));
    }
    if resize {
        properties.push(("RESIZE", resize_string));
    }
    for (key, value) in extra_properties {
        properties.push((*key, *value));
    }

    let msg = crate::net::uevent::UeventMessage::build(action, &devpath, &properties);
    crate::net::uevent::broadcast_uevent(msg);
}

fn fill_ioctl_status(ioctl: &mut DmIoctl, status: &DmDeviceStatus) {
    ioctl.version = status.version;
    ioctl.open_count = status.open_count;
    ioctl.flags = (ioctl.flags & !DM_SUSPEND_FLAG) | status.flags;
    ioctl.event_nr = status.event_nr;
    ioctl.dev = status.dev;
    ioctl.target_count = status.target_count;
    write_fixed_cstr(&mut ioctl.name, &status.name);
    write_fixed_cstr(&mut ioctl.uuid, &status.uuid);
}

fn active_or_inactive_table_for(name: &str) -> Result<Option<DmTableSpecKind>, i32> {
    let devices = DM_CONTROL.lock();
    let dev = find_device(&devices, name)?;
    Ok(dev.active.as_ref().or(dev.inactive.as_ref()).cloned())
}

fn parent_devs_for_table(spec: &DmTableSpecKind) -> Result<Vec<u64>, i32> {
    match spec {
        DmTableSpecKind::Linear(linear) => {
            let parent = lookup_block_device(&linear.parent_name).ok_or(ENOENT)?;
            Ok(alloc::vec![parent.id])
        }
        DmTableSpecKind::LinearTargets(linear) => {
            let mut deps = Vec::new();
            for spec in linear {
                let parent = lookup_block_device(&spec.parent_name).ok_or(ENOENT)?;
                if !deps.iter().any(|dev| *dev == parent.id) {
                    deps.push(parent.id);
                }
            }
            Ok(deps)
        }
        DmTableSpecKind::Multipath(multipath) => {
            let mut deps = Vec::new();
            for path in &multipath.paths {
                let parent = lookup_block_device(&path.parent_name).ok_or(ENOENT)?;
                deps.push(parent.id);
            }
            Ok(deps)
        }
    }
}

fn fixed_cstr_to_string(bytes: &[u8]) -> Result<String, i32> {
    let len = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..len])
        .map(String::from)
        .map_err(|_| EINVAL)
}

fn write_fixed_cstr(dst: &mut [u8], value: &str) {
    dst.fill(0);
    let n = value.len().min(dst.len().saturating_sub(1));
    dst[..n].copy_from_slice(&value.as_bytes()[..n]);
}

fn read_struct<T: Copy>(buf: &[u8], offset: usize) -> Result<T, i32> {
    let end = offset.checked_add(size_of::<T>()).ok_or(EINVAL)?;
    let src = buf.get(offset..end).ok_or(EINVAL)?;
    let mut value = MaybeUninit::<T>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), value.as_mut_ptr().cast::<u8>(), src.len());
        Ok(value.assume_init())
    }
}

fn write_struct<T: Copy>(buf: &mut [u8], offset: usize, value: &T) -> Result<(), i32> {
    let end = offset.checked_add(size_of::<T>()).ok_or(EINVAL)?;
    let dst = buf.get_mut(offset..end).ok_or(EINVAL)?;
    unsafe {
        core::ptr::copy_nonoverlapping(
            (value as *const T).cast::<u8>(),
            dst.as_mut_ptr(),
            dst.len(),
        );
    }
    Ok(())
}

fn parse_table_from_ioctl(buf: &[u8], ioctl: &DmIoctl) -> Result<DmTableSpecKind, i32> {
    if ioctl.target_count == 0 || ioctl.target_count as usize > DM_MAX_TABLE_TARGETS {
        return Err(EINVAL);
    }
    let data_end = (ioctl.data_size as usize).min(buf.len());
    let mut off = ioctl.data_start as usize;
    let mut linear_specs = Vec::new();
    let mut multipath = None;

    for idx in 0..ioctl.target_count as usize {
        let target: DmTargetSpec = read_struct(buf, off)?;
        let target_type = fixed_cstr_to_string(&target.target_type)?;
        let params_start = off.checked_add(size_of::<DmTargetSpec>()).ok_or(EINVAL)?;
        let params_end = if target.next == 0 {
            if idx + 1 != ioctl.target_count as usize {
                return Err(EINVAL);
            }
            data_end
        } else {
            off.checked_add(target.next as usize)
                .ok_or(EINVAL)?
                .min(data_end)
        };
        if params_start >= params_end {
            return Err(EINVAL);
        }
        let params = cstr_from_range(&buf[params_start..params_end])?;

        match target_type.as_str() {
            "linear" => {
                if multipath.is_some() {
                    return Err(EINVAL);
                }
                linear_specs.push(parse_linear_table_params(&target, params)?);
            }
            "multipath" => {
                if ioctl.target_count != 1 || !linear_specs.is_empty() {
                    return Err(EINVAL);
                }
                multipath = Some(parse_multipath_table_params(&target, params)?);
            }
            _ => return Err(EINVAL),
        }

        if target.next == 0 {
            off = data_end;
        } else {
            off = off.checked_add(target.next as usize).ok_or(EINVAL)?;
        }
        if off > data_end {
            return Err(EINVAL);
        }
    }

    if let Some(multipath) = multipath {
        return Ok(DmTableSpecKind::Multipath(multipath));
    }
    validate_linear_table_specs(&linear_specs)?;
    if linear_specs.len() == 1 {
        Ok(DmTableSpecKind::Linear(linear_specs.remove(0)))
    } else {
        Ok(DmTableSpecKind::LinearTargets(linear_specs))
    }
}

fn parse_linear_table_params(
    target: &DmTargetSpec,
    params: &str,
) -> Result<DmLinearTableSpec, i32> {
    let mut parts = params.split_whitespace();
    let parent = parts.next().ok_or(EINVAL)?;
    let start = parse_u64_arg(parts.next().ok_or(EINVAL)?)?;
    if parts.next().is_some() {
        return Err(EINVAL);
    }

    Ok(DmLinearTableSpec::new(
        parent,
        target.sector_start,
        target.length,
        start,
    ))
}

fn parse_multipath_table_params(
    target: &DmTargetSpec,
    params: &str,
) -> Result<DmMultipathTableSpec, i32> {
    let tokens = params.split_whitespace().collect::<Vec<_>>();
    let mut idx = 0usize;

    let feature_args = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
    let features = take_tokens(&tokens, &mut idx, feature_args)?;

    let handler_args = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
    let hardware_handler = take_tokens(&tokens, &mut idx, handler_args)?;

    let nr_groups = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
    let initial_group = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
    if nr_groups == 0 || initial_group == 0 || initial_group > nr_groups {
        return Err(EINVAL);
    }

    let mut groups = Vec::new();
    let mut paths = Vec::new();
    let mut selector_name = None;
    for group_idx in 0..nr_groups {
        let selector = next_token(&tokens, &mut idx)?;
        if group_idx == initial_group - 1 {
            selector_name = Some(String::from(selector));
        }
        let selector_args = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
        let selector_arg_values = take_tokens(&tokens, &mut idx, selector_args)?;

        let nr_paths = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
        let per_path_args = parse_usize_arg(next_token(&tokens, &mut idx)?)?;
        groups.push(DmMultipathPriorityGroup {
            selector: String::from(selector),
            selector_args: selector_arg_values,
            per_path_arg_count: per_path_args,
            bypassed: false,
        });
        for _ in 0..nr_paths {
            let path = next_token(&tokens, &mut idx)?;
            let path_args = take_tokens(&tokens, &mut idx, per_path_args)?;
            paths.push(DmMultipathPathSpec::with_group(path, group_idx, path_args));
        }
    }
    if idx != tokens.len() || paths.is_empty() {
        return Err(EINVAL);
    }

    Ok(DmMultipathTableSpec {
        sector_start: target.sector_start,
        length: target.length,
        features,
        hardware_handler,
        initial_group: initial_group - 1,
        selector: selector_name.ok_or(EINVAL)?,
        groups,
        paths,
    })
}

fn next_token<'a>(tokens: &'a [&str], idx: &mut usize) -> Result<&'a str, i32> {
    let token = *tokens.get(*idx).ok_or(EINVAL)?;
    *idx += 1;
    Ok(token)
}

fn skip_tokens(tokens: &[&str], idx: &mut usize, count: usize) -> Result<(), i32> {
    let end = idx.checked_add(count).ok_or(EINVAL)?;
    if end > tokens.len() {
        return Err(EINVAL);
    }
    *idx = end;
    Ok(())
}

fn take_tokens(tokens: &[&str], idx: &mut usize, count: usize) -> Result<Vec<String>, i32> {
    let start = *idx;
    skip_tokens(tokens, idx, count)?;
    Ok(tokens[start..*idx]
        .iter()
        .map(|token| String::from(*token))
        .collect())
}

fn parse_usize_arg(value: &str) -> Result<usize, i32> {
    value.parse::<usize>().map_err(|_| EINVAL)
}

fn parse_u64_arg(value: &str) -> Result<u64, i32> {
    value.parse::<u64>().map_err(|_| EINVAL)
}

fn cstr_from_ioctl_data<'a>(buf: &'a [u8], ioctl: &DmIoctl) -> Result<&'a str, i32> {
    let start = ioctl.data_start as usize;
    let end = (ioctl.data_size as usize).min(buf.len());
    if start >= end {
        return Err(EINVAL);
    }
    cstr_from_range(&buf[start..end])
}

fn cstr_from_range(bytes: &[u8]) -> Result<&str, i32> {
    let len = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    core::str::from_utf8(&bytes[..len]).map_err(|_| EINVAL)
}

fn write_table_deps_to_ioctl(buf: &mut [u8], ioctl: &mut DmIoctl, name: &str) -> Result<(), i32> {
    let off = ioctl.data_start as usize;
    let table = active_or_inactive_table_for(name)?;
    let deps = match table.as_ref() {
        Some(table) => parent_devs_for_table(table)?,
        None => Vec::new(),
    };
    let count = deps.len();
    let total = size_of::<DmTargetDeps>() + count * size_of::<u64>();
    let end = off.checked_add(total).ok_or(EINVAL)?;
    if end > buf.len() {
        ioctl.flags |= DM_BUFFER_FULL_FLAG;
        return Ok(());
    }

    write_struct(
        buf,
        off,
        &DmTargetDeps {
            count: count as u32,
            padding: 0,
        },
    )?;
    for (idx, dev) in deps.iter().enumerate() {
        put_u64(
            buf,
            off + size_of::<DmTargetDeps>() + idx * size_of::<u64>(),
            *dev,
        )?;
    }
    ioctl.data_size = end as u32;
    Ok(())
}

fn write_target_versions_to_ioctl(
    buf: &mut [u8],
    ioctl: &mut DmIoctl,
    only_name: Option<&str>,
) -> Result<(), i32> {
    let off = ioctl.data_start as usize;
    let targets = dm_target_version_entries();
    let selected: Vec<(&str, [u32; 3])> = match only_name {
        Some(name) => {
            let entry = targets
                .iter()
                .find(|(target_name, _)| *target_name == name)
                .copied()
                .ok_or(EINVAL)?;
            alloc::vec![entry]
        }
        None => targets.to_vec(),
    };

    let total = selected
        .iter()
        .map(|(name, _)| align_up(size_of::<DmTargetVersions>() + name.len() + 1, 8))
        .sum::<usize>();
    let end = off.checked_add(total).ok_or(EINVAL)?;
    if end > buf.len() {
        ioctl.flags |= DM_BUFFER_FULL_FLAG;
        return Ok(());
    }

    let mut cursor = off;
    for (idx, (name, version)) in selected.iter().enumerate() {
        let entry_len = align_up(size_of::<DmTargetVersions>() + name.len() + 1, 8);
        let next = if idx + 1 == selected.len() {
            0
        } else {
            entry_len as u32
        };
        write_struct(
            buf,
            cursor,
            &DmTargetVersions {
                next,
                version: *version,
            },
        )?;
        let name_start = cursor + size_of::<DmTargetVersions>();
        let entry_end = cursor + entry_len;
        buf[name_start..entry_end].fill(0);
        buf[name_start..name_start + name.len()].copy_from_slice(name.as_bytes());
        cursor = entry_end;
    }

    ioctl.target_count = selected.len() as u32;
    ioctl.data_size = end as u32;
    Ok(())
}

fn dm_target_version_entries() -> &'static [(&'static str, [u32; 3])] {
    &[
        ("linear", DM_LINEAR_TARGET_VERSION),
        ("multipath", DM_MULTIPATH_TARGET_VERSION),
    ]
}

fn parse_geometry_from_ioctl(buf: &[u8], ioctl: &DmIoctl) -> Result<DmGeometry, i32> {
    let geostr = cstr_from_ioctl_data(buf, ioctl)?;
    let mut parts = geostr.split_whitespace();
    let cylinders = parts
        .next()
        .ok_or(EINVAL)?
        .parse::<u64>()
        .map_err(|_| EINVAL)?;
    let heads = parts
        .next()
        .ok_or(EINVAL)?
        .parse::<u64>()
        .map_err(|_| EINVAL)?;
    let sectors = parts
        .next()
        .ok_or(EINVAL)?
        .parse::<u64>()
        .map_err(|_| EINVAL)?;
    let start = parts
        .next()
        .ok_or(EINVAL)?
        .parse::<u64>()
        .map_err(|_| EINVAL)?;
    if parts.next().is_some() || cylinders > u16::MAX as u64 || heads > 255 || sectors > 255 {
        return Err(EINVAL);
    }

    Ok(DmGeometry {
        cylinders: cylinders as u16,
        heads: heads as u8,
        sectors: sectors as u8,
        start,
    })
}

fn parse_target_msg_from_ioctl(buf: &[u8], ioctl: &DmIoctl) -> Result<DmParsedTargetMsg, i32> {
    let off = ioctl.data_start as usize;
    let msg: DmTargetMsg = read_struct(buf, off)?;
    let message_start = off.checked_add(size_of::<DmTargetMsg>()).ok_or(EINVAL)?;
    let end = (ioctl.data_size as usize).min(buf.len());
    if message_start >= end {
        return Err(EINVAL);
    }
    let message = cstr_from_range(&buf[message_start..end])?;
    if message.split_whitespace().next().is_none() {
        return Err(EINVAL);
    }
    Ok(DmParsedTargetMsg {
        sector: msg.sector,
        message: String::from(message),
    })
}

fn write_table_status_to_ioctl(
    buf: &mut [u8],
    ioctl: &mut DmIoctl,
    status: &DmDeviceStatus,
) -> Result<(), i32> {
    let table = active_or_inactive_table_for(&status.name)?.ok_or(EINVAL)?;
    let tables = table.table_statuses();
    let off = ioctl.data_start as usize;
    let spec_len = size_of::<DmTargetSpec>();
    let total = tables
        .iter()
        .map(|table| align_up(spec_len + table.params.len() + 1, 8))
        .sum::<usize>();
    let end = off.checked_add(total).ok_or(EINVAL)?;
    if end > buf.len() {
        return Err(EINVAL);
    }
    let mut cursor = off;
    for (idx, table) in tables.iter().enumerate() {
        let params = table.params.as_bytes();
        let entry_len = align_up(spec_len + params.len() + 1, 8);
        let entry_end = cursor.checked_add(entry_len).ok_or(EINVAL)?;
        let mut spec = DmTargetSpec {
            sector_start: table.sector_start,
            length: table.length,
            status: 0,
            next: if idx + 1 == tables.len() {
                0
            } else {
                entry_len as u32
            },
            target_type: [0; DM_MAX_TYPE_NAME],
        };
        write_fixed_cstr(&mut spec.target_type, table.target_type);
        write_struct(buf, cursor, &spec)?;
        let params_start = cursor + spec_len;
        buf[params_start..entry_end].fill(0);
        buf[params_start..params_start + params.len()].copy_from_slice(params);
        cursor = entry_end;
    }
    ioctl.target_count = tables.len() as u32;
    ioctl.data_size = end as u32;
    Ok(())
}

fn write_device_list_to_ioctl(buf: &mut [u8], ioctl: &mut DmIoctl) -> Result<(), i32> {
    let devices = dm_list_devices();
    let mut off = ioctl.data_start as usize;
    for (idx, dev) in devices.iter().enumerate() {
        let entry_len = align_up(8 + 4 + dev.name.len() + 1, 8);
        let end = off.checked_add(entry_len).ok_or(EINVAL)?;
        if end > buf.len() {
            return Err(EINVAL);
        }
        put_u64(buf, off, dev.dev)?;
        put_u32(
            buf,
            off + 8,
            if idx + 1 == devices.len() {
                0
            } else {
                entry_len as u32
            },
        )?;
        let name_start = off + 12;
        buf[name_start..end].fill(0);
        buf[name_start..name_start + dev.name.len()].copy_from_slice(dev.name.as_bytes());
        off = end;
    }
    ioctl.target_count = devices.len() as u32;
    ioctl.data_size = off as u32;
    Ok(())
}

fn put_u32(buf: &mut [u8], offset: usize, value: u32) -> Result<(), i32> {
    let bytes = value.to_ne_bytes();
    let dst = buf.get_mut(offset..offset + bytes.len()).ok_or(EINVAL)?;
    dst.copy_from_slice(&bytes);
    Ok(())
}

fn put_u64(buf: &mut [u8], offset: usize, value: u64) -> Result<(), i32> {
    let bytes = value.to_ne_bytes();
    let dst = buf.get_mut(offset..offset + bytes.len()).ok_or(EINVAL)?;
    dst.copy_from_slice(&bytes);
    Ok(())
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::bio::{BIO_OP_READ, BIO_OP_WRITE, BioOp, BioVec};
    use crate::block::mem::{MemBlockDevice, mem_block_device_ops};

    #[test]
    fn dm_linear_translates_reads_and_writes() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent0", 16 * 512),
            mem_block_device_ops(),
        );
        let dm = dm_linear_block_device(parent.clone(), 4, 4).expect("dm-linear");

        let parent_write = bio_alloc(parent.clone(), BioOp(BIO_OP_WRITE), 4);
        parent_write.add_vec(BioVec::new(alloc::vec![0x5a; 512]));
        submit_bio(parent_write).expect("seed parent");

        let dm_read = bio_alloc(dm.clone(), BioOp(BIO_OP_READ), 0);
        dm_read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(dm_read.clone()).expect("dm read");
        let read_vecs = dm_read.vecs.lock();
        let read_data = read_vecs[0].data.lock();
        assert!(read_data.iter().all(|byte| *byte == 0x5a));
        drop(read_data);
        drop(read_vecs);

        let dm_write = bio_alloc(dm, BioOp(BIO_OP_WRITE), 1);
        dm_write.add_vec(BioVec::new(alloc::vec![0xa5; 512]));
        submit_bio(dm_write).expect("dm write");

        let parent_read = bio_alloc(parent, BioOp(BIO_OP_READ), 5);
        parent_read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(parent_read.clone()).expect("parent read");
        let vecs = parent_read.vecs.lock();
        let data = vecs[0].data.lock();
        assert!(data.iter().all(|byte| *byte == 0xa5));
    }

    #[test]
    fn dm_linear_rejects_out_of_range_io_and_invalid_table() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent1", 8 * 512),
            mem_block_device_ops(),
        );
        assert_eq!(
            dm_linear_block_device(parent.clone(), 0, 0).err(),
            Some(EINVAL)
        );
        assert_eq!(
            dm_linear_block_device(parent.clone(), 7, 2).err(),
            Some(EIO)
        );

        let dm = dm_linear_block_device(parent, 2, 2).expect("dm-linear");
        let read = bio_alloc(dm, BioOp(BIO_OP_READ), 2);
        read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        assert_eq!(submit_bio(read), Err(EIO));
    }

    #[test]
    fn dm_linear_multi_target_table_routes_by_virtual_sector() {
        let parent_a = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37multia", 32 * 512),
            mem_block_device_ops(),
        );
        let parent_b = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37multib", 32 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37multia", parent_a.clone()).expect("register parent a");
        register_block_device("dmparent37multib", parent_b.clone()).expect("register parent b");

        dm_dev_create("cl-multi37", "").expect("dm create");
        dm_table_load_linear_targets(
            "cl-multi37",
            alloc::vec![
                DmLinearTableSpec::new("dmparent37multia", 0, 4, 8),
                DmLinearTableSpec::new("dmparent37multib", 4, 4, 12),
            ],
        )
        .expect("multi-target load");
        let resumed = dm_resume("cl-multi37").expect("dm resume");
        assert_eq!(resumed.target_count, 2);

        let dm = lookup_block_device("/dev/mapper/cl-multi37").expect("dm alias");
        write_sector(&dm, 1, 0x61);
        write_sector(&dm, 5, 0x62);
        assert_sector_byte(&parent_a, 9, 0x61);
        assert_sector_byte(&parent_b, 13, 0x62);

        let crossing = bio_alloc(dm, BioOp(BIO_OP_READ), 3);
        crossing.add_vec(BioVec::new(alloc::vec![0u8; 1024]));
        assert_eq!(submit_bio(crossing), Err(EIO));
    }

    #[test]
    fn dm_linear_registers_dev_mapper_alias() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent2", 16 * 512),
            mem_block_device_ops(),
        );
        let registered = register_dm_linear("dm-test37", &["mapper/cl-swap-test37"], parent, 0, 8)
            .expect("register dm-linear");

        assert_eq!(registered.name, "dm-test37");
        assert_eq!(registered.aliases, ["mapper/cl-swap-test37"]);
        assert_eq!(registered.bdev.capacity_sectors(), 8);
        assert!(lookup_block_device("dm-test37").is_some());
        assert!(lookup_block_device("/dev/mapper/cl-swap-test37").is_some());
    }

    #[test]
    fn dm_ioctl_uapi_layout_matches_linux() {
        assert_eq!(core::mem::size_of::<DmIoctl>(), 312);
        assert_eq!(core::mem::offset_of!(DmIoctl, version), 0);
        assert_eq!(core::mem::offset_of!(DmIoctl, data_size), 12);
        assert_eq!(core::mem::offset_of!(DmIoctl, data_start), 16);
        assert_eq!(core::mem::offset_of!(DmIoctl, target_count), 20);
        assert_eq!(core::mem::offset_of!(DmIoctl, open_count), 24);
        assert_eq!(core::mem::offset_of!(DmIoctl, flags), 28);
        assert_eq!(core::mem::offset_of!(DmIoctl, event_nr), 32);
        assert_eq!(core::mem::offset_of!(DmIoctl, dev), 40);
        assert_eq!(core::mem::offset_of!(DmIoctl, name), 48);
        assert_eq!(core::mem::offset_of!(DmIoctl, uuid), 176);
        assert_eq!(core::mem::offset_of!(DmIoctl, data), 305);

        assert_eq!(core::mem::size_of::<DmTargetSpec>(), 40);
        assert_eq!(core::mem::offset_of!(DmTargetSpec, sector_start), 0);
        assert_eq!(core::mem::offset_of!(DmTargetSpec, length), 8);
        assert_eq!(core::mem::offset_of!(DmTargetSpec, status), 16);
        assert_eq!(core::mem::offset_of!(DmTargetSpec, next), 20);
        assert_eq!(core::mem::offset_of!(DmTargetSpec, target_type), 24);

        assert_eq!(core::mem::size_of::<DmTargetDeps>(), 8);
        assert_eq!(core::mem::offset_of!(DmTargetDeps, count), 0);
        assert_eq!(core::mem::offset_of!(DmTargetDeps, padding), 4);

        assert_eq!(core::mem::size_of::<DmTargetVersions>(), 16);
        assert_eq!(core::mem::offset_of!(DmTargetVersions, next), 0);
        assert_eq!(core::mem::offset_of!(DmTargetVersions, version), 4);

        assert_eq!(core::mem::size_of::<DmTargetMsg>(), 8);
        assert_eq!(core::mem::offset_of!(DmTargetMsg, sector), 0);
        assert_eq!(DM_MPATH_PROBE_PATHS_IOCTL, 0xfd12);
    }

    #[test]
    fn dm_control_create_load_resume_registers_mapper_device() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ctl0", 32 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ctl0", parent.clone()).expect("register parent");

        let created = dm_dev_create("cl-swap-ctl37", "LVM-test-cl-swap-ctl37").expect("dm create");
        assert!(created.suspended);
        assert!(!created.active);
        assert_eq!(created.flags & DM_SUSPEND_FLAG, DM_SUSPEND_FLAG);

        let loaded = dm_table_load_linear(
            "cl-swap-ctl37",
            DmLinearTableSpec::new("dmparent37ctl0", 0, 8, 4),
        )
        .expect("dm table load");
        assert!(loaded.inactive);
        assert_eq!(loaded.target_count, 0);

        let resumed = dm_resume("cl-swap-ctl37").expect("dm resume");
        assert!(!resumed.suspended);
        assert!(resumed.active);
        assert!(!resumed.inactive);
        assert_eq!(resumed.target_count, 1);
        assert_eq!(resumed.table.as_ref().unwrap().target_type, "linear");
        assert_eq!(resumed.table.as_ref().unwrap().params, "dmparent37ctl0 4");
        assert!(lookup_block_device(&format!("dm-{}", resumed.minor)).is_some());
        assert!(lookup_block_device("/dev/mapper/cl-swap-ctl37").is_some());

        let dm = lookup_block_device("/dev/mapper/cl-swap-ctl37").expect("dm alias");
        assert_eq!(dm.capacity_sectors(), 8);

        let dm_write = bio_alloc(dm.clone(), BioOp(BIO_OP_WRITE), 0);
        dm_write.add_vec(BioVec::new(alloc::vec![0x37; 512]));
        submit_bio(dm_write).expect("dm write");

        let parent_read = bio_alloc(parent, BioOp(BIO_OP_READ), 4);
        parent_read.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(parent_read.clone()).expect("parent read");
        let vecs = parent_read.vecs.lock();
        let data = vecs[0].data.lock();
        assert!(data.iter().all(|byte| *byte == 0x37));
    }

    #[test]
    fn dm_control_rejects_invalid_or_missing_tables() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ctl1", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ctl1", parent).expect("register parent");

        assert_eq!(dm_dev_create("", "").err(), Some(EINVAL));
        assert_eq!(dm_dev_create("bad/name", "").err(), Some(EINVAL));
        assert!(dm_dev_create("cl-swap-ctl37-dupe", "uuid-dupe37").is_ok());
        assert_eq!(
            dm_dev_create("cl-swap-ctl37-dupe", "uuid-dupe37b").err(),
            Some(EBUSY)
        );
        assert_eq!(
            dm_dev_create("cl-swap-ctl37-dupe-b", "uuid-dupe37").err(),
            Some(EBUSY)
        );

        assert_eq!(dm_resume("cl-swap-ctl37-dupe").err(), Some(EINVAL));
        assert_eq!(
            dm_table_load_linear(
                "cl-swap-ctl37-dupe",
                DmLinearTableSpec::new("missing-parent37", 0, 8, 0),
            )
            .err(),
            Some(ENOENT)
        );
        assert_eq!(
            dm_table_load_linear(
                "cl-swap-ctl37-dupe",
                DmLinearTableSpec::new("dmparent37ctl1", 1, 8, 0),
            )
            .err(),
            Some(EINVAL)
        );
    }

    #[test]
    fn dm_control_status_and_suspend_track_events() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ctl2", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ctl2", parent).expect("register parent");
        dm_dev_create("cl-swap-ctl37-status", "").expect("dm create");
        dm_table_load_linear(
            "cl-swap-ctl37-status",
            DmLinearTableSpec::new("dmparent37ctl2", 0, 4, 2),
        )
        .expect("dm table load");
        let resumed = dm_resume("cl-swap-ctl37-status").expect("dm resume");
        assert_eq!(resumed.event_nr, 1);

        let suspended = dm_suspend("cl-swap-ctl37-status").expect("dm suspend");
        assert!(suspended.suspended);
        assert_eq!(suspended.event_nr, 2);

        let status = dm_dev_status("cl-swap-ctl37-status").expect("dm status");
        assert_eq!(status.name, "cl-swap-ctl37-status");
        assert_eq!(status.table.as_ref().unwrap().length, 4);
        assert!(
            dm_list_devices()
                .iter()
                .any(|dev| dev.name == "cl-swap-ctl37-status")
        );
    }

    #[test]
    fn dm_control_emits_linux_shaped_uevents() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();

        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37uevent0", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37uevent0", parent).expect("register parent");

        dm_dev_create("cl-swap-uevent37", "uuid-uevent37").expect("dm create");
        dm_table_load_linear(
            "cl-swap-uevent37",
            DmLinearTableSpec::new("dmparent37uevent0", 0, 4, 0),
        )
        .expect("dm table load");

        let mut resume = ioctl_buffer("cl-swap-uevent37", "", 0);
        let mut ioctl: DmIoctl = read_struct(&resume, 0).expect("ioctl header");
        ioctl.event_nr = 99;
        write_struct(&mut resume, 0, &ioctl).expect("rewrite ioctl header");
        dm_control_ioctl_buffer(DM_DEV_SUSPEND_IOCTL, &mut resume).expect("resume ioctl");

        let events = crate::net::uevent::drain_pending();
        let add = events
            .iter()
            .find(|msg| {
                msg.payload.starts_with(b"add@/devices/virtual/block/dm-")
                    && has_record(&msg.payload, b"DM_NAME=cl-swap-uevent37")
            })
            .expect("dm add uevent");
        assert!(has_record(&add.payload, b"SUBSYSTEM=block"));
        assert!(has_record(&add.payload, b"DEVTYPE=disk"));
        assert!(has_record(&add.payload, b"DM_UUID=uuid-uevent37"));
        assert!(has_record(&add.payload, b"DM_SUSPENDED=1"));

        let change = events
            .iter()
            .find(|msg| {
                msg.payload
                    .starts_with(b"change@/devices/virtual/block/dm-")
                    && has_record(&msg.payload, b"DM_NAME=cl-swap-uevent37")
            })
            .expect("dm change uevent");
        assert!(has_record(&change.payload, b"DM_COOKIE=99"));
        assert!(has_record(&change.payload, b"DM_SUSPENDED=0"));
        assert!(has_record(&change.payload, b"MAJOR=253"));
        assert!(
            change
                .payload
                .windows(b"DM_SEQNUM=".len())
                .any(|w| w == b"DM_SEQNUM=")
        );
    }

    #[test]
    fn dm_multipath_path_events_emit_linux_env_and_probe_counts() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();

        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37mpath0", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37mpath0", parent).expect("register parent");

        dm_dev_create("cl-swap-mpath37", "uuid-mpath37").expect("dm create");
        dm_table_load_linear(
            "cl-swap-mpath37",
            DmLinearTableSpec::new("dmparent37mpath0", 0, 4, 0),
        )
        .expect("dm table load");
        let resumed = dm_resume("cl-swap-mpath37").expect("dm resume");
        assert_eq!(resumed.valid_paths, 1);
        assert_eq!(dm_multipath_probe_active_paths("cl-swap-mpath37"), Ok(1));
        let mapped = lookup_block_device("/dev/mapper/cl-swap-mpath37").expect("dm mapped bdev");
        assert_eq!(
            crate::block::block_device::block_device_ioctl(&mapped, DM_MPATH_PROBE_PATHS_IOCTL, 0),
            Ok(0)
        );
        let dentry = crate::fs::dcache::d_alloc("cl-swap-mpath37");
        let file = crate::fs::types::File::new(
            dentry,
            0,
            0,
            &crate::block::block_device::BLOCK_DEVICE_FILE_OPS,
        );
        crate::fs::file::set_path_hint(&file, String::from("/dev/mapper/cl-swap-mpath37"));
        let block_ioctl = crate::block::block_device::BLOCK_DEVICE_FILE_OPS
            .ioctl
            .expect("block device ioctl");
        assert_eq!(block_ioctl(&file, DM_MPATH_PROBE_PATHS_IOCTL, 0), Ok(0));

        let _ = crate::net::uevent::drain_pending();
        let before = dm_path_events().len();

        let failed = dm_multipath_path_event(
            "cl-swap-mpath37",
            "dmparent37mpath0",
            DmPathEventKind::PathFailed,
            321,
        )
        .expect("path failed");
        assert_eq!(failed.valid_paths, 0);
        assert_eq!(
            dm_multipath_probe_active_paths("cl-swap-mpath37").err(),
            Some(ENOTCONN)
        );
        assert_eq!(
            crate::block::block_device::block_device_ioctl(&mapped, DM_MPATH_PROBE_PATHS_IOCTL, 0)
                .err(),
            Some(ENOTCONN)
        );
        assert_eq!(
            block_ioctl(&file, DM_MPATH_PROBE_PATHS_IOCTL, 0).err(),
            Some(ENOTCONN)
        );

        let reinstated = dm_multipath_path_event(
            "cl-swap-mpath37",
            "dmparent37mpath0",
            DmPathEventKind::PathReinstated,
            322,
        )
        .expect("path reinstated");
        assert_eq!(reinstated.valid_paths, 1);
        assert_eq!(dm_multipath_probe_active_paths("cl-swap-mpath37"), Ok(1));
        assert_eq!(
            crate::block::block_device::block_device_ioctl(&mapped, DM_MPATH_PROBE_PATHS_IOCTL, 0),
            Ok(0)
        );
        assert_eq!(
            crate::block::block_device::block_device_ioctl(&mapped, 0xdead_beef, 0).err(),
            Some(ENOTTY)
        );

        let recorded = dm_path_events();
        let new_events = &recorded[before..];
        assert!(new_events.iter().any(|event| {
            event.name == "cl-swap-mpath37"
                && event.path == "dmparent37mpath0"
                && event.kind == DmPathEventKind::PathFailed
                && event.valid_paths == 0
        }));
        assert!(new_events.iter().any(|event| {
            event.name == "cl-swap-mpath37"
                && event.path == "dmparent37mpath0"
                && event.kind == DmPathEventKind::PathReinstated
                && event.valid_paths == 1
        }));

        let events = crate::net::uevent::drain_pending();
        let failed_event = events
            .iter()
            .find(|msg| {
                msg.payload
                    .starts_with(b"change@/devices/virtual/block/dm-")
                    && has_record(&msg.payload, b"DM_NAME=cl-swap-mpath37")
                    && has_record(&msg.payload, b"DM_ACTION=PATH_FAILED")
            })
            .expect("path failed uevent");
        assert!(has_record(&failed_event.payload, b"DM_TARGET=multipath"));
        assert!(has_record(
            &failed_event.payload,
            b"DM_PATH=dmparent37mpath0"
        ));
        assert!(has_record(&failed_event.payload, b"DM_NR_VALID_PATHS=0"));
        assert!(has_record(&failed_event.payload, b"DM_COOKIE=321"));

        let reinstated_event = events
            .iter()
            .find(|msg| {
                msg.payload
                    .starts_with(b"change@/devices/virtual/block/dm-")
                    && has_record(&msg.payload, b"DM_NAME=cl-swap-mpath37")
                    && has_record(&msg.payload, b"DM_ACTION=PATH_REINSTATED")
            })
            .expect("path reinstated uevent");
        assert!(has_record(
            &reinstated_event.payload,
            b"DM_NR_VALID_PATHS=1"
        ));
        assert!(has_record(&reinstated_event.payload, b"DM_COOKIE=322"));
    }

    #[test]
    fn dm_multipath_table_selects_active_paths_and_target_messages() {
        let parent_a = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37mpatha", 16 * 512),
            mem_block_device_ops(),
        );
        let parent_b = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37mpathb", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37mpatha", parent_a.clone()).expect("register parent a");
        register_block_device("dmparent37mpathb", parent_b.clone()).expect("register parent b");

        dm_dev_create("cl-swap-mpath-table37", "uuid-mpath-table37").expect("dm create");
        dm_table_load_multipath(
            "cl-swap-mpath-table37",
            DmMultipathTableSpec::new(
                0,
                8,
                "round-robin",
                alloc::vec!["dmparent37mpatha", "dmparent37mpathb"],
            ),
        )
        .expect("dm multipath table load");
        let resumed = dm_resume("cl-swap-mpath-table37").expect("dm resume");
        assert_eq!(resumed.valid_paths, 2);
        assert_eq!(resumed.table.as_ref().unwrap().target_type, "multipath");

        let dm = lookup_block_device("/dev/mapper/cl-swap-mpath-table37").expect("dm alias");
        write_sector(&dm, 0, 0x11);
        write_sector(&dm, 0, 0x22);
        assert_sector_byte(&parent_a, 0, 0x11);
        assert_sector_byte(&parent_b, 0, 0x22);

        let mut fail_msg =
            target_msg_ioctl_buffer("cl-swap-mpath-table37", 0, "fail_path dmparent37mpatha");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut fail_msg).expect("fail path message");
        assert_eq!(
            dm_dev_status("cl-swap-mpath-table37")
                .expect("status")
                .valid_paths,
            1
        );

        write_sector(&dm, 1, 0x33);
        assert_sector_byte(&parent_b, 1, 0x33);
        assert_sector_byte(&parent_a, 1, 0x00);

        let mut reinstate_msg = target_msg_ioctl_buffer(
            "cl-swap-mpath-table37",
            0,
            "reinstate_path /dev/dmparent37mpatha",
        );
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut reinstate_msg)
            .expect("reinstate path message");
        assert_eq!(
            dm_dev_status("cl-swap-mpath-table37")
                .expect("status")
                .valid_paths,
            2
        );

        write_sector(&dm, 2, 0x44);
        assert_sector_byte(&parent_a, 2, 0x44);
    }

    #[test]
    fn dm_multipath_priority_groups_preserve_handler_and_fail_over() {
        let parent_a = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37prioa", 16 * 512),
            mem_block_device_ops(),
        );
        let parent_b = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37priob", 16 * 512),
            mem_block_device_ops(),
        );
        let parent_c = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37prioc", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37prioa", parent_a.clone()).expect("register parent a");
        register_block_device("dmparent37priob", parent_b.clone()).expect("register parent b");
        register_block_device("dmparent37prioc", parent_c.clone()).expect("register parent c");

        let mut create = ioctl_buffer("cl-swap-prio37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_CREATE_IOCTL, &mut create).expect("create ioctl");
        let params = "1 queue_if_no_path 2 1 alua 2 1 service-time 0 1 0 dmparent37prioa round-robin 0 2 0 dmparent37priob dmparent37prioc";
        let mut load = multipath_table_load_ioctl_buffer_with_params("cl-swap-prio37", params, 8);
        dm_control_ioctl_buffer(DM_TABLE_LOAD_IOCTL, &mut load).expect("priority load ioctl");

        let mut resume = ioctl_buffer("cl-swap-prio37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_SUSPEND_IOCTL, &mut resume).expect("resume ioctl");
        let dm = lookup_block_device("/dev/mapper/cl-swap-prio37").expect("dm alias");

        write_sector(&dm, 0, 0xa1);
        assert_sector_byte(&parent_a, 0, 0xa1);

        let mut fail_msg =
            target_msg_ioctl_buffer("cl-swap-prio37", 0, "fail_path dmparent37prioa");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut fail_msg).expect("fail priority group");
        write_sector(&dm, 1, 0xb1);
        write_sector(&dm, 2, 0xc1);
        assert_sector_byte(&parent_b, 1, 0xb1);
        assert_sector_byte(&parent_c, 2, 0xc1);

        let mut reinstate_msg =
            target_msg_ioctl_buffer("cl-swap-prio37", 0, "reinstate_path dmparent37prioa");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut reinstate_msg)
            .expect("reinstate priority group");
        write_sector(&dm, 3, 0xa2);
        assert_sector_byte(&parent_a, 3, 0xa2);

        let mut table_status = ioctl_buffer_with_size("cl-swap-prio37", "", 768);
        dm_control_ioctl_buffer(DM_TABLE_STATUS_IOCTL, &mut table_status)
            .expect("priority status ioctl");
        let status_header: DmIoctl = read_struct(&table_status, 0).expect("priority status header");
        let status_params = cstr_from_range(
            &table_status[status_header.data_start as usize + core::mem::size_of::<DmTargetSpec>()
                ..status_header.data_size as usize],
        )
        .expect("priority params");
        assert!(status_params.contains("queue_if_no_path"));
        assert!(status_params.contains("1 alua"));
        assert!(status_params.contains("service-time"));
        assert!(status_params.contains("round-robin"));

        let mut fail_if_no_path = target_msg_ioctl_buffer("cl-swap-prio37", 0, "fail_if_no_path");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut fail_if_no_path)
            .expect("disable queue_if_no_path");
        let mut no_queue_status = ioctl_buffer_with_size("cl-swap-prio37", "", 768);
        dm_control_ioctl_buffer(DM_TABLE_STATUS_IOCTL, &mut no_queue_status)
            .expect("no queue table status");
        let no_queue_header: DmIoctl =
            read_struct(&no_queue_status, 0).expect("no queue status header");
        let no_queue_params = cstr_from_range(
            &no_queue_status[no_queue_header.data_start as usize
                + core::mem::size_of::<DmTargetSpec>()
                ..no_queue_header.data_size as usize],
        )
        .expect("no queue params");
        assert!(!no_queue_params.contains("queue_if_no_path"));

        let mut queue_if_no_path = target_msg_ioctl_buffer("cl-swap-prio37", 0, "queue_if_no_path");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut queue_if_no_path)
            .expect("enable queue_if_no_path");

        let mut disable_group = target_msg_ioctl_buffer("cl-swap-prio37", 0, "disable_group 1");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut disable_group)
            .expect("disable priority group 1");
        write_sector(&dm, 4, 0xd1);
        assert_sector_byte(&parent_b, 4, 0xd1);
        assert_sector_byte(&parent_a, 4, 0x00);

        let mut enable_group = target_msg_ioctl_buffer("cl-swap-prio37", 0, "enable_group 1");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut enable_group)
            .expect("enable priority group 1");
        let mut switch_group_2 = target_msg_ioctl_buffer("cl-swap-prio37", 0, "switch_group 2");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut switch_group_2)
            .expect("switch to priority group 2");
        write_sector(&dm, 5, 0xe1);
        assert_sector_byte(&parent_b, 5, 0xe1);

        let mut switch_group_1 = target_msg_ioctl_buffer("cl-swap-prio37", 0, "switch_group 1");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut switch_group_1)
            .expect("switch to priority group 1");
        write_sector(&dm, 6, 0xf1);
        assert_sector_byte(&parent_a, 6, 0xf1);

        let mut bad_group = target_msg_ioctl_buffer("cl-swap-prio37", 0, "disable_group 0");
        assert_eq!(
            dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut bad_group).err(),
            Some(EINVAL)
        );
        let mut extra_arg = target_msg_ioctl_buffer("cl-swap-prio37", 0, "queue_if_no_path extra");
        assert_eq!(
            dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut extra_arg).err(),
            Some(EINVAL)
        );

        let mut deps = ioctl_buffer_with_size("cl-swap-prio37", "", 512);
        dm_control_ioctl_buffer(DM_TABLE_DEPS_IOCTL, &mut deps).expect("deps ioctl");
        let deps_header: DmIoctl = read_struct(&deps, 0).expect("deps header");
        let deps_body: DmTargetDeps =
            read_struct(&deps, deps_header.data_start as usize).expect("deps body");
        assert_eq!(deps_body.count, 3);
    }

    #[test]
    fn dm_control_ioctl_loads_multipath_table_and_reports_deps() {
        let parent_a = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37mpathioctla", 16 * 512),
            mem_block_device_ops(),
        );
        let parent_b = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37mpathioctlb", 16 * 512),
            mem_block_device_ops(),
        );
        let parent_a_dev = parent_a.id;
        let parent_b_dev = parent_b.id;
        register_block_device("dmparent37mpathioctla", parent_a).expect("register parent a");
        register_block_device("dmparent37mpathioctlb", parent_b).expect("register parent b");

        let mut create = ioctl_buffer("cl-swap-mpath-ioctl37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_CREATE_IOCTL, &mut create).expect("create ioctl");

        let mut load = multipath_table_load_ioctl_buffer(
            "cl-swap-mpath-ioctl37",
            &["dmparent37mpathioctla", "dmparent37mpathioctlb"],
            8,
        );
        dm_control_ioctl_buffer(DM_TABLE_LOAD_IOCTL, &mut load).expect("multipath load ioctl");

        let mut resume = ioctl_buffer("cl-swap-mpath-ioctl37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_SUSPEND_IOCTL, &mut resume).expect("resume ioctl");

        let mut table_status = ioctl_buffer_with_size("cl-swap-mpath-ioctl37", "", 512);
        dm_control_ioctl_buffer(DM_TABLE_STATUS_IOCTL, &mut table_status)
            .expect("multipath status ioctl");
        let status_header: DmIoctl = read_struct(&table_status, 0).expect("status header");
        let spec: DmTargetSpec =
            read_struct(&table_status, status_header.data_start as usize).expect("target spec");
        assert_eq!(
            fixed_cstr_to_string(&spec.target_type).unwrap(),
            "multipath"
        );
        let params = cstr_from_range(
            &table_status[status_header.data_start as usize + core::mem::size_of::<DmTargetSpec>()
                ..status_header.data_size as usize],
        )
        .expect("params");
        assert!(params.contains("round-robin"));
        assert!(params.contains("dmparent37mpathioctla"));
        assert!(params.contains("dmparent37mpathioctlb"));

        let mut deps = ioctl_buffer_with_size("cl-swap-mpath-ioctl37", "", 512);
        dm_control_ioctl_buffer(DM_TABLE_DEPS_IOCTL, &mut deps).expect("deps ioctl");
        let deps_header: DmIoctl = read_struct(&deps, 0).expect("deps header");
        let deps_body: DmTargetDeps =
            read_struct(&deps, deps_header.data_start as usize).expect("deps body");
        assert_eq!(deps_body.count, 2);
        let dep0: u64 = read_struct(
            &deps,
            deps_header.data_start as usize + core::mem::size_of::<DmTargetDeps>(),
        )
        .expect("dep0");
        let dep1: u64 = read_struct(
            &deps,
            deps_header.data_start as usize + core::mem::size_of::<DmTargetDeps>() + 8,
        )
        .expect("dep1");
        assert_eq!(
            alloc::vec![dep0, dep1],
            alloc::vec![parent_a_dev, parent_b_dev]
        );
    }

    #[test]
    fn lvm_text_metadata_activates_linear_swap_lv() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37a", 128 * 512),
            mem_block_device_ops(),
        );
        register_block_device("lvmparent37a", parent.clone()).expect("register pv");

        let metadata = r#"
cl {
    extent_size = 8
    physical_volumes {
        pv0 {
            device = "/dev/lvmparent37a"
            pe_start = 2
        }
    }
    logical_volumes {
        swap {
            segment_count = 1
            segment1 {
                start_extent = 0
                extent_count = 4
                type = "striped"
                stripe_count = 1
                stripes = [
                    "pv0", 3
                ]
            }
        }
    }
}
"#;

        let lvs = lvm_parse_text_metadata(metadata).expect("parse lvm metadata");
        assert_eq!(lvs.len(), 1);
        assert_eq!(lvs[0].dm_name, "cl-swap");
        assert_eq!(lvs[0].parent_name, "lvmparent37a");
        assert_eq!(lvs[0].target_start, 26);
        assert_eq!(lvs[0].length, 32);

        let statuses = lvm_activate_text_metadata(metadata).expect("activate lvm");
        assert_eq!(statuses.len(), 1);
        assert!(statuses[0].active);
        assert_eq!(
            statuses[0].table.as_ref().unwrap().params,
            "lvmparent37a 26"
        );

        let dm = lookup_block_device("/dev/mapper/cl-swap").expect("activated mapper LV");
        write_sector(&dm, 0, 0x5c);
        assert_sector_byte(&parent, 26, 0x5c);
    }

    #[test]
    fn lvm_text_metadata_escapes_hyphenated_mapper_names() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37b", 64 * 512),
            mem_block_device_ops(),
        );
        register_block_device("lvmparent37b", parent).expect("register pv");

        let metadata = r#"
vg-main {
    extent_size = 4
    physical_volumes {
        pv-fast {
            device = "lvmparent37b"
            pe_start = 1
        }
    }
    logical_volumes {
        swap-fast {
            segment_count = 1
            segment1 {
                start_extent = 0
                extent_count = 2
                stripes = [ "pv-fast", 0 ]
            }
        }
    }
}
"#;

        let lvs = lvm_parse_text_metadata(metadata).expect("parse lvm metadata");
        assert_eq!(lvs[0].dm_name, "vg--main-swap--fast");
        assert_eq!(lvs[0].target_start, 1);
        assert_eq!(lvs[0].length, 8);
    }

    #[test]
    fn lvm_scan_block_device_reads_label_metadata_and_activates_lv() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37scan", 128 * 512),
            mem_block_device_ops(),
        );
        register_block_device("lvmparent37scan", parent.clone()).expect("register scan pv");

        let metadata = r#"
clscan {
    extent_size = 8
    physical_volumes {
        pv0 {
            device = "/dev/lvmparent37scan"
            pe_start = 16
        }
    }
    logical_volumes {
        swap {
            segment_count = 1
            segment1 {
                start_extent = 0
                extent_count = 2
                stripes = [ "pv0", 1 ]
            }
        }
    }
}
"#;
        write_lvm2_scan_fixture(&parent, metadata);

        let texts =
            lvm_scan_text_metadata_from_block_device(&parent).expect("scan lvm text metadata");
        assert_eq!(texts.len(), 1);
        assert!(texts[0].contains("clscan"));

        let statuses = lvm_activate_block_device("lvmparent37scan").expect("activate scanned pv");
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].name, "clscan-swap");
        assert!(statuses[0].active);

        let dm = lookup_block_device("/dev/mapper/clscan-swap").expect("activated scanned LV");
        write_sector(&dm, 0, 0xa7);
        assert_sector_byte(&parent, 24, 0xa7);
    }

    #[test]
    fn lvm_registered_scan_deduplicates_metadata_and_activates_multi_pv_vg() {
        let fast = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37scanfast", 128 * 512),
            mem_block_device_ops(),
        );
        let slow = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37scanslow", 128 * 512),
            mem_block_device_ops(),
        );
        register_block_device("lvmparent37scanfast", fast.clone()).expect("register fast pv");
        register_block_device("lvmparent37scanslow", slow.clone()).expect("register slow pv");

        let metadata = r#"
vgscan37 {
    extent_size = 8
    physical_volumes {
        pv_fast {
            device = "/dev/lvmparent37scanfast"
            pe_start = 16
        }
        pv_slow {
            device = "/dev/lvmparent37scanslow"
            pe_start = 32
        }
    }
    logical_volumes {
        swap {
            segment_count = 1
            segment1 {
                start_extent = 0
                extent_count = 2
                stripes = [ "pv_fast", 1 ]
            }
        }
        var {
            segment_count = 1
            segment1 {
                start_extent = 0
                extent_count = 3
                stripes = [ "pv_slow", 2 ]
            }
        }
    }
}
"#;
        write_lvm2_scan_fixture(&fast, metadata);
        write_lvm2_scan_fixture(&slow, metadata);

        let records = lvm_scan_registered_text_metadata().expect("scan registered lvm pvs");
        assert_eq!(
            records
                .iter()
                .filter(|text| text.contains("vgscan37"))
                .count(),
            1
        );

        let statuses =
            lvm_activate_registered_block_devices().expect("activate registered lvm pvs");
        assert_eq!(status_count(&statuses, "vgscan37-swap"), 1);
        assert_eq!(status_count(&statuses, "vgscan37-var"), 1);

        let swap = lookup_block_device("/dev/mapper/vgscan37-swap").expect("activated swap LV");
        let var = lookup_block_device("/dev/mapper/vgscan37-var").expect("activated var LV");
        write_sector(&swap, 0, 0x51);
        write_sector(&var, 0, 0x52);
        assert_sector_byte(&fast, 24, 0x51);
        assert_sector_byte(&slow, 48, 0x52);
    }

    #[test]
    fn lvm_text_metadata_activates_multi_segment_lv_as_multi_target_table() {
        let fast = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37segfast", 128 * 512),
            mem_block_device_ops(),
        );
        let slow = BlockDevice::wrap(
            MemBlockDevice::new("lvmparent37segslow", 128 * 512),
            mem_block_device_ops(),
        );
        register_block_device("lvmparent37segfast", fast.clone()).expect("register fast pv");
        register_block_device("lvmparent37segslow", slow.clone()).expect("register slow pv");

        let metadata = r#"
vgseg37 {
    extent_size = 8
    physical_volumes {
        pv_fast {
            device = "/dev/lvmparent37segfast"
            pe_start = 8
        }
        pv_slow {
            device = "/dev/lvmparent37segslow"
            pe_start = 32
        }
    }
    logical_volumes {
        swap {
            segment_count = 2
            segment1 {
                start_extent = 0
                extent_count = 2
                stripes = [ "pv_fast", 1 ]
            }
            segment2 {
                start_extent = 2
                extent_count = 3
                stripes = [ "pv_slow", 2 ]
            }
        }
    }
}
"#;

        let lvs = lvm_parse_text_metadata(metadata).expect("parse multi-segment lvm");
        assert_eq!(lvs.len(), 1);
        assert_eq!(lvs[0].segments.len(), 2);
        assert_eq!(lvs[0].length, 40);
        assert_eq!(lvs[0].segments[1].sector_start, 16);

        let statuses = lvm_activate_text_metadata(metadata).expect("activate multi-segment lvm");
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].target_count, 2);

        let dm = lookup_block_device("/dev/mapper/vgseg37-swap").expect("activated segmented LV");
        write_sector(&dm, 1, 0x81);
        write_sector(&dm, 17, 0x82);
        assert_sector_byte(&fast, 17, 0x81);
        assert_sector_byte(&slow, 49, 0x82);
    }

    #[test]
    fn dm_control_ioctl_create_load_resume_and_report_table() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ioctl0", 32 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ioctl0", parent).expect("register parent");

        let mut create = ioctl_buffer("cl-swap-ioctl37", "uuid-ioctl37", 0);
        dm_control_ioctl_buffer(DM_DEV_CREATE_IOCTL, &mut create).expect("dev create ioctl");
        let created: DmIoctl = read_struct(&create, 0).expect("created header");
        assert_eq!(created.version, dm_version());
        assert_eq!(created.flags & DM_SUSPEND_FLAG, DM_SUSPEND_FLAG);
        assert_eq!(
            fixed_cstr_to_string(&created.name).unwrap(),
            "cl-swap-ioctl37"
        );

        let mut load = table_load_ioctl_buffer("cl-swap-ioctl37", "dmparent37ioctl0", 8, 4);
        dm_control_ioctl_buffer(DM_TABLE_LOAD_IOCTL, &mut load).expect("table load ioctl");
        let loaded: DmIoctl = read_struct(&load, 0).expect("loaded header");
        assert_eq!(loaded.target_count, 0);

        let mut resume = ioctl_buffer("cl-swap-ioctl37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_SUSPEND_IOCTL, &mut resume).expect("resume ioctl");
        let resumed: DmIoctl = read_struct(&resume, 0).expect("resumed header");
        assert_eq!(resumed.flags & DM_SUSPEND_FLAG, 0);
        assert_eq!(resumed.target_count, 1);
        assert!(lookup_block_device("/dev/mapper/cl-swap-ioctl37").is_some());

        let mut table_status = ioctl_buffer_with_size("cl-swap-ioctl37", "", 512);
        dm_control_ioctl_buffer(DM_TABLE_STATUS_IOCTL, &mut table_status)
            .expect("table status ioctl");
        let status_header: DmIoctl = read_struct(&table_status, 0).expect("status header");
        assert_eq!(status_header.target_count, 1);
        let spec: DmTargetSpec =
            read_struct(&table_status, status_header.data_start as usize).expect("target spec");
        assert_eq!(spec.sector_start, 0);
        assert_eq!(spec.length, 8);
        assert_eq!(fixed_cstr_to_string(&spec.target_type).unwrap(), "linear");
        let params = cstr_from_range(
            &table_status[status_header.data_start as usize + core::mem::size_of::<DmTargetSpec>()
                ..status_header.data_size as usize],
        )
        .expect("params");
        assert_eq!(params, "dmparent37ioctl0 4");

        let mut list = ioctl_buffer_with_size("", "", 512);
        dm_control_ioctl_buffer(DM_LIST_DEVICES_IOCTL, &mut list).expect("list devices ioctl");
        let list_header: DmIoctl = read_struct(&list, 0).expect("list header");
        assert!(list_header.target_count >= 1);
        let name_start = list_header.data_start as usize + 12;
        let first_name = cstr_from_range(&list[name_start..]).expect("first listed name");
        assert!(!first_name.is_empty());
    }

    #[test]
    fn dm_control_ioctl_loads_multi_target_linear_table_status_and_deps() {
        let parent_a = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ioctlmultia", 64 * 512),
            mem_block_device_ops(),
        );
        let parent_b = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ioctlmultib", 64 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ioctlmultia", parent_a.clone())
            .expect("register parent a");
        register_block_device("dmparent37ioctlmultib", parent_b.clone())
            .expect("register parent b");

        let mut create = ioctl_buffer("cl-linear-multi-ioctl37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_CREATE_IOCTL, &mut create).expect("dev create ioctl");

        let mut load = linear_targets_table_load_ioctl_buffer(
            "cl-linear-multi-ioctl37",
            &[
                ("dmparent37ioctlmultia", 0, 4, 8),
                ("dmparent37ioctlmultib", 4, 6, 16),
            ],
        );
        dm_control_ioctl_buffer(DM_TABLE_LOAD_IOCTL, &mut load).expect("multi-target load ioctl");

        let mut resume = ioctl_buffer("cl-linear-multi-ioctl37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_SUSPEND_IOCTL, &mut resume).expect("resume ioctl");
        let resumed: DmIoctl = read_struct(&resume, 0).expect("resumed header");
        assert_eq!(resumed.target_count, 2);

        let dm = lookup_block_device("/dev/mapper/cl-linear-multi-ioctl37").expect("dm alias");
        write_sector(&dm, 2, 0x71);
        write_sector(&dm, 6, 0x72);
        assert_sector_byte(&parent_a, 10, 0x71);
        assert_sector_byte(&parent_b, 18, 0x72);

        let mut table_status = ioctl_buffer_with_size("cl-linear-multi-ioctl37", "", 768);
        dm_control_ioctl_buffer(DM_TABLE_STATUS_IOCTL, &mut table_status)
            .expect("table status ioctl");
        let status_header: DmIoctl =
            read_struct(&table_status, 0).expect("multi table status header");
        assert_eq!(status_header.target_count, 2);

        let first_off = status_header.data_start as usize;
        let first: DmTargetSpec = read_struct(&table_status, first_off).expect("first target");
        assert_eq!(first.sector_start, 0);
        assert_eq!(first.length, 4);
        assert!(first.next > 0);
        assert_eq!(fixed_cstr_to_string(&first.target_type).unwrap(), "linear");
        let first_params = cstr_from_range(
            &table_status
                [first_off + core::mem::size_of::<DmTargetSpec>()..first_off + first.next as usize],
        )
        .expect("first params");
        assert_eq!(first_params, "dmparent37ioctlmultia 8");

        let second_off = first_off + first.next as usize;
        let second: DmTargetSpec = read_struct(&table_status, second_off).expect("second target");
        assert_eq!(second.sector_start, 4);
        assert_eq!(second.length, 6);
        assert_eq!(second.next, 0);
        let second_params = cstr_from_range(
            &table_status[second_off + core::mem::size_of::<DmTargetSpec>()
                ..status_header.data_size as usize],
        )
        .expect("second params");
        assert_eq!(second_params, "dmparent37ioctlmultib 16");

        let mut deps = ioctl_buffer_with_size("cl-linear-multi-ioctl37", "", 512);
        dm_control_ioctl_buffer(DM_TABLE_DEPS_IOCTL, &mut deps).expect("deps ioctl");
        let deps_header: DmIoctl = read_struct(&deps, 0).expect("deps header");
        let deps_body: DmTargetDeps =
            read_struct(&deps, deps_header.data_start as usize).expect("deps body");
        assert_eq!(deps_body.count, 2);
    }

    #[test]
    fn dm_control_ioctl_rejects_bad_magic_or_table_type() {
        let mut create = ioctl_buffer("bad-magic37", "", 0);
        assert_eq!(
            dm_control_ioctl_buffer(DM_DEV_CREATE_IOCTL + 0x100, &mut create).err(),
            Some(ENOTTY)
        );

        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ioctl1", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ioctl1", parent).expect("register parent");
        dm_dev_create("cl-swap-ioctl37-bad-table", "").expect("create device");
        let mut load =
            table_load_ioctl_buffer("cl-swap-ioctl37-bad-table", "dmparent37ioctl1", 4, 0);
        let off = core::mem::size_of::<DmIoctl>();
        let mut spec: DmTargetSpec = read_struct(&load, off).expect("target spec");
        write_fixed_cstr(&mut spec.target_type, "striped");
        write_struct(&mut load, off, &spec).expect("rewrite target spec");
        assert_eq!(
            dm_control_ioctl_buffer(DM_TABLE_LOAD_IOCTL, &mut load).err(),
            Some(EINVAL)
        );
    }

    #[test]
    fn dm_control_ioctl_table_clear_rename_and_remove() {
        let _guard = crate::net::uevent::test_lock();
        let _ = crate::net::uevent::drain_pending();

        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37ctlremove", 16 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37ctlremove", parent).expect("register parent");

        dm_dev_create("cl-swap-remove37", "").expect("dm create");
        dm_table_load_linear(
            "cl-swap-remove37",
            DmLinearTableSpec::new("dmparent37ctlremove", 0, 4, 0),
        )
        .expect("dm table load");
        let cleared = dm_table_clear("cl-swap-remove37").expect("table clear");
        assert!(!cleared.inactive);
        assert!(!cleared.active);

        dm_table_load_linear(
            "cl-swap-remove37",
            DmLinearTableSpec::new("dmparent37ctlremove", 0, 4, 0),
        )
        .expect("dm table reload");
        let resumed = dm_resume("cl-swap-remove37").expect("dm resume");
        let dm_name = format!("dm-{}", resumed.minor);
        assert!(lookup_block_device("/dev/mapper/cl-swap-remove37").is_some());

        let mut rename = ioctl_buffer_with_payload("cl-swap-remove37", "", b"cl-swap-renamed37\0");
        dm_control_ioctl_buffer(DM_DEV_RENAME_IOCTL, &mut rename).expect("rename ioctl");
        assert!(lookup_block_device("/dev/mapper/cl-swap-remove37").is_none());
        assert!(lookup_block_device("/dev/mapper/cl-swap-renamed37").is_some());
        assert!(dm_dev_status("cl-swap-renamed37").is_ok());

        let mut rename_uuid =
            ioctl_buffer_with_payload("cl-swap-renamed37", "", b"uuid-renamed37\0");
        let mut ioctl: DmIoctl = read_struct(&rename_uuid, 0).expect("ioctl header");
        ioctl.flags = DM_UUID_FLAG;
        write_struct(&mut rename_uuid, 0, &ioctl).expect("rewrite header");
        dm_control_ioctl_buffer(DM_DEV_RENAME_IOCTL, &mut rename_uuid).expect("rename uuid ioctl");
        assert_eq!(
            dm_dev_status("cl-swap-renamed37").expect("status").uuid,
            "uuid-renamed37"
        );

        let mut remove = ioctl_buffer("cl-swap-renamed37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_REMOVE_IOCTL, &mut remove).expect("remove ioctl");
        assert!(dm_dev_status("cl-swap-renamed37").is_err());
        assert!(lookup_block_device(&dm_name).is_none());
        assert!(lookup_block_device("/dev/mapper/cl-swap-renamed37").is_none());

        let events = crate::net::uevent::drain_pending();
        assert!(events.iter().any(|msg| {
            msg.payload
                .starts_with(b"change@/devices/virtual/block/dm-")
                && has_record(&msg.payload, b"DM_NAME=cl-swap-renamed37")
        }));
        assert!(events.iter().any(|msg| {
            msg.payload
                .starts_with(b"remove@/devices/virtual/block/dm-")
                && has_record(&msg.payload, b"DM_NAME=cl-swap-renamed37")
        }));
    }

    #[test]
    fn dm_control_ioctl_deps_versions_geometry_and_arm_poll() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37deps0", 32 * 512),
            mem_block_device_ops(),
        );
        let parent_dev = parent.id;
        register_block_device("dmparent37deps0", parent).expect("register parent");

        dm_dev_create("cl-swap-deps37", "").expect("dm create");
        dm_table_load_linear(
            "cl-swap-deps37",
            DmLinearTableSpec::new("dmparent37deps0", 0, 8, 4),
        )
        .expect("dm table load");
        dm_resume("cl-swap-deps37").expect("dm resume");

        let mut deps = ioctl_buffer_with_size("cl-swap-deps37", "", 512);
        dm_control_ioctl_buffer(DM_TABLE_DEPS_IOCTL, &mut deps).expect("table deps ioctl");
        let deps_header: DmIoctl = read_struct(&deps, 0).expect("deps header");
        assert_eq!(deps_header.target_count, 1);
        let deps_body: DmTargetDeps =
            read_struct(&deps, deps_header.data_start as usize).expect("deps body");
        assert_eq!(deps_body.count, 1);
        let parent_dep: u64 = read_struct(
            &deps,
            deps_header.data_start as usize + core::mem::size_of::<DmTargetDeps>(),
        )
        .expect("parent dep");
        assert_eq!(parent_dep, parent_dev);

        let mut versions = ioctl_buffer_with_size("", "", 512);
        dm_control_ioctl_buffer(DM_LIST_VERSIONS_IOCTL, &mut versions)
            .expect("list versions ioctl");
        let versions_header: DmIoctl = read_struct(&versions, 0).expect("versions header");
        assert_eq!(versions_header.target_count, 2);
        let linear_version: DmTargetVersions =
            read_struct(&versions, versions_header.data_start as usize).expect("version body");
        assert_eq!(linear_version.version, DM_LINEAR_TARGET_VERSION);
        let linear_name = cstr_from_range(
            &versions[versions_header.data_start as usize + core::mem::size_of::<DmTargetVersions>()
                ..versions_header.data_start as usize + linear_version.next as usize],
        )
        .expect("target name");
        assert_eq!(linear_name, "linear");
        assert!(linear_version.next > 0);
        let multipath_off = versions_header.data_start as usize + linear_version.next as usize;
        let multipath_version: DmTargetVersions =
            read_struct(&versions, multipath_off).expect("multipath version body");
        assert_eq!(multipath_version.version, DM_MULTIPATH_TARGET_VERSION);
        assert_eq!(multipath_version.next, 0);
        let multipath_name = cstr_from_range(
            &versions[multipath_off + core::mem::size_of::<DmTargetVersions>()
                ..versions_header.data_size as usize],
        )
        .expect("multipath target name");
        assert_eq!(multipath_name, "multipath");

        let mut get_version = ioctl_buffer_with_size("linear", "", 512);
        dm_control_ioctl_buffer(DM_GET_TARGET_VERSION_IOCTL, &mut get_version)
            .expect("get target version ioctl");
        let get_header: DmIoctl = read_struct(&get_version, 0).expect("get version header");
        let get_linear: DmTargetVersions =
            read_struct(&get_version, get_header.data_start as usize).expect("get version body");
        assert_eq!(get_linear.version, DM_LINEAR_TARGET_VERSION);

        let mut get_multipath = ioctl_buffer_with_size("multipath", "", 512);
        dm_control_ioctl_buffer(DM_GET_TARGET_VERSION_IOCTL, &mut get_multipath)
            .expect("get multipath target version ioctl");
        let get_mpath_header: DmIoctl =
            read_struct(&get_multipath, 0).expect("get multipath version header");
        let get_mpath: DmTargetVersions =
            read_struct(&get_multipath, get_mpath_header.data_start as usize)
                .expect("get multipath version body");
        assert_eq!(get_mpath.version, DM_MULTIPATH_TARGET_VERSION);

        let mut unknown = ioctl_buffer_with_size("striped", "", 512);
        assert_eq!(
            dm_control_ioctl_buffer(DM_GET_TARGET_VERSION_IOCTL, &mut unknown).err(),
            Some(EINVAL)
        );

        let mut geometry = ioctl_buffer_with_payload("cl-swap-deps37", "", b"1024 16 63 2048\0");
        dm_control_ioctl_buffer(DM_DEV_SET_GEOMETRY_IOCTL, &mut geometry).expect("geometry ioctl");
        assert_eq!(
            dm_dev_status("cl-swap-deps37").expect("status").geometry,
            Some(DmGeometry {
                cylinders: 1024,
                heads: 16,
                sectors: 63,
                start: 2048,
            })
        );

        let mut arm_poll = ioctl_buffer("cl-swap-deps37", "", 0);
        dm_control_ioctl_buffer(DM_DEV_ARM_POLL_IOCTL, &mut arm_poll).expect("arm poll ioctl");
    }

    #[test]
    fn dm_control_ioctl_target_message_parses_linear_and_core_messages() {
        let parent = BlockDevice::wrap(
            MemBlockDevice::new("dmparent37msg0", 32 * 512),
            mem_block_device_ops(),
        );
        register_block_device("dmparent37msg0", parent).expect("register parent");

        dm_dev_create("cl-swap-msg37", "").expect("dm create");
        dm_table_load_linear(
            "cl-swap-msg37",
            DmLinearTableSpec::new("dmparent37msg0", 0, 8, 4),
        )
        .expect("dm table load");
        dm_resume("cl-swap-msg37").expect("dm resume");

        let mut linear_msg = target_msg_ioctl_buffer("cl-swap-msg37", 0, "flush");
        assert_eq!(
            dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut linear_msg).err(),
            Some(EINVAL)
        );

        let mut outside = target_msg_ioctl_buffer("cl-swap-msg37", 9, "flush");
        assert_eq!(
            dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut outside).err(),
            Some(EINVAL)
        );

        let mut core_msg = target_msg_ioctl_buffer("cl-swap-msg37", 0, "@cancel_deferred_remove");
        dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut core_msg).expect("core message ioctl");
        let header: DmIoctl = read_struct(&core_msg, 0).expect("core message header");
        assert_eq!(header.target_count, 1);
        assert_eq!(fixed_cstr_to_string(&header.name).unwrap(), "cl-swap-msg37");

        let mut bad_core = target_msg_ioctl_buffer("cl-swap-msg37", 0, "@unknown");
        assert_eq!(
            dm_control_ioctl_buffer(DM_TARGET_MSG_IOCTL, &mut bad_core).err(),
            Some(EINVAL)
        );
    }

    fn ioctl_buffer(name: &str, uuid: &str, extra_len: usize) -> Vec<u8> {
        let len = core::mem::size_of::<DmIoctl>() + extra_len;
        ioctl_buffer_with_size(name, uuid, len)
    }

    fn ioctl_buffer_with_size(name: &str, uuid: &str, len: usize) -> Vec<u8> {
        let mut buf = alloc::vec![0u8; len];
        let mut ioctl = DmIoctl {
            version: dm_version(),
            data_size: len as u32,
            data_start: core::mem::size_of::<DmIoctl>() as u32,
            target_count: 0,
            open_count: 0,
            flags: 0,
            event_nr: 0,
            padding: 0,
            dev: 0,
            name: [0; DM_NAME_LEN],
            uuid: [0; DM_UUID_LEN],
            data: [0; 7],
        };
        write_fixed_cstr(&mut ioctl.name, name);
        write_fixed_cstr(&mut ioctl.uuid, uuid);
        write_struct(&mut buf, 0, &ioctl).expect("write ioctl header");
        buf
    }

    fn ioctl_buffer_with_payload(name: &str, uuid: &str, payload: &[u8]) -> Vec<u8> {
        let mut buf = ioctl_buffer(name, uuid, payload.len());
        let start = core::mem::size_of::<DmIoctl>();
        buf[start..start + payload.len()].copy_from_slice(payload);
        buf
    }

    fn table_load_ioctl_buffer(name: &str, parent: &str, length: u64, start: u64) -> Vec<u8> {
        let params = format!("{parent} {start}\0");
        let extra_len = align_up(
            core::mem::size_of::<DmTargetSpec>() + params.as_bytes().len(),
            8,
        );
        let mut buf = ioctl_buffer(name, "", extra_len);
        let mut ioctl: DmIoctl = read_struct(&buf, 0).expect("ioctl header");
        ioctl.target_count = 1;
        let mut spec = DmTargetSpec {
            sector_start: 0,
            length,
            status: 0,
            next: extra_len as u32,
            target_type: [0; DM_MAX_TYPE_NAME],
        };
        write_fixed_cstr(&mut spec.target_type, "linear");
        let off = ioctl.data_start as usize;
        write_struct(&mut buf, off, &spec).expect("target spec");
        let params_start = off + core::mem::size_of::<DmTargetSpec>();
        buf[params_start..params_start + params.as_bytes().len()]
            .copy_from_slice(params.as_bytes());
        write_struct(&mut buf, 0, &ioctl).expect("rewrite ioctl header");
        buf
    }

    fn linear_targets_table_load_ioctl_buffer(
        name: &str,
        targets: &[(&str, u64, u64, u64)],
    ) -> Vec<u8> {
        let entry_lens = targets
            .iter()
            .map(|(parent, _, _, start)| {
                let params = format!("{parent} {start}\0");
                align_up(core::mem::size_of::<DmTargetSpec>() + params.len(), 8)
            })
            .collect::<Vec<_>>();
        let extra_len = entry_lens.iter().sum::<usize>();
        let mut buf = ioctl_buffer(name, "", extra_len);
        let mut ioctl: DmIoctl = read_struct(&buf, 0).expect("ioctl header");
        ioctl.target_count = targets.len() as u32;

        let mut off = ioctl.data_start as usize;
        for (idx, (parent, sector_start, length, target_start)) in targets.iter().enumerate() {
            let params = format!("{parent} {target_start}\0");
            let entry_len = entry_lens[idx];
            let mut spec = DmTargetSpec {
                sector_start: *sector_start,
                length: *length,
                status: 0,
                next: if idx + 1 == targets.len() {
                    0
                } else {
                    entry_len as u32
                },
                target_type: [0; DM_MAX_TYPE_NAME],
            };
            write_fixed_cstr(&mut spec.target_type, "linear");
            write_struct(&mut buf, off, &spec).expect("target spec");
            let params_start = off + core::mem::size_of::<DmTargetSpec>();
            let entry_end = off + entry_len;
            buf[params_start..entry_end].fill(0);
            buf[params_start..params_start + params.len()].copy_from_slice(params.as_bytes());
            off = entry_end;
        }

        write_struct(&mut buf, 0, &ioctl).expect("rewrite ioctl header");
        buf
    }

    fn multipath_table_load_ioctl_buffer(name: &str, parents: &[&str], length: u64) -> Vec<u8> {
        let mut params = format!("0 0 1 1 round-robin 0 {} 0", parents.len());
        for parent in parents {
            params.push(' ');
            params.push_str(parent);
        }
        multipath_table_load_ioctl_buffer_with_params(name, &params, length)
    }

    fn multipath_table_load_ioctl_buffer_with_params(
        name: &str,
        params: &str,
        length: u64,
    ) -> Vec<u8> {
        let mut params = String::from(params);
        params.push('\0');
        let extra_len = align_up(
            core::mem::size_of::<DmTargetSpec>() + params.as_bytes().len(),
            8,
        );
        let mut buf = ioctl_buffer(name, "", extra_len);
        let mut ioctl: DmIoctl = read_struct(&buf, 0).expect("ioctl header");
        ioctl.target_count = 1;
        let mut spec = DmTargetSpec {
            sector_start: 0,
            length,
            status: 0,
            next: extra_len as u32,
            target_type: [0; DM_MAX_TYPE_NAME],
        };
        write_fixed_cstr(&mut spec.target_type, "multipath");
        let off = ioctl.data_start as usize;
        write_struct(&mut buf, off, &spec).expect("target spec");
        let params_start = off + core::mem::size_of::<DmTargetSpec>();
        buf[params_start..params_start + params.as_bytes().len()]
            .copy_from_slice(params.as_bytes());
        write_struct(&mut buf, 0, &ioctl).expect("rewrite ioctl header");
        buf
    }

    fn target_msg_ioctl_buffer(name: &str, sector: u64, message: &str) -> Vec<u8> {
        let payload_len = core::mem::size_of::<DmTargetMsg>() + message.len() + 1;
        let mut buf = ioctl_buffer(name, "", payload_len);
        let off = core::mem::size_of::<DmIoctl>();
        write_struct(&mut buf, off, &DmTargetMsg { sector }).expect("target msg");
        let message_start = off + core::mem::size_of::<DmTargetMsg>();
        buf[message_start..message_start + message.len()].copy_from_slice(message.as_bytes());
        buf
    }

    fn has_record(payload: &[u8], record: &[u8]) -> bool {
        payload
            .split(|byte| *byte == 0)
            .any(|field| field == record)
    }

    fn write_lvm2_scan_fixture(bdev: &BlockDeviceRef, metadata: &str) {
        const MDA_OFFSET: u64 = 4096;
        const MDA_SIZE: u64 = 8192;
        const RAW_OFFSET: u64 = 512;
        let raw_size = metadata.len() + 1;
        let raw_write_len = align_up(raw_size, 512);
        assert!(RAW_OFFSET as usize + raw_write_len <= MDA_SIZE as usize);

        let mut label = alloc::vec![0u8; 512];
        label[0..8].copy_from_slice(LVM_LABEL_ID);
        put_le_u64(&mut label, 8, 1);
        put_le_u32(&mut label, 20, LVM_LABEL_HEADER_SIZE as u32);
        label[24..32].copy_from_slice(LVM_LABEL_TYPE);

        let pv = LVM_LABEL_HEADER_SIZE;
        let uuid = b"0123456789abcdef0123456789abcdef";
        label[pv..pv + 32].copy_from_slice(uuid);
        put_le_u64(&mut label, pv + 32, bdev.capacity_bytes());

        let mut loc = pv + LVM_PV_HEADER_SIZE;
        put_le_u64(&mut label, loc, 8192);
        put_le_u64(&mut label, loc + 8, bdev.capacity_bytes() - 8192);
        loc += LVM_DISK_LOCN_SIZE;
        loc += LVM_DISK_LOCN_SIZE; // data-area terminator remains zeroed.
        put_le_u64(&mut label, loc, MDA_OFFSET);
        put_le_u64(&mut label, loc + 8, MDA_SIZE);
        write_bytes(bdev, 512, &label);

        let mut header = alloc::vec![0u8; 512];
        header[4..20].copy_from_slice(LVM_MDA_MAGIC);
        put_le_u32(&mut header, 20, 1);
        put_le_u64(&mut header, 24, MDA_OFFSET);
        put_le_u64(&mut header, 32, MDA_SIZE);
        put_le_u64(&mut header, 40, RAW_OFFSET);
        put_le_u64(&mut header, 48, raw_size as u64);
        put_le_u32(&mut header, 56, 0x1234_5678);
        write_bytes(bdev, MDA_OFFSET, &header);

        let mut raw = alloc::vec![0u8; raw_write_len];
        raw[..metadata.len()].copy_from_slice(metadata.as_bytes());
        write_bytes(bdev, MDA_OFFSET + RAW_OFFSET, &raw);
    }

    fn write_bytes(bdev: &BlockDeviceRef, byte_offset: u64, bytes: &[u8]) {
        assert_eq!(byte_offset % 512, 0);
        assert_eq!(bytes.len() % 512, 0);
        let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_WRITE), byte_offset / 512);
        bio.add_vec(BioVec::new(bytes.to_vec()));
        submit_bio(bio).expect("write bytes");
    }

    fn status_count(statuses: &[DmDeviceStatus], name: &str) -> usize {
        statuses.iter().filter(|status| status.name == name).count()
    }

    fn put_le_u32(bytes: &mut [u8], off: usize, value: u32) {
        bytes[off..off + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_le_u64(bytes: &mut [u8], off: usize, value: u64) {
        bytes[off..off + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_sector(bdev: &BlockDeviceRef, sector: u64, byte: u8) {
        let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_WRITE), sector);
        bio.add_vec(BioVec::new(alloc::vec![byte; 512]));
        submit_bio(bio).expect("write sector");
    }

    fn assert_sector_byte(bdev: &BlockDeviceRef, sector: u64, byte: u8) {
        let bio = bio_alloc(bdev.clone(), BioOp(BIO_OP_READ), sector);
        bio.add_vec(BioVec::new(alloc::vec![0u8; 512]));
        submit_bio(bio.clone()).expect("read sector");
        let vecs = bio.vecs.lock();
        let data = vecs[0].data.lock();
        assert!(data.iter().all(|actual| *actual == byte));
    }
}
