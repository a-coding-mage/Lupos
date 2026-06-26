//! linux-parity: complete
//! linux-source: vendor/linux/kernel/resource_kunit.c
//! test-origin: linux:vendor/linux/kernel/resource_kunit.c
//! Resource union/intersection KUnit table coverage.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resource {
    pub start: u64,
    pub end: u64,
}

pub const R0: Resource = Resource {
    start: 0x0000,
    end: 0xffff,
};
pub const R1: Resource = Resource {
    start: 0x1234,
    end: 0x2345,
};
pub const R2: Resource = Resource {
    start: 0x4567,
    end: 0x5678,
};
pub const R4: Resource = Resource {
    start: 0x2000,
    end: 0x7000,
};

pub fn resource_union(a: Resource, b: Resource) -> Option<Resource> {
    let touches = a.start <= b.end.saturating_add(1) && b.start <= a.end.saturating_add(1);
    touches.then(|| Resource {
        start: a.start.min(b.start),
        end: a.end.max(b.end),
    })
}

pub fn resource_intersection(a: Resource, b: Resource) -> Option<Resource> {
    let start = a.start.max(b.start);
    let end = a.end.min(b.end);
    (start <= end).then_some(Resource { start, end })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_kunit_matches_linux_original_union_and_intersection_tables() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/resource_kunit.c"
        ));

        assert!(source.contains("static struct result results_for_union[]"));
        assert!(source.contains("static struct result results_for_intersection[]"));
        assert!(source.contains("KUNIT_CASE(resource_test_union)"));
        assert!(source.contains("KUNIT_CASE(resource_test_intersection)"));
        assert!(source.contains("KUNIT_CASE(resource_test_region_intersects)"));
        assert!(source.contains(".name = \"resource\""));
        assert!(
            source
                .contains("MODULE_DESCRIPTION(\"I/O Port & Memory Resource manager unit tests\")")
        );
        assert!(source.contains("REGION_INTERSECTS"));
        assert!(source.contains("REGION_DISJOINT"));
        assert!(source.contains("REGION_MIXED"));

        assert_eq!(resource_union(R1, R0), Some(R0));
        assert_eq!(resource_union(R2, R1), None);
        assert_eq!(
            resource_union(R4, R1),
            Some(Resource {
                start: R1.start,
                end: R4.end
            })
        );
        assert_eq!(resource_intersection(R1, R0), Some(R1));
        assert_eq!(resource_intersection(R2, R1), None);
        assert_eq!(
            resource_intersection(R4, R1),
            Some(Resource {
                start: R4.start,
                end: R1.end
            })
        );
    }
}
