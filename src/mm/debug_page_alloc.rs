//! linux-parity: complete
//! linux-source: vendor/linux/mm/debug_page_alloc.c
//! test-origin: linux:vendor/linux/mm/debug_page_alloc.c
//! Debug page allocation guard-page controls.

use crate::include::uapi::errno::EINVAL;

pub const MAX_PAGE_ORDER: u32 = crate::mm::zone::MAX_PAGE_ORDER as u32;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DebugPageAllocState {
    pub pagealloc_enabled_early: bool,
    pub pagealloc_enabled: bool,
    pub guardpage_enabled: bool,
    pub guardpage_minorder: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DebugPage {
    pub guard: bool,
    pub private: u32,
    pub buddy_list_initialized: bool,
}

pub fn early_debug_pagealloc(state: &mut DebugPageAllocState, value: &str) -> Result<(), i32> {
    state.pagealloc_enabled_early = parse_bool(value).ok_or(-EINVAL)?;
    Ok(())
}

pub fn debug_guardpage_minorder_setup(
    state: &mut DebugPageAllocState,
    value: &str,
) -> Result<(), i32> {
    let order = value.parse::<u32>().map_err(|_| -EINVAL)?;
    if order > MAX_PAGE_ORDER / 2 {
        return Err(-EINVAL);
    }
    state.guardpage_minorder = order;
    Ok(())
}

pub const fn set_page_guard(
    state: DebugPageAllocState,
    mut page: DebugPage,
    order: u32,
) -> (bool, DebugPage) {
    if order >= state.guardpage_minorder {
        return (false, page);
    }
    page.guard = true;
    page.buddy_list_initialized = true;
    page.private = order;
    (true, page)
}

pub const fn clear_page_guard(mut page: DebugPage) -> DebugPage {
    page.guard = false;
    page.private = 0;
    page
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "1" | "y" | "Y" | "yes" | "true" | "on" => Some(true),
        "0" | "n" | "N" | "no" | "false" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_page_alloc_guards_match_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/mm/debug_page_alloc.c"
        ));
        assert!(source.contains("_debug_guardpage_minorder"));
        assert!(source.contains("early_param(\"debug_pagealloc\", early_debug_pagealloc);"));
        assert!(source.contains("kstrtoul(buf, 10, &res)"));
        assert!(source.contains("res > MAX_PAGE_ORDER / 2"));
        assert!(source.contains("early_param(\"debug_guardpage_minorder\""));
        assert!(source.contains("if (order >= debug_guardpage_minorder())"));
        assert!(source.contains("__SetPageGuard(page);"));
        assert!(source.contains("INIT_LIST_HEAD(&page->buddy_list);"));
        assert!(source.contains("set_page_private(page, order);"));
        assert!(source.contains("__ClearPageGuard(page);"));

        let mut state = DebugPageAllocState::default();
        assert_eq!(early_debug_pagealloc(&mut state, "on"), Ok(()));
        assert!(state.pagealloc_enabled_early);
        assert_eq!(debug_guardpage_minorder_setup(&mut state, "2"), Ok(()));
        let (set, page) = set_page_guard(state, DebugPage::default(), 1);
        assert!(set);
        assert!(page.guard);
        assert_eq!(page.private, 1);
        assert!(!set_page_guard(state, DebugPage::default(), 2).0);
        assert!(!clear_page_guard(page).guard);
    }
}
