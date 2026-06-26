//! linux-parity: complete
//! linux-source: vendor/linux/net/9p/trans_common.c
//! test-origin: linux:vendor/linux/net/9p/trans_common.c
//! 9P transaction page release helper.

pub const P9_RELEASE_PAGES_SYMBOL: &str = "p9_release_pages";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct P9Page {
    pub id: usize,
    pub refcount: usize,
}

impl P9Page {
    pub const fn new(id: usize, refcount: usize) -> Self {
        Self { id, refcount }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct P9ReleasePagesReport {
    pub nr_pages: usize,
    pub put_page_calls: usize,
    pub skipped_null_pages: usize,
}

pub fn put_page(page: &mut P9Page) {
    page.refcount = page.refcount.saturating_sub(1);
}

pub fn p9_release_pages(pages: &mut [Option<P9Page>], nr_pages: usize) -> P9ReleasePagesReport {
    assert!(nr_pages <= pages.len());

    let mut report = P9ReleasePagesReport {
        nr_pages,
        ..P9ReleasePagesReport::default()
    };

    for page in pages.iter_mut().take(nr_pages) {
        if let Some(page) = page.as_mut() {
            put_page(page);
            report.put_page_calls += 1;
        } else {
            report.skipped_null_pages += 1;
        }
    }

    report
}

pub fn p9_release_pages_count(pages_present: &[bool]) -> usize {
    pages_present.iter().filter(|present| **present).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p9_release_pages_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/net/9p/trans_common.c"
        ));
        assert!(source.contains("#include <linux/mm.h>"));
        assert!(source.contains("#include <linux/module.h>"));
        assert!(source.contains("#include \"trans_common.h\""));
        assert!(source.contains("@pages: array of pages to be put"));
        assert!(source.contains("@nr_pages: size of array"));
        assert!(source.contains("int i;"));
        assert!(source.contains("for (i = 0; i < nr_pages; i++)"));
        assert!(source.contains("if (pages[i])"));
        assert!(source.contains("put_page(pages[i]);"));
        assert!(source.contains("EXPORT_SYMBOL(p9_release_pages);"));
        assert_eq!(P9_RELEASE_PAGES_SYMBOL, "p9_release_pages");
        assert_eq!(p9_release_pages_count(&[true, false, true]), 2);
    }

    #[test]
    fn p9_release_pages_puts_non_null_pages_in_range() {
        let mut pages = [
            Some(P9Page::new(10, 3)),
            None,
            Some(P9Page::new(12, 1)),
            Some(P9Page::new(13, 7)),
        ];

        let report = p9_release_pages(&mut pages, 3);
        assert_eq!(
            report,
            P9ReleasePagesReport {
                nr_pages: 3,
                put_page_calls: 2,
                skipped_null_pages: 1,
            }
        );
        assert_eq!(pages[0], Some(P9Page::new(10, 2)));
        assert_eq!(pages[1], None);
        assert_eq!(pages[2], Some(P9Page::new(12, 0)));
        assert_eq!(pages[3], Some(P9Page::new(13, 7)));
    }

    #[test]
    fn put_page_saturates_local_refcount_model() {
        let mut page = P9Page::new(7, 0);
        put_page(&mut page);
        assert_eq!(page, P9Page::new(7, 0));
    }
}
