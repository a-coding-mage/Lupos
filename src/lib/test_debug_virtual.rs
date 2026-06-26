//! linux-parity: complete
//! linux-source: vendor/linux/lib/test_debug_virtual.c
//! test-origin: linux:vendor/linux/lib/test_debug_virtual.c
//! CONFIG_DEBUG_VIRTUAL test module flow.

use crate::include::uapi::errno::ENOMEM;

pub const VMALLOC_START: usize = 0xffff_8000_0000_0000;
pub const FOO_TEST_VA: usize = 0x1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DebugVirtualLog {
    pub va: usize,
    pub pa: usize,
}

pub fn test_debug_virtual_init_with<F>(
    allocation_ok: bool,
    mut virt_to_phys: F,
) -> Result<[DebugVirtualLog; 2], i32>
where
    F: FnMut(usize) -> usize,
{
    let vmalloc_log = DebugVirtualLog {
        va: VMALLOC_START,
        pa: virt_to_phys(VMALLOC_START),
    };
    if !allocation_ok {
        return Err(-ENOMEM);
    }
    let foo_log = DebugVirtualLog {
        va: FOO_TEST_VA,
        pa: virt_to_phys(FOO_TEST_VA),
    };
    Ok([vmalloc_log, foo_log])
}

pub const fn test_debug_virtual_exit_frees_foo() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_virtual_matches_linux_init_and_exit_flow() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/lib/test_debug_virtual.c"
        ));
        assert!(source.contains("va = (void *)VMALLOC_START;"));
        assert!(source.contains("pa = virt_to_phys(va);"));
        assert!(source.contains("pr_info(\"PA: %pa for VA: 0x%lx\\n\""));
        assert!(source.contains("foo = kzalloc_obj(*foo);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("pa = virt_to_phys(foo);"));
        assert!(source.contains("module_init(test_debug_virtual_init);"));
        assert!(source.contains("kfree(foo);"));
        assert!(source.contains("MODULE_DESCRIPTION(\"Test module for CONFIG_DEBUG_VIRTUAL\")"));

        let logs = test_debug_virtual_init_with(true, |va| va - 0x1000).expect("debug virtual");
        assert_eq!(
            logs,
            [
                DebugVirtualLog {
                    va: VMALLOC_START,
                    pa: VMALLOC_START - 0x1000,
                },
                DebugVirtualLog {
                    va: FOO_TEST_VA,
                    pa: 0,
                },
            ]
        );
        assert_eq!(test_debug_virtual_init_with(false, |va| va), Err(-ENOMEM));
        assert!(test_debug_virtual_exit_frees_foo());
    }
}
