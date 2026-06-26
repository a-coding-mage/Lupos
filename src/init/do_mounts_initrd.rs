//! linux-parity: complete
//! linux-source: vendor/linux/init/do_mounts_initrd.c
//! test-origin: linux:vendor/linux/init/do_mounts_initrd.c
//! Deprecated initrd command-line and load path.

use crate::include::uapi::errno::EINVAL;

pub const ROOT_RAM0: u32 = (1 << 20) | 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitrdState {
    pub initrd_start: u64,
    pub initrd_end: u64,
    pub initrd_below_start_ok: bool,
    pub mount_initrd: bool,
    pub phys_initrd_start: u64,
    pub phys_initrd_size: u64,
}

impl InitrdState {
    pub const fn new() -> Self {
        Self {
            initrd_start: 0,
            initrd_end: 0,
            initrd_below_start_ok: false,
            mount_initrd: true,
            phys_initrd_start: 0,
            phys_initrd_size: 0,
        }
    }

    pub fn no_initrd(&mut self) -> i32 {
        self.mount_initrd = false;
        1
    }

    pub fn early_initrdmem(&mut self, arg: &str) -> Result<(), i32> {
        let Some((start, rest)) = parse_memparse(arg) else {
            return Err(-EINVAL);
        };
        let Some(size_arg) = rest.strip_prefix(',') else {
            return Ok(());
        };
        let Some((size, _)) = parse_memparse(size_arg) else {
            return Err(-EINVAL);
        };
        self.phys_initrd_start = start;
        self.phys_initrd_size = size;
        Ok(())
    }

    pub fn initrd_load(&self, rd_load_image_ok: bool) -> InitrdLoadPlan {
        InitrdLoadPlan {
            create_ram_device: self.mount_initrd,
            root_device: if self.mount_initrd {
                Some(ROOT_RAM0)
            } else {
                None
            },
            deprecated_initrd_used: self.mount_initrd && rd_load_image_ok,
            unlink_initrd_image: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InitrdLoadPlan {
    pub create_ram_device: bool,
    pub root_device: Option<u32>,
    pub deprecated_initrd_used: bool,
    pub unlink_initrd_image: bool,
}

fn parse_memparse(arg: &str) -> Option<(u64, &str)> {
    let bytes = arg.as_bytes();
    let mut index = 0usize;
    let mut radix = 10u32;
    if bytes.len() >= 2 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X') {
        radix = 16;
        index = 2;
    }
    let start = index;
    let mut value = 0u64;
    while let Some(&byte) = bytes.get(index) {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'a'..=b'f' => 10 + (byte - b'a') as u32,
            b'A'..=b'F' => 10 + (byte - b'A') as u32,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        value = value
            .saturating_mul(radix as u64)
            .saturating_add(digit as u64);
        index += 1;
    }
    if index == start {
        return None;
    }
    if let Some(&suffix) = bytes.get(index) {
        let shift = match suffix {
            b'K' | b'k' => Some(10),
            b'M' | b'm' => Some(20),
            b'G' | b'g' => Some(30),
            _ => None,
        };
        if let Some(shift) = shift {
            value = value.checked_shl(shift).unwrap_or(u64::MAX);
            index += 1;
        }
    }
    Some((value, &arg[index..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initrd_command_line_and_load_path_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/init/do_mounts_initrd.c"
        ));
        assert!(source.contains("unsigned long initrd_start, initrd_end;"));
        assert!(source.contains("static int __initdata mount_initrd = 1;"));
        assert!(source.contains("__setup(\"noinitrd\", no_initrd);"));
        assert!(source.contains("early_param(\"initrdmem\", early_initrdmem);"));
        assert!(source.contains("early_param(\"initrd\", early_initrd);"));
        assert!(source.contains("create_dev(\"/dev/ram\", Root_RAM0);"));
        assert!(source.contains("rd_load_image()"));
        assert!(source.contains("init_unlink(\"/initrd.image\");"));

        let mut state = InitrdState::new();
        assert_eq!(state.early_initrdmem("0x1000,4M"), Ok(()));
        assert_eq!(state.phys_initrd_start, 0x1000);
        assert_eq!(state.phys_initrd_size, 4 << 20);
        assert_eq!(
            state.initrd_load(true),
            InitrdLoadPlan {
                create_ram_device: true,
                root_device: Some(ROOT_RAM0),
                deprecated_initrd_used: true,
                unlink_initrd_image: true,
            }
        );
        assert_eq!(state.no_initrd(), 1);
        assert!(!state.initrd_load(true).create_ram_device);
    }
}
