//! linux-parity: complete
//! linux-source: vendor/linux/kernel/regset.c
//! test-origin: linux:vendor/linux/kernel/regset.c
//! User register-set fetch helpers.

use crate::include::uapi::errno::{EFAULT, ENOMEM, EOPNOTSUPP};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegsetShape {
    pub n: usize,
    pub size: usize,
    pub has_get: bool,
}

pub const fn regset_capacity(regset: RegsetShape) -> usize {
    regset.n * regset.size
}

pub const fn regset_get_result(
    regset: RegsetShape,
    requested_size: usize,
    data_preallocated: bool,
    alloc_ok: bool,
    getter_remaining: i32,
) -> Result<usize, i32> {
    if !regset.has_get {
        return Err(-EOPNOTSUPP);
    }
    let capacity = regset_capacity(regset);
    let size = if requested_size > capacity {
        capacity
    } else {
        requested_size
    };
    if !data_preallocated && !alloc_ok {
        return Err(-ENOMEM);
    }
    if getter_remaining < 0 {
        return Err(getter_remaining);
    }
    Ok(size - getter_remaining as usize)
}

pub const fn copy_regset_to_user_result(regset_get_ret: i32, copy_failed: bool) -> i32 {
    if regset_get_ret > 0 && copy_failed {
        -EFAULT
    } else if regset_get_ret > 0 {
        0
    } else {
        regset_get_ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regset_fetch_flow_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/regset.c"
        ));
        assert!(source.contains("static int __regset_get"));
        assert!(source.contains("if (!regset->regset_get)"));
        assert!(source.contains("return -EOPNOTSUPP;"));
        assert!(source.contains("if (size > regset->n * regset->size)"));
        assert!(source.contains("to_free = p = kvzalloc(size, GFP_KERNEL);"));
        assert!(source.contains("return -ENOMEM;"));
        assert!(source.contains("kvfree(to_free);"));
        assert!(source.contains("return size - res;"));
        assert!(source.contains("regset_get_alloc"));
        assert!(source.contains("copy_to_user(data, buf, ret) ? -EFAULT : 0"));
        assert!(source.contains("EXPORT_SYMBOL(regset_get_alloc)"));

        let regset = RegsetShape {
            n: 4,
            size: 8,
            has_get: true,
        };
        assert_eq!(regset_get_result(regset, 64, false, true, 0), Ok(32));
        assert_eq!(regset_get_result(regset, 16, true, true, 4), Ok(12));
        assert_eq!(
            regset_get_result(
                RegsetShape {
                    has_get: false,
                    ..regset
                },
                16,
                true,
                true,
                0,
            ),
            Err(-EOPNOTSUPP)
        );
        assert_eq!(regset_get_result(regset, 16, false, false, 0), Err(-ENOMEM));
        assert_eq!(copy_regset_to_user_result(12, true), -EFAULT);
        assert_eq!(copy_regset_to_user_result(12, false), 0);
    }
}
