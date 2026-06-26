//! linux-parity: complete
//! linux-source: vendor/linux/kernel/livepatch/state.c
//! test-origin: linux:vendor/linux/kernel/livepatch/state.c
//! Livepatch system-state compatibility checks.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KlpState {
    pub id: u64,
    pub version: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KlpPatch<'a> {
    pub states: &'a [KlpState],
    pub replace: bool,
}

pub fn klp_get_state<'a>(patch: &'a KlpPatch<'a>, id: u64) -> Option<&'a KlpState> {
    patch
        .states
        .iter()
        .take_while(|state| state.id != 0)
        .find(|state| state.id == id)
}

pub fn klp_get_prev_state<'a>(
    patches: &'a [KlpPatch<'a>],
    transition_index: Option<usize>,
    id: u64,
) -> Option<&'a KlpState> {
    let transition_index = transition_index?;
    let mut last_state = None;
    for (idx, patch) in patches.iter().enumerate() {
        if idx == transition_index {
            break;
        }
        if let Some(state) = klp_get_state(patch, id) {
            last_state = Some(state);
        }
    }
    last_state
}

pub fn klp_is_state_compatible(patch: &KlpPatch<'_>, old_state: &KlpState) -> bool {
    match klp_get_state(patch, old_state.id) {
        None => !patch.replace,
        Some(state) => state.version >= old_state.version,
    }
}

pub fn klp_is_patch_compatible(patch: &KlpPatch<'_>, old_patches: &[KlpPatch<'_>]) -> bool {
    for old_patch in old_patches {
        for old_state in old_patch.states.iter().take_while(|state| state.id != 0) {
            if !klp_is_state_compatible(patch, old_state) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn livepatch_state_lookup_matches_linux_source() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/livepatch/state.c"
        ));
        assert!(source.contains("#define klp_for_each_state(patch, state)"));
        assert!(source.contains("for (state = patch->states; state && state->id; state++)"));
        assert!(source.contains(
            "struct klp_state *klp_get_state(struct klp_patch *patch, unsigned long id)"
        ));
        assert!(source.contains("if (state->id == id)"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(klp_get_state);"));
        assert!(source.contains("struct klp_state *klp_get_prev_state(unsigned long id)"));
        assert!(source.contains("if (WARN_ON_ONCE(!klp_transition_patch))"));
        assert!(source.contains("if (patch == klp_transition_patch)"));
        assert!(source.contains("last_state = state;"));
        assert!(source.contains("EXPORT_SYMBOL_GPL(klp_get_prev_state);"));

        let states = [
            KlpState { id: 10, version: 1 },
            KlpState { id: 20, version: 3 },
            KlpState { id: 0, version: 0 },
            KlpState { id: 30, version: 9 },
        ];
        let patch = KlpPatch {
            states: &states,
            replace: false,
        };
        assert_eq!(klp_get_state(&patch, 20), Some(&states[1]));
        assert_eq!(klp_get_state(&patch, 30), None);
    }

    #[test]
    fn livepatch_compatibility_matches_replace_and_version_rules() {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/vendor/linux/kernel/livepatch/state.c"
        ));
        assert!(source.contains("static bool klp_is_state_compatible"));
        assert!(source.contains("return !patch->replace;"));
        assert!(source.contains("return state->version >= old_state->version;"));
        assert!(source.contains("bool klp_is_patch_compatible(struct klp_patch *patch)"));
        assert!(source.contains("klp_for_each_patch(old_patch)"));
        assert!(source.contains("klp_for_each_state(old_patch, old_state)"));
        assert!(source.contains("return false;"));
        assert!(source.contains("return true;"));

        let old_states = [
            KlpState { id: 1, version: 2 },
            KlpState { id: 0, version: 0 },
        ];
        let newer_states = [
            KlpState { id: 1, version: 3 },
            KlpState { id: 0, version: 0 },
        ];
        let older_states = [
            KlpState { id: 1, version: 1 },
            KlpState { id: 0, version: 0 },
        ];
        let missing_states = [KlpState { id: 0, version: 0 }];
        let old_patch = KlpPatch {
            states: &old_states,
            replace: false,
        };
        let cumulative_new = KlpPatch {
            states: &newer_states,
            replace: true,
        };
        let cumulative_old = KlpPatch {
            states: &older_states,
            replace: true,
        };
        let non_cumulative_missing = KlpPatch {
            states: &missing_states,
            replace: false,
        };
        let cumulative_missing = KlpPatch {
            states: &missing_states,
            replace: true,
        };

        assert!(klp_is_patch_compatible(&cumulative_new, &[old_patch]));
        assert!(!klp_is_patch_compatible(&cumulative_old, &[old_patch]));
        assert!(klp_is_patch_compatible(
            &non_cumulative_missing,
            &[old_patch]
        ));
        assert!(!klp_is_patch_compatible(&cumulative_missing, &[old_patch]));

        let patches = [old_patch, cumulative_new];
        assert_eq!(
            klp_get_prev_state(&patches, Some(1), 1),
            Some(&old_states[0])
        );
        assert_eq!(klp_get_prev_state(&patches, None, 1), None);
    }
}
