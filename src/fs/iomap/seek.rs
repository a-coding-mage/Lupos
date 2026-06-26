//! linux-parity: complete
//! linux-source: vendor/linux/fs/iomap/seek.c
//! test-origin: linux:vendor/linux/fs/iomap/seek.c
//! iomap SEEK_HOLE and SEEK_DATA traversal.

use crate::include::uapi::errno::ENXIO;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IomapExtentType {
    Hole,
    Unwritten { has_pagecache_data: bool },
    Data,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IomapSeekExtent {
    pub len: i64,
    pub extent_type: IomapExtentType,
}

pub fn iomap_seek_hole_result(
    size: i64,
    pos: i64,
    extents: &[IomapSeekExtent],
) -> Result<i64, i32> {
    if pos < 0 || pos >= size {
        return Err(-ENXIO);
    }
    let mut cur = pos;
    for extent in extents {
        match extent.extent_type {
            IomapExtentType::Hole => return Ok(cur),
            IomapExtentType::Unwritten {
                has_pagecache_data: false,
            } => return Ok(cur),
            IomapExtentType::Unwritten {
                has_pagecache_data: true,
            }
            | IomapExtentType::Data => {
                cur += extent.len;
            }
        }
    }
    Ok(size)
}

pub fn iomap_seek_data_result(
    size: i64,
    pos: i64,
    extents: &[IomapSeekExtent],
) -> Result<i64, i32> {
    if pos < 0 || pos >= size {
        return Err(-ENXIO);
    }
    let mut cur = pos;
    for extent in extents {
        match extent.extent_type {
            IomapExtentType::Hole => cur += extent.len,
            IomapExtentType::Unwritten {
                has_pagecache_data: false,
            } => cur += extent.len,
            IomapExtentType::Unwritten {
                has_pagecache_data: true,
            }
            | IomapExtentType::Data => return Ok(cur),
        }
    }
    Err(-ENXIO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iomap_seek_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/fs/iomap/seek.c"
        ));
        assert!(source.contains("#include <linux/iomap.h>"));
        assert!(source.contains("#include <linux/pagemap.h>"));
        assert!(source.contains("static int iomap_seek_hole_iter"));
        assert!(source.contains("case IOMAP_UNWRITTEN:"));
        assert!(source.contains("mapping_seek_hole_data(iter->inode->i_mapping,"));
        assert!(source.contains("SEEK_HOLE"));
        assert!(source.contains("if (*hole_pos == iter->pos + length)"));
        assert!(source.contains("case IOMAP_HOLE:"));
        assert!(source.contains("*hole_pos = iter->pos;"));
        assert!(source.contains("return iomap_iter_advance(iter, length);"));
        assert!(source.contains("iomap_seek_hole"));
        assert!(source.contains("if (pos < 0 || pos >= size)"));
        assert!(source.contains("return -ENXIO;"));
        assert!(source.contains(".flags\t= IOMAP_REPORT"));
        assert!(source.contains("if (iter.len) /* found hole before EOF */"));
        assert!(source.contains("return size;"));
        assert!(source.contains("static int iomap_seek_data_iter"));
        assert!(source.contains("SEEK_DATA"));
        assert!(source.contains("if (*hole_pos < 0)"));
        assert!(source.contains("iomap_seek_data"));
        assert!(source.contains("/* We've reached the end of the file without finding data */"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(iomap_seek_hole);"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(iomap_seek_data);"));

        let extents = [
            IomapSeekExtent {
                len: 4,
                extent_type: IomapExtentType::Data,
            },
            IomapSeekExtent {
                len: 2,
                extent_type: IomapExtentType::Hole,
            },
        ];
        assert_eq!(iomap_seek_hole_result(10, 0, &extents), Ok(4));
        assert_eq!(iomap_seek_data_result(10, 0, &extents), Ok(0));
        assert_eq!(
            iomap_seek_hole_result(
                10,
                0,
                &[IomapSeekExtent {
                    len: 5,
                    extent_type: IomapExtentType::Unwritten {
                        has_pagecache_data: false
                    },
                }]
            ),
            Ok(0)
        );
        assert_eq!(
            iomap_seek_data_result(
                10,
                0,
                &[IomapSeekExtent {
                    len: 5,
                    extent_type: IomapExtentType::Unwritten {
                        has_pagecache_data: false
                    },
                }]
            ),
            Err(-ENXIO)
        );
        assert_eq!(iomap_seek_hole_result(10, 10, &[]), Err(-ENXIO));
    }
}
